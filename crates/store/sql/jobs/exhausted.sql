SELECT id, project_id, job_type, subject, attempts, claimed_at, last_error
 FROM index_jobs
WHERE project_id = $1 AND attempts >= $2
ORDER BY priority, available_at
