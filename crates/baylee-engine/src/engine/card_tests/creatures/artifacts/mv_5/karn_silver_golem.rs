#[allow(clippy::wildcard_imports)]
use super::*;

fn karn_silver_golem() -> CardIndex {
    card_index("8a7c6391-020a-4d7a-8abb-702631bbee56")
}

#[test]
fn karn_silver_golem_animates_noncreature_artifact_with_pt_equal_to_cmc() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[karn_silver_golem(), sol_ring(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, karn_silver_golem(), 1);
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ring), (1, 1));
}

#[test]
fn karn_animates_only_noncreature_artifacts_until_cleanup() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[karn_silver_golem(), forest()],
        &[sol_ring()],
        &[],
        &[],
    );
    let karn = lk::obj(&e, p0, karn_silver_golem());
    let land = lk::obj(&e, p0, forest());
    let ring = lk::obj(&e, p1, sol_ring());
    lk::float(&mut e, p0, &[(ManaColor::Colorless, 1)]);
    activate(&mut e, p0, karn_silver_golem(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(!options.contains(&karn) && !options.contains(&land));
    lk::aim(&mut e, &[ring], &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, ring), (1, 1));
    assert!(
        e.state()
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    reach_their_main_phase(&mut e, p1);
    assert!(
        !e.state()
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}

#[test]
fn karn_blocks_or_becomes_blocked_once_even_by_two_creatures() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for attacking in [false, true] {
        let mut e = lk::game(
            SEED,
            &[karn_silver_golem()],
            &[grizzly_bears(), llanowar_elves()],
            &[],
            &[],
        );
        let karn = lk::obj(&e, p0, karn_silver_golem());
        let bear = lk::obj(&e, p1, grizzly_bears());
        let elves = lk::obj(&e, p1, llanowar_elves());
        if attacking {
            lk::attack(&mut e, &[karn]);
            lk::block(&mut e, &[(bear, karn), (elves, karn)]);
        } else {
            reach_their_main_phase(&mut e, p1);
            lk::attack(&mut e, &[bear]);
            lk::block(&mut e, &[(karn, bear)]);
        }
        lk::settle(&mut e);
        assert_eq!(pt(&e, karn), (0, 8));
        lk::through_combat(&mut e);
        assert_eq!(lk::life(&e, p0), 20);
        assert_eq!(lk::life(&e, p1), 20);
        reach_their_main_phase(&mut e, if attacking { p1 } else { p0 });
        assert_eq!(pt(&e, karn), (4, 4));
    }
}
