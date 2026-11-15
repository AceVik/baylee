//! Private, immutable duel material. A gateway executes this; a player never receives it.
use crate::{
    MAX_STAGES, RULES_VERSION, Refusal,
    cards::{Catalog, Collection, Stack},
    economy::Economy,
    items::Equipment,
    loadout,
    matching::Cohort,
    run::{Phase, Run},
};
use baylee_core::{
    card_metadata::{DeckChoices, RulesMetadata},
    ids::{CardIndex, PrintRef},
    preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatController, SeatSpec,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Both bounded decks and inventory fit comfortably below this transport limit.
pub const MAX_PACKAGE_BYTES: usize = 128 * 1024;

/// Rules-relevant part of one frozen build, without its owner's wallet or private collection.
/// No `Debug`: a reserved opponent's hidden deck must not enter ordinary diagnostics.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// Legal deck from the run's owned cards, ordered by rules identity.
    pub deck: Vec<Stack>,
    /// Purchased run-only relics.
    pub equipment: Vec<Equipment>,
    /// Healing and mana charges present before the duel.
    pub potions: [u8; 2],
}

impl Build {
    /// Freeze the build before queueing, never after learning the opponent.
    /// # Errors
    /// Invalid run or a run that is no longer shopping.
    pub fn freeze(run: &Run, catalog: &Catalog) -> Result<Self, Refusal> {
        run.validate(catalog)?;
        if run.phase != Phase::Shop {
            return Err(Refusal::Unavailable);
        }
        let mut build = Self {
            deck: run.cards.deck.clone(),
            equipment: run.economy.equipment.clone(),
            potions: run.economy.potions,
        };
        build.deck.sort_by_key(|stack| stack.card);
        build.equipment.sort_by_key(|item| item.relic);
        Ok(build)
    }

    /// Bounds and rules check for an already authorized frozen snapshot.
    /// Ownership is checked by `freeze`, not inferred from this portable value.
    /// # Errors
    /// Invalid card, copy, deck-size, equipment or charge limits.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        // Check the bound before constructing any derived inventory.
        if self.deck.len() > usize::from(crate::cards::DECK_MAX) {
            return Err(Refusal::Capacity);
        }
        Collection {
            formation: None,
            owned: self
                .deck
                .iter()
                .filter(|stack| catalog.card(stack.card).is_none_or(|card| !card.basic_land))
                .copied()
                .collect(),
            deck: self.deck.clone(),
        }
        .validate(catalog)?;
        self.economy().validate()
    }

    fn economy(&self) -> Economy {
        Economy {
            gold: 0,
            equipment: self.equipment.clone(),
            potions: self.potions,
        }
    }
}

/// Exact reserved execution input; delivered only to authenticated execution capacity.
/// The service binds its digest to the reservation and never accepts a player's replacement.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    /// Transport schema, independent from the balance version.
    pub schema: u8,
    /// Immutable balance, catalog and exact executor cohort.
    pub cohort: Cohort,
    /// Same progression ordinal for both builds, independent of calendar date.
    pub stage: u8,
    /// Human first, Expert-controlled echo or clearly labelled house fallback second.
    pub builds: [Build; 2],
    /// Private random shuffle seed, fixed once per reservation.
    pub shuffle_seed: u64,
    /// Independent private policy seed; reconnect never changes the opponent's policy.
    pub policy_seed: u64,
}

/// Canonical catalog fingerprint, including this economy and item rules revision.
/// # Errors
/// The catalog violates its bounds or cannot be encoded.
pub fn catalog_digest(catalog: &Catalog) -> Result<[u8; 32], Refusal> {
    catalog.validate()?;
    let mut cards = catalog.cards.clone();
    cards.sort_by_key(|card| card.card);
    let encoded = serde_json::to_vec(&cards).map_err(|_| Refusal::Invalid)?;
    Ok(Sha256::new()
        .chain_update(b"baylee/tower/catalog/v1\0")
        .chain_update(RULES_VERSION.to_le_bytes())
        .chain_update(encoded)
        .finalize()
        .into())
}

