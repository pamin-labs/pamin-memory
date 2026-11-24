SELECT id, name, path, created_at FROM topics
WHERE project_id = $1 AND name = $2
UNION ALL
SELECT t.id, t.name, t.path, t.created_at
  FROM topic_aliases a JOIN topics t ON t.id = a.topic_id
 WHERE a.project_id = $1 AND a.name = $2
   AND NOT EXISTS (
       SELECT 1 FROM topics WHERE project_id = $1 AND name = $2
   )
LIMIT 1
