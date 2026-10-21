pub mod blame;
pub mod checkout;
pub mod log;
pub mod settings;
pub mod stash;
pub mod workspace;

use crate::i18n::t;
use crate::settings::Settings;
use crate::SvnInfo;
use egui::{Color32, RichText, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Workspace,
    Log,
    Blame,
    Stash,
    Checkout,
    Settings,
}

impl Page {
    pub const ALL: [Page; 6] = [
        Page::Workspace,
        Page::Log,
        Page::Blame,
        Page::Stash,
        Page::Checkout,
        Page::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Workspace => t("工作区", "Workspace"),
            Page::Log => t("日志", "Log"),
            Page::Blame => t("追溯", "Blame"),
            Page::Stash => t("贮藏", "Stash"),
            Page::Checkout => t("检出", "Checkout"),
            Page::Settings => t("设置", "Settings"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Page::Workspace => "🗀",
            Page::Log => "🕑",
            Page::Blame => "👤",
            Page::Stash => "📦",
            Page::Checkout => "⬇",
            Page::Settings => "⚙",
        }
    }
}

/// State shared between pages and the application shell.
pub struct Shared {
    pub settings: Settings,
    pub workspace: Option<String>,
    pub info: Option<SvnInfo>,
    pub notice: Option<(bool, String)>,
    pub output: Vec<String>,
    pub show_output: bool,
    pub open_workspace: Option<String>,
    pub goto: Option<Page>,
    pub blame_file: Option<String>,
    pub stash_files: Option<Vec<String>>,
    pub info_dirty: bool,
}

impl Shared {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            workspace: None,
            info: None,
            notice: None,
            output: Vec::new(),
            show_output: false,
            open_workspace: None,
            goto: None,
            blame_file: None,
            stash_files: None,
            info_dirty: false,
        }
    }

    pub fn ok(&mut self, message: impl Into<String>) {
        self.notice = Some((true, message.into()));
    }

    pub fn err(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.output.push(message.clone());
        self.notice = Some((false, message));
    }

    /// Record raw SVN output and surface a short success notice.
    pub fn done(&mut self, label: &str, output: &str) {
        if !output.trim().is_empty() {
            self.output
                .push(format!("$ svn {label}\n{}", output.trim_end()));
        }
        self.ok(format!("{label} {}", t("完成", "done")));
    }
}

pub fn status_color(code: &str) -> Color32 {
    match code {
        "modified" => Color32::from_rgb(220, 150, 30),
        "added" => Color32::from_rgb(40, 160, 90),
        "deleted" | "missing" => Color32::from_rgb(220, 70, 70),
        "conflicted" | "obstructed" => Color32::from_rgb(200, 40, 160),
        "replaced" => Color32::from_rgb(120, 110, 220),
        "unversioned" => Color32::from_rgb(130, 130, 130),
        _ => Color32::from_rgb(90, 140, 220),
    }
}

pub fn status_letter(code: &str) -> &'static str {
    match code {
        "modified" => "M",
        "added" => "A",
        "deleted" => "D",
        "missing" => "!",
        "conflicted" => "C",
        "replaced" => "R",
        "unversioned" => "?",
        "obstructed" => "~",
        "ignored" => "I",
        "external" => "X",
        "incomplete" => "…",
        "normal" => " ",
        _ => "*",
    }
}

pub fn status_badge(ui: &mut Ui, code: &str) {
    ui.label(
        RichText::new(status_letter(code))
            .monospace()
            .strong()
            .color(status_color(code)),
    )
    .on_hover_text(code);
}

pub fn spinner_label(ui: &mut Ui, running: bool, text: &str) {
    if running {
        ui.spinner();
        ui.label(RichText::new(text).weak());
    }
}

/// A modal yes/no prompt; returns Some(true) when confirmed.
pub fn confirm_modal(ctx: &egui::Context, id: &str, title: &str, body: &str) -> Option<bool> {
    let mut answer = None;
    let modal = egui::Modal::new(egui::Id::unique(id)).show(ctx, |ui| {
        ui.set_max_width(460.0);
        ui.heading(title);
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .show(ui, |ui| {
                ui.label(body);
            });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button(t("确认", "Confirm")).clicked() {
                answer = Some(true);
            }
            if ui.button(t("取消", "Cancel")).clicked() {
                answer = Some(false);
            }
        });
    });
    if modal.should_close() && answer.is_none() {
        answer = Some(false);
    }
    answer
}
