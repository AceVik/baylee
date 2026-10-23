//! `cards/creatures/mv_3/lowland_basilisk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lowland_basilisk() -> CardIndex {
    card_index("effe239e-ade0-4ee2-99d7-f93b2f5ef585")
}

/// Lowland Basilisk (`effe239e-ade0-4ee2-99d7-f93b2f5ef585`) — {2}{G} 1/3 Creature — Basilisk (Partial):
/// "Whenever this creature deals damage to a creature, destroy that creature at end of combat."
#[test]
fn lowland_basilisk_destroys_creature_dealt_combat_damage_at_end_of_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lowland_basilisk()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let basilisk = on_battlefield(&engine, P0, lowland_basilisk()).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(basilisk, Defender::Player(P1))],
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
                blockers: vec![(giant, basilisk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });

    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
}
