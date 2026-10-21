use super::{confirm_modal, spinner_label, status_badge, Shared};
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::{load_stashes, save_stashes, StashEntry};
use crate::svn;
use crate::task::Task;
use crate::SvnStatus;
use egui::{RichText, Ui};
use std::collections::BTreeSet;

enum Action {
    Apply,
    Pop,
}

#[derive(Default)]
pub struct StashPage {
    loaded_for: Option<String>,
    stashes: Vec<StashEntry>,
    changes: Vec<SvnStatus>,
    status_task: Task<Vec<SvnStatus>>,
    checked: BTreeSet<String>,
    name: String,
    create_task: Task<(String, Vec<String>)>,
    apply_task: Task<String>,
    pending: Option<(Action, String)>,
    confirm_drop: Option<String>,
    preview: Option<DiffDocument>,
}

fn stashable(status: &SvnStatus) -> bool {
    matches!(
        status.status_code.as_str(),
        "modified" | "added" | "deleted" | "replaced"
    )
}

impl StashPage {
    fn reload_status(&mut self, ctx: &egui::Context, workspace: &str) {
        let path = workspace.to_string();
        self.status_task = Task::spawn(ctx, async move {
            svn::status(&path).await.map_err(|e| e.to_string())
        });
    }

    fn persist(&self) {
        let workspace = self.loaded_for.clone().unwrap_or_default();
        let mut all: Vec<_> = load_stashes()
            .into_iter()
            .filter(|s| s.workspace != workspace)
            .collect();
        all.extend(self.stashes.iter().cloned());
        save_stashes(&all);
    }

    pub fn show(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let ctx = ui.ctx().clone();
        let Some(workspace) = shared.workspace.clone() else {
            return;
        };
        if self.loaded_for.as_deref() != Some(workspace.as_str()) {
            *self = Self {
                loaded_for: Some(workspace.clone()),
                stashes: load_stashes()
                    .into_iter()
                    .filter(|s| s.workspace == workspace)
                    .collect(),
                ..Default::default()
            };
            self.reload_status(&ctx, &workspace);
        }
        if shared.stash_files.is_some() && !self.status_task.running() {
            self.reload_status(&ctx, &workspace);
        }
        self.poll(&ctx, shared, &workspace);

        egui::Panel::left("stash-create")
            .resizable(true)
            .default_size(420.0)
            .show(ui, |ui| self.create_panel(ui, &ctx, &workspace));
        egui::CentralPanel::default().show(ui, |ui| self.list_panel(ui, &ctx, &workspace));

        if let Some(id) = self.confirm_drop.clone() {
            let name = self
                .stashes
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            if let Some(answer) = confirm_modal(
                &ctx,
                "stash-drop",
                t("删除该贮藏？", "Drop this stash?"),
                &name,
            ) {
                self.confirm_drop = None;
                if answer {
                    self.stashes.retain(|s| s.id != id);
                    self.persist();
                }
            }
        }
    }

    fn poll(&mut self, ctx: &egui::Context, shared: &mut Shared, workspace: &str) {
        if let Some(result) = self.status_task.poll() {
            match result {
                Ok(changes) => {
                    self.changes = changes.into_iter().filter(stashable).collect();
                    if let Some(files) = shared.stash_files.take() {
                        self.checked = files.into_iter().collect();
                    }
                    let paths: BTreeSet<_> = self.changes.iter().map(|c| c.path.clone()).collect();
                    self.checked.retain(|p| paths.contains(p));
                }
                Err(error) => shared.err(error),
            }
        }
        if let Some(result) = self.create_task.poll() {
            match result {
                Ok((patch, files)) => {
                    let now = chrono::Local::now();
                    let name = if self.name.trim().is_empty() {
                        format!("stash {}", now.format("%Y-%m-%d %H:%M"))
                    } else {
                        self.name.trim().to_string()
                    };
                    self.stashes.insert(
                        0,
                        StashEntry {
                            id: now.timestamp_nanos_opt().unwrap_or_default().to_string(),
                            name,
                            workspace: workspace.to_string(),
                            created_at: now.to_rfc3339(),
                            patch,
                            files,
                        },
                    );
                    self.persist();
                    self.name.clear();
                    self.checked.clear();
                    shared.ok(t(
                        "已贮藏并还原所选文件",
                        "Stashed and reverted the selected files",
                    ));
                }
                Err(error) => shared.err(error),
            }
            shared.info_dirty = true;
            self.reload_status(ctx, workspace);
        }
        if let Some(result) = self.apply_task.poll() {
            match result {
                Ok(output) => {
                    shared.done("patch", &output);
                    if let Some((Action::Pop, id)) = &self.pending {
                        let id = id.clone();
                        self.stashes.retain(|s| s.id != id);
                        self.persist();
                    }
                }
                Err(error) => shared.err(error),
            }
            self.pending = None;
            self.reload_status(ctx, workspace);
        }
    }

