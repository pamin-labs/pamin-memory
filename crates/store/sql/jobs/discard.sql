DELETE FROM index_jobs
WHERE project_id = $1 AND attempts >= $2
