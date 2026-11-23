//! Central generations isolate replicated tables from single-host result delivery.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE ranked_run ADD COLUMN generation bigint,
                ADD COLUMN primary_host boolean,
                ADD CONSTRAINT ranked_role CHECK
                    ((generation IS NULL AND primary_host IS NULL) OR
                     (generation IS NOT NULL AND generation > 0 AND primary_host IS NOT NULL));
             DROP INDEX ranked_delivery;
             CREATE INDEX ranked_legacy_delivery ON ranked_run(serviced_at,match_id)
                WHERE generation IS NULL;
             CREATE INDEX ranked_replica_expiry ON ranked_run(seen_at,match_id)
                WHERE generation IS NOT NULL",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Downgrading while a replicated match is outstanding would let the old
        // delivery worker settle or void it from a single engine's evidence.
        let connection = manager.get_connection();
        connection
            .execute_unprepared(
                "LOCK TABLE ranked_run IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN IF EXISTS (SELECT FROM ranked_run WHERE generation IS NOT NULL) THEN
                RAISE EXCEPTION 'replicated ranked games must finish before downgrade';
             END IF; END $$;
             DROP INDEX ranked_legacy_delivery, ranked_replica_expiry;
             CREATE INDEX ranked_delivery ON ranked_run(serviced_at,match_id);
             ALTER TABLE ranked_run DROP COLUMN primary_host, DROP COLUMN generation",
            )
            .await?;
        Ok(())
    }
}
