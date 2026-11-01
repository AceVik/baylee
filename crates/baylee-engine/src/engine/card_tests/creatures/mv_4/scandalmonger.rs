//! `cards/creatures/mv_4/scandalmonger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "2d98d8d8-e277-42c4-bf94-dd7d7f2047c3"
fn scandalmonger() -> CardIndex {
    card_index("2d98d8d8-e277-42c4-bf94-dd7d7f2047c3")
}

/// `Scandalmonger` (`Coverage::Generated`):
/// "{2}: Target player discards a card. Any player may activate this ability but only as a sorcery."
///
/// Verifies that any player may activate `Scandalmonger` during their own main
/// phase with an empty stack to make target player discard a card.
#[test]
fn scandalmonger_makes_target_player_discard_and_can_be_activated_by_any_player_as_sorcery() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9616, swamp())
        .battlefield(0, &[scandalmonger()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(0, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    let monger = on_battlefield(&engine, p0, scandalmonger()).expect("P0 Scandalmonger");
    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(p0_hand_before, 2);

    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: monger,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "Scandalmonger causes target player to discard a card"
    );
}
