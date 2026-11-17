//! Unified diff pre-split into rows with line numbers, rendered through a
//! virtualized `uniform_list`.

use crate::ui::{self, mono_font, Palette};
use gpui::prelude::*;
use gpui::{div, px, rgb, uniform_list, AnyElement, SharedString};
use std::rc::Rc;

/// One diff row with its line numbers in the original and new file
/// (0 when the row has none, e.g. headers or the other side of a change).
struct DiffLine {
    text: String,
    old: u32,
    new: u32,
}

/// A unified diff pre-split into lines for virtualized rendering.
pub struct DiffDocument {
    pub title: String,
    rows: Rc<Vec<DiffLine>>,
    /// Digits needed for the widest line number, for the gutter width.
    digits: usize,
    added: usize,
    removed: usize,
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
        let count = |sign: char, header: &str| {
            lines
                .iter()
                .filter(|l| l.text.starts_with(sign) && !l.text.starts_with(header))
                .count()
        };
        Self {
            title: title.into(),
            digits: max.max(1).to_string().len(),
            added: count('+', "+++"),
            removed: count('-', "---"),
            rows: Rc::new(lines),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|line| line.text.trim().is_empty())
    }

    /// (added, removed) line counts, ignoring the +++/--- file headers.
    pub fn stats(&self) -> (usize, usize) {
        (self.added, self.removed)
    }

    pub fn render(&self, pal: Palette) -> AnyElement {
        let rows = self.rows.clone();
        let digits = self.digits;
        let gutter_width = (digits * 2 + 4) as f32 * 7.5 + 3.0;
        let mono = mono_font();
        let number = move |n: u32| {
            if n == 0 {
                " ".repeat(digits)
            } else {
                format!("{n:>digits$}")
            }
        };
        let numbers = Rc::new(
            rows.iter()
                .map(|line| (number(line.old), number(line.new)))
                .collect::<Vec<_>>(),
        );
        let id = SharedString::from(format!("diff:{}", self.title));
        uniform_list(id, rows.len(), move |range, _window, _cx| {
            range
                .map(|index| {
                    let line = &rows[index];
                    let (text_color, background) = ui::diff_row_style(&line.text, pal);
                    let (old, new) = &numbers[index];
                    div()
                        .flex()
                        .flex_row()
                        .h(px(18.0))
                        .w_full()
                        .font_family(mono.clone())
                        .text_size(px(12.0))
                        .when_some(background, |this, bg| this.bg(rgb(bg)))
                        .child(
                            div()
                                .w(px(gutter_width))
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_color(rgb(pal.muted))
                                .child(format!("{old} {new} │ ")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_color(rgb(text_color))
                                .child(line.text.clone()),
                        )
                })
                .collect::<Vec<_>>()
        })
        .flex_1()
        .into_any_element()
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
