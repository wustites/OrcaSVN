//! Standalone diff page (mirrors `main`'s `DiffView`): choose a changed file,
//! optionally diff against a revision, inspect the result.

use super::Shared;
use crate::app::OrcaApp;
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::cache_dir;
use crate::svn;
use crate::task;
use crate::ui::{self, Palette, TextField};
use crate::SvnStatus;
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct DiffPage {
    loaded_for: Option<String>,
    entries: Vec<SvnStatus>,
    status_running: bool,
    focus: Option<String>,
    diff: Option<DiffDocument>,
    running: bool,
    revision: TextField,
}

impl DiffPage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        Self {
            loaded_for: None,
            entries: Vec::new(),
            status_running: false,
            focus: None,
            diff: None,
            running: false,
            revision: TextField::new(cx, window, t("版本（可选）", "Revision (optional)")),
        }
    }

    pub fn reset(&mut self) {
        self.loaded_for = None;
        self.entries.clear();
        self.focus = None;
        self.diff = None;
        self.status_running = false;
        self.running = false;
        self.revision.clear();
    }

    fn refresh(&mut self, cx: &mut Context<OrcaApp>) {
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
                let result = svn::cached_status(&workspace, false, cache_dir())
                    .await
                    .map_err(|error| error.to_string());
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                app.diff_page.status_running = false;
                if app.diff_page.loaded_for.as_deref() != Some(workspace.as_str()) {
                    return;
                }
                match result {
                    Ok(mut entries) => {
                        entries.sort_by(|a, b| a.path.cmp(&b.path));
                        app.diff_page.entries = entries;
                    }
                    Err(error) => app.shared.err(error),
                }
                cx.notify();
            },
        );
    }

    fn load(&mut self, file: String, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.loaded_for.clone() else {
            return;
        };
        let text = self.revision.text().trim().to_string();
        let revision = if text.is_empty() {
            None
        } else {
            match text.trim_start_matches('r').parse::<u64>() {
                Ok(value) => Some(value),
                Err(_) => {
                    self.diff = None;
                    return;
                }
            }
        };
        self.focus = Some(file.clone());
        self.running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::diff(&workspace, &file, revision, None)
                    .await
                    .map_err(|error| error.to_string());
                (file, result)
            },
            |app, (file, result), cx| {
                app.diff_page.running = false;
                if app.diff_page.focus.as_deref() != Some(file.as_str()) {
                    return;
                }
                match result {
                    Ok(diff) => app.diff_page.diff = Some(DiffDocument::new(diff.path, &diff.diff)),
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

        let entity = cx.entity();
        let rows = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let path = entry.path.clone();
                let code = entry.status_code.clone();
                let active = self.focus.as_deref() == Some(path.as_str());
                let entity = entity.clone();
                let path_click = path.clone();
                div()
                    .id(("diff-row", index))
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
                    .cursor_pointer()
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |app, cx| app.diff_page.load(path_click.clone(), cx));
                    })
                    .child(ui::status_badge(&code, pal))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(path),
                    )
            })
            .collect::<Vec<_>>();

        let body = match &self.diff {
            Some(diff) if !diff.is_empty() => diff.render(pal),
            Some(_) => div()
                .p_6()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t(
                    "没有文本差异（属性或二进制变更）",
                    "No textual changes (property or binary change)",
                ))
                .into_any_element(),
            None if self.focus.is_some() => div()
                .p_6()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t("正在加载差异…", "Loading diff…"))
                .into_any_element(),
            None => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t("选择一个文件查看差异", "Select a file to see its diff"))
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
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .child(ui::button(
                        "diff-refresh",
                        format!("🔄 {}", t("刷新", "Refresh")),
                        pal,
                        !self.status_running,
                        cx.listener(|app, _, _, cx| app.diff_page.refresh(cx)),
                    ))
                    .child(
                        div()
                            .w(px(170.0))
                            .child(self.revision.input(window, cx).w_full()),
                    )
                    .child(ui::button(
                        "diff-reload",
                        t("重新加载", "Reload"),
                        pal,
                        !self.running,
                        cx.listener(|app, _, _, cx| {
                            if let Some(focus) = app.diff_page.focus.clone() {
                                app.diff_page.load(focus, cx);
                            }
                        }),
                    ))
                    .when(self.running, |this| this.child(ui::spinner(pal, ""))),
            )
            .child(ui::sep(pal))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("diff-files")
                            .w(px(420.0))
                            .flex_shrink_0()
                            .min_h_0()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .children(rows),
                    )
                    .child(ui::sep_vertical(pal))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .child(body),
                    ),
            )
            .into_any_element()
    }
}
