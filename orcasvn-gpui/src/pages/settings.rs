use super::Shared;
use crate::app::OrcaApp;
use crate::i18n::{self, t, Lang};
use crate::settings::Theme;
use crate::svn;
use crate::task;
use crate::ui::{self, Palette, TextField};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct SettingsPage {
    svn_path: TextField,
    svn_running: bool,
    svn_version: Option<String>,
    checked_svn: bool,
}

impl SettingsPage {
    pub fn new(
        cx: &mut Context<OrcaApp>,
        window: &mut Window,
        settings: &crate::settings::Settings,
    ) -> Self {
        let mut svn_path = TextField::new(cx, window, "svn");
        svn_path.set_text(settings.svn_path.clone());
        Self {
            svn_path,
            svn_running: false,
            svn_version: None,
            checked_svn: false,
        }
    }

    fn check_svn(&mut self, path: Option<String>, cx: &mut Context<OrcaApp>) {
        if self.svn_running {
            return;
        }
        self.svn_running = true;
        task::spawn(
            cx,
            async move {
                svn::configure_svn_executable(path.as_deref())
                    .await
                    .map_err(|error| error.to_string())
            },
            |app, result, cx| {
                app.settings_page.svn_running = false;
                match result {
                    Ok(version) => {
                        app.shared.settings.svn_path =
                            app.settings_page.svn_path.text().trim().to_string();
                        app.shared.settings.save();
                        app.settings_page.svn_version = Some(version);
                    }
                    Err(error) => {
                        app.settings_page.svn_version = Some(String::new());
                        app.shared.err(format!(
                            "{}: {error}",
                            t(
                                "无法使用该 SVN 可执行文件",
                                "Cannot use this SVN executable"
                            )
                        ));
                    }
                }
                cx.notify();
            },
        );
    }

    fn apply_contrast(shared: &mut Shared, lang: Lang) {
        i18n::set_lang(lang);
        shared.settings.save();
    }

