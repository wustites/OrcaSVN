//! Multi-project rail for the egui client, mirroring `main`'s WorkspaceSidebar.

use super::Shared;
use crate::i18n::t;
use crate::settings::{cache_dir, Settings};
use crate::svn;
use crate::task::Task;
use egui::{RichText, Ui};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const SUMMARY_REFRESH: Duration = Duration::from_secs(60);

#[derive(Default, Clone, Copy)]
pub struct Summary {
    pub changed: Option<usize>,
    pub modified: Option<u64>,
}

#[derive(Default)]
pub struct Sidebar {
    summaries: HashMap<String, Summary>,
    task: Task<Vec<(String, Summary)>>,
    last_refresh: Option<Instant>,
    note_edit: Option<(String, String)>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn poll(&mut self) {
        if let Some(Ok(list)) = self.task.poll() {
            for (path, summary) in list {
                self.summaries.insert(path, summary);
            }
        }
    }

    fn refresh(&mut self, ctx: &egui::Context, settings: &Settings) {
        if self.task.running() {
            return;
        }
        let paths = settings.recent.clone();
        if paths.is_empty() {
            return;
        }
        self.last_refresh = Some(Instant::now());
        self.task = Task::spawn(ctx, async move {
            let mut out = Vec::new();
            for path in paths {
                let modified = svn::cached_latest_modified(&path).await.ok().flatten();
                let changed = svn::cached_status(&path, false, cache_dir())
                    .await
                    .ok()
                    .map(|entries| entries.len());
                out.push((path, Summary { changed, modified }));
            }
            Ok::<_, String>(out)
        });
    }

