//! Small widget helpers shared by all pages: palette, buttons, badges and a
//! minimal single-line/multi-line text field (GPUI ships no text input).

use gpui::prelude::*;
use gpui::*;
use std::sync::OnceLock;

static MONO: OnceLock<SharedString> = OnceLock::new();

/// Resolve a monospaced family once; GPUI does not expose a generic family.
pub fn init_fonts(cx: &App) {
    let names = cx.text_system().all_font_names();
    let pick = [
        "JetBrains Mono",
        "Cascadia Code",
        "Consolas",
        "DejaVu Sans Mono",
        "Liberation Mono",
        "Menlo",
        "Monaco",
        "Noto Sans Mono",
        "monospace",
    ]
    .iter()
    .find_map(|candidate| {
        names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(candidate))
            .cloned()
    })
    .unwrap_or_else(|| "monospace".to_string());
    let _ = MONO.set(pick.into());
}

pub fn mono_font() -> SharedString {
    MONO.get().cloned().unwrap_or_else(|| "monospace".into())
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub window: u32,
    pub panel: u32,
    pub elevated: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub accent_soft: u32,
    pub add_bg: u32,
    pub del_bg: u32,
    pub hunk: u32,
    pub ok: u32,
    pub warn: u32,
    pub err: u32,
    pub conflict: u32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            window: 0x1b1d21,
            panel: 0x23262b,
            elevated: 0x2b2f36,
            border: 0x3a3f47,
            text: 0xdfe3e8,
            muted: 0x8b939d,
            accent: 0x4f8ff7,
            accent_soft: 0x2a3a52,
            add_bg: 0x184028,
            del_bg: 0x501e22,
            hunk: 0x6ea0e6,
            ok: 0x28a05a,
            warn: 0xdc9628,
            err: 0xdc4646,
            conflict: 0xc828a0,
        }
    } else {
        Palette {
            window: 0xf6f7f9,
            panel: 0xffffff,
            elevated: 0xeef0f4,
            border: 0xd3d8e0,
            text: 0x1d2229,
            muted: 0x606a78,
            accent: 0x1e5ab4,
            accent_soft: 0xdbe6f8,
            add_bg: 0xdafbe1,
            del_bg: 0xffe1e1,
            hunk: 0x1e5ab4,
            ok: 0x28a05a,
            warn: 0xdc9628,
            err: 0xdc4646,
            conflict: 0xc828a0,
        }
    }
}

pub fn status_color(code: &str, pal: Palette) -> u32 {
    match code {
        "modified" => pal.warn,
        "added" => pal.ok,
        "deleted" | "missing" => pal.err,
        "conflicted" | "obstructed" => pal.conflict,
        "replaced" => 0x786ee6,
        "unversioned" => pal.muted,
        _ => pal.accent,
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

/// A status letter badge with a fixed width so rows stay aligned.
pub fn status_badge(code: &str, pal: Palette) -> Div {
    div()
        .w(px(16.0))
        .flex_shrink_0()
        .font_family(mono_font())
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(status_color(code, pal)))
        .child(status_letter(code))
}

/// A flat, hoverable button.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    pal: Palette,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let color = if enabled { pal.text } else { pal.muted };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(if enabled { pal.border } else { pal.panel }))
        .bg(rgb(pal.elevated))
        .text_color(rgb(color))
        .text_sm()
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgb(pal.accent_soft)))
        })
        .when(enabled, |this| this.on_click(on_click))
        .child(label.into())
}

pub fn sep(pal: Palette) -> Div {
    div()
        .h(px(1.0))
        .w_full()
        .flex_shrink_0()
        .bg(rgb(pal.border))
}

pub fn sep_vertical(pal: Palette) -> Div {
    div()
        .w(px(1.0))
        .h_full()
        .flex_shrink_0()
        .bg(rgb(pal.border))
}

pub fn spinner(pal: Palette, label: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .text_xs()
        .text_color(rgb(pal.muted))
        .child("⏳")
        .child(label.into())
}

/// A minimal text field. GPUI has no built-in input widget, so this keeps a
/// `String` plus a character cursor and renders a caret when focused.
pub struct TextField {
    id: &'static str,
    focus: FocusHandle,
    text: String,
    cursor: usize,
    multiline: bool,
}

