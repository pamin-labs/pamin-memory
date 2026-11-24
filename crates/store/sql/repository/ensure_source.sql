WITH found AS (
    SELECT id FROM sources WHERE project_id = $1 AND locator = $2 FOR UPDATE
), inserted AS (
    INSERT INTO sources (id, project_id, kind, locator, created_at)
    SELECT $3, $1, $4, $2, $5
     WHERE NOT EXISTS (SELECT 1 FROM found)
    ON CONFLICT (project_id, locator) DO NOTHING
    RETURNING id
)
SELECT id, false AS created FROM found
UNION ALL
SELECT id, true AS created FROM inserted
