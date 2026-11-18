//! Dedicated commit page (mirrors `main`'s `CommitView`): pick files, write the
//! message, preview the diff, commit.

use super::Shared;
use crate::app::OrcaApp;
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::cache_dir;
use crate::svn;
use crate::task;
use crate::ui::{self, Palette, TextArea};
use crate::SvnStatus;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::BTreeSet;

pub struct CommitPage {
    loaded_for: Option<String>,
    entries: Vec<SvnStatus>,
    status_running: bool,
    checked: BTreeSet<String>,
    focus: Option<String>,
    message: TextArea,
    diff: Option<DiffDocument>,
    diff_running: bool,
    op_running: bool,
}

impl CommitPage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        Self {
            loaded_for: None,
            entries: Vec::new(),
            status_running: false,
            checked: BTreeSet::new(),
            focus: None,
            message: TextArea::new(cx, window, t("提交信息…", "Commit message…"), 4),
            diff: None,
            diff_running: false,
            op_running: false,
        }
    }

    pub fn reset(&mut self) {
        self.loaded_for = None;
        self.entries.clear();
        self.checked.clear();
        self.focus = None;
        self.message.clear();
        self.diff = None;
        self.status_running = false;
        self.diff_running = false;
        self.op_running = false;
    }

    pub fn refresh(&mut self, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.loaded_for.clone() else {
            return;
        };
        if self.status_running {
            return;
        }
        self.status_running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::cached_status(&workspace, true, cache_dir())
                    .await
                    .map_err(|error| error.to_string());
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                app.commit.status_running = false;
                if app.commit.loaded_for.as_deref() != Some(workspace.as_str()) {
                    return;
                }
                match result {
                    Ok(mut entries) => {
                        entries.sort_by(|a, b| a.path.cmp(&b.path));
                        let paths: BTreeSet<_> =
                            entries.iter().map(|entry| entry.path.clone()).collect();
                        app.commit.checked.retain(|path| paths.contains(path));
                        if app
                            .commit
                            .focus
                            .as_ref()
                            .is_some_and(|focus| !paths.contains(focus))
                        {
                            app.commit.focus = None;
                            app.commit.diff = None;
                        }
                        app.commit.entries = entries;
                    }
                    Err(error) => app.shared.err(error),
                }
                cx.notify();
            },
        );
    }

    fn load_diff(&mut self, file: String, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.loaded_for.clone() else {
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
                app.commit.diff_running = false;
                if app.commit.focus.as_deref() != Some(file.as_str()) {
                    return;
                }
                match result {
                    Ok(diff) => app.commit.diff = Some(DiffDocument::new(diff.path, &diff.diff)),
                    Err(error) => app.shared.err(error),
                }
                cx.notify();
            },
        );
    }

    fn commit(&mut self, shared: &mut Shared, cx: &mut Context<OrcaApp>) {
        if self.op_running || self.checked.is_empty() || self.message.is_empty() {
            return;
        }
        let Some(workspace) = self.loaded_for.clone() else {
            return;
        };
        let files: Vec<String> = self.checked.iter().cloned().collect();
        let unversioned: Vec<String> = files
            .iter()
            .filter(|file| {
                self.entries
                    .iter()
                    .any(|entry| &entry.path == *file && entry.status_code == "unversioned")
            })
            .cloned()
            .collect();
        let message = self.message.text().trim().to_string();
        let auto_delete_missing = shared.settings.auto_delete_missing;
        self.op_running = true;
        task::spawn(
            cx,
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
            |app, result, cx| {
                app.commit.op_running = false;
                match result {
                    Ok(output) => {
                        app.shared.done("commit", &output);
                        app.commit.checked.clear();
                        app.commit.message.clear();
                        app.shared.info_dirty = true;
                        app.commit.refresh(cx);
                    }
                    Err(error) => app.shared.err(error),
                }
                cx.notify();
            },
        );
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
            self.refresh(cx);
        }

        let busy = self.op_running;
        let all =
            !self.entries.is_empty() && self.entries.iter().all(|e| self.checked.contains(&e.path));
        let (selected, total) = (self.checked.len(), self.entries.len());

        let mut rows = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            let path = entry.path.clone();
            let checked = self.checked.contains(&path);
            let active = self.focus.as_deref() == Some(path.as_str());
            let code = entry.status_code.clone();
            let entity = cx.entity();
            let entity_toggle = entity.clone();
            let entity_diff = entity.clone();
            let path_click = path.clone();
            let path_toggle = path.clone();
            rows.push(
                div()
                    .id(("commit-row", index))
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
                        ("commit-check", index),
                        checked,
                        pal,
                        move |_, _, cx| {
                            cx.stop_propagation();
                            entity_toggle.update(cx, |app, _| {
                                if app.commit.checked.contains(&path_toggle) {
                                    app.commit.checked.remove(&path_toggle);
                                } else {
                                    app.commit.checked.insert(path_toggle.clone());
                                }
                            });
                            cx.refresh_windows();
                        },
                    ))
                    .child(ui::status_badge(&code, pal))
                    .child(
                        div()
                            .id(("commit-path", index))
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .cursor_pointer()
                            .on_click(move |_, _, cx| {
                                let path = path_click.clone();
                                entity_diff.update(cx, |app, cx| app.commit.load_diff(path, cx));
                            })
                            .child(path),
                    )
                    .into_any_element(),
            );
        }

        let list = div()
            .id("commit-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .when(self.entries.is_empty() && !self.status_running, |this| {
                this.child(
                    div()
                        .p_6()
                        .text_sm()
                        .text_color(rgb(pal.muted))
                        .child(t("没有可提交的修改", "Nothing to commit")),
                )
            })
            .children(rows);

        let diff_body = match (&self.diff, &self.focus) {
            (Some(diff), _) if !diff.is_empty() => diff.render(pal),
            (Some(_), _) => div()
                .p_4()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t(
                    "没有文本差异（属性或二进制变更）",
                    "No textual changes (property or binary change)",
                ))
                .into_any_element(),
            (None, Some(_)) => div()
                .p_4()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t("正在加载差异…", "Loading diff…"))
                .into_any_element(),
            (None, None) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t("选择文件查看差异", "Select a file to see its diff"))
                .into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .child(ui::button(
                        "commit-refresh",
                        format!("🔄 {}", t("刷新", "Refresh")),
                        pal,
                        !busy,
                        cx.listener(|app, _, _, cx| app.commit.refresh(cx)),
                    ))
                    .child(ui::checkbox(
                        "commit-all",
                        all,
                        pal,
                        cx.listener(|app, _, _, cx| {
                            if app.commit.checked.len() == app.commit.entries.len() {
                                app.commit.checked.clear();
                            } else {
                                app.commit.checked =
                                    app.commit.entries.iter().map(|e| e.path.clone()).collect();
                            }
                            cx.notify();
                        }),
                    ))
                    .child(div().text_xs().text_color(rgb(pal.muted)).child(format!(
                        "{} {} · {} {}",
                        total,
                        t("项变更", "changes"),
                        selected,
                        t("已选", "selected")
                    )))
                    .when(busy, |this| this.child(ui::spinner(pal, "svn commit…"))),
            )
            .child(ui::sep(pal))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(div().flex_1().min_w_0().flex().flex_col().child(list))
                    .child(ui::sep_vertical(pal))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .child(diff_body),
                    ),
            )
            .child(ui::sep(pal))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .child(self.message.textarea(window, cx).h(px(84.0)).w_full())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(ui::button(
                                "commit-run",
                                format!("✔ {} ({selected})", t("提交", "Commit")),
                                pal,
                                !busy && selected > 0 && !self.message.is_empty(),
                                cx.listener(|app, _, _, cx| {
                                    let OrcaApp { commit, shared, .. } = app;
                                    commit.commit(shared, cx);
                                }),
                            ))
                            .when(busy, |this| this.child(ui::spinner(pal, ""))),
                    ),
            )
            .into_any_element()
    }
}
