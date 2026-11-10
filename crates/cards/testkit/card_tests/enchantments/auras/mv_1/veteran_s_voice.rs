//! `cards/enchantments/auras/mv_1/veteran_s_voice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

#[test]
fn veteran_s_voice_taps_enchanted_creature_to_pump_another_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("68d25ba4-d710-4090-bceb-5cfbd6bb63a1");
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[card])
        .battlefield(0, &[mountain(), grizzly_bears(), HILL_GIANT])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let enemy = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // "Enchant creature you control": the opponent's creature is not offered.
    cast_from_hand(&mut engine, p0, card);
    let aura_targets = aim_at(&mut engine, p0, bear);
    assert!(aura_targets.contains(&giant));
    assert!(
        !aura_targets.contains(&enemy),
        "the Aura enchants a creature its controller controls"
    );
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, card).unwrap();

    // Activate Veteran's Voice: any creature other than the enchanted Bear,
    // the opponent's included.
    activate(&mut engine, p0, card, 1);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&bear),
        "enchanted creature cannot be targeted by Veteran's Voice"
    );
    assert!(options.contains(&giant));
    assert!(options.contains(&enemy));
    aim_at(&mut engine, p0, giant);

    // Choose the enchanted Grizzly Bears to tap as the cost.
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!(
            "expected ChooseCards to tap enchanted creature, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(options, &vec![bear]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, bear));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (5, 4));
    assert_eq!(pt(&engine, enemy), (2, 2), "only the target is pumped");

    // "Activate only if enchanted creature is untapped": it is tapped now.
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.abilities.contains(&(aura, 1)));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: aura,
                    ability_index: 1,
                },
            )
            .is_err()
    );

    // "Until end of turn".
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, giant), (3, 3));
}
