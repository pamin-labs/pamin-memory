# Performance

Rules for code on a request's path (a service's hot path, named in its `AGENTS.md`), in any language. Read `core.md` first: its "Parse once", "Hot paths keep their database round trips flat", bounded state and "Measure before and after" rules apply too. `rust.md` and `go.md` say how their code meets these.

- **Hot paths allocate once.** No allocation or cloning inside loops: reuse buffers sized from what is known, never build an intermediate collection only to iterate it once, and never format a value only compared or written out.
- **No repeated parsing** on a hot path; reuse existing optimised kernels rather than writing your own loop.
- **No blocking I/O on async paths;** blocking work runs on a thread meant for it.
- **Static dispatch on hot paths;** dynamic dispatch only at boundaries crossed once per request or less.
- **Connections:** a database pool's size is set explicitly, and a transaction holds its connection only for SQL, never across a call to another service.
- **Start-up** does not wait on other services: connecting, fetching keys, listing tools and loading what can wait happen on first use or in the background, unless the program can answer nothing without them.
- **Benchmarks** run on the entry point the product calls, before and after the change, at the deployed instance's size when the claim is about production; both numbers go in the PR.
