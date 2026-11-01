//! `cards/creatures/mv_3/elite_javelineer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elite_javelineer() -> CardIndex {
    card_index("649300e2-c491-435d-92d5-f1b88f0e2e2e")
}

/// Elite Javelineer (`649300e2-c491-435d-92d5-f1b88f0e2e2e`) — {2}{W} Creature — Human Soldier:
/// "Whenever this creature blocks, it deals 1 damage to target attacking creature."
#[test]
fn elite_javelineer_deals_one_damage_to_target_attacker_when_blocking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[index::GRIZZLY_BEARS, llanowar_elves(), index::HILL_GIANT],
        )
        .battlefield(1, &[elite_javelineer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    let javelineer = on_battlefield(&engine, P1, elite_javelineer()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1)), (elves, Defender::Player(P1))],
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
                blockers: vec![(javelineer, bears)],
            },
        )
        .unwrap();

    // Elite Javelineer targets the unblocked attacking Llanowar Elves.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let idle = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let menu = aim_at(&mut engine, P1, elves);
    assert!(!menu.contains(&idle));
    assert!(!menu.contains(&javelineer));
    assert!(menu.contains(&bears));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, P0, llanowar_elves()).is_some());
}