impl TextField {
    pub fn new<T: 'static>(cx: &mut Context<T>, id: &'static str, multiline: bool) -> Self {
        Self {
            id,
            focus: cx.focus_handle(),
            text: String::new(),
            cursor: 0,
            multiline,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.chars().count();
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    fn chars(&self) -> Vec<char> {
        self.text.chars().collect()
    }

    fn write(&mut self, chars: &[char]) {
        self.text = chars.iter().collect();
        self.cursor = self.cursor.min(chars.len());
    }

    /// Apply a key press. Returns true when the key was consumed.
    pub fn handle_key(&mut self, event: &KeyDownEvent) -> bool {
        let keystroke = &event.keystroke;
        let modified = keystroke.modifiers.control
            || keystroke.modifiers.platform
            || keystroke.modifiers.alt
            || keystroke.modifiers.function;
        let mut chars = self.chars();
        match keystroke.key.as_str() {
            "backspace" => {
                if self.cursor > 0 {
                    chars.remove(self.cursor - 1);
                    self.cursor -= 1;
                    self.write(&chars);
                }
                true
            }
            "delete" => {
                if self.cursor < chars.len() {
                    chars.remove(self.cursor);
                    self.write(&chars);
                }
                true
            }
            "left" => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            "right" => {
                self.cursor = (self.cursor + 1).min(chars.len());
                true
            }
            "home" => {
                self.cursor = 0;
                true
            }
            "end" => {
                self.cursor = chars.len();
                true
            }
            "enter" => {
                if self.multiline {
                    chars.insert(self.cursor, '\n');
                    self.cursor += 1;
                    self.write(&chars);
                    true
                } else {
                    false
                }
            }
            _ => {
                if modified {
                    return false;
                }
                let Some(typed) = keystroke.key_char.as_deref() else {
                    return false;
                };
                let mut inserted = false;
                for ch in typed.chars() {
                    if ch.is_control() {
                        continue;
                    }
                    chars.insert(self.cursor, ch);
                    self.cursor += 1;
                    inserted = true;
                }
                if inserted {
                    self.write(&chars);
                }
                inserted
            }
        }
    }

    fn split(&self) -> (String, String) {
        let chars = self.chars();
        let cursor = self.cursor.min(chars.len());
        (
            chars[..cursor].iter().collect(),
            chars[cursor..].iter().collect(),
        )
    }
}

/// Render a text field. `on_key` receives every key press so the caller can
/// update its own state and trigger side effects (e.g. Enter to search).
pub fn text_field<T: 'static>(
    cx: &mut Context<T>,
    field: &TextField,
    placeholder: &str,
    width: Option<f32>,
    height: Option<f32>,
    window: &Window,
    pal: Palette,
    on_key: impl Fn(&mut T, &KeyDownEvent, &mut Window, &mut Context<T>) + 'static,
) -> Stateful<Div> {
    let focus = field.focus.clone();
    let focused = field.focus.is_focused(window);
    let (before, after) = field.split();
    let show_placeholder = field.text.is_empty();
    let (text, color) = if show_placeholder {
        (placeholder.to_string(), pal.muted)
    } else {
        (String::new(), pal.text)
    };

    let mut content = div().flex().flex_row().items_center().overflow_hidden();
    if show_placeholder {
        content = content.child(div().text_color(rgb(color)).child(text));
    } else if field.multiline {
        content = content
            .flex_col()
            .items_start()
            .child(div().text_color(rgb(pal.text)).child(before))
            .when(focused, |this| {
                this.child(div().text_color(rgb(pal.accent)).child("│"))
            })
            .child(div().text_color(rgb(pal.text)).child(after));
    } else {
        content = content
            .child(div().text_color(rgb(pal.text)).child(before))
            .when(focused, |this| {
                this.child(div().text_color(rgb(pal.accent)).child("│"))
            })
            .child(div().text_color(rgb(pal.text)).child(after));
    }

    div()
        .id(field.id)
        .flex()
        .items_center()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(if focused { pal.accent } else { pal.border }))
        .bg(rgb(if field.multiline {
            pal.panel
        } else {
            pal.elevated
        }))
        .text_sm()
        .when_some(width, |this, width| this.w(px(width)))
        .when_some(height, |this, height| this.h(px(height)).items_start())
        .track_focus(&focus)
        .on_mouse_down(MouseButton::Left, move |_, window, _| {
            window.focus(&focus);
        })
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            on_key(view, event, window, cx);
        }))
        .child(content)
}

/// Turn a checkbox-ish glyph into a clickable element.
pub fn checkbox(
    id: impl Into<ElementId>,
    checked: bool,
    pal: Palette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(16.0))
        .h(px(16.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if checked { pal.accent } else { pal.border }))
        .bg(rgb(if checked { pal.accent } else { pal.panel }))
        .text_xs()
        .text_color(rgb(if checked { pal.panel } else { pal.muted }))
        .cursor_pointer()
        .on_click(on_click)
        .child(if checked { "✔" } else { "" })
}

/// Colors of a diff row.
pub fn diff_row_style(text: &str, pal: Palette) -> (u32, Option<u32>) {
    if text.starts_with("@@") {
        (pal.hunk, None)
    } else if text.starts_with("+++")
        || text.starts_with("---")
        || text.starts_with("Index:")
        || text.starts_with("====")
    {
        (pal.text, None)
    } else if text.starts_with('+') {
        (pal.text, Some(pal.add_bg))
    } else if text.starts_with('-') {
        (pal.text, Some(pal.del_bg))
    } else {
        (pal.text, None)
    }
}
