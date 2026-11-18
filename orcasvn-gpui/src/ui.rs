//! Small widget helpers shared by all pages: palette, buttons, badges and a
//! minimal single-line/multi-line text field (GPUI ships no text input).

use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::Theme as KitTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::cell::RefCell;
use std::rc::Rc;
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

fn color_u32(color: Hsla) -> u32 {
    let rgba = color.to_rgb();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    (channel(rgba.r) << 16) | (channel(rgba.g) << 8) | channel(rgba.b)
}

/// Derive the application palette from the GPUI Kit theme so the hand-drawn
/// layout and the kit controls share one set of colors.
pub fn palette_from_theme(theme: &KitTheme) -> Palette {
    let colors = &theme.colors;
    let base = palette(theme.mode.is_dark());
    Palette {
        window: color_u32(colors.background),
        panel: color_u32(colors.secondary),
        elevated: color_u32(colors.muted),
        border: color_u32(colors.border),
        text: color_u32(colors.foreground),
        muted: color_u32(colors.muted_foreground),
        accent: color_u32(colors.blue),
        accent_soft: color_u32(colors.selection),
        add_bg: base.add_bg,
        del_bg: base.del_bg,
        hunk: color_u32(colors.blue),
        ok: color_u32(colors.green),
        warn: color_u32(colors.yellow),
        err: color_u32(colors.red),
        conflict: base.conflict,
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

/// Single-line text field backed by the GPUI Kit input. The kit provides the
/// IME, selection, undo and clipboard behaviour that a hand-rolled input lacks.
pub struct TextField {
    state: Entity<InputState>,
    cache: Rc<RefCell<String>>,
    pending: Option<String>,
    _sub: Subscription,
}

impl TextField {
    pub fn new<T: 'static>(
        cx: &mut Context<T>,
        window: &mut Window,
        placeholder: impl Into<SharedString>,
    ) -> Self {
        let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let cache = Rc::new(RefCell::new(String::new()));
        let sub = cx.subscribe(&state, {
            let cache = cache.clone();
            move |_app: &mut T,
                  state: Entity<InputState>,
                  event: &InputEvent,
                  cx: &mut Context<T>| {
                if matches!(event, InputEvent::Change) {
                    *cache.borrow_mut() = state.read(cx).value().to_string();
                }
            }
        });
        Self {
            state,
            cache,
            pending: None,
            _sub: sub,
        }
    }

    pub fn text(&self) -> String {
        self.pending
            .clone()
            .unwrap_or_else(|| self.cache.borrow().clone())
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.pending = Some(text.into());
    }

    pub fn clear(&mut self) {
        self.pending = Some(String::new());
    }

    pub fn is_empty(&self) -> bool {
        self.text().trim().is_empty()
    }

    pub fn state(&self) -> &Entity<InputState> {
        &self.state
    }

    /// Flush a programmatic value change (the kit needs a window to move the
    /// caret) and return the element for the current frame.
    pub fn input(&mut self, window: &mut Window, cx: &mut App) -> Input {
        if let Some(text) = self.pending.take() {
            self.cache.replace(text.clone());
            self.state
                .update(cx, |state, cx| state.set_value(text, window, cx));
        }
        Input::new(&self.state)
    }
}

/// Subscribe to Enter on a single-line field (search/filter boxes).
pub fn on_enter<T: 'static>(
    cx: &mut Context<T>,
    state: &Entity<InputState>,
    handler: impl Fn(&mut T, &mut Context<T>) + 'static,
) -> Subscription {
    cx.subscribe(state, move |app, _state, event: &InputEvent, cx| {
        if matches!(event, InputEvent::PressEnter { .. }) {
            handler(app, cx);
        }
    })
}

/// Multi-line field used for commit messages.
pub struct TextArea {
    state: Entity<TextareaState>,
    cache: Rc<RefCell<String>>,
    pending: Option<String>,
    _sub: Subscription,
}

impl TextArea {
    pub fn new<T: 'static>(
        cx: &mut Context<T>,
        window: &mut Window,
        placeholder: impl Into<SharedString>,
        rows: usize,
    ) -> Self {
        let state = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(placeholder)
                .rows(rows)
        });
        let cache = Rc::new(RefCell::new(String::new()));
        let sub = cx.subscribe(&state, {
            let cache = cache.clone();
            move |_app: &mut T,
                  state: Entity<TextareaState>,
                  event: &InputEvent,
                  cx: &mut Context<T>| {
                if matches!(event, InputEvent::Change) {
                    *cache.borrow_mut() = state.read(cx).value().to_string();
                }
            }
        });
        Self {
            state,
            cache,
            pending: None,
            _sub: sub,
        }
    }

    pub fn text(&self) -> String {
        self.pending
            .clone()
            .unwrap_or_else(|| self.cache.borrow().clone())
    }

    pub fn clear(&mut self) {
        self.pending = Some(String::new());
    }

    pub fn is_empty(&self) -> bool {
        self.text().trim().is_empty()
    }

    pub fn textarea(&mut self, window: &mut Window, cx: &mut App) -> Textarea {
        if let Some(text) = self.pending.take() {
            self.cache.replace(text.clone());
            self.state
                .update(cx, |state, cx| state.set_value(text, window, cx));
        }
        Textarea::new(&self.state)
    }
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
