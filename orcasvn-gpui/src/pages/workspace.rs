use super::Shared;
use crate::app::OrcaApp;
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::cache_dir;
use crate::svn;
use crate::system::{self, OpenTarget};
use crate::task;
use crate::ui::{self, Palette, TextArea, TextField};
use crate::SvnStatus;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::BTreeSet;
use std::future::Future;
use std::rc::Rc;
use std::time::{Duration, Instant};

const AUTO_REFRESH: Duration = Duration::from_secs(8);

enum Confirm {
    Revert(Vec<String>),
    Remove(Vec<String>),
    DeleteUnversioned(Vec<String>),
}

pub struct WorkspacePage {
    loaded_for: Option<String>,
    entries: Vec<SvnStatus>,
    status_running: bool,
    last_refresh: Option<Instant>,
    checked: BTreeSet<String>,
    focus: Option<String>,
    filter: TextField,
    hide_unversioned: bool,
    diff: Option<DiffDocument>,
    diff_running: bool,
    message: TextArea,
    op_running: bool,
    op_label: String,
    confirm: Option<Confirm>,
    update_rev: TextField,
    _subs: Vec<Subscription>,
}

impl WorkspacePage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        let filter = TextField::new(cx, window, t("过滤文件…", "Filter files…"));
        let message = TextArea::new(cx, window, t("提交信息…", "Commit message…"), 3);
        let update_rev = TextField::new(cx, window, "HEAD");
        let subs = vec![ui::on_enter(cx, update_rev.state(), |app, cx| {
            app.workspace.submit_update(&mut app.shared, cx);
        })];
        Self {
            loaded_for: None,
            entries: Vec::new(),
            status_running: false,
            last_refresh: None,
            checked: BTreeSet::new(),
            focus: None,
            filter,
            hide_unversioned: false,
            diff: None,
            diff_running: false,
            message,
            op_running: false,
            op_label: String::new(),
            confirm: None,
            update_rev,
            _subs: subs,
        }
    }

    /// Run `svn update` with the revision currently typed in the toolbar.
    fn submit_update(&mut self, shared: &mut Shared, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.workspace() else {
            return;
        };
        let text = self.update_rev.text().trim().to_string();
        let revision = text.trim_start_matches('r').parse::<u64>().ok();
        if revision.is_none() && !text.is_empty() {
            shared.err(t("无效的版本号", "Invalid revision"));
            cx.notify();
            return;
        }
        self.run_op(
            "update",
            async move {
                svn::update(&workspace, revision)
                    .await
                    .map_err(|error| error.to_string())
            },
            cx,
        );
    }

    /// Drop workspace-specific state but keep the text fields and their focus
    /// handles, which must stay stable for the lifetime of the window.
    pub fn reset(&mut self) {
        self.loaded_for = None;
        self.entries.clear();
        self.checked.clear();
        self.focus = None;
        self.diff = None;
        self.filter.clear();
        self.message.clear();
        self.update_rev.clear();
        self.confirm = None;
        self.op_running = false;
        self.status_running = false;
        self.diff_running = false;
        self.last_refresh = None;
    }

    fn workspace(&self) -> Option<String> {
        self.loaded_for.clone()
    }

    fn refresh(&mut self, cx: &mut Context<OrcaApp>, force: bool) {
        if self.status_running {
            return;
        }
        let Some(workspace) = self.workspace() else {
            return;
        };
        self.status_running = true;
        self.last_refresh = Some(Instant::now());
        task::spawn(
            cx,
            async move {
                let result = svn::cached_status(&workspace, force, cache_dir())
                    .await
                    .map_err(|error| error.to_string());
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                app.workspace
                    .apply_status(workspace, result, &mut app.shared, cx)
            },
        );
    }

    pub fn maybe_auto_refresh(&mut self, cx: &mut Context<OrcaApp>) {
        if self.op_running || self.status_running {
            return;
        }
        if self
            .last_refresh
            .is_none_or(|at| at.elapsed() >= AUTO_REFRESH)
        {
            self.refresh(cx, false);
        }
    }

    fn run_op<F>(&mut self, label: &str, future: F, cx: &mut Context<OrcaApp>)
    where
        F: Future<Output = Result<String, String>> + Send + 'static,
    {
        if self.op_running || self.loaded_for.is_none() {
            return;
        }
        self.op_running = true;
        self.op_label = label.to_string();
        let label = label.to_string();
        task::spawn(cx, future, move |app, result, cx| {
            app.workspace.apply_op(label, result, &mut app.shared, cx)
        });
    }

    fn load_diff(&mut self, file: String, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.workspace() else {
            return;
        };
        self.focus = Some(file.clone());
        self.diff_running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::diff(&workspace, &file, None, None)
                    .await
                    .map_err(|error| error.to_string());
                (file, result)
            },
            |app, (file, result), cx| {
                app.workspace.diff_running = false;
                if app.workspace.focus.as_deref() != Some(file.as_str()) {
                    return;
                }
                match result {
                    Ok(diff) => app.workspace.diff = Some(DiffDocument::new(diff.path, &diff.diff)),
                    Err(error) => app.shared.err(error),
                }
                cx.notify();
            },
        );
    }

    fn apply_status(
        &mut self,
        workspace: String,
        result: Result<Vec<SvnStatus>, String>,
        shared: &mut Shared,
        cx: &mut Context<OrcaApp>,
    ) {
        self.status_running = false;
        if self.loaded_for.as_deref() != Some(workspace.as_str()) {
            return;
        }
        match result {
            Ok(mut entries) => {
                entries.sort_by(|a, b| a.path.cmp(&b.path));
                let paths: BTreeSet<_> = entries.iter().map(|e| e.path.clone()).collect();
                self.checked.retain(|path| paths.contains(path));
                if self
                    .focus
                    .as_ref()
                    .is_some_and(|focus| !paths.contains(focus))
                {
                    self.focus = None;
                    self.diff = None;
                }
                self.entries = entries;
            }
            Err(error) => shared.err(error),
        }
        cx.notify();
    }

    fn apply_op(
        &mut self,
        label: String,
        result: Result<String, String>,
        shared: &mut Shared,
        cx: &mut Context<OrcaApp>,
    ) {
        self.op_running = false;
        match result {
            Ok(output) => {
                let conflicts = if label == "update" {
                    svn::parse_update_conflicts(&output)
                } else {
                    Vec::new()
                };
                shared.done(&label, &output);
                if !conflicts.is_empty() {
                    shared.warn(format!(
                        "{}: {}",
                        t("更新完成，但存在冲突", "Update finished with conflicts"),
                        conflicts.join(", ")
                    ));
                }
                if label == "commit" {
                    self.message.clear();
                    self.checked.clear();
                }
            }
            Err(error) => shared.err(error),
        }
        shared.info_dirty = true;
        self.refresh(cx, true);
        if let Some(focus) = self.focus.clone() {
            self.load_diff(focus, cx);
        }
        cx.notify();
    }

    fn selection(&self) -> Vec<String> {
        if !self.checked.is_empty() {
            self.checked.iter().cloned().collect()
        } else {
            self.focus.iter().cloned().collect()
        }
    }

    fn code_of(&self, path: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.path == path)
            .map(|entry| entry.status_code.as_str())
    }

    fn visible_indices(&self) -> Vec<usize> {
        let filter = self.filter.text().to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                !(self.hide_unversioned && entry.status_code == "unversioned")
                    && (filter.is_empty() || entry.path.to_lowercase().contains(&filter))
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn render(
        &mut self,
        shared: &Shared,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let Some(workspace) = shared.workspace.clone() else {
            return div().into_any_element();
        };
        if self.loaded_for.as_deref() != Some(workspace.as_str()) {
            self.reset();
            self.loaded_for = Some(workspace.clone());
            self.refresh(cx, false);
        }

        let toolbar = self.toolbar(&workspace, window, cx, pal);
        let left = self.file_panel(&workspace, window, cx, pal);
        let right = self.diff_panel(&workspace, window, cx, pal);

        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(toolbar)
            .child(ui::sep(pal))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(left)
                    .child(ui::sep_vertical(pal))
                    .child(right),
            )
            .when_some(self.confirm_modal(&workspace, cx, pal), |this, modal| {
                this.child(modal)
            })
            .into_any_element()
    }

    fn toolbar(
        &mut self,
        workspace: &str,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let busy = self.op_running;
        let refresh = ui::button(
            "ws-refresh",
            format!("🔄 {}", t("刷新", "Refresh")),
            pal,
            !busy,
            cx.listener(|app, _, _, cx| app.workspace.refresh(cx, true)),
        );
        let update = ui::button(
            "ws-update",
            format!("⬇ {}", t("更新", "Update")),
            pal,
            !busy,
            cx.listener(|app, _, _, cx| app.workspace.submit_update(&mut app.shared, cx)),
        );
        let workspace_for_cleanup = workspace.to_string();
        let cleanup = ui::button(
            "ws-cleanup",
            t("清理", "Cleanup"),
            pal,
            !busy,
            cx.listener(move |app, _, _, cx| {
                let path = workspace_for_cleanup.clone();
                app.workspace.run_op(
                    "cleanup",
                    async move { svn::cleanup(&path).await.map_err(|error| error.to_string()) },
                    cx,
                );
            }),
        );

        let mut targets = Vec::new();
        for (index, (label, target)) in [
            (t("文件管理器", "File manager"), OpenTarget::Explorer),
            ("VS Code", OpenTarget::Vscode),
            (t("终端", "Terminal"), OpenTarget::Terminal),
        ]
        .into_iter()
        .enumerate()
        {
            let path = workspace.to_string();
            targets.push(ui::button(
                ("ws-open", index),
                label,
                pal,
                true,
                cx.listener(move |app, _, _, cx| {
                    if let Err(error) = system::open_workspace_target(path.clone(), target) {
                        app.shared.err(error);
                        cx.notify();
                    }
                }),
            ));
        }

        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(refresh)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(self.update_rev.input(window, cx).w(px(84.0)))
                    .child(update),
            )
            .child(cleanup)
            .child(ui::sep_vertical(pal))
            .children(targets)
            .when(busy, |this| {
                this.child(ui::spinner(pal, format!("svn {}…", self.op_label)))
            })
            .when(self.status_running && !busy, |this| {
                this.child(ui::spinner(pal, t("正在扫描…", "Scanning…")))
            })
            .into_any_element()
    }

    fn file_panel(
        &mut self,
        workspace: &str,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let filter_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .pt_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(self.filter.input(window, cx).w_full()),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(ui::checkbox(
                        "ws-hide-unversioned",
                        self.hide_unversioned,
                        pal,
                        cx.listener(|app, _, _, cx| {
                            app.workspace.hide_unversioned = !app.workspace.hide_unversioned;
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(pal.muted))
                            .child(t("隐藏未版本", "Hide '?'")),
                    ),
            );

        let visible = self.visible_indices();
        let all_checked = !visible.is_empty()
            && visible
                .iter()
                .all(|&index| self.checked.contains(&self.entries[index].path));
        let visible_for_all = visible.clone();
        let counts = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .child(ui::checkbox(
                "ws-check-all",
                all_checked,
                pal,
                cx.listener(move |app, _, _, cx| {
                    let paths: Vec<String> = visible_for_all
                        .iter()
                        .filter_map(|&index| app.workspace.entries.get(index))
                        .map(|entry| entry.path.clone())
                        .collect();
                    if all_checked {
                        for path in paths {
                            app.workspace.checked.remove(&path);
                        }
                    } else {
                        for path in paths {
                            app.workspace.checked.insert(path);
                        }
                    }
                    cx.notify();
                }),
            ))
            .child(div().text_xs().text_color(rgb(pal.muted)).child(format!(
                "{} {} · {} {}",
                self.entries.len(),
                t("项变更", "changes"),
                self.checked.len(),
                t("已选", "selected")
            )));

        let list = self.entry_list(visible, cx, pal);

        let commit_box = self.commit_box(workspace, window, cx, pal);

        div()
            .w(px(470.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .min_h_0()
            .child(filter_row)
            .child(counts)
            .child(ui::sep(pal))
            .child(list)
            .child(ui::sep(pal))
            .child(commit_box)
            .into_any_element()
    }

    fn entry_list(
        &mut self,
        visible: Vec<usize>,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        if self.entries.is_empty() && !self.status_running {
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t(
                    "工作副本没有本地修改",
                    "The working copy has no local changes",
                ))
                .into_any_element();
        }
        let visible = Rc::new(visible);
        let entity = cx.entity();
        uniform_list("ws-files", visible.len(), move |range, _window, cx| {
            let app = entity.read(cx);
            let page = &app.workspace;
            range
                .map(|slot| {
                    let index = visible[slot];
                    let entry = &page.entries[index];
                    let active = page.focus.as_deref() == Some(entry.path.as_str());
                    let checked = page.checked.contains(&entry.path);
                    let italic = entry.prop_status != "none" && entry.prop_status != "normal";
                    let path = entry.path.clone();
                    let entity_for_toggle = entity.clone();
                    let entity_for_diff = entity.clone();
                    div()
                        .id(("ws-row", index))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .h(px(24.0))
                        .w_full()
                        .text_sm()
                        .when(active, |this| this.bg(rgb(pal.accent_soft)))
                        .hover(|style| style.bg(rgb(pal.elevated)))
                        .child(ui::checkbox(
                            ("ws-check", index),
                            checked,
                            pal,
                            move |_, _, cx| {
                                cx.stop_propagation();
                                entity_for_toggle.update(cx, |app, cx| {
                                    let path = path.clone();
                                    if app.workspace.checked.contains(&path) {
                                        app.workspace.checked.remove(&path);
                                    } else {
                                        app.workspace.checked.insert(path);
                                    }
                                    cx.notify();
                                });
                            },
                        ))
                        .child(ui::status_badge(&entry.status_code, pal))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .when(italic, |this| this.italic())
                                .child(entry.path.clone()),
                        )
                        .on_click({
                            let entity = entity_for_diff;
                            let path = entry.path.clone();
                            move |_, _, cx| {
                                entity.update(cx, |app, cx| {
                                    app.workspace.load_diff(path.clone(), cx);
                                });
                            }
                        })
                })
                .collect::<Vec<_>>()
        })
        .flex_1()
        .min_h_0()
        .into_any_element()
    }

    fn commit_box(
        &mut self,
        workspace: &str,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let busy = self.op_running;
        let can_commit = !busy && !self.checked.is_empty() && !self.message.is_empty();
        let workspace = workspace.to_string();
        let selected: Vec<String> = self.checked.iter().cloned().collect();
        let unversioned: Vec<String> = selected
            .iter()
            .filter(|file| self.code_of(file) == Some("unversioned"))
            .cloned()
            .collect();
        let message = self.message.text().trim().to_string();

        let selection = self.selection();
        let codes: Vec<(String, Option<String>)> = selection
            .iter()
            .map(|file| (file.clone(), self.code_of(file).map(str::to_string)))
            .collect();

        let (add_ws, add_files) = (workspace.clone(), selection.clone());
        let revert_files = selection.clone();
        let delete_codes = codes.clone();
        let (stash_files, stash_len) = (selection.clone(), selection.len());
        let (commit_ws, commit_files) = (workspace.clone(), selected.clone());
        let (commit_unversioned, commit_message) = (unversioned, message);
        let commit_count = self.checked.len();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .child(self.message.textarea(window, cx).h(px(70.0)).w_full())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(ui::button(
                        "ws-commit",
                        format!("✔ {} ({commit_count})", t("提交", "Commit")),
                        pal,
                        can_commit,
                        cx.listener(move |app, _, _, cx| {
                            let files = commit_files.clone();
                            let unversioned = commit_unversioned.clone();
                            let message = commit_message.clone();
                            let path = commit_ws.clone();
                            let auto_delete_missing = app.shared.settings.auto_delete_missing;
                            app.workspace.run_op(
                                "commit",
                                async move {
                                    if !unversioned.is_empty() {
                                        svn::add(&path, &unversioned)
                                            .await
                                            .map_err(|error| error.to_string())?;
                                    }
                                    svn::commit_with_options(
                                        &path,
                                        &message,
                                        Some(&files),
                                        auto_delete_missing,
                                    )
                                    .await
                                    .map_err(|error| error.to_string())
                                },
                                cx,
                            );
                        }),
                    ))
                    .child(ui::button(
                        "ws-add",
                        t("添加", "Add"),
                        pal,
                        !busy && !add_files.is_empty(),
                        cx.listener(move |app, _, _, cx| {
                            let path = add_ws.clone();
                            let files = add_files.clone();
                            app.workspace.run_op(
                                "add",
                                async move {
                                    svn::add(&path, &files)
                                        .await
                                        .map_err(|error| error.to_string())
                                },
                                cx,
                            );
                        }),
                    ))
                    .child(ui::button(
                        "ws-revert",
                        t("还原", "Revert"),
                        pal,
                        !busy && !revert_files.is_empty(),
                        cx.listener(move |app, _, _, cx| {
                            app.workspace.confirm = Some(Confirm::Revert(revert_files.clone()));
                            cx.notify();
                        }),
                    ))
                    .child(ui::button(
                        "ws-delete",
                        t("删除", "Delete"),
                        pal,
                        !busy && !delete_codes.is_empty(),
                        cx.listener(move |app, _, _, cx| {
                            let (unversioned, versioned): (Vec<_>, Vec<_>) = delete_codes
                                .iter()
                                .cloned()
                                .partition(|(_, code)| code.as_deref() == Some("unversioned"));
                            app.workspace.confirm = Some(if !versioned.is_empty() {
                                Confirm::Remove(
                                    versioned.into_iter().map(|(path, _)| path).collect(),
                                )
                            } else {
                                Confirm::DeleteUnversioned(
                                    unversioned.into_iter().map(|(path, _)| path).collect(),
                                )
                            });
                            cx.notify();
                        }),
                    ))
                    .child(ui::button(
                        "ws-stash",
                        t("存入贮藏", "Stash"),
                        pal,
                        !busy && stash_len > 0,
                        cx.listener(move |app, _, _, cx| {
                            app.shared.stash_files = Some(stash_files.clone());
                            app.shared.goto = Some(super::Page::Stash);
                            cx.notify();
                        }),
                    )),
            )
            .into_any_element()
    }

    fn diff_panel(
        &mut self,
        workspace: &str,
        _window: &Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let Some(diff) = self.diff.as_ref() else {
            let text = if self.focus.is_some() {
                t("正在加载差异…", "Loading diff…")
            } else {
                t("选择一个文件查看差异", "Select a file to view its diff")
            };
            return div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(text)
                .into_any_element();
        };
        let (added, removed) = diff.stats();
        let title = diff.title.clone();
        let title_for_display = title.clone();
        let title_for_blame = title.clone();
        let title_for_reveal = title.clone();
        let workspace = workspace.to_string();
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title_for_display),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(pal.ok))
                    .child(format!("+{added}")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(pal.err))
                    .child(format!("-{removed}")),
            )
            .child(ui::button(
                "ws-diff-blame",
                t("追溯", "Blame"),
                pal,
                true,
                cx.listener(move |app, _, _, cx| {
                    app.shared.blame_file = Some(title_for_blame.clone());
                    cx.notify();
                }),
            ))
            .child(ui::button(
                "ws-diff-reload",
                "🔄",
                pal,
                true,
                cx.listener(move |app, _, _, cx| {
                    app.workspace.load_diff(title.clone(), cx);
                    cx.notify();
                }),
            ))
            .child(ui::button(
                "ws-diff-reveal",
                t("在文件管理器中显示", "Reveal in file manager"),
                pal,
                true,
                cx.listener({
                    let workspace = workspace.clone();
                    let file = title_for_reveal;
                    move |app, _, _, cx| {
                        if let Err(error) =
                            system::reveal_workspace_file(workspace.clone(), file.clone())
                        {
                            app.shared.err(error);
                            cx.notify();
                        }
                    }
                }),
            ))
            .when(self.diff_running, |this| this.child(ui::spinner(pal, "")));

        let body = if diff.is_empty() {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t(
                    "没有文本差异（可能是属性或二进制变更）",
                    "No textual changes (property or binary change)",
                ))
                .into_any_element()
        } else {
            diff.render(pal)
        };
        let _ = workspace;

        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .child(ui::sep(pal))
            .child(body)
            .into_any_element()
    }

    fn confirm_modal(
        &mut self,
        workspace: &str,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> Option<AnyElement> {
        let (title, files) = match self.confirm.as_ref()? {
            Confirm::Revert(files) => (
                t(
                    "还原以下文件的本地修改？",
                    "Revert local changes to these files?",
                ),
                files.clone(),
            ),
            Confirm::Remove(files) => (
                t(
                    "从版本控制中删除以下文件？",
                    "Schedule these files for deletion?",
                ),
                files.clone(),
            ),
            Confirm::DeleteUnversioned(files) => (
                t(
                    "永久删除以下未版本文件？",
                    "Permanently delete these unversioned files?",
                ),
                files.clone(),
            ),
        };
        let workspace = workspace.to_string();
        Some(
            div()
                .absolute()
                .inset_0()
                .occlude()
                .bg(gpui_kit::rgba(0x00000088))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(460.0))
                        .max_h(px(420.0))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .rounded_lg()
                        .border_1()
                        .border_color(rgb(pal.border))
                        .bg(rgb(pal.panel))
                        .shadow_lg()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .id("ws-confirm-files")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .font_family(ui::mono_font())
                                .text_xs()
                                .children(
                                    files
                                        .iter()
                                        .map(|file| div().whitespace_normal().child(file.clone())),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .child(ui::button(
                                    "confirm-yes",
                                    t("确认", "Confirm"),
                                    pal,
                                    true,
                                    cx.listener(move |app, _, _, cx| {
                                        app.workspace.confirm_selected(true, workspace.clone(), cx);
                                    }),
                                ))
                                .child(ui::button(
                                    "confirm-no",
                                    t("取消", "Cancel"),
                                    pal,
                                    true,
                                    cx.listener(|app, _, _, cx| {
                                        app.workspace.confirm = None;
                                        cx.notify();
                                    }),
                                )),
                        ),
                )
                .into_any_element(),
        )
    }

    fn confirm_selected(&mut self, answer: bool, workspace: String, cx: &mut Context<OrcaApp>) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        if !answer {
            cx.notify();
            return;
        }
        match confirm {
            Confirm::Revert(files) => self.run_op(
                "revert",
                async move {
                    svn::revert(&workspace, &files)
                        .await
                        .map_err(|error| error.to_string())
                },
                cx,
            ),
            Confirm::Remove(files) => self.run_op(
                "delete",
                async move {
                    svn::delete(&workspace, &files)
                        .await
                        .map_err(|error| error.to_string())
                },
                cx,
            ),
            Confirm::DeleteUnversioned(files) => {
                self.run_op(
                    "rm",
                    async move {
                        system::delete_unversioned(&workspace, &files).map(|_| files.join("\n"))
                    },
                    cx,
                )
            }
        }
    }
}
