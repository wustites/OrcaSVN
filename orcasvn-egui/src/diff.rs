use egui::{Color32, RichText, Ui};

/// A unified diff pre-split into lines for virtualized rendering.
#[derive(Default)]
pub struct DiffDocument {
    pub title: String,
    lines: Vec<String>,
}

impl DiffDocument {
    pub fn new(title: impl Into<String>, diff: &str) -> Self {
        Self {
            title: title.into(),
            lines: diff
                .lines()
                .map(|line| line.replace('\t', "    "))
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|line| line.trim().is_empty())
    }

    pub fn stats(&self) -> (usize, usize) {
        let added = self
            .lines
            .iter()
            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
            .count();
        let removed = self
            .lines
            .iter()
            .filter(|l| l.starts_with('-') && !l.starts_with("---"))
            .count();
        (added, removed)
    }

    pub fn show(&self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        let (add_bg, del_bg, hunk) = if dark {
            (
                Color32::from_rgb(24, 64, 40),
                Color32::from_rgb(80, 30, 34),
                Color32::from_rgb(110, 160, 230),
            )
        } else {
            (
                Color32::from_rgb(218, 251, 225),
                Color32::from_rgb(255, 225, 225),
                Color32::from_rgb(30, 90, 180),
            )
        };
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        egui::ScrollArea::both()
            .id_salt(("diff", &self.title))
            .auto_shrink([false, false])
            .show_rows(ui, row_height, self.lines.len(), |ui, range| {
                for line in &self.lines[range] {
                    let text = RichText::new(line.as_str()).monospace();
                    let text = if line.starts_with("@@") {
                        text.color(hunk)
                    } else if line.starts_with("+++")
                        || line.starts_with("---")
                        || line.starts_with("Index:")
                        || line.starts_with("====")
                    {
                        text.strong()
                    } else if line.starts_with('+') {
                        text.background_color(add_bg)
                    } else if line.starts_with('-') {
                        text.background_color(del_bg)
                    } else {
                        text
                    };
                    ui.add(egui::Label::new(text).extend());
                }
            });
    }
}
