INSERT INTO projects (id, name, created_at)
VALUES ($1, $2, $3)
ON CONFLICT (name) DO NOTHING
RETURNING id, name, created_at
