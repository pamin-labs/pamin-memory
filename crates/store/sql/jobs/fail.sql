UPDATE index_jobs
  SET last_error   = $3,
      claimed_at   = NULL,
      claimed_by   = NULL,
      available_at = CASE WHEN $4 THEN available_at ELSE $5 END
WHERE id = $1 AND claimed_by = $2
