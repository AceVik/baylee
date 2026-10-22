//! `cards/creatures/mv_5/bellowing_fiend.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bellowing_fiend() -> CardIndex {
    card_index("37331d0e-edda-4791-8bcc-fb457f984877")
}

/// Bellowing Fiend (37331d0e-edda-4791-8bcc-fb457f984877) — {4}{B} Creature — Spirit (Partial):
/// "Flying. Whenever this creature deals damage to a creature, this creature deals 3 damage to that creature's controller and 3 damage to you."
#[test]
fn bellowing_fiend_deals_three_damage_to_both_players_when_dealing_combat_damage_to_creature() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[bellowing_fiend()])
        .battlefield(1, &[index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let fiend = on_battlefield(&engine, P0, bellowing_fiend()).unwrap();
    let blocker = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fiend, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, fiend)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 17);
}
