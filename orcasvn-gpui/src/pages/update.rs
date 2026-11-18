//! Update page (mirrors `main`'s `UpdateView`): check remote revisions, update
//! to HEAD or a chosen revision, surface conflicts.

use super::Shared;
use crate::app::OrcaApp;
use crate::i18n::t;
use crate::svn;
use crate::task;
use crate::ui::{self, Palette, TextField};
use crate::SvnLogEntry;
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct UpdatePage {
    loaded_for: Option<String>,
    revision: TextField,
    running: bool,
    checking: bool,
    local: Option<u64>,
    remote: Option<u64>,
    incoming: Vec<SvnLogEntry>,
    output: Option<String>,
    conflicts: Vec<String>,
}

impl UpdatePage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        Self {
            loaded_for: None,
            revision: TextField::new(cx, window, "HEAD"),
            running: false,
            checking: false,
            local: None,
            remote: None,
            incoming: Vec::new(),
            output: None,
            conflicts: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.loaded_for = None;
        self.revision.clear();
        self.running = false;
        self.checking = false;
        self.local = None;
        self.remote = None;
        self.incoming.clear();
        self.output = None;
        self.conflicts.clear();
    }

    fn load_local(&mut self, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.loaded_for.clone() else {
            return;
        };
        task::spawn(
            cx,
            async move {
                let result = svn::local_revision(&workspace)
                    .await
                    .map_err(|error| error.to_string());
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                if app.update.loaded_for.as_deref() == Some(workspace.as_str()) {
                    if let Ok(revision) = result {
                        app.update.local = Some(revision);
                    }
                }
                cx.notify();
            },
        );
    }

    fn check(&mut self, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.loaded_for.clone() else {
            return;
        };
        if self.checking {
            return;
        }
        self.checking = true;
        task::spawn(
            cx,
            async move {
                let result = async {
                    let info = svn::remote_info(&workspace)
                        .await
                        .map_err(|error| error.to_string())?;
                    let local = svn::local_revision(&workspace)
                        .await
                        .map_err(|error| error.to_string())?;
                    let incoming = svn::incoming_log(&workspace, Some(50), info.revision, local)
                        .await
                        .map_err(|error| error.to_string())?;
                    Ok::<_, String>((info.revision, local, incoming))
                }
                .await;
                (workspace, result)
            },
            |app, (workspace, result), cx| {
                app.update.checking = false;
                if app.update.loaded_for.as_deref() != Some(workspace.as_str()) {
                    return;
                }
                match result {
                    Ok((remote, local, incoming)) => {
                        app.update.remote = Some(remote);
                        app.update.local = Some(local);
                        app.update.incoming = incoming;
                    }
                    Err(error) => app.shared.err(error),
                }
                cx.notify();
            },
        );
    }

    fn update(&mut self, cx: &mut Context<OrcaApp>) {
        let Some(workspace) = self.loaded_for.clone() else {
            return;
        };
        if self.running {
            return;
        }
        let text = self.revision.text().trim().to_string();
        let revision = text.trim_start_matches('r').parse::<u64>().ok();
        if revision.is_none() && !text.is_empty() {
            self.output = Some(t("无效的版本号", "Invalid revision").to_string());
            return;
        }
        self.running = true;
        task::spawn(
            cx,
            async move {
                let result = svn::update(&workspace, revision)
                    .await
                    .map_err(|error| error.to_string());
                (workspace, result)
            },
            |app, (_workspace, result), cx| {
                app.update.running = false;
                match result {
                    Ok(output) => {
                        app.update.conflicts = svn::parse_update_conflicts(&output);
                        app.shared.done("update", &output);
                        app.update.output = Some(output);
                        app.shared.info_dirty = true;
                        app.update.load_local(cx);
                    }
                    Err(error) => app.shared.err(error),
                }
                cx.notify();
            },
        );
    }

    pub fn render(
        &mut self,
        shared: &Shared,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let Some(workspace) = shared.workspace.clone() else {
            return div().into_any_element();
        };
        if self.loaded_for.as_deref() != Some(workspace.as_str()) {
            self.reset();
            self.loaded_for = Some(workspace.clone());
            self.load_local(cx);
        }

        let local = self
            .local
            .map(|revision| format!("r{revision}"))
            .unwrap_or_else(|| "—".to_string());
        let remote = self
            .remote
            .map(|revision| format!("r{revision}"))
            .unwrap_or_else(|| "—".to_string());

        let revisions = self
            .incoming
            .iter()
            .map(|entry| {
                let date = entry
                    .date
                    .get(..16)
                    .unwrap_or(&entry.date)
                    .replace('T', " ");
                (
                    entry.revision,
                    format!("r{}  ·  {}  ·  {date}", entry.revision, entry.author),
                    entry.message.lines().next().unwrap_or_default().to_string(),
                )
            })
            .collect::<Vec<_>>();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .child(
                        div()
                            .text_sm()
                            .child(format!("{} {local}", t("本地版本", "Local"))),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("{} {remote}", t("远端版本", "Remote"))),
                    )
                    .child(ui::sep_vertical(pal))
                    .child(
                        div()
                            .w(px(110.0))
                            .child(self.revision.input(window, cx).w_full()),
                    )
                    .child(ui::button(
                        "update-run",
                        format!("⬇ {}", t("更新", "Update")),
                        pal,
                        !self.running,
                        cx.listener(|app, _, _, cx| app.update.update(cx)),
                    ))
                    .child(ui::button(
                        "update-check",
                        format!("🔍 {}", t("检查更新", "Check")),
                        pal,
                        !self.checking && !self.running,
                        cx.listener(|app, _, _, cx| app.update.check(cx)),
                    ))
                    .when(self.running, |this| {
                        this.child(ui::spinner(pal, "svn update…"))
                    })
                    .when(self.checking, |this| {
                        this.child(ui::spinner(pal, t("查询远端…", "Checking remote…")))
                    }),
            )
            .child(ui::sep(pal))
            .when(!self.conflicts.is_empty(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_sm()
                        .text_color(rgb(pal.warn))
                        .child(format!(
                            "⚠ {}: {}",
                            t("更新完成，但存在冲突", "Update finished with conflicts"),
                            self.conflicts.join(", ")
                        )),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(pal.muted))
                            .child(t("待更新版本", "Incoming revisions")),
                    )
                    .child(
                        div()
                            .id("update-incoming")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .children(revisions.into_iter().map(|(revision, meta, summary)| {
                                div()
                                    .id(("update-rev", revision as usize))
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .px_3()
                                    .py_2()
                                    .border_b_1()
                                    .border_color(rgb(pal.border))
                                    .child(div().text_xs().text_color(rgb(pal.muted)).child(meta))
                                    .child(div().text_sm().child(summary))
                            }))
                            .when(self.incoming.is_empty() && !self.checking, |this| {
                                this.child(div().p_6().text_sm().text_color(rgb(pal.muted)).child(
                                    t(
                                        "没有检测到待更新版本，点击“检查更新”查询远端",
                                        "No incoming revisions; press Check to query the remote",
                                    ),
                                ))
                            }),
                    ),
            )
            .when_some(self.output.clone(), |this, output| {
                this.child(ui::sep(pal)).child(
                    div()
                        .id("update-output")
                        .h(px(140.0))
                        .overflow_y_scroll()
                        .p_3()
                        .font_family(ui::mono_font())
                        .text_xs()
                        .whitespace_normal()
                        .child(output),
                )
            })
            .into_any_element()
    }
}
