use super::{confirm_modal, spinner_label, status_badge, Page, Shared};
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::settings::cache_dir;
use crate::svn;
use crate::system::{self, OpenTarget};
use crate::task::Task;
use crate::{DiffResult, SvnStatus};
use egui::{RichText, Ui};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

const AUTO_REFRESH: Duration = Duration::from_secs(8);

enum Confirm {
    Revert(Vec<String>),
    Remove(Vec<String>),
    DeleteUnversioned(Vec<String>),
}

#[derive(Default)]
pub struct WorkspacePage {
    loaded_for: Option<String>,
    entries: Vec<SvnStatus>,
    status_task: Task<Vec<SvnStatus>>,
    last_refresh: Option<Instant>,
    force_next: bool,
    checked: BTreeSet<String>,
    focus: Option<String>,
    filter: String,
    hide_unversioned: bool,
    diff: Option<DiffDocument>,
    diff_task: Task<DiffResult>,
    message: String,
    op_task: Task<String>,
    op_label: String,
    confirm: Option<Confirm>,
    update_rev: String,
}

impl WorkspacePage {
    fn refresh(&mut self, ctx: &egui::Context, workspace: &str, force: bool) {
        if self.status_task.running() {
            return;
        }
        let path = workspace.to_string();
        self.last_refresh = Some(Instant::now());
        self.status_task = Task::spawn(ctx, async move {
            svn::cached_status(&path, force, cache_dir())
                .await
                .map_err(|e| e.to_string())
        });
    }

    fn run_op<F>(&mut self, ctx: &egui::Context, label: &str, future: F)
    where
        F: std::future::Future<Output = Result<String, String>> + Send + 'static,
    {
        if self.op_task.running() {
            return;
        }
        self.op_label = label.to_string();
        self.op_task = Task::spawn(ctx, future);
    }

    fn load_diff(&mut self, ctx: &egui::Context, workspace: &str, file: &str) {
        let (workspace, file) = (workspace.to_string(), file.to_string());
        self.focus = Some(file.clone());
        self.diff_task = Task::spawn(ctx, async move {
            svn::diff(&workspace, &file, None, None)
                .await
                .map_err(|e| e.to_string())
        });
    }

    fn poll(&mut self, ctx: &egui::Context, shared: &mut Shared, workspace: &str) {
        if let Some(result) = self.status_task.poll() {
            match result {
                Ok(mut entries) => {
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
                    shared.done(&self.op_label, &output);
                    if self.op_label == "commit" {
                        self.message.clear();
                        self.checked.clear();
                    }
                }
                Err(error) => shared.err(error),
            }
            shared.info_dirty = true;
            self.force_next = true;
            self.refresh(ctx, workspace, true);
            if let Some(focus) = self.focus.clone() {
                self.load_diff(ctx, workspace, &focus);
            }
        }
    }

    fn selection(&self) -> Vec<String> {
        if !self.checked.is_empty() {
            self.checked.iter().cloned().collect()
        } else {
            self.focus.iter().cloned().collect()
        }
    }

