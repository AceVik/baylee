//! `cards/instants/mv_1/demonic_consultation.rs`, played.
//!
//! Oracle:
//! - "Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way."
//!
//! The card is Partial: it reveals only the top card, which goes to the hand
//! when it has the chosen name and to the graveyard when it does not. These
//! tests play that, both ways.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "9a1412db-45ad-46ea-8f12-a85d203113d8"
fn demonic_consultation() -> CardIndex {
    card_index("9a1412db-45ad-46ea-8f12-a85d203113d8")
}

fn zone_size(engine: &Engine<RegistryLookup>, zone: ZoneLocation) -> usize {
    engine.state().zones.list(zone).len()
}

/// A library of Swamps; casts Consultation naming `named` and resolves it.
fn consult(named: CardIndex) -> (Engine<RegistryLookup>, usize, usize) {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .hand(0, &[demonic_consultation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand = zone_size(&engine, ZoneLocation::Hand(P0));
    let library = zone_size(&engine, ZoneLocation::Library(P0));
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, P0, demonic_consultation());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: named,
                face: 0,
            },
        )
        .expect("any card name may be chosen");
    pass_until(&mut engine, stack_is_empty);
    (engine, hand, library)
}

#[test]
fn demonic_consultation_names_a_card_and_puts_matching_top_card_into_hand() {
    let (engine, hand, library) = consult(index::SWAMP);

    assert!(
        in_hand(&engine, P0, index::SWAMP).is_some(),
        "the top card had the chosen name: it is put into your hand"
    );
    assert_eq!(
        zone_size(&engine, ZoneLocation::Hand(P0)),
        hand,
        "Consultation left the hand and the Swamp came in"
    );
    assert_eq!(zone_size(&engine, ZoneLocation::Library(P0)), library - 1);
}

#[test]
fn demonic_consultation_puts_a_top_card_without_the_name_into_the_graveyard() {
    let (engine, hand, library) = consult(index::LIGHTNING_BOLT);

    assert!(
        in_hand(&engine, P0, index::SWAMP).is_none(),
        "a card without the chosen name is not put into your hand"
    );
    assert_eq!(zone_size(&engine, ZoneLocation::Hand(P0)), hand - 1);
    assert!(
        in_graveyard(&engine, P0, index::SWAMP).is_some(),
        "the revealed Swamp goes to the graveyard (the Partial's stand-in for exile)"
    );
    assert_eq!(zone_size(&engine, ZoneLocation::Library(P0)), library - 1);
}
