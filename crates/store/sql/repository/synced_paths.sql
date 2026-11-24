SELECT p.name, s.path, s.vector_index
 FROM synced_paths s JOIN projects p ON p.id = s.project_id
ORDER BY p.name, s.path
