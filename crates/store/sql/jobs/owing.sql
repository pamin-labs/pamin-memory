SELECT project.id, project.name,
      EXISTS (
          SELECT 1 FROM index_jobs
           WHERE project_id = project.id
             AND job_type = ANY($1)
             AND priority = ANY($2)
             AND available_at <= $3
             AND attempts < $4
             AND last_error IS NULL
      ) AS never_failed
 FROM projects AS project
WHERE EXISTS (
          SELECT 1 FROM index_jobs
           WHERE project_id = project.id
             AND job_type = ANY($1)
             AND priority = ANY($2)
             AND available_at <= $3
             AND attempts < $4
      )
ORDER BY project.name
