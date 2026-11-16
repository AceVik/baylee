-- Invalidations become visible only at transaction commit, including writes from
-- another authority/verifier process. No names, inventory or credentials ride here.
CREATE FUNCTION @tower.notify_change() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$
BEGIN
    IF TG_TABLE_NAME = 'runs' THEN
        PERFORM pg_catalog.pg_notify(TG_TABLE_SCHEMA, 'run:' || COALESCE(NEW.id, OLD.id)::text);
    ELSIF TG_TABLE_NAME = 'cohorts' THEN
        PERFORM pg_catalog.pg_notify(TG_TABLE_SCHEMA, 'catalog');
    ELSE
        PERFORM pg_catalog.pg_notify(TG_TABLE_SCHEMA, 'ranking');
    END IF;
    RETURN NULL;
END
$$;
CREATE TRIGGER run_changed AFTER INSERT OR UPDATE OR DELETE ON @tower.runs
    FOR EACH ROW EXECUTE FUNCTION @tower.notify_change();
CREATE TRIGGER catalog_changed AFTER INSERT OR UPDATE OR DELETE ON @tower.cohorts
    FOR EACH STATEMENT EXECUTE FUNCTION @tower.notify_change();
CREATE TRIGGER ranking_changed AFTER INSERT OR UPDATE OR DELETE ON @tower.personal_bests
    FOR EACH STATEMENT EXECUTE FUNCTION @tower.notify_change();
CREATE TRIGGER ranking_identity_changed AFTER UPDATE OF rank_name, rank_tag, rank_listed ON @tower.players
    FOR EACH STATEMENT EXECUTE FUNCTION @tower.notify_change();
