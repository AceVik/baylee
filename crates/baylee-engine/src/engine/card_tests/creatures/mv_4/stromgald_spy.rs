//! `cards/creatures/mv_4/stromgald_spy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn stromgald_spy() -> CardIndex {
    card_index("b7aa0625-c691-4219-b099-3798b349d09c")
}

/// How many times `who`'s hand has been revealed so far.
fn reveals(engine: &Engine<RegistryLookup>, who: PlayerId) -> usize {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| {
            matches!(
                entry.event,
                crate::event::GameEvent::Revealed { player, .. } if player == who
            )
        })
        .count()
}

/// A game on P0's first main phase with Stromgald Spy ready to attack.
fn spy_game(p1_board: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[stromgald_spy()])
        .battlefield(1, p1_board)
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spy = on_battlefield(&engine, p0, stromgald_spy()).unwrap();
    (engine, spy)
}

fn attack_with(engine: &mut Engine<RegistryLookup>, spy: ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(spy, Defender::Player(p1))],
            },
        )
        .unwrap();
}

/// Stromgald Spy ({3}{B} 2/4 Creature — Human Rogue):
/// - "Whenever this creature attacks and isn't blocked, you may have defending player play with their hand revealed ... If you do, this creature assigns no combat damage this turn."
///
/// The hand is revealed once (not for as long as the Spy stays out); the
/// "may" and the "no combat damage" are played.
#[test]
fn stromgald_spy_reveals_the_hand_and_deals_no_combat_damage_when_you_do() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, spy) = spy_game(&[]);
    let revealed_before = reveals(&engine, p1);
    attack_with(&mut engine, spy);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        reveals(&engine, p1),
        revealed_before + 1,
        "defending player's hand must be revealed"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "if you do, the Spy assigns no combat damage"
    );
}

/// "you may": declined, nothing is revealed and the Spy hits for 2.
#[test]
fn stromgald_spy_declined_reveals_nothing_and_deals_its_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, spy) = spy_game(&[]);
    let revealed_before = reveals(&engine, p1);
    attack_with(&mut engine, spy);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(reveals(&engine, p1), revealed_before);
    assert_eq!(engine.state().players[1].life, 18);
}

/// "attacks and isn't blocked": a blocked Spy asks nothing and reveals nothing.
#[test]
fn stromgald_spy_does_nothing_when_it_is_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, spy) = spy_game(&[grizzly_bears()]);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let revealed_before = reveals(&engine, p1);
    attack_with(&mut engine, spy);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, spy)],
            },
        )
        .unwrap();
    // `pass_until` stops on any question it does not own: a "may reveal"
    // question for P0 would fail the test here.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(reveals(&engine, p1), revealed_before);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(on_battlefield(&engine, p0, stromgald_spy()).is_some());
}
