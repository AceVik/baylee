//! Armored Guardian — {3}{W}{U} — Creature — Cat Soldier
//! Oracle: {1}{W}{W}: Target creature you control gains protection from the color of your choice until end of turn.
//! Oracle: {1}{U}{U}: This creature gains shroud until end of turn.

use super::*;
use baylee_core::generated::index;

#[test]
fn armored_guardian_grants_protection_from_chosen_color_and_self_shroud() {
    let guardian = card_index("46752289-01e8-4839-8889-49db1efd2e1a");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                island(),
                island(),
                island(),
                island(),
                island(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[guardian, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, guardian);
    pass_until(&mut engine, stack_is_empty);

    let guardian_id = on_battlefield(&engine, p0, guardian).expect("Armored Guardian on board");
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, guardian_id), (2, 5));

    // Float ample white, blue, and red mana to test both activated abilities and protection.
    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[
            (ManaColor::White, 3),
            (ManaColor::Blue, 3),
            (ManaColor::Red, 1),
        ],
    );

    // Ability 0: {1}{W}{W}: Target creature you control gains protection from chosen color until EOT.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: guardian_id,
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

    // Ability 1: {1}{U}{U}: Armored Guardian gains shroud until end of turn.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: guardian_id,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, guardian_id).contains(KeywordSet::SHROUD));

    // Cast Lightning Bolt: neither Grizzly Bears (protection from red) nor Armored Guardian (shroud) is a legal target.
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected target choice for Lightning Bolt");
    };
    assert!(!options.contains(&bears_id));
    assert!(!options.contains(&guardian_id));
}
