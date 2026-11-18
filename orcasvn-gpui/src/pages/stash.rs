use super::Shared;
use crate::app::OrcaApp;
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::{load_stashes, save_stashes, StashEntry};
use crate::svn;
use crate::task;
use crate::ui::{self, Palette, TextField};
use crate::SvnStatus;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::BTreeSet;

enum Action {
    Apply,
    Pop,
}

fn stashable(status: &SvnStatus) -> bool {
    matches!(
        status.status_code.as_str(),
        "modified" | "added" | "deleted" | "replaced"
    )
}

pub struct StashPage {
    workspace: Option<String>,
    loaded_for: Option<String>,
    stashes: Vec<StashEntry>,
    changes: Vec<SvnStatus>,
    status_running: bool,
    checked: BTreeSet<String>,
    name: TextField,
    create_running: bool,
    apply_running: bool,
    pending: Option<(Action, String)>,
    confirm_drop: Option<String>,
    preview: Option<DiffDocument>,
}

impl StashPage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        Self {
            workspace: None,
            loaded_for: None,
            stashes: Vec::new(),
            changes: Vec::new(),
            status_running: false,
            checked: BTreeSet::new(),
            name: TextField::new(cx, window, t("贮藏名称（可选）", "Stash name (optional)")),
            create_running: false,
            apply_running: false,
            pending: None,
            confirm_drop: None,
            preview: None,
        }
    }

    pub fn reset(&mut self) {
        self.workspace = None;
        self.loaded_for = None;
        self.stashes.clear();
        self.changes.clear();
        self.checked.clear();
        self.name.clear();
        self.preview = None;
        self.status_running = false;
        self.create_running = false;
        self.apply_running = false;
    }

    fn reload_status(&mut self, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.workspace.clone() else {
            return;
        };
        if self.status_running {
            return;
        }
        self.status_running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::status(&workspace)
                    .await
                    .map_err(|error| error.to_string());
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                app.stash
                    .apply_status(&workspace, result, &mut app.shared, cx)
            },
        );
    }

    fn apply_status(
        &mut self,
        workspace: &str,
        result: Result<Vec<SvnStatus>, String>,
        shared: &mut Shared,
        cx: &mut Context<OrcaApp>,
    ) {
        self.status_running = false;
        if self.loaded_for.as_deref() != Some(workspace) {
            return;
        }
        match result {
            Ok(changes) => {
                self.changes = changes.into_iter().filter(stashable).collect();
                if let Some(files) = shared.stash_files.take() {
                    self.checked = files.into_iter().collect();
                }
                let paths: BTreeSet<_> = self.changes.iter().map(|c| c.path.clone()).collect();
                self.checked.retain(|path| paths.contains(path));
            }
            Err(error) => shared.err(error),
        }
        cx.notify();
    }

    fn persist(&self) {
        let workspace = self.loaded_for.clone().unwrap_or_default();
        let mut all: Vec<_> = load_stashes()
            .into_iter()
            .filter(|stash| stash.workspace != workspace)
            .collect();
        all.extend(self.stashes.iter().cloned());
        save_stashes(&all);
    }

    fn create(&mut self, cx: &mut Context<OrcaApp>) {
        let (Some(workspace), false) = (self.workspace.clone(), self.create_running) else {
            return;
        };
        if self.checked.is_empty() {
            return;
        }
        self.create_running = true;
        let files: Vec<String> = self.checked.iter().cloned().collect();
        task::spawn(
            cx,
            async move {
                let result = build_stash(&workspace, &files).await;
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                app.stash
                    .apply_create(&workspace, result, &mut app.shared, cx)
            },
        );
    }

    fn apply_create(
        &mut self,
        workspace: &str,
        result: Result<(String, Vec<String>), String>,
        shared: &mut Shared,
        cx: &mut Context<OrcaApp>,
    ) {
        self.create_running = false;
        match result {
            Ok((patch, files)) => {
                let now = chrono::Local::now();
                let name = if self.name.is_empty() {
                    format!("stash {}", now.format("%Y-%m-%d %H:%M"))
                } else {
                    self.name.text().trim().to_string()
                };
                self.stashes.insert(
                    0,
                    StashEntry {
                        id: now.timestamp_nanos_opt().unwrap_or_default().to_string(),
                        name,
                        workspace: workspace.to_string(),
                        created_at: now.to_rfc3339(),
                        patch,
                        files,
                    },
                );
                self.persist();
                self.name.clear();
                self.checked.clear();
                shared.ok(t(
                    "已贮藏并还原所选文件",
                    "Stashed and reverted the selected files",
                ));
            }
            Err(error) => shared.err(error),
        }
        shared.info_dirty = true;
        self.reload_status(cx);
        cx.notify();
    }

    fn apply_one(&mut self, kind: Action, id: String, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.workspace.clone() else {
            return;
        };
        let Some(stash) = self.stashes.iter().find(|stash| stash.id == id) else {
            return;
        };
        if self.apply_running {
            return;
        }
        let patch = stash.patch.clone();
        self.apply_running = true;
        self.pending = Some((kind, id));
        task::spawn(
            cx,
            async move {
                let result = svn::apply_patch(&workspace, &patch, false)
                    .await
                    .map_err(|error| error.to_string());
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                app.stash
                    .apply_patch_result(&workspace, result, &mut app.shared, cx)
            },
        );
    }

    fn apply_patch_result(
        &mut self,
        workspace: &str,
        result: Result<String, String>,
        shared: &mut Shared,
        cx: &mut Context<OrcaApp>,
    ) {
        self.apply_running = false;
        match result {
            Ok(output) => {
                shared.done("patch", &output);
                if let Some((Action::Pop, id)) = &self.pending {
                    let id = id.clone();
                    self.stashes.retain(|stash| stash.id != id);
                    self.persist();
                }
            }
            Err(error) => shared.err(error),
        }
        self.pending = None;
        let _ = workspace;
        self.reload_status(cx);
        cx.notify();
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
        self.workspace = Some(workspace.clone());
        if self.loaded_for.as_deref() != Some(workspace.as_str()) {
            self.checked.clear();
            self.changes.clear();
            self.preview = None;
            self.loaded_for = Some(workspace.clone());
            self.stashes = load_stashes()
                .into_iter()
                .filter(|stash| stash.workspace == workspace)
                .collect();
            self.reload_status(cx);
        }
        if shared.stash_files.is_some() && !self.status_running {
            self.reload_status(cx);
        }

        let left = self.create_panel(window, cx, pal);
        let right = self.list_panel(window, cx, pal);
        let modal = self.drop_modal(cx, pal);

        div()
            .relative()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .child(left)
            .child(ui::sep_vertical(pal))
            .child(right)
            .when_some(modal, |this, modal| this.child(modal))
            .into_any_element()
    }

    fn create_panel(
        &mut self,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let busy = self.create_running || self.apply_running;
        let entity = cx.entity();
        let rows: Vec<(usize, String, bool)> = self
            .changes
            .iter()
            .enumerate()
            .map(|(index, change)| {
                (
                    index,
                    change.path.clone(),
                    self.checked.contains(&change.path),
                )
            })
            .collect();
        let codes: Vec<String> = self
            .changes
            .iter()
            .map(|change| change.status_code.clone())
            .collect();

        div()
            .w(px(440.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .min_h_0()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .child(t("新建贮藏", "New stash")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(pal.muted))
                    .whitespace_normal()
                    .child(t(
                        "保存所选文件的修改为补丁，并还原这些文件。未版本文件不会被贮藏。",
                        "Saves the selected changes as a patch and reverts them. Unversioned files are not stashed.",
                    )),
            )
            .child(self.name.input(window, cx).w_full())
            .child(ui::sep(pal))
            .child(
                div()
                    .id("stash-files")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(rows.into_iter().map(|(index, path, checked)| {
                        let entity = entity.clone();
                        let code = codes[index].clone();
                        let label = path.clone();
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .py_1()
                            .child(ui::checkbox(
                                ("stash-check", index),
                                checked,
                                pal,
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    entity
                                        .update(cx, |app, cx| {
                                            if app.stash.checked.contains(&path) {
                                                app.stash.checked.remove(&path);
                                            } else {
                                                app.stash.checked.insert(path.clone());
                                            }
                                            cx.notify();
                                        })
                                        ;
                                },
                            ))
                            .child(ui::status_badge(&code, pal))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .text_sm()
                                    .child(label),
                            )
                    }))
                    .when(self.changes.is_empty() && !self.status_running, |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(rgb(pal.muted))
                                .child(t("没有可贮藏的修改", "Nothing to stash")),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::button(
                        "stash-create",
                        format!("📦 {}", t("贮藏", "Stash")),
                        pal,
                        !busy && !self.checked.is_empty(),
                        cx.listener(move |app, _, _, cx| app.stash.create(cx)),
                    ))
                    .child(ui::button(
                        "stash-reload",
                        "🔄",
                        pal,
                        !busy,
                        cx.listener(|app, _, _, cx| app.stash.reload_status(cx)),
                    ))
                    .when(busy || self.status_running, |this| {
                        this.child(ui::spinner(pal, ""))
                    }),
            )
            .into_any_element()
    }

    fn list_panel(
        &mut self,
        _window: &Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let busy = self.apply_running || self.create_running;
        let entity = cx.entity();
        let stashes: Vec<(usize, String, String, String, usize, String)> = self
            .stashes
            .iter()
            .enumerate()
            .map(|(index, stash)| {
                let date = stash
                    .created_at
                    .get(..16)
                    .unwrap_or(&stash.created_at)
                    .replace('T', " ");
                (
                    index,
                    stash.id.clone(),
                    stash.name.clone(),
                    date,
                    stash.files.len(),
                    stash.files.join("\n"),
                )
            })
            .collect();
        let empty = self.stashes.is_empty();

        let list = div()
            .id("stash-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .children(
                stashes
                    .into_iter()
                    .map(|(index, id, name, date, count, files)| {
                        let entity = entity.clone();
                        let id_apply = id.clone();
                        let id_pop = id.clone();
                        let id_preview = id.clone();
                        let id_drop = id.clone();
                        div()
                            .id(("stash-item", index))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(pal.border))
                            .bg(rgb(pal.panel))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div().text_sm().font_weight(FontWeight::BOLD).child(name),
                                    )
                                    .child(div().text_xs().text_color(rgb(pal.muted)).child(date))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(pal.muted))
                                            .child(format!("{count} {}", t("个文件", "files"))),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(pal.muted))
                                            .whitespace_nowrap()
                                            .child(
                                                files.lines().take(1).collect::<Vec<_>>().join(""),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(ui::button(
                                        ("stash-apply", index),
                                        t("应用", "Apply"),
                                        pal,
                                        !busy,
                                        {
                                            let entity = entity.clone();
                                            move |_, _, cx| {
                                                entity.update(cx, |app, cx| {
                                                    app.stash.apply_one(
                                                        Action::Apply,
                                                        id_apply.clone(),
                                                        cx,
                                                    )
                                                });
                                            }
                                        },
                                    ))
                                    .child(ui::button(
                                        ("stash-pop", index),
                                        t("弹出", "Pop"),
                                        pal,
                                        !busy,
                                        {
                                            let entity = entity.clone();
                                            move |_, _, cx| {
                                                entity.update(cx, |app, cx| {
                                                    app.stash.apply_one(
                                                        Action::Pop,
                                                        id_pop.clone(),
                                                        cx,
                                                    )
                                                });
                                            }
                                        },
                                    ))
                                    .child(ui::button(
                                        ("stash-preview", index),
                                        t("预览", "Preview"),
                                        pal,
                                        true,
                                        {
                                            let entity = entity.clone();
                                            move |_, _, cx| {
                                                entity.update(cx, |app, cx| {
                                                    if let Some(stash) = app
                                                        .stash
                                                        .stashes
                                                        .iter()
                                                        .find(|stash| stash.id == id_preview)
                                                    {
                                                        app.stash.preview =
                                                            Some(DiffDocument::new(
                                                                stash.name.clone(),
                                                                &stash.patch,
                                                            ));
                                                    }
                                                    cx.notify();
                                                });
                                            }
                                        },
                                    ))
                                    .child(ui::button(
                                        ("stash-drop", index),
                                        t("删除", "Drop"),
                                        pal,
                                        true,
                                        {
                                            let entity = entity.clone();
                                            move |_, _, cx| {
                                                entity.update(cx, |app, cx| {
                                                    app.stash.confirm_drop = Some(id_drop.clone());
                                                    cx.notify();
                                                });
                                            }
                                        },
                                    )),
                            )
                    }),
            );

        let preview = self
            .preview
            .as_ref()
            .map(|doc| (doc.title.clone(), doc.render(pal)));

        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .min_h_0()
            .child(div().text_sm().font_weight(FontWeight::BOLD).child(format!(
                "{} ({})",
                t("贮藏列表", "Stashes"),
                self.stashes.len()
            )))
            .child(ui::sep(pal))
            .child(list)
            .when(empty, |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(rgb(pal.muted))
                        .child(t("暂无贮藏", "No stashes yet")),
                )
            })
            .when_some(preview, |this, (title, body)| {
                this.child(ui::sep(pal))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(body)
            })
            .into_any_element()
    }

    fn drop_modal(&mut self, cx: &mut Context<OrcaApp>, pal: Palette) -> Option<AnyElement> {
        let id = self.confirm_drop.clone()?;
        let name = self
            .stashes
            .iter()
            .find(|stash| stash.id == id)
            .map(|stash| stash.name.clone())
            .unwrap_or_default();
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
                        .w(px(380.0))
                        .flex()
                        .flex_col()
                        .gap_3()
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
                                .child(t("删除该贮藏？", "Drop this stash?")),
                        )
                        .child(div().text_sm().child(name))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .child(ui::button(
                                    "stash-drop-yes",
                                    t("确认", "Confirm"),
                                    pal,
                                    true,
                                    cx.listener(move |app, _, _, cx| {
                                        let Some(id) = app.stash.confirm_drop.take() else {
                                            return;
                                        };
                                        app.stash.stashes.retain(|stash| stash.id != id);
                                        app.stash.persist();
                                        cx.notify();
                                    }),
                                ))
                                .child(ui::button(
                                    "stash-drop-no",
                                    t("取消", "Cancel"),
                                    pal,
                                    true,
                                    cx.listener(|app, _, _, cx| {
                                        app.stash.confirm_drop = None;
                                        cx.notify();
                                    }),
                                )),
                        ),
                )
                .into_any_element(),
        )
    }
}

async fn build_stash(workspace: &str, files: &[String]) -> Result<(String, Vec<String>), String> {
    let mut patch = String::new();
    for file in files {
        let diff = svn::diff(workspace, file, None, None)
            .await
            .map_err(|error| error.to_string())?;
        patch.push_str(&diff.diff);
        if !patch.ends_with('\n') {
            patch.push('\n');
        }
    }
    if patch.trim().is_empty() {
        return Err(t(
            "所选文件没有文本差异",
            "The selected files have no textual changes",
        )
        .to_string());
    }
    svn::revert(workspace, files)
        .await
        .map_err(|error| error.to_string())?;
    Ok((patch, files.to_vec()))
}
