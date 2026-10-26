//! `cards/sorceries/mv_5/sever_soul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "577d027a-96c3-46fe-880b-f8b5dd3f3a3d"
fn sever_soul() -> CardIndex {
    card_index("577d027a-96c3-46fe-880b-f8b5dd3f3a3d")
}

/// `Sever Soul` (`Coverage::Generated`):
/// "Destroy target nonblack creature. It can't be regenerated. You gain life equal to its toughness."
///
/// Verifies that `Sever Soul` destroys target nonblack creature and its caster
/// gains life equal to the destroyed creature's toughness (using LKI).
#[test]
fn sever_soul_destroys_target_nonblack_creature_and_gains_life_equal_to_its_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9605, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[sever_soul()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).expect("P1 Hill Giant (3/3)");
    assert_eq!(engine.state().players[0].life, 20);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, sever_soul());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, index::HILL_GIANT).is_none(),
        "Hill Giant was destroyed"
    );
    assert_eq!(
        engine.state().players[0].life,
        23,
        "Caster gains 3 life equal to Hill Giant's toughness"
    );
}
