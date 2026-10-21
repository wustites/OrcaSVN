use crate::i18n::{self, t};
use crate::pages::{self, Page, Shared};
use crate::settings::{Settings, Theme};
use crate::svn;
use crate::system;
use crate::task::{self, Task};
use crate::SvnInfo;
use egui::{Color32, RichText};

pub struct OrcaApp {
    shared: Shared,
    page: Page,
    info_task: Task<SvnInfo>,
    workspace: pages::workspace::WorkspacePage,
    log: pages::log::LogPage,
    blame: pages::blame::BlamePage,
    stash: pages::stash::StashPage,
    checkout: pages::checkout::CheckoutPage,
    settings: pages::settings::SettingsPage,
}

/// egui's bundled fonts have no CJK glyphs; borrow one from the system.
fn install_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
    ];
    let Some(bytes) = CANDIDATES.iter().find_map(|path| std::fs::read(path).ok()) else {
        tracing::warn!("no CJK font found; Chinese text may not render");
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".to_string(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("cjk".to_string());
    }
    ctx.set_fonts(fonts);
}

pub fn apply_appearance(ctx: &egui::Context, settings: &Settings) {
    i18n::set_lang(settings.lang);
    ctx.set_theme(match settings.theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
    ctx.set_zoom_factor(settings.zoom.clamp(0.6, 2.5));
}

impl OrcaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        install_cjk_font(ctx);
        let settings = Settings::load();
        apply_appearance(ctx, &settings);

        let svn_path = (!settings.svn_path.trim().is_empty()).then(|| settings.svn_path.clone());
        if let Err(error) = task::block_on(svn::configure_svn_executable(svn_path.as_deref())) {
            tracing::warn!(%error, "configured svn executable is unusable");
        }

        let launch = system::launch_workspace_path().or_else(|| settings.workspace.clone());
        let mut shared = Shared::new(settings);
        shared.open_workspace = launch;

        Self {
            settings: pages::settings::SettingsPage::new(&shared.settings),
            shared,
            page: Page::Workspace,
            info_task: Task::default(),
            workspace: Default::default(),
            log: Default::default(),
            blame: Default::default(),
            stash: Default::default(),
            checkout: Default::default(),
        }
    }

    fn switch_workspace(&mut self, ctx: &egui::Context, path: String) {
        let path = path.trim_end_matches(['/', '\\']).to_string();
        if path.is_empty() {
            return;
        }
        self.shared.settings.remember(&path);
        self.shared.workspace = Some(path.clone());
        self.shared.info = None;
        self.workspace = Default::default();
        self.log = Default::default();
        self.blame = Default::default();
        self.stash = Default::default();
        self.info_task = Task::spawn(ctx, async move {
            svn::info(&path).await.map_err(|e| e.to_string())
        });
    }

    fn pick_workspace(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.shared.open_workspace = Some(dir.to_string_lossy().into_owned());
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("🐋 OrcaSVN").strong());
            ui.separator();
            let current = self
                .shared
                .workspace
                .clone()
                .unwrap_or_else(|| t("未打开工作副本", "No working copy").to_string());
            egui::ComboBox::from_id_salt("workspace-picker")
                .width(ui.available_width().min(520.0) - 140.0)
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for path in self.shared.settings.recent.clone() {
                        let active = self.shared.workspace.as_deref() == Some(path.as_str());
                        if ui.selectable_label(active, &path).clicked() {
                            self.shared.open_workspace = Some(path);
                        }
                    }
                });
            if ui.button(format!("🗁 {}", t("打开…", "Open…"))).clicked() {
                self.pick_workspace();
            }
            if let Some(info) = &self.shared.info {
                ui.separator();
                ui.label(RichText::new(format!("r{}", info.revision)).monospace())
                    .on_hover_text(&info.url);
                ui.label(RichText::new(&info.url).weak())
                    .on_hover_text(&info.repository_root);
            }
        });
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        for page in Page::ALL {
            let label = format!("{}  {}", page.icon(), page.title());
            let response = ui.add_sized(
                [ui.available_width(), 30.0],
                egui::Button::selectable(self.page == page, label),
            );
            if response.clicked() {
                self.page = page;
            }
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some((ok, message)) = &self.shared.notice {
                let color = if *ok {
                    Color32::from_rgb(40, 160, 90)
                } else {
                    Color32::from_rgb(220, 70, 70)
                };
                ui.label(RichText::new(if *ok { "✔" } else { "✖" }).color(color));
                ui.label(message.lines().next().unwrap_or_default())
                    .on_hover_text(message.as_str());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).weak());
                let label = if self.shared.show_output {
                    t("隐藏输出", "Hide output")
                } else {
                    t("输出", "Output")
                };
                if ui.small_button(label).clicked() {
                    self.shared.show_output = !self.shared.show_output;
                }
            });
        });
    }
}

impl eframe::App for OrcaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if let Some(path) = self.shared.open_workspace.take() {
            self.switch_workspace(&ctx, path);
        }
        if let Some(result) = self.info_task.poll() {
            match result {
                Ok(info) => self.shared.info = Some(info),
                Err(error) => self.shared.err(error),
            }
        }
        if self.shared.info_dirty {
            self.shared.info_dirty = false;
            if let Some(path) = self.shared.workspace.clone() {
                self.info_task = Task::spawn(&ctx, async move {
                    svn::info(&path).await.map_err(|e| e.to_string())
                });
            }
        }
        if let Some(page) = self.shared.goto.take() {
            self.page = page;
        }
        if let Some(file) = self.shared.blame_file.take() {
            self.blame.open(&ctx, &self.shared, file);
        }

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        if self.shared.show_output {
            egui::Panel::bottom("output")
                .resizable(true)
                .default_size(160.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(t("命令输出", "Command output"));
                        if ui.small_button(t("清空", "Clear")).clicked() {
                            self.shared.output.clear();
                        }
                    });
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for text in &self.shared.output {
                                ui.label(RichText::new(text).monospace());
                            }
                        });
                });
        }
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(150.0)
            .show(ui, |ui| self.nav(ui));

        egui::CentralPanel::default().show(ui, |ui| {
            let needs_workspace = !matches!(self.page, Page::Checkout | Page::Settings);
            if needs_workspace && self.shared.workspace.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.heading(t(
                        "打开一个 SVN 工作副本开始使用",
                        "Open an SVN working copy to get started",
                    ));
                    ui.add_space(12.0);
                    if ui
                        .button(format!("🗁 {}", t("打开工作副本…", "Open working copy…")))
                        .clicked()
                    {
                        self.pick_workspace();
                    }
                    if ui
                        .button(format!(
                            "⬇ {}",
                            t("检出新的工作副本", "Check out a new working copy")
                        ))
                        .clicked()
                    {
                        self.page = Page::Checkout;
                    }
                });
                return;
            }
            match self.page {
                Page::Workspace => self.workspace.show(ui, &mut self.shared),
                Page::Log => self.log.show(ui, &mut self.shared),
                Page::Blame => self.blame.show(ui, &mut self.shared),
                Page::Stash => self.stash.show(ui, &mut self.shared),
                Page::Checkout => self.checkout.show(ui, &mut self.shared),
                Page::Settings => self.settings.show(ui, &mut self.shared),
            }
        });
    }
}
