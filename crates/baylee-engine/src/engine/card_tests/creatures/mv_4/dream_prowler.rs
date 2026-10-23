//! `cards/creatures/mv_4/dream_prowler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dream_prowler() -> CardIndex {
    card_index("b56c5b39-2465-4a25-b806-bb84866e576f")
}

/// Dream Prowler (`b56c5b39-2465-4a25-b806-bb84866e576f`) — {2}{U}{U} 1/5 Creature — Illusion (Implemented):
/// "Dream Prowler can't be blocked as long as it's attacking alone."
#[test]
fn dream_prowler_becomes_unblockable_when_attacking_alone() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dream_prowler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let prowler = on_battlefield(&engine, P0, dream_prowler()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(prowler, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, prowler).contains(KeywordSet::UNBLOCKABLE));
}

#[test]
fn unblockability_tracks_current_attackers_in_both_directions() {
    use baylee_core::generated::index;
    let mut e = lk::game(
        SEED,
        &[dream_prowler(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[lightning_bolt()],
        &[],
    );
    let prowler = lk::obj(&e, P0, dream_prowler());
    let bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&e, P1, index::HILL_GIANT);
    assert!(!keywords(&e, prowler).contains(KeywordSet::UNBLOCKABLE));
    lk::attackers_offered(&mut e);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (prowler, Defender::Player(P1)),
                (bear, Defender::Player(P1)),
            ],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert!(!keywords(&e, prowler).contains(KeywordSet::UNBLOCKABLE));
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, P0, lightning_bolt(), &[bear], &[]);
    assert!(keywords(&e, prowler).contains(KeywordSet::UNBLOCKABLE));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!lk::may_block(blockers, giant, prowler));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!keywords(&e, prowler).contains(KeywordSet::UNBLOCKABLE));
    assert_eq!(lk::life(&e, P1), 19);
}
