INSERT INTO span_vectors (project_id, revision, passage_hash, encoding, vector)
SELECT $1, $2, passage_hash, $3, vector FROM unnest($4::bytea[], $5::bytea[])
       AS v (passage_hash, vector)
ON CONFLICT (project_id, revision, passage_hash, encoding) DO UPDATE
   SET vector = EXCLUDED.vector
 WHERE octet_length(span_vectors.vector) <> octet_length(EXCLUDED.vector)
