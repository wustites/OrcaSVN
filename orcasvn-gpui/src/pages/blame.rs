use super::Shared;
use crate::app::OrcaApp;
use crate::i18n::t;
use crate::svn::{self, BlameLine};
use crate::task;
use crate::ui::{self, Palette, TextField};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct BlamePage {
    workspace: Option<String>,
    file: TextField,
    shown: String,
    lines: Vec<BlameLine>,
    running: bool,
    highlight: Option<u64>,
    _subs: Vec<Subscription>,
}

impl BlamePage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        let file = TextField::new(
            cx,
            window,
            t("工作副本内的文件路径", "File path inside the working copy"),
        );
        let subs = vec![ui::on_enter(cx, file.state(), |app, cx| app.blame.run(cx))];
        Self {
            workspace: None,
            file,
            shown: String::new(),
            lines: Vec::new(),
            running: false,
            highlight: None,
            _subs: subs,
        }
    }

    pub fn reset(&mut self) {
        self.shown.clear();
        self.lines.clear();
        self.highlight = None;
        self.running = false;
        self.file.clear();
    }

    pub fn open(&mut self, file: String, cx: &mut Context<OrcaApp>) {
        self.file.set_text(file);
        self.run(cx);
    }

    fn run(&mut self, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.workspace.clone() else {
            return;
        };
        let file = self.file.text().trim().replace('\\', "/");
        if file.is_empty() || self.running {
            return;
        }
        self.shown = file.clone();
        self.lines.clear();
        self.highlight = None;
        self.running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::blame(&workspace, &file)
                    .await
                    .map_err(|error| error.to_string());
                (file, result)
            },
            |app, (file, result), cx| {
                app.blame.running = false;
                if app.blame.shown != file {
                    return;
                }
                match result {
                    Ok(lines) => app.blame.lines = lines,
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
        self.workspace = shared.workspace.clone();
        let browse = ui::button(
            "blame-browse",
            t("浏览…", "Browse…"),
            pal,
            true,
            cx.listener(|app, _, _, cx| {
                let Some(workspace) = app.shared.workspace.clone() else {
                    return;
                };
                task::spawn_blocking(
                    cx,
                    move || {
                        let picked = rfd::FileDialog::new()
                            .set_directory(&workspace)
                            .pick_file()?;
                        let root = std::fs::canonicalize(&workspace)
                            .unwrap_or_else(|_| workspace.clone().into());
                        let picked = std::fs::canonicalize(&picked).unwrap_or(picked);
                        let relative = picked.strip_prefix(&root).ok()?;
                        Some(relative.to_string_lossy().replace('\\', "/"))
                    },
                    |app, picked, cx| {
                        match picked {
                            Some(file) => {
                                app.blame.file.set_text(file);
                                app.blame.run(cx);
                            }
                            None => {
                                // `None` also covers a file outside the working copy;
                                // the dialog itself returning nothing is harmless.
                            }
                        }
                        cx.notify();
                    },
                );
            }),
        );

        let input = div()
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
                    .child(self.file.input(window, cx).w_full()),
            )
            .child(browse)
            .child(ui::button(
                "blame-run",
                t("追溯", "Blame"),
                pal,
                !self.running,
                cx.listener(|app, _, _, cx| app.blame.run(cx)),
            ))
            .when(self.running, |this| {
                this.child(ui::spinner(pal, t("加载中…", "Loading…")))
            });

        let body = if self.lines.is_empty() {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(pal.muted))
                .child(if self.running {
                    t("正在加载…", "Loading…")
                } else {
                    t(
                        "输入文件路径以查看逐行作者",
                        "Enter a file to see who changed each line",
                    )
                })
                .into_any_element()
        } else {
            self.table(cx, pal)
        };

        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.shown.clone()),
            )
            .child(div().text_xs().text_color(rgb(pal.muted)).child(format!(
                "{} {}",
                self.lines.len(),
                t("行", "lines")
            )))
            .when_some(self.highlight, |this, revision| {
                this.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .text_xs()
                        .child(format!("· r{revision}"))
                        .child(ui::button(
                            "blame-clear-highlight",
                            "✖",
                            pal,
                            true,
                            cx.listener(|app, _, _, cx| {
                                app.blame.highlight = None;
                                cx.notify();
                            }),
                        )),
                )
            });

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(input)
            .child(ui::sep(pal))
            .when(!self.lines.is_empty(), |this| {
                this.child(header).child(ui::sep(pal))
            })
            .child(body)
            .into_any_element()
    }

    fn table(&mut self, cx: &mut Context<OrcaApp>, pal: Palette) -> AnyElement {
        let entity = cx.entity();
        let rows: Vec<(usize, u64, String, String)> = self
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                (
                    index,
                    line.revision,
                    line.author.clone(),
                    line.line.replace('\t', "    "),
                )
            })
            .collect();
        let highlight = self.highlight;
        let mono = ui::mono_font();
        uniform_list("blame-lines", rows.len(), move |range, _window, _cx| {
            range
                .map(|slot| {
                    let (index, revision, author, text) = &rows[slot];
                    let marked = highlight == Some(*revision);
                    let entity = entity.clone();
                    let revision = *revision;
                    div()
                        .id(("blame-row", *index))
                        .flex()
                        .flex_row()
                        .h(px(20.0))
                        .w_full()
                        .font_family(mono.clone())
                        .text_size(px(12.0))
                        .when(marked, |this| this.bg(rgb(pal.accent_soft)))
                        .hover(|style| style.bg(rgb(pal.elevated)))
                        .child(
                            div()
                                .w(px(52.0))
                                .flex_shrink_0()
                                .px_2()
                                .text_color(rgb(pal.muted))
                                .child((index + 1).to_string()),
                        )
                        .child(
                            div()
                                .id(("blame-rev", *index))
                                .w(px(76.0))
                                .flex_shrink_0()
                                .text_color(rgb(pal.accent))
                                .cursor_pointer()
                                .hover(|style| style.underline())
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |app, cx| {
                                        app.blame.highlight = Some(revision);
                                        cx.notify();
                                    });
                                })
                                .child(format!("r{revision}")),
                        )
                        .child(
                            div()
                                .w(px(120.0))
                                .flex_shrink_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(author.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(text.clone()),
                        )
                })
                .collect::<Vec<_>>()
        })
        .flex_1()
        .min_h_0()
        .into_any_element()
    }
}
