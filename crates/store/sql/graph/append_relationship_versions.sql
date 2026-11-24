WITH claim AS (
    SELECT * FROM unnest(
        $3::uuid[], $4::uuid[], $5::uuid[], $6::timestamptz[], $7::timestamptz[],
        $8::uuid[], $9::real[], $10::text[], $11::uuid[], $12::uuid[], $13::text[]
    ) AS claim(id, relationship_id, supersedes, valid_from, valid_to,
               caused_by, confidence, derivation, from_topic, to_topic, kind)
), closed AS (
    UPDATE relationship_versions
       SET invalidated_at = $2, tombstone_reason = $14
      FROM claim WHERE relationship_versions.id = claim.supersedes
)
INSERT INTO relationship_versions (
    id, project_id, relationship_id, version, valid_from, valid_to,
    created_at, supersedes, caused_by_topic_state, confidence, derivation,
    from_topic, to_topic, kind
)
SELECT claim.id, $1, claim.relationship_id,
       COALESCE((SELECT MAX(version) FROM relationship_versions
                  WHERE project_id = $1
                    AND relationship_id = claim.relationship_id), 0) + 1,
       claim.valid_from, claim.valid_to, $2, claim.supersedes, claim.caused_by,
       claim.confidence, claim.derivation, claim.from_topic, claim.to_topic,
       claim.kind
  FROM claim
RETURNING id, relationship_id, version, valid_from, valid_to, created_at,
          invalidated_at, supersedes, caused_by_topic_state,
          confidence, derivation, tombstone_reason
