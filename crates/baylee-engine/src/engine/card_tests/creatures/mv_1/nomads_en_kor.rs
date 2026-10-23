//! `cards/creatures/mv_1/nomads_en_kor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn nomads_en_kor() -> CardIndex {
    card_index("b8d395a3-0bfe-45c3-bb3a-820d4f235b88")
}

/// Nomads en-Kor (`b8d395a3-0bfe-45c3-bb3a-820d4f235b88`) — {W} 1/1 Creature — Kor Nomad Soldier (Partial):
/// Plays as a 1/1 Kor Nomad Soldier.
#[test]
fn nomads_en_kor_enters_and_deals_combat_damage() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[nomads_en_kor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let nomads = on_battlefield(&engine, P0, nomads_en_kor()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(nomads, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });

    assert_eq!(engine.state().players[1].life, 19);
}
