//! Cinder Shade — {1}{B}{R} — Creature — Shade
//! Oracle: {B}: This creature gets +1/+1 until end of turn.
//! Oracle: {R}, Sacrifice this creature: It deals damage equal to its power to target creature.

use super::*;
use baylee_core::generated::index;

#[test]
fn cinder_shade_pumps_and_sacrifices_to_deal_damage_equal_to_its_last_known_power() {
    let shade = card_index("9e120284-0fcc-432c-9471-9106eab71025");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), mountain()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[shade])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, shade);
    pass_until(&mut engine, stack_is_empty);
    let shade_id = on_battlefield(&engine, p0, shade).expect("Cinder Shade on battlefield");
    let giant_id =
        on_battlefield(&engine, p1, index::HILL_GIANT).expect("Hill Giant on battlefield");
    assert_eq!(pt(&engine, shade_id), (1, 1));

    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[(ManaColor::Black, 2), (ManaColor::Red, 1)],
    );

    // Pump twice with {B}: 1/1 -> 3/3.
    for _ in 0..2 {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: shade_id,
                    ability_index: 0,
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }
    assert_eq!(pt(&engine, shade_id), (3, 3));

    // Sacrifice with {R} targeting the 3/3 Hill Giant.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shade_id,
                ability_index: 1,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, giant_id);
    assert!(on_battlefield(&engine, p0, shade).is_none());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}
