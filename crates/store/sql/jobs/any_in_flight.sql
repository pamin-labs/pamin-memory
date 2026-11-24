SELECT EXISTS (
    SELECT 1 FROM index_jobs
     WHERE project_id = $1
       AND (
           (claimed_at IS NOT NULL AND available_at > $2)
           OR (attempts < $3 AND (claimed_at IS NOT NULL OR available_at <= $2))
       )
)
