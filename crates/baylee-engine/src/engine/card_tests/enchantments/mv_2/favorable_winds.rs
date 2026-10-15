//! `cards/enchantments/mv_2/favorable_winds.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn favorable_winds() -> CardIndex {
    card_index("2361ca87-6352-4ba3-8d91-b3d71242914d")
}

// Checklist: own flying creature gets +1/+1 and deals its increased damage;
// own nonflyer and opposing flyer are unchanged; removing the enchantment removes both bonuses.
#[test]
fn favorable_winds_buffs_only_flying_creatures_its_controller_controls() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                favorable_winds(),
                snapping_drake(),
                llanowar_elves(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[snapping_drake()])
        .hand(0, &[index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    let source = on_battlefield(&e, p0, favorable_winds()).expect("Favorable Winds");
    let own_flier = on_battlefield(&e, p0, snapping_drake()).expect("our flyer");
    let elf = on_battlefield(&e, p0, llanowar_elves()).expect("our nonflyer");
    let their_flier = on_battlefield(&e, p1, snapping_drake()).expect("opposing flyer");
    assert_eq!(pt(&e, own_flier), (4, 3));
    assert_eq!(pt(&e, elf), (1, 1));
    assert_eq!(pt(&e, their_flier), (3, 2));

    let blocks = attack_and_collect_blocks(&mut e, own_flier, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == their_flier && b.attackers.contains(&own_flier))
    );
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("the opponent has no legal block");
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(
        e.state().players[1].life,
        16,
        "the other flyer deals four damage"
    );

    cast_from_hand(&mut e, p0, index::DISENCHANT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![source],
        },
    )
    .expect("Disenchant destroys Favorable Winds");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, own_flier), (3, 2), "the bonus ends with its source");
}
