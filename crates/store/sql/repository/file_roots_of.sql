SELECT substr(locator, 6), root FROM sources
WHERE project_id = $1 AND locator = ANY($2) AND root IS NOT NULL
