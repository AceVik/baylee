//! A session-local Commander collection, using the ordinary editor and rules.
//! A limited editor never creates a library save request. Its caller owns the
//! authoritative transaction and must validate the exported composition again.
use super::{DeckBuilder, Zone};
use baylee_cardtext::{Problem, Report};
use baylee_core::{deckrow::PrintChoice, ids::CardIndex};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// Copies and presentation available in this session, across both deck zones.
#[derive(Clone, Debug)]
pub struct Holding {
    /// Combined limit; basic lands may use `u16::MAX`.
    pub copies: u16,
    /// Frozen presentation, unchanged by edits and moves.
    pub print: PrintChoice,
}

/// Curated Commander geometry and collection. Ordinary format bans do not apply.
#[derive(Clone, Debug)]
pub struct Commander {
    /// Exact main-deck size, including its single commander.
    pub total: u16,
    /// Maximum sideboard size.
    pub side: u16,
    /// Owned identities only, with unlimited basics explicitly included.
    pub stock: BTreeMap<u32, Holding>,
}

impl Commander {
    pub(super) fn check(
        &self,
        main: &[CardIndex],
        side: &[CardIndex],
        leaders: &[CardIndex],
    ) -> Report {
        // The shared rules check every identity and copy over both lists. Only
        // the custom geometry differs from ordinary 100-card Commander.
        let combined: Vec<_> = main.iter().chain(side).copied().collect();
        let mut problems = baylee_cards::formats::check(
            baylee_cards::formats::Shape::Commander,
            &combined,
            leaders,
        );
        problems.retain(|problem| {
            !matches!(problem, Problem::WrongSize { .. } | Problem::Leaders { .. })
        });
        if main.len() + leaders.len() != usize::from(self.total) {
            problems.push(Problem::WrongSize {
                have: u32::try_from(main.len() + leaders.len()).unwrap_or(u32::MAX),
                need: u32::from(self.total),
            });
        }
        if side.len() > usize::from(self.side) {
            problems.push(Problem::SideboardTooBig {
                have: u32::try_from(side.len()).unwrap_or(u32::MAX),
                limit: u32::from(self.side),
            });
        }
        if leaders.len() != 1 {
            problems.push(Problem::Leaders {
                have: u32::try_from(leaders.len()).unwrap_or(u32::MAX),
            });
        }
        let mut held = BTreeMap::<u32, u32>::new();
        for card in combined.iter().chain(leaders) {
            *held.entry(card.get()).or_default() += 1;
        }
        for (card, have) in held {
            let limit = self
                .stock
                .get(&card)
                .map_or(0, |holding| u32::from(holding.copies));
            if have > limit {
                problems.push(Problem::TooManyCopies { card, have, limit });
            }
        }
        Report {
            problems,
            ..Report::default()
        }
    }
}

impl DeckBuilder {
    /// Restricts a populated editor to an isolated collection and its printings.
    pub fn limit_to(&mut self, limits: Commander) {
        for entry in self.main.iter_mut().chain(&mut self.side) {
            if let Some(card) = self.pool.get(entry.slot)
                && let Some(holding) = limits.stock.get(&card.index)
            {
                entry.print = holding.print.clone();
            }
        }
        self.limited = Some(limits);
        self.recheck();
    }

    /// Whether the caller, rather than the normal deck library, owns this session.
    #[must_use]
    pub const fn limited(&self) -> Option<&Commander> {
        self.limited.as_ref()
    }

    pub(super) fn limited_add(&self, slot: usize, zone: Zone) -> bool {
        let Some(limits) = &self.limited else {
            return true;
        };
        let Some(card) = self.pool.get(slot) else {
            return false;
        };
        let Some(holding) = limits.stock.get(&card.index) else {
            return false;
        };
        self.limited_room(zone)
            && u32::from(self.count_of(slot, Zone::Main))
                + u32::from(self.count_of(slot, Zone::Side))
                < u32::from(holding.copies)
    }

    pub(super) fn limited_room(&self, zone: Zone) -> bool {
        let Some(limits) = &self.limited else {
            return true;
        };
        let counts = self.counts();
        let (have, limit) = match zone {
            Zone::Main => (counts.main, limits.total),
            Zone::Side => (counts.side, limits.side),
        };
        have < u32::from(limit)
    }

    /// Export the same identities the shared legality overview checks. A leader
    /// is removed from the main list exactly once, never duplicated in the game.
    #[must_use]
    pub fn composition(&self) -> (Vec<CardIndex>, Vec<CardIndex>, Vec<CardIndex>) {
        self.cards()
    }
}
