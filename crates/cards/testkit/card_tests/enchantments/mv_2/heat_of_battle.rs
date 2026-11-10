//! `cards/enchantments/mv_2/heat_of_battle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn heat_of_battle() -> CardIndex {
    card_index("400285bb-ded9-4018-a7ea-3e6f4d24b900")
}

/// Heat of Battle (`400285bb-ded9-4018-a7ea-3e6f4d24b900`) — {1}{R} Enchantment (Implemented):
/// "Whenever a creature blocks, this enchantment deals 1 damage to that creature's controller."
#[test]
fn heat_of_battle_deals_one_damage_to_blocking_creatures_controller() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[heat_of_battle(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let attacker = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

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
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 20);
}
