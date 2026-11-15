use super::{
    Catalog, Collection, Definition, Deserialize, Phase, Refusal, Run, Serialize, Stack, loadout,
};

/// Rules-only pre-duel build. Does not carry gold, reserve items or acquisition data.
/// No Debug: an opponent's hidden deck must not enter ordinary operational logs.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// Forty to sixty cards in canonical ascending identity order.
    pub deck: Vec<Stack>,
    /// Active definitions in slot order; reserve equipment contributes nothing.
    pub active: Vec<Definition>,
    /// Human-owned or echo-snapshot healing/mana charges before the duel.
    pub potions: [u8; 2],
}

impl Build {
    /// Freeze an authority-owned shopping state before revealing the next opponent.
    /// # Errors
    /// Invalid run, wrong phase or unsupported build.
    pub fn freeze(run: &Run, catalog: &Catalog) -> Result<Self, Refusal> {
        run.validate(catalog)?;
        if run.phase != Phase::Shop {
            return Err(Refusal::Unavailable);
        }
        let mut build = Self {
            deck: run.cards.deck.clone(),
            active: run.economy.inventory.active(),
            potions: run.economy.potions,
        };
        build.deck.sort_by_key(|stack| stack.card);
        build.validate(catalog)?;
        Ok(build)
    }

    /// Validate an already-authorized historical build, not its acquisition provenance.
    /// # Errors
    /// Wrong card/count/order, active resource caps or charge limits.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        if self.deck.len() > usize::from(crate::cards::DECK_MAX)
            || self
                .deck
                .windows(2)
                .any(|pair| pair[0].card >= pair[1].card)
        {
            return Err(Refusal::Invalid);
        }
        Collection {
            owned: self
                .deck
                .iter()
                .filter(|stack| catalog.card(stack.card).is_none_or(|card| !card.basic_land))
                .copied()
                .collect(),
            deck: self.deck.clone(),
        }
        .validate(catalog)?;
        loadout::resources(&self.active, self.potions, 0)?;
        Ok(())
    }

    /// Reject definitions introduced after the historical cohort's shop generation.
    /// # Errors
    /// Invalid build or an unsupported generation/definition.
    pub fn validate_for(&self, catalog: &Catalog, generator: u16) -> Result<(), Refusal> {
        crate::v2::generation::families(generator)?;
        for definition in &self.active {
            crate::v2::generation::definition(generator, *definition)?;
        }
        self.validate(catalog)
    }
}
