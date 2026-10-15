//! `cards/creatures/mv_5/windstorm_drake.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;

fn windstorm_drake() -> CardIndex {
    card_index("16977ebd-6384-4c28-b9c3-448563a07807")
}

// Checklist: Drake has flying and keeps its printed 3/3; other friendly flyers
// get +1/+0 and deal the resulting damage; nonflyers/opposing flyers do not change;
// leaving removes the grant.
#[test]
fn windstorm_drake_boosts_only_its_controllers_other_flyers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                windstorm_drake(),
                snapping_drake(),
                llanowar_elves(),
                island(),
            ],
        )
        .battlefield(1, &[snapping_drake()])
        .hand(0, &[unsummon()])
        .start();
    keep_mulligans(&mut e);
    let source = on_battlefield(&e, p0, windstorm_drake()).expect("Windstorm Drake");
    let own_flier = on_battlefield(&e, p0, snapping_drake()).expect("our other flyer");
    let elf = on_battlefield(&e, p0, llanowar_elves()).expect("our nonflyer");
    let their_flier = on_battlefield(&e, p1, snapping_drake()).expect("opponent's flyer");
    assert!(keywords(&e, source).contains(KeywordSet::FLYING));
    assert_eq!(pt(&e, own_flier), (4, 2), "only power increases");
    assert!(keywords(&e, own_flier).contains(KeywordSet::FLYING));
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
    assert_eq!(
        pt(&e, source),
        (3, 3),
        "Windstorm Drake does not buff itself"
    );

    cast_from_hand(&mut e, p0, unsummon());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![source],
        },
    )
    .expect("Unsummon returns the Drake that granted the bonus");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, own_flier), (3, 2), "the grant ends with its source");
}
