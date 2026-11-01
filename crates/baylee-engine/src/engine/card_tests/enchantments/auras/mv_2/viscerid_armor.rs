//! `cards/enchantments/auras/mv_2/viscerid_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn viscerid_armor_pumps_enchanted_creature_and_returns_itself_to_hand() {
    let p0 = PlayerId::new(0);
    let card = card_index("87b4cbec-3521-4bdb-bf01-64ddb526b655");
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[card])
        .battlefield(
            0,
            &[island(), island(), island(), island(), grizzly_bears()],
        )
        .battlefield(1, &[island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let enemy = on_battlefield(&engine, PlayerId::new(1), grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, bear), (2, 2));

    cast_from_hand(&mut engine, p0, card);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (3, 3));
    assert_eq!(
        pt(&engine, enemy),
        (2, 2),
        "only the enchanted creature gets +1/+1"
    );

    // Activate {1}{U}: Return this Aura to its owner's hand.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, card, 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, card).is_none());
    assert!(in_hand(&engine, p0, card).is_some());
    assert_eq!(pt(&engine, bear), (2, 2));
}
