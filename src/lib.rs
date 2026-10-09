//! `grt`: goroutine-flavored ergonomics as a thin facade over tokio.
//!
//! # The idea
//! Don't build a scheduler — borrow tokio's. `go!` spawns onto the *ambient*
//! tokio runtime when one exists, and onto a single immortal global runtime
//! otherwise, so it can be called from anywhere, like Go's `go` statement.
//!
//! # Honest leaks
//! - **No preemption.** A task that never `.await`s starves its worker thread.
//!   Go would signal-preempt it; this cannot. Schedule accordingly.
//! - **`Send` is required**, like `tokio::spawn`. `!Send` futures need
//!   `tokio::task::LocalSet` + `spawn_local` — the abstraction won't lie to you.
//! - **The global fallback runtime is immortal.** Process exit kills in-flight
//!   tasks, exactly like Go's scheduler. Faithful, warts included.

use std::{future::Future, sync::OnceLock, time::Duration};

use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};

/// The immortal global runtime. Built once, never shut down.
fn global_runtime() -> &'static Runtime {
    static GLOBAL: OnceLock<Runtime> = OnceLock::new();
    GLOBAL.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .thread_name("grt-worker")
            .enable_all()
            .build()
            .expect("grt: failed to build global runtime")
    })
}

/// Spawn a future like a goroutine.
///
/// - Inside a tokio runtime (`#[tokio::main]`, `#[tokio::test]`, or any async
///   context), the task rides the **ambient** runtime.
/// - Anywhere else (plain `fn main`, a bare thread), it lands on grt's
///   **global** runtime.
pub fn go<F>(fut: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    match Handle::try_current() {
        Ok(handle) => handle.spawn(fut),
        Err(_) => global_runtime().spawn(fut),
    }
}

/// Run a blocking closure without parking a tokio worker — Go's transparent
/// blocking, minus the magic. Blocking code goes to the blocking pool.
pub fn go_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match Handle::try_current() {
        Ok(handle) => handle.spawn_blocking(f),
        Err(_) => global_runtime().spawn_blocking(f),
    }
}

/// Drive a future to completion on the global runtime, from sync code.
///
/// Panics if called inside an async context (inherited from tokio).
pub fn block_on<F>(fut: F) -> F::Output
where
    F: Future,
{
    global_runtime().block_on(fut)
}

/// A Go-style channel: `tokio::sync::mpsc` in a trench coat.
pub fn channel<T>(
    buffer: usize,
) -> (
    tokio::sync::mpsc::Sender<T>,
    tokio::sync::mpsc::Receiver<T>,
) {
    tokio::sync::mpsc::channel(buffer)
}

/// Sleep without blocking a worker. Must be called in async context.
pub async fn sleep(d: Duration) {
    tokio::time::sleep(d).await;
}

/// `tokio::select!`, re-exported so the whole vocabulary lives in one place.
pub use tokio::select;

/// Spawn a future like a goroutine. See [`go`].
#[macro_export]
macro_rules! go {
    ($fut:expr) => {
        $crate::go($fut)
    };
}

/// Run a blocking closure off the workers. See [`go_blocking`].
#[macro_export]
macro_rules! go_blocking {
    ($f:expr) => {
        $crate::go_blocking($f)
    };
}
