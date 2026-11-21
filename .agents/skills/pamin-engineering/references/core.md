# Core engineering rules

Every repository that imports this file holds its code to these rules, whatever the language. The bar: high performance, low resource use, fast builds, and clean, DRY, decoupled code that is easy to maintain and extend. Each repository's `AGENTS.md` names its layers, how it meets these rules and the tests that check them; a language's reference adds only that language's mechanisms. Its source is the public `pamin-labs/skills` repository; repositories install it with the skills CLI and change it only there.

## Before writing anything

- **Does it need to exist?** Build what a real caller needs now. No speculative options, layers, traits, interfaces or config; add them when a second real use appears.
- **One implementation per concern.** Search the repository and its shared libraries for the concern first, and extend its owner through its interface; never add a parallel version with its own rules.
- **Do not reinvent what exists.** Reach, in order, for the repository's own implementation, the service's official SDK, the standard library, the platform, a dependency already in the tree, then a maintained, widely used one; write your own only when none meets the need. The PR's "Reused" section says what was searched, reused and hand-written, and why; hand-written code one of those covers is a defect review sends back.
- **Dependency versions.** Lock files pin exact versions. A published library requires caret minimums so consumers can unify, pinning exactly only with a stated reason.
- **Before adding a dependency,** check its maintenance, licence, and added build time and bundle or binary size, and record them in the PR.

## Shape of the code

