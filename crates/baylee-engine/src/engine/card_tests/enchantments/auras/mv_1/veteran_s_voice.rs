//! `cards/enchantments/auras/mv_1/veteran_s_voice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const VETERAN_S_VOICE: &str = "68d25ba4-d710-4090-bceb-5cfbd6bb63a1";

#[test]
fn veteran_s_voice_taps_enchanted_creature_to_pump_another_creature() {
    let p0 = PlayerId::new(0);
    let card = card_index(VETERAN_S_VOICE);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[card])
        .battlefield(0, &[mountain(), grizzly_bears(), HILL_GIANT])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, card);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    // Activate Veteran's Voice: target must be a creature other than the enchanted Bear.
    activate(&mut engine, p0, card, 1);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&bear),
        "enchanted creature cannot be targeted by Veteran's Voice"
    );
    assert!(options.contains(&giant));
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
}