    fn code_of(&self, path: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.status_code.as_str())
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
            self.refresh(&ctx, &workspace, false);
        }
        self.poll(&ctx, shared, &workspace);

        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if focused
            && !self.op_task.running()
            && self
                .last_refresh
                .is_none_or(|at| at.elapsed() >= AUTO_REFRESH)
        {
            self.refresh(&ctx, &workspace, false);
        }
        ctx.request_repaint_after(AUTO_REFRESH);

        self.toolbar(ui, &ctx, shared, &workspace);
        ui.separator();

        egui::Panel::left("ws-files")
            .resizable(true)
            .default_size(460.0)
            .size_range(280.0..=900.0)
            .show(ui, |ui| self.file_panel(ui, &ctx, shared, &workspace));

        egui::CentralPanel::default().show(ui, |ui| self.diff_panel(ui, &ctx, shared, &workspace));

        self.confirm_dialog(&ctx, &workspace);
    }

    fn toolbar(&mut self, ui: &mut Ui, ctx: &egui::Context, shared: &mut Shared, workspace: &str) {
        let busy = self.op_task.running();
        ui.horizontal_wrapped(|ui| {
            if ui.button(format!("🔄 {}", t("刷新", "Refresh"))).clicked() {
                self.refresh(ctx, workspace, true);
            }
            ui.add_enabled_ui(!busy, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.update_rev)
                        .desired_width(70.0)
                        .hint_text("HEAD"),
                );
                if ui.button(format!("⬇ {}", t("更新", "Update"))).clicked() {
                    let revision = self.update_rev.trim().trim_start_matches('r').parse().ok();
                    if revision.is_none() && !self.update_rev.trim().is_empty() {
                        shared.err(t("无效的版本号", "Invalid revision"));
                    } else {
                        let path = workspace.to_string();
                        self.run_op(ctx, "update", async move {
                            svn::update(&path, revision)
                                .await
                                .map_err(|e| e.to_string())
                        });
                    }
                }
                if ui.button(t("清理", "Cleanup")).clicked() {
                    let path = workspace.to_string();
                    self.run_op(ctx, "cleanup", async move {
                        svn::cleanup(&path).await.map_err(|e| e.to_string())
                    });
                }
            });
            ui.separator();
            for (label, target) in [
                (t("文件管理器", "File manager"), OpenTarget::Explorer),
                ("VS Code", OpenTarget::Vscode),
                (t("终端", "Terminal"), OpenTarget::Terminal),
            ] {
                if ui.button(label).clicked() {
                    if let Err(error) = system::open_workspace_target(workspace.to_string(), target)
                    {
                        shared.err(error);
                    }
                }
            }
            spinner_label(ui, busy, &format!("svn {}…", self.op_label));
            spinner_label(
                ui,
                self.status_task.running() && !busy,
                t("正在扫描…", "Scanning…"),
            );
        });
    }

    fn file_panel(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        shared: &mut Shared,
        workspace: &str,
    ) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text(t("过滤文件…", "Filter files…"))
                    .desired_width(ui.available_width() - 120.0),
            );
            ui.checkbox(&mut self.hide_unversioned, t("隐藏未版本", "Hide '?'"));
        });

        let filter = self.filter.to_lowercase();
        let visible: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !(self.hide_unversioned && e.status_code == "unversioned"))
            .filter(|(_, e)| filter.is_empty() || e.path.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();

        ui.horizontal(|ui| {
            let all = !visible.is_empty()
                && visible
                    .iter()
                    .all(|&i| self.checked.contains(&self.entries[i].path));
            let mut toggle = all;
            if ui.checkbox(&mut toggle, "").changed() {
                for &i in &visible {
                    let path = self.entries[i].path.clone();
                    if toggle {
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

        let commit_height = 170.0;
        let list_height = (ui.available_height() - commit_height).max(120.0);
        let row_height = ui.spacing().interact_size.y;
        let mut clicked = None;
        let mut context_action = None;
        egui::ScrollArea::vertical()
            .id_salt("ws-list")
            .max_height(list_height)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, visible.len(), |ui, range| {
                for &index in &visible[range] {
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
                        let mut text = RichText::new(&entry.path);
                        if entry.prop_status != "none" && entry.prop_status != "normal" {
                            text = text.italics();
                        }
                        let response = ui.add(egui::Button::selectable(active, text).truncate());
                        if response.clicked() {
                            clicked = Some(entry.path.clone());
                        }
                        response.context_menu(|ui| {
                            for (label, action) in [
                                (t("查看差异", "Show diff"), "diff"),
                                (t("追溯", "Blame"), "blame"),
                                (t("在文件管理器中显示", "Reveal in file manager"), "reveal"),
                                (t("标记冲突已解决", "Mark resolved"), "resolve"),
                            ] {
                                if ui.button(label).clicked() {
                                    context_action = Some((action, entry.path.clone()));
                                    ui.close();
                                }
                            }
                        });
                    });
                }
            });
        if let Some(path) = clicked {
            self.load_diff(ctx, workspace, &path);
        }
        match context_action {
            Some(("diff", path)) => self.load_diff(ctx, workspace, &path),
            Some(("blame", path)) => {
                shared.blame_file = Some(path);
                shared.goto = Some(Page::Blame);
            }
            Some(("reveal", path)) => {
                if let Err(error) = system::reveal_workspace_file(workspace.to_string(), path) {
                    shared.err(error);
                }
            }
            Some(("resolve", path)) => {
                let ws = workspace.to_string();
                self.run_op(ctx, "resolve", async move {
                    svn::resolve(&ws, &[path], "working")
                        .await
                        .map_err(|e| e.to_string())
                });
            }
            _ => {}
        }

        ui.separator();
        self.commit_box(ui, ctx, shared, workspace);
    }

    fn commit_box(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        shared: &mut Shared,
        workspace: &str,
    ) {
        let busy = self.op_task.running();
        let selection = self.selection();
        ui.add(
            egui::TextEdit::multiline(&mut self.message)
                .hint_text(t("提交信息…", "Commit message…"))
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal_wrapped(|ui| {
            let can_commit = !busy && !self.checked.is_empty() && !self.message.trim().is_empty();
            if ui
                .add_enabled(
                    can_commit,
                    egui::Button::new(format!(
                        "✔ {} ({})",
                        t("提交", "Commit"),
                        self.checked.len()
                    )),
                )
                .on_disabled_hover_text(t(
                    "勾选文件并填写提交信息",
                    "Check files and enter a message",
                ))
                .clicked()
            {
                let files: Vec<String> = self.checked.iter().cloned().collect();
                let unversioned: Vec<String> = files
                    .iter()
                    .filter(|f| self.code_of(f) == Some("unversioned"))
                    .cloned()
                    .collect();
                let (ws, message) = (workspace.to_string(), self.message.trim().to_string());
                self.run_op(ctx, "commit", async move {
                    if !unversioned.is_empty() {
                        svn::add(&ws, &unversioned)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    svn::commit(&ws, &message, Some(&files))
                        .await
                        .map_err(|e| e.to_string())
                });
            }
            ui.add_enabled_ui(!busy && !selection.is_empty(), |ui| {
                if ui.button(t("添加", "Add")).clicked() {
                    let ws = workspace.to_string();
                    let files = selection.clone();
                    self.run_op(ctx, "add", async move {
                        svn::add(&ws, &files).await.map_err(|e| e.to_string())
                    });
                }
                if ui.button(t("还原", "Revert")).clicked() {
                    self.confirm = Some(Confirm::Revert(selection.clone()));
                }
                if ui.button(t("删除", "Delete")).clicked() {
                    let (unversioned, versioned): (Vec<_>, Vec<_>) = selection
                        .iter()
                        .cloned()
                        .partition(|f| self.code_of(f) == Some("unversioned"));
                    if !versioned.is_empty() {
                        self.confirm = Some(Confirm::Remove(versioned));
                    } else {
                        self.confirm = Some(Confirm::DeleteUnversioned(unversioned));
                    }
                }
                if ui.button(t("存入贮藏", "Stash")).clicked() {
                    shared.stash_files = Some(selection.clone());
                    shared.goto = Some(Page::Stash);
                }
            });
        });
    }

    fn diff_panel(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        shared: &mut Shared,
        workspace: &str,
    ) {
        match (&self.diff, &self.focus) {
            (Some(diff), _) => {
                let (added, removed) = diff.stats();
                let title = diff.title.clone();
                let mut reload = false;
                ui.horizontal(|ui| {
                    ui.strong(&title);
                    ui.label(
                        RichText::new(format!("+{added}")).color(super::status_color("added")),
                    );
                    ui.label(
                        RichText::new(format!("-{removed}")).color(super::status_color("deleted")),
                    );
                    if ui.small_button(t("追溯", "Blame")).clicked() {
                        shared.blame_file = Some(title.clone());
                        shared.goto = Some(Page::Blame);
                    }
                    reload = ui.small_button("🔄").clicked();
                    spinner_label(ui, self.diff_task.running(), "");
                });
                ui.separator();
                if let Some(diff) = &self.diff {
                    if diff.is_empty() {
                        ui.label(
                            RichText::new(t(
                                "没有文本差异（可能是属性或二进制变更）",
                                "No textual changes (property or binary change)",
                            ))
                            .weak(),
                        );
                    } else {
                        diff.show(ui);
                    }
                }
                if reload {
                    self.load_diff(ctx, workspace, &title);
                }
            }
            (None, Some(_)) => {
                spinner_label(ui, true, t("正在加载差异…", "Loading diff…"));
            }
            (None, None) => {
                ui.centered_and_justified(|ui| {
                    let text = if self.entries.is_empty() && !self.status_task.running() {
                        t(
                            "工作副本没有本地修改",
                            "The working copy has no local changes",
                        )
                    } else {
                        t("选择一个文件查看差异", "Select a file to view its diff")
                    };
                    ui.label(RichText::new(text).weak());
                });
            }
        }
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context, workspace: &str) {
        let Some(confirm) = &self.confirm else { return };
        let (title, files) = match confirm {
            Confirm::Revert(files) => (
                t(
                    "还原以下文件的本地修改？",
                    "Revert local changes to these files?",
                ),
                files,
            ),
            Confirm::Remove(files) => (
                t(
                    "从版本控制中删除以下文件？",
                    "Schedule these files for deletion?",
                ),
                files,
            ),
            Confirm::DeleteUnversioned(files) => (
                t(
                    "永久删除以下未版本文件？",
                    "Permanently delete these unversioned files?",
                ),
                files,
            ),
        };
        let body = files.join("\n");
        let Some(answer) = confirm_modal(ctx, "ws-confirm", title, &body) else {
            return;
        };
        let confirm = self.confirm.take().expect("confirm present");
        if !answer {
            return;
        }
        let ws = workspace.to_string();
        match confirm {
            Confirm::Revert(files) => self.run_op(ctx, "revert", async move {
                svn::revert(&ws, &files).await.map_err(|e| e.to_string())
            }),
            Confirm::Remove(files) => self.run_op(ctx, "delete", async move {
                svn::delete(&ws, &files).await.map_err(|e| e.to_string())
            }),
            Confirm::DeleteUnversioned(files) => self.run_op(ctx, "rm", async move {
                system::delete_unversioned(&ws, &files).map(|_| files.join("\n"))
            }),
        }
    }
}
