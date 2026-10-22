//! `cards/sorceries/mv_2/superior_numbers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn superior_numbers() -> CardIndex {
    card_index("b98177b5-599f-41b0-aa1e-4769d7005bbd")
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

fn hill_giant() -> CardIndex {
    baylee_core::generated::index::HILL_GIANT
}

/// Superior Numbers (`{G}{G}`, `Coverage::Partial`): deals damage to target
/// creature equal to the number of creatures you control.
#[test]
fn superior_numbers_deals_damage_equal_to_your_creature_count() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                grizzly_bears(),
                llanowar_elves(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[superior_numbers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    cast_from_hand(&mut engine, p0, superior_numbers());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    // 3 creatures controlled by p0 deal 3 lethal damage to 3/3 Hill Giant.
    assert!(on_battlefield(&engine, p1, hill_giant()).is_none());
}
