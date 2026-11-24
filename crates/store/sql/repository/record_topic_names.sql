INSERT INTO topic_name_tokens (project_id, topic_id, name_key, token_count)
SELECT $1, topic_id, name_key, token_count
  FROM unnest($2::uuid[], $3::text[], $4::smallint[])
    AS incoming(topic_id, name_key, token_count)
ON CONFLICT (project_id, topic_id) DO UPDATE
    SET name_key = EXCLUDED.name_key, token_count = EXCLUDED.token_count
