use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Ui};

/// One diff row with its line numbers in the original and new file
/// (0 when the row has none, e.g. headers or the other side of a change).
struct DiffLine {
    text: String,
    old: u32,
    new: u32,
}

/// A unified diff pre-split into lines for virtualized rendering.
#[derive(Default)]
pub struct DiffDocument {
    pub title: String,
    lines: Vec<DiffLine>,
    /// Digits needed for the widest line number, for the gutter width.
    digits: usize,
}

/// Parse `@@ -old[,count] +new[,count] @@` into (old, old_count, new, new_count).
fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    let rest = line.strip_prefix("@@ -")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(" +")?;
    let range = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (old_start, old_count) = range(old)?;
    let (new_start, new_count) = range(new)?;
    Some((old_start, old_count, new_start, new_count))
}

fn number_lines(diff: &str) -> Vec<DiffLine> {
    let (mut old_next, mut new_next) = (0u32, 0u32);
    let (mut old_remain, mut new_remain) = (0u32, 0u32);
    diff.lines()
        .map(|raw| {
            let text = raw.replace('\t', "    ");
            let (mut old, mut new) = (0, 0);
            let in_hunk = old_remain > 0 || new_remain > 0;
            if !in_hunk {
                if let Some((o, oc, n, nc)) = parse_hunk_header(raw) {
                    (old_next, old_remain, new_next, new_remain) = (o, oc, n, nc);
                }
            } else {
                // Inside a hunk the first character decides the side, so
                // content such as "-- x" is counted correctly.
                match raw.as_bytes().first() {
                    Some(b'-') => {
                        old = old_next;
                        old_next += 1;
                        old_remain = old_remain.saturating_sub(1);
                    }
                    Some(b'+') => {
                        new = new_next;
                        new_next += 1;
                        new_remain = new_remain.saturating_sub(1);
                    }
                    Some(b'\\') => {}
                    _ => {
                        old = old_next;
                        new = new_next;
                        old_next += 1;
                        new_next += 1;
                        old_remain = old_remain.saturating_sub(1);
                        new_remain = new_remain.saturating_sub(1);
                    }
                }
            }
            DiffLine { text, old, new }
        })
        .collect()
}

impl DiffDocument {
    pub fn new(title: impl Into<String>, diff: &str) -> Self {
        let lines = number_lines(diff);
        let max = lines.iter().map(|l| l.old.max(l.new)).max().unwrap_or(0);
        Self {
            title: title.into(),
            digits: max.max(1).to_string().len(),
            lines,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|line| line.text.trim().is_empty())
    }

    pub fn stats(&self) -> (usize, usize) {
        let count = |sign: char, header: &str| {
            self.lines
                .iter()
                .filter(|l| l.text.starts_with(sign) && !l.text.starts_with(header))
                .count()
        };
        (count('+', "+++"), count('-', "---"))
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
        let text_color = ui.visuals().text_color();
        let gutter_color = ui.visuals().weak_text_color();
        let strong_color = ui.visuals().strong_text_color();
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let digits = self.digits;
        let number = |n: u32| {
            if n == 0 {
                " ".repeat(digits)
            } else {
                format!("{n:>digits$}")
            }
        };
        egui::ScrollArea::both()
            .id_salt(("diff", &self.title))
            .auto_shrink([false, false])
            .show_rows(ui, row_height, self.lines.len(), |ui, range| {
                for line in &self.lines[range] {
                    let text = line.text.as_str();
                    let mut format = TextFormat::simple(font.clone(), text_color);
                    if text.starts_with("@@") {
                        format.color = hunk;
                    } else if text.starts_with("+++")
                        || text.starts_with("---")
                        || text.starts_with("Index:")
                        || text.starts_with("====")
                    {
                        format.color = strong_color;
                    } else if text.starts_with('+') {
                        format.background = add_bg;
                    } else if text.starts_with('-') {
                        format.background = del_bg;
                    }
                    let mut job = LayoutJob::default();
                    job.append(
                        &format!("{} {} │ ", number(line.old), number(line.new)),
                        0.0,
                        TextFormat::simple(FontId::clone(&font), gutter_color),
                    );
                    job.append(text, 0.0, format);
                    ui.add(egui::Label::new(job).extend());
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_carry_line_numbers_from_hunk_headers() {
        let text = [
            "--- a",
            "+++ b",
            "@@ -10,3 +20,3 @@",
            " keep",
            "-- dashed",
            "+new",
            "\\ No newline at end of file",
            " tail",
            "@@ -50 +60,2 @@",
            "-x",
            "+y",
            "+z",
        ]
        .join("\n");
        let nums: Vec<_> = number_lines(&text).iter().map(|l| (l.old, l.new)).collect();
        assert_eq!(
            nums,
            [
                (0, 0),
                (0, 0),
                (0, 0),
                (10, 20),
                (11, 0),
                (0, 21),
                (0, 0),
                (12, 22),
                (0, 0),
                (50, 0),
                (0, 60),
                (0, 61)
            ]
        );
    }

    #[test]
    fn gutter_width_follows_largest_line_number() {
        let doc = DiffDocument::new("t", "@@ -998,2 +998,3 @@\n a\n b\n+c");
        assert_eq!(doc.digits, 4);
        assert_eq!(doc.stats(), (1, 0));
    }
}
