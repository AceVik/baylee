//! `cards/artifacts/mv_6/jandor_s_ring.rs`, played.
//!
//! Jandor's Ring ({6}, Artifact): "{2}, {T}, Discard the last card you drew
//! this turn: Draw a card."
//!
//! Written from the printed sentence, the two Scryfall rulings (2004-10-04:
//! "If you do not have the card still in your hand, you can't pay the cost";
//! "If you draw more than one card due to a spell or ability, you must
//! discard the last one of those drawn") and the Comprehensive Rules: a cost
//! is paid whole or not at all (CR 118.3), a card is drawn one at a time
//! (CR 121.2), and the player who goes first skips their first draw in a
//! two-player game (CR 103.8a). The Ring and the lands are seeded; the cards
//! are drawn by the game (the draw step, Ancestral Recall) and every
//! activation is played through `Engine::apply`. The cards that will be
//! drawn are put on top of the library out of the opening hand, so that
//! which card is "the last" is known by identity, not by luck.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "737db899-dcdb-48f9-8d30-cbbce3ae3434"
fn jandors_ring() -> CardIndex {
    card_index("737db899-dcdb-48f9-8d30-cbbce3ae3434")
}

// {U} instant: "Target player draws three cards."
fn ancestral_recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

// {1}{G} 2/2, no rules text.
fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Whether the seat holding priority is offered any ability of `ring`.
#[track_caller]
fn ring_is_offered(engine: &Engine<RegistryLookup>, ring: ObjectId) -> bool {
    !kit::offered(engine, ring).is_empty()
}

/// Activates the one ability the Ring is offered, paying `{2}` out of exactly
/// the two lands in `pay_with`, and lets it resolve.
#[track_caller]
fn activate_ring(engine: &mut Engine<RegistryLookup>, ring: ObjectId, pay_with: &[ObjectId]) {
    let p0 = kit::P0;
    let offered = kit::offered(engine, ring);
    assert_eq!(
        offered.len(),
        1,
        "the Ring prints one ability and it is on offer: {offered:?}"
    );
    tap_mana_where(engine, p0, |id| pay_with.contains(&id));
    kit::activate_object(engine, p0, ring, offered[0]);
    pass_until(engine, stack_is_empty);
}

/// A game where p0 has the Ring and `forests` Forests, and a Llanowar Elves
/// on top of the library, with a Grizzly Bears left in hand. The first
/// player skips the draw of their first turn (CR 103.8a), so the Elves is
/// drawn in p0's *second* turn, in the draw step.
fn ring_and_a_card_on_top(forests: usize) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = kit::P0;
    let mut board = vec![jandors_ring()];
    board.extend(std::iter::repeat_n(forest(), forests));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[llanowar_elves(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    let elves = kit::stack_library(&mut engine, p0, &[llanowar_elves()])[0];
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, jandors_ring()).expect("the Ring is out");
    (engine, ring, elves)
}

/// Ancestral Recall has been cast and has resolved: p0 has drawn, in this
/// order, a Grizzly Bears, a Sol Ring and a Llanowar Elves (so the Elves is
/// the *last* card drawn this turn), and has the Ring (`rings` of them),
/// four untapped Forests and nothing else. Returns the engine and the three
/// drawn objects, first drawn first.
fn after_ancestral_recall(rings: usize) -> (Engine<RegistryLookup>, [ObjectId; 3]) {
    let p0 = kit::P0;
    let mut board = vec![jandors_ring(); rings];
    board.push(island());
    board.extend(std::iter::repeat_n(forest(), 4));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(
            0,
            &[
                ancestral_recall(),
                grizzly_bears(),
                sol_ring(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let stacked = kit::stack_library(
        &mut engine,
        p0,
        &[grizzly_bears(), sol_ring(), llanowar_elves()],
    );
    reach_main_phase(&mut engine, p0);
    let islands = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| islands.contains(&id));
    cast_with_floating(&mut engine, p0, ancestral_recall());
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p0))
        .expect("\"target player\": the caster");
    pass_until(&mut engine, stack_is_empty);
    for (card, name) in stacked.iter().zip(["Grizzly Bears", "Sol Ring", "Elves"]) {
        assert_eq!(
            kit::zone_of(&engine, *card),
            Zone::Hand,
            "Ancestral Recall drew the {name}"
        );
    }
    (engine, [stacked[0], stacked[1], stacked[2]])
}

