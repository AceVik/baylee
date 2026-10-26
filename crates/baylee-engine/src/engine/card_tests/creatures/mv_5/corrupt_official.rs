//! `cards/creatures/mv_5/corrupt_official.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Corrupt Official — {4}{B} 3/1 Creature — Human Minion:
/// "{2}{B}: Regenerate this creature. Whenever this creature becomes blocked, defending player discards a card at random."
#[test]
fn corrupt_official_regenerates_and_makes_defender_discard_at_random_when_blocked() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[index::CORRUPT_OFFICIAL, swamp(), swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let official = on_battlefield(&engine, P0, index::CORRUPT_OFFICIAL).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, index::CORRUPT_OFFICIAL, 0);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(official, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, official)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_hand(&engine, P1, forest()).is_none());
    assert!(in_graveyard(&engine, P1, forest()).is_some());
    assert!(on_battlefield(&engine, P0, index::CORRUPT_OFFICIAL).is_some());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
