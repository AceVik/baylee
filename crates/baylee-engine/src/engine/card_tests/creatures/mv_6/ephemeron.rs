//! `cards/creatures/mv_6/ephemeron.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn ephemeron() -> CardIndex {
    card_index("e4f28144-71c8-4393-b383-047529fe8878")
}

/// Ephemeron (`e4f28144-71c8-4393-b383-047529fe8878`) — {4}{U}{U} Creature — Illusion (Implemented):
/// "Flying. Discard a card: Return this creature to its owner's hand."
#[test]
fn ephemeron_discards_a_card_to_return_itself_to_owners_hand() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[ephemeron()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let eph = on_battlefield(&engine, P0, ephemeron()).unwrap();
    let card = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    kit::activate_object(&mut engine, P0, eph, 0);
    kit::settle(&mut engine, P0, |_, p| kit::choose_cards(p, &[card]));

    assert!(on_battlefield(&engine, P0, ephemeron()).is_none());
    assert!(in_hand(&engine, P0, ephemeron()).is_some());
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
