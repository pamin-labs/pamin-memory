UPDATE index_jobs
  SET attempts = 0, available_at = $2, claimed_at = NULL, claimed_by = NULL
WHERE project_id = $1 AND attempts >= $3
