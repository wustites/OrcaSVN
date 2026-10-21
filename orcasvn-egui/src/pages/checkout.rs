use super::{spinner_label, Page, Shared};
use crate::i18n::t;
use crate::svn;
use crate::task::Task;
use egui::Ui;

#[derive(Default)]
pub struct CheckoutPage {
    url: String,
    path: String,
    revision: String,
    task: Task<String>,
    target: String,
}

impl CheckoutPage {
    pub fn show(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let ctx = ui.ctx().clone();
        if let Some(result) = self.task.poll() {
            match result {
                Ok(output) => {
                    shared.done("checkout", &output);
                    shared.open_workspace = Some(self.target.clone());
                    shared.goto = Some(Page::Workspace);
                }
                Err(error) => shared.err(error),
            }
        }

        ui.heading(t("检出工作副本", "Check out a working copy"));
        ui.add_space(8.0);
        let busy = self.task.running();
        egui::Grid::new("checkout-form")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                ui.label(t("仓库 URL", "Repository URL"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.url)
                        .hint_text("https://svn.example.com/repo/trunk")
                        .desired_width(480.0),
                );
                ui.end_row();

                ui.label(t("本地目录", "Local directory"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(390.0));
                    if ui.button(t("浏览…", "Browse…")).clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.path = dir.to_string_lossy().into_owned();
                        }
                    }
                });
                ui.end_row();

                ui.label(t("版本", "Revision"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.revision)
                        .hint_text("HEAD")
                        .desired_width(120.0),
                );
                ui.end_row();
            });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let ready = !busy && !self.url.trim().is_empty() && !self.path.trim().is_empty();
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(format!("⬇ {}", t("检出", "Check out"))),
                )
                .clicked()
            {
                let revision = self.revision.trim().trim_start_matches('r');
                let revision = if revision.is_empty() {
                    None
                } else if let Ok(value) = revision.parse() {
                    Some(value)
                } else {
                    shared.err(t("无效的版本号", "Invalid revision"));
                    return;
                };
                let (url, path) = (self.url.trim().to_string(), self.path.trim().to_string());
                self.target = path.clone();
                self.task = Task::spawn(&ctx, async move {
                    svn::checkout(&url, &path, revision)
                        .await
                        .map_err(|e| e.to_string())
                });
            }
            spinner_label(ui, busy, t("正在检出…", "Checking out…"));
        });
    }
}
