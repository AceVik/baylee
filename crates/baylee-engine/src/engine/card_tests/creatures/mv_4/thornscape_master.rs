//! Thornscape Master — {2}{G}{G} — Creature — Human Wizard
//! Oracle: {R}{R}, {T}: This creature deals 2 damage to target creature.
//! Oracle: {W}{W}, {T}: Target creature gains protection from the color of your choice until end of turn.

use super::*;
use baylee_core::generated::index;

#[test]
fn thornscape_master_deals_two_damage_to_target_creature_and_grants_protection_from_chosen_color() {
    let master = card_index("ea802eda-fc8d-45c7-b5a3-238a2cdc4dcb");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                master,
                mountain(),
                mountain(),
                plains(),
                plains(),
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::TERROR])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let master_id = on_battlefield(&engine, p0, master).expect("Thornscape Master on board");
    let bears_id = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let giant_id = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, master_id), (2, 2));

    // Ability 0: {R}{R}, {T}: This creature deals 2 damage to target creature.
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
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());

    // Next turn of P0: Thornscape Master untaps and activates Ability 1 ({W}{W}, {T}: Target creature gains protection from the color of your choice until end of turn).
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    let master_id = on_battlefield(&engine, p0, master).expect("Thornscape Master survived");
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
    aim_at(&mut engine, p0, giant_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Hill Giant now has protection from black and cannot be targeted by Terror.
    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, p0, index::TERROR);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected target choice for Terror");
    };
    assert!(!options.contains(&giant_id));
    assert!(options.contains(&master_id));
}
