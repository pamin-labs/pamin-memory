INSERT INTO synced_paths (project_id, path, vector_index, added_at)
SELECT $1, path, $3, now() FROM unnest($2::text[]) AS path
ON CONFLICT (project_id, path) DO UPDATE SET vector_index = excluded.vector_index
