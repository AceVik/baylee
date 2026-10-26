//! `cards/creatures/mv_2/cackling_witch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "ee608650-410a-4d6b-be6e-63d7955cff39"
fn cackling_witch() -> CardIndex {
    card_index("ee608650-410a-4d6b-be6e-63d7955cff39")
}

/// Cackling Witch ({1}{B} 1/1 Creature — Human Spellshaper):
/// "{X}{B}, {T}, Discard a card: Target creature gets +X/+0 until end of turn."
#[test]
fn cackling_witch_pumps_target_creature_power_by_x_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[cackling_witch(), index::GRIZZLY_BEARS],
        &[],
        &[index::SWAMP],
        &[],
    );
    let witch = lk::obj(&engine, P0, cackling_witch());
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let fodder = in_hand(&engine, P0, index::SWAMP).expect("card to discard in hand");

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Black, 1)],
    );
    activate(&mut engine, P0, cackling_witch(), 0);
    lk::drive(&mut engine, &[bears, fodder], true, 3);
    lk::settle(&mut engine);

    assert!(lk::tapped(&engine, witch));
    assert!(
        in_graveyard(&engine, P0, index::SWAMP).is_some(),
        "Cackling Witch discarded a card as part of the activation cost"
    );
    assert_eq!(
        pt(&engine, bears),
        (5, 2),
        "Cackling Witch gave Grizzly Bears +3/+0 until end of turn"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "+X/+0 wore off at end of turn");
}