    fn sorted_paths(&self, settings: &Settings) -> Vec<String> {
        let mut paths = settings.recent.clone();
        let favorites = &settings.favorites;
        let changed_of = |path: &str| {
            self.summaries
                .get(path)
                .and_then(|summary| summary.changed)
                .unwrap_or(usize::MAX)
        };
        paths.sort_by(|a, b| {
            let favorite = |path: &str| favorites.iter().position(|item| item == path);
            match (favorite(a), favorite(b)) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => match settings.workspace_sort_mode.as_str() {
                    "name" => workspace_name(a)
                        .to_lowercase()
                        .cmp(&workspace_name(b).to_lowercase()),
                    "changed" => changed_of(a).cmp(&changed_of(b)),
                    _ => std::cmp::Ordering::Equal,
                },
            }
        });
        paths
    }

    pub fn show(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let ctx = ui.ctx().clone();
        self.poll();
        if self
            .last_refresh
            .is_none_or(|at| at.elapsed() >= SUMMARY_REFRESH)
        {
            self.refresh(&ctx, &shared.settings);
        }

        if shared.settings.workspace_list_collapsed {
            ui.vertical_centered(|ui| {
                if ui
                    .button("»")
                    .on_hover_text(t("展开项目栏", "Expand rail"))
                    .clicked()
                {
                    shared.settings.workspace_list_collapsed = false;
                    shared.settings.save();
                }
                for path in self.sorted_paths(&shared.settings) {
                    let name = workspace_name(&path);
                    let initial = name.chars().next().unwrap_or('?').to_string();
                    let active = shared.workspace.as_deref() == Some(path.as_str());
                    let button = egui::Button::new(initial)
                        .selected(active)
                        .min_size(egui::vec2(32.0, 26.0));
                    if ui.add(button).on_hover_text(&path).clicked() {
                        shared.open_workspace = Some(path);
                    }
                }
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.strong(format!(
                "🗂 {} ({})",
                t("工作区", "Workspaces"),
                shared.settings.recent.len()
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("«")
                    .on_hover_text(t("折叠项目栏", "Collapse rail"))
                    .clicked()
                {
                    shared.settings.workspace_list_collapsed = true;
                    shared.settings.save();
                }
                if ui
                    .add_enabled(!self.task.running(), egui::Button::new("🔄"))
                    .clicked()
                {
                    self.refresh(&ctx, &shared.settings);
                }
                let label = match shared.settings.workspace_sort_mode.as_str() {
                    "name" => t("⇅ 名称", "⇅ Name"),
                    "changed" => t("⇅ 变更", "⇅ Changes"),
                    _ => t("⇅ 手动", "⇅ Manual"),
                };
                if ui.small_button(label).clicked() {
                    shared.settings.workspace_sort_mode =
                        match shared.settings.workspace_sort_mode.as_str() {
                            "manual" => "name",
                            "name" => "changed",
                            _ => "manual",
                        }
                        .to_string();
                    shared.settings.save();
                }
            });
        });

        ui.horizontal(|ui| {
            if ui.button(format!("📂 {}", t("打开…", "Open…"))).clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    shared.open_workspace = Some(dir.to_string_lossy().into_owned());
                }
            }
        });
        ui.separator();

        let paths = self.sorted_paths(&shared.settings);
        let manual = shared.settings.workspace_sort_mode == "manual";
        let mut switch_to = None;
        let mut toggle_favorite = None;
        let mut edit_note = None;
        let mut move_path = None;
        egui::ScrollArea::vertical()
            .id_salt("sidebar-list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, path) in paths.iter().enumerate() {
                    let active = shared.workspace.as_deref() == Some(path.as_str());
                    let favorite = shared.settings.favorites.iter().any(|item| item == path);
                    let note = shared.settings.workspace_notes.get(path).cloned();
                    let summary = self.summaries.get(path).copied().unwrap_or_default();
                    egui::Frame::group(ui.style())
                        .fill(if active {
                            ui.visuals().selection.bg_fill
                        } else {
                            ui.visuals().faint_bg_color
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(active, workspace_name(path)).clicked() {
                                    switch_to = Some(path.clone());
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button(if favorite { "★" } else { "☆" })
                                            .on_hover_text(t("收藏", "Favorite"))
                                            .clicked()
                                        {
                                            toggle_favorite = Some(path.clone());
                                        }
                                        if ui
                                            .small_button("✎")
                                            .on_hover_text(t("备注", "Note"))
                                            .clicked()
                                        {
                                            edit_note = Some((
                                                path.clone(),
                                                note.clone().unwrap_or_default(),
                                            ));
                                        }
                                        if manual {
                                            if ui
                                                .add_enabled(
                                                    index + 1 < paths.len(),
                                                    egui::Button::new("↓").small(),
                                                )
                                                .clicked()
                                            {
                                                move_path = Some((path.clone(), 1isize));
                                            }
                                            if ui
                                                .add_enabled(
                                                    index > 0,
                                                    egui::Button::new("↑").small(),
                                                )
                                                .clicked()
                                            {
                                                move_path = Some((path.clone(), -1isize));
                                            }
                                        }
                                    },
                                );
                            });
                            let changed = summary
                                .changed
                                .map(|count| count.to_string())
                                .unwrap_or_else(|| "—".to_string());
                            let modified = summary
                                .modified
                                .map(format_modified)
                                .unwrap_or_else(|| "—".to_string());
                            ui.label(
                                RichText::new(format!(
                                    "{} {} · {modified}",
                                    t("变更", "changed"),
                                    changed
                                ))
                                .small()
                                .weak(),
                            );
                            if let Some(note) = &note {
                                ui.label(RichText::new(format!("✎ {note}")).small().weak());
                            }
                        });
                    ui.add_space(4.0);
                }
                if paths.is_empty() {
                    ui.label(
                        RichText::new(t(
                            "还没有最近打开的工作副本",
                            "No recent working copies yet",
                        ))
                        .weak(),
                    );
                }
            });

        if let Some(path) = switch_to {
            shared.open_workspace = Some(path);
        }
        if let Some(path) = toggle_favorite {
            if let Some(index) = shared
                .settings
                .favorites
                .iter()
                .position(|item| item == &path)
            {
                shared.settings.favorites.remove(index);
            } else {
                shared.settings.favorites.push(path);
            }
            shared.settings.save();
        }
        if let Some((path, note)) = edit_note {
            self.note_edit = Some((path, note));
        }
        if let Some((path, delta)) = move_path {
            if let Some(index) = shared.settings.recent.iter().position(|item| item == &path) {
                let target = index as isize + delta;
                if target >= 0 && (target as usize) < shared.settings.recent.len() {
                    shared.settings.recent.swap(index, target as usize);
                    shared.settings.save();
                }
            }
        }

        if let Some((path, note)) = &mut self.note_edit {
            let mut close = false;
            let mut save = false;
            egui::Window::new(t("编辑备注", "Edit note"))
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label(path.as_str());
                    ui.text_edit_multiline(note);
                    ui.horizontal(|ui| {
                        if ui.button(t("保存", "Save")).clicked() {
                            save = true;
                        }
                        if ui.button(t("取消", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
            if save {
                let trimmed = note.trim().to_string();
                if trimmed.is_empty() {
                    shared.settings.workspace_notes.remove(path);
                } else {
                    shared
                        .settings
                        .workspace_notes
                        .insert(path.clone(), trimmed);
                }
                shared.settings.save();
                close = true;
            }
            if close {
                self.note_edit = None;
            }
        }
    }
}

pub fn workspace_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn format_modified(millis: u64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_millis_opt(millis as i64) {
        chrono::LocalResult::Single(time) => time.format("%m-%d %H:%M").to_string(),
        _ => "—".to_string(),
    }
}
