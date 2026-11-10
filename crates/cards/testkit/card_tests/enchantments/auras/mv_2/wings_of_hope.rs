//! `cards/enchantments/auras/mv_2/wings_of_hope.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wings of Hope` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +1/+3 and has flying."
///
/// Verifies that casting `Wings of Hope` gives +1/+3 and `KeywordSet::FLYING`
/// to the targeted creature without affecting an identical bystander.
#[test]
fn wings_of_hope_grants_pump_and_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1305, forest())
        .battlefield(0, &[plains(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wings_of_hope()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, wings_of_hope());
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

    let aura = on_battlefield(&engine, p0, wings_of_hope()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 4),
        "1/1 elf gets +1/+3 to become 2/4"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature gains flying"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "opponent elf does not gain flying"
    );
}
