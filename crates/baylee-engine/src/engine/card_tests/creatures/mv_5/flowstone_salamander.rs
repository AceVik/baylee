//! `cards/creatures/mv_5/flowstone_salamander.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flowstone_salamander() -> CardIndex {
    card_index("a0e40eb1-a909-405b-8fb4-3759df8edf7f")
}

/// Flowstone Salamander (`a0e40eb1-a909-405b-8fb4-3759df8edf7f`) — {3}{R}{R} Creature — Salamander:
/// "{R}: This creature deals 1 damage to target creature blocking it."
#[test]
fn flowstone_salamander_deals_one_damage_to_target_creature_blocking_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[flowstone_salamander(), index::MOUNTAIN, index::MOUNTAIN],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sal = on_battlefield(&engine, P0, flowstone_salamander()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sal, Defender::Player(P1))],
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
                blockers: vec![(bears, sal)],
            },
        )
        .unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, flowstone_salamander(), 0);
    let idle = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    let menu = aim_at(&mut engine, P0, bears);
    assert!(!menu.contains(&idle));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        crate::engine::card_tests::legends_kit::damage_on(&engine, bears),
        1
    );
}
