//! Multi-project rail modelled on `main`'s `WorkspaceSidebar`: recent
//! workspaces with change counts, last-modified time, favorites, notes and a
//! collapsible layout.

use crate::app::OrcaApp;
use crate::i18n::t;
use crate::pages::Shared;
use crate::settings::cache_dir;
use crate::svn;
use crate::task;
use crate::ui::{self, Palette};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::WindowExt as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const SUMMARY_REFRESH: Duration = Duration::from_secs(60);

#[derive(Default, Clone, Copy)]
pub struct Summary {
    pub changed: Option<usize>,
    pub modified: Option<u64>,
}

#[derive(Default)]
pub struct WorkspaceRail {
    pub summaries: HashMap<String, Summary>,
    running: bool,
    last_refresh: Option<Instant>,
}

impl WorkspaceRail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn maybe_refresh(&mut self, shared: &Shared, cx: &mut Context<OrcaApp>) {
        if self.running
            || self
                .last_refresh
                .is_some_and(|at| at.elapsed() < SUMMARY_REFRESH)
        {
            return;
        }
        self.refresh(shared, cx);
    }

    pub fn refresh(&mut self, shared: &Shared, cx: &mut Context<OrcaApp>) {
        if self.running {
            return;
        }
        let paths = shared.settings.recent.clone();
        if paths.is_empty() {
            return;
        }
        self.running = true;
        self.last_refresh = Some(Instant::now());
        for path in paths {
            task::spawn(
                cx,
                async move {
                    let modified = svn::cached_latest_modified(&path).await.ok().flatten();
                    let changed = svn::cached_status(&path, false, cache_dir())
                        .await
                        .ok()
                        .map(|entries| entries.len());
                    (path, Summary { changed, modified })
                },
                |app, (path, summary), cx| {
                    app.rail.summaries.insert(path, summary);
                    cx.notify();
                },
            );
        }
        // The individual tasks report nothing when they all finish; clear the
        // running flag after a short grace period instead of tracking each.
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            this.update(cx, |app, cx| {
                app.rail.running = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn favorite(&self, shared: &Shared, path: &str) -> bool {
        shared.settings.favorites.iter().any(|item| item == path)
    }

    fn toggle_favorite(shared: &mut Shared, path: &str) {
        if let Some(index) = shared
            .settings
            .favorites
            .iter()
            .position(|item| item == path)
        {
            shared.settings.favorites.remove(index);
        } else {
            shared.settings.favorites.push(path.to_string());
        }
        shared.settings.save();
    }

    pub fn render(
        &mut self,
        shared: &Shared,
        _window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let collapsed = shared.settings.workspace_list_collapsed;
        let mut paths = shared.settings.recent.clone();
        // Favorites first, then the configured order.
        let favorites = shared.settings.favorites.clone();
        let sort_mode = shared.settings.workspace_sort_mode.clone();
        let changed_of = |path: &str| {
            self.summaries
                .get(path)
                .and_then(|summary| summary.changed)
                .unwrap_or(usize::MAX)
        };
        paths.sort_by(|a, b| {
            let favorite = |path: &str| favorites.iter().position(|item| item == path);
            match (favorite(a), favorite(b)) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => match sort_mode.as_str() {
                    "name" => workspace_name(a)
                        .to_lowercase()
                        .cmp(&workspace_name(b).to_lowercase()),
                    "changed" => changed_of(a).cmp(&changed_of(b)),
                    _ => std::cmp::Ordering::Equal,
                },
            }
        });

        if collapsed {
            return div()
                .w(px(44.0))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .py_2()
                .bg(rgb(pal.panel))
                .child(ui::button(
                    "rail-expand",
                    "»",
                    pal,
                    true,
                    cx.listener(|app, _, _, cx| {
                        app.shared.settings.workspace_list_collapsed = false;
                        app.shared.settings.save();
                        cx.notify();
                    }),
                ))
                .children(paths.iter().enumerate().map(|(index, path)| {
                    let name = workspace_name(path);
                    let initial = name.chars().next().unwrap_or('?').to_string();
                    let active = shared.workspace.as_deref() == Some(path.as_str());
                    let path = path.clone();
                    div()
                        .id(("rail-collapsed", index))
                        .w(px(30.0))
                        .h(px(26.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .text_xs()
                        .when(active, |this| this.bg(rgb(pal.accent_soft)))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(pal.elevated)))
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.shared.open_workspace = Some(path.clone());
                            cx.notify();
                        }))
                        .child(initial)
                }))
                .into_any_element();
        }

        let rows = paths
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let name = workspace_name(path);
                let summary = self.summaries.get(path).copied().unwrap_or_default();
                let active = shared.workspace.as_deref() == Some(path.as_str());
                let favorite = self.favorite(shared, path);
                let note = shared.settings.workspace_notes.get(path).cloned();
                let changed = summary
                    .changed
                    .map(|count| count.to_string())
                    .unwrap_or_else(|| "—".to_string());
                let modified = summary
                    .modified
                    .map(format_modified)
                    .unwrap_or_else(|| "—".to_string());
                let path_click = path.clone();
                let path_star = path.clone();
                let path_note = path.clone();
                let note_text = note.clone().unwrap_or_default();
                let manual = sort_mode == "manual";
                let can_up = index > 0;
                let can_down = index + 1 < paths.len();
                let path_up = path.clone();
                let path_down = path.clone();
                div()
                    .id(("rail-row", index))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if active { pal.accent } else { pal.panel }))
                    .when(active, |this| this.bg(rgb(pal.accent_soft)))
                    .hover(|style| style.bg(rgb(pal.elevated)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |app, _, _, cx| {
                        app.shared.open_workspace = Some(path_click.clone());
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .text_sm()
                                    .when(active, |this| this.font_weight(FontWeight::SEMIBOLD))
                                    .child(name),
                            )
                            .when(manual, |this| {
                                this.child(
                                    div()
                                        .id(("rail-up", index))
                                        .px_1()
                                        .text_sm()
                                        .text_color(rgb(pal.muted))
                                        .cursor_pointer()
                                        .when(can_up, |this| {
                                            this.on_click(cx.listener(move |app, _, _, cx| {
                                                cx.stop_propagation();
                                                move_recent(&mut app.shared, &path_up, -1);
                                                cx.notify();
                                            }))
                                        })
                                        .child("↑"),
                                )
                                .child(
                                    div()
                                        .id(("rail-down", index))
                                        .px_1()
                                        .text_sm()
                                        .text_color(rgb(pal.muted))
                                        .cursor_pointer()
                                        .when(can_down, |this| {
                                            this.on_click(cx.listener(move |app, _, _, cx| {
                                                cx.stop_propagation();
                                                move_recent(&mut app.shared, &path_down, 1);
                                                cx.notify();
                                            }))
                                        })
                                        .child("↓"),
                                )
                            })
                            .child(
                                div()
                                    .id(("rail-star", index))
                                    .px_1()
                                    .text_sm()
                                    .text_color(rgb(if favorite { pal.warn } else { pal.muted }))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |app, _, _, cx| {
                                        cx.stop_propagation();
                                        WorkspaceRail::toggle_favorite(&mut app.shared, &path_star);
                                        cx.notify();
                                    }))
                                    .child(if favorite { "★" } else { "☆" }),
                            )
                            .child(
                                div()
                                    .id(("rail-note", index))
                                    .px_1()
                                    .text_sm()
                                    .text_color(rgb(if note.is_some() {
                                        pal.accent
                                    } else {
                                        pal.muted
                                    }))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |_app, _, window, cx| {
                                        cx.stop_propagation();
                                        open_note_dialog(
                                            window,
                                            cx,
                                            path_note.clone(),
                                            note_text.clone(),
                                        );
                                    }))
                                    .child("✎"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .text_color(rgb(pal.muted))
                            .child(format!("{} {}", t("变更", "changed"), changed))
                            .child("·")
                            .child(modified)
                            .when_some(note, |this, note| {
                                this.child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(format!("✎ {note}")),
                                )
                            }),
                    )
            })
            .collect::<Vec<_>>();

        div()
            .w(px(280.0))
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
                    .py_1()
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("🗂 {} ({})", t("工作区", "Workspaces"), paths.len())),
                    )
                    .child(ui::button(
                        "rail-sort",
                        match sort_mode.as_str() {
                            "name" => t("⇅ 名称", "⇅ Name"),
                            "changed" => t("⇅ 变更", "⇅ Changes"),
                            _ => t("⇅ 手动", "⇅ Manual"),
                        },
                        pal,
                        true,
                        cx.listener(|app, _, _, cx| {
                            let next = match app.shared.settings.workspace_sort_mode.as_str() {
                                "manual" => "name",
                                "name" => "changed",
                                _ => "manual",
                            };
                            app.shared.settings.workspace_sort_mode = next.to_string();
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    ))
                    .child(ui::button(
                        "rail-refresh",
                        "🔄",
                        pal,
                        !self.running,
                        cx.listener(|app, _, _, cx| {
                            app.rail.refresh(&app.shared, cx);
                        }),
                    ))
                    .child(ui::button(
                        "rail-collapse",
                        "«",
                        pal,
                        true,
                        cx.listener(|app, _, _, cx| {
                            app.shared.settings.workspace_list_collapsed = true;
                            app.shared.settings.save();
                            cx.notify();
                        }),
                    )),
            )
            .child(ui::button(
                "rail-open",
                format!("📂 {}", t("打开…", "Open…")),
                pal,
                true,
                cx.listener(|app, _, _, cx| app.pick_workspace(cx)),
            ))
            .child(ui::sep(pal))
            .child(
                div()
                    .id("rail-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(rows)
                    .when(paths.is_empty(), |this| {
                        this.child(
                            div()
                                .p_2()
                                .text_xs()
                                .text_color(rgb(pal.muted))
                                .whitespace_normal()
                                .child(t(
                                    "还没有最近打开的工作副本",
                                    "No recent working copies yet",
                                )),
                        )
                    }),
            )
            .into_any_element()
    }
}

