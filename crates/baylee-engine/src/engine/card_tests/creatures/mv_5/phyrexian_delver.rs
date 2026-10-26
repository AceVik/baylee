//! Phyrexian Delver — {3}{B}{B} — Creature — Phyrexian Zombie
//! Oracle: When this creature enters, return target creature card from your graveyard to the battlefield. You lose life equal to that card's mana value.

use super::*;
use crate::engine::card_tests::legends_kit::life;

#[test]
fn phyrexian_delver_returns_creature_from_graveyard_and_loses_life_equal_to_cmc() {
    let delver = card_index("a13cbac0-4c76-4970-b61e-5f4e020ee95c");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), swamp(), serra_angel()],
        )
        .hand(0, &[delver])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let angel_id = on_battlefield(&engine, p0, serra_angel()).unwrap();
    bury(&mut engine, &[angel_id]);
    assert!(in_graveyard(&engine, p0, serra_angel()).is_some());

    cast_from_hand(&mut engine, p0, delver);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let dead = in_graveyard(&engine, p0, serra_angel()).unwrap();
    aim_at(&mut engine, p0, dead);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, delver).is_some());
    assert!(on_battlefield(&engine, p0, serra_angel()).is_some());
    // Serra Angel has mana value 5, so p0 loses 5 life (20 -> 15).
    assert_eq!(life(&engine, p0), 15);
}
