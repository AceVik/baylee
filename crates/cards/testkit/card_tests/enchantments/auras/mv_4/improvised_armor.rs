//! `cards/enchantments/auras/mv_4/improvised_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Improvised Armor` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +2/+5. Cycling `{{3}}`."
///
/// Verifies that casting `Improvised Armor` gives +2/+5 to the targeted creature
/// while leaving a bystander creature unaffected.
#[test]
fn improvised_armor_gives_plus_two_plus_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1313, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[improvised_armor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, improvised_armor());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, improvised_armor()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 6),
        "1/1 elf gets +2/+5 to become 3/6"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
}