/// The sentence: "{2}, {T}, Discard the last card you drew this turn: Draw a
/// card." The control is the first turn, in which the starting player skips
/// their draw: with the lands untapped and the Ring untapped it is not on
/// offer, because nothing has been drawn this turn. In the second turn the
/// draw step draws the Elves, the Ring is on offer, and activating it discards
/// *that card* (the same object, and not the Bears that has been in hand
/// since the opening hand), draws another, taps the Ring and costs exactly
/// two mana.
#[test]
fn jandors_ring_discards_the_card_drawn_this_turn_and_draws_a_card() {
    let p0 = kit::P0;
    let (mut engine, ring, elves) = ring_and_a_card_on_top(4);
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 4, "four Forests are seated");

    // Turn 1: nothing drawn this turn, nothing to discard.
    assert_eq!(
        kit::zone_of(&engine, elves),
        Zone::Library,
        "the starting player skipped their draw: the Elves is still on top"
    );
    assert!(
        !ring_is_offered(&engine, ring),
        "no card drawn this turn: the Ring's cost cannot be paid, mana or no mana"
    );

    // Turn 3: the draw step draws the Elves.
    kit::walk_to_next_own_main(&mut engine, p0);
    assert_eq!(
        kit::zone_of(&engine, elves),
        Zone::Hand,
        "the draw step drew the Elves"
    );
    let bears = in_hand(&engine, p0, grizzly_bears()).expect("the Bears was never drawn this turn");
    assert!(
        ring_is_offered(&engine, ring),
        "a card drawn this turn: on offer"
    );
    let library_before = kit::library_of(&engine, p0).len();

    activate_ring(&mut engine, ring, &forests[..2]);

    assert_eq!(
        kit::zone_of(&engine, elves),
        Zone::Graveyard,
        "the card drawn this turn was the one discarded"
    );
    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Hand,
        "the card from the opening hand was not"
    );
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert_eq!(
        hand.len(),
        2,
        "one card out, one card in: the Bears and the new draw, got {hand:?}"
    );
    assert!(hand.contains(&bears) && !hand.contains(&elves));
    assert_eq!(
        kit::library_of(&engine, p0).len(),
        library_before - 1,
        "\"Draw a card\": one card came off the library"
    );
    assert!(is_tapped(&engine, ring), "{{T}}: the Ring is tapped");
    assert!(
        is_tapped(&engine, forests[0]) && is_tapped(&engine, forests[1]),
        "the two Forests paid {{2}}"
    );
    assert!(
        !is_tapped(&engine, forests[2]) && !is_tapped(&engine, forests[3]),
        "and only those two"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}} was paid and nothing was left over"
    );
    assert!(
        !ring_is_offered(&engine, ring),
        "tapped: it cannot be activated again this turn"
    );
}

/// "{2}": the generic cost is part of the sentence. With the card drawn and
/// the Ring untapped, a Ring with two Forests behind it is on offer and a
/// Ring with one Forest behind it is not.
#[test]
fn jandors_ring_needs_two_mana() {
    let p0 = kit::P0;
    for (forests, offered) in [(2, true), (1, false)] {
        let (mut engine, ring, elves) = ring_and_a_card_on_top(forests);
        kit::walk_to_next_own_main(&mut engine, p0);
        assert_eq!(kit::zone_of(&engine, elves), Zone::Hand, "drawn this turn");
        assert_eq!(
            ring_is_offered(&engine, ring),
            offered,
            "{forests} Forest(s) behind a Ring whose card was drawn this turn"
        );
    }
}

/// The ruling about several cards drawn at once: Ancestral Recall draws a
/// Bears, then a Sol Ring, then an Elves. "You must discard the last one of
/// those drawn": the Elves goes, and the two drawn before it stay in hand.
#[test]
fn jandors_ring_discards_the_last_of_several_cards_drawn_at_once() {
    let p0 = kit::P0;
    let (mut engine, [bears, sol, elves]) = after_ancestral_recall(1);
    let ring = on_battlefield(&engine, p0, jandors_ring()).expect("the Ring");
    let forests = all_on_battlefield(&engine, p0, forest());

    assert!(ring_is_offered(&engine, ring));
    activate_ring(&mut engine, ring, &forests[..2]);

    assert_eq!(
        kit::zone_of(&engine, elves),
        Zone::Graveyard,
        "the last of the three drawn is the one discarded"
    );
    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Hand,
        "the first drawn stays"
    );
    assert_eq!(
        kit::zone_of(&engine, sol),
        Zone::Hand,
        "the second drawn stays"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        3,
        "three drawn, one discarded, one drawn by the Ring"
    );
}

