//! Exile drawn as two places: the pile, and the waiting place its suspended
//! cards lie on (CR 702.62). What lies where, in which order, and that the
//! zone still counts as one.

use super::*;
use crate::browser::BrowseZone;
use baylee_view::{CounterEntry, CounterKind};

fn suspended(id: u32, name: &str, time: u16) -> PublicObject {
    let mut card = printed(id, 0, name, 1);
    card.suspended = true;
    card.counters.push(CounterEntry {
        kind: CounterKind::Time,
        count: time,
    });
    card
}

fn pile(view: &PlayerView, seat: u8, kind: PileKind) -> Option<ZonePile> {
    zone_piles(view, PlayerId::new(seat))
        .into_iter()
        .find(|p| p.kind == kind)
}

/// A suspended card lies on the waiting place and every other exiled card
/// on the pile: each drawn once, and the two together are what the seat
/// plate and the zone dialog count as exile. The waiting place shows what
/// comes back next: the fewest time counters on top, the rest under it in
/// that order, each with its own count.
#[test]
fn a_suspended_card_waits_apart_and_still_counts_in_exile() {
    let mut hidden = suspended(15, "Unseen", 2);
    hidden.card = None;
    let exiled = vec![
        printed(10, 0, "Swords' prey", 1),
        suspended(11, "Greater Gargadon", 10),
        printed(12, 0, "Path's prey", 1),
        suspended(13, "Rift Bolt", 1),
        suspended(14, "Ancestral Vision", 4),
        hidden,
    ];
    let view = ViewBuilder::new(2).with_exile(0, exiled).build();

    let exile = pile(&view, 0, PileKind::Exile).expect("every seat has an exile pile");
    let waiting = pile(&view, 0, PileKind::Waiting).expect("three cards wait");
    assert_eq!(
        exile.count, 3,
        "two plain cards and the one nobody may name"
    );
    assert_eq!(waiting.count, 3);
    assert_eq!(
        waiting.name.as_deref(),
        Some("Rift Bolt"),
        "the card that comes back next lies on top"
    );
    assert_eq!(
        waiting
            .fan
            .iter()
            .map(|c| (c.name.as_str(), c.time))
            .collect::<Vec<_>>(),
        [
            ("Rift Bolt", Some(1)),
            ("Ancestral Vision", Some(4)),
            ("Greater Gargadon", Some(10)),
        ],
        "soonest first, each with its own time counters"
    );
    assert!(
        exile.fan.iter().all(|c| c.time.is_none()),
        "a card on the pile waits for nothing"
    );
    // One object, one place.
    for card in &waiting.fan {
        assert!(
            exile
                .fan
                .iter()
                .chain(&exile.rest)
                .all(|c| c.object != card.object),
            "{} lies on both places",
            card.name
        );
    }
    // And one zone: the board's two places, the seat plate and the zone
    // dialog's tab count the same cards.
    let zone = BrowseZone::Exile(PlayerId::new(0)).count_in(&view);
    assert_eq!(zone, 6);
    assert_eq!((exile.count + waiting.count) as usize, zone);
    assert_eq!(
        BrowseZone::of_pile(PileKind::Waiting, PlayerId::new(0)),
        Some(BrowseZone::Exile(PlayerId::new(0))),
        "the waiting place opens the zone its cards are in"
    );

    // A seat with nothing suspended has no waiting place at all.
    assert!(pile(&view, 1, PileKind::Waiting).is_none());
}

/// The last time counter coming off empties the place, and the place goes:
/// the card is on the pile again until the engine moves it.
#[test]
fn a_card_with_no_time_left_no_longer_waits() {
    let mut view = ViewBuilder::new(2)
        .with_exile(0, vec![suspended(11, "Rift Bolt", 1)])
        .build();
    assert_eq!(pile(&view, 0, PileKind::Waiting).map(|p| p.count), Some(1));
    assert_eq!(pile(&view, 0, PileKind::Exile).map(|p| p.count), Some(0));
    view.exile[0][0].counters[0].count = 0;
    assert!(pile(&view, 0, PileKind::Waiting).is_none());
    assert_eq!(pile(&view, 0, PileKind::Exile).map(|p| p.count), Some(1));
}
