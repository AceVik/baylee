//! `cards/enchantments/auras/mv_2/torment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Torment` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets -3/-0."
///
/// Verifies that casting `Torment` on an opponent's creature reduces its power
/// by 3 while leaving its toughness unchanged, and leaves other creatures untouched.
#[test]
fn torment_reduces_power_by_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1302, forest())
        .battlefield(0, &[swamp(), forest()])
        .battlefield(1, &[rootbreaker_wurm(), llanowar_elves()])
        .hand(0, &[torment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("wurm deployed");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf deployed");
    assert_eq!(pt(&engine, wurm), (6, 6));

    cast_from_hand(&mut engine, p0, torment());
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
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, torment()).expect("Torment resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(wurm),
        "Torment attached to wurm"
    );
    assert_eq!(
        pt(&engine, wurm),
        (3, 6),
        "6/6 wurm gets -3/-0 to become 3/6"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "bystander elf is untouched");
}
