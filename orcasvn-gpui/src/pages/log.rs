use super::Shared;
use crate::app::OrcaApp;
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::svn;
use crate::task;
use crate::ui::{self, Palette, TextField};
use crate::SvnLogEntry;
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct LogPage {
    loaded_for: Option<String>,
    entries: Vec<SvnLogEntry>,
    task_running: bool,
    appending: bool,
    exhausted: bool,
    keyword: TextField,
    author: TextField,
    selected: Option<usize>,
    diff: Option<DiffDocument>,
    diff_running: bool,
    limit: u32,
    _subs: Vec<Subscription>,
}

impl LogPage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        let keyword = TextField::new(
            cx,
            window,
            t("关键字 / 版本 / 路径", "Keyword / revision / path"),
        );
        let author = TextField::new(cx, window, t("作者", "Author"));
        let subs = vec![
            ui::on_enter(cx, keyword.state(), |app, cx| app.log.load(false, cx)),
            ui::on_enter(cx, author.state(), |app, cx| app.log.load(false, cx)),
        ];
        Self {
            loaded_for: None,
            entries: Vec::new(),
            task_running: false,
            appending: false,
            exhausted: false,
            keyword,
            author,
            selected: None,
            diff: None,
            diff_running: false,
            limit: 100,
            _subs: subs,
        }
    }

    pub fn reset(&mut self) {
        self.loaded_for = None;
        self.entries.clear();
        self.selected = None;
        self.diff = None;
        self.keyword.clear();
        self.author.clear();
        self.task_running = false;
        self.diff_running = false;
        self.exhausted = false;
    }

    fn load(&mut self, append: bool, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.loaded_for.clone() else {
            return;
        };
        if self.task_running {
            return;
        }
        let start = if append {
            match self.entries.last() {
                Some(last) if last.revision > 1 => Some(last.revision - 1),
                _ => {
                    self.exhausted = true;
                    return;
                }
            }
        } else {
            self.entries.clear();
            self.selected = None;
            self.diff = None;
            self.exhausted = false;
            None
        };
        self.appending = append;
        self.task_running = true;
        let limit = self.limit;
        let keyword = Some(self.keyword.text().trim().to_string()).filter(|s| !s.is_empty());
        let author = Some(self.author.text().trim().to_string()).filter(|s| !s.is_empty());
        task::spawn(
            cx,
            async move {
                let result = svn::log(
                    &workspace,
                    Some(limit),
                    start,
                    None,
                    keyword.as_deref(),
                    author.as_deref(),
                    None,
                    None,
                )
                .await
                .map_err(|error| error.to_string());
                (workspace, limit, append, result)
            },
            |app, (workspace, limit, append, result), cx| {
                app.log
                    .apply_log(&workspace, limit, append, result, &mut app.shared, cx)
            },
        );
    }

    fn apply_log(
        &mut self,
        workspace: &str,
        limit: u32,
        append: bool,
        result: Result<Vec<SvnLogEntry>, String>,
        shared: &mut Shared,
        cx: &mut Context<OrcaApp>,
    ) {
        self.task_running = false;
        self.appending = false;
        if self.loaded_for.as_deref() != Some(workspace) {
            return;
        }
        match result {
            Ok(entries) => {
                if entries.len() < limit as usize {
                    self.exhausted = true;
                }
                if append {
                    self.entries.extend(entries);
                } else {
                    self.entries = entries;
                }
            }
            Err(error) => shared.err(error),
        }
        cx.notify();
    }

    fn show_diff(
        &mut self,
        revision: u64,
        path: Option<String>,
        shared: &Shared,
        cx: &mut Context<OrcaApp>,
    ) {
        let (Some(workspace), Some(info)) = (shared.workspace.clone(), shared.info.as_ref()) else {
            return;
        };
        let root = info.repository_root.trim_end_matches('/').to_string();
        let target = match path {
            Some(path) => format!("{root}{path}"),
            None => root,
        };
        self.diff = None;
        self.diff_running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::diff(&workspace, &target, Some(revision), None)
                    .await
                    .map(|mut result| {
                        result.path = format!("r{revision} {}", result.path);
                        result
                    })
                    .map_err(|error| error.to_string());
                result
            },
            |app, result, cx| {
                app.log.diff_running = false;
                match result {
                    Ok(diff) => app.log.diff = Some(DiffDocument::new(diff.path, &diff.diff)),
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
        self.limit = shared.settings.log_limit.max(1);
        if self.loaded_for.as_deref() != shared.workspace.as_deref() {
            self.reset();
            self.loaded_for = shared.workspace.clone();
            self.load(false, cx);
        }

        let search = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(
                div()
                    .w(px(260.0))
                    .child(self.keyword.input(window, cx).w_full()),
            )
            .child(
                div()
                    .w(px(140.0))
                    .child(self.author.input(window, cx).w_full()),
            )
            .child(ui::button(
                "log-search",
                format!("🔍 {}", t("搜索", "Search")),
                pal,
                true,
                cx.listener(|app, _, _, cx| app.log.load(false, cx)),
            ))
            .child(ui::button(
                "log-reset",
                t("重置", "Reset"),
                pal,
                true,
                cx.listener(|app, _, _, cx| {
                    app.log.keyword.clear();
                    app.log.author.clear();
                    app.log.load(false, cx);
                    cx.notify();
                }),
            ))
            .when(self.task_running, |this| {
                this.child(ui::spinner(pal, t("加载中…", "Loading…")))
            });

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(search)
            .child(ui::sep(pal))
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("log-split")
                        .child(
                            resizable_panel()
                                .size(px(520.0))
                                .size_range(px(320.0)..px(900.0))
                                .child(self.list(cx, pal)),
                        )
                        .child(resizable_panel().child(self.details(cx, pal))),
                ),
            )
            .into_any_element()
    }

    fn list(&mut self, cx: &mut Context<OrcaApp>, pal: Palette) -> AnyElement {
        let entity = cx.entity();
        let rows: Vec<(usize, String, String, String, bool)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let date = entry
                    .date
                    .get(..16)
                    .unwrap_or(&entry.date)
                    .replace('T', " ");
                (
                    index,
                    format!("r{}  ·  {}  ·  {}", entry.revision, entry.author, date),
                    entry.message.lines().next().unwrap_or_default().to_string(),
                    entry.author.clone(),
                    self.selected == Some(index),
                )
            })
            .collect();
        let exhausted = self.exhausted;
        let running = self.task_running;
        let empty = self.entries.is_empty() && !running;

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .id("log-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(
                        rows.into_iter()
                            .map(|(index, meta, summary, _author, active)| {
                                let entity = entity.clone();
                                div()
                                    .id(("log-row", index))
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .px_3()
                                    .py_2()
                                    .border_b_1()
                                    .border_color(rgb(pal.border))
                                    .when(active, |this| this.bg(rgb(pal.accent_soft)))
                                    .hover(|style| style.bg(rgb(pal.elevated)))
                                    .cursor_pointer()
                                    .on_click(move |_, _, cx| {
                                        entity.update(cx, |app, cx| {
                                            app.log.selected = Some(index);
                                            app.log.diff = None;
                                            cx.notify();
                                        });
                                    })
                                    .child(div().text_xs().text_color(rgb(pal.muted)).child(meta))
                                    .child(
                                        div()
                                            .text_sm()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .child(summary),
                                    )
                            }),
                    ),
            )
            .when(!self.entries.is_empty() && !exhausted, |this| {
                this.child(div().p_2().child(ui::button(
                    "log-more",
                    t("加载更多", "Load more"),
                    pal,
                    !running,
                    cx.listener(|app, _, _, cx| app.log.load(true, cx)),
                )))
            })
            .when(empty, |this| {
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(rgb(pal.muted))
                        .child(t("没有日志记录", "No log entries")),
                )
            })
            .into_any_element()
    }

    fn details(&mut self, cx: &mut Context<OrcaApp>, pal: Palette) -> AnyElement {
        let Some(entry) = self
            .selected
            .and_then(|index| self.entries.get(index))
            .cloned()
        else {
            return div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(t(
                    "选择一个版本查看详情",
                    "Select a revision to see details",
                ))
                .into_any_element();
        };

        let revision = entry.revision;
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(format!("r{revision}")),
            )
            .child(div().text_sm().child(entry.author.clone()))
            .child(
                div().text_xs().text_color(rgb(pal.muted)).child(
                    entry
                        .date
                        .get(..16)
                        .unwrap_or(&entry.date)
                        .replace('T', " "),
                ),
            )
            .child(ui::button(
                "log-diff-revision",
                t("查看整个版本的差异", "Diff whole revision"),
                pal,
                true,
                cx.listener(move |app, _, _, cx| {
                    app.log.show_diff(revision, None, &app.shared, cx);
                }),
            ))
            .when(self.diff_running, |this| this.child(ui::spinner(pal, "")));

        let paths = entry.changed_paths.clone();
        let paths = div()
            .id("log-paths")
            .max_h(px(140.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .px_3()
            .children(paths.into_iter().enumerate().map(|(path_index, changed)| {
                let code = match changed.action.as_str() {
                    "A" => "added",
                    "D" => "deleted",
                    "R" => "replaced",
                    _ => "modified",
                };
                let path = changed.path.clone();
                let entity = cx.entity();
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .py_1()
                    .child(ui::status_badge(code, pal))
                    .child(
                        div()
                            .id(("log-path", path_index))
                            .text_sm()
                            .text_color(rgb(pal.accent))
                            .cursor_pointer()
                            .hover(|style| style.underline())
                            .on_click(move |_, _, cx| {
                                let path = path.clone();
                                entity.update(cx, |app, cx| {
                                    app.log.show_diff(revision, Some(path), &app.shared, cx);
                                });
                            })
                            .child(changed.path),
                    )
            }));

        let message = div()
            .px_3()
            .py_2()
            .text_sm()
            .whitespace_normal()
            .child(entry.message.clone());

        let diff = self.diff.as_ref().map(|diff| {
            (
                diff.title.clone(),
                if diff.is_empty() {
                    None
                } else {
                    Some(diff.render(pal))
                },
            )
        });

        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .child(ui::sep(pal))
            .child(message)
            .child(ui::sep(pal))
            .child(paths)
            .child(ui::sep(pal))
            .when_some(diff, |this, (title, body)| {
                this.child(
                    div()
                        .px_3()
                        .py_1()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .when_some(body, |this, body| this.child(body))
            })
            .into_any_element()
    }
}
