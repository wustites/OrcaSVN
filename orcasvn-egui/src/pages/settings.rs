use super::{spinner_label, Shared};
use crate::app::apply_appearance;
use crate::i18n::{t, Lang};
use crate::settings::{Settings, Theme};
use crate::svn;
use crate::task::Task;
use egui::{RichText, Ui};

pub struct SettingsPage {
    svn_path: String,
    svn_task: Task<String>,
    svn_version: Option<String>,
    note_edit: Option<(String, String)>,
}

impl SettingsPage {
    pub fn new(settings: &Settings) -> Self {
        Self {
            svn_path: settings.svn_path.clone(),
            svn_task: Task::default(),
            svn_version: None,
            note_edit: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let ctx = ui.ctx().clone();
        if self.svn_version.is_none() && !self.svn_task.running() {
            self.check_svn(&ctx, Some(shared.settings.svn_path.clone()));
        }
        if let Some(result) = self.svn_task.poll() {
            match result {
                Ok(version) => {
                    shared.settings.svn_path = self.svn_path.trim().to_string();
                    shared.settings.save();
                    self.svn_version = Some(version);
                }
                Err(error) => {
                    self.svn_version = Some(String::new());
                    shared.err(format!(
                        "{}: {error}",
                        t(
                            "无法使用该 SVN 可执行文件",
                            "Cannot use this SVN executable"
                        )
                    ));
                }
            }
        }

        let before = shared.settings.clone();
        let settings = &mut shared.settings;
        ui.heading(t("设置", "Settings"));
        ui.add_space(8.0);
        egui::Grid::new("settings-grid")
            .num_columns(2)
            .spacing([16.0, 12.0])
            .show(ui, |ui| {
                ui.label("语言 / Language");
                egui::ComboBox::from_id_salt("lang")
                    .selected_text(match settings.lang {
                        Lang::ZhCn => "简体中文",
                        Lang::EnUs => "English",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.lang, Lang::ZhCn, "简体中文");
                        ui.selectable_value(&mut settings.lang, Lang::EnUs, "English");
                    });
                ui.end_row();

                ui.label(t("主题", "Theme"));
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut settings.theme,
                        Theme::System,
                        t("跟随系统", "System"),
                    );
                    ui.selectable_value(&mut settings.theme, Theme::Light, t("浅色", "Light"));
                    ui.selectable_value(&mut settings.theme, Theme::Dark, t("深色", "Dark"));
                });
                ui.end_row();

                ui.label(t("界面缩放", "UI scale"));
                ui.add(egui::Slider::new(&mut settings.zoom, 0.75..=2.0).step_by(0.05));
                ui.end_row();

                ui.label(t("日志默认加载条数", "Log entries per page"));
                ui.add(egui::DragValue::new(&mut settings.log_limit).range(10..=2000));
                ui.end_row();

                ui.label(t("提交时自动删除缺失文件", "Auto-delete missing files"))
                    .on_hover_text(t(
                        "默认关闭；开启后，提交已选中的缺失文件时会先执行 svn delete --force",
                        "Off by default. When enabled, selected missing files are scheduled with svn delete --force before commit.",
                    ));
                ui.checkbox(&mut settings.auto_delete_missing, t("开启", "Enabled"));
                ui.end_row();

