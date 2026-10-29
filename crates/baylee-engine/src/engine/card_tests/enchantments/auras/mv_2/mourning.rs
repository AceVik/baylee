//! Mourning — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets -2/-0.
//! Oracle: {B}: Return this Aura to its owner's hand.

use super::*;
use baylee_core::generated::index;

#[test]
fn mourning_shrinks_enchanted_creature_and_returns_to_hand() {
    let mourning = card_index("20602d11-c0ae-47ce-a7fd-998b12480e18");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), index::GRIZZLY_BEARS])
        .hand(0, &[mourning])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears_id =
        on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears on board");
    cast_from_hand(&mut engine, p0, mourning);
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    let mourning_id = on_battlefield(&engine, p0, mourning).expect("Mourning on battlefield");
    assert_eq!(pt(&engine, bears_id), (0, 2));

    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Black, 1)]);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mourning_id,
                ability_index: 2,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mourning).is_none());
    assert!(in_hand(&engine, p0, mourning).is_some());
    assert_eq!(pt(&engine, bears_id), (2, 2));
}
