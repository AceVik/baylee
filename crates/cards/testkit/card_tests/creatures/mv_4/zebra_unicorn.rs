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

/// "Whenever this creature deals damage" is any damage it deals, to a creature
/// as well as to a player, and only the damage it deals: the 2 the blocking
/// Bears deal to it gain nothing (life ends at 22, not 24), and the opponent
/// takes no damage.
#[test]
fn zebra_unicorn_gains_life_for_damage_dealt_to_a_blocker_but_not_damage_taken() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[zebra_unicorn()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let unicorn = on_battlefield(&engine, p0, zebra_unicorn()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
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
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, unicorn)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "the Unicorn dealt its 2 damage to the Bears"
    );
    assert!(
        in_graveyard(&engine, p0, zebra_unicorn()).is_some(),
        "the Bears dealt their 2 damage back"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "2 life for 2 dealt, none for 2 taken"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the blocked attack hit no player"
    );
}
