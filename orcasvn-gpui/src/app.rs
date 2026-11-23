use crate::i18n::{self, t};
use crate::pages::sidebar::WorkspaceRail;
use crate::pages::{self, NoticeLevel, Page, Shared};
use crate::settings::{Settings, Theme};
use crate::svn;
use crate::system;
use crate::task;
use crate::ui::{self, Palette};
use crate::SvnInfo;
use gpui_kit::component::{Theme as KitTheme, ThemeMode};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::time::Duration;

actions!(
    orcasvn,
    [
        OpenWorkspace,
        RefreshWorkspace,
        FocusFilter,
        CommitSelection,
        Page1,
        Page2,
        Page3,
        Page4,
        Page5,
        Page6,
        Page7,
        Page8,
        Page9,
    ]
);

pub struct OrcaApp {
    pub shared: Shared,
    pub page: Page,
    pub workspace: pages::workspace::WorkspacePage,
    pub commit: pages::commit::CommitPage,
    pub update: pages::update::UpdatePage,
    pub diff_page: pages::diff::DiffPage,
    pub log: pages::log::LogPage,
    pub blame: pages::blame::BlamePage,
    pub stash: pages::stash::StashPage,
    pub checkout: pages::checkout::CheckoutPage,
    pub settings_page: pages::settings::SettingsPage,
    pub rail: WorkspaceRail,
    info_running: bool,
    recent_open: bool,
    output_scroll: ScrollHandle,
    shown_output: usize,
    root_focus: FocusHandle,
}