    fn create_panel(&mut self, ui: &mut Ui, ctx: &egui::Context, workspace: &str) {
        ui.strong(t("新建贮藏", "New stash"));
        ui.label(
            RichText::new(t(
                "保存所选文件的修改为补丁，并还原这些文件。未版本文件不会被贮藏。",
                "Saves the selected changes as a patch and reverts them. Unversioned files are not stashed.",
            ))
            .weak(),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.name)
                .hint_text(t("贮藏名称（可选）", "Stash name (optional)")),
        );
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("stash-files")
            .max_height((ui.available_height() - 40.0).max(80.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for change in &self.changes {
                    ui.horizontal(|ui| {
                        let mut on = self.checked.contains(&change.path);
                        if ui.checkbox(&mut on, "").changed() {
                            if on {
                                self.checked.insert(change.path.clone());
                            } else {
                                self.checked.remove(&change.path);
                            }
                        }
                        status_badge(ui, &change.status_code);
                        ui.label(&change.path);
                    });
                }
                if self.changes.is_empty() && !self.status_task.running() {
                    ui.label(RichText::new(t("没有可贮藏的修改", "Nothing to stash")).weak());
                }
            });
        ui.horizontal(|ui| {
            let busy = self.create_task.running() || self.apply_task.running();
            if ui
                .add_enabled(
                    !busy && !self.checked.is_empty(),
                    egui::Button::new(format!("📦 {}", t("贮藏", "Stash"))),
                )
                .clicked()
            {
                let files: Vec<String> = self.checked.iter().cloned().collect();
                let ws = workspace.to_string();
                self.create_task = Task::spawn(ctx, async move {
                    let mut patch = String::new();
                    for file in &files {
                        let diff = svn::diff(&ws, file, None, None)
                            .await
                            .map_err(|e| e.to_string())?;
                        patch.push_str(&diff.diff);
                        if !patch.ends_with('\n') {
                            patch.push('\n');
                        }
                    }
                    if patch.trim().is_empty() {
                        return Err(t(
                            "所选文件没有文本差异",
                            "The selected files have no textual changes",
                        )
                        .to_string());
                    }
                    svn::revert(&ws, &files).await.map_err(|e| e.to_string())?;
                    Ok((patch, files))
                });
            }
            if ui.small_button("🔄").clicked() {
                self.reload_status(ctx, workspace);
            }
            spinner_label(ui, busy || self.status_task.running(), "");
        });
    }

    fn list_panel(&mut self, ui: &mut Ui, ctx: &egui::Context, workspace: &str) {
        ui.strong(format!(
            "{} ({})",
            t("贮藏列表", "Stashes"),
            self.stashes.len()
        ));
        ui.separator();
        let busy = self.apply_task.running() || self.create_task.running();
        let mut action = None;
        egui::ScrollArea::vertical()
            .id_salt("stash-list")
            .max_height(ui.available_height() * 0.45)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for stash in &self.stashes {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(&stash.name);
                            let date = stash
                                .created_at
                                .get(..16)
                                .unwrap_or(&stash.created_at)
                                .replace('T', " ");
                            ui.label(RichText::new(date).weak());
                            ui.label(
                                RichText::new(format!(
                                    "{} {}",
                                    stash.files.len(),
                                    t("个文件", "files")
                                ))
                                .weak(),
                            )
                            .on_hover_text(stash.files.join("\n"));
                        });
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(!busy, |ui| {
                                if ui.button(t("应用", "Apply")).clicked() {
                                    action = Some(("apply", stash.id.clone()));
                                }
                                if ui
                                    .button(t("弹出", "Pop"))
                                    .on_hover_text(t("应用并删除", "Apply and drop"))
                                    .clicked()
                                {
                                    action = Some(("pop", stash.id.clone()));
                                }
                            });
                            if ui.button(t("预览", "Preview")).clicked() {
                                action = Some(("preview", stash.id.clone()));
                            }
                            if ui.button(t("删除", "Drop")).clicked() {
                                action = Some(("drop", stash.id.clone()));
                            }
                        });
                    });
                }
                if self.stashes.is_empty() {
                    ui.label(RichText::new(t("暂无贮藏", "No stashes yet")).weak());
                }
            });

        if let Some((kind, id)) = action {
            let Some(stash) = self.stashes.iter().find(|s| s.id == id) else {
                return;
            };
            match kind {
                "preview" => {
                    self.preview = Some(DiffDocument::new(stash.name.clone(), &stash.patch))
                }
                "drop" => self.confirm_drop = Some(id),
                _ => {
                    let (ws, patch) = (workspace.to_string(), stash.patch.clone());
                    self.pending = Some((
                        if kind == "pop" {
                            Action::Pop
                        } else {
                            Action::Apply
                        },
                        id,
                    ));
                    self.apply_task = Task::spawn(ctx, async move {
                        svn::apply_patch(&ws, &patch, false)
                            .await
                            .map_err(|e| e.to_string())
                    });
                }
            }
        }

        if let Some(preview) = &self.preview {
            ui.separator();
            ui.strong(&preview.title);
            preview.show(ui);
        }
    }
}
