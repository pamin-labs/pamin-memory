DELETE FROM file_records WHERE project_id = $1 AND file = ANY($2)
