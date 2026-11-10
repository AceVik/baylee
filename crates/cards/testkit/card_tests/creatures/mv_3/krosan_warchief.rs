//! Krosan Warchief — `{2}{G}` 2/2 Creature — Beast (SCG).
//! Oracle: "Beast spells you cast cost {1} less to cast. {1}{G}: Regenerate target Beast."

use super::*;

fn krosan_warchief() -> CardIndex {
    card_index("88350861-244c-4634-9bc9-e9a79401bd02")
}

#[test]
fn krosan_warchief_reduces_beast_spell_cost_and_regenerates_target_beast() {
    let p0 = PlayerId::new(0);
    let warchief = krosan_warchief();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
            ],
        )
        .hand(0, &[warchief, warchief, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();

    // First Krosan Warchief costs full {2}{G} (3 mana).
    tap_mana_where(&mut engine, p0, |id| forests[..3].contains(&id));
    cast_with_floating(&mut engine, p0, warchief);
    pass_until(&mut engine, stack_is_empty);

    // Second Krosan Warchief (a Beast) costs {1} less -> {1}{G} (2 mana).
    tap_mana_where(&mut engine, p0, |id| forests[3..5].contains(&id));
    cast_with_floating(&mut engine, p0, warchief);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let beasts = all_on_battlefield(&engine, p0, warchief);
    assert_eq!(beasts.len(), 2);
    let target_beast = beasts[0];

    // Activate {1}{G} (ability 0) on the first Warchief targeting itself.
    tap_mana_where(&mut engine, p0, |id| forests[5..7].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: target_beast,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target_beast],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Lightning Bolt deals 3 lethal damage to the 2/2 Beast; regeneration saves it.
    tap_mana_where(&mut engine, p0, |id| id == mtn);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target_beast],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(all_on_battlefield(&engine, p0, warchief).len(), 2);
    let obj = engine.state().object(target_beast).unwrap();
    assert!(obj.status.contains(Status::TAPPED));
    assert_eq!(obj.damage, 0);
}
