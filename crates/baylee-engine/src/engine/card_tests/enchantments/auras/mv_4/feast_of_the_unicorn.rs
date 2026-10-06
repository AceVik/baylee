//! `cards/enchantments/auras/mv_4/feast_of_the_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Feast of the Unicorn` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +4/+0."
///
/// Verifies that casting `Feast of the Unicorn` grants +4/+0 to the targeted creature
/// without modifying its toughness or affecting a bystander creature.
#[test]
fn feast_of_the_unicorn_grants_plus_four_plus_zero() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1311, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[feast_of_the_unicorn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, feast_of_the_unicorn());
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

    let aura = on_battlefield(&engine, p0, feast_of_the_unicorn()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (5, 1),
        "1/1 elf gets +4/+0 to become 5/1"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
}
