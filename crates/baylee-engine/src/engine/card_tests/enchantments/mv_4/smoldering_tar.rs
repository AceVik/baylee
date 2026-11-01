//! Smoldering Tar — {2}{B}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, target player loses 1 life.
//! Oracle: Sacrifice this enchantment: It deals 4 damage to target creature. Activate only as a sorcery.

use super::*;
use baylee_core::generated::index;

#[test]
fn smoldering_tar_drains_target_player_on_upkeep_and_sacrifices_at_sorcery_speed_for_four_damage() {
    let tar = card_index("f268699f-c0a2-43f7-b933-7665599ab5ff");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), mountain(), mountain()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[tar])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, tar);
    pass_until(&mut engine, stack_is_empty);
    let tar_id = on_battlefield(&engine, p0, tar).expect("Smoldering Tar on board");

    // Walk to P0's next upkeep: trigger asks for target player, P1 loses 1 life.
    assert!(walk_to_own_main(&mut engine, p1));
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
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
    assert_eq!(life_of(&engine, p1), 19);

    // Reach P0's main phase and sacrifice Smoldering Tar to deal 4 damage to Hill Giant (3/3).
    assert!(walk_to_own_main(&mut engine, p0));
    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tar_id,
                ability_index: 1,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, giant_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, tar).is_none());
    assert!(in_graveyard(&engine, p0, tar).is_some());
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
}
