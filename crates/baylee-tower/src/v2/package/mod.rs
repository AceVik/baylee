//! Private immutable execution material. Players receive only a deliberately
//! restricted projection, never this package or its historical hidden deck.
use super::{
    GENERATOR, MAX_STAGES, PACK,
    catalog::Definition,
    loadout,
    run::{Phase, Run},
};
use crate::{
    Refusal,
    cards::{Catalog, Collection, Stack},
};
use baylee_core::{
    ids::CardIndex,
    preset::{GamePreset, PrintInfo},
};
use baylee_protocol::execution::{Identity, Variant};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod build;
mod cohort;
mod preset;
pub use build::Build;
pub use cohort::{Cohort, catalog_digest, catalog_digest_for};

/// Both bounded decks and active equipment, including the full execution identity.
pub const MAX_PACKAGE_BYTES: usize = 128 * 1024;

/// Attribution reference, resolved through current visibility at projection time.
/// A house fallback can never masquerade as a player snapshot.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Origin {
    /// Historical pre-duel player build admitted after verified completion.
    Echo {
        /// Database-generated immutable snapshot `UUIDv7`.
        snapshot: [u8; 16],
        /// Central account `UUIDv7`; never a credential or copied display name.
        account: [u8; 16],
    },
    /// Explicitly labelled fallback using a reviewed starter archetype.
    House {
        /// One-based frozen starter identity, initially three archetypes.
        archetype: u8,
    },
}

impl Origin {
    fn validate(&self) -> Result<(), Refusal> {
        let uuid7 = |bytes: &[u8; 16]| bytes[6] >> 4 == 7 && bytes[8] >> 6 == 2;
        let valid = match self {
            Self::Echo { snapshot, account } => uuid7(snapshot) && uuid7(account),
            Self::House { archetype } => (1..=3).contains(archetype),
        };
        if valid { Ok(()) } else { Err(Refusal::Invalid) }
    }
}

/// Already-authorized selection from the authority's same-ordinal eligible pool.
/// This portable value cannot itself prove eligibility, ownership or consent.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opponent {
    /// Same number of prior shop/income opportunities as the human.
    pub stage: u8,
    /// Exact catalog, rules, AI and executable contract.
    pub cohort: Cohort,
    /// Current attribution reference, not a historical display string.
    pub origin: Origin,
    /// Pre-duel deck/equipment/charges, not post-duel damage or consumed inventory.
    pub build: Build,
}

/// Independent private random streams, fixed once in the reservation transaction.
#[derive(Clone, Copy)]
pub struct Seeds {
    /// Shuffle and ordinary game RNG; never derived from public offer IDs.
    pub shuffle: u64,
    /// Frozen Expert policy stream, independent of hidden card ordering.
    pub policy: u64,
}

/// Exact execution input for trusted capacity. Intentionally does not implement Debug.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    /// Strict transport shape, separate from rules revision and generator identity.
    pub schema: u16,
    /// Immutable compatibility contract.
    pub cohort: Cohort,
    /// Same duel ordinal for both builds.
    pub stage: u8,
    /// Attribution reference; projection must recheck visibility/withdrawal.
    pub opponent: Origin,
    /// Human first, frozen Expert echo second.
    pub builds: [Build; 2],
    /// Private immutable shuffle stream.
    pub shuffle_seed: u64,
    /// Private immutable house policy stream.
    pub policy_seed: u64,
}

/// Commit both values together with the unique reservation and chosen opponent.
pub struct Reservation {
    /// Shop locked, package commitment installed, no economy change.
    pub run: Run,
    /// The sole private execution input for that reservation.
    pub package: Package,
}

impl Package {
    /// Freeze the owned loadout and an already-authorized same-stage selection.
    /// # Errors
    /// Stale/wrong-phase run, incompatible opponent/cohort or malformed package.
    pub fn reserve(
        run: &Run,
        revision: u64,
        catalog: &Catalog,
        cohort: &Cohort,
        opponent: Opponent,
        seeds: Seeds,
        byte_limit: u64,
    ) -> Result<Reservation, Refusal> {
        if opponent.stage != run.shop.stage
            || &opponent.cohort != cohort
            || run.shop.generator != cohort.generator
        {
            return Err(Refusal::Invalid);
        }
        let package = Self {
            schema: 2,
            cohort: cohort.clone(),
            stage: run.shop.stage,
            opponent: opponent.origin,
            builds: [Build::freeze(run, catalog)?, opponent.build],
            shuffle_seed: seeds.shuffle,
            policy_seed: seeds.policy,
        };
        let digest = package.digest(catalog)?;
        let run = run.reserved(revision, digest, byte_limit, catalog)?;
        Ok(Reservation { run, package })
    }

    /// Validate exact compatibility before executing; no fallback to another binary.
    /// # Errors
    /// Wrong version, identity, stage, attribution, catalog or build bounds.
    pub fn validate(&self, catalog: &Catalog, executor: &Identity) -> Result<(), Refusal> {
        self.cohort.validate(catalog)?;
        if self.schema != 2
            || &self.cohort.executor != executor
            || !(1..=MAX_STAGES).contains(&self.stage)
        {
            return Err(Refusal::Invalid);
        }
        self.opponent.validate()?;
        for build in &self.builds {
            build.validate_for(catalog, self.cohort.generator)?;
        }
        Ok(())
    }

    /// Strict bounded private-package decoding at the authenticated worker boundary.
    /// # Errors
    /// Oversized/malformed input or an incompatible executor/catalog.
    pub fn decode(bytes: &[u8], catalog: &Catalog, executor: &Identity) -> Result<Self, Refusal> {
        if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES {
            return Err(Refusal::Capacity);
        }
        let package: Self = serde_json::from_slice(bytes).map_err(|_| Refusal::Invalid)?;
        package.validate(catalog, executor)?;
        Ok(package)
    }

    /// Canonical validated encoding for authority storage and authenticated delivery.
    /// # Errors
    /// Invalid state or package size beyond the transport limit.
    pub fn encode(&self, catalog: &Catalog) -> Result<Vec<u8>, Refusal> {
        self.validate(catalog, &self.cohort.executor)?;
        let bytes = serde_json::to_vec(self).map_err(|_| Refusal::Invalid)?;
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(Refusal::Capacity);
        }
        Ok(bytes)
    }

    /// Canonical content commitment. This hash is not a credential or a signature.
    /// # Errors
    /// Invalid state/encoding.
    pub fn digest(&self, catalog: &Catalog) -> Result<[u8; 32], Refusal> {
        Ok(Sha256::new()
            .chain_update(b"baylee/tower/package/v2\0")
            .chain_update(self.encode(catalog)?)
            .finalize()
            .into())
    }
}

#[cfg(test)]
mod tests;
