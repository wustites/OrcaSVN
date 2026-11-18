//! Bridging between GPUI (smol-based) and the tokio runtime the SVN backend
//! expects. SVN operations are awaited on a shared tokio runtime running on a
//! GPUI background thread, and their results are delivered back to the view on
//! the foreground thread.

use gpui_kit::{AppContext as _, AsyncApp, Context};
use std::future::Future;
use std::sync::OnceLock;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
    })
}

/// Run `future` on the tokio runtime and hand its output to `on_done` on the
/// GPUI foreground thread.
pub fn spawn<T, F, H>(cx: &mut Context<T>, future: F, on_done: H)
where
    T: 'static,
    F: Future + Send + 'static,
    F::Output: Send + 'static,
    H: FnOnce(&mut T, F::Output, &mut Context<T>) + 'static,
{
    cx.spawn(async move |this, cx: &mut AsyncApp| {
        let output = cx
            .background_spawn(async move { runtime().block_on(future) })
            .await;
        this.update(cx, |view, cx| on_done(view, output, cx)).ok();
    })
    .detach();
}

/// Run a blocking closure (file dialogs, filesystem walks) on the GPUI
/// background executor and deliver the result on the foreground thread.
pub fn spawn_blocking<T, R, F, H>(cx: &mut Context<T>, work: F, on_done: H)
where
    T: 'static,
    R: Send + 'static,
    F: FnOnce() -> R + Send + 'static,
    H: FnOnce(&mut T, R, &mut Context<T>) + 'static,
{
    cx.spawn(async move |this, cx: &mut AsyncApp| {
        let output = cx.background_spawn(async move { work() }).await;
        this.update(cx, |view, cx| on_done(view, output, cx)).ok();
    })
    .detach();
}
