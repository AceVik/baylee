//! Crimson Acolyte — {1}{W} — Creature — Human Cleric
//! Oracle: Protection from red
//! Oracle: {W}: Target creature gains protection from red until end of turn.

use super::*;
use baylee_core::generated::index;

#[test]
fn crimson_acolyte_has_protection_from_red_and_grants_it_to_target_creature() {
    let acolyte = card_index("969626f0-19d9-47ae-8766-4d59269e389c");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .hand(0, &[acolyte, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, acolyte);
    pass_until(&mut engine, stack_is_empty);

    let acolyte_id = on_battlefield(&engine, p0, acolyte).expect("Crimson Acolyte on battlefield");
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");

    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[(ManaColor::White, 1), (ManaColor::Red, 1)],
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: acolyte_id,
                ability_index: 1,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected target choice for Lightning Bolt");
    };
    assert!(!options.contains(&acolyte_id));
    assert!(!options.contains(&bears_id));
}
