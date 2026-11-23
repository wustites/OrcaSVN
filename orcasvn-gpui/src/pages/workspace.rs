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
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::table::{Column, ColumnSort, DataTable, TableDelegate, TableState};
use gpui_kit::component::FocusableExt as _;
use gpui_kit::component::Theme as KitTheme;
use gpui_kit::component::WindowExt as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::BTreeSet;
use std::future::Future;
use std::time::{Duration, Instant};

const AUTO_REFRESH: Duration = Duration::from_secs(8);

#[derive(Clone)]
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
    update_rev: TextField,
    table: Entity<TableState<StatusDelegate>>,
    focus_filter_requested: bool,
    _subs: Vec<Subscription>,
}

/// Renders the status list through the kit's virtualized table. The delegate
/// keeps only row indices and reads the actual entries from the app entity, so
/// there is a single source of truth for selection and filtering.
struct StatusDelegate {
    app: WeakEntity<OrcaApp>,
    rows: Vec<usize>,
    sort: ColumnSort,
}

impl StatusDelegate {
    fn resort(&mut self, cx: &App) {
        if self.sort == ColumnSort::Default {
            return;
        }
        let Some(app) = self.app.upgrade() else {
            return;
        };
        let app = app.read(cx);
        let path_of = |index: &usize| {
            app.workspace
                .entries
                .get(*index)
                .map(|entry| entry.path.clone())
                .unwrap_or_default()
        };
        self.rows.sort_by_key(path_of);
        if self.sort == ColumnSort::Descending {
            self.rows.reverse();
        }
    }
}

