use super::{
    Catalog, Deserialize, Digest, GENERATOR, Identity, PACK, Refusal, Serialize, Sha256, Variant,
};
use crate::v2::generation;

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
    catalog_digest_for(catalog, GENERATOR)
}

/// Canonical commitment using the saved generation, never the current default.
/// # Errors
/// Unsupported generation or malformed catalog.
pub fn catalog_digest_for(catalog: &Catalog, generator: u16) -> Result<[u8; 32], Refusal> {
    let contract = generation::contract(generator)?;
    catalog.validate()?;
    let mut cards = catalog.cards.clone();
    cards.sort_by_key(|card| card.card);
    Ok(Sha256::new()
        .chain_update(b"baylee/tower/catalog/v2\0")
        .chain_update(PACK.to_le_bytes())
        .chain_update(contract.revision.to_le_bytes())
        .chain_update(generator.to_le_bytes())
        .chain_update(serde_json::to_vec(&cards).map_err(|_| Refusal::Invalid)?)
        .finalize()
        .into())
}

impl Cohort {
    /// Construct the current rules contract for an explicitly selected Tower artifact.
    /// # Errors
    /// Unsupported execution variant/policy or malformed catalog.
    pub fn new(executor: Identity, catalog: &Catalog) -> Result<Self, Refusal> {
        Self::for_generation(executor, catalog, GENERATOR)
    }

    /// Construct an explicitly supported frozen contract without changing an old run.
    /// # Errors
    /// Unsupported generation, execution identity or malformed catalog.
    pub fn for_generation(
        executor: Identity,
        catalog: &Catalog,
        generator: u16,
    ) -> Result<Self, Refusal> {
        let result = Self {
            revision: generation::contract(generator)?.revision,
            generator,
            executor,
            catalog: catalog_digest_for(catalog, generator)?,
        };
        result.validate(catalog)?;
        Ok(result)
    }

    /// Validate rules/policy semantics; artifact equality is checked at execution.
    /// # Errors
    /// A changed rules, generation, policy, variant or catalog contract.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        let contract = generation::contract(self.generator)?;
        if self.revision != contract.revision
            || !self.executor.valid()
            || self.executor.variant != Variant::Tower
            || self.executor.rules != format!("tower-{}", contract.revision)
            || self.executor.ai != format!("tower-house-{}", contract.ai_generation)
            || self.catalog != catalog_digest_for(catalog, self.generator)?
        {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }
}