fn move_recent(shared: &mut Shared, path: &str, delta: isize) {
    let Some(index) = shared.settings.recent.iter().position(|item| item == path) else {
        return;
    };
    let target = index as isize + delta;
    if target < 0 || target as usize >= shared.settings.recent.len() {
        return;
    }
    shared.settings.recent.swap(index, target as usize);
    shared.settings.save();
}

pub fn workspace_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn format_modified(millis: u64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_millis_opt(millis as i64) {
        chrono::LocalResult::Single(time) => time.format("%m-%d %H:%M").to_string(),
        _ => "—".to_string(),
    }
}

/// Shared "edit workspace note" dialog, used by the rail and the settings page.
pub fn open_note_dialog(
    window: &mut Window,
    cx: &mut Context<OrcaApp>,
    path: String,
    note: String,
) {
    let state = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder(t("工作区备注", "Workspace note"))
            .default_value(note)
    });
    let entity = cx.entity();
    window.open_dialog(cx, move |dialog, _window, _cx| {
        let entity = entity.clone();
        let path = path.clone();
        let state = state.clone();
        let content_state = state.clone();
        dialog
            .title(t("编辑备注", "Edit note"))
            .content(move |content, _window, _cx| {
                content.child(Input::new(&content_state).w_full())
            })
            .footer(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("note-cancel")
                            .label(t("取消", "Cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        Button::new("note-save")
                            .primary()
                            .label(t("保存", "Save"))
                            .on_click(move |_, window, cx| {
                                let value = state.read(cx).value().trim().to_string();
                                entity.update(cx, |app, cx| {
                                    if value.is_empty() {
                                        app.shared.settings.workspace_notes.remove(&path);
                                    } else {
                                        app.shared
                                            .settings
                                            .workspace_notes
                                            .insert(path.clone(), value);
                                    }
                                    app.shared.settings.save();
                                    cx.notify();
                                });
                                window.close_dialog(cx);
                            }),
                    ),
            )
    });
}