impl OrcaApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = Settings::load();
        i18n::set_lang(settings.lang);

        let launch = system::launch_workspace_path().or_else(|| settings.workspace.clone());
        let mut shared = Shared::new(settings);
        shared.open_workspace = launch;

        let mut app = Self {
            settings_page: pages::settings::SettingsPage::new(cx, window, &shared.settings),
            rail: WorkspaceRail::new(),
            shared,
            page: Page::Workspace,
            workspace: pages::workspace::WorkspacePage::new(cx, window),
            commit: pages::commit::CommitPage::new(cx, window),
            update: pages::update::UpdatePage::new(cx, window),
            diff_page: pages::diff::DiffPage::new(cx, window),
            log: pages::log::LogPage::new(cx, window),
            blame: pages::blame::BlamePage::new(cx, window),
            stash: pages::stash::StashPage::new(cx, window),
            checkout: pages::checkout::CheckoutPage::new(cx, window),
            info_running: false,
            recent_open: false,
            output_scroll: ScrollHandle::new(),
            shown_output: 0,
            root_focus: cx.focus_handle(),
        };
        window.focus(&app.root_focus, cx);
        app.configure_svn(cx);
        app.start_clock(cx);
        app.install_shortcuts(cx);
        app
    }

    /// Global shortcuts are registered as actions so they keep working when a
    /// page switch drops the focused control.
    fn install_shortcuts(&mut self, cx: &mut Context<Self>) {
        cx.bind_keys([
            KeyBinding::new("ctrl-o", OpenWorkspace, None),
            KeyBinding::new("ctrl-r", RefreshWorkspace, None),
            KeyBinding::new("f5", RefreshWorkspace, None),
            KeyBinding::new("ctrl-f", FocusFilter, None),
            KeyBinding::new("ctrl-enter", CommitSelection, None),
            KeyBinding::new("ctrl-1", Page1, None),
            KeyBinding::new("ctrl-2", Page2, None),
            KeyBinding::new("ctrl-3", Page3, None),
            KeyBinding::new("ctrl-4", Page4, None),
            KeyBinding::new("ctrl-5", Page5, None),
            KeyBinding::new("ctrl-6", Page6, None),
            KeyBinding::new("ctrl-7", Page7, None),
            KeyBinding::new("ctrl-8", Page8, None),
            KeyBinding::new("ctrl-9", Page9, None),
        ]);

        let entity = cx.entity().downgrade();
        let open = entity.clone();
        App::on_action(&mut *cx, move |_: &OpenWorkspace, cx: &mut App| {
            open.update(cx, |app, cx| app.pick_workspace(cx)).ok();
        });
        let refresh = entity.clone();
        App::on_action(&mut *cx, move |_: &RefreshWorkspace, cx: &mut App| {
            refresh
                .update(cx, |app, cx| {
                    if app.page == Page::Workspace {
                        app.workspace.refresh(cx, true);
                    }
                })
                .ok();
        });
        let focus_filter = entity.clone();
        App::on_action(&mut *cx, move |_: &FocusFilter, cx: &mut App| {
            focus_filter
                .update(cx, |app, cx| {
                    if app.page == Page::Workspace {
                        app.workspace.request_filter_focus();
                        cx.notify();
                    }
                })
                .ok();
        });
        let commit = entity.clone();
        App::on_action(&mut *cx, move |_: &CommitSelection, cx: &mut App| {
            commit
                .update(cx, |app, cx| {
                    if app.page == Page::Workspace {
                        let Self {
                            workspace, shared, ..
                        } = app;
                        workspace.commit(shared, cx);
                    }
                })
                .ok();
        });
        macro_rules! page_action {
            ($action:ty, $page:expr) => {{
                let entity = entity.clone();
                App::on_action(&mut *cx, move |_: &$action, cx: &mut App| {
                    entity
                        .update(cx, |app, cx| {
                            app.page = $page;
                            cx.notify();
                        })
                        .ok();
                });
            }};
        }
        page_action!(Page1, Page::Workspace);
        page_action!(Page2, Page::Log);
        page_action!(Page3, Page::Blame);
        page_action!(Page4, Page::Stash);
        page_action!(Page5, Page::Checkout);
        page_action!(Page6, Page::Stash);
        page_action!(Page7, Page::Blame);
        page_action!(Page8, Page::Checkout);
        page_action!(Page9, Page::Settings);
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
        self.rail.maybe_refresh(&self.shared, cx);
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
        self.commit.reset();
        self.update.reset();
        self.diff_page.reset();
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

    fn apply_window(&mut self, window: &mut Window, cx: &mut App) {
        let mode = match self.shared.settings.theme {
            Theme::Dark => ThemeMode::Dark,
            Theme::Light => ThemeMode::Light,
            Theme::System => ThemeMode::from(window.appearance()),
        };
        if KitTheme::global(cx).mode != mode {
            KitTheme::change(mode, Some(window), cx);
        }
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
        let workspace_count = self.workspace.change_count();
        let groups: [(&'static str, &[Page]); 3] = [
            (t("工作副本", "Working copy"), &[Page::Workspace, Page::Log]),
            (
                t("仓库", "Repository"),
                &[
                    Page::Commit,
                    Page::Update,
                    Page::Diff,
                    Page::Stash,
                    Page::Blame,
                ],
            ),
            (t("应用", "Application"), &[Page::Checkout, Page::Settings]),
        ];
        let repo = self
            .shared
            .workspace
            .as_deref()
            .map(pages::sidebar::workspace_name)
            .unwrap_or_else(|| t("未打开工作副本", "No working copy").to_string());
        let revision = self
            .shared
            .info
            .as_ref()
            .map(|info| format!("r{}", info.revision));

        div()
            .w(px(168.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .bg(rgb(pal.panel))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_1()
                    .pb_1()
                    .child(div().text_sm().child("📁"))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(repo),
                    )
                    .when_some(revision, |this, revision| {
                        this.child(
                            div()
                                .text_xs()
                                .font_family(ui::mono_font())
                                .text_color(rgb(pal.muted))
                                .child(revision),
                        )
                    }),
            )
            .children(groups.into_iter().flat_map(|(heading, pages)| {
                let mut items = vec![div()
                    .px_1()
                    .pt_2()
                    .pb_1()
                    .text_xs()
                    .text_color(rgb(pal.muted))
                    .child(heading)
                    .into_any_element()];
                items.extend(pages.iter().map(|&page| {
                    let active = self.page == page;
                    let badge = match page {
                        Page::Workspace if workspace_count > 0 => Some(workspace_count.to_string()),
                        _ => None,
                    };
                    div()
                        .id(("nav", page as usize))
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
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(page.title()),
                        )
                        .when_some(badge, |this, badge| {
                            this.child(div().text_xs().text_color(rgb(pal.muted)).child(badge))
                        })
                        .into_any_element()
                }));
                items
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
        let path = self
            .shared
            .workspace
            .clone()
            .unwrap_or_else(|| t("未打开工作副本", "No working copy").to_string());
        let count = self
            .shared
            .workspace
            .as_deref()
            .and_then(|path| self.rail.summaries.get(path))
            .and_then(|summary| summary.changed)
            .unwrap_or_else(|| self.workspace.change_count());
        let revision = self
            .shared
            .info
            .as_ref()
            .map(|info| format!("r{}", info.revision));
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
                    .text_xs()
                    .text_color(rgb(pal.muted))
                    .when_some(notice, |this, (symbol, color, message)| {
                        this.child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .child(div().text_color(rgb(color)).child(symbol))
                                .child(div().text_color(rgb(color)).child(message)),
                        )
                    })
                    .when(self.shared.notice.is_none(), |this| this.child(path)),
            )
            .when(count > 0, |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(rgb(pal.muted))
                        .child(t("变更", "changes").to_string() + &format!(" {count}")),
                )
            })
            .when_some(revision, |this, revision| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .font_family(ui::mono_font())
                        .text_color(rgb(pal.muted))
                        .child(revision),
                )
            })
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

    fn route_header(&self, _cx: &mut Context<Self>, pal: Palette) -> AnyElement {
        let repo = self
            .shared
            .workspace
            .as_deref()
            .map(pages::sidebar::workspace_name)
            .unwrap_or_else(|| t("未打开工作副本", "No working copy").to_string());
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(
                div()
                    .w(px(3.0))
                    .h(px(14.0))
                    .rounded_sm()
                    .bg(rgb(pal.accent)),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .child(self.page.title()),
            )
            .child(div().flex_1())
            .child(div().text_xs().text_color(rgb(pal.muted)).child(repo))
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
            commit,
            update,
            diff_page,
            log,
            blame,
            stash,
            checkout,
            ..
        } = self;
        match self.page {
            Page::Workspace => workspace.render(shared, window, cx, pal),
            Page::Commit => commit.render(shared, window, cx, pal),
            Page::Update => update.render(shared, window, cx, pal),
            Page::Diff => diff_page.render(shared, window, cx, pal),
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
        self.apply_window(window, cx);
        // Keep a focus target so shortcuts still reach the shell after a page
        // switch unmounts the previously focused control.
        if window.focused(cx).is_none() {
            window.focus(&self.root_focus, cx);
        }
        let pal = ui::palette_from_theme();

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
        if self.page == Page::Commit {
            if let Some(scope) = self.shared.commit_scope.take() {
                self.commit
                    .set_scope(if scope.is_empty() { None } else { Some(scope) });
            }
            if let Some(file) = self.shared.commit_file.take() {
                self.commit.preselect(file, cx);
            }
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
                    .child(self.rail.render(&self.shared, window, cx, pal))
                    .child(ui::sep_vertical(pal))
                    .child(self.nav(cx, pal))
                    .child(ui::sep_vertical(pal))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .child(self.route_header(cx, pal))
                            .child(ui::sep(pal))
                            .child(self.central(window, cx, pal)),
                    ),
            )
            .when_some(output, |this, output| this.child(output))
            .child(self.status_bar(cx, pal))
            .track_focus(&self.root_focus)
    }
}

/// Data the shell needs when a page asks for a workspace to be opened.
#[allow(dead_code)]
pub fn info_revision(info: &Option<SvnInfo>) -> Option<u64> {
    info.as_ref().map(|info| info.revision)
}
