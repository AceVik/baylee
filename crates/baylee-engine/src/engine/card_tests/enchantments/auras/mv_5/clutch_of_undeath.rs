//! `cards/enchantments/auras/mv_5/clutch_of_undeath.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Clutch of Undeath` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +3/+3 as long as it's a Zombie. Otherwise, it gets -3/-3."
///
/// Verifies that casting `Clutch of Undeath` on a Zombie creature grants it +3/+3.
#[test]
fn clutch_of_undeath_gives_plus_three_plus_three_to_zombie() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1316, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                festering_goblin(),
            ],
        )
        .hand(0, &[clutch_of_undeath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let goblin =
        on_battlefield(&engine, p0, festering_goblin()).expect("festering goblin deployed");
    assert_eq!(pt(&engine, goblin), (1, 1));

    cast_from_hand(&mut engine, p0, clutch_of_undeath());
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
                objects: vec![goblin],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, clutch_of_undeath()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(goblin),
        "Aura attached to chosen goblin"
    );
    assert_eq!(
        pt(&engine, goblin),
        (4, 4),
        "1/1 Zombie gets +3/+3 to become 4/4"
    );
}

/// Clutch of Undeath's other half: "Otherwise, it gets -3/-3." Aimed at a
/// Serra Angel, which is no Zombie, the 4/4 becomes a 1/1.
#[test]
fn clutch_of_undeath_shrinks_a_creature_that_is_not_a_zombie() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4701, forest())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), swamp(), serra_angel()],
        )
        .hand(0, &[clutch_of_undeath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let angel = on_battlefield(&engine, p0, serra_angel()).expect("the Angel");
    assert_eq!(pt(&engine, angel), (4, 4));

    cast_from_hand(&mut engine, p0, clutch_of_undeath());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![angel],
            },
        )
        .expect("the Angel is a creature to enchant");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, angel), (1, 1), "4/4 with -3/-3");
}
