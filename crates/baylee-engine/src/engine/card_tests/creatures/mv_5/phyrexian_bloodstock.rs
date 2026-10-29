//! `cards/creatures/mv_5/phyrexian_bloodstock.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Phyrexian Bloodstock — {4}{B} Creature — Phyrexian Zombie 3/3
/// "When this creature leaves the battlefield, destroy target white creature. It can't be regenerated."
#[test]
fn phyrexian_bloodstock_destroys_target_white_creature_when_it_leaves_the_battlefield() {
    let bloodstock = card_index("dfeaec62-ca77-46dc-8eae-f627a5e1c02a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[bloodstock, island()])
        .hand(0, &[index::UNSUMMON])
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bloodstock_id =
        on_battlefield(&engine, p0, bloodstock).expect("Phyrexian Bloodstock on board");
    let lions_id = on_battlefield(&engine, p1, index::SAVANNAH_LIONS).unwrap();
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(lions_id)
        .unwrap()
        .regeneration_shields = 1;

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::UNSUMMON);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bloodstock_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lions_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, bloodstock).is_some());
    assert!(on_battlefield(&engine, p1, index::SAVANNAH_LIONS).is_none());
    assert!(in_graveyard(&engine, p1, index::SAVANNAH_LIONS).is_some());
}
