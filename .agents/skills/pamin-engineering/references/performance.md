# Performance

Read this when touching a request's hot path (named in the service's `AGENTS.md`), a pool, start-up or a benchmark, or before claiming a change is faster, smaller or cheaper, in any language. Read `core.md` first: its "Parse once", "Hot paths keep their database round trips flat", bounded caches and state and "Measure before and after" rules apply too and are not repeated here, nor is `data.md`'s "Calls to other services stay out of transactions". `rust.md`, `go.md` and `zig.md` say how their code meets these.

- **Hot paths allocate once.** No allocation or cloning inside loops: reuse buffers sized from what is known, never build an intermediate collection only to iterate it once, and never format a value only compared or written out.
- **No repeated parsing** on a hot path: a value is parsed once and its parsed form passed on.
- **Reuse existing optimised kernels** (the standard library's or a dependency's search, hashing or SIMD routines) rather than writing your own loop.
- **Where the runtime schedules async tasks, blocking work runs on a thread meant for it.**
- **Static dispatch on hot paths;** dynamic dispatch only at boundaries crossed once per request or less.
- **Connections:** a database pool's size is set explicitly.
- **Start-up** does not wait on other services: connecting, fetching keys, listing tools and loading what can wait happen on first use or in the background, unless the program can answer nothing without them.
- **Benchmarks** for a claim about production run at the deployed instance's size.
- **A measurement PR carries a table:** metric, before, after, absolute difference and percentage, with links to the kept evidence. A cell not measured or not applicable says N/A and why, so no table implies a gain or a lack of regression nobody measured.
- **Name what was compared:** the commits, the hardware, cold or warm runs, and the settings and inputs that change the result. Timing arms are repeated and rotated so noise is not read as a gain; one pass is provisional. A component's figure is labelled as such and never stands for the entry point's.
