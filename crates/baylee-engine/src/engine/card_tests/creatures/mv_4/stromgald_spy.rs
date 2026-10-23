//! `cards/creatures/mv_4/stromgald_spy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn stromgald_spy() -> CardIndex {
    card_index("b7aa0625-c691-4219-b099-3798b349d09c")
}

/// Stromgald Spy ({3}{B} 2/4 Creature — Human Rogue):
/// - "Whenever this creature attacks and isn't blocked, you may have defending player play with their hand revealed for as long as this creature remains on the battlefield. If you do, this creature assigns no combat damage this turn."
#[test]
fn stromgald_spy_reveals_defending_players_hand_when_unblocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[stromgald_spy()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spy = on_battlefield(&engine, p0, stromgald_spy()).unwrap();

    pass_until(&mut engine, |e| {
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
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        engine.state().journal.entries().iter().any(|e| {
            matches!(
                e.event,
                crate::event::GameEvent::Revealed { player, .. } if player == p1
            )
        }),
        "defending player's hand must be revealed"
    );
}
