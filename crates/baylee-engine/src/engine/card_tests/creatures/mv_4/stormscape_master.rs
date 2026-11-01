//! Stormscape Master — {2}{U}{U} — Creature — Human Wizard
//! Oracle: {W}{W}, {T}: Target creature gains protection from the color of your choice until end of turn.
//! Oracle: {B}{B}, {T}: Target player loses 2 life and you gain 2 life.

use super::*;
use baylee_core::generated::index;

#[test]
fn stormscape_master_grants_protection_from_chosen_color_and_drains_two_life() {
    let master = card_index("bfcd6c5d-7189-4d84-b0df-0a5ce5f43528");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                master,
                plains(),
                plains(),
                swamp(),
                swamp(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let master_id = on_battlefield(&engine, p0, master).expect("Stormscape Master on board");
    let bears_id = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, master_id), (2, 2));

    // Ability 0: {W}{W}, {T}: Target creature gains protection from the color of your choice until end of turn.
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
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    // Grizzly Bears now has protection from red and cannot be targeted by Lightning Bolt.
    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected target choice for Lightning Bolt");
    };
    assert!(!options.contains(&bears_id));
    assert!(options.contains(&master_id));
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
    assert_eq!(engine.state().players[1].life, 17);

    // Next turn of P0: Stormscape Master untaps and activates Ability 1 ({B}{B}, {T}: Target player loses 2 life and you gain 2 life).
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    let master_id = on_battlefield(&engine, p0, master).expect("Stormscape Master survived");
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

    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(engine.state().players[0].life, 22);
}
