#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn vug_lizard() -> CardIndex {
    card_index("ac9ec838-fbad-4f01-846b-3084e1d37a44")
}

#[test]
fn vug_lizard_has_mountainwalk_and_echo() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[vug_lizard()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[mountain(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, vug_lizard());
    pass_until(&mut engine, stack_is_empty);

    let liz = on_battlefield(&engine, p0, vug_lizard()).expect("lizard on bf");
    assert!(keywords(&engine, liz).contains(KeywordSet::MOUNTAINWALK));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, vug_lizard()).is_none());
}

#[test]
fn vug_lizard_echo_can_be_paid_and_is_not_due_twice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[], &[], &[vug_lizard()], &[]);
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, p0, vug_lizard(), &[], &[]);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, vug_lizard()).is_some(),
        "no echo on opponent upkeep"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, vug_lizard()).is_some());
    assert_eq!(lk::pool_total(&engine, p0), 0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, vug_lizard()).is_some(),
        "echo was paid already"
    );
}

#[test]
fn vug_lizard_mountainwalk_changes_the_block_offer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for land in [mountain(), forest()] {
        let mut engine = lk::game(SEED, &[], &[gray_ogre(), land], &[vug_lizard()], &[]);
        lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
        lk::cast_at(&mut engine, p0, vug_lizard(), &[], &[]);
        reach_their_main_phase(&mut engine, p1);
        pass_until(&mut engine, |e| {
            e.state().turn.active == p0
                && e.state().turn.step == crate::turn::Step::Upkeep
                && matches!(e.pending(), Pending::Priority { .. })
        });
        lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        lk::yes_no(&mut engine, true);
        reach_main_phase(&mut engine, p0);
        let lizard = lk::obj(&engine, p0, vug_lizard());
        let ogre = lk::obj(&engine, p1, gray_ogre());
        let blocks = lk::attack(&mut engine, &[lizard]);
        assert_eq!(lk::may_block(&blocks, ogre, lizard), land != mountain());
    }
}
