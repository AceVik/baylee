//! `cards/enchantments/auras/mv_7/mythic_proportions.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Mythic Proportions` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +8/+8 and has trample."
///
/// Verifies that casting `Mythic Proportions` grants +8/+8 and `KeywordSet::TRAMPLE`
/// to the targeted creature while leaving a bystander creature unaffected.
#[test]
fn mythic_proportions_grants_pump_and_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1317, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[mythic_proportions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, mythic_proportions());
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

    let aura = on_battlefield(&engine, p0, mythic_proportions()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (9, 9),
        "1/1 elf gets +8/+8 to become 9/9"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "enchanted creature gains trample"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "opponent elf does not gain trample"
    );
}
