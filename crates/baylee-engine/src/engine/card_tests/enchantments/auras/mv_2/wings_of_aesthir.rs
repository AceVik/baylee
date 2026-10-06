//! `cards/enchantments/auras/mv_2/wings_of_aesthir.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wings of Aesthir` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +1/+0 and has flying and first strike."
///
/// Verifies that casting `Wings of Aesthir` grants +1/+0, `KeywordSet::FLYING`,
/// and `KeywordSet::FIRST_STRIKE` to the enchanted creature while leaving
/// bystander creatures unaffected.
#[test]
fn wings_of_aesthir_grants_pump_flying_and_first_strike() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1304, forest())
        .battlefield(0, &[plains(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wings_of_aesthir()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, wings_of_aesthir());
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

    let aura = on_battlefield(&engine, p0, wings_of_aesthir()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 1),
        "1/1 elf gets +1/+0 to become 2/1"
    );
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE)),
        "enchanted creature gains flying and first strike"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    let their_kw = keywords(&engine, theirs);
    assert!(
        !their_kw.contains(KeywordSet::FLYING) && !their_kw.contains(KeywordSet::FIRST_STRIKE),
        "opponent elf receives no keywords"
    );
}
