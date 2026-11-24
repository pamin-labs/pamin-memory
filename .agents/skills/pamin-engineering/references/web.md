# Web clients

Read this when writing or reviewing a React, Vite and TypeScript web client, an end-user client or an operator console alike. It holds the architecture (modules, state, queries, types, tests, security); how the interface looks and behaves is `web-interface.md`, read with it for any change to what a screen shows. Read `core.md` and `typescript.md` first. A web client meets the same bar as a server.

## Stack

- **Do not reinvent.** Reach for the platform (`Intl`, CSS, native elements, `fetch`, `URL`, `AbortController`; CSS over JavaScript, native elements over widgets), then the app's stack (its component and query libraries), then a maintained, widely used library pinned exactly, before writing your own.
- **The React Compiler memoizes.** It is on in the build and in the tests, and any bailout fails the gate, so code keeps to the rules of React. A `"use no memo"` with its reason (a documented incompatible library) is the only opt-out. No hand-written `useMemo`, `useCallback` or `memo`: the compiler does it, and a hand copy only drifts.

## Modules

- **Modules by concern.** `src/` holds `api/` (the generated client, any stream, and hooks with their own optimistic, conditional invalidation or cache logic), `i18n/` (messages, formatters, errors as text), `ui/` (shared components and tokens), one folder per screen under `screens/`, and any other folder the app's `AGENTS.md` names. That `AGENTS.md` lists every `src/` folder in backticks with a trailing slash; a test fails on a folder it does not name or a named one that is gone, so a new folder is described there first.
- **The top of `src/`** holds only the entry, the router, the routes, global CSS and app-wide tests.
- **`ui/` holds only role folders** named in the app's `AGENTS.md`, checked by the same layout test.
- **Every address is in one `routes.ts`,** and the router is built from it.
- **Module direction.** `api/`, `i18n/` and `ui/` never import from `screens/`, and a check fails one that does; a screen reaches another only through a lazy import of its entry.
- **Screens reach the server only through generated hooks or `api/`'s own.** A screen imports its operation's generated query and mutation hooks directly; `api/` adds a hook only for logic of its own (an optimistic update, a conditional invalidation, a stream, a cache replacement), never one that only forwards. Per-operation query defaults (`staleTime`, polling, retry) are small exported `query.queryOptions` mutators named under `override.operations.<operationId>` in the Orval config, because Orval deprecates `query.options`; no operation-name lookup, central defaults table or wrapper hook. A lint override fails a screen that imports the transport, `QueryClient` or `useQueryClient`, or calls `fetch`.
- **Size.** A component file is split by concern when it holds a second one, never to meet a line count; a component file uses at most one data hook, its rules moving to plain functions with tests, and a test enforces the hook limit.

## Logic and state

- **Logic apart from views.** Components render and handle events; rules, parsing and shaping are plain functions beside them or in `api/`, tested without rendering. Date and calendar maths are plain functions in `i18n/` with tests.
- **No server rule is copied into the client.** The client reads the server's answer instead of computing it or ordering several answers; what the account must do first is the server's answer, mapped to a screen by one `switch`.
- **Server state only through the query library** (TanStack Query), never copied into `useState`; local UI state lives in the component that owns it.
- **Query keys and invalidation.** Keys come only from the generated client's query-key functions or an `api/` helper built on them, never string tuples. The one invalidation prefix is the generated key function without path parameters (Orval's `useOperationIdAsQueryKey`), checked against the generated keys by a test. Plain mutation invalidation is grouped in Orval's config (`mutationInvalidates`); optimistic and conditional invalidation stays in one `api/` module, with its reason; cache reads and writes use the generated `useSetQueryData`/`useGetQueryData`. Screens never take a `QueryClient`, and work ahead of the screens (a preload) is an `api/` function too.
- **Deadlines.** Every request carries `AbortSignal.timeout` with a duration named once in `api/`; when it passes, the screen offers a retry.
- **Retries follow the server.** The client retries, or offers a retry, from the problem's `retryable` flag, never from its own list of codes.
- **Client ids on charged writes.** A charged write the client may send twice carries a client id, a UUID kept until the server has taken the request; a request the end user means as a new one (a regenerate) gets none.
- **An event stream** is parsed with a maintained parser (`eventsource-parser`) that honours `id:` for resuming, with URLs and headers from the generated client's base path and auth.
- **Storage.** `localStorage` and `sessionStorage` are reached only through one module that tolerates their absence; every key is a constant there, and a value read back is checked with a guard, never cast.
- **One source for environment-derived values:** the API base path, the end user's time zone and the locale each come from one helper.
- **No hard-coded values owned elsewhere.** Limits, enums and locale lists come from the generated API types or one module, never literals in a form; the API base path, the server's port and storage keys are each written once; a fact the server owns (a document's version) is read from it or generated, never copied.
- **Forms** validate with the API document's generated schemas through one form helper.
- **Logs.** No `console` output of end-user content; a reported error carries ids and codes only.

## Types and routes

- **Typed from the server's OpenAPI document:** no hand-written request or response types. Errors are the server's problems, read by `code` typed as `Problem["code"]`, never bare strings. A stream event's required fields are listed once, typed from the document.
- **Every route tree has an error element:** the root route has an `errorElement` (as does a layout whose children can fail apart from it) that shows the problem in the end user's locale with a way to retry or go home, never the router's default page or an error's own text; a test reads the routes and fails on a root without one.
- **One route over a list.** A family of pages is one route whose segment takes values from one list; a name the list lacks is a 404.

## Text and locales

- **Text** is only the i18n library's messages, through `i18n/`; dates, numbers and language names come from `Intl`.
- **Locales** are the i18n library's list, never written out; a value per locale is a `Record<Locale, …>` checked with `satisfies`, never a `locale === "…"` branch, a per-language name or a third-party locale switch that fails silently for a new locale.
- **The locale is read from the locale hook** and passed to messages and formatters, so the React Compiler sees it as an input and re-renders when it changes.

## Interface

How a screen looks and behaves (components, tokens, accessibility, interaction, public pages, screenshots) is `web-interface.md`.

- **One implementation per concern:** one formatter per kind of value, besides the hook rules above and `web-interface.md`'s component rule.

## Tests

- **One shared test root** sets up every test; the server is mocked with MSW handlers generated from the API document; the random seed and the reference date are fixed.

## Security and performance

- **Content Security Policy.** The build writes the page's CSP into `index.html`: its own files, each inline script by hash, each third-party origin by name; no `'unsafe-inline'` script. A new third-party origin is added there with a test; headers a meta tag cannot carry are the edge's.
- **Budget.** The shipped JavaScript has a budget the check enforces; screens are split with `lazy`, and heavy libraries load on first use. A speed or size claim measures the bundle and first render before and after.

## Operator consoles

- **An operator console is its own app,** not screens of an end-user client: its types come from the operator API's own document, and it shares with an end-user client only what both draw and validate with (design tokens, a project of primitives in them, and the helpers that turn generated schemas' issues and server refusals into each app's messages, named in the repository's `AGENTS.md`). An end-user client never reaches the operator API. The console's own components (an admin framework's layout, tables and views) stay in it.
- **A console on an admin framework's data hooks** gives the framework one data provider that calls only the operator API's generated client, typed route by route; the framework keys and invalidates queries by resource, the console's exception to the key rule above. Invalidation that is not plain resource invalidation (optimistic, conditional or across resources) stays in one `api/` module with its reason, as the key rule says.
