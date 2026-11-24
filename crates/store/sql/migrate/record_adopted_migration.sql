INSERT INTO _sqlx_migrations
    (version, description, success, checksum, execution_time)
VALUES ($1, $2, TRUE, $3, -1)
ON CONFLICT (version) DO NOTHING
