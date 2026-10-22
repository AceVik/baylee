//! `cards/sorceries/mv_5/dregs_of_sorrow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dregs_of_sorrow() -> CardIndex {
    card_index("8126e60b-cf3d-4bf0-bfb5-f1e914d3e20e")
}

/// Dregs of Sorrow (`8126e60b-cf3d-4bf0-bfb5-f1e914d3e20e`) — {X}{4}{B} Sorcery:
/// "Destroy X target nonblack creatures. Draw X cards."
#[test]
fn dregs_of_sorrow_destroys_x_nonblack_creatures_and_draws_x_cards() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .hand(0, &[dregs_of_sorrow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    let spell = in_hand(&engine, P0, dregs_of_sorrow()).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .unwrap();
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_after_cast + 2
    );
}
