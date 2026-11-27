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
    pub gitignore: TextField,
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
        let mut gitignore = TextField::new(cx, window, ".gitignore");
        gitignore.set_text(settings.gitignore_file.clone());
        Self {
            svn_path,
            gitignore,
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
                cx.listener(|app, _, window, cx| {
                    app.shared.settings.theme = Theme::System;
                    app.shared.settings.save();
                    app.apply_appearance(window, cx);
                }),
            ))
            .child(ui::button(
                "theme-light",
                t("浅色", "Light"),
                pal,
                theme != Theme::Light,
                cx.listener(|app, _, window, cx| {
                    app.shared.settings.theme = Theme::Light;
                    app.shared.settings.save();
                    app.apply_appearance(window, cx);
                }),
            ))
            .child(ui::button(
                "theme-dark",
                t("深色", "Dark"),
                pal,
                theme != Theme::Dark,
                cx.listener(|app, _, window, cx| {
                    app.shared.settings.theme = Theme::Dark;
                    app.shared.settings.save();
                    app.apply_appearance(window, cx);
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
                cx.listener(|app, _, window, cx| {
                    app.shared.settings.zoom = (app.shared.settings.zoom - 0.05).max(0.75);
                    app.shared.settings.save();
                    app.apply_appearance(window, cx);
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
                cx.listener(|app, _, window, cx| {
                    app.shared.settings.zoom = (app.shared.settings.zoom + 0.05).min(2.0);
                    app.shared.settings.save();
                    app.apply_appearance(window, cx);
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

        let favorites = shared.settings.favorites.clone();
        let notes: Vec<(String, String)> = shared
            .settings
            .workspace_notes
            .iter()
            .map(|(path, note)| (path.clone(), note.clone()))
            .collect();
        let collapsed = shared.settings.workspace_list_collapsed;
        let workspace_section = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(ui::sep(pal))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .child(t("多项目工作区", "Workspaces")),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::checkbox(
                        "settings-rail-collapsed",
                        collapsed,
                        pal,
                        cx.listener(|app, _, _, cx| {
                            app.shared.settings.workspace_list_collapsed =
                                !app.shared.settings.workspace_list_collapsed;
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(pal.muted))
                            .child(t("折叠多项目栏", "Collapse workspace rail")),
                    ),
            )
            .child(
                div()
                    .pt_1()
                    .text_xs()
                    .text_color(rgb(pal.muted))
                    .child(format!("★ {}", t("收藏", "Favorites"))),
            )
            .children(favorites.into_iter().enumerate().map(|(index, path)| {
                let remove = path.clone();
                div()
                    .id(("settings-favorite", index))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .py_1()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_xs()
                            .child(path),
                    )
                    .child(ui::button(
                        "settings-favorite-remove",
                        t("移除", "Remove"),
                        pal,
                        true,
                        cx.listener(move |app, _, _, cx| {
                            app.shared.settings.favorites.retain(|item| item != &remove);
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    ))
            }))
            .when(shared.settings.favorites.is_empty(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(rgb(pal.muted))
                        .child(t("还没有收藏的工作区", "No favorite workspaces yet")),
                )
            })
            .child(
                div()
                    .pt_1()
                    .text_xs()
                    .text_color(rgb(pal.muted))
                    .child(format!("✎ {}", t("备注", "Notes"))),
            )
            .children(notes.into_iter().enumerate().map(|(index, (path, note))| {
                let edit_path = path.clone();
                let edit_note = note.clone();
                let clear_path = path.clone();
                div()
                    .id(("settings-note", index))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .py_1()
                    .child(
                        div()
                            .w(px(220.0))
                            .flex_shrink_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_xs()
                            .child(path),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_xs()
                            .text_color(rgb(pal.muted))
                            .child(note),
                    )
                    .child(ui::button(
                        "settings-note-edit",
                        t("编辑", "Edit"),
                        pal,
                        true,
                        cx.listener(move |_app, _, window, cx| {
                            super::sidebar::open_note_dialog(
                                window,
                                cx,
                                edit_path.clone(),
                                edit_note.clone(),
                            );
                        }),
                    ))
                    .child(ui::button(
                        "settings-note-clear",
                        t("清除", "Clear"),
                        pal,
                        true,
                        cx.listener(move |app, _, _, cx| {
                            app.shared.settings.workspace_notes.remove(&clear_path);
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    ))
            }))
            .when(shared.settings.workspace_notes.is_empty(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(rgb(pal.muted))
                        .child(t("还没有工作区备注", "No workspace notes yet")),
                )
            });

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
                t("Gitignore 过滤", "Gitignore filter"),
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::checkbox(
                        "gitignore-enabled",
                        shared.settings.gitignore_enabled,
                        pal,
                        cx.listener(|app, _, _, cx| {
                            app.shared.settings.gitignore_enabled =
                                !app.shared.settings.gitignore_enabled;
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .w(px(180.0))
                            .child(self.gitignore.input(window, cx).w_full()),
                    )
                    .child(ui::button(
                        "gitignore-apply",
                        t("应用", "Apply"),
                        pal,
                        true,
                        cx.listener(|app, _, _, cx| {
                            app.shared.settings.gitignore_file =
                                app.settings_page.gitignore.text().trim().to_string();
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(pal.muted))
                            .child(t(
                                "开启后隐藏被 .gitignore 匹配的未版本文件",
                                "Hide unversioned files matched by .gitignore",
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
            .child(workspace_section)
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