impl Package {
    /// Validate before executing; compatibility is not a signature or authorization.
    /// # Errors
    /// Wrong schema, stage, rules, executor, catalog or malformed build.
    pub fn validate(&self, catalog: &Catalog, executor: &str) -> Result<(), Refusal> {
        self.cohort.validate()?;
        if self.schema != 1
            || self.cohort.rules != RULES_VERSION
            || self.cohort.executor != executor
            || self.cohort.catalog != catalog_digest(catalog)?
            || !(1..=MAX_STAGES).contains(&self.stage)
        {
            return Err(Refusal::Invalid);
        }
        for build in &self.builds {
            build.validate(catalog)?;
        }
        Ok(())
    }

    /// Derive the entire engine setup from purchased resources and frozen cards.
    /// There is no incoming arbitrary preset, capability, script or fixed opening hand.
    /// `printing` resolves this executor's canonical registry artwork, never a player's URL.
    /// # Errors
    /// Invalid package or resources that cannot form a legal engine preset.
    pub fn preset(
        &self,
        catalog: &Catalog,
        executor: &str,
        printing: impl Fn(CardIndex) -> Option<PrintInfo>,
    ) -> Result<GamePreset, Refusal> {
        self.validate(catalog, executor)?;
        let resources = [
            loadout::resources(&self.builds[0].economy(), 0)?,
            loadout::resources(&self.builds[1].economy(), 1)?,
        ];
        let mut prints = Vec::new();
        let seats = vec![
            seat(&self.builds[0], &resources[0], &mut prints, &printing)?,
            seat(&self.builds[1], &resources[1], &mut prints, &printing)?,
        ];
        let preset = GamePreset {
            format: FormatId::Custom,
            seed: self.shuffle_seed,
            house_rules: HouseRules {
                challenge_policy_seed: Some(self.policy_seed),
                decision_timeout_secs: 0,
                reconnect_window_secs: 0,
                time_extension_votes: false,
                opening: resources.iter().map(|r| r.opening).collect(),
                supplies: resources.iter().map(|r| r.supplies).collect(),
                ..HouseRules::for_a_table()
            },
            modifiers: Vec::new(),
            prints,
            rules: RulesMetadata::default(),
            seats,
        };
        preset.validate().map_err(|_| Refusal::Invalid)?;
        Ok(preset)
    }

    /// Bounded decoding at the authenticated executor boundary.
    /// # Errors
    /// Oversized, malformed or incompatible private execution material.
    pub fn decode(bytes: &[u8], catalog: &Catalog, executor: &str) -> Result<Self, Refusal> {
        if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES {
            return Err(Refusal::Invalid);
        }
        let package: Self = serde_json::from_slice(bytes).map_err(|_| Refusal::Invalid)?;
        package.validate(catalog, executor)?;
        Ok(package)
    }

    /// Validate and encode frozen material for durable storage and delivery.
    /// # Errors
    /// Invalid or oversized package.
    pub fn encode(&self, catalog: &Catalog) -> Result<Vec<u8>, Refusal> {
        self.validate(catalog, &self.cohort.executor)?;
        let bytes = serde_json::to_vec(self).map_err(|_| Refusal::Invalid)?;
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(Refusal::Capacity);
        }
        Ok(bytes)
    }
}

fn seat(
    build: &Build,
    resources: &loadout::DuelResources,
    prints: &mut Vec<PrintInfo>,
    printing: &impl Fn(CardIndex) -> Option<PrintInfo>,
) -> Result<SeatSpec, Refusal> {
    let mut deck = Vec::new();
    for stack in &build.deck {
        let print = PrintRef::new(u16::try_from(prints.len()).map_err(|_| Refusal::Capacity)?);
        prints.push(printing(stack.card).ok_or(Refusal::Unavailable)?);
        deck.extend(std::iter::repeat_n(
            DeckEntry {
                card: stack.card,
                print,
                rule_print: None,
                choices: DeckChoices::default(),
            },
            usize::from(stack.copies),
        ));
    }
    Ok(SeatSpec {
        controller: if resources.opening.seat == 0 {
            SeatController::Open
        } else {
            SeatController::Ai(AIProfile::TOWER_EXPERT)
        },
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck,
        sideboard: Vec::new(),
        commanders: Vec::new(),
        starting_life: Some(resources.starting_life),
        starting_hand: None,
        starting_battlefield: Vec::new(),
        emblems: Vec::new(),
        team: None,
    })
}

#[cfg(test)]
mod tests;
