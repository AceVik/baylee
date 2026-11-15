-- The publication remains authoritative for its exact starter count. Older
-- immutable cohorts still admit three; Commander run cohorts admit five to eight.
ALTER TABLE @tower.house_builds DROP CONSTRAINT house_builds_archetype_check;
ALTER TABLE @tower.house_builds ADD CONSTRAINT house_builds_archetype_check
    CHECK (archetype BETWEEN 1 AND 8);