    pub fn render(
        &mut self,
        shared: &mut Shared,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        if !self.checked_svn {
            self.checked_svn = true;
            let path = shared.settings.svn_path.clone();
            self.svn_path.set_text(shared.settings.svn_path.clone());
            self.check_svn(Some(path), cx);
        }

        let lang_zh = shared.settings.lang == Lang::ZhCn;
        let theme = shared.settings.theme;
        let zoom = shared.settings.zoom;
        let log_limit = shared.settings.log_limit;
        let auto_delete_missing = shared.settings.auto_delete_missing;

        let lang_row = div()
            .flex()
            .flex_row()
            .gap_2()
            .child(ui::button(
                "lang-zh",
                "简体中文",
                pal,
                !lang_zh,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.lang = Lang::ZhCn;
                    SettingsPage::apply_contrast(&mut app.shared, Lang::ZhCn);
                    cx.notify();
                }),
            ))
            .child(ui::button(
                "lang-en",
                "English",
                pal,
                lang_zh,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.lang = Lang::EnUs;
                    SettingsPage::apply_contrast(&mut app.shared, Lang::EnUs);
                    cx.notify();
                }),
            ));

        let theme_row = div()
            .flex()
            .flex_row()
            .gap_2()
            .child(ui::button(
                "theme-system",
                t("跟随系统", "System"),
                pal,
                theme != Theme::System,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.theme = Theme::System;
                    app.shared.settings.save();
                    cx.notify();
                }),
            ))
            .child(ui::button(
                "theme-light",
                t("浅色", "Light"),
                pal,
                theme != Theme::Light,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.theme = Theme::Light;
                    app.shared.settings.save();
                    cx.notify();
                }),
            ))
            .child(ui::button(
                "theme-dark",
                t("深色", "Dark"),
                pal,
                theme != Theme::Dark,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.theme = Theme::Dark;
                    app.shared.settings.save();
                    cx.notify();
                }),
            ));

        let zoom_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(ui::button(
                "zoom-out",
                "−",
                pal,
                zoom > 0.75,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.zoom = (app.shared.settings.zoom - 0.05).max(0.75);
                    app.shared.settings.save();
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .w(px(56.0))
                    .text_sm()
                    .child(format!("{:.0}%", zoom * 100.0)),
            )
            .child(ui::button(
                "zoom-in",
                "+",
                pal,
                zoom < 2.0,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.zoom = (app.shared.settings.zoom + 0.05).min(2.0);
                    app.shared.settings.save();
                    cx.notify();
                }),
            ));

        let log_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(ui::button(
                "log-limit-dec",
                "−",
                pal,
                log_limit > 10,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.log_limit =
                        app.shared.settings.log_limit.saturating_sub(50).max(10);
                    app.shared.settings.save();
                    cx.notify();
                }),
            ))
            .child(div().w(px(56.0)).text_sm().child(log_limit.to_string()))
            .child(ui::button(
                "log-limit-inc",
                "+",
                pal,
                log_limit < 2000,
                cx.listener(|app, _, _, cx| {
                    app.shared.settings.log_limit = (app.shared.settings.log_limit + 50).min(2000);
                    app.shared.settings.save();
                    cx.notify();
                }),
            ));

        let svn_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(340.0))
                    .child(self.svn_path.input(window, cx).w_full()),
            )
            .child(ui::button(
                "svn-browse",
                t("浏览…", "Browse…"),
                pal,
                true,
                cx.listener(|_app, _, _, cx| {
                    task::spawn_blocking(
                        cx,
                        || rfd::FileDialog::new().pick_file(),
                        |app, picked, cx| {
                            if let Some(file) = picked {
                                let path = file.to_string_lossy().into_owned();
                                app.settings_page.svn_path.set_text(path.clone());
                                app.settings_page.check_svn(Some(path), cx);
                            }
                            cx.notify();
                        },
                    );
                }),
            ))
            .child(ui::button(
                "svn-apply",
                t("应用", "Apply"),
                pal,
                !self.svn_running,
                cx.listener(|app, _, _, cx| {
                    let path = app.settings_page.svn_path.text().trim().to_string();
                    app.settings_page.check_svn(Some(path), cx);
                }),
            ))
            .when(self.svn_running, |this| this.child(ui::spinner(pal, "")));

        let svn_status = match self.svn_version.as_deref() {
            Some("") => div()
                .text_sm()
                .text_color(rgb(pal.err))
                .child(t("SVN 不可用", "SVN unavailable")),
            Some(version) => div()
                .text_sm()
                .text_color(rgb(pal.ok))
                .child(format!("✔ SVN {version}")),
            None => div(),
        };

        let row = |label: &str, control: AnyElement| {
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap_3()
                .py_1()
                .child(
                    div()
                        .w(px(200.0))
                        .flex_shrink_0()
                        .text_sm()
                        .child(label.to_string()),
                )
                .child(control)
        };

        div()
            .id("settings-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .p_6()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(t("设置", "Settings")),
            )
            .child(row("语言 / Language", lang_row.into_any_element()))
            .child(row(t("主题", "Theme"), theme_row.into_any_element()))
            .child(row(t("界面缩放", "UI scale"), zoom_row.into_any_element()))
            .child(row(
                t("日志默认加载条数", "Log entries per page"),
                log_row.into_any_element(),
            ))
            .child(row(
                t("提交时自动删除缺失文件", "Auto-delete missing files"),
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::checkbox(
                        "auto-delete-missing",
                        auto_delete_missing,
                        pal,
                        cx.listener(|app, _, _, cx| {
                            app.shared.settings.auto_delete_missing =
                                !app.shared.settings.auto_delete_missing;
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(pal.muted))
                            .child(t(
                                "开启后，提交已选中的缺失文件时会先执行 svn delete --force",
                                "When enabled, selected missing files are scheduled with svn delete --force before commit",
                            )),
                    )
                    .into_any_element(),
            ))
            .child(row(
                t("SVN 可执行文件", "SVN executable"),
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(svn_row)
                    .child(svn_status)
                    .into_any_element(),
            ))
            .child(ui::sep(pal))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(pal.muted))
                    .child(format!(
                        "OrcaSVN v{} · Rust + GPUI (experimental)",
                        env!("CARGO_PKG_VERSION")
                    )),
            )
            .into_any_element()
    }
}
