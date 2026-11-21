---
name: pamin-engineering
description: The engineering bar for code in this repository - fast, small, quick to build, clean, DRY, decoupled and easy to extend. Use it before writing or changing any code, before adding a dependency, before claiming something is faster or smaller, and when reviewing a change, even when the change looks small. Reach for it on words like implement, refactor, optimise, add a crate or package, abstraction, interface, layer, handler, migration, error code, setting, performance, memory, binary size, bundle size, build time, or "is this over-engineered".
---

# The engineering bar

Every repository that carries this skill holds code to one bar, stated once in `references/`; each `AGENTS.md` imports the references its code needs. This file says how to use them and states no rule of its own.

## Decide before you write

Walk these in order and stop at the first that answers; each is a rule of `core.md`.

1. **Does it need to exist?** Not for a second use that has not appeared.
2. **Does it exist already?** Search this repository, then its shared libraries, and extend the owner through its interface ("One implementation per concern").
3. **Can something else do it?** Climb `core.md`'s "Do not reinvent what exists"; writing it yourself is the last rung, explained in the PR's "Reused" section.
4. **Where does it go?** Its layer and directory are in the area's `AGENTS.md` ("Layers here", the layout or module list); if none fits, the layout changes in the same PR.
5. **Will it be faster or smaller?** Measure before and after on the entry point the product calls, with both numbers in the PR. No number, no claim.

## Which reference to read

Read every reference the change touches, before writing and again before the PR opens.

| Reference | Read it when |
| --- | --- |
| `references/core.md` | Always: shape of code, layers and domains, errors, data, settings, calls to other services, resources, review, commits and PRs |
| `references/rust.md` | Writing or reviewing Rust: toolchain, errors, panics, `unsafe`, async, tasks, SQL, tests |
| `references/go.md` | Writing or reviewing Go: tools, file layout, errors, context, concurrency, types, tests |
| `references/performance.md` | Touching a request's hot path, a pool, start-up, or a benchmark |
| `references/web.md` | A React, Vite and TypeScript web client or operator console |
| `references/workers.md` | A Cloudflare Worker: routing, containers, config, deploys |
| `references/rule-scan.md` | Running or continuing a rule-scan pass over a whole repository |

`AGENTS.md` files import these with `@` lines, so Claude Code loads them already; other tools open the paths they name.

## When the reference and the repository disagree

- A repository's `AGENTS.md` says how it meets each rule and what it has not built yet; it never weakens a rule, and a gap it names is no licence to widen: new code follows the rule.
- A rule that seems wrong for a case is raised with the owner and recorded in a decision record, then changed in `pamin-labs/skills`; never work around it in code.

## Reviewing a change and opening the PR

Follow `core.md`'s "Working and review" and "Commits and PRs": rebase and build on what landed, run every touched `AGENTS.md`'s gates locally, hold the whole diff, tests included, against every imported reference and touched `AGENTS.md`, apply the extensibility check (would one more of the same thing touch one place?), fix misses in new commits, run the peer-review skill, and put measurements and how they were taken under the PR's verification. The coordination tool's commit skill checks the mechanics.

## Where a question goes

- A rule for every language: `core.md`; how one language meets it: that language's reference.
- How this repository meets it, its layers, crates or packages, and gates: its own `AGENTS.md`.
- Why a rule exists and who decided it: the decision records, cited with a SHA permalink.

## Changing these rules

This skill's source is the public `pamin-labs/skills` repository; a repository installs it with the skills CLI and never edits its copy. A change is made there, cites its decision record, and adds no repository-specific fact; those stay in the repository's `AGENTS.md`.
