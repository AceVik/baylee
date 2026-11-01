//! `cards/creatures/mv_2/overtaker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "71668016-cefb-43d2-a84c-47956cffd71e"
fn overtaker() -> CardIndex {
    card_index("71668016-cefb-43d2-a84c-47956cffd71e")
}

/// `Overtaker` (`Coverage::Implemented`):
/// "{3}{U}, {T}, Discard a card: Untap target creature and gain control of it
/// until end of turn. That creature gains haste until end of turn."
#[test]
fn overtaker_untaps_gains_control_and_grants_haste_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9702, island())
        .battlefield(0, &[overtaker(), island(), island(), island(), island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let source = on_battlefield(&engine, p0, overtaker()).expect("Overtaker");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(bears)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();
    let discard_card = in_hand(&engine, p0, island()).expect("card to discard");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![discard_card],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, bears),
        "Overtaker untaps target creature"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        p0,
        "Overtaker gains control of target creature until end of turn"
    );
    assert!(
        keywords(&engine, bears).contains(KeywordSet::HASTE),
        "Overtaker grants target creature haste until end of turn"
    );
}
