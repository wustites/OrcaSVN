//! Standalone diff page for the egui client (mirrors `main`'s DiffView).

use super::{spinner_label, status_badge, Shared};
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::cache_dir;
use crate::svn;
use crate::task::Task;
use crate::{DiffResult, SvnStatus};
use egui::{RichText, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum DiffMode {
    #[default]
    Working,
    Change,
    Between,
}

#[derive(Default)]
pub struct DiffPage {
    loaded_for: Option<String>,
    entries: Vec<SvnStatus>,
    status_task: Task<Vec<SvnStatus>>,
    focus: Option<String>,
    diff: Option<DiffDocument>,
    diff_task: Task<DiffResult>,
    mode: DiffMode,
    change_rev: String,
    old_rev: String,
    new_rev: String,
}

impl DiffPage {
    fn load_status(&mut self, ctx: &egui::Context, workspace: &str) {
        if self.status_task.running() {
            return;
        }
        let path = workspace.to_string();
        self.status_task = Task::spawn(ctx, async move {
            svn::cached_status(&path, false, cache_dir())
                .await
                .map_err(|e| e.to_string())
        });
    }

    fn load(&mut self, ctx: &egui::Context, workspace: &str, file: &str) {
        let parse = |text: &str| -> Result<Option<u64>, ()> {
            let text = text.trim().trim_start_matches('r');
            if text.is_empty() {
                Ok(None)
            } else {
                text.parse::<u64>().map(Some).map_err(|_| ())
            }
        };
        let (old_rev, new_rev) = match self.mode {
            DiffMode::Working => (None, None),
            DiffMode::Change => match parse(&self.change_rev) {
                Ok(Some(rev)) => (Some(rev), None),
                _ => {
                    self.diff = None;
                    return;
                }
            },
            DiffMode::Between => match (parse(&self.old_rev), parse(&self.new_rev)) {
                (Ok(Some(old)), Ok(Some(new))) if old <= new => (Some(old), Some(new)),
                _ => {
                    self.diff = None;
                    return;
                }
            },
        };
        self.focus = Some(file.to_string());
        let (workspace, file) = (workspace.to_string(), file.to_string());
        self.diff_task = Task::spawn(ctx, async move {
            svn::diff(&workspace, &file, old_rev, new_rev)
                .await
                .map_err(|e| e.to_string())
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
            self.load_status(&ctx, &workspace);
        }

        if let Some(result) = self.status_task.poll() {
            match result {
                Ok(entries) => {
                    let mut entries =
                        crate::settings::filter_gitignore(entries, &shared.settings, &workspace);
                    entries.sort_by(|a, b| a.path.cmp(&b.path));
                    self.entries = entries;
                }
                Err(error) => shared.err(error),
            }
        }
        if let Some(result) = self.diff_task.poll() {
            match result {
                Ok(diff) => self.diff = Some(DiffDocument::new(diff.path, &diff.diff)),
                Err(error) => shared.err(error),
            }
        }

        ui.horizontal_wrapped(|ui| {
            if ui.button(format!("🔄 {}", t("刷新", "Refresh"))).clicked() {
                self.load_status(&ctx, &workspace);
            }
            if ui
                .add(
                    egui::Button::new(t("工作副本", "Working copy"))
                        .selected(self.mode == DiffMode::Working),
                )
                .clicked()
            {
                self.mode = DiffMode::Working;
            }
            if ui
                .add(
                    egui::Button::new(t("指定版本", "One revision"))
                        .selected(self.mode == DiffMode::Change),
                )
                .clicked()
            {
                self.mode = DiffMode::Change;
            }
            if ui
                .add(
                    egui::Button::new(t("版本比较", "Between revisions"))
                        .selected(self.mode == DiffMode::Between),
                )
                .clicked()
            {
                self.mode = DiffMode::Between;
            }
            match self.mode {
                DiffMode::Working => {}
                DiffMode::Change => {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.change_rev)
                            .hint_text(t("版本", "Revision"))
                            .desired_width(90.0),
                    );
                }
                DiffMode::Between => {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.old_rev)
                            .hint_text(t("起始版本", "From"))
                            .desired_width(90.0),
                    );
                    ui.label("→");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_rev)
                            .hint_text(t("结束版本", "To"))
                            .desired_width(90.0),
                    );
                }
            }
            if ui
                .add_enabled(
                    self.focus.is_some() && !self.diff_task.running(),
                    egui::Button::new(t("重新加载", "Reload")),
                )
                .clicked()
            {
                if let Some(focus) = self.focus.clone() {
                    self.load(&ctx, &workspace, &focus);
                }
            }
            spinner_label(ui, self.diff_task.running(), "");
        });
        ui.separator();

        egui::Panel::left("diff-files")
            .resizable(true)
            .default_size(420.0)
            .show(ui, |ui| {
                let mut clicked = None;
                egui::ScrollArea::vertical()
                    .id_salt("diff-list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for entry in &self.entries {
                            ui.horizontal(|ui| {
                                status_badge(ui, &entry.status_code);
                                let active = self.focus.as_deref() == Some(entry.path.as_str());
                                if ui
                                    .add(egui::Button::selectable(active, &entry.path).truncate())
                                    .clicked()
                                {
                                    clicked = Some(entry.path.clone());
                                }
                            });
                        }
                        if self.entries.is_empty() && !self.status_task.running() {
                            ui.label(
                                RichText::new(t(
                                    "工作副本没有本地修改",
                                    "The working copy has no local changes",
                                ))
                                .weak(),
                            );
                        }
                    });
                if let Some(path) = clicked {
                    self.load(&ctx, &workspace, &path);
                }
            });

        egui::CentralPanel::default().show(ui, |ui| match &self.diff {
            Some(diff) if !diff.is_empty() => {
                ui.strong(&diff.title);
                diff.show(ui);
            }
            Some(_) => {
                ui.label(
                    RichText::new(t(
                        "没有文本差异（属性或二进制变更）",
                        "No textual changes (property or binary change)",
                    ))
                    .weak(),
                );
            }
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(t("选择文件查看差异", "Select a file to see its diff"))
                            .weak(),
                    );
                });
            }
        });
    }
}
