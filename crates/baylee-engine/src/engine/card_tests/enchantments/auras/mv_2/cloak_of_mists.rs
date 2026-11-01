#[allow(clippy::wildcard_imports)]
use super::*;

fn cloak_of_mists() -> CardIndex {
    card_index("13358b1d-0a35-48ed-a7fb-70359fa3998f")
}

#[test]
fn cloak_of_mists_makes_enchanted_creature_unblockable() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[cloak_of_mists()])
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, cloak_of_mists());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::UNBLOCKABLE));
}

#[test]
fn cloak_of_mists_excludes_its_host_from_the_block_offer_only() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[grizzly_bears(), gray_ogre()],
        &[grizzly_bears()],
        &[cloak_of_mists()],
        &[],
    );
    let host = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let other = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    lk::float(&mut engine, p0, &[(ManaColor::Blue, 2)]);
    lk::cast_at(&mut engine, p0, cloak_of_mists(), &[host], &[]);
    let offer = lk::attack(&mut engine, &[host, other]);
    assert!(!lk::may_block(&offer, blocker, host));
    assert!(lk::may_block(&offer, blocker, other));
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, host)]
                }
            )
            .is_err()
    );
    lk::block(&mut engine, &[(blocker, other)]);
}
