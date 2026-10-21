use super::{spinner_label, Shared};
use crate::diff::DiffDocument;
use crate::i18n::t;
use crate::svn;
use crate::task::Task;
use crate::{DiffResult, SvnLogEntry};
use egui::{RichText, Ui};

#[derive(Default)]
pub struct LogPage {
    loaded_for: Option<String>,
    entries: Vec<SvnLogEntry>,
    task: Task<Vec<SvnLogEntry>>,
    appending: bool,
    exhausted: bool,
    keyword: String,
    author: String,
    selected: Option<usize>,
    diff: Option<DiffDocument>,
    diff_task: Task<DiffResult>,
}

impl LogPage {
    fn load(&mut self, ctx: &egui::Context, shared: &Shared, append: bool) {
        let Some(workspace) = shared.workspace.clone() else {
            return;
        };
        if self.task.running() {
            return;
        }
        let start = if append {
            match self.entries.last() {
                Some(last) if last.revision > 1 => Some(last.revision - 1),
                _ => {
                    self.exhausted = true;
                    return;
                }
            }
        } else {
            self.entries.clear();
            self.selected = None;
            self.diff = None;
            self.exhausted = false;
            None
        };
        self.appending = append;
        let limit = shared.settings.log_limit.max(1);
        let keyword = Some(self.keyword.trim().to_string()).filter(|s| !s.is_empty());
        let author = Some(self.author.trim().to_string()).filter(|s| !s.is_empty());
        self.task = Task::spawn(ctx, async move {
            svn::log(
                &workspace,
                Some(limit),
                start,
                None,
                keyword.as_deref(),
                author.as_deref(),
                None,
                None,
            )
            .await
            .map_err(|e| e.to_string())
        });
    }

    fn show_diff(
        &mut self,
        ctx: &egui::Context,
        shared: &Shared,
        revision: u64,
        path: Option<&str>,
    ) {
        let (Some(workspace), Some(info)) = (shared.workspace.clone(), shared.info.as_ref()) else {
            return;
        };
        let root = info.repository_root.trim_end_matches('/');
        let target = match path {
            Some(path) => format!("{root}{path}"),
            None => root.to_string(),
        };
        self.diff = None;
        self.diff_task = Task::spawn(ctx, async move {
            svn::diff(&workspace, &target, Some(revision), None)
                .await
                .map(|mut result| {
                    result.path = format!("r{revision} {}", result.path);
                    result
                })
                .map_err(|e| e.to_string())
        });
    }

