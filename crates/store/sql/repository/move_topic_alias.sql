WITH gone AS (DELETE FROM topic_aliases WHERE project_id = $1 AND name = $3)
INSERT INTO topic_aliases (project_id, name, topic_id, renamed_at)
VALUES ($1, $2, $4, $5)
ON CONFLICT (project_id, name)
DO UPDATE SET topic_id = EXCLUDED.topic_id, renamed_at = EXCLUDED.renamed_at
