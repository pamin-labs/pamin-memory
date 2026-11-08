-- Row-level security binds the tables' owner too.
--
-- Hosted, the connection is an ordinary role that ran the migrations and so
-- owns the tables, and an owner skips row-level security unless it is forced.
-- V20's guard then held only where a transaction took on pamin_memory_app;
-- any query outside one read every project. Forced, the owner has no policy of
-- its own and sees no guarded row, so a query outside a scoped transaction
-- reads nothing instead of everything.
--
-- A superuser still skips it, so a local workspace, whose connection is one,
-- behaves as before. Foreign-key cascades skip it too, so deleting a project
-- still removes its rows. A later migration that rewrites a guarded table's
-- rows has to lift the force around them; `migrate.rs` says how.

-- Every table V20 enabled it on, and no other.
DO $$
DECLARE
    guarded TEXT;
BEGIN
    FOR guarded IN
        SELECT relname FROM pg_class
         WHERE relnamespace = current_schema()::regnamespace AND relrowsecurity
    LOOP
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', guarded);
    END LOOP;
END
$$;
