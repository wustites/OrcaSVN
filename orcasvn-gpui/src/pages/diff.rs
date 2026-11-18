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

#[derive(Clone, Copy, PartialEq, Eq)]
enum DiffMode {
    Working,
    Change,
    Between,
}

pub struct DiffPage {
    loaded_for: Option<String>,
    entries: Vec<SvnStatus>,
    status_running: bool,
    focus: Option<String>,
    diff: Option<DiffDocument>,
    running: bool,
    mode: DiffMode,
    change_rev: TextField,
    old_rev: TextField,
    new_rev: TextField,
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
            mode: DiffMode::Working,
            change_rev: TextField::new(cx, window, t("版本", "Revision")),
            old_rev: TextField::new(cx, window, t("起始版本", "From revision")),
            new_rev: TextField::new(cx, window, t("结束版本", "To revision")),
        }
    }

    pub fn reset(&mut self) {
        self.loaded_for = None;
        self.entries.clear();
        self.focus = None;
        self.diff = None;
        self.status_running = false;
        self.running = false;
        self.mode = DiffMode::Working;
        self.change_rev.clear();
        self.old_rev.clear();
        self.new_rev.clear();
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
        let parse = |text: String| -> Result<Option<u64>, ()> {
            let text = text.trim().trim_start_matches('r').to_string();
            if text.is_empty() {
                Ok(None)
            } else {
                text.parse::<u64>().map(Some).map_err(|_| ())
            }
        };
        let (old_rev, new_rev) = match self.mode {
            DiffMode::Working => (None, None),
            DiffMode::Change => match parse(self.change_rev.text()) {
                Ok(Some(rev)) => (Some(rev), None),
                _ => {
                    self.diff = None;
                    return;
                }
            },
            DiffMode::Between => match (parse(self.old_rev.text()), parse(self.new_rev.text())) {
                (Ok(Some(old)), Ok(Some(new))) if old <= new => (Some(old), Some(new)),
                _ => {
                    self.diff = None;
                    return;
                }
            },
        };
        self.focus = Some(file.clone());
        self.running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::diff(&workspace, &file, old_rev, new_rev)
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
                    .child(ui::button(
                        "diff-mode-working",
                        t("工作副本", "Working copy"),
                        pal,
                        self.mode != DiffMode::Working,
                        cx.listener(|app, _, _, cx| {
                            app.diff_page.mode = DiffMode::Working;
                            cx.notify();
                        }),
                    ))
                    .child(ui::button(
                        "diff-mode-change",
                        t("指定版本", "One revision"),
                        pal,
                        self.mode != DiffMode::Change,
                        cx.listener(|app, _, _, cx| {
                            app.diff_page.mode = DiffMode::Change;
                            cx.notify();
                        }),
                    ))
                    .child(ui::button(
                        "diff-mode-between",
                        t("版本比较", "Between revisions"),
                        pal,
                        self.mode != DiffMode::Between,
                        cx.listener(|app, _, _, cx| {
                            app.diff_page.mode = DiffMode::Between;
                            cx.notify();
                        }),
                    ))
                    .when(self.mode == DiffMode::Change, |this| {
                        this.child(
                            div()
                                .w(px(110.0))
                                .child(self.change_rev.input(window, cx).w_full()),
                        )
                    })
                    .when(self.mode == DiffMode::Between, |this| {
                        this.child(
                            div()
                                .w(px(120.0))
                                .child(self.old_rev.input(window, cx).w_full()),
                        )
                        .child(div().text_xs().text_color(rgb(pal.muted)).child("→"))
                        .child(
                            div()
                                .w(px(120.0))
                                .child(self.new_rev.input(window, cx).w_full()),
                        )
                    })
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
