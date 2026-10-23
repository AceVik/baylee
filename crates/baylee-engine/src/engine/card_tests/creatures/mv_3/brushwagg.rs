//! `cards/creatures/mv_3/brushwagg.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn brushwagg_index() -> CardIndex {
    card_index("33a981ac-878d-4257-936f-55b4e6543c12")
}

#[test]
fn brushwagg_gets_minus_two_plus_two_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brushwagg_index()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let brushwagg = on_battlefield(&engine, p0, brushwagg_index()).unwrap();
    let blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brushwagg, Defender::Player(p1))],
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
                blockers: vec![(blocker, brushwagg)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, brushwagg), (1, 4));
    // "Until end of turn": back to 3/2 on the opponent's turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, brushwagg), (3, 2));
}

/// p0 attacks p1 with the Brushwagg; stops at p1's declare blockers.
fn brushwagg_attacks(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    reach_main_phase(engine, p0);
    let brushwagg = on_battlefield(engine, p0, brushwagg_index()).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brushwagg, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    brushwagg
}

#[test]
fn brushwagg_is_unchanged_when_it_is_not_blocked() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brushwagg_index()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    let brushwagg = brushwagg_attacks(&mut engine);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, brushwagg), (3, 2));
}

#[test]
fn brushwagg_gets_it_once_when_blocked_by_two_creatures() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brushwagg_index()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    let brushwagg = brushwagg_attacks(&mut engine);
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, brushwagg), (giant, brushwagg)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, brushwagg),
        (1, 4),
        "it becomes blocked once, however many creatures block it"
    );
}

#[test]
fn brushwagg_gets_it_when_it_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brushwagg_index()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    let brushwagg = on_battlefield(&engine, p0, brushwagg_index()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(brushwagg, bear)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, brushwagg), (1, 4));
}
