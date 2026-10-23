//! `cards/instants/mv_1/scapegoat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn scapegoat() -> CardIndex {
    card_index("04eb4fc0-0cc5-44f1-af5a-44579125d5c2")
}

/// Scapegoat (`04eb4fc0-0cc5-44f1-af5a-44579125d5c2`) — {W} Instant (Implemented):
/// "As an additional cost to cast this spell, sacrifice a creature. Return any number of target creatures you control to their owner's hand."
#[test]
fn scapegoat_sacrifices_a_creature_and_returns_target_creatures_to_hand() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                index::SAVANNAH_LIONS,
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
                plains(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[scapegoat()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let lions = on_battlefield(&engine, P0, index::SAVANNAH_LIONS).unwrap();
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, scapegoat());
    let enemy = lk::obj(&engine, lk::P1, index::GRIZZLY_BEARS);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![enemy],
                    players: vec![]
                }
            )
            .is_err()
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bear, giant],
                players: vec![],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![lions],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::SAVANNAH_LIONS).is_none());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P0, index::HILL_GIANT).is_none());
    let hand = engine.state().zones.list(ZoneLocation::Hand(P0));
    assert_eq!(hand.len(), 2);
    assert!(in_graveyard(&engine, P0, index::SAVANNAH_LIONS).is_some());
    assert!(on_battlefield(&engine, lk::P1, index::GRIZZLY_BEARS).is_some());
}
