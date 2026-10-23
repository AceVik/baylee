#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn bouncing_beebles() -> CardIndex {
    card_index("4fa46972-d953-4328-9c31-96e134234133")
}

#[test]
fn bouncing_beebles_is_unblockable_when_opponent_controls_an_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[bouncing_beebles()])
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[sol_ring(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bouncing_beebles());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bb = on_battlefield(&engine, p0, bouncing_beebles()).unwrap();
    let legal = attack_and_collect_blocks(&mut engine, bb, p1);
    assert!(legal.is_empty());
}

#[test]
fn bouncing_beebles_only_reads_the_player_it_is_attacking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[bouncing_beebles()])
        .battlefield(1, &[grizzly_bears()])
        .battlefield(2, &[sol_ring(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beebles = lk::obj(&engine, p0, bouncing_beebles());
    let bear = lk::obj(&engine, p1, grizzly_bears());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beebles, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        lk::may_block(blockers, bear, beebles),
        "another opponent's artifact grants no evasion against p1"
    );
}
