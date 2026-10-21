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
}

impl SettingsPage {
    pub fn new(settings: &Settings) -> Self {
        Self {
            svn_path: settings.svn_path.clone(),
            svn_task: Task::default(),
            svn_version: None,
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
            || before.log_limit != settings.log_limit;
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
