//! `cards/creatures/mv_4/crypt_cobra.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::CRYPT_COBRA;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Crypt Cobra (456bbc0e-a648-481f-b37e-9996029556bd) — {3}{B} 3/3:
/// "Whenever this creature attacks and isn't blocked, defending player gets a
/// poison counter."
#[test]
fn crypt_cobra_gives_defending_player_a_poison_counter_when_unblocked() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[CRYPT_COBRA])
        .start();
    keep_mulligans(&mut engine);

    let cobra = on_battlefield(&engine, P0, CRYPT_COBRA).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cobra, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].poison, 1);
    assert_eq!(engine.state().players[1].life, 17);
}
