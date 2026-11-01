//! `cards/sorceries/mv_2/mob_justice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mob_justice() -> CardIndex {
    card_index("0ff97869-f27c-4379-92fd-605062aa5418")
}

/// Mob Justice (`0ff97869-f27c-4379-92fd-605062aa5418`) — {1}{R} Sorcery (Implemented):
/// "Mob Justice deals damage to target player or planeswalker equal to the number of creatures you control."
#[test]
fn mob_justice_deals_damage_equal_to_creatures_you_control_to_target_player() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, forest()])
        .hand(0, &[mob_justice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mob_justice());
    let enemy = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![enemy],
                    players: vec![]
                }
            )
            .is_err(),
        "creatures are not legal targets"
    );
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
}
