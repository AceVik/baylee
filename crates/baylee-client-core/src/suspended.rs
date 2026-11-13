//! Which exiled cards are suspended, and how long they still wait (CR
//! 702.62): public knowledge, the same from every seat.
//!
//! What a player meets it as is on the table: the waiting place
//! ([`crate::layout::PileKind::Waiting`]) and the number beside it. Until
//! 13.11.2026 this was a list of sentences in a panel over the table's top
//! left corner.
use baylee_view::{CounterKind, PublicObject};

/// The time counters on `card`, where it is a suspended card every seat may
/// name: suspended, face up for this view, and with a counter left. Counters
/// and not turns: nothing here promises when the last one comes off.
///
/// The one test of "is this a waiting card" in the client: the table's
/// waiting place and the zone dialog's badge both ask here. A card whose
/// identity this view withholds is never one, whatever its flag says.
#[must_use]
pub fn time_of(card: &PublicObject) -> Option<u16> {
    let counters = card
        .counters
        .iter()
        .find(|c| c.kind == CounterKind::Time)?
        .count;
    (card.suspended && card.card.is_some() && counters > 0).then_some(counters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::printed;
    use baylee_view::CounterEntry;

    /// A suspended card says its counters; time counters on a card that is
    /// not suspended, a suspended card with none left and one whose identity
    /// is withheld say nothing.
    #[test]
    fn only_a_named_suspended_card_with_time_left_waits() {
        let time = |count| CounterEntry {
            kind: CounterKind::Time,
            count,
        };
        let mut card = printed(10, 0, "Ancestral Vision", 1);
        card.suspended = true;
        assert_eq!(time_of(&card), None, "no counter, nothing to wait for");
        card.counters.push(time(4));
        assert_eq!(time_of(&card), Some(4));
        card.counters[0].count = 1;
        assert_eq!(time_of(&card), Some(1), "the count follows the view");
        card.counters[0].count = 0;
        assert_eq!(time_of(&card), None, "the last counter is off");
        card.counters[0].count = 2;
        card.card = None;
        assert_eq!(
            time_of(&card),
            None,
            "an unknown identity never waits in public"
        );
        let mut ordinary = printed(20, 1, "Other", 1);
        ordinary.counters.push(time(2));
        assert_eq!(
            time_of(&ordinary),
            None,
            "time counters alone are not suspend"
        );
    }
}
