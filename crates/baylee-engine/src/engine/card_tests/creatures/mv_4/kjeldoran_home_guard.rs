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