                ui.label(t("Gitignore 过滤", "Gitignore filter"));
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut settings.gitignore_enabled,
                        t("开启", "Enabled"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut settings.gitignore_file)
                            .hint_text(".gitignore")
                            .desired_width(160.0),
                    );
                    ui.label(
                        RichText::new(t(
                            "隐藏被该文件匹配的未版本文件",
                            "Hide unversioned files matched by this file",
                        ))
                        .weak()
                        .small(),
                    );
                });
                ui.end_row();

                ui.label(t("SVN 可执行文件", "SVN executable"));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.svn_path)
                            .hint_text("svn")
                            .desired_width(320.0),
                    );
                    if ui.button(t("浏览…", "Browse…")).clicked() {
                        if let Some(file) = rfd::FileDialog::new().pick_file() {
                            self.svn_path = file.to_string_lossy().into_owned();
                        }
                    }
                    if ui
                        .add_enabled(
                            !self.svn_task.running(),
                            egui::Button::new(t("应用", "Apply")),
                        )
                        .clicked()
                    {
                        self.check_svn(&ctx, Some(self.svn_path.clone()));
                    }
                    spinner_label(ui, self.svn_task.running(), "");
                });
                ui.end_row();

                ui.label("");
                match self.svn_version.as_deref() {
                    Some("") => ui.label(
                        RichText::new(t("SVN 不可用", "SVN unavailable"))
                            .color(super::status_color("deleted")),
                    ),
                    Some(version) => ui.label(
                        RichText::new(format!("✔ SVN {version}"))
                            .color(super::status_color("added")),
                    ),
                    None => ui.label(""),
                };
                ui.end_row();
            });

        ui.add_space(16.0);
        ui.separator();
        ui.heading(t("多项目工作区", "Workspaces"));
        let mut collapsed = settings.workspace_list_collapsed;
        if ui
            .checkbox(&mut collapsed, t("折叠多项目栏", "Collapse workspace rail"))
            .changed()
        {
            settings.workspace_list_collapsed = collapsed;
            settings.save();
        }

        ui.label(RichText::new(format!("★ {}", t("收藏", "Favorites"))).weak());
        let favorites = settings.favorites.clone();
        let mut remove_favorite = None;
        for path in &favorites {
            ui.horizontal(|ui| {
                ui.label(path);
                if ui.small_button(t("移除", "Remove")).clicked() {
                    remove_favorite = Some(path.clone());
                }
            });
        }
        if favorites.is_empty() {
            ui.label(RichText::new(t("还没有收藏的工作区", "No favorite workspaces yet")).weak());
        }
        if let Some(path) = remove_favorite {
            settings.favorites.retain(|item| item != &path);
            settings.save();
        }

        ui.label(RichText::new(format!("✎ {}", t("备注", "Notes"))).weak());
        let notes: Vec<(String, String)> = settings
            .workspace_notes
            .iter()
            .map(|(path, note)| (path.clone(), note.clone()))
            .collect();
        let mut clear_note = None;
        for (path, note) in &notes {
            ui.horizontal(|ui| {
                ui.label(path);
                ui.label(RichText::new(note).weak());
                if ui.small_button(t("编辑", "Edit")).clicked() {
                    self.note_edit = Some((path.clone(), note.clone()));
                }
                if ui.small_button(t("清除", "Clear")).clicked() {
                    clear_note = Some(path.clone());
                }
            });
        }
        if notes.is_empty() {
            ui.label(RichText::new(t("还没有工作区备注", "No workspace notes yet")).weak());
        }
        if let Some(path) = clear_note {
            settings.workspace_notes.remove(&path);
            settings.save();
        }

        if let Some((path, note)) = &mut self.note_edit {
            let mut close = false;
            let mut save = false;
            egui::Window::new(t("编辑备注", "Edit note"))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
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
                    settings.workspace_notes.remove(path);
                } else {
                    settings.workspace_notes.insert(path.clone(), trimmed);
                }
                settings.save();
                close = true;
            }
            if close {
                self.note_edit = None;
            }
        }

        ui.add_space(16.0);
        ui.separator();
        ui.label(
            RichText::new(format!(
                "OrcaSVN v{} · Rust + egui",
                env!("CARGO_PKG_VERSION")
            ))
            .weak(),
        );

        let changed = before.lang != settings.lang
            || before.theme != settings.theme
            || before.zoom != settings.zoom
            || before.log_limit != settings.log_limit
            || before.auto_delete_missing != settings.auto_delete_missing;
        if changed {
            apply_appearance(&ctx, settings);
            settings.save();
        }
    }

    fn check_svn(&mut self, ctx: &egui::Context, path: Option<String>) {
        self.svn_task = Task::spawn(ctx, async move {
            let path = path.filter(|p| !p.trim().is_empty());
            svn::configure_svn_executable(path.as_deref())
                .await
                .map_err(|e| e.to_string())
        });
    }
}
