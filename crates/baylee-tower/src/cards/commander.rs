//! Closed forty-card format. Metadata is frozen by the publisher and checked
//! against the native registry at execution, never supplied in player commands.
use super::{BTreeSet, CardIndex, Catalog, Collection, Deserialize, Refusal, Serialize, Stack};
use baylee_core::color::ColorSet;

/// Identity and leader eligibility, independent of cosmetic printings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCard {
    /// Full rules color identity, including back faces and basic land types.
    pub identity: ColorSet,
    /// Admitted as the single commander; partner pairs are not part of this format.
    pub commander: bool,
}

/// Cards outside the 39-card main library, still bound to the run's ownership.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Formation {
    /// Exactly one owned commander, never also in the library or sideboard.
    pub commander: CardIndex,
    /// Up to five cards; global singleton and color identity apply here too.
    pub sideboard: Vec<Stack>,
}

impl Catalog {
    /// Admit the complete set of starting choices under this catalog's format.
    /// # Errors
    /// Invalid catalog/decks, wrong choice count or repeated run commanders.
    pub fn validate_starters(&self, starters: &[Collection]) -> Result<(), Refusal> {
        if if self.is_run() {
            !(5..=8).contains(&starters.len())
        } else {
            starters.len() != 3
        } {
            return Err(Refusal::Invalid);
        }
        self.validate_distinct_starters(starters)
    }

    /// The expanded, immutable publication pool. Enrollment separately authorizes
    /// three sampled choices; this does not change the older five-choice contract.
    /// # Errors
    /// Wrong pool size, legacy format, repeated commander or illegal formation.
    pub fn validate_starter_pool(&self, starters: &[Collection]) -> Result<(), Refusal> {
        if !self.is_run() || !(50..=64).contains(&starters.len()) {
            return Err(Refusal::Invalid);
        }
        self.validate_distinct_starters(starters)
    }

    fn validate_distinct_starters(&self, starters: &[Collection]) -> Result<(), Refusal> {
        self.validate()?;
        let mut leaders = BTreeSet::new();
        for starter in starters {
            starter.validate_cards(self)?;
            if let Some(formation) = &starter.formation
                && !leaders.insert(formation.commander)
            {
                return Err(Refusal::Invalid);
            }
        }
        Ok(())
    }
}

pub(super) fn catalog(catalog: &Catalog) -> Result<(), Refusal> {
    let run = catalog.is_run();
    let mut basics = ColorSet::EMPTY;
    for card in &catalog.cards {
        if card.run.is_some() != run {
            return Err(Refusal::Invalid);
        }
        if let Some(rules) = card.run {
            if !rules.identity.difference(ColorSet::ALL).is_empty() {
                return Err(Refusal::Invalid);
            }
            if card.basic_land {
                if rules.commander || rules.identity.len() != 1 || basics.intersects(rules.identity)
                {
                    return Err(Refusal::Invalid);
                }
                basics = basics.union(rules.identity);
            }
        }
    }
    if run && basics != ColorSet::ALL {
        return Err(Refusal::Invalid);
    }
    Ok(())
}

pub(super) fn validate(collection: &Collection, catalog: &Catalog) -> Result<(), Refusal> {
    let formation = collection.formation.as_ref().ok_or(Refusal::Invalid)?;
    if collection.owned.len() > super::MAX_CATALOG
        || collection.deck.len() > 39
        || formation.sideboard.len() > 5
    {
        return Err(Refusal::Capacity);
    }
    let mut owned = BTreeSet::new();
    for stack in &collection.owned {
        let card = catalog.card(stack.card).ok_or(Refusal::Unavailable)?;
        if card.basic_land || stack.copies != 1 || !owned.insert(stack.card) {
            return Err(Refusal::Invalid);
        }
    }
    let leader = catalog
        .card(formation.commander)
        .and_then(|card| card.run)
        .filter(|rules| rules.commander)
        .ok_or(Refusal::Unavailable)?;
    if !owned.contains(&formation.commander) {
        return Err(Refusal::Unavailable);
    }
    let mut used = BTreeSet::from([formation.commander]);
    let main = zone(
        &collection.deck,
        catalog,
        &owned,
        &mut used,
        leader.identity,
        39,
    )?;
    let side = zone(
        &formation.sideboard,
        catalog,
        &owned,
        &mut used,
        leader.identity,
        5,
    )?;
    if main != 39 || side > 5 {
        return Err(Refusal::Invalid);
    }
    Ok(())
}

fn zone(
    stacks: &[Stack],
    catalog: &Catalog,
    owned: &BTreeSet<CardIndex>,
    used: &mut BTreeSet<CardIndex>,
    identity: ColorSet,
    limit: u16,
) -> Result<u16, Refusal> {
    let mut total = 0;
    let mut rows = BTreeSet::new();
    for stack in stacks {
        let card = catalog.card(stack.card).ok_or(Refusal::Unavailable)?;
        let rules = card.run.ok_or(Refusal::Invalid)?;
        if stack.copies == 0
            || !rows.insert(stack.card)
            || !rules.identity.difference(identity).is_empty()
            || (!card.basic_land
                && (stack.copies != 1 || !owned.contains(&stack.card) || !used.insert(stack.card)))
        {
            return Err(Refusal::Invalid);
        }
        total += u16::from(stack.copies);
        if total > limit {
            return Err(Refusal::Capacity);
        }
    }
    Ok(total)
}

impl Collection {
    /// Atomically change all deck zones and the commander from the run's owned pool.
    /// # Errors
    /// Legacy catalog, unowned card, identity, singleton or 39+1 / sideboard violation.
    pub fn refitted(
        &self,
        catalog: &Catalog,
        deck: Vec<Stack>,
        mut formation: Formation,
    ) -> Result<Self, Refusal> {
        self.validate(catalog)?;
        if !catalog.is_run() || deck.len() > 39 || formation.sideboard.len() > 5 {
            return Err(Refusal::Invalid);
        }
        formation.sideboard.sort_by_key(|stack| stack.card);
        let mut next = Self {
            owned: self.owned.clone(),
            deck,
            formation: Some(formation),
        };
        next.validate(catalog)?;
        next.deck.sort_by_key(|stack| stack.card);
        Ok(next)
    }
}
