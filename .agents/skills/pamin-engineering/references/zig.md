# Zig

Read this when writing or reviewing Zig. It says how Zig code meets `core.md`, `data.md` and `performance.md`; read those first. Every rule of `core.md` applies in full; this file says how Zig code meets each one, and adds the rules particular to Zig. The std names below are those of the Zig release named in `build.zig.zon`, and move with it.

## Toolchain and build

- **Version.** The Zig release is named once, as `minimum_zig_version` in `build.zig.zon`, and CI installs exactly it. Moving to a new release is its own change, because std APIs change between releases. Code is `zig fmt`-clean (`zig fmt --check .` in the gates).
- **Checks beyond `zig fmt` and the compiler** (a linter rule, or a source test that parses with `std.zig.Ast`, never a text match) are added only after their findings on the current tree are counted, and the PR adding one fixes them all.
- **Dependencies** are declared in `build.zig.zon` with a pinned `url` (a tag or commit, never a branch) and its `hash`; `zig fetch --save` writes both. Vendored C (a decoder, a header-only library) is fetched the same way and compiled by `build.zig` with the narrowest flags and only the sources the code calls; it is never copied into the tree. Before adding one, check its maintenance and licence, and measure its added clean build time and release binary size; record all four in the PR.
- **`build.zig` is the one build entry.** Every step (`test`, `bench`, `install`, each cross-compiled target) is named there, and fuzzing runs as `zig build test --fuzz`; scripts and CI call `zig build <step>`, never the compiler with flags of their own. A value both the build and the code use (version, target list, output name) is defined once in `build.zig` or `build.zig.zon` and reaches code through `b.addOptions`.
- **Generated files** are written by a `zig build` step through `b.addUpdateSourceFiles`, and a test fails when regenerating them changes them.
- **Build modes.** Tests and local runs use Debug or ReleaseSafe. A shipped binary is ReleaseSafe unless a measurement on the product's entry point shows ReleaseFast or ReleaseSmall worth losing the safety checks; the choice and its numbers are in the PR and beside the target in `build.zig`. A WebAssembly or size-bound target may default to ReleaseSmall on the same terms. A module that needs another mode than the rest gets its own `b.createModule(.{ .optimize = ... })`, never a global override.
- **Build speed and footprint.** Few, well-chosen dependencies; heavy ones (a codec, a TLS stack) only in the modules and targets that use them, so an edit rebuilds little and a target links only what it calls. Release binaries are stripped (`.strip = true`) and as small as the mode allows; a shipped binary's size budget is named in `build.zig` and checked by a step: a small Zig program run with `b.addRunArtifact`, given the installed binary's path and the budget, since `std.Build.Step`'s custom-step API changes between Zig releases.
- **Built files are never committed:** `.zig-cache/`, `zig-out/`, benchmark output and fuzzing corpora are git-ignored; small committed fixtures are the exception.
- **Published packages** keep semver: a breaking change to a public declaration, an error set, or a CLI or JSON output is called out in the PR and released as one. An enum that may grow is non-exhaustive (`_`); a new struct field has a default; adding an error to a public error set is breaking; `b.option` options are additive, with the smallest defaults. `build.zig.zon` pins dependencies by hash and has no caret ranges, so core's caret minimums have no Zig form.

## Structure: layers, domains and reuse

- **Before writing.** core's "Does it need to exist?", "One implementation per concern" and "Do not reinvent what exists" in Zig terms: search the repository first, then climb to `std` (`std.mem`, `std.sort`, `std.hash`, `std.crypto`, `std.json`, `std.simd`, `std.Io`), a package already in `build.zig.zon`, then a maintained one; the PR's "Reused" section says which. No speculative options, comptime switches or interfaces for a second use that has not appeared.
- **A module per concern, a directory per layer.** Each `.zig` file is a module named for what it holds (`png.zig`, `alignment.zig`; never `utils.zig`, `common.zig` or `helpers.zig`), covering one concern with a small public surface. A directory holds one layer or concern, and the repository's `AGENTS.md` names each directory and what may import it; a new directory updates that list, and the layer test below, in the same PR. A module with sub-modules is `name.zig` beside a `name/` directory, the way std is laid out (`std/json.zig` and `std/json/`), and outside code imports `name.zig`, never a file inside `name/`; never `name/name.zig`. Programs other than the product (a benchmark, a generator) live outside `src/` and import only the public module. A root module (`root.zig`) re-exports the public surface, importing module roots only, and a package's consumers import only it. A layer directory is a module with sub-modules too, so it has its own root beside it, and `root.zig` imports that root, never a file inside the directory:

  ```text
  src/
    root.zig          // pub const diff = @import("diff.zig"); never @import("diff/rows.zig")
    diff.zig          // the diff layer's public surface
    diff/rows.zig
    diff/regions.zig
    codec.zig
    codec/png.zig
  bench/main.zig      // @import("lib") only, the package's public module
  ```
