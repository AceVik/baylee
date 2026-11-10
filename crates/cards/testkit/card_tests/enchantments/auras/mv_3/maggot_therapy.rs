//! `cards/enchantments/auras/mv_3/maggot_therapy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Maggot Therapy` (`Coverage::Implemented`):
/// "Flash. Enchant creature. Enchanted creature gets +2/-2."
///
/// Verifies that `Maggot Therapy` modifies power by +2 and toughness by -2
/// on the targeted creature, leaving another creature unchanged.
#[test]
fn maggot_therapy_modifies_power_and_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1306, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), a_seven_five_wurm()])
        .battlefield(1, &[a_seven_five_wurm()])
        .hand(0, &[maggot_therapy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, a_seven_five_wurm()).expect("my wurm deployed");
    let theirs = on_battlefield(&engine, p1, a_seven_five_wurm()).expect("their wurm deployed");
    assert_eq!(pt(&engine, mine), (7, 5));

    cast_from_hand(&mut engine, p0, maggot_therapy());
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

    let aura = on_battlefield(&engine, p0, maggot_therapy()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to the chosen wurm"
    );
    assert_eq!(
        pt(&engine, mine),
        (9, 3),
        "7/5 wurm gets +2/-2 to become 9/3"
    );
    assert_eq!(
        pt(&engine, theirs),
        (7, 5),
        "the wurm across the table is untouched"
    );
}
