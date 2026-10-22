//! `cards/creatures/mv_4/zebra_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn zebra_unicorn() -> CardIndex {
    card_index("2f91818f-a6fe-4472-b52b-ab574f790905")
}

/// Zebra Unicorn (`{2}{G}{W}`, `Coverage::Implemented`): 2/2 Unicorn with
/// "Whenever this creature deals damage, you gain that much life."
#[test]
fn zebra_unicorn_gains_life_equal_to_damage_dealt() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[zebra_unicorn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let unicorn = on_battlefield(&engine, p0, zebra_unicorn()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(unicorn, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22);
}