impl TableDelegate for StatusDelegate {
    fn columns_count(&self, _: &App) -> usize {
        3
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        match col_ix {
            0 => Column::new("check", "")
                .width(px(36.0))
                .resizable(false)
                .movable(false)
                .selectable(false),
            1 => Column::new("status", t("状态", "Status"))
                .width(px(52.0))
                .resizable(false)
                .movable(false)
                .selectable(false),
            _ => Column::new("path", t("文件", "Path"))
                .width(px(380.0))
                .min_width(px(120.0))
                .sortable(),
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        if col_ix != 2 {
            return;
        }
        self.sort = sort;
        self.resort(cx);
        cx.notify();
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(entity) = self.app.upgrade() else {
            return div().into_any_element();
        };
        let app = entity.read(cx);
        let pal = ui::palette_from_theme(KitTheme::global(cx));
        let Some(&index) = self.rows.get(row_ix) else {
            return div().into_any_element();
        };
        let Some(entry) = app.workspace.entries.get(index) else {
            return div().into_any_element();
        };
        let path = entry.path.clone();

        match col_ix {
            0 => {
                let checked = app.workspace.checked.contains(&path);
                let entity = entity.clone();
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h_full()
                    .child(ui::checkbox(
                        ("dt-check", row_ix),
                        checked,
                        pal,
                        move |_, _, cx| {
                            cx.stop_propagation();
                            entity.update(cx, |app, _| {
                                if app.workspace.checked.contains(&path) {
                                    app.workspace.checked.remove(&path);
                                } else {
                                    app.workspace.checked.insert(path.clone());
                                }
                            });
                            cx.refresh_windows();
                        },
                    ))
                    .into_any_element()
            }
            1 => div()
                .flex()
                .items_center()
                .h_full()
                .child(ui::status_badge(&entry.status_code, pal))
                .into_any_element(),
            _ => {
                let active = app.workspace.focus.as_deref() == Some(path.as_str());
                let italic = entry.prop_status != "none" && entry.prop_status != "normal";
                let workspace = app.workspace.loaded_for.clone().unwrap_or_default();
                let entity_click = entity.clone();
                let path_click = path.clone();
                let entity_menu = entity.clone();
                let path_menu = path.clone();
                div()
                    .id(("dt-path", row_ix))
                    .flex()
                    .items_center()
                    .h_full()
                    .w_full()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_sm()
                    .when(active, |this| {
                        this.bg(rgb(pal.accent_soft))
                            .font_weight(FontWeight::SEMIBOLD)
                    })
                    .when(italic, |this| this.italic())
                    .cursor_pointer()
                    .on_click(move |_, _, cx| {
                        entity_click.update(cx, |app, cx| {
                            app.workspace.load_diff(path_click.clone(), cx);
                        });
                    })
                    .context_menu(move |menu, _window, _cx| {
                        let path = path_menu.clone();
                        let workspace = workspace.clone();
                        file_context_menu(menu, entity_menu.clone(), path, workspace)
                    })
                    .child(path)
                    .into_any_element()
            }
        }
    }
}

/// Shared context-menu entries for a working-copy file.
fn file_context_menu(
    menu: PopupMenu,
    entity: Entity<OrcaApp>,
    path: String,
    workspace: String,
) -> PopupMenu {
    let path_diff = path.clone();
    let entity_diff = entity.clone();
    let path_blame = path.clone();
    let entity_blame = entity.clone();
    let path_reveal = path.clone();
    let workspace_reveal = workspace.clone();
    let path_resolve = path.clone();
    let entity_resolve = entity.clone();
    let workspace_resolve = workspace.clone();
    let path_commit = path.clone();
    let entity_commit = entity.clone();
    let path_commit_dir = path.clone();
    let entity_commit_dir = entity.clone();
    menu.item(
        PopupMenuItem::new(t("查看差异", "Show diff")).on_click(move |_, _, cx| {
            entity_diff.update(cx, |app, cx| {
                app.workspace.load_diff(path_diff.clone(), cx);
            });
        }),
    )
    .item(
        PopupMenuItem::new(t("提交此文件", "Commit this file")).on_click(move |_, _, cx| {
            entity_commit.update(cx, |app, cx| {
                app.shared.commit_file = Some(path_commit.clone());
                app.shared.goto = Some(super::Page::Commit);
                cx.notify();
            });
        }),
    )
    .item(
        PopupMenuItem::new(t("提交此目录", "Commit this directory")).on_click(move |_, _, cx| {
            entity_commit_dir.update(cx, |app, cx| {
                let path = path_commit_dir.clone();
                let directory = path
                    .rsplit_once('/')
                    .map(|(dir, _)| dir.to_string())
                    .unwrap_or_default();
                app.shared.commit_scope = Some(directory);
                app.shared.goto = Some(super::Page::Commit);
                cx.notify();
            });
        }),
    )
    .item(
        PopupMenuItem::new(t("追溯", "Blame")).on_click(move |_, _, cx| {
            entity_blame.update(cx, |app, cx| {
                app.shared.blame_file = Some(path_blame.clone());
                app.shared.goto = Some(super::Page::Blame);
                cx.notify();
            });
        }),
    )
    .item(
        PopupMenuItem::new(t("在文件管理器中显示", "Reveal in file manager")).on_click(
            move |_, _, cx| {
                if let Err(error) =
                    system::reveal_workspace_file(workspace_reveal.clone(), path_reveal.clone())
                {
                    tracing::warn!(%error, "reveal failed");
                }
                cx.refresh_windows();
            },
        ),
    )
    .separator()
    .item(
        PopupMenuItem::new(t("标记冲突已解决", "Mark resolved")).on_click(move |_, _, cx| {
            entity_resolve.update(cx, |app, cx| {
                let path = path_resolve.clone();
                let workspace = workspace_resolve.clone();
                app.workspace.run_op(
                    "resolve",
                    async move {
                        svn::resolve(&workspace, &[path], "working")
                            .await
                            .map_err(|error| error.to_string())
                    },
                    cx,
                );
            });
        }),
    )
}

impl WorkspacePage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        let filter = TextField::new(cx, window, t("过滤文件…", "Filter files…"));
        let message = TextArea::new(cx, window, t("提交信息…", "Commit message…"), 3);
        let update_rev = TextField::new(cx, window, "HEAD");
        let subs = vec![ui::on_enter(cx, update_rev.state(), |app, cx| {
            app.workspace.submit_update(&mut app.shared, cx);
        })];
        let app = cx.entity().downgrade();
        let table = cx.new(|cx| {
            TableState::new(
                StatusDelegate {
                    app,
                    rows: Vec::new(),
                    sort: ColumnSort::Default,
                },
                window,
                cx,
            )
        });
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
            update_rev,
            table,
            focus_filter_requested: false,
            _subs: subs,
        }
    }

    /// Commit the checked files with the message in the textarea.
    pub fn commit(&mut self, shared: &mut Shared, cx: &mut Context<OrcaApp>) {
        if self.op_running || self.checked.is_empty() || self.message.is_empty() {
            return;
        }
        let Some(workspace) = self.workspace() else {
            return;
        };
        let files: Vec<String> = self.checked.iter().cloned().collect();
        let unversioned: Vec<String> = files
            .iter()
            .filter(|file| self.code_of(file) == Some("unversioned"))
            .cloned()
            .collect();
        let message = self.message.text().trim().to_string();
        let auto_delete_missing = shared.settings.auto_delete_missing;
        self.run_op(
            "commit",
            async move {
                if !unversioned.is_empty() {
                    svn::add(&workspace, &unversioned)
                        .await
                        .map_err(|error| error.to_string())?;
                }
                svn::commit_with_options(&workspace, &message, Some(&files), auto_delete_missing)
                    .await
                    .map_err(|error| error.to_string())
            },
            cx,
        );
    }

    /// Deferred focus requested by the global shortcut handler.
    pub fn change_count(&self) -> usize {
        self.entries.len()
    }

    pub fn request_filter_focus(&mut self) {
        self.focus_filter_requested = true;
    }

    /// Open the kit dialog for a destructive workspace action.
    fn open_confirm(&mut self, window: &mut Window, cx: &mut Context<OrcaApp>, confirm: Confirm) {
        let (title, files) = match &confirm {
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
        let body = files.join("\n");
        let entity = cx.entity();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let confirm = confirm.clone();
            let content_body = body.clone();
            dialog
                .title(title)
                .content(move |content, _window, _cx| {
                    content.child(
                        div()
                            .id("dialog-files")
                            .max_h(px(280.0))
                            .overflow_y_scroll()
                            .font_family(ui::mono_font())
                            .text_xs()
                            .whitespace_normal()
                            .child(content_body.clone()),
                    )
                })
                .footer(
                    div()
                        .flex()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .child(
                            Button::new("dialog-cancel")
                                .label(t("取消", "Cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("dialog-ok")
                                .primary()
                                .label(t("确认", "Confirm"))
                                .on_click(move |_, window, cx| {
                                    entity.update(cx, |app, cx| {
                                        app.workspace.run_confirm(confirm.clone(), cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }

    fn run_confirm(&mut self, confirm: Confirm, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.workspace() else {
            return;
        };
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
        self.op_running = false;
        self.status_running = false;
        self.diff_running = false;
        self.last_refresh = None;
    }

    fn workspace(&self) -> Option<String> {
        self.loaded_for.clone()
    }

    pub fn refresh(&mut self, cx: &mut Context<OrcaApp>, force: bool) {
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
            Ok(entries) => {
                let mut entries =
                    crate::settings::filter_gitignore(entries, &shared.settings, &workspace);
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

        if self.focus_filter_requested {
            self.focus_filter_requested = false;
            self.filter.focus(window, cx);
        }
        let visible = self.visible_indices();
        self.sync_table(&visible, cx);

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
                div().flex_1().min_h_0().child(
                    h_resizable("ws-split")
                        .child(
                            resizable_panel()
                                .size(px(470.0))
                                .size_range(px(300.0)..px(900.0))
                                .child(left),
                        )
                        .child(resizable_panel().child(right)),
                ),
            )
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
            .flex_wrap()
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

        let list = self.entry_list(pal);

        let commit_box = self.commit_box(workspace, window, cx, pal);

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(filter_row)
            .child(counts)
            .child(ui::sep(pal))
            .child(list)
            .child(ui::sep(pal))
            .child(commit_box)
            .into_any_element()
    }

    fn entry_list(&mut self, pal: Palette) -> AnyElement {
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
        div()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                DataTable::new(&self.table)
                    .stripe(false)
                    .bordered(false)
                    .focus_ring(false),
            )
            .into_any_element()
    }

    /// Keep the table's visible row indices in sync with the current filter.
    fn sync_table(&mut self, visible: &[usize], cx: &mut Context<OrcaApp>) {
        let sort = self.table.read(cx).delegate().sort;
        let mut rows = visible.to_vec();
        if sort != ColumnSort::Default {
            let entries = &self.entries;
            rows.sort_by_key(|index| {
                entries
                    .get(*index)
                    .map(|entry| entry.path.clone())
                    .unwrap_or_default()
            });
            if sort == ColumnSort::Descending {
                rows.reverse();
            }
        }
        if self.table.read(cx).delegate().rows == rows {
            return;
        }
        self.table
            .update(cx, |state, _| state.delegate_mut().rows = rows);
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
        let selection = self.selection();
        let codes: Vec<(String, Option<String>)> = selection
            .iter()
            .map(|file| (file.clone(), self.code_of(file).map(str::to_string)))
            .collect();

        let (add_ws, add_files) = (workspace.clone(), selection.clone());
        let revert_files = selection.clone();
        let delete_codes = codes.clone();
        let (stash_files, stash_len) = (selection.clone(), selection.len());
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
                        cx.listener(|app, _, _, cx| {
                            app.workspace.commit(&mut app.shared, cx);
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
                        cx.listener(move |app, _, window, cx| {
                            app.workspace.open_confirm(
                                window,
                                cx,
                                Confirm::Revert(revert_files.clone()),
                            );
                        }),
                    ))
                    .child(ui::button(
                        "ws-delete",
                        t("删除", "Delete"),
                        pal,
                        !busy && !delete_codes.is_empty(),
                        cx.listener(move |app, _, window, cx| {
                            let (unversioned, versioned): (Vec<_>, Vec<_>) = delete_codes
                                .iter()
                                .cloned()
                                .partition(|(_, code)| code.as_deref() == Some("unversioned"));
                            let confirm = if !versioned.is_empty() {
                                Confirm::Remove(
                                    versioned.into_iter().map(|(path, _)| path).collect(),
                                )
                            } else {
                                Confirm::DeleteUnversioned(
                                    unversioned.into_iter().map(|(path, _)| path).collect(),
                                )
                            };
                            app.workspace.open_confirm(window, cx, confirm);
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
}
