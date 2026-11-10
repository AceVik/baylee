//! `cards/creatures/artifacts/mv_3/wizard_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wizard_replica() -> CardIndex {
    card_index("0ba3b1e0-1790-4247-93ab-72a43e8fb580")
}

/// Wizard Replica — flying 1/3; "{U}, Sacrifice this creature: Counter target
/// spell unless its controller pays {2}." Seat 1 casts a three-mana spell off
/// exactly three lands and cannot pay: the spell is countered.
#[test]
fn wizard_replica_counters_a_spell_its_controller_cannot_pay_for() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wizard_replica(), island()])
        .hand(1, &[onulet()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let replica = on_battlefield(&engine, p0, wizard_replica()).expect("seated");
    assert_eq!(pt(&engine, replica), (1, 3));
    assert!(keywords(&engine, replica).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, onulet());
    let spell = on_stack(&engine, onulet()).expect("the spell is on the stack");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, wizard_replica(), 0);
    aim_at(&mut engine, p0, spell);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, wizard_replica()).is_some(),
        "sacrificed"
    );
    assert!(on_battlefield(&engine, p1, onulet()).is_none(), "countered");
    assert!(in_graveyard(&engine, p1, onulet()).is_some());
}
