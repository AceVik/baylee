//! `cards/enchantments/auras/mv_4/serra_s_embrace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Serra's Embrace` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +2/+2 and has flying and vigilance."
///
/// Verifies that `Serra's Embrace` grants +2/+2, `KeywordSet::FLYING`, and
/// `KeywordSet::VIGILANCE` to the enchanted creature while leaving a bystander creature unaffected.
#[test]
fn serra_s_embrace_grants_pump_flying_and_vigilance() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1314, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[serra_s_embrace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, serra_s_embrace());
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

    let aura = on_battlefield(&engine, p0, serra_s_embrace()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "1/1 elf gets +2/+2 to become 3/3"
    );
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING.union(KeywordSet::VIGILANCE)),
        "enchanted creature gains flying and vigilance"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    let their_kw = keywords(&engine, theirs);
    assert!(
        !their_kw.contains(KeywordSet::FLYING) && !their_kw.contains(KeywordSet::VIGILANCE),
        "opponent elf receives no keywords"
    );
}
