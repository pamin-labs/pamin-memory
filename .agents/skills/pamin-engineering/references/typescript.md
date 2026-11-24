# TypeScript

Read this when writing or reviewing TypeScript anywhere: apps, Workers, tooling and shared libraries. It says how TypeScript code meets `core.md`; read that first. `web.md` and `workers.md` add each platform's rules.

## Toolchain and dependencies

- **TypeScript in strict mode,** Bun for installs and scripts, Biome for formatting and lint, Vitest for tests. Biome warnings fail the check; after a Biome upgrade, `biome migrate` clears deprecated keys.
- **Type-checks** run TypeScript's native `tsc`, with no `baseUrl` in any `tsconfig`.
- **Dependencies** are pinned to exact versions, and none is kept without an import; knip fails the check on an unused one.

## Types

- **Typed end to end:** no `any`, no untyped JSON, and no hand-written type for what a generated API document or binding types already describe.
- **Guards, not casts.** No `as` cast except on generated types. A value from storage, the URL, a stream, settings, another service or a request is checked with a type predicate, never cast after a partial check, and each guard has a test with malformed input. A schema generated from an API document (zod) is a guard: what it parses needs no hand-written predicate.
- **Exhaustive switches.** A `switch` over a closed set handles every case and ends with `default: x satisfies never`, so a case added upstream fails the type check. Biome's exhaustive-switch rule does not see generated unions, so TypeScript does it.

## Code

- **Every promise** is awaited, returned, or `void`ed with a `.catch`.
- **Callbacks are passed as themselves:** `retry={query.refetch}`, never `retry={() => query.refetch()}`, when the callee calls it with arguments it accepts. An arrow stays only where it drops or changes arguments (an `on*` prop passes its event, which `refetch(options)` would read as options), needs its receiver (`window.location.reload`), or reads a value late (`ref.current.focus()`). A lint plugin, named in the repository's `AGENTS.md`, refuses the wrappers it can prove removable.