- **Shared types inside a module.** Types several files under `name/` share go in `name/types.zig`, which those files import directly and `name.zig` re-exports; a file under `name/` never imports `../name.zig`.
- **The layout test.** A test walks `src/` (`std.Io.Dir.walk`) against the backticked paths in `AGENTS.md` and fails on a file outside them or on a named path that is gone.
- **Layers point one way:** core's entry, wiring, transport, domain, then store and integrations, then values. In a Zig program: `main.zig` is the entry and wiring; a CLI's argument parser, a WebAssembly export or a request handler is transport; the logic is domain; a module that owns persisted files or a database is the store; a C library, the network or another program is an integration behind the one module that owns it; shared plain types are values. A test in `zig build test` reads each source file's `@import`s and fails on one that points back toward the entry, that skips a layer the way the next rule forbids, or that imports a C library's translated module from outside its owner. The same test fails on a domain's `pub` signature that names a store type, and on a store function called from two domain modules.
- **Transport only translates.** A CLI command, an export or a handler parses its input into typed values, calls one domain function, and shapes its result as output; any rule there (a threshold, a limit, a refusal) is a bug. It never imports an integration or the store directly; the layer test checks it.
- **Each handler group sees only its own components.** Each CLI subcommand or route group is in its own file and is given only the components it calls.
- **Each layer converts at its edge.** A C library's structs and error codes stop at its binding; a decoder's output becomes the domain's own image or record type; the domain's types become the output's (JSON, a report) in transport. A change inside one layer then fails to compile at a conversion instead of leaking through.
- **A domain module holds its types, errors and rules,** depends only on the store, on integrations through an interface it declares, on other domain modules' public functions, and on values; never on a CLI, an export or a C library's module.
- **Interfaces are declared by their consumer** with only what it calls: a comptime parameter whose required declarations are checked with `@hasDecl` and reported with `@compileError`, or, at a boundary crossed once per request or less, a small struct of function pointers in the style of `std.mem.Allocator`. Functions return concrete types.
- **Wiring is in one place.** `main` (or the export's init) builds each component once (allocator, `Io`, decoders, settings) and passes each function what it uses; no module reaches for a global.
- **One rule, one place.** Every constant, threshold, limit, format version, file name and default is declared once, in the module that owns it, and imported everywhere else. A second copy of a loop, a conversion or a parsing step is a bug: extend the owner's function instead. A rule is computed in one function that others call; a count shown to an end user and the refusal at that count come from the same function, and a test checks they agree.
- **A closed set is one named type with one list:** an `enum`, matched with exhaustive `switch`es (no `else` where every case is known), and output names come from `@tagName` or one table beside the type, never written again as strings.
- **Small functions with one job,** names that say what they hold, comments that say why rather than what, no `pub` by default, and no dead code. Zig analyses only what is referenced, so an unreferenced function's compile errors stay hidden: each file ends with `test { std.testing.refAllDecls(@This()); }`, which reaches its `pub` declarations (it is neither recursive nor sees private ones), and the root module's test references every file; a private function without a caller is dead code, which review checks.
- **Extensibility check.** One more format, output, option or target touches its one definition (a table, an `enum` and its `switch`es, a `build.zig` list) plus its tests and docs; if it needs more places, fold them into one source in the same PR.
- **Services.** A Zig service with a database or an API follows core's data, API, settings and model-call rules through its store, transport and integration modules, as core states them; the process rules (review, commits and PRs, no deferral) are core's.

## Names and files

The [Zig Style Guide](https://ziglang.org/documentation/master/#Style-Guide) applies, and review checks it:

- **Case.** Types and functions returning `type` are `TitleCase`; other functions `camelCase`; variables, fields, constants and namespace structs (no fields) `snake_case`. Acronyms are words (`PngDecoder`, `HttpClient`); established names (`ENOENT`) keep theirs. A file with top-level fields is `TitleCase.zig`; any other file and every directory is `snake_case`.
- **No redundant names.** A name reads in its namespace: `png.Decoder`, not `png.PngDecoder`. No `Data`, `Value`, `Context`, `Manager`, `State`, `utils` or `misc`, which would fit any type.
- **Doc comments.** `//!` opens each file with what it holds; `///` on each `pub` declaration says what the name does not, including who owns returned memory ("Caller owns the returned slice"). "Asserts" names an invariant checked in safe modes, "assumes" one that is not.

This reference adds:

- **No underscore prefixes.** Zig has no private fields, so a field is named for what it holds and its doc comment says how to use it; what is private is a declaration without `pub`.
- **Comments claim only what the code guarantees** (core.md); a guarantee that waits on later work is named as not yet given.
- **File order** follows core's: imports (`std` first); then, in a `TitleCase.zig` file, its fields; types; constants; `init` and `deinit`; methods, `pub` before private; plain helpers in rough call order; `test` blocks last. A test parses each file with `std.zig.Ast.parse` and fails on a declaration out of this order.

## Memory, resources and I/O

- **Allocators and `Io` are passed in.** A function that allocates takes an `std.mem.Allocator` (or holds one given at `init`); a function that touches files, the network, the clock or threads takes an `std.Io`. Library code has no global allocator or `Io`. `main` takes `std.process.Init` and passes on its `gpa` (leak-checked in Debug), its `io` and its `arena` (process-lifetime data only); it builds its own allocator or `std.Io.Threaded` only for a stated reason, and an arena per unit of work for that work's short-lived allocations.
- **Ownership is explicit.** Every allocation and handle has one owner that frees it: `defer` right after a successful acquire, `errdefer` for what a function returns on success but must release on error. A type that owns memory has `deinit`, and its doc says so.
- **Allocate once.** Buffers are sized from what is known (`initCapacity`, `ensureTotalCapacity`) and reused across iterations (`clearRetainingCapacity`); no allocation inside a loop over elements, rows or requests; no list built only to iterate it once; no `allocPrint` for a value only compared or written. Large inputs are streamed or memory-mapped rather than copied when the format allows.
- **Bounded.** Every buffer, queue, cache and per-key table has a bound named once as a constant or option, and the code says what happens at it (refuse with an error, wait, evict). Idle processes hold little memory.
- **Per-key state** uses the repository's one bounded-cache module; a test fails on a `std.AutoHashMap` or `std.HashMap` field keyed by an end user or tenant id type outside it.
- **Parse once.** A large value read more than once (a font, a table, a schema) is parsed once: at comptime when it is known at build time (`@embedFile` with a comptime parser), once at start when it is fixed for the run, or into a bounded cache keyed by version when it varies; never copied per request.
- **I/O** goes through buffered `std.Io.Writer` and `std.Io.Reader`, flushed once per output, never one write per value. Code under `std.Io` never calls blocking `std.posix` or C I/O directly.
- **Durable writes.** A file write that must survive a crash goes through `std.Io.Dir.createFileAtomic`. A store module binds every SQL value and keeps per-transaction state only.
- **Concurrency.** Parallel work goes through `std.Io` (`io.async`, `io.concurrent`, `std.Io.Group`), limited by `std.Io.Threaded`'s CPU-count default, or by a named cap passed as its `async_limit` and `concurrent_limit` when the work needs less; every started task is awaited or cancelled by its starter, so its error is seen. Shared state has one owner, or one `std.Io.Mutex` declared beside the fields it guards; never two locks taken in different orders.
- **Deadlines.** Each call a request waits on has a named `std.Io.Clock.Duration` beside it and is raced against a sleep with `std.Io.Select`, the loser cancelled; on timeout it is logged and the request goes on without the result, or returns a declared error.
- **Cancellation and shutdown.** Code that gets `error.Canceled` leaves its state consistent through `errdefer`. A long-running program handles SIGTERM (`std.posix.sigaction`) by cancelling its `std.Io.Group`, in which its background tasks run, finishing work in flight within a named deadline, flushing its writers, then exiting.

## Errors and safety

- **Explicit error sets at boundaries.** A public function returns a named error set (`DecodeError!Image`), merged with `||` where layers meet, so a caller can switch over every case; an inferred `!T` only on private helpers. Every `pub fn` counts, a program's `main` included (the CLI's, a benchmark's, a generator's): it returns a named set such as `MainError!void` or `MainError!u8`, so the program's failures are listed in one place. `anyerror` only where a callback or interface requires it. Errors travel with `try` and are handled once, where something can be done: returned or logged, never both, never swallowed (`catch {}` is a defect).
- **Errors carry context.** Zig errors carry no payload, so what a caller needs (which file, which offset, which field) goes in a diagnostics value the caller passes in, filled before the error returns.
- **One code, one error.** Zig error names are global, so an error a client is told about maps to its kind and stable code in one table beside its domain, and a comptime check over the tables fails on a code given twice.
- **Retryable from the kind.** Whether an error is retryable is one `switch` on the kind enum, and the output carries that flag.
- **API documents.** An operation's codes are built at comptime from its explicit error set (`@typeInfo(E).error_set`), so the document lists exactly the codes it can send.
- **No illegal behaviour reachable from input.** On every path a file, an argument or another program's output reaches: no `unreachable`, `catch unreachable`, `orelse unreachable` or `.?` on a value input decides; no `@intCast`, `@enumFromInt` into an exhaustive enum, `@intFromFloat`, division, shift or index whose operand input can push out of range; no `+ - *` that input can overflow. Check and return an error, or use `std.math.cast` (it returns null), saturating (`+|`) or wrapping (`+%`) arithmetic where that is the meaning. These are allowed in tests, and where an invariant rules the failure out, with a comment stating it.
- **No silent data loss.** `@truncate` and `@floatCast` are defined but drop information without a trace; each is used only where the dropped part is known not to matter, and a comment says why.
- **Safety opt-outs.** `@setRuntimeSafety(false)`, `@ptrCast`, `@alignCast`, `@setFloatMode(.optimized)` and inline assembly appear only where a measured kernel needs them, in the smallest scope, with a comment naming each invariant relied on and a test that exercises it.
- **C interop.** Every C call's result is checked; C memory is freed by the C library's own free; the library's error codes become the Zig error set in its binding, whose header is translated by `b.addTranslateC` in `build.zig`, and which is the only file that imports the translated module or declares the library's `extern` functions (core's one integration module per external service).
- **Trust boundaries are explicit.** Identity and tenant reach a domain function as typed values transport took from authentication, never from a request body or an argument. Sizes, counts and offsets read from a file or a request are validated against their limits before they size an allocation or a loop.
- **Settings.** Where a program has deployment settings, `main` reads each once from `std.process.Init`'s `environ_map` into a typed config with no defaults, and exits naming the first missing one; a test keeps the config's fields equal to `.env.example`.
- **Stubs** are a module `build.zig` adds only to test and bench compiles; no production code branches on `builtin.is_test` or `builtin.mode` to change behaviour.
- **Logs** go through `std.log` with a scope per module, and carry ids, counts and durations only, never the content being processed.
- **Secrets.** A type holding one has a `format` method that writes a redaction, and is printed only with `{f}`, never `{any}`.

## Types

- **Types over integers and strings.** Ids, units and amounts are distinct types: `enum(u32) { _ }` for an id, a struct for a size, a colour or a rectangle; never a bare `u32` or `[]const u8` passed between modules. Input is parsed into these at the boundary, so inner code sees only valid values.
- **Typed ids are checked:** a test over `std.zig.Ast` fails on a `pub` field or parameter named for an id whose type is a bare integer or `[]const u8`.
- **Typed external data.** JSON is written from structs with `std.json.Stringify.value` and read into structs with `std.json.parseFromSlice` and `.ignore_unknown_fields = true` (the default rejects unknown fields); JSON is never assembled with `print` or string concatenation. Walking a `std.json.Value` by hand is a defect; a value a protocol requires echoed unchanged stays a `std.json.Value` or a raw slice, with a comment saying why. Other formats (CLI arguments, a binary header) are parsed into structs and enums the same way.
- **comptime** removes repetition and fixes sizes known at build time; it is not a second type system. Generic code (`anytype`, a function returning a type) states in its doc comment the interface it needs, and fails with a clear `@compileError` when a type lacks it.

## Performance mechanisms

- **SIMD** through `@Vector` and `std.simd`, with a scalar path for the tail; a kernel's speed-up over the scalar version is measured on the product's inputs and recorded in the PR.
- **Static dispatch** through comptime parameters; a struct of function pointers (`std.mem.Allocator`, `std.Io.Writer`) only at a boundary crossed once per request or less.
- **Measure before and after** on the entry point the product calls, in the release mode it ships: time, peak memory and binary size, with the method and both numbers in the PR. `zig build bench` runs the benchmark the claim rests on.

## Tests

- **Tests meet the same bar as code.** `test` blocks sit at the end of the file they test; tests through a package's public API go in `tests/`; both run from `zig build test`. Tests build inputs from typed values, name values from their owners, and decode JSON output into the program's own types, never compare it as text. Repeated cases are one table looped with `for`.
- **Memory is checked.** Tests use `std.testing.allocator`, so a leak fails the run, and `std.testing.checkAllAllocationFailures` for code whose allocation-failure path matters.
- **Deterministic and independent:** tests use `std.testing.io`, no sleeps, wall clock or network, and run against stubs, never a real paid service; anything random seeds from `std.testing.random_seed`, which the run prints and `zig build test --seed <n>` replays; fixtures are small committed files. A fix comes with a test that fails without it.
- **Data races.** Tests of concurrent code also run in a build step, called by the gates, whose test module sets `.sanitize_thread = true`.
- **Parsers and decoders are fuzzed** with `std.testing.fuzz`, run by `zig build test --fuzz`, and a crash it finds becomes a fixture.
