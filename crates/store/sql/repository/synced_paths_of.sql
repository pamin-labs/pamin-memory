SELECT p.name, s.path, s.vector_index
 FROM synced_paths s JOIN projects p ON p.id = s.project_id
WHERE s.project_id = $1
ORDER BY s.path
