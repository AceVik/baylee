//! Obsidian Acolyte — {1}{W} — Creature — Human Cleric
//! Oracle: Protection from black
//! Oracle: {W}: Target creature gains protection from black until end of turn.

use super::*;
use baylee_core::generated::index;

#[test]
fn obsidian_acolyte_has_protection_from_black_and_grants_it_to_target_creature() {
    let acolyte = card_index("dd1e7e89-72d7-44d7-922c-f08f0b7d5166");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[acolyte, index::DISFIGURE])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, acolyte);
    pass_until(&mut engine, stack_is_empty);

    let acolyte_id = on_battlefield(&engine, p0, acolyte).expect("Obsidian Acolyte on battlefield");
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");
    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).expect("Hill Giant on board");

    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[(ManaColor::White, 1), (ManaColor::Black, 1)],
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

    cast_with_floating(&mut engine, p0, index::DISFIGURE);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected target choice for Disfigure");
    };
    assert!(!options.contains(&acolyte_id));
    assert!(!options.contains(&bears_id));
    assert!(options.contains(&giant_id));
}
