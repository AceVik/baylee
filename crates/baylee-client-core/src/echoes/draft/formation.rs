use super::{CardIndex, Cohort, Command, Draft, Phase, Run, Stack};

/// Editable zones in a Commander run. The commander has a separate explicit choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeckZone {
    /// The thirty-nine-card draw library.
    #[default]
    Main,
    /// At most five run-owned cards outside the library.
    Sideboard,
}

impl Draft {
    /// Restore every zone, including a changed commander.
    pub fn reset_deck(&mut self, run: &Run) {
        self.deck.clone_from(&run.cards.deck);
        self.formation.clone_from(&run.cards.formation);
    }

    /// Submit the zones together, so a move cannot duplicate ownership between zones.
    #[must_use]
    pub fn save_deck(&self) -> Command {
        self.formation.as_ref().map_or_else(
            || Command::Edit {
                deck: self.deck.clone(),
            },
            |formation| Command::Refit {
                deck: self.deck.clone(),
                formation: formation.clone(),
            },
        )
    }

    /// Whether a zone edit is available, without constructing or submitting a command.
    #[must_use]
    pub fn can_card_in(
        &self,
        run: &Run,
        cohort: &Cohort,
        card: CardIndex,
        zone: DeckZone,
        add: bool,
    ) -> bool {
        if self.bound != Some((run.id, run.revision)) || run.phase != Phase::Shop {
            return false;
        }
        let Some(formation) = &self.formation else {
            return false;
        };
        let Some(entry) = cohort.catalog.card(card) else {
            return false;
        };
        if add {
            let Some(identity) = cohort
                .catalog
                .card(formation.commander)
                .and_then(|card| card.run)
            else {
                return false;
            };
            if entry
                .run
                .is_none_or(|rules| !rules.identity.difference(identity.identity).is_empty())
                || (!entry.basic_land
                    && (run.cards.copies(card) == 0
                        || formation.commander == card
                        || self
                            .deck
                            .iter()
                            .chain(&formation.sideboard)
                            .any(|row| row.card == card)))
            {
                return false;
            }
        }
        let rows = match zone {
            DeckZone::Main => &self.deck,
            DeckZone::Sideboard => &formation.sideboard,
        };
        if add {
            let limit = if zone == DeckZone::Main { 39 } else { 5 };
            rows.iter().map(|row| u16::from(row.copies)).sum::<u16>() < limit
        } else {
            rows.iter().any(|row| row.card == card)
        }
    }

    /// Change a run zone. Incomplete drafts are allowed; Save validates the whole formation.
    pub fn card_in(
        &mut self,
        run: &Run,
        cohort: &Cohort,
        card: CardIndex,
        zone: DeckZone,
        add: bool,
    ) {
        if !self.can_card_in(run, cohort, card, zone, add) {
            return;
        }
        let Some(formation) = self.formation.as_mut() else {
            return;
        };
        let rows = match zone {
            DeckZone::Main => &mut self.deck,
            DeckZone::Sideboard => &mut formation.sideboard,
        };
        let at = rows.iter().position(|row| row.card == card);
        if add {
            if let Some(at) = at {
                rows[at].copies += 1;
            } else {
                rows.push(Stack { card, copies: 1 });
            }
        } else if let Some(at) = at {
            if rows[at].copies == 1 {
                rows.remove(at);
            } else {
                rows[at].copies -= 1;
            }
        }
        rows.sort_by_key(|row| row.card);
    }

    /// Select an eligible owned leader; remove that copy from the other draft zones.
    /// Other off-color cards stay visible for the player to replace before Save.
    pub fn commander(&mut self, run: &Run, cohort: &Cohort, card: CardIndex) {
        if self.bound != Some((run.id, run.revision))
            || run.phase != Phase::Shop
            || run.cards.copies(card) == 0
            || cohort
                .catalog
                .card(card)
                .and_then(|card| card.run)
                .is_none_or(|rules| !rules.commander)
        {
            return;
        }
        if let Some(formation) = self.formation.as_mut() {
            formation.commander = card;
            formation.sideboard.retain(|row| row.card != card);
            self.deck.retain(|row| row.card != card);
        }
    }
}
