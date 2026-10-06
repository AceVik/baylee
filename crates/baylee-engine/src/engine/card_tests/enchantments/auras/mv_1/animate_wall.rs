//! `cards/enchantments/auras/mv_1/animate_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Animate Wall: "Enchant Wall" / "Enchanted Wall can attack as though it
/// didn't have defender." The Wall it enchants is offered as an attacker; a
/// second, unenchanted Wall beside it — just as much a Wall — is not.
#[test]
fn animate_wall_lets_only_the_enchanted_wall_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wall_of_spears(), wall_of_roots(), plains()])
        .hand(0, &[animate_wall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let enchanted_wall = on_battlefield(&engine, p0, wall_of_spears()).expect("seated");
    let bare_wall = on_battlefield(&engine, p0, wall_of_roots()).expect("seated");

    cast_from_hand(&mut engine, p0, animate_wall());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Enchant Wall asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&enchanted_wall) && options.contains(&bare_wall),
        "either Wall may be enchanted: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchanted_wall],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, animate_wall()).is_some()
    });
    let aura = on_battlefield(&engine, p0, animate_wall()).expect("resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(enchanted_wall),
        "the Aura attached to the Wall it targeted"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers")
    };
    assert!(
        attackers.contains(&enchanted_wall),
        "Animate Wall lets its enchanted Wall attack despite defender"
    );
    assert!(
        !attackers.contains(&bare_wall),
        "the unenchanted Wall still can't attack"
    );
}
