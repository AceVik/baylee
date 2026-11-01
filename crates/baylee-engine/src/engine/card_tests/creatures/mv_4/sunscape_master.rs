//! Sunscape Master — {2}{W}{W} — Creature — Human Wizard
//! Oracle: {G}{G}, {T}: Creatures you control get +2/+2 until end of turn.
//! Oracle: {U}{U}, {T}: Return target creature to its owner's hand.

use super::*;
use baylee_core::generated::index;

#[test]
fn sunscape_master_bounces_target_creature_and_pumps_controlled_creatures_until_end_of_turn() {
    let master = card_index("e3be8e19-cec1-43ae-9e22-d7cb374570bb");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                master,
                island(),
                island(),
                forest(),
                forest(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let master_id = on_battlefield(&engine, p0, master).expect("Sunscape Master on board");
    let bears_id = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, master_id), (2, 2));
    assert_eq!(pt(&engine, bears_id), (2, 2));

    // Ability 0: {U}{U}, {T}: Return target creature to its owner's hand.
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
    aim_at(&mut engine, p0, giant_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_hand(&engine, p1, index::HILL_GIANT).is_some());

    // Next turn of P0: Sunscape Master untaps and activates Ability 1 ({G}{G}, {T}: Creatures you control get +2/+2 until end of turn).
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    let master_id = on_battlefield(&engine, p0, master).expect("Sunscape Master survived");
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

    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(pt(&engine, master_id), (2, 2));
    assert_eq!(pt(&engine, bears_id), (2, 2));
}
