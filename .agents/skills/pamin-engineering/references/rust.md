# Rust

How Rust code meets `core.md` and `performance.md`; read those first. Like the rest of this skill, it changes only in `pamin-labs/skills`.

## Toolchain and build

- **Toolchain and lints.** Edition and toolchain are pinned in `rust-toolchain.toml` and upgraded as their own change; code is `rustfmt`-clean and `clippy`-clean with warnings denied (`cargo clippy --all-targets -- -D warnings`), and `cargo deny` checks licences and bans. A lint beyond clippy's defaults (a `clippy::pedantic` lint, `missing_docs`) is enabled in `[lints]` only after its findings on the current tree are counted and found few and worth fixing, and the enabling PR fixes them all.
- **Build footprint.** A shared `CARGO_TARGET_DIR` keeps disk use down; dev profiles use `debug = "line-tables-only"`.
- **Dependency features.** Each dependency is declared once (in `[workspace.dependencies]` in a workspace) with only the features the code calls; `tokio`, `axum` and other large crates never take `full` or unused defaults. A new dependency's added clean build time (`cargo build --timings`) and release binary size are measured and recorded in the PR.
- **Published crates** (ones another repository depends on) keep semver: a public struct or enum that may grow is `#[non_exhaustive]` or has private fields and a constructor, and a breaking change is called out in the PR and released as one. Features are additive (enabling one never removes or changes what another relies on), and the defaults are the smallest set a typical user needs.

## Types and errors

- **Errors.** A library declares typed errors with `thiserror`, one enum per boundary, with variants a caller can act on; `anyhow` or a boxed error only in binaries and tests. Errors travel with `?`, each layer adding what it was doing (a variant's field, or `.context(...)` in a binary).
- **No panic reachable from input.** `unwrap`, `expect`, `panic!`, `unreachable!`, indexing or slicing that can go out of bounds, and arithmetic that can overflow stay off every path a request, a file or another service's answer reaches. They are allowed in tests, at start-up before serving, and where an invariant rules the failure out, with an `expect` message stating it.
- **`unsafe`** only where no safe API gives the result (FFI, a measured SIMD kernel), in the smallest block, with a `// SAFETY:` comment naming every invariant it relies on and a test that exercises it. A new crate that needs none starts with `#![forbid(unsafe_code)]`.
- **Types over strings and integers.** Ids, units and amounts are newtypes or standard types (`struct TenantId(Uuid)`, `Duration`), never a bare `String`, `i64` or `u64`; a closed set is an enum, never a string compared in code. Input is parsed at the boundary with `serde`, `FromStr` or `TryFrom`.
- **Typed JSON.** Bodies are `serde` types; walking a `serde_json::Value` by hand, `json!` for a body of known shape and `format!` building JSON are defects. A value a protocol requires echoed unchanged is a `Box<RawValue>`, and the code says why.
- **Borrow before cloning.** A `.clone()` added on a hot path to satisfy the borrow checker is a defect: restructure, borrow, or share an `Arc` and say why. Parameters take `&str`, `&[T]` or `impl AsRef<_>`; functions return owned values.

## Async and concurrency

- **No blocking in async.** No blocking file or network call, `std::thread::sleep`, long CPU-bound loop or contended `std::sync` lock on an async task: use the async API, `spawn_blocking` or a dedicated thread. No lock guard is held across `.await` (clippy's `await_holding_lock` stays on); a `tokio::sync` lock only when the guard must cross one.
- **Everything queued is bounded.** Channels, queues, caches and per-key maps have a bound named once as a constant or setting, and the code says what happens at it (wait, refuse or evict); an unbounded channel is a defect outside tests.
- **Tasks are owned.** Every spawned task's `JoinHandle` is kept (or the task lives in a `JoinSet` or `TaskTracker`) and awaited or aborted by its owner, so its panic or error is seen. Every `tokio::select!` branch is cancellation-safe, or a comment says why the lost work does not matter. A long-running service shuts down gracefully on SIGTERM: it stops accepting, finishes or hands over work in flight within a deadline, flushes its buffers, then exits.
- **Shared state** is `Send + Sync`; on a hot path prefer atomics or message passing to a lock, and never take two locks in different orders.

## Logs, SQL and tests

- **Logs** go through `tracing` with structured fields; a type holding a secret redacts it in `Debug`.
- **SQL** (`sqlx`) binds every value (`$1`, `.bind`); a query built with `format!` from input is a defect, and what cannot be bound (an identifier) comes from a closed list in code.
- **Tests.** Unit tests sit beside the code in `#[cfg(test)] mod tests`; tests of a crate's behaviour through its public API go in its `tests/` directory. Tests are deterministic and independent: no `sleep` to wait (await the event, poll with a deadline, or pause tokio's clock), no dependence on test order, the wall clock, the network or another test's state; anything random uses a fixed seed. A fix comes with a test that fails without it.

## Performance mechanisms

- **Allocate once:** reuse buffers with `clear()`, size them with `with_capacity` from a known size; no `collect()` into a `Vec` only to iterate it once; no `to_string()` or `format!` for a value only compared or written out.
- **Static dispatch** is generics or `impl Trait`; `dyn` only at boundaries crossed once per request or less.
