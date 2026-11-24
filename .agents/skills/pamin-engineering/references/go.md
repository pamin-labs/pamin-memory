# Go

Read this when writing or reviewing Go. It says how Go code meets `core.md`, `data.md` and `performance.md`; read those first.

## Toolchain and build

- **Version and tools.** The Go version is pinned in `go.mod`; upgrading Go or an SDK is its own change. Tools (staticcheck, sqlc, goose and the like) are pinned as `go tool`s with `go get -tool`.
- **Clean and modern.** Code is `gofmt`, `go vet`, staticcheck and `modernize` clean, in the idioms of the `go.mod` version (`errors.AsType`, `any`, `min`/`max`, range over integers and iterators such as `strings.FieldsSeq`, `new(x)` for a pointer to a value); `go tool modernize -fix` applies them.
- **Linters beyond vet and staticcheck:** wrapcheck, so an error from another package or the store is wrapped with context where it enters; sloglint, so logs use the context variants (`slog.InfoContext`) and name an error `err`.
- **The standard library first:** `slices`, `maps`, `cmp`, `errors`, `context`, `log/slog`, `net/http` and its pattern routing, `sync`, and `errgroup` from `golang.org/x/sync`.
- **Builds** are static: `CGO_ENABLED=0`, `-trimpath -ldflags="-s -w"`.

## Packages and files

- **Package layout.** Code lives under `internal/<layer>/<package>`, one package per directory. A layer that is a single package (a store, an API) may sit directly under `internal/` as `internal/<layer>`; every other package sits in its layer's directory. A test reads each package's layer from its import path and fails on a package in no layer or an import that points back (core's layer test).
- **Packages** follow core's module rules: named for what they hold (`account`, never `utils`), one per concern, unexported by default, interfaces declared by the consumer. A name reads with its package: `account.Service`, never `account.AccountService`.
- **File order** is core's in Go terms: imports; types, each enum's constants right after its type; other constants and variables, sentinel errors included; constructors; methods grouped by receiver, exported first; plain helpers in call order. Functions are grouped as the Uber Go Style Guide's "Function Grouping and Ordering" says: by receiver, in rough call order.
- **Shared types.** Types several files of a package share go in its `types.go` (an API package's request and response types, a domain's errors and value types); a type with methods stays with them.

## Errors, context and concurrency

- **Errors** wrap with `%w` and context (`fmt.Errorf("load the profile: %w", err)`), are checked with `errors.Is` and `errors.AsType`, and are lowercase without trailing punctuation.
- **Refusals are listed.** A domain lists each method's declared refusals beside its types, and the operation that calls the method sends that list as its errors, so the API document's codes come from it.
- **No panic reachable from input.** Input errors are returned; a `Must…` function (`regexp.MustCompile`, `template.Must`) runs only at start-up or in a package-level `var` or `init`.
- **Context** is the first parameter of anything that does I/O and is never stored in a struct. Every goroutine ends with its context, and its starter waits for it (`errgroup`, `sync.WaitGroup.Go`).
- **Deadlines.** Each call a request waits on runs under `context.WithTimeoutCause`, with a duration named beside the call and a cause naming it. Work that must finish after the request ends (an audit write, a cleanup) runs on `context.WithoutCancel(ctx)` with its own timeout.
- **Shutdown.** `signal.NotifyContext` cancels the root context on SIGTERM; `http.Server.Shutdown` drains requests before a named deadline, then the wiring awaits its background tasks.
- **No data races:** shared state is owned by one goroutine or guarded by one mutex; `go test -race` runs in the gates.
- **Caches and per-end-user state** use `github.com/hashicorp/golang-lru/v2`, safe for concurrent use; a test fails on a struct field that is a map keyed by an end user's id.

## Types

- **Types over strings and maps:** typed ids and enums, structs with explicit JSON tags, `time.Time` and `time.Duration` for times, useful zero values.
- **Typed ids are checked:** a test reports a struct field, parameter or result named for an id whose type is a bare `string` or `int64`.
- **A closed set** is a named type with its constants and one list (`Status` and `Statuses`); an API's enum is built from the list by one helper, never written in a struct tag.
- **Typed JSON.** `encoding/json` decodes into structs, which ignore unknown fields; walking a `map[string]any` is a defect. `json.RawMessage` holds only a value a protocol requires echoed unchanged, with a comment saying so.
- **A stream's event** is named by a constant beside its payload, and a payload field that is a closed set is converted to the API's own type in the stream, so its enum reaches the API document.

## Logs, stubs and tests

- **Logs** go through `log/slog` with stable keys.
- **Stubs and test support** are packages of their own, and a test fails when non-test code imports one.
- **Tests** are table-driven where cases repeat, and run against the local database and the stubs, never a real paid API. Each top-level test calls `t.Parallel()` or says in a comment why it cannot. They use `t.Context()` and `t.Cleanup`; a cleanup that needs the test's context takes `context.WithoutCancel(t.Context())`, since that context is cancelled before cleanups run.
- **Fuzzing.** Each parser of outside input has a `FuzzX` target with seed cases (`f.Add`); a crasher `go test -fuzz` finds is committed under `testdata/fuzz/` and stays a regression case.

## Performance mechanisms

- **Allocate once:** slices, maps and `strings.Builder`s are pre-sized from a known size (`make([]T, 0, n)`, `b.Grow(n)`).
- **Static dispatch:** concrete types in hot loops; an interface only at a boundary crossed once per request or less.
- **Bounded concurrency:** a channel is buffered to a size named once; a fan-out is capped with `errgroup.SetLimit`.
- **Round trips:** keyset pagination (`id < $1 order by id desc limit n`) with indexes matching each query; independent reads batched (`pgx.Batch`) or folded into one statement.
- **Paired keyset lists** in sqlc write their source, joins, filters and projection once, in `with filtered as not materialized (...)`; the forward and backward halves read from it and keep only the direction guard, cursor comparison, order and limit. PostgreSQL inlines a `not materialized` CTE into each half, so both still seek the index, and the plan test keeps checking that they do.
- **Memory:** `GOGC` and `GOMEMLIMIT` are set for the instance size.
- **Connections:** one pool per process, sized explicitly.