/// The ruling about a card that has left the hand: "If you do not have the
/// card still in your hand, you can't pay the cost." The Elves is the last
/// card drawn; the Bears and the Sol Ring, drawn earlier the same turn, are in
/// hand beside it. While the Elves is in hand the Ring is on offer (the
/// control, with the mana for it there). Once the Elves is cast the Ring is
/// *not* on offer, although two other cards drawn this turn are right there:
/// it is the last card or no card, not "the last one still in hand".
#[test]
fn jandors_ring_is_not_offered_once_the_last_card_drawn_has_left_the_hand() {
    let p0 = kit::P0;
    let (mut engine, [bears, sol, elves]) = after_ancestral_recall(1);
    let ring = on_battlefield(&engine, p0, jandors_ring()).expect("the Ring");
    let forests = all_on_battlefield(&engine, p0, forest());
    assert!(
        ring_is_offered(&engine, ring),
        "control: the last card drawn is in hand, and the mana is there"
    );

    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_ne!(
        kit::zone_of(&engine, elves),
        Zone::Hand,
        "the Elves has left the hand"
    );
    assert_eq!(kit::zone_of(&engine, bears), Zone::Hand);
    assert_eq!(kit::zone_of(&engine, sol), Zone::Hand);
    assert!(
        !is_tapped(&engine, forests[1])
            && !is_tapped(&engine, forests[2])
            && !is_tapped(&engine, forests[3]),
        "three Forests are still untapped: the mana for {{2}} is there"
    );
    assert!(
        !ring_is_offered(&engine, ring),
        "the last card drawn is gone from the hand, so the cost cannot be paid \
         with an earlier card drawn this turn"
    );
}

/// "The last card you drew this turn" moves when the Ring itself draws:
/// the card one Ring draws is the last card drawn, and a second Ring then
/// discards *that* card, not the one the first discarded and not an earlier
/// one. Two Rings, four Forests, the Recall's three cards.
#[test]
fn jandors_ring_second_ring_discards_the_card_the_first_ring_drew() {
    let p0 = kit::P0;
    let (mut engine, [bears, sol, elves]) = after_ancestral_recall(2);
    let rings = all_on_battlefield(&engine, p0, jandors_ring());
    assert_eq!(rings.len(), 2, "two Rings");
    let forests = all_on_battlefield(&engine, p0, forest());

    activate_ring(&mut engine, rings[0], &forests[..2]);
    assert_eq!(
        kit::zone_of(&engine, elves),
        Zone::Graveyard,
        "the first Ring: the Elves"
    );
    let drawn_by_the_first = in_hand(&engine, p0, forest()).expect("the first Ring drew a Forest");
    assert!(
        ring_is_offered(&engine, rings[1]),
        "the last card drawn is now the Forest the first Ring drew"
    );

    activate_ring(&mut engine, rings[1], &forests[2..]);
    assert_eq!(
        kit::zone_of(&engine, drawn_by_the_first),
        Zone::Graveyard,
        "the second Ring discards the card the first one drew"
    );
    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Hand,
        "an earlier draw stays"
    );
    assert_eq!(
        kit::zone_of(&engine, sol),
        Zone::Hand,
        "an earlier draw stays"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        3,
        "the Bears, the Sol Ring and what the second Ring drew"
    );
}

/// "The last card you drew *this turn*": a card drawn in an earlier turn
/// does not count. The Elves, drawn in p0's turn, is still in p0's hand
/// during the opponent's turn, with the Ring untapped and the Forests
/// untapped, and the Ring is not on offer then; in p0's own turn, with the
/// very same card, it was (the control).
#[test]
fn jandors_ring_does_not_reach_back_to_a_card_drawn_in_an_earlier_turn() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, ring, elves) = ring_and_a_card_on_top(4);
    kit::walk_to_next_own_main(&mut engine, p0);
    assert_eq!(kit::zone_of(&engine, elves), Zone::Hand, "drawn this turn");
    assert!(
        ring_is_offered(&engine, ring),
        "control: in the turn it was drawn, the Ring can discard it"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        kit::zone_of(&engine, elves),
        Zone::Hand,
        "the Elves is still in p0's hand"
    );
    assert!(!is_tapped(&engine, ring), "the Ring is untapped");
    for land in all_on_battlefield(&engine, p0, forest()) {
        assert!(!is_tapped(&engine, land), "and so are the Forests");
    }
    assert!(
        !ring_is_offered(&engine, ring),
        "p0 drew nothing in p1's turn: there is no last card drawn this turn"
    );
}
