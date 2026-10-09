//! Cooperation proofs: tasks spawned from sync land (global fallback runtime)
//! and tasks spawned inside `#[tokio::main]`-style runtimes (ambient runtime)
//! must interoperate — channels are runtime-agnostic, and that's the whole game.

use std::time::{Duration, Instant};

#[test]
fn go_from_sync_land_uses_global_fallback() {
    // Prove there is genuinely no ambient runtime here.
    assert!(tokio::runtime::Handle::try_current().is_err());

    let (tx, mut rx) = grt::channel::<u32>(8);
    let handle = grt::go!(async move {
        tx.send(42).await.unwrap();
    });
    grt::block_on(async move {
        assert_eq!(rx.recv().await, Some(42));
        handle.await.unwrap();
    });
}

#[tokio::test]
async fn go_inside_tokio_rides_the_ambient_runtime() {
    let ambient = tokio::runtime::Handle::current();
    let same_runtime = grt::go!(async move {
        tokio::runtime::Handle::try_current().unwrap().id() == ambient.id()
    });
    assert!(
        same_runtime.await.unwrap(),
        "go! inside tokio must use the ambient runtime, not the global fallback"
    );
}

#[tokio::test]
async fn cross_runtime_tasks_cooperate_over_one_channel() {
    let (tx, mut rx) = grt::channel::<&'static str>(1);
    // A bare thread has no ambient runtime, so go! falls back to global.
    // The test body runs on the ambient runtime. One channel bridges them.
    let thread = std::thread::spawn(move || {
        let h = grt::go!(async move {
            tx.send("ping").await.unwrap();
        });
        grt::block_on(h).unwrap();
    });
    assert_eq!(rx.recv().await, Some("ping"));
    thread.join().unwrap();
}

#[test]
fn blocking_work_overlaps_async_work() {
    // go_blocking! must not serialize behind async work on the same runtime.
    let start = Instant::now();
    grt::block_on(async {
        let blocking = grt::go_blocking!(|| {
            std::thread::sleep(Duration::from_millis(400));
        });
        let snooze = grt::go!(async {
            grt::sleep(Duration::from_millis(400)).await;
        });
        blocking.await.unwrap();
        snooze.await.unwrap();
    });
    let elapsed = start.elapsed();
    // Serialized, this would take ~800ms. Overlapped, ~400ms.
    assert!(
        elapsed < Duration::from_millis(700),
        "blocking and async work did not overlap: took {elapsed:?}"
    );
}
