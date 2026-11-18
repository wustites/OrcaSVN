//! Update page for the egui client (mirrors `main`'s UpdateView).

use super::{spinner_label, Shared};
use crate::i18n::t;
use crate::svn;
use crate::task::Task;
use crate::SvnLogEntry;
use egui::{RichText, Ui};

#[derive(Default)]
pub struct UpdatePage {
    loaded_for: Option<String>,
    revision: String,
    local: Option<u64>,
    remote: Option<u64>,
    incoming: Vec<SvnLogEntry>,
    local_task: Task<u64>,
    check_task: Task<(u64, Vec<SvnLogEntry>)>,
    op_task: Task<String>,
    output: Option<String>,
    conflicts: Vec<String>,
}

impl UpdatePage {
    fn load_local(&mut self, ctx: &egui::Context, workspace: &str) {
        let path = workspace.to_string();
        self.local_task = Task::spawn(ctx, async move {
            svn::local_revision(&path).await.map_err(|e| e.to_string())
        });
    }

    fn check(&mut self, ctx: &egui::Context, workspace: &str) {
        if self.check_task.running() {
            return;
        }
        let path = workspace.to_string();
        self.check_task = Task::spawn(ctx, async move {
            let info = svn::remote_info(&path).await.map_err(|e| e.to_string())?;
            let local = svn::local_revision(&path)
                .await
                .map_err(|e| e.to_string())?;
            let incoming = svn::incoming_log(&path, Some(50), info.revision, local)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((info.revision, incoming))
        });
    }

    pub fn show(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let ctx = ui.ctx().clone();
        let Some(workspace) = shared.workspace.clone() else {
            return;
        };
        if self.loaded_for.as_deref() != Some(workspace.as_str()) {
            *self = Self {
                loaded_for: Some(workspace.clone()),
                ..Default::default()
            };
            self.load_local(&ctx, &workspace);
        }

        if let Some(result) = self.local_task.poll() {
            match result {
                Ok(revision) => self.local = Some(revision),
                Err(error) => shared.err(error),
            }
        }
        if let Some(result) = self.check_task.poll() {
            match result {
                Ok((remote, incoming)) => {
                    self.remote = Some(remote);
                    self.incoming = incoming;
                }
                Err(error) => shared.err(error),
            }
        }
        if let Some(result) = self.op_task.poll() {
            match result {
                Ok(output) => {
                    self.conflicts = svn::parse_update_conflicts(&output);
                    shared.done("update", &output);
                    self.output = Some(output);
                    shared.info_dirty = true;
                    self.load_local(&ctx, &workspace);
                }
                Err(error) => shared.err(error),
            }
        }
        if shared.info_dirty {
            self.load_local(&ctx, &workspace);
        }

        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{} {}",
                t("本地版本", "Local"),
                self.local
                    .map(|r| format!("r{r}"))
                    .unwrap_or_else(|| "—".into())
            ));
            ui.label(format!(
                "{} {}",
                t("远端版本", "Remote"),
                self.remote
                    .map(|r| format!("r{r}"))
                    .unwrap_or_else(|| "—".into())
            ));
            ui.separator();
            ui.add(
                egui::TextEdit::singleline(&mut self.revision)
                    .hint_text("HEAD")
                    .desired_width(90.0),
            );
            if ui
                .add_enabled(
                    !self.op_task.running(),
                    egui::Button::new(format!("⬇ {}", t("更新", "Update"))),
                )
                .clicked()
            {
                let text = self.revision.trim().trim_start_matches('r').to_string();
                let revision = if text.is_empty() {
                    None
                } else {
                    match text.parse() {
                        Ok(value) => Some(value),
                        Err(_) => {
                            shared.err(t("无效的版本号", "Invalid revision"));
                            return;
                        }
                    }
                };
                let path = workspace.clone();
                self.op_task = Task::spawn(&ctx, async move {
                    svn::update(&path, revision)
                        .await
                        .map_err(|e| e.to_string())
                });
            }
            if ui
                .add_enabled(
                    !self.check_task.running(),
                    egui::Button::new(format!("🔍 {}", t("检查更新", "Check"))),
                )
                .clicked()
            {
                self.check(&ctx, &workspace);
            }
            spinner_label(ui, self.op_task.running(), "svn update…");
            spinner_label(
                ui,
                self.check_task.running(),
                t("查询远端…", "Checking remote…"),
            );
        });
        ui.separator();

        if !self.conflicts.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(220, 150, 40),
                format!(
                    "⚠ {}: {}",
                    t("更新完成，但存在冲突", "Update finished with conflicts"),
                    self.conflicts.join(", ")
                ),
            );
        }

        ui.label(
            RichText::new(t("待更新版本", "Incoming revisions"))
                .small()
                .weak(),
        );
        egui::ScrollArea::vertical()
            .id_salt("update-list")
            .max_height((ui.available_height() - 160.0).max(120.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in &self.incoming {
                    let date = entry
                        .date
                        .get(..16)
                        .unwrap_or(&entry.date)
                        .replace('T', " ");
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "r{}  ·  {}  ·  {date}",
                                entry.revision, entry.author
                            ))
                            .small()
                            .weak(),
                        );
                    });
                    ui.label(entry.message.lines().next().unwrap_or_default());
                    ui.separator();
                }
                if self.incoming.is_empty() && !self.check_task.running() {
                    ui.label(
                        RichText::new(t(
                            "没有检测到待更新版本，点击“检查更新”查询远端",
                            "No incoming revisions; press Check to query the remote",
                        ))
                        .weak(),
                    );
                }
            });

        if let Some(output) = &self.output {
            ui.separator();
            ui.strong(t("命令输出", "Command output"));
            egui::ScrollArea::vertical()
                .id_salt("update-output")
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(output).monospace()).wrap());
                });
        }
    }
}