    pub fn show(&mut self, ui: &mut Ui, shared: &mut Shared) {
        let ctx = ui.ctx().clone();
        if self.loaded_for != shared.workspace {
            *self = Self {
                loaded_for: shared.workspace.clone(),
                ..Default::default()
            };
            self.load(&ctx, shared, false);
        }
        if let Some(result) = self.task.poll() {
            match result {
                Ok(entries) => {
                    if entries.len() < shared.settings.log_limit as usize {
                        self.exhausted = true;
                    }
                    self.entries.extend(entries);
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

        ui.horizontal(|ui| {
            let mut submit = false;
            submit |= ui
                .add(
                    egui::TextEdit::singleline(&mut self.keyword)
                        .hint_text(t("关键字 / 版本 / 路径", "Keyword / revision / path")),
                )
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            submit |= ui
                .add(
                    egui::TextEdit::singleline(&mut self.author)
                        .hint_text(t("作者", "Author"))
                        .desired_width(120.0),
                )
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button(format!("🔍 {}", t("搜索", "Search"))).clicked() || submit {
                self.load(&ctx, shared, false);
            }
            if ui.button(t("重置", "Reset")).clicked() {
                self.keyword.clear();
                self.author.clear();
                self.load(&ctx, shared, false);
            }
            spinner_label(ui, self.task.running(), t("加载中…", "Loading…"));
        });
        ui.separator();

        egui::Panel::left("log-list")
            .resizable(true)
            .default_size(480.0)
            .size_range(300.0..=900.0)
            .show(ui, |ui| self.list(ui, &ctx, shared));

        egui::CentralPanel::default().show(ui, |ui| self.details(ui, &ctx, shared));
    }

    fn list(&mut self, ui: &mut Ui, ctx: &egui::Context, shared: &Shared) {
        let mut clicked = None;
        let mut load_more = false;
        egui::ScrollArea::vertical()
            .id_salt("log-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, entry) in self.entries.iter().enumerate() {
                    let summary = entry.message.lines().next().unwrap_or_default();
                    let date = entry
                        .date
                        .get(..16)
                        .unwrap_or(&entry.date)
                        .replace('T', " ");
                    let active = self.selected == Some(index);
                    let width = ui.available_width();
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(width, 40.0), egui::Sense::click());
                    let visuals = ui.visuals();
                    if active {
                        ui.painter()
                            .rect_filled(rect, 4.0, visuals.selection.bg_fill);
                    } else if response.hovered() {
                        ui.painter()
                            .rect_filled(rect, 4.0, visuals.widgets.hovered.weak_bg_fill);
                    }
                    let strong = if active {
                        visuals.selection.stroke.color
                    } else {
                        visuals.strong_text_color()
                    };
                    let weak = if active {
                        strong
                    } else {
                        visuals.weak_text_color()
                    };
                    let painter = ui.painter_at(rect.shrink2(egui::vec2(8.0, 0.0)));
                    let body = egui::FontId::proportional(13.0);
                    let small = egui::FontId::proportional(11.5);
                    painter.text(
                        rect.left_top() + egui::vec2(8.0, 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("r{}  ·  {}  ·  {}", entry.revision, entry.author, date),
                        small,
                        weak,
                    );
                    painter.text(
                        rect.left_top() + egui::vec2(8.0, 20.0),
                        egui::Align2::LEFT_TOP,
                        summary,
                        body,
                        strong,
                    );
                    if response.clicked() {
                        clicked = Some(index);
                    }
                }
                if !self.entries.is_empty() && !self.exhausted {
                    ui.add_space(6.0);
                    if ui
                        .add_enabled(
                            !self.task.running(),
                            egui::Button::new(t("加载更多", "Load more")),
                        )
                        .clicked()
                    {
                        load_more = true;
                    }
                }
                if self.entries.is_empty() && !self.task.running() {
                    ui.label(RichText::new(t("没有日志记录", "No log entries")).weak());
                }
            });
        if load_more {
            self.load(ctx, shared, true);
        }
        if let Some(index) = clicked {
            self.selected = Some(index);
            self.diff = None;
        }
    }

    fn details(&mut self, ui: &mut Ui, ctx: &egui::Context, shared: &Shared) {
        let Some(entry) = self.selected.and_then(|i| self.entries.get(i)).cloned() else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new(t(
                        "选择一个版本查看详情",
                        "Select a revision to see details",
                    ))
                    .weak(),
                );
            });
            return;
        };
        ui.horizontal(|ui| {
            ui.heading(format!("r{}", entry.revision));
            ui.label(&entry.author);
            ui.label(RichText::new(entry.date.replace('T', " ")).weak());
            if ui
                .button(t("查看整个版本的差异", "Diff whole revision"))
                .clicked()
            {
                self.show_diff(ctx, shared, entry.revision, None);
            }
        });
        ui.add(egui::Label::new(&entry.message).wrap());
        ui.separator();
        let mut open = None;
        egui::ScrollArea::vertical()
            .id_salt("log-paths")
            .max_height(180.0)
            .show(ui, |ui| {
                for changed in &entry.changed_paths {
                    ui.horizontal(|ui| {
                        let code = match changed.action.as_str() {
                            "A" => "added",
                            "D" => "deleted",
                            "R" => "replaced",
                            _ => "modified",
                        };
                        super::status_badge(ui, code);
                        if ui.link(&changed.path).clicked() && changed.action != "D" {
                            open = Some(changed.path.clone());
                        }
                    });
                }
            });
        if let Some(path) = open {
            self.show_diff(ctx, shared, entry.revision, Some(&path));
        }
        ui.separator();
        spinner_label(
            ui,
            self.diff_task.running(),
            t("正在加载差异…", "Loading diff…"),
        );
        if let Some(diff) = &self.diff {
            ui.strong(&diff.title);
            diff.show(ui);
        }
    }
}
