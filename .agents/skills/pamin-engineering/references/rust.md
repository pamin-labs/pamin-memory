# Rust

Read this when writing or reviewing Rust. It says how Rust code meets `core.md`, `data.md` and `performance.md`; read those first.

## Toolchain and build

- **Toolchain and lints.** Edition and toolchain are pinned in `rust-toolchain.toml` and upgraded as their own change; code is `rustfmt`-clean and `clippy`-clean with warnings denied (`cargo clippy --all-targets -- -D warnings`), and `cargo deny` checks licences and bans. A lint beyond clippy's defaults (a `clippy::pedantic` lint, `missing_docs`) is enabled in `[lints]` only after its findings on the current tree are counted and found few and worth fixing, and the enabling PR fixes them all.
- **Build footprint.** A shared `CARGO_TARGET_DIR` keeps disk use down; dev profiles use `debug = "line-tables-only"`.
- **Unreleased shared crates** are developed against sibling clones through a git-ignored `.cargo/config.toml` `[patch]` section; path or submodule dependencies are never committed, because they break every other checkout and the release build.
- **Dependency features.** Each dependency is declared once (in `[workspace.dependencies]` in a workspace) with only the features the code calls; `tokio`, `axum` and other large crates never take `full` or unused defaults. A new dependency's added build time is measured with `cargo build --timings`.
- **Published crates** (ones another repository depends on) keep semver: a public struct or enum that may grow is `#[non_exhaustive]` or has private fields and a constructor, and a breaking change is called out in the PR and released as one. Features are additive (enabling one never removes or changes what another relies on), and the defaults are the smallest set a typical consumer needs.

## Modules and files

- **Module layout.** A module with sub-modules is `name.rs` beside a `name/` directory, never `name/mod.rs` or `name/name.rs`. Outside code uses the items `name.rs` re-exports, never a path into `name/`. Types several files of a module share go in `name/types.rs`.
- **The layer test** reads `cargo metadata` in a workspace and fails on a crate dependency that points back; in a single crate it reads each file's `use` paths.
- **File order** is core's in Rust terms: `use` lines, std first, then external crates, then the crate's own; types; `const` and `static`; `impl` blocks grouped by type, constructors first, public methods before private; free functions in rough call order; `#[cfg(test)] mod tests` last.

## Types and errors

- **Errors.** A library declares typed errors with `thiserror`, one enum per boundary, with variants a caller can act on; `anyhow` or a boxed error only in binaries and tests. Errors travel with `?`, each layer adding what it was doing (a variant's field, or `.context(...)` in a binary).
- **Error kinds.** An error a client is told about exposes its kind and stable code through one method, which transport maps to an answer; transport never matches a domain's variants.
- **No panic reachable from input.** `unwrap`, `expect`, `panic!`, `unreachable!`, indexing or slicing that can go out of bounds, and arithmetic that can overflow stay off every path a request, a file or another service's answer reaches. They are allowed in tests, at start-up before serving, and where an invariant rules the failure out, with an `expect` message stating it.
- **`unsafe`** only where no safe API gives the result (FFI, a measured SIMD kernel), in the smallest block, with a `// SAFETY:` comment naming every invariant it relies on and a test that exercises it. A new crate that needs none starts with `#![forbid(unsafe_code)]`.
- **Types over strings and integers.** Ids, units and amounts are newtypes or standard types (`struct TenantId(Uuid)`, `Duration`), never a bare `String`, `i64` or `u64`; a closed set is an enum, never a string compared in code. Input is parsed at the boundary with `serde`, `FromStr` or `TryFrom`.
- **A closed set's list** is one `const ALL: &[Self]` on its enum, from which wire enums and schemas are built.
- **Typed JSON.** Bodies are `serde` types; walking a `serde_json::Value` by hand, `json!` for a body of known shape and `format!` building JSON are defects. A value a protocol requires echoed unchanged is a `Box<RawValue>`, and the code says why.
- **Settings** are parsed once at start into a typed config; a missing or invalid one is an error naming it, never an `unwrap_or` default.
- **Borrow before cloning.** A `.clone()` added on a hot path to satisfy the borrow checker is a defect: restructure, borrow, or share an `Arc` and say why. Parameters take `&str`, `&[T]` or `impl AsRef<_>`; functions return owned values.

## Async and concurrency

- **No blocking in async.** No blocking file or network call, `std::thread::sleep`, long CPU-bound loop or contended `std::sync` lock on an async task: use the async API, `spawn_blocking` or a dedicated thread. No lock guard is held across `.await` (clippy's `await_holding_lock` stays on); a `tokio::sync` lock only when the guard must cross one.
- **Deadlines.** Each call a request waits on is wrapped in `tokio::time::timeout` with a `Duration` named once beside it.
- **Everything queued is bounded.** Channels, queues, caches and per-key maps have a bound named once as a constant or setting, and the code says what happens at it (wait, refuse or evict); an unbounded channel is a defect outside tests.
- **Tasks are owned.** Every spawned task's `JoinHandle` is kept (or the task lives in a `JoinSet` or `TaskTracker`) and awaited or aborted by its owner, so its panic or error is seen. Every `tokio::select!` branch is cancellation-safe, or a comment says why the lost work does not matter. A long-running service shuts down gracefully on SIGTERM: it stops accepting, finishes or hands over work in flight within a deadline, flushes its buffers, then exits.
- **Shared state** has one owner: a lock wraps exactly the fields it guards, beside them in one struct (`Mutex<Inner>`). On a hot path prefer atomics or message passing to a lock, and never take two locks in different orders.

## Logs, SQL, stubs and tests

- **Logs** go through `tracing` with structured fields carrying ids, counts and durations only; a type holding a secret redacts it in `Debug`.
- **SQL** (`sqlx`) lives in `.sql` files read with `sqlx::query_file!` or `query_file_as!` (checked at compile time) or `include_str!`, never as a string literal in Rust, and binds every value (`$1`, `.bind`); a query built with `format!` from input is a defect, and what cannot be bound (an identifier) comes from a closed list in code.
- **Stubs** compile only under `#[cfg(test)]` or a non-default feature that tests, the benchmark and the local runner enable.
- **Tests.** Unit tests sit beside the code in `#[cfg(test)] mod tests`; tests of a crate's behaviour through its public API go in its `tests/` directory. Tests are deterministic and independent: no `sleep` to wait (await the event, poll with a deadline, or pause tokio's clock), no dependence on test order, the wall clock, the network or another test's state; anything random uses a fixed seed. A fix comes with a test that fails without it.
- **Fuzzing.** Parsers of outside input are fuzzed with `cargo fuzz` or property-tested with `proptest`; a crash found becomes a regression test.

## Performance mechanisms

- **Allocate once:** reuse buffers with `clear()`, size them with `with_capacity` from a known size; no `collect()` into a `Vec` only to iterate it once; no `to_string()` or `format!` for a value only compared or written out.
- **Static dispatch** is generics or `impl Trait`; `dyn` only at boundaries crossed once per request or less.
