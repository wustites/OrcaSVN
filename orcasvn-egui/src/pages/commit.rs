//! Dedicated commit page for the egui client (mirrors `main`'s CommitView).

use super::{spinner_label, status_badge, Shared};
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::cache_dir;
use crate::svn;
use crate::task::Task;
use crate::{DiffResult, SvnStatus};
use egui::{RichText, Ui};
use std::collections::BTreeSet;

#[derive(Default)]
pub struct CommitPage {
    loaded_for: Option<String>,
    scope: Option<String>,
    entries: Vec<SvnStatus>,
    checked: BTreeSet<String>,
    focus: Option<String>,
    status_task: Task<Vec<SvnStatus>>,
    diff_task: Task<DiffResult>,
    diff: Option<DiffDocument>,
    op_task: Task<String>,
    message: String,
}

impl CommitPage {
    pub fn set_scope(&mut self, scope: Option<String>) {
        self.scope = scope;
        self.checked.clear();
        self.focus = None;
        self.diff = None;
    }

    pub fn preselect(&mut self, ctx: &egui::Context, shared: &Shared, file: String) {
        self.checked.insert(file.clone());
        if let Some(workspace) = shared.workspace.clone() {
            self.load_diff(ctx, &workspace, &file);
        }
    }

    fn load(&mut self, ctx: &egui::Context, workspace: &str) {
        if self.status_task.running() {
            return;
        }
        let path = workspace.to_string();
        self.status_task = Task::spawn(ctx, async move {
            svn::cached_status(&path, true, cache_dir())
                .await
                .map_err(|e| e.to_string())
        });
    }

    fn load_diff(&mut self, ctx: &egui::Context, workspace: &str, file: &str) {
        self.focus = Some(file.to_string());
        let (workspace, file) = (workspace.to_string(), file.to_string());
        self.diff_task = Task::spawn(ctx, async move {
            svn::diff(&workspace, &file, None, None)
                .await
                .map_err(|e| e.to_string())
        });
    }

    fn visible(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| match self.scope.as_deref() {
                Some(scope) if !scope.is_empty() => {
                    entry.path == scope || entry.path.starts_with(&format!("{scope}/"))
                }
                _ => true,
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn poll(&mut self, ctx: &egui::Context, shared: &mut Shared, workspace: &str) {
        if let Some(result) = self.status_task.poll() {
            match result {
                Ok(entries) => {
                    let mut entries =
                        crate::settings::filter_gitignore(entries, &shared.settings, workspace);
                    entries.sort_by(|a, b| a.path.cmp(&b.path));
                    let paths: BTreeSet<_> = entries.iter().map(|e| e.path.clone()).collect();
                    self.checked.retain(|path| paths.contains(path));
                    if self.focus.as_ref().is_some_and(|f| !paths.contains(f)) {
                        self.focus = None;
                        self.diff = None;
                    }
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
        if let Some(result) = self.op_task.poll() {
            match result {
                Ok(output) => {
                    shared.done("commit", &output);
                    self.checked.clear();
                    self.message.clear();
                    shared.info_dirty = true;
                    self.load(ctx, workspace);
                }
                Err(error) => shared.err(error),
            }
        }
    }

    pub fn commit(&mut self, ctx: &egui::Context, shared: &Shared, workspace: &str) {
        if self.op_task.running() || self.checked.is_empty() || self.message.trim().is_empty() {
            return;
        }
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
        let message = self.message.trim().to_string();
        let path = workspace.to_string();
        let auto_delete_missing = shared.settings.auto_delete_missing;
        self.op_task = Task::spawn(ctx, async move {
            if !unversioned.is_empty() {
                svn::add(&path, &unversioned)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            svn::commit_with_options(&path, &message, Some(&files), auto_delete_missing)
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
                scope: shared.commit_scope.take(),
                ..Default::default()
            };
            self.load(&ctx, &workspace);
            if let Some(file) = shared.commit_file.take() {
                self.checked.insert(file.clone());
                self.load_diff(&ctx, &workspace, &file);
            }
        }
        self.poll(&ctx, shared, &workspace);

        let visible = self.visible();
        let directories: Vec<String> = visible
            .iter()
            .filter_map(|&index| {
                self.entries[index]
                    .path
                    .rsplit_once('/')
                    .map(|(dir, _)| dir.to_string())
            })
            .filter(|dir| !dir.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(8)
            .collect();

        ui.horizontal_wrapped(|ui| {
            if ui.button(format!("🔄 {}", t("刷新", "Refresh"))).clicked() {
                self.load(&ctx, &workspace);
            }
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    t("范围", "Scope"),
                    self.scope
                        .clone()
                        .filter(|scope| !scope.is_empty())
                        .unwrap_or_else(|| t("全部目录", "All directories").to_string())
                ))
                .weak(),
            );
            if self.scope.is_some() && ui.small_button(t("全部目录", "All directories")).clicked()
            {
                self.scope = None;
            }
            for directory in &directories {
                let active = self.scope.as_deref() == Some(directory.as_str());
                if ui
                    .add(egui::Button::new(format!("{directory}/")).selected(active))
                    .clicked()
                {
                    self.scope = Some(directory.clone());
                }
            }
            spinner_label(ui, self.op_task.running(), "svn commit…");
            spinner_label(
                ui,
                self.status_task.running() && !self.op_task.running(),
                t("正在扫描…", "Scanning…"),
            );
        });
        ui.separator();

        egui::Panel::left("commit-files")
            .resizable(true)
            .default_size(440.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let all_checked = !visible.is_empty()
                        && visible
                            .iter()
                            .all(|&index| self.checked.contains(&self.entries[index].path));
                    let mut all = all_checked;
                    if ui.checkbox(&mut all, "").changed() {
                        for &index in &visible {
                            let path = self.entries[index].path.clone();
                            if all {
                                self.checked.insert(path);
                            } else {
                                self.checked.remove(&path);
                            }
                        }
                    }
                    ui.label(format!(
                        "{} {} · {} {}",
                        self.entries.len(),
                        t("项变更", "changes"),
                        self.checked.len(),
                        t("已选", "selected")
                    ));
                });
                ui.separator();
                let mut clicked = None;
                egui::ScrollArea::vertical()
                    .id_salt("commit-list")
                    .max_height((ui.available_height() - 40.0).max(120.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for &index in &visible {
                            let entry = &self.entries[index];
                            ui.horizontal(|ui| {
                                let mut on = self.checked.contains(&entry.path);
                                if ui.checkbox(&mut on, "").changed() {
                                    if on {
                                        self.checked.insert(entry.path.clone());
                                    } else {
                                        self.checked.remove(&entry.path);
                                    }
                                }
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
                        if visible.is_empty() && !self.status_task.running() {
                            ui.label(
                                RichText::new(t("没有可提交的修改", "Nothing to commit")).weak(),
                            );
                        }
                    });
                if let Some(path) = clicked {
                    self.load_diff(&ctx, &workspace, &path);
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

        ui.separator();
        ui.add(
            egui::TextEdit::multiline(&mut self.message)
                .hint_text(t("提交信息…", "Commit message…"))
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            let ready = !self.op_task.running()
                && !self.checked.is_empty()
                && !self.message.trim().is_empty();
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(format!(
                        "✔ {} ({})",
                        t("提交", "Commit"),
                        self.checked.len()
                    )),
                )
                .clicked()
            {
                self.commit(&ctx, shared, &workspace);
            }
            if !self.checked.is_empty() && ui.small_button(t("清空选择", "Clear")).clicked() {
                self.checked.clear();
            }
        });
    }
}
