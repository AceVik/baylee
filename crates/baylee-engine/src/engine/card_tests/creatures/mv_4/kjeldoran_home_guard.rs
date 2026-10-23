//! Kjeldoran Home Guard — `{3}{W}` 1/6 Creature — Human Soldier (Alliances)
//!
//! At end of combat, if this creature attacked or blocked this combat, put a
//! -0/-1 counter on this creature and create a 0/1 white Deserter creature
//! token.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kjeldoran_home_guard() -> CardIndex {
    card_index("5f7ae611-a78d-4535-9d76-755001bda34c")
}

#[test]
fn kjeldoran_home_guard_gets_minus_zero_minus_one_counter_and_creates_deserter_at_end_of_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[kjeldoran_home_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p0);

    let guard = on_battlefield(&engine, p0, kjeldoran_home_guard()).unwrap();
    assert_eq!(pt(&engine, guard), (1, 6));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guard, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(pt(&engine, guard), (1, 5));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (0, 1));
}

/// Player 1 attacks player 0 with a Grizzly Bears, and the Guard blocks it when `block`. Returns
/// the engine in player 1's second main phase, with the end of combat behind it.
fn bears_attack_the_guard(block: bool) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[kjeldoran_home_guard()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guard = on_battlefield(&engine, p0, kjeldoran_home_guard()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    let blockers = if block { vec![(guard, bears)] } else { vec![] };
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::SecondMain
    });
    (engine, guard)
}

/// "If this creature attacked or blocked this combat": blocking counts, so it gets the counter and
/// the Deserter is its controller's.
#[test]
fn kjeldoran_home_guard_blocking_also_costs_a_toughness_and_makes_a_deserter() {
    let p0 = PlayerId::new(0);
    let (engine, guard) = bears_attack_the_guard(true);
    assert_eq!(pt(&engine, guard), (1, 5));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (0, 1));
    assert_eq!(engine.state().players[0].life, 20, "the Guard blocked");
}

/// ... and a combat in which it did neither is nothing: the other player's creature attacked, the
/// Guard stayed out of it, and there is no counter and no token (on either side of the table).
#[test]
fn kjeldoran_home_guard_that_neither_attacked_nor_blocked_does_nothing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (engine, guard) = bears_attack_the_guard(false);
    assert_eq!(pt(&engine, guard), (1, 6));
    assert!(tokens_of(&engine, p0).is_empty());
    assert!(tokens_of(&engine, p1).is_empty());
    assert_eq!(
        engine.state().players[0].life,
        18,
        "the Bears were not blocked"
    );
}
