SELECT sv.content_hash FROM sources s
  JOIN source_versions sv ON sv.project_id = s.project_id AND sv.source_id = s.id
WHERE s.project_id = $1 AND s.locator = $2
ORDER BY sv.version DESC LIMIT 1
