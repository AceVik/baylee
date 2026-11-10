//! Thunderscape Master — {2}{R}{R} — Creature — Human Wizard
//! Oracle: {B}{B}, {T}: Target player loses 2 life and you gain 2 life.
//! Oracle: {G}{G}, {T}: Creatures you control get +2/+2 until end of turn.

use super::*;
use baylee_core::generated::index;

#[test]
fn thunderscape_master_drains_target_player_and_pumps_controlled_creatures_until_end_of_turn() {
    let master = card_index("e1db1798-17d1-4ef3-80d7-3f9115a01f7b");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                master,
                swamp(),
                swamp(),
                forest(),
                forest(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let master_id = on_battlefield(&engine, p0, master).expect("Thunderscape Master on board");
    let bears_id = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, master_id), (2, 2));

    // Ability 0: {B}{B}, {T}: Target player loses 2 life and you gain 2 life.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: master_id,
                ability_index: 0,
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

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22);

    // Next turn of P0: Thunderscape Master untaps and activates Ability 1 ({G}{G}, {T}: Creatures you control get +2/+2 until end of turn).
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    let master_id = on_battlefield(&engine, p0, master).expect("Thunderscape Master survived");
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: master_id,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, master_id), (4, 4));
    assert_eq!(pt(&engine, bears_id), (4, 4));
    assert_eq!(pt(&engine, giant_id), (3, 3));

    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(pt(&engine, master_id), (2, 2));
    assert_eq!(pt(&engine, bears_id), (2, 2));
}
