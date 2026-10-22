//! `cards/creatures/mv_1/shadow_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn shadow_guildmage() -> CardIndex {
    card_index("cdda63d7-e3ea-455c-b7d4-c6cfdda2703c")
}

/// Shadow Guildmage (`{B}`, `Coverage::Partial`): `{U}, {T}` puts target
/// creature you control on top (or bottom) of its owner's library, and
/// `{R}, {T}` deals 1 damage to any target and 1 damage to you.
#[test]
fn shadow_guildmage_abilities_return_creature_and_deal_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[shadow_guildmage(), island(), mountain(), grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mage = on_battlefield(&engine, p0, shadow_guildmage()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // Activate ability 0 ({U}, {T}): put target creature you control on top of library.
    let my_island = on_battlefield(&engine, p0, island()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: my_island,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mage,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());

    // Next turn, activate ability 1 ({R}, {T}): deal 1 damage to p1 and 1 damage to p0.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let my_mountain = on_battlefield(&engine, p0, mountain()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: my_mountain,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mage,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 19);
}
