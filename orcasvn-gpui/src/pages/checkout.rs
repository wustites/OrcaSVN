use super::{Page, Shared};
use crate::app::OrcaApp;
use crate::i18n::t;
use crate::svn;
use crate::task;
use crate::ui::{self, Palette, TextField};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct CheckoutPage {
    url: TextField,
    path: TextField,
    revision: TextField,
    running: bool,
    target: String,
}

impl CheckoutPage {
    pub fn new(cx: &mut Context<OrcaApp>, window: &mut Window) -> Self {
        Self {
            url: TextField::new(cx, window, "https://svn.example.com/repo/trunk"),
            path: TextField::new(cx, window, ""),
            revision: TextField::new(cx, window, "HEAD"),
            running: false,
            target: String::new(),
        }
    }

    pub fn render(
        &mut self,
        _shared: &Shared,
        window: &mut Window,
        cx: &mut Context<OrcaApp>,
        pal: Palette,
    ) -> AnyElement {
        let busy = self.running;
        let ready = !busy && !self.url.is_empty() && !self.path.is_empty();
        let url_for_run = self.url.text().trim().to_string();
        let path_for_run = self.path.text().trim().to_string();
        let revision_for_run = self.revision.text().trim().to_string();

        let browse = ui::button(
            "checkout-browse",
            t("浏览…", "Browse…"),
            pal,
            true,
            cx.listener(|_app, _, _, cx| {
                task::spawn_blocking(
                    cx,
                    || rfd::FileDialog::new().pick_folder(),
                    |app, picked, cx| {
                        if let Some(directory) = picked {
                            app.checkout
                                .path
                                .set_text(directory.to_string_lossy().into_owned());
                        }
                        cx.notify();
                    },
                );
            }),
        );

        let form = div()
            .flex()
            .flex_col()
            .gap_3()
            .max_w(px(620.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().child(t("仓库 URL", "Repository URL")))
                    .child(self.url.input(window, cx).w_full()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().child(t("本地目录", "Local directory")))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(self.path.input(window, cx).w_full()),
                            )
                            .child(browse),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().child(t("版本", "Revision")))
                    .child(
                        div()
                            .w(px(160.0))
                            .child(self.revision.input(window, cx).w_full()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::button(
                        "checkout-run",
                        format!("⬇ {}", t("检出", "Check out")),
                        pal,
                        ready,
                        cx.listener(move |app, _, _, cx| {
                            let revision = revision_for_run.trim().trim_start_matches('r');
                            let revision = if revision.is_empty() {
                                None
                            } else if let Ok(value) = revision.parse() {
                                Some(value)
                            } else {
                                app.shared.err(t("无效的版本号", "Invalid revision"));
                                cx.notify();
                                return;
                            };
                            let (url, path) = (url_for_run.clone(), path_for_run.clone());
                            app.checkout.target = path.clone();
                            app.checkout.running = true;
                            task::spawn(
                                cx,
                                async move {
                                    svn::checkout(&url, &path, revision)
                                        .await
                                        .map_err(|error| error.to_string())
                                },
                                |app, result, cx| {
                                    app.checkout.running = false;
                                    match result {
                                        Ok(output) => {
                                            app.shared.done("checkout", &output);
                                            app.shared.open_workspace =
                                                Some(app.checkout.target.clone());
                                            app.shared.goto = Some(Page::Workspace);
                                        }
                                        Err(error) => app.shared.err(error),
                                    }
                                    cx.notify();
                                },
                            );
                        }),
                    ))
                    .when(busy, |this| {
                        this.child(ui::spinner(pal, t("正在检出…", "Checking out…")))
                    }),
            );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_6()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(t("检出工作副本", "Check out a working copy")),
            )
            .child(form)
            .into_any_element()
    }
}
