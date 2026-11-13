//! Which decks a deal of `eval4` seats where: one reading for the evaluator,
//! its research attempt ledger and `eval4 --write-deals`, so a list of deals
//! chosen by deck is the list the run plays.
//!
//! Deal `d` is table `d / seats` with the net in seat `d % seats`; the table's
//! seed is the first seed plus the table. Every seat of a table is the net's
//! in one deal, so a table's decks are each piloted once.
//!
//! [`DeckSchedule::Fixed`] seats deck `(table · seats + seat) mod n`: each deck
//! meets the same neighbours on every table it plays, and with two seats and
//! an even `n` a deck only ever meets one opponent. [`DeckSchedule::RoundRobin`]
//! (two seats) moves the opponent on by one deck every `n` tables, so over
//! `n − 1` cycles every deck meets every other once in each seat.

use anyhow::{Result, ensure};

/// How the decks of a run are seated at its tables.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "play", derive(clap::ValueEnum))]
pub enum DeckSchedule {
    /// Deck `(table · seats + seat) mod n`, as every run before the schedule
    /// had a name.
    #[default]
    Fixed,
    /// Two seats: in cycle `c = table / n` the deck `table mod n` meets the
    /// deck `c mod (n − 1) + 1` places on.
    RoundRobin,
}

impl DeckSchedule {
    /// The schedule's name, as `run.json` and the summary carry it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::RoundRobin => "round-robin",
        }
    }

    /// Whether a table of `seats` from `decks` decks can be seated.
    ///
    /// # Errors
    /// Fewer decks than seats, or a round robin of other than two seats.
    pub fn check(self, seats: usize, decks: usize) -> Result<()> {
        ensure!(
            seats >= 1 && decks >= seats,
            "{decks} decks cannot seat {seats}"
        );
        if self == Self::RoundRobin {
            ensure!(seats == 2, "a round robin seats two");
        }
        Ok(())
    }

    /// The deck of each seat at `table`, as indices into the run's decks.
    ///
    /// # Panics
    /// When [`Self::check`] refuses `seats` and `decks`.
    #[must_use]
    pub fn table_decks(self, table: usize, seats: usize, decks: usize) -> Vec<usize> {
        match self {
            Self::Fixed => (0..seats)
                .map(|seat| (table * seats + seat) % decks)
                .collect(),
            Self::RoundRobin => {
                assert!(seats == 2 && decks >= 2, "a round robin seats two");
                let first = table % decks;
                let step = (table / decks) % (decks - 1) + 1;
                vec![first, (first + step) % decks]
            }
        }
    }
}

/// Where deal `deal` sits: its table and the net's seat.
#[must_use]
pub const fn deal_seat(deal: u64, seats: usize) -> (usize, usize) {
    let deal = deal as usize;
    (deal / seats, deal % seats)
}

/// One deal as a run plays it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deal {
    /// The deal's number.
    pub deal: u64,
    /// Its table: the seed is the first seed plus this.
    pub table: usize,
    /// The net's seat.
    pub focal_seat: usize,
    /// Indices into the run's decks, one per seat.
    pub decks: Vec<usize>,
}

impl Deal {
    /// Deal `deal` of a run of `decks` decks at tables of `seats`.
    #[must_use]
    pub fn of(schedule: DeckSchedule, deal: u64, seats: usize, decks: usize) -> Self {
        let (table, focal_seat) = deal_seat(deal, seats);
        Self {
            deal,
            table,
            focal_seat,
            decks: schedule.table_decks(table, seats, decks),
        }
    }

    /// The deck the net pilots.
    #[must_use]
    pub fn focal_deck(&self) -> usize {
        self.decks[self.focal_seat]
    }
}

/// The deals in `first..first + count` whose net pilots a deck `pilots` keeps.
///
/// # Errors
/// As [`DeckSchedule::check`], and on a range that overflows.
pub fn deals_piloting(
    schedule: DeckSchedule,
    seats: usize,
    decks: usize,
    first: u64,
    count: u64,
    pilots: impl Fn(usize) -> bool,
) -> Result<Vec<Deal>> {
    schedule.check(seats, decks)?;
    let end = first
        .checked_add(count)
        .ok_or_else(|| anyhow::anyhow!("deal offset plus count overflows"))?;
    Ok((first..end)
        .map(|deal| Deal::of(schedule, deal, seats, decks))
        .filter(|deal| pilots(deal.focal_deck()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// The fixed schedule is the evaluator's arithmetic from before it had
    /// a name: deck `(table · seats + seat) mod n`.
    #[test]
    fn the_fixed_schedule_is_the_old_arithmetic() {
        for seats in 2..=4 {
            for deal in 0..200_u64 {
                let deal_row = Deal::of(DeckSchedule::Fixed, deal, seats, 78);
                let table = deal as usize / seats;
                assert_eq!(deal_row.table, table);
                assert_eq!(deal_row.focal_seat, deal as usize % seats);
                let old: Vec<usize> = (0..seats).map(|seat| (table * seats + seat) % 78).collect();
                assert_eq!(deal_row.decks, old);
            }
        }
    }

    /// Why the round robin exists: with two seats and an even number of
    /// decks, the fixed schedule gives a deck one opponent, whatever the seed.
    #[test]
    fn a_fixed_pair_of_seats_meets_one_opponent() {
        let opponents: std::collections::BTreeSet<usize> =
            deals_piloting(DeckSchedule::Fixed, 2, 26, 0, 26 * 200, |deck| deck == 7)
                .unwrap()
                .iter()
                .map(|deal| deal.decks[1 - deal.focal_seat])
                .collect();
        assert_eq!(opponents.len(), 1);
    }

    /// Over `n − 1` cycles of `n` tables every deck pilots against every
    /// other deck once from each seat, and never against itself.
    #[test]
    fn a_round_robin_meets_every_opponent_once_per_seat() {
        let decks = 26;
        let round = (2 * decks * (decks - 1)) as u64;
        for pilot in [0, 7, 25] {
            let deals = deals_piloting(DeckSchedule::RoundRobin, 2, decks, 0, round, |deck| {
                deck == pilot
            })
            .unwrap();
            assert_eq!(deals.len(), 2 * (decks - 1));
            let mut met: BTreeMap<(usize, usize), usize> = BTreeMap::new();
            for deal in &deals {
                let opponent = deal.decks[1 - deal.focal_seat];
                assert_ne!(opponent, pilot);
                *met.entry((deal.focal_seat, opponent)).or_default() += 1;
            }
            assert_eq!(met.len(), 2 * (decks - 1));
            assert!(met.values().all(|count| *count == 1));
        }
    }

    #[test]
    fn an_offset_window_keeps_the_deal_numbers() {
        let deals =
            deals_piloting(DeckSchedule::RoundRobin, 2, 5, 100, 40, |deck| deck == 3).unwrap();
        assert!(deals.iter().all(|deal| (100..140).contains(&deal.deal)));
        assert!(deals.iter().all(|deal| deal.focal_deck() == 3));
        assert!(!deals.is_empty());
    }

    #[test]
    fn a_round_robin_of_four_seats_is_refused() {
        assert!(DeckSchedule::RoundRobin.check(4, 26).is_err());
        assert!(DeckSchedule::Fixed.check(4, 26).is_ok());
        assert!(DeckSchedule::Fixed.check(4, 3).is_err());
    }
}
