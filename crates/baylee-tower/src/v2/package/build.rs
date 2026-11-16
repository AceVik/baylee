use super::{
    Catalog, Collection, Definition, Deserialize, Phase, Refusal, Run, Serialize, Stack, loadout,
};

/// Rules-only pre-duel build. Does not carry gold, reserve items or acquisition data.
/// No Debug: an opponent's hidden deck must not enter ordinary operational logs.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// Run commander and sideboard; absent in legacy frozen builds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<crate::cards::Formation>,
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
            formation: run.cards.formation.clone(),
            deck: run.cards.deck.clone(),
            active: run.economy.inventory.active(),
            potions: run.economy.potions,
        };
        build.deck.sort_by_key(|stack| stack.card);
        if let Some(formation) = &mut build.formation {
            formation.sideboard.sort_by_key(|stack| stack.card);
        }
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
        let mut owned: Vec<_> = self
            .deck
            .iter()
            .chain(
                self.formation
                    .iter()
                    .flat_map(|formation| &formation.sideboard),
            )
            .filter(|stack| catalog.card(stack.card).is_none_or(|card| !card.basic_land))
            .copied()
            .collect();
        if let Some(formation) = &self.formation {
            if formation
                .sideboard
                .windows(2)
                .any(|pair| pair[0].card >= pair[1].card)
            {
                return Err(Refusal::Invalid);
            }
            owned.push(Stack {
                card: formation.commander,
                copies: 1,
            });
        }
        Collection {
            formation: self.formation.clone(),
            owned,
            deck: self.deck.clone(),
        }
        .validate(catalog)?;
        let generator = if catalog.is_run() {
            // Structural maximum only. validate_for separately enforces the
            // historical cohort's immutable catalog, including retained runs.
            crate::v2::generation::RUNIC
        } else {
            // Legacy builds have no Commander formation. Never reinterpret
            // their resource limits when the current new-run default changes.
            3
        };
        loadout::resources_for(&self.active, self.potions, 0, generator)?;
        Ok(())
    }

    /// Reject definitions introduced after the historical cohort's shop generation.
    /// # Errors
    /// Invalid build or an unsupported generation/definition.
    pub fn validate_for(&self, catalog: &Catalog, generator: u16) -> Result<(), Refusal> {
        crate::v2::generation::families(generator)?;
        if (crate::v2::generation::is_run(generator)) != catalog.is_run() {
            return Err(Refusal::Invalid);
        }
        for definition in &self.active {
            crate::v2::generation::definition(generator, *definition)?;
        }
        self.validate(catalog)
    }
}
