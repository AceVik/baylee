//! `cards/instants/mv_2/blood_frenzy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn blood_frenzy() -> CardIndex {
    card_index("8d3d5e87-8b12-4549-a46e-70a103dbcf44")
}

/// Blood Frenzy (8d3d5e87-8b12-4549-a46e-70a103dbcf44) — {1}{R} Instant:
/// "Cast this spell only before the combat damage step. Target attacking or blocking creature gets +4/+0 until end of turn. Destroy that creature at the beginning of the next end step."
#[test]
fn blood_frenzy_pumps_attacker_and_destroys_it_at_end_step() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[quiet_creature(), mountain(), mountain()])
        .hand(0, &[blood_frenzy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let attacker = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    cast_from_hand(&mut engine, P0, blood_frenzy());
    aim_at(&mut engine, P0, attacker);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, attacker), (5, 1));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 15);

    reach_their_main_phase(&mut engine, P1);
    assert!(in_graveyard(&engine, P0, quiet_creature()).is_some());
}
