//! Immutable, explicitly labelled bootstrap opponents. Historical echoes take priority.
use super::{ConnectionTrait, Error, Principal, Refusal, Role, Store, Uuid, encode};
use baylee_tower::v2::{MAX_STAGES, package::Build};

impl Store {
    /// Publish a reviewed house fallback, frozen by cohort/stage/archetype.
    /// Structural admission is not a balance or acquisition-path certificate.
    /// # Errors
    /// Wrong role, invalid build/stage or conflict with already published content.
    pub async fn publish_house(
        &self,
        principal: &Principal,
        cohort: Uuid,
        stage: u8,
        archetype: u8,
        build: &Build,
    ) -> Result<(), Error> {
        if !(1..=MAX_STAGES).contains(&stage) || !(1..=8).contains(&archetype) {
            return Err(Refusal::Invalid.into());
        }
        let tx = self.transaction().await?;
        self.role(&tx, principal, Role::Publisher).await?;
        let publication = self.publication(&tx, cohort, false).await?;
        if usize::from(archetype) > publication.starters.len() {
            return Err(Refusal::Invalid.into());
        }
        build.validate_for(&publication.catalog, publication.cohort.generator)?;
        let bytes = encode(build)?;
        if bytes.len() > 16 * 1024 {
            return Err(Refusal::Capacity.into());
        }
        let changed = tx.execute_raw(self.statement("INSERT INTO @tower.house_builds AS saved(cohort,stage,archetype,build) VALUES($1,$2,$3,$4)
            ON CONFLICT(cohort,stage,archetype) DO UPDATE SET build=EXCLUDED.build WHERE saved.build=EXCLUDED.build",
            vec![cohort.into(), i16::from(stage).into(), i16::from(archetype).into(), bytes.into()])).await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}
