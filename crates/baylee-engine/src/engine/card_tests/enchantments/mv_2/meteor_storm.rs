//! Meteor Storm — {R}{G} — Enchantment
//! Oracle: {2}{R}{G}, Discard two cards at random: This enchantment deals 4 damage to any target.

use super::*;
use baylee_core::generated::index;

#[test]
fn meteor_storm_discards_two_at_random_and_deals_four_damage() {
    let storm = card_index("03f96c23-de0b-4b85-81f8-febc850aa621");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), forest()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[storm, mountain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, storm);
    pass_until(&mut engine, stack_is_empty);

    let storm_id = on_battlefield(&engine, p0, storm).expect("Meteor Storm on battlefield");
    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).expect("Hill Giant on board");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        0
    );

    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: storm_id,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, giant_id);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2
    );
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
}
