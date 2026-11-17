use crate::i18n::{self, t};
use crate::pages::{self, NoticeLevel, Page, Shared};
use crate::settings::{Settings, Theme};
use crate::svn;
use crate::system;
use crate::task;
use crate::ui::{self, palette, Palette};
use crate::SvnInfo;
use gpui::prelude::*;
use gpui::*;
use std::time::Duration;

pub struct OrcaApp {
    pub shared: Shared,
    pub page: Page,
    pub workspace: pages::workspace::WorkspacePage,
    pub log: pages::log::LogPage,
    pub blame: pages::blame::BlamePage,
    pub stash: pages::stash::StashPage,
    pub checkout: pages::checkout::CheckoutPage,
    pub settings_page: pages::settings::SettingsPage,
    info_running: bool,
    recent_open: bool,
    output_scroll: ScrollHandle,
    shown_output: usize,
}

fn is_dark(window: &Window, theme: Theme) -> bool {
    match theme {
        Theme::Light => false,
        Theme::Dark => true,
        Theme::System => matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
    }
}

impl OrcaApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let settings = Settings::load();
        i18n::set_lang(settings.lang);

        let launch = system::launch_workspace_path().or_else(|| settings.workspace.clone());
        let mut shared = Shared::new(settings);
        shared.open_workspace = launch;

        let mut app = Self {
            settings_page: pages::settings::SettingsPage::new(cx, &shared.settings),
            shared,
            page: Page::Workspace,
            workspace: pages::workspace::WorkspacePage::new(cx),
            log: pages::log::LogPage::new(cx),
            blame: pages::blame::BlamePage::new(cx),
            stash: pages::stash::StashPage::new(cx),
            checkout: pages::checkout::CheckoutPage::new(cx),
            info_running: false,
            recent_open: false,
            output_scroll: ScrollHandle::new(),
            shown_output: 0,
        };
        app.configure_svn(cx);
        app.start_clock(cx);
        app
    }

    fn configure_svn(&mut self, cx: &mut Context<Self>) {
        let executable = self.shared.settings.svn_path.trim().to_string();
        task::spawn(
            cx,
            async move {
                let path = (!executable.is_empty()).then_some(executable.as_str());
                svn::configure_svn_executable(path)
                    .await
                    .map_err(|error| error.to_string())
            },
            |_app, result, cx| {
                match result {
                    Ok(version) => tracing::info!(%version, "svn executable ready"),
                    Err(error) => tracing::warn!(%error, "configured svn executable is unusable"),
                }
                cx.notify();
            },
        );
    }

    /// Re-render periodically so the workspace list can auto-refresh.
    fn start_clock(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            if this.update(cx, |app, cx| app.on_tick(cx)).is_err() {
                break;
            }
        })
        .detach();
    }

    fn on_tick(&mut self, cx: &mut Context<Self>) {
        if self.shared.info_dirty {
            self.shared.info_dirty = false;
            self.load_info(cx);
        }
        if self.page == Page::Workspace {
            self.workspace.maybe_auto_refresh(cx);
        }
    }

    fn switch_workspace(&mut self, path: String, cx: &mut Context<Self>) {
        let path = path.trim_end_matches(['/', '\\']).to_string();
        if path.is_empty() {
            return;
        }
        if self.shared.workspace.as_deref() == Some(path.as_str()) {
            return;
        }
        self.shared.settings.remember(&path);
        self.shared.workspace = Some(path.clone());
        self.shared.info = None;
        self.shared.notice = None;
        self.workspace.reset();
        self.log.reset();
        self.blame.reset();
        self.stash.reset();
        self.recent_open = false;
        self.load_info(cx);
    }

    pub fn load_info(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.shared.workspace.clone() else {
            return;
        };
        if self.info_running {
            return;
        }
        self.info_running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::info(&path).await.map_err(|error| error.to_string());
                (path, result)
            },
            |app, (path, result), cx| {
                app.info_running = false;
                if app.shared.workspace.as_deref() == Some(path.as_str()) {
                    match result {
                        Ok(info) => app.shared.info = Some(info),
                        Err(error) => app.shared.err(error),
                    }
                }
                cx.notify();
            },
        );
    }

    pub fn pick_workspace(&mut self, cx: &mut Context<Self>) {
        task::spawn_blocking(
            cx,
            || rfd::FileDialog::new().pick_folder(),
            |app, picked, cx| {
                if let Some(directory) = picked {
                    app.shared.open_workspace = Some(directory.to_string_lossy().into_owned());
                }
                cx.notify();
            },
        );
    }

    fn apply_window(&mut self, window: &mut Window) {
        let target = px(16.0 * self.shared.settings.zoom.clamp(0.6, 2.5));
        if (window.rem_size() - target).abs() > px(0.5) {
            window.set_rem_size(target);
        }
    }

    fn top_bar(&mut self, cx: &mut Context<Self>, pal: Palette) -> AnyElement {
        let workspace = self
            .shared
            .workspace
            .clone()
            .unwrap_or_else(|| t("未打开工作副本", "No working copy").to_string());
        let recent: Vec<String> = self.shared.settings.recent.clone();

        let picker = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .id("workspace-picker")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .flex_1()
                    .min_w_0()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(pal.border))
                    .bg(rgb(pal.elevated))
                    .text_sm()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(pal.accent_soft)))
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.recent_open = !app.recent_open;
                        cx.notify();
                    }))
                    .child(div().flex_shrink_0().child("📂"))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(workspace),
                    )
                    .child(div().flex_shrink_0().text_color(rgb(pal.muted)).child("▾")),
            )
            .when(self.recent_open && !recent.is_empty(), |this| {
                this.child(
                    div()
                        .id("recent-list")
                        .absolute()
                        .top(px(38.0))
                        .left(px(0.0))
                        .w(px(560.0))
                        .max_h(px(280.0))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(pal.border))
                        .bg(rgb(pal.panel))
                        .shadow_lg()
                        .children(recent.iter().enumerate().map(|(index, path)| {
                            let path = path.clone();
                            let label = path.clone();
                            div()
                                .id(("recent", index))
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_sm()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(pal.accent_soft)))
                                .on_click(cx.listener(move |app, _, _, cx| {
                                    app.shared.open_workspace = Some(path.clone());
                                    app.recent_open = false;
                                    cx.notify();
                                }))
                                .child(label)
                        })),
                )
            });

        let info = self.shared.info.as_ref().map(|info| {
            (
                format!("r{}", info.revision),
                info.url.clone(),
                info.repository_root.clone(),
            )
        });

        div()
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(
                div()
                    .flex_shrink_0()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child("🐋 OrcaSVN"),
            )
            .child(ui::sep_vertical(pal))
            .child(picker)
            .child(ui::button(
                "open-workspace",
                format!("📂 {}", t("打开…", "Open…")),
                pal,
                true,
                cx.listener(|app, _, _, cx| app.pick_workspace(cx)),
            ))
            .when_some(info, |this, (revision, url, root)| {
                this.child(ui::sep_vertical(pal))
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_family(ui::mono_font())
                            .text_sm()
                            .child(revision),
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
                            .child(if url.is_empty() { root } else { url }),
                    )
            })
            .when(self.recent_open, |this| {
                this.on_mouse_down_out(cx.listener(|app, _, _, cx| {
                    app.recent_open = false;
                    cx.notify();
                }))
            })
            .into_any_element()
    }

    fn nav(&mut self, cx: &mut Context<Self>, pal: Palette) -> AnyElement {
        div()
            .w(px(150.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .bg(rgb(pal.panel))
            .children(Page::ALL.iter().enumerate().map(|(index, &page)| {
                let active = self.page == page;
                div()
                    .id(("nav", index))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_sm()
                    .when(active, |this| this.bg(rgb(pal.accent_soft)))
                    .when(!active, |this| {
                        this.cursor_pointer()
                            .hover(|style| style.bg(rgb(pal.elevated)))
                    })
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.page = page;
                        app.recent_open = false;
                        cx.notify();
                    }))
                    .child(div().w(px(16.0)).flex_shrink_0().child(page.icon()))
                    .child(page.title())
            }))
            .into_any_element()
    }

    fn status_bar(&mut self, cx: &mut Context<Self>, pal: Palette) -> AnyElement {
        let notice = self.shared.notice.as_ref().map(|(level, message)| {
            let (symbol, color) = match level {
                NoticeLevel::Ok => ("✔", pal.ok),
                NoticeLevel::Warn => ("⚠", pal.warn),
                NoticeLevel::Error => ("✖", pal.err),
            };
            (
                symbol,
                color,
                message.lines().next().unwrap_or_default().to_string(),
            )
        });
        let output_label = if self.shared.show_output {
            t("隐藏输出", "Hide output")
        } else {
            t("输出", "Output")
        };
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .bg(rgb(pal.panel))
            .border_t_1()
            .border_color(rgb(pal.border))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_sm()
                    .when_some(notice, |this, (symbol, color, message)| {
                        this.child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .child(div().text_color(rgb(color)).child(symbol))
                                .child(div().text_color(rgb(color)).child(message)),
                        )
                    }),
            )
            .child(ui::button(
                "toggle-output",
                output_label,
                pal,
                true,
                cx.listener(|app, _, _, cx| {
                    app.shared.show_output = !app.shared.show_output;
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(rgb(pal.muted))
                    .child(format!("v{} · GPUI", env!("CARGO_PKG_VERSION"))),
            )
            .into_any_element()
    }

    fn output_panel(&mut self, cx: &mut Context<Self>, pal: Palette) -> Option<AnyElement> {
        if !self.shared.show_output {
            return None;
        }
        if self.shown_output != self.shared.output.len() {
            self.shown_output = self.shared.output.len();
            self.output_scroll.scroll_to_bottom();
        }
        let lines: Vec<String> = self.shared.output.clone();
        Some(
            div()
                .h(px(180.0))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_t_1()
                .border_color(rgb(pal.border))
                .bg(rgb(pal.panel))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .child(t("命令输出", "Command output")),
                        )
                        .child(ui::button(
                            "clear-output",
                            t("清空", "Clear"),
                            pal,
                            true,
                            cx.listener(|app, _, _, cx| {
                                app.shared.output.clear();
                                cx.notify();
                            }),
                        )),
                )
                .child(
                    div()
                        .id("output-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .track_scroll(&self.output_scroll)
                        .px_3()
                        .font_family(ui::mono_font())
                        .text_xs()
                        .children(
                            lines
                                .iter()
                                .map(|line| div().whitespace_normal().child(line.clone())),
                        ),
                )
                .into_any_element(),
        )
    }

    fn empty_state(&mut self, cx: &mut Context<Self>, pal: Palette) -> AnyElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(t(
                "打开一个 SVN 工作副本开始使用",
                "Open an SVN working copy to get started",
            )))
            .child(ui::button(
                "empty-open",
                format!("📂 {}", t("打开工作副本…", "Open working copy…")),
                pal,
                true,
                cx.listener(|app, _, _, cx| app.pick_workspace(cx)),
            ))
            .child(ui::button(
                "empty-checkout",
                format!(
                    "⬇ {}",
                    t("检出新的工作副本", "Check out a new working copy")
                ),
                pal,
                true,
                cx.listener(|app, _, _, cx| {
                    app.page = Page::Checkout;
                    cx.notify();
                }),
            ))
            .into_any_element()
    }

    fn central(&mut self, window: &mut Window, cx: &mut Context<Self>, pal: Palette) -> AnyElement {
        let needs_workspace = !matches!(self.page, Page::Checkout | Page::Settings);
        if needs_workspace && self.shared.workspace.is_none() {
            return self.empty_state(cx, pal);
        }
        let Self {
            settings_page,
            shared,
            workspace,
            log,
            blame,
            stash,
            checkout,
            ..
        } = self;
        match self.page {
            Page::Workspace => workspace.render(shared, window, cx, pal),
            Page::Log => log.render(shared, window, cx, pal),
            Page::Blame => blame.render(shared, window, cx, pal),
            Page::Stash => stash.render(shared, window, cx, pal),
            Page::Checkout => checkout.render(shared, window, cx, pal),
            Page::Settings => settings_page.render(shared, window, cx, pal),
        }
    }
}

impl Render for OrcaApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_window(window);
        let pal = palette(is_dark(window, self.shared.settings.theme));

        if let Some(path) = self.shared.open_workspace.take() {
            self.switch_workspace(path, cx);
        }
        if self.shared.info_dirty {
            self.shared.info_dirty = false;
            self.load_info(cx);
        }
        if let Some(page) = self.shared.goto.take() {
            self.page = page;
        }
        if let Some(file) = self.shared.blame_file.take() {
            self.blame.open(file, cx);
            self.page = Page::Blame;
        }

        let output = self.output_panel(cx, pal);
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(pal.window))
            .text_color(rgb(pal.text))
            .child(self.top_bar(cx, pal))
            .child(ui::sep(pal))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(self.nav(cx, pal))
                    .child(ui::sep_vertical(pal))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .child(self.central(window, cx, pal)),
                    ),
            )
            .when_some(output, |this, output| this.child(output))
            .child(self.status_bar(cx, pal))
    }
}

/// Data the shell needs when a page asks for a workspace to be opened.
#[allow(dead_code)]
pub fn info_revision(info: &Option<SvnInfo>) -> Option<u64> {
    info.as_ref().map(|info| info.revision)
}
