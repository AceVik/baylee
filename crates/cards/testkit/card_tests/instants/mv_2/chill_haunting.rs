//! `cards/instants/mv_2/chill_haunting.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "ebe61471-e236-4ff8-9416-151970b5efd6"
fn chill_haunting() -> CardIndex {
    card_index("ebe61471-e236-4ff8-9416-151970b5efd6")
}

/// Chill Haunting ({1}{B}): X is bounded by CREATURE cards in your graveyard
/// (ignoring noncreature cards there); exiling X creature cards gives target
/// creature -X/-X until end of turn, then wears off next turn.
#[test]
fn chill_haunting_exiles_x_creature_cards_from_gy_and_shrinks_by_x_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::SERRA_ANGEL],
        &[
            chill_haunting(),
            index::GRIZZLY_BEARS,
            index::SAVANNAH_LIONS,
            shock(),
        ],
        &[],
    );
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    // Move 2 creature cards and 1 noncreature card into P0's graveyard.
    for card in [index::GRIZZLY_BEARS, index::SAVANNAH_LIONS, shock()] {
        let id = in_hand(&engine, P0, card).unwrap();
        engine
            .dev_state_mut(P0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(P0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
    let gy_shock = in_graveyard(&engine, P0, shock()).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, chill_haunting());
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseNumber { max, .. } => {
                assert_eq!(max, 2, "only 2 creature cards in GY (Shock excluded)");
                engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                lk::aim(&mut engine, &[angel], &[]);
            }
            Pending::ChooseCards { options, .. } => {
                assert!(
                    !options.contains(&gy_shock),
                    "noncreature Shock cannot pay Chill Haunting's cost: {options:?}"
                );
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    lk::settle(&mut engine);

    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P0)).len(), 2);
    assert_eq!(pt(&engine, angel), (2, 2), "4/4 Angel gets -2/-2 until EOT");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, angel), (4, 4), "-2/-2 wears off at end of turn");
}
