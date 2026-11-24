INSERT INTO file_records AS f
       (project_id, root, file, size, modified_ns, changed_ns, inode, read_ns,
        content_hash, sections, build)
SELECT $1, * FROM unnest($2::text[], $3::text[], $4::bigint[], $5::bigint[],
                         $6::bigint[], $7::bigint[], $8::bigint[], $9::text[],
                         $10::int[], $11::text[])
ON CONFLICT (project_id, root, file) DO UPDATE
   SET size = excluded.size, modified_ns = excluded.modified_ns,
       changed_ns = excluded.changed_ns, inode = excluded.inode,
       read_ns = excluded.read_ns, content_hash = excluded.content_hash,
       sections = excluded.sections, build = excluded.build
