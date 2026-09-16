//! What a hover spreads out of a pile, and what it may never spread out
//! of a library.

use super::*;

fn pile(view: &PlayerView, kind: PileKind) -> ZonePile {
    zone_piles(view, PlayerId::new(0))
        .into_iter()
        .find(|p| p.kind == kind)
        .expect("the seat has this pile")
}

/// Top of the pile first, and never more than seven — a graveyard of
/// ten fans its last seven, newest first.
///
/// `ZonePosition::Top` pushes, so the object listed *last* is the one
/// lying on top; the fan reverses that, which is the whole of the
/// ordering claim. It is asserted against the names rather than
/// against a length, because a fan that took the first seven would
/// also be seven cards long and would be the wrong seven.
#[test]
fn a_graveyard_fans_its_newest_seven_newest_first() {
    let dead: Vec<_> = (0..10)
        .map(|i| printed(i, 0, &format!("card {i}"), u16::try_from(i).unwrap() + 1))
        .collect();
    let view = ViewBuilder::new(2).with_graveyard(0, dead).build();
    let graveyard = pile(&view, PileKind::Graveyard);

    assert_eq!(graveyard.count, 10);
    assert_eq!(graveyard.fan_len(), ZonePile::FAN_MAX);
    assert_eq!(
        graveyard
            .fan
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        [
            "card 9", "card 8", "card 7", "card 6", "card 5", "card 4", "card 3"
        ],
        "the fan is not the newest seven, newest first"
    );
    assert_eq!(
        graveyard.fan.first().map(|c| c.object),
        graveyard.top,
        "the card on top of the pile is the card at the top of the fan"
    );
}

/// A pile shallower than the fan fans what it has, and an empty one
/// fans nothing at all.
#[test]
fn a_short_pile_fans_what_it_has() {
    let view = ViewBuilder::new(2)
        .with_exile(0, vec![printed(1, 0, "Oblivion Ring", 4)])
        .build();
    let exile = pile(&view, PileKind::Exile);
    assert_eq!(exile.fan_len(), 1);
    assert_eq!(exile.fan.len(), 1);

    let empty = ZonePile::empty(PileKind::Graveyard);
    assert_eq!(empty.fan_len(), 0);
    assert!(empty.fan.is_empty());
}

/// The second reading of CR 401.2, and the one this model enforces by
/// construction: a library fans, and has nothing to fan.
///
/// [`ZonePile::fan`] is empty for a library not because a rule here
/// empties it but because a `PlayerView` carries a library as a
/// *count* — there are no cards in it to put in the list. What the
/// fan draws there is [`ZonePile::fan_len`] card backs, which say how
/// deep the pile is and nothing else. The counter-test is the
/// graveyard above: same code, same seat, seven faces.
#[test]
fn a_library_fans_backs_and_never_faces() {
    let view = ViewBuilder::new(2).build();
    let library = pile(&view, PileKind::Library);

    assert_eq!(library.count, 80, "the builder deals a full library");
    assert_eq!(
        library.fan_len(),
        ZonePile::FAN_MAX,
        "a library fans like any other pile"
    );
    assert!(
        library.fan.is_empty(),
        "a library handed the fan a face to draw"
    );
    assert!(library.art.is_none() && library.top.is_none());
}

/// A token in a graveyard is a slot in the fan with no picture, not a
/// card the fan skips.
///
/// It is really lying there — a token that has left the battlefield
/// ceases to exist only when state-based actions are next checked
/// (CR 111.7) — and a fan that dropped it would say the pile is
/// shallower than it is, on exactly the frame a player is looking to
/// see what just died.
#[test]
fn a_token_in_the_graveyard_is_a_blank_slot_and_not_a_gap() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![printed(1, 0, "Llanowar Elves", 3), token(2, 0, "Elf", 1, 1)],
        )
        .build();
    let graveyard = pile(&view, PileKind::Graveyard);

    assert_eq!(graveyard.fan.len(), 2, "the token was dropped from the fan");
    assert_eq!(graveyard.fan[0].name, "Elf");
    assert!(graveyard.fan[0].art.is_none(), "a token has no printing");
    assert!(graveyard.fan[1].art.is_some());
}

/// A hover opens the pile the card is in, at the seat it belongs to.
///
/// A graveyard is public, so an opponent's opens like anyone's — and
/// it has to open as *theirs*, because the fan stands beside their
/// mat and not beside the viewer's.
#[test]
fn a_hover_opens_the_pile_the_card_is_lying_in() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![printed(1, 0, "buried", 1), printed(3, 0, "on top", 3)],
        )
        .with_exile(1, vec![printed(2, 1, "theirs", 2)])
        .build();
    let model = model(&view);
    let card_in = |seat: u8, kind, at: usize| {
        model
            .pod(PlayerId::new(seat))
            .expect("the seat is at the table")
            .piles
            .iter()
            .find(|p| p.kind == kind)
            .expect("the seat has this pile")
            .fan[at]
            .object
    };

    assert_eq!(
        model.fanned_pile(Some(card_in(0, PileKind::Graveyard, 0))),
        Some((PlayerId::new(0), PileKind::Graveyard))
    );
    // And the card *under* that one, which is what the pointer is
    // over once the fan is out — and what a reading that knew only
    // the pile's top card would answer nothing for.
    assert_eq!(
        model.fanned_pile(Some(card_in(0, PileKind::Graveyard, 1))),
        Some((PlayerId::new(0), PileKind::Graveyard)),
        "the fan shut under a pointer that had travelled along it"
    );
    assert_eq!(
        model.fanned_pile(Some(card_in(1, PileKind::Exile, 0))),
        Some((PlayerId::new(1), PileKind::Exile)),
        "an opponent's exile opened as somebody else's pile"
    );
    assert_eq!(model.fanned_pile(None), None);
    assert_eq!(
        model.fanned_pile(Some(ObjectId::new(99, 0))),
        None,
        "a card lying on nothing opened a pile"
    );
}

/// Every face the fan will draw is resident before the hover, and
/// each is asked for once.
///
/// A fan is a hover and a hover has no frame to spare for a fetch,
/// which is the same reason the pile's own top card is in this list.
/// The dedup is the second half: the top card is in the fan *and* in
/// `ZonePile::art`, so a list that did not dedup would ask for it
/// twice.
#[test]
fn the_whole_fan_is_resident_before_the_hover() {
    let dead: Vec<_> = (0..3)
        .map(|i| printed(i, 0, &format!("card {i}"), u16::try_from(i).unwrap() + 1))
        .collect();
    let view = ViewBuilder::new(2).with_graveyard(0, dead).build();
    let keys = model(&view).required_images();
    let graveyard = pile(&view, PileKind::Graveyard);

    for card in &graveyard.fan {
        let key = card.art.expect("every one of these is a printed card");
        assert_eq!(
            keys.iter().filter(|k| **k == key).count(),
            1,
            "{} is not asked for exactly once",
            card.name
        );
    }
    assert_eq!(keys.len(), 3);
}
