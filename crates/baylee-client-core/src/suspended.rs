//! Public cards waiting on suspend, independent of the camera's current seat.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::{CounterKind, PlayerView};

/// One face-up suspended card and its remaining time counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuspendedCard {
    /// Card to inspect.
    pub object: ObjectId,
    /// Whose exile pile contains it.
    pub owner: PlayerId,
    /// Remaining time counters; this is not a promise about the number of turns.
    pub counters: u16,
}

/// All seats' suspended cards, in seat and exile order.
#[must_use]
pub fn read(view: &PlayerView) -> Vec<SuspendedCard> {
    view.exile
        .iter()
        .flatten()
        .filter_map(|card| {
            let counters = card
                .counters
                .iter()
                .find(|c| c.kind == CounterKind::Time)?
                .count;
            (card.suspended && card.card.is_some() && counters > 0).then_some(SuspendedCard {
                object: card.id,
                owner: card.owner,
                counters,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, printed};
    use baylee_view::CounterEntry;

    #[test]
    fn every_seat_sees_the_queue_and_counter_updates_but_not_other_exiled_cards() {
        let mut view = ViewBuilder::new(3).build();
        for seat in 0..3 {
            let mut card = printed(10 + seat, seat as u8, "Ancestral Vision", 1);
            card.suspended = true;
            card.counters.push(CounterEntry {
                kind: CounterKind::Time,
                count: 4,
            });
            view.exile[seat as usize].push(card);
        }
        let mut ordinary = printed(20, 1, "Other", 1);
        ordinary.counters.push(CounterEntry {
            kind: CounterKind::Time,
            count: 2,
        });
        view.exile[1].push(ordinary);
        for seat in 0..3 {
            view.seat = PlayerId::new(seat);
            let rows = read(&view);
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[2].owner, PlayerId::new(2));
        }
        view.exile[1][0].counters[0].count = 1;
        assert_eq!(read(&view)[1].counters, 1);
        view.exile[1][0].counters[0].count = 0;
        assert_eq!(read(&view).len(), 2);
        view.exile[2][0].card = None;
        assert_eq!(
            read(&view).len(),
            1,
            "unknown identities never become queue entries"
        );
        view.exile[0].clear();
        assert!(read(&view).is_empty());
    }
}
