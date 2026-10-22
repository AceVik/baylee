//! `cards/creatures/mv_3/zhalfirin_crusader.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn zhalfirin_crusader() -> CardIndex {
    card_index("9e60c102-412e-4956-a7bb-a2cd737d6692")
}

/// `Zhalfirin Crusader` ({1}{W}{W}, 2/2 Creature — Human Knight,
/// `Coverage::Partial`) has flanking, giving a non-flanking blocker -1/-1 until
/// end of turn.
#[test]
fn zhalfirin_crusader_has_flanking_and_shrinks_non_flanking_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[zhalfirin_crusader()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, zhalfirin_crusader());
    pass_until(&mut engine, stack_is_empty);
    let crusader = on_battlefield(&engine, p0, zhalfirin_crusader()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crusader, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, crusader)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, blocker), (1, 1));
}
