//! `cards/enchantments/mv_2/angelic_shield.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Angelic Shield` (`Coverage::Implemented`):
/// "Creatures you control get +0/+1. Sacrifice this enchantment: Return target creature to its owner's hand."
///
/// Verifies that `Angelic Shield` provides a static +0/+1 toughness boost to creatures
/// you control, and that its second ability sacrifices itself to return a target
/// creature to its owner's hand, removing the static boost.
#[test]
fn angelic_shield_buffs_toughness_and_sacrifices_to_bounce() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1327, forest())
        .battlefield(0, &[angelic_shield(), llanowar_elves()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("wurm deployed");

    // Static ability boosts controlled creature toughness by 1
    assert_eq!(
        pt(&engine, elf),
        (1, 2),
        "elf gets +0/+1 from Angelic Shield"
    );
    assert_eq!(pt(&engine, wurm), (6, 6), "opponent wurm is untouched");

    // Ability 1 is the activated ability that sacrifices itself to bounce
    activate(&mut engine, p0, angelic_shield(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Angelic Shield, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&wurm));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_none(),
        "bounced wurm left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, rootbreaker_wurm()).is_some(),
        "bounced wurm is in opponent's hand"
    );
    assert!(
        in_graveyard(&engine, p0, angelic_shield()).is_some(),
        "Angelic Shield is in graveyard"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "elf returns to 1/1 after Angelic Shield leaves"
    );
}
