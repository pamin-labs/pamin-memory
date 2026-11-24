SELECT rolname::text FROM pg_roles
WHERE rolname IN (current_user, $1) AND (rolsuper OR rolbypassrls)
ORDER BY rolname
