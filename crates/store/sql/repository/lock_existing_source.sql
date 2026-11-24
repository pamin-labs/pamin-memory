SELECT id, false AS created FROM sources WHERE project_id = $1 AND locator = $2 FOR UPDATE
