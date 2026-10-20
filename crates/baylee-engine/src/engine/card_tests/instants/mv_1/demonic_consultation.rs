//! `cards/instants/mv_1/demonic_consultation.rs`, played.
//!
//! Oracle:
//! - "Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "9a1412db-45ad-46ea-8f12-a85d203113d8"
fn demonic_consultation() -> CardIndex {
    card_index("9a1412db-45ad-46ea-8f12-a85d203113d8")
}

#[test]
fn demonic_consultation_names_a_card_and_puts_matching_top_card_into_hand() {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .hand(0, &[demonic_consultation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let before_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, P0, demonic_consultation());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: index::SWAMP,
                face: 0,
            },
        )
        .expect("choose Swamp as the card name");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before_hand,
        "Demonic Consultation left hand and put the matching top Swamp into hand"
    );
}
