# Web clients

How a React, Vite and TypeScript web client, user client or operator console alike, meets `core.md`; read it first. A web client meets the same bar as a server.

## Stack and dependencies

- **TypeScript in strict mode,** Bun for installs and scripts, Biome for formatting and lint, Vitest for tests. Biome warnings fail the check; after a Biome upgrade, `biome migrate` clears deprecated keys.
- **Do not reinvent.** Reach for the platform (`Intl`, CSS, native elements, `fetch`, `URL`, `AbortController`; CSS over JavaScript, native elements over widgets), then the app's stack (its component and query libraries), then a maintained, widely used library pinned exactly, before writing your own.
- **Dependencies:** none without an import; the check fails on an unused one (knip).

## Modules

- **Modules by concern.** `src/` holds `api/` (the generated client, any stream, and hooks with their own optimistic, conditional invalidation or cache logic), `i18n/` (messages, formatters, errors as text), `ui/` (shared components and tokens), one folder per screen under `screens/`, and any other folder the app's `AGENTS.md` names. That `AGENTS.md` lists every `src/` folder in backticks with a trailing slash; a test fails on a folder it does not name or a named one that is gone, so a new folder is described there first.
- **Module direction.** `api/`, `i18n/` and `ui/` never import from `screens/`, and a lint override fails one that does; a screen never imports another screen's internals.
- **Screens reach the server only through generated hooks or `api/`'s own.** A screen imports its operation's generated query and mutation hooks directly; `api/` adds a hook only for logic of its own (an optimistic update, a conditional invalidation, a stream, a cache replacement), never one that only forwards. Per-operation query defaults (`staleTime`, polling, retry) are small exported `query.queryOptions` mutators named under `override.operations.<operationId>` in `tooling/orval.config.ts`, because Orval deprecates `query.options`; no operation-name lookup, central defaults table or wrapper hook. A lint override fails a screen that imports the transport, `QueryClient` or `useQueryClient`, or calls `fetch`.
- **Size.** A component file over about 250 lines, or with more than one data hook, is split by concern and its rules move to plain functions with tests; a test fails on a `.tsx` file over 250 lines.

## Logic and state

- **Logic apart from views.** Components render and handle events; rules, parsing and shaping are plain functions beside them or in `api/`, tested without rendering. Date and calendar maths are plain functions in `i18n/` with tests.
- **No server rule is copied into the client.** The client reads the server's answer instead of computing it or ordering several answers; what the account must do first is the server's answer, mapped to a screen by one `switch`.
- **Server state only through the query library** (TanStack Query), never copied into `useState`; local UI state lives in the component that owns it.
- **Query keys and invalidation.** Keys come only from the generated client's query-key functions or an `api/` helper built on them, never string tuples. The one invalidation prefix is the generated key function without path parameters (Orval's `useOperationIdAsQueryKey`), checked against the generated keys by a test. Plain mutation invalidation is grouped in Orval's config (`mutationInvalidates`); optimistic and conditional invalidation stays in `api/queries.ts` with its reason; cache reads and writes use the generated `useSetQueryData`/`useGetQueryData`. Screens never take a `QueryClient`, and work ahead of the screens (a preload) is an `api/` function too.
- **An event stream** is parsed with a maintained parser (`eventsource-parser`) that honours `id:` for resuming, with URLs and headers from the generated client's base path and auth.
- **Storage.** `localStorage` and `sessionStorage` are reached only through one module that tolerates their absence; every key is a constant there, and a value read back is checked with a guard, never cast.
- **One source for environment-derived values:** the API base path, the user's time zone and the locale each come from one helper.
- **No hard-coded values owned elsewhere.** Limits, enums and locale lists come from the generated API types or one module, never literals in a form; the API base path, the server's port and storage keys are each written once; a fact the server owns (a document's version) is read from it or generated, never copied.

## Types

- **Typed end to end** from the server's OpenAPI document: no `any`, no untyped JSON, no hand-written request or response types. Errors are the server's problems, read by `code` typed as `Problem["code"]`, never bare strings.
- **Guards, not casts.** No `as` cast except on generated types; a value from storage, the URL or a stream is checked with a type predicate, never cast after a partial check, and each guard has a test with malformed input. A schema generated from the API document (zod) is a guard: what it parses needs no hand-written predicate. A stream event's required fields are listed once, typed from the document.
- **Exhaustive switches.** A `switch` over a kind the document enumerates handles every kind and ends with `default: x satisfies never`, so a kind the server adds fails the type check; Biome's `useExhaustiveSwitchCases` does not see generated unions (checked on 2.6.4), so TypeScript does it.
- **Every route tree has an error element:** the root route has an `errorElement` (as does a layout whose children can fail apart from it) that shows the problem in the user's locale with a way to retry or go home, never the router's default page or an error's own text; a test reads the routes and fails on a root without one.
- **One route over a list.** A family of pages is one route whose segment takes values from one list; a name the list lacks is a 404.
- **Every promise** is awaited, returned, or `void`ed with a `.catch`.
- **Callbacks are passed as themselves:** `retry={query.refetch}`, never `retry={() => query.refetch()}`, when the callee calls it with arguments it accepts. An arrow stays only where it drops or changes arguments (an `on*` prop passes its event, which `refetch(options)` would read as options), needs its receiver (`window.location.reload`), or reads a value late (`ref.current.focus()`). A lint plugin, named in the repository's `AGENTS.md`, refuses the wrappers it can prove removable.

## Text and locales

- **Text** is only the i18n library's messages, through `i18n/`; dates, numbers and language names come from `Intl`.
- **Locales** are the i18n library's list, never written out; a value per locale is a `Record<Locale, …>` checked with `satisfies`, never a `locale === "…"` branch, a per-language name or a third-party locale switch that fails silently for a new locale.

## Interface

- **One component per pattern, composed, never rebuilt.** A pattern on more than one screen in the same role is one `ui/` component that screens compose, never rebuild from classes or restyle; a second screen's variant is a prop. The design document names each component with its file, and a new pattern adds its component and line there in the same PR. A shared piece moves to `ui/` the second time it appears.
- **One implementation per concern:** one formatter per kind of value, besides the hook and component rules above.
- **Tokens.** Colours and other design tokens are CSS variables from the design document, never literals in components; a token's value lives in one CSS file. Tailwind classes, no inline styles except computed values.
- **Accessibility:** native elements first; every control has an accessible name; focus is visible; motion respects `prefers-reduced-motion`, Tailwind's `animate-*` utilities included.

## Security and performance

- **Content Security Policy.** The build writes the page's CSP into `index.html`: its own files, each inline script by hash, each third-party origin by name; no `'unsafe-inline'` script. A new third-party origin is added there with a test; headers a meta tag cannot carry are the edge's.
- **Budget.** The shipped JavaScript has a budget the check enforces; screens are split with `lazy`, and heavy libraries load on first use. A speed or size claim measures the bundle and first render before and after.

## Operator consoles

- **An operator console is its own app,** not screens of a user client: its types come from the operator API's own document, and it shares with a user client only what both draw and validate with (design tokens, a project of primitives in them, and the helpers that turn generated schemas' issues and server refusals into each app's messages, named in the repository's `AGENTS.md`). A user client never reaches the operator API. The console's own components (an admin framework's layout, tables and views) stay in it.
- **A console on an admin framework's data hooks** gives the framework one data provider that calls only the operator API's generated client, typed route by route; the framework keys and invalidates queries by resource, the console's exception to the key rule above.
