//! `cards/creatures/mv_5/bubbling_beebles.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bubbling_beebles() -> CardIndex {
    card_index("75903c3b-2dbe-4662-b6a8-39599e49adde")
}

#[test]
fn bubbling_beebles_reads_only_the_player_it_is_attacking() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for defender_has_enchantment in [false, true] {
        let defender_board = if defender_has_enchantment {
            vec![gray_ogre(), sylvan_library()]
        } else {
            vec![gray_ogre()]
        };
        let mut engine = Duel::table(SEED, forest(), 3)
            .battlefield(0, &[bubbling_beebles()])
            .battlefield(1, &defender_board)
            .battlefield(2, &[sylvan_library(), gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let beebles = lk::obj(&engine, p0, bubbling_beebles());
        let blocker = lk::obj(&engine, p1, gray_ogre());
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
        assert_eq!(
            lk::may_block(blockers, blocker, beebles),
            !defender_has_enchantment,
            "a different opponent's enchantment never grants evasion against p1"
        );
    }
}

#[test]
fn creature_body_remains_playable() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[bubbling_beebles()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Blue, 6)]);
    cast_with_floating(&mut engine, lk::P0, bubbling_beebles());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, bubbling_beebles()).is_some());
}
