//! `cards/creatures/mv_3/wandering_mage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const WANDERING_MAGE: &str = "ac9c81a1-f444-4051-b309-f1af1b5df12a";

#[test]
fn wandering_mage_prevents_damage_across_all_three_abilities() {
    let p0 = PlayerId::new(0);
    let card = card_index(WANDERING_MAGE);
    let mut engine = Duel::new(SEED, plains())
        .hand(
            0,
            &[card, lightning_bolt(), lightning_bolt(), lightning_bolt()],
        )
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                island(),
                island(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                HILL_GIANT,
            ],
        )
        .battlefield(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Wandering Mage ({W}{U}{B}).
    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    let mage = on_battlefield(&engine, p0, card).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    // Ability 0 ({W}, Pay 1 life): Prevent the next 2 damage to target creature (Hill Giant).
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, card, 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        1,
        "2 of Lightning Bolt's 3 damage should be prevented"
    );

    // Ability 1 ({U}): Prevent the next 1 damage to target Cleric or Wizard creature.
    // Hill Giant is neither Cleric nor Wizard; Wandering Mage is both.
    activate(&mut engine, p0, card, 1);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&mage));
    assert!(!options.contains(&giant));
    aim_at(&mut engine, p0, mage);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, mage);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(mage).unwrap().damage,
        2,
        "1 of Lightning Bolt's 3 damage to Wandering Mage should be prevented"
    );

    // Ability 2 ({B}, Put a -1/-1 counter on a creature you control):
    // Prevent the next 2 damage to target player or planeswalker (p0).
    activate(&mut engine, p0, card, 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(giant)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        1
    );

    let bolt3 = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt3 })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "2 of Lightning Bolt's 3 damage to p0 should be prevented (19 -> 18)"
    );
}
