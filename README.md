# grt

Goroutine-flavored ergonomics as a thin facade over [tokio](https://tokio.rs).
Don't build a scheduler — borrow his.

```rust
// From anywhere — plain `fn main`, a bare thread, or inside tokio:
let handle = grt::go!(async {
    println!("hello from a goroutine-ish task");
});

// Blocking code never parks a tokio worker:
let result = grt::go_blocking!(|| {
    std::thread::sleep(std::time::Duration::from_secs(1));
    42
}).await.unwrap();

// Go-style channels (tokio's mpsc in a trench coat):
let (tx, mut rx) = grt::channel::<String>(16);
```

## The idea

`go!` spawns onto the **ambient** tokio runtime when one exists
(`#[tokio::main]`, `#[tokio::test]`, any async context), and onto a single
**immortal global runtime** otherwise — so it can be called from anywhere,
like Go's `go` statement. Channels are runtime-agnostic, so tasks from both
worlds cooperate over the same channel.

## Honest leaks

- **No preemption.** A task that never `.await`s starves its worker thread.
  Go would signal-preempt it; this cannot.
- **`Send` is required**, like `tokio::spawn`. `!Send` futures need
  `tokio::task::LocalSet` + `spawn_local` — the abstraction won't lie to you.
- **The global fallback runtime is immortal.** Process exit kills in-flight
  tasks, exactly like Go's scheduler. Faithful, warts included.

## Prior art

`goroutine`, `go-lib`, `goish`, `gorust`, `go`, and most completely
[`may`](https://github.com/Xudong-Huang/may) (stackful coroutines, its own
scheduler). All niche; none mainstream. This crate's lane is narrower: a
~150-line facade that refuses to build a scheduler and just borrows tokio's.

## License

MIT — see [LICENSE](LICENSE).
