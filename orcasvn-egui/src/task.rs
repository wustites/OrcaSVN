use std::future::Future;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
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

/// A background SVN operation whose result is polled from the UI thread.
pub struct Task<T>(Option<Receiver<Result<T, String>>>);

impl<T> Default for Task<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: Send + 'static> Task<T> {
    pub fn spawn<F>(ctx: &egui::Context, future: F) -> Self
    where
        F: Future<Output = Result<T, String>> + Send + 'static,
    {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        runtime().spawn(async move {
            let _ = tx.send(future.await);
            ctx.request_repaint();
        });
        Self(Some(rx))
    }

    pub fn running(&self) -> bool {
        self.0.is_some()
    }

    pub fn poll(&mut self) -> Option<Result<T, String>> {
        let result = match self.0.as_ref()?.try_recv() {
            Ok(value) => value,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("task aborted".to_string()),
        };
        self.0 = None;
        Some(result)
    }
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    runtime().block_on(future)
}
