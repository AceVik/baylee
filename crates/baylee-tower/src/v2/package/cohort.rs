use super::{
    Catalog, Deserialize, Digest, GENERATOR, Identity, PACK, REVISION, Refusal, Serialize, Sha256,
    Variant,
};
use baylee_core::preset::AIProfile;

/// Full frozen compatibility identity, independent of gateway or calendar date.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohort {
    /// Native pack/economy revision, fixed at enrollment.
    pub revision: u16,
    /// Shop-generation revision, fixed at enrollment.
    pub generator: u16,
    /// Exact locally installed execution identity, never a command or path.
    pub executor: Identity,
    /// Canonical admitted cards/prices bound to this native revision.
    pub catalog: [u8; 32],
}

/// Canonical card catalog and revision commitment, insensitive to input ordering.
/// # Errors
/// Malformed catalog or encoding failure.
pub fn catalog_digest(catalog: &Catalog) -> Result<[u8; 32], Refusal> {
    catalog.validate()?;
    let mut cards = catalog.cards.clone();
    cards.sort_by_key(|card| card.card);
    Ok(Sha256::new()
        .chain_update(b"baylee/tower/catalog/v2\0")
        .chain_update(PACK.to_le_bytes())
        .chain_update(REVISION.to_le_bytes())
        .chain_update(GENERATOR.to_le_bytes())
        .chain_update(serde_json::to_vec(&cards).map_err(|_| Refusal::Invalid)?)
        .finalize()
        .into())
}

impl Cohort {
    /// Construct the current rules contract for an explicitly selected Tower artifact.
    /// # Errors
    /// Unsupported execution variant/policy or malformed catalog.
    pub fn new(executor: Identity, catalog: &Catalog) -> Result<Self, Refusal> {
        let result = Self {
            revision: REVISION,
            generator: GENERATOR,
            executor,
            catalog: catalog_digest(catalog)?,
        };
        result.validate(catalog)?;
        Ok(result)
    }

    /// Validate rules/policy semantics; artifact equality is checked at execution.
    /// # Errors
    /// A changed rules, generation, policy, variant or catalog contract.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        if self.revision != REVISION
            || self.generator != GENERATOR
            || !self.executor.valid()
            || self.executor.variant != Variant::Tower
            || self.executor.rules != format!("tower-{REVISION}")
            || self.executor.ai != format!("tower-house-{}", AIProfile::GENERATION)
            || self.catalog != catalog_digest(catalog)?
        {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }
}
