WITH topic AS (
    INSERT INTO topics (id, project_id, name, created_at)
    VALUES ($1, $2, $3, $4)
    ON CONFLICT (project_id, name) DO NOTHING
    RETURNING id, name, path, created_at, current_state_id
), named AS (
    INSERT INTO topic_name_tokens (project_id, topic_id, name_key, token_count)
    SELECT $2, id, $5, $6 FROM topic WHERE $5::text IS NOT NULL
)
SELECT id, name, path, created_at, current_state_id FROM topic
