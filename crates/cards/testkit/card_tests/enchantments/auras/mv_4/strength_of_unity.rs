//! `cards/enchantments/auras/mv_4/strength_of_unity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn strength_of_unity() -> CardIndex {
    card_index("00606d5e-a134-4363-9ee6-05e03d58f234")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Domain: enchanted creature gets +1/+1 for each distinct basic land type
/// among lands you control (non-basics/Sol Ring add 0, duplicates count once,
/// opponent's lands ignored).
#[test]
fn strength_of_unity_pumps_by_your_distinct_basic_land_types() {
    let p0 = PlayerId::new(0);
    let mut e = lk::game(
        SEED,
        &[
            grizzly_bears(),
            savannah_lions(),
            sol_ring(),
            plains(),
            plains(),
            forest(),
        ],
        &[swamp(), mountain(), island()],
        &[strength_of_unity()],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let lions = lk::obj(&e, p0, savannah_lions());

    // Plains + Plains + Forest = 2 distinct types (opponent's 3 types ignored).
    lk::float(&mut e, p0, &[(ManaColor::White, 4)]);
    lk::cast_at(&mut e, p0, strength_of_unity(), &[bear], &[]);
    assert_eq!(pt(&e, bear), (4, 4));
    assert_eq!(pt(&e, lions), (2, 1));

    // All 5 basic types across Badlands (Swamp+Mountain) + Island + Plains + Forest -> +5/+5.
    let mut e2 = lk::game(
        SEED,
        &[grizzly_bears(), plains(), island(), badlands(), forest()],
        &[],
        &[strength_of_unity()],
        &[],
    );
    let bear2 = lk::obj(&e2, p0, grizzly_bears());
    lk::float(&mut e2, p0, &[(ManaColor::White, 4)]);
    lk::cast_at(&mut e2, p0, strength_of_unity(), &[bear2], &[]);
    assert_eq!(pt(&e2, bear2), (7, 7));

    // Zero basic land types -> +0/+0.
    let mut e3 = lk::game(
        SEED,
        &[grizzly_bears(), sol_ring()],
        &[plains(), island()],
        &[strength_of_unity()],
        &[],
    );
    let bear3 = lk::obj(&e3, p0, grizzly_bears());
    lk::float(&mut e3, p0, &[(ManaColor::White, 4)]);
    lk::cast_at(&mut e3, p0, strength_of_unity(), &[bear3], &[]);
    assert_eq!(pt(&e3, bear3), (2, 2));
}