- **One rule, one place.** A change to a rule touches one function or module; if you must edit the same logic in two places, the logic is in the wrong place.
- **Business logic lives in one layer per area,** apart from transport, views, wiring and storage; each area's `AGENTS.md` says where.
- **A directory says what its code is.** New code goes into the directory its area's `AGENTS.md` names for its layer or concern, never beside them at the top, and that file names each of its parts by path. A new directory at that level updates that `AGENTS.md`, and any layout test's list, in the same PR; where no test reads the list yet, review checks it.
- **Small functions with one job,** names that say what they hold, comments that say why, not what, and no dead code.
- **Comments claim only what the code guarantees.** A guarantee that waits on later work is named as not yet given.
- **Errors carry context** and are handled once, where something can be done: returned or logged, never both, never swallowed.
- **Types over strings and integers.** Ids, units, amounts and times are named or standard types, never bare strings or numbers; input is parsed into them at the boundary, so inner code only sees valid values.
- **Trust boundaries are explicit.** Identity, tenant, end user and project come from authentication, never from a request body, a tool argument or a model.
- **Model external data with types.** External JSON (requests, MCP results, other services' answers, webhooks, files another program writes) is parsed into typed structs and enums that ignore unknown fields, and requests are serialized from typed structs; no hand-built JSON, untyped lookup chains or substring searches. A value stays opaque only when a protocol requires it echoed back unchanged, and the code says so.
- **No hard-coded value in more than one place.** Every port, address, URL, id, version, limit, price, locale list, file name and default is defined once, where it is owned (a module's constant, a Makefile variable, a Dockerfile `ARG`, a Wrangler config, `.env.example`); everything else reads it or is checked against it by a test or a generator. A literal that names a value owned elsewhere is a bug, in code, config, scripts, tests and docs alike. A value two projects use has one source, and a test or a generated file keeps the other in step.
- **Deployments have no defaults.** A setting a deployed binary or service must be given has no fallback in code: a missing or empty one stops the program at start, naming itself. Only local use (a cache directory, a socket, a Makefile, Vite, a benchmark) names local values, each once.
- **Shared state has one owner,** a type or a task, keeping each lock next to the fields it guards.
- **Built files are never committed.** Build output, benchmark and evaluation results, model caches, indexes and generated caches are git-ignored; only generated files a gate checks (snapshots, generated queries, API documents) are committed.
- **Extensibility check before every PR.** One more of something the code already has (an operation, setting, route, module, model, locale, transport) touches its one definition plus its tests, messages and docs; if it needed more places, fold them into one source in the same PR.
- **Tests meet the same bar as code.** A test builds requests from typed values, decodes answers into the API's own types (never a store row), compares JSON by decoding it, never as strings, and names a value owned elsewhere from its owner.
- **No deferral inside a PR's own code.** A written rule broken by code the PR adds or touches is fixed in that PR, in its own commit; a follow-up is only work outside the PR's concern.

## Layers, domains and decoupling

- **Layers point one way:** entry, wiring, transport, domain, then store and integrations, then values. Dependencies point only rightward, and a test fails on one that points back (a Rust workspace: over `cargo metadata`; a single crate: over its `use` paths; Go: the import test). A new module or crate goes in its layer's directory, never beside the layers.
- **Transport only translates.** A handler, middleware or CLI command parses the request into types, calls one domain function and shapes its result as the API's types. Any business rule there (a refusal, quota, limit, metering decision or check of any kind) is a bug; a middleware may read the request but calls the function that decides, which declares the error.
- **Transport never reaches the store:** no call, type or error from it. A domain function that only passes a call through is still worth having, so a later rule has one place to go.
- **Each layer converts at its edge.** Row types never leave the store: store read types become domain types, then wire types, so a schema change fails to compile at a conversion instead of silently changing the API. A domain's public functions and fields use its own types; the only store type it shows is its store handle; a type several domains and the store share is a value.
- **A domain owns its data.** It calls another domain's service, never its queries or tables; the caller's domain, not the store, orchestrates cross-domain work. Each store function is called from one domain only, and a test fails otherwise.
- **A domain module holds its service, its types, its errors and its rules.** It depends only on the store, on integrations through traits or interfaces it declares, and on other domains' services; never on an HTTP framework, MCP transport code or the API layer.
- **Wiring is in one place.** The entry point or app module builds each service once and hands each transport the services it uses. A background task is domain code on a schedule, under the same rules, owned by the wiring that awaits it at shutdown.
- **Each handler group sees only its own services.** Each business area registers its operations in its own file and receives only its service and the caller's identity; an API is never one handler holding every dependency.
- **Traits and interfaces are declared by their consumer,** with only the methods it calls. Functions return concrete types.
- **One definition per concern.** Each rule, constant, limit, price, page size, setting name, header, path or schema is defined once, in its owning module, and imported elsewhere. Before adding a helper, search for one. A second copy of a loop over an SDK is a bug: put one helper per repeated SDK flow in its integration module.
- **A rule is computed in one function,** which other domains call rather than restate. What a client must do first is that function's answer, not flags the client orders.
- **A counter and its refusal come from one function.** The "left" figure a caller is told comes from the function that refuses, and a test checks they agree.
- **A closed set of values is one named type with one list.** Wire enums and schemas are built from that list, never written again in a tag or string, and a test checks it against the database's check constraint. A store value is never a wire name.
- **Modules are named for what they hold;** no `utils`, `common`, `models` or `helpers`. Each covers one concern, with a small public surface, private by default. A file covers one business area, not one type, in this order: imports; types; constants and statics; constructors; methods grouped by type, public first; plain helpers, in rough call order. A type is declared before the code that uses it. An enum's constants follow its type at once.
- **Writes are safe to repeat.** A write a client or retry may send twice is idempotent: a repeat returns what the first stored and changes nothing more. A call it makes to another service is outside the transaction and safe to retry, with that service's "not found" counting as done. Steps are ordered so a failure part way leaves no gap.
- **Per-transaction state only.** A deployed PostgreSQL may sit behind a transaction pooler, so a connection belongs to one transaction. Use `set local` or `set_config(name, value, true)`, and protocol-level prepared statements. Never use a session `set` or `set_config(..., false)` meant to outlast its transaction, session advisory locks (use `pg_advisory_xact_lock`), `LISTEN`/`NOTIFY`, temporary tables, SQL `PREPARE`, or cursors held across transactions. Test fixtures may.
- **State kept per user, tenant or key beyond a request is bounded:** a structure with eviction, never a map pruned by hand. Per-user work that must be serialised is serialised in the database (a lease or row lock), not in one instance's memory, so instances stay stateless.
- **Hot paths keep their database round trips flat.** A request's round trips do not grow with a user's data: no per-request `count(*)`, `offset` or `max()` over a growing set. Keep counters, use keyset pagination with matching indexes, and batch independent reads.
- **Parse once.** Large values (manifests, schemas, prompts, models) are parsed once and cached by version in a bounded cache, never copied per request. Answers are streamed rather than buffered where they can be.

## Errors, data, settings and calls to other services

### Errors and API documents

- **Values are leaves.** A type two layers share is declared once, in a value module that imports no service. The store reads and writes these types and never imports a domain, a model harness or transport.
- **One code, one error.** Every error a client is told about is declared once, in the domain that owns it, with a kind (invalid, forbidden, conflict, too many, unavailable, ...) and a stable code; a code declared twice fails at start or in a test. A limit says when it lifts.
  - A business error belongs to the domain whose rule raises it; the store keeps only data errors (not found, busy), and the domain maps a store error that transport must act on to its own error.
  - Transport answers any declared error by its kind, with no table of every domain's errors, and never names the store's errors.
  - A layer with nothing more particular to say reuses the error of its kind and adds context, never declaring the same code again.
- **What a client decides from an error is the server's to say.** An error carries whether it is retryable, from its kind; a client reads that, never its own list of codes.
- **An API document lists the codes its operations send, and no other.** Each operation, and each middleware where installed, declares its errors, and the document's code enum is built from them, so a client never maps a code it cannot receive. A document builds without warnings. A file committed from a generator (an API document, a catalogue, a list another project reads) is checked current by a test or CI step.
- **APIs are isolated.** Public, operator and internal APIs each have their own module, port or listener, and document; none imports another's modules, and the operator API is reachable only from its own host.

### Data and transactions

- **Rules inside a transaction are the domain's.** A rule judged atomically with a write gets its facts from the store, inside the transaction, through a function the domain passes in; the store reports counts and flags and never decides. A query takes the domain's numbers as parameters (a window's start, a limit, a reason), never naming a period, age or reason itself.
- **Calls to other services stay out of transactions.** Commit, then call, idempotently so a failed call can be retried. What must still happen elsewhere is recorded in the change's transaction, and a worker retries it until done, with the other service's "not found" counting as done.
- **A write that charges, and that a client may send twice, takes a client id,** generated by the client, the same on every attempt and unique per caller. A repeat with the same id and request returns what was stored and charges nothing; the same id with a different request is refused.
- **Schema changes expand, then contract.** A migration adds what new code needs and removes only what no deployed version reads, so the previous release runs on the new schema and can be rolled back to. A service starts on a schema at or above its own version.
- **Queries and migrations.** One query per concern, reused, not copied. No SQL is assembled in code except test fixtures. A column that holds one of a type's values decodes straight into that type, never through a hand-written conversion. Migrations are hand-written, numbered and append-only (a merged one is never edited, renamed or removed; a change is a new one), and checked for locking and destructive changes before they merge.

### Settings and switches

- **Settings have one source each.** A deployment setting is read in one place at start into a typed config. `.env.example`, or the service's own list of required names, names it once, and a test keeps the two in step. It has no default in code.
- **Product settings live in the database.** Models, prices, plans, limits, retention and the like are named with their defaults in one settings module, stored only where they differ, changed through the operator API with an audited reason, and read live, never copied at start.
- **No development-only behaviour in production code.** Local runs use a stub of each service ("Stubs exist only for tests and local runs"), never a back door.
- **A kill switch is a product setting, judged in one domain function** that every entry point of its kind already calls, never in a handler. Operators are warned while it is on, and its refusal is a declared, non-retryable error.
- **Operator writes are audited and safe to repeat.** Every operator change carries a reason, written in one transaction with its audit record.

### Calls to other services and models

- **Every call a request waits on has its own deadline and fallback.** The deadline is named once beside the call (a constant, or a setting when operators tune it) and set on the call's context or future, never left to an HTTP client's default timeouts. When it passes, it is logged and the request goes on without the result or fails with a declared error.
- **One integration module per external service.** It wraps the official SDK, or a small typed client where there is none, and holds the service's URLs and ids as constants. A domain declares the trait or interface it calls and the wiring hands it the implementation; only the integration module imports the SDK.
- **Model and inference calls have one owner.** Every model call goes through one layer (a harness, inference module or pool) that names each call's task, budget and settings in one list and prices or meters its usage. A new model call gets a row in the project's list of model calls, with measured cost and latency, and a roadmap entry.
- **Cost is metered and watched.** Every metered unit is recorded once, by the layer that owns billing: for a module a hosting platform serves, that is the platform, never the module's own code. Spend no single user causes (background work, batch jobs, re-checks) is metered too, attributed to no user, and a total past an operator-set threshold raises an alert.
- **Stubs exist only for tests and local runs:** reachable only from tests, the benchmark and the local runner, and a test fails when production code imports one.
- **Logs carry ids, counts and durations only,** never message text, memory content, tokens, secrets, request bodies or other personal data. Each request or call writes one line, joined to its trace.

## Resources, build speed and measurement

- **Resources.** Idle processes hold little memory, caches are bounded and evict, and work that can wait runs in the background with a budget.
- **Build speed.** Shallow dependency trees, one well-chosen dependency over several small ones, unused default features off. Split code so an edit rebuilds little, with heavy dependencies (inference, TLS) behind the modules or features that need them.
- **Footprint.** Release builds are as small as the language allows without losing safety checks the product relies on.
- **Measure before and after** any change claimed faster, smaller or cheaper, on the entry point the product calls, with both numbers and the method in the PR. A claim without a measurement is removed.

## Working and review

- Everything committed is English: code, comments, logs, commits, PR titles and bodies, documents.
- Never write tokens, keys or credentials to files, commits or logs.
- **Nothing lives only in a conversation.** Work found and not done (review follow-ups, audit findings, a flaky test) goes into the project's roadmap in the same session; an owner's decision goes into a decision record before or with the change.
- **Rule scans** of a whole repository follow `rule-scan.md`.
- **Before a PR opens,** rebase on the latest `main`; if `main` gained something overlapping, build on what landed and say what was reused and removed. Run the gates of every `AGENTS.md` the change touches, locally.
- **Review before every PR.** Check the whole diff, tests included, against every rule of each `AGENTS.md` it touches and the references they import, fixing misses in new commits; then run the peer-review skill and put its verdict and findings under the PR's verification. The agent opening the PR reviews delegated work the same way, itself, never on the delegate's report alone.
- **A PR merges only when its final commit is proven:** GitHub reports it clean, every check has passed (a failed or pending check blocks it, however small the change), its tests pass, and, for a deployed or shipped service or binary, an end-to-end check (a local acceptance run or staging) has passed on that same final commit, never a run from before the last review fix.

## Commits and PRs

The coordination tool's commit skill holds the details; its hooks and CI check them.

- Before editing, declare an intent with the coordination tool and work on its branch `<type>/i-<number>-<slug>` (no person or tool names); the PR description says `Intent: I-<number>`, and CI rejects a PR without a live intent for its branch. Check the briefing's intents of products this one uses before changing code that depends on them.
- Conventional Commits with a required scope (`fix(engine): ...`) for commits and PR titles; first line and title at most 72 characters, scope included. Count before committing: a pushed commit is not rewritten without the owner.
- One logical change per commit, each building and passing alone; refactors, behaviour changes and formatting are separate commits.
- A PR covers one concern, reviewable in one sitting; large work is a stack of PRs, each mergeable alone. Its description has a summary, the changes, what it reused, how it was verified (with numbers for performance), and any risk or follow-up.
- No agent attribution: no `Co-Authored-By` trailers, session URLs or ids, or "Generated with" footers. Some tools append one when they create or edit a PR, so read the description back after each such call and remove it.
- The commit author is the engineer's own git identity, never invented; if missing, stop and ask, then save it as `GIT_AUTHOR_NAME` and `GIT_AUTHOR_EMAIL` in the environment settings.
- Never squash-merge; keep every commit.
