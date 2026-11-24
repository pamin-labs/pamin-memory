SELECT count(*) FROM (
    SELECT 1 FROM index_jobs WHERE project_id = $1 LIMIT $2
) AS owed
