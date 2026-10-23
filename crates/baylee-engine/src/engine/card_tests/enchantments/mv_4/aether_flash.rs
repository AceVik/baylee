//! `cards/enchantments/mv_4/aether_flash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn aether_flash() -> CardIndex {
    card_index("a3c35742-e306-49b6-b042-db4f685c6f86")
}

/// `Aether Flash` ({2}{R}{R}, Enchantment) deals 2 damage to each creature as
/// it enters the battlefield.
#[test]
fn aether_flash_deals_two_damage_to_entering_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[aether_flash(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aether_flash());
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}

/// "Whenever *a* creature enters": 2 damage to it, whoever's it is, and no
/// more: a Hill Giant survives its own entering with exactly 2 marked, the
/// caster's and the opponent's alike.
#[test]
fn aether_flash_marks_two_damage_on_any_creature_that_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                aether_flash(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[HILL_GIANT])
        .hand(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);
    let mine = on_battlefield(&engine, p0, HILL_GIANT).expect("a 3/3 survives 2 damage");
    assert_eq!(lk::damage_on(&engine, mine), 2);

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);
    let theirs = on_battlefield(&engine, p1, HILL_GIANT).expect("the opponent's survives too");
    assert_eq!(
        lk::damage_on(&engine, theirs),
        2,
        "the opponent's creature is hit as well"
    );
}
