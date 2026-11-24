# Data, queries and migrations

Read this before touching a query, a migration, a transaction or a list a store serves. It adds the storage rules to `core.md`, whose layer, domain and hot-path rules still apply; a language's reference says how its database library meets them.

## Transactions

- **Rules inside a transaction are the domain's.** A rule judged atomically with a write gets its facts from the store, inside the transaction, through a function the domain passes in; the store reports counts and flags and never decides. A query takes the domain's numbers as parameters (a window's start, a limit, a reason), never naming a period, age or reason itself, and a rule written in SQL (which kind sorts first, which state a row returns to) becomes a parameter the domain passes, so the rule has one home.
- **Calls to other services stay out of transactions.** Commit, then call, idempotently so a failed call can be retried. What must still happen elsewhere is recorded in the change's transaction, and a worker retries it until done, with the other service's "not found" counting as done.
- **Per-transaction state only.** A deployed PostgreSQL may sit behind a transaction pooler, so a connection belongs to one transaction. Use `set local` or `set_config(name, value, true)`, and protocol-level prepared statements. Never use a session `set` or `set_config(..., false)` meant to outlast its transaction, session advisory locks (use `pg_advisory_xact_lock`), `LISTEN`/`NOTIFY`, temporary tables, SQL `PREPARE`, or cursors held across transactions. Test fixtures may. A test reads the queries and the store's code and fails on any of these, because a pooler breaks them silently, long after review.

## Queries

- **SQL lives only in `.sql` files,** never in code, not even as fragments, so every statement is readable, checked by the editor and seen by the plan tests; test fixtures aside, no SQL is assembled in code. The store uses no query builder or ORM: their SQL shapes escape the plan tests, eager loading adds round trips, and their migrations fight hand-written ones.
- **One query per concern,** reused, not copied. A column that holds one of a type's values decodes straight into that type, never through a hand-written conversion.
- **Literals are checked.** A query may write a literal for a typed column's value (a partial index's predicate needs one to serve a generic plan), and a test checks every such literal, compared, listed in `in (...)` or inserted, against the code type's list. A new closed set a query names gets its type and list first.
- **Query plans are tested.** A test explains each hot or list query and fails one that scans a growing table or sorts without bound, and requires the index each is meant to seek. Each keyset list seeks its index in both directions, forward and backward; the two halves of a paired list share their filters in one place, as the language's reference says, because a direction cannot be a parameter without losing the seek.

## Migrations

- **Schema changes expand, then contract.** A migration adds what new code needs and removes only what no deployed version reads, so the previous release runs on the new schema and can be rolled back to. A contract step may land with its expand step only before the first production deploy, when no deployed version reads the old shape.
- **Migrations and seed data run in the deploy step,** never at a service's start: at start a service only checks that the schema is at or above its own version, so several instances never race to migrate and a start never waits on one.
- **Migrations are hand-written, numbered and append-only** (a merged one is never edited, renamed or removed; a change is a new one), and checked for locking and destructive changes before they merge.
