use super::{spinner_label, Shared};
use crate::i18n::t;
use crate::svn::{self, BlameLine};
use crate::task::Task;
use egui::{RichText, Ui};
use egui_extras::{Column, TableBuilder};
use std::path::Path;

#[derive(Default)]
pub struct BlamePage {
    file: String,
    shown: String,
    lines: Vec<BlameLine>,
    task: Task<Vec<BlameLine>>,
    highlight: Option<u64>,
}

impl BlamePage {
    pub fn open(&mut self, ctx: &egui::Context, shared: &Shared, file: String) {
        self.file = file;
        self.run(ctx, shared);
    }

    fn run(&mut self, ctx: &egui::Context, shared: &Shared) {
        let Some(workspace) = shared.workspace.clone() else {
            return;
        };
        let file = self.file.trim().replace('\\', "/");
        if file.is_empty() {
            return;
        }
        self.shown = file.clone();
        self.lines.clear();
        self.highlight = None;
        self.task = Task::spawn(ctx, async move {
            svn::blame(&workspace, &file)
                .await
                .map_err(|e| e.to_string())
        });
    }

    pub fn show(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let ctx = ui.ctx().clone();
        if let Some(result) = self.task.poll() {
            match result {
                Ok(lines) => self.lines = lines,
                Err(error) => shared.err(error),
            }
        }

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.file)
                    .hint_text(t(
                        "工作副本内的文件路径",
                        "File path inside the working copy",
                    ))
                    .desired_width(ui.available_width() - 200.0),
            );
            let submit = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button(t("浏览…", "Browse…")).clicked() {
                if let Some(workspace) = &shared.workspace {
                    if let Some(picked) =
                        rfd::FileDialog::new().set_directory(workspace).pick_file()
                    {
                        let root =
                            std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.into());
                        let picked = std::fs::canonicalize(&picked).unwrap_or(picked);
                        match picked.strip_prefix(&root) {
                            Ok(relative) => {
                                self.file = relative.to_string_lossy().replace('\\', "/");
                                self.run(&ctx, shared);
                            }
                            Err(_) => shared.err(t(
                                "文件必须位于当前工作副本内",
                                "The file must be inside the working copy",
                            )),
                        }
                    }
                }
            }
            if ui.button(t("追溯", "Blame")).clicked() || submit {
                self.run(&ctx, shared);
            }
            spinner_label(ui, self.task.running(), t("加载中…", "Loading…"));
        });
        ui.separator();

        if self.lines.is_empty() {
            if !self.task.running() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(t(
                            "输入文件路径以查看逐行作者",
                            "Enter a file to see who changed each line",
                        ))
                        .weak(),
                    );
                });
            }
            return;
        }

        ui.horizontal(|ui| {
            ui.strong(Path::new(&self.shown).display().to_string());
            ui.label(RichText::new(format!("{} {}", self.lines.len(), t("行", "lines"))).weak());
            if let Some(revision) = self.highlight {
                ui.label(format!("· r{revision}"));
                if ui.small_button("✖").clicked() {
                    self.highlight = None;
                }
            }
        });

        let accent = ui.visuals().selection.bg_fill.gamma_multiply(0.35);
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
        let mut pick = None;
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::exact(48.0))
            .column(Column::exact(70.0))
            .column(Column::initial(110.0).clip(true))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                for title in [
                    "#",
                    t("版本", "Rev"),
                    t("作者", "Author"),
                    t("内容", "Content"),
                ] {
                    header.col(|ui| {
                        ui.strong(title);
                    });
                }
            })
            .body(|body| {
                body.rows(row_height, self.lines.len(), |mut row| {
                    let index = row.index();
                    let line = &self.lines[index];
                    let mark = self.highlight == Some(line.revision);
                    let paint = |ui: &mut Ui| {
                        if mark {
                            ui.painter().rect_filled(ui.max_rect(), 0.0, accent);
                        }
                    };
                    row.col(|ui| {
                        paint(ui);
                        ui.label(RichText::new((index + 1).to_string()).monospace().weak());
                    });
                    row.col(|ui| {
                        paint(ui);
                        if ui
                            .link(RichText::new(format!("r{}", line.revision)).monospace())
                            .clicked()
                        {
                            pick = Some(line.revision);
                        }
                    });
                    row.col(|ui| {
                        paint(ui);
                        ui.label(&line.author);
                    });
                    row.col(|ui| {
                        paint(ui);
                        ui.add(
                            egui::Label::new(
                                RichText::new(line.line.replace('\t', "    ")).monospace(),
                            )
                            .extend(),
                        );
                    });
                });
            });
        if pick.is_some() {
            self.highlight = pick;
        }
    }
}
