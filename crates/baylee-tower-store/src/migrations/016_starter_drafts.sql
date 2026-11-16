-- Publication-generation validation still bounds retained three/eight archetypes.
-- Expanded cohorts freeze fifty to sixty-four eligible commanders.
ALTER TABLE @tower.house_builds DROP CONSTRAINT house_builds_archetype_check;
ALTER TABLE @tower.house_builds ADD CONSTRAINT house_builds_archetype_check
    CHECK (archetype BETWEEN 1 AND 64);

-- Only operator-reviewed immutable content grows. Player state, request and
-- receipt limits remain 128 KiB. Three publication projections stay below 1 MiB.
ALTER TABLE @tower.cohorts DROP CONSTRAINT cohorts_publication_check;
ALTER TABLE @tower.cohorts ADD CONSTRAINT cohorts_publication_check
    CHECK (octet_length(publication) BETWEEN 1 AND 262144);

-- Initial draft reads do not notify (and cannot form a read/write feedback loop).
-- Only verified settlement replaces an existing start seed.
CREATE FUNCTION @tower.notify_draft() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$
BEGIN
    PERFORM pg_catalog.pg_notify(TG_TABLE_SCHEMA, 'draft:' || NEW.account::text);
    RETURN NULL;
END
$$;
CREATE TRIGGER draft_changed AFTER UPDATE OF start_seed ON @tower.enrollment
    FOR EACH ROW WHEN (OLD.start_seed IS DISTINCT FROM NEW.start_seed)
    EXECUTE FUNCTION @tower.notify_draft();
