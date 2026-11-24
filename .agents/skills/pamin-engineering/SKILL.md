---
name: pamin-engineering
description: The engineering bar shared by Pamin Labs repositories - fast, small, quick to build, clean, DRY, decoupled and easy to extend. Use it before writing or changing any code, before adding a dependency, before claiming something is faster or smaller, and when reviewing a change, even when the change looks small. Reach for it on words like Go, Rust, Zig, React, TypeScript, Cloudflare Worker, implement, refactor, optimise, add a crate or package, abstraction, interface, layer, handler, query, SQL, migration, pagination, locale, i18n, error code, setting, deploy, CI, screen, UI, accessibility, screenshot, performance, memory, binary size, bundle size, build time, benchmark, test, lint, review, PR, or "is this over-engineered".
---

# The engineering bar

Every repository that carries this skill holds code to one bar, stated once in `references/`; each `AGENTS.md` imports the references its code needs. This file says how to use them and states no rule of its own.

**The user** is the person the agent works for, who decides what the rules leave open and confirms what they ask to see. The people a product serves are always **end users**, so the two never blur.

## Decide before you write

Walk these in order and stop at the first that answers; each is a rule of `core.md`.

1. **Does it need to exist?** Not for a second use that has not appeared.
2. **Does it exist already?** Search this repository, then its shared libraries, and extend the module that owns it through its interface ("One implementation per concern").
3. **Can something else do it?** Climb `core.md`'s "Do not reinvent what exists"; writing it yourself is the last rung, explained in the PR's "Reused" section.
4. **Where does it go?** Its layer and directory are in the area's `AGENTS.md` ("Layers here", the layout or module list); if none fits, the layout changes in the same PR.
5. **Will it be faster or smaller?** Measure before and after on the entry point the product calls, with both numbers in the PR. No number, no claim.

## Which reference to read

Read `core.md` and each reference for a language, platform or concern the diff changes, before writing and again before the PR opens; each reference opens with when to read it.

| Reference | Read it when |
| --- | --- |
| `references/core.md` | Always: shape of code, text and locales, layers and domains, errors and API documents, settings, calls to other services, resources, review, commits and PRs |
| `references/data.md` | Before touching a query, a migration, a transaction or a list a store serves: the pooler, SQL files, checked literals, query plans, migrations at deploy |
| `references/rust.md` | Writing or reviewing Rust: toolchain, module layout, errors, panics, `unsafe`, types and JSON, async and tasks, logs, SQL, tests, performance |
| `references/go.md` | Writing or reviewing Go: tools, package layout, file layout, errors, context, concurrency, types, tests |
| `references/zig.md` | Writing or reviewing Zig: toolchain, build modes, module layout, layers and reuse, names, allocators, error sets, illegal behaviour, SIMD, tests |
| `references/performance.md` | Touching a request's hot path, a pool, start-up, or a benchmark, or claiming a change is faster, smaller or cheaper (its measurement table) |
| `references/typescript.md` | TypeScript anywhere: apps, Workers, tooling, shared libraries |
| `references/web.md` | A React, Vite and TypeScript web client or operator console: modules, the React Compiler, server state, types and guards, locales, CSP and budget, operator consoles |
| `references/web-interface.md` | Changing what a screen shows or how it responds, or reviewing a design: components and tokens, accessibility, interaction, public pages, design findings, PR screenshots |
| `references/workers.md` | A Cloudflare Worker: routing, 404s, containers, config, deploys and image pruning |
| `references/rule-scan.md` | Running or continuing a rule-scan pass over a whole repository |

`AGENTS.md` files import these with `@` lines, so Claude Code loads them already; other tools open the paths they name.

## When the reference and the repository disagree

- A repository's `AGENTS.md` says how it meets each rule and what it has not built yet; it never weakens a rule, and a gap it names is no licence to widen: new code follows the rule.
- A rule that seems wrong for a case is raised with the user and recorded in a decision record, then changed at the skill's source (`pamin-labs/skills`; outside the team, your own fork); never work around it in code.

## Reviewing a change and opening the PR

Follow `core.md`'s "Working and review" and "Commits and PRs": rebase and build on what landed, run every touched `AGENTS.md`'s gates locally, hold the whole diff, tests included, against every imported reference and touched `AGENTS.md`, apply the extensibility check (would one more of the same thing touch one place?), fix misses in new commits, have it reviewed by a second reader, and put measurements and how they were taken under the PR's verification. In the team's own repositories, the coordination tool's peer-review and commit skills do the review and check the mechanics.

## Where a question goes

- A rule for every language: `core.md`; how one language meets it: that language's reference.
- How this repository meets it, its layers, crates or packages, and gates: its own `AGENTS.md`; history, worked examples and measured figures: the `docs/` pages it links.
- Why a rule exists and who decided it: the decision records, cited with a SHA permalink.

## Changing these rules

This skill's source is the public `pamin-labs/skills` repository; a repository installs it with the skills CLI and never edits its copy. A change for the team's repositories is made there (a project outside the team may keep its own fork), cites its decision record, and adds no repository-specific fact; those stay in the repository's `AGENTS.md`.
