//! `cards/creatures/mv_4/dream_prowler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dream_prowler() -> CardIndex {
    card_index("b56c5b39-2465-4a25-b806-bb84866e576f")
}

/// Dream Prowler (`b56c5b39-2465-4a25-b806-bb84866e576f`) — {2}{U}{U} 1/5 Creature — Illusion (Partial):
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
