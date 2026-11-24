SELECT passage_hash, encoding, vector FROM span_vectors
WHERE project_id = $1 AND revision = $2 AND passage_hash = ANY($3)
  AND ($4::text IS NULL OR encoding = $4)
