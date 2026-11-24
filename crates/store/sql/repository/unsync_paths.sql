DELETE FROM synced_paths WHERE project_id = $1 AND path = ANY($2) RETURNING path
