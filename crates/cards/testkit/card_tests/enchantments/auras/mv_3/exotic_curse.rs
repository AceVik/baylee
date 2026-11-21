//! `cards/enchantments/auras/mv_3/exotic_curse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn exotic_curse() -> CardIndex {
    card_index("344476b1-8710-4c7c-826d-efb7d13ceaa7")
}

/// Domain: enchanted creature gets -1/-1 per distinct basic land type among
/// lands you control (duplicates and opponent's basic land types do not count).
#[test]
fn exotic_curse_shrinks_by_your_distinct_basic_land_types() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[island(), island(), swamp()],
        &[killer_whale(), grizzly_bears(), forest(), plains()],
        &[exotic_curse()],
        &[],
    );
    let whale = lk::obj(&e, p1, killer_whale());
    let bear = lk::obj(&e, p1, grizzly_bears());

    // Two Islands + one Swamp = 2 distinct basic land types (opponent's Forest & Plains ignored).
    lk::float(&mut e, p0, &[(ManaColor::Black, 3)]);
    lk::cast_at(&mut e, p0, exotic_curse(), &[whale], &[]);
    assert_eq!(pt(&e, whale), (1, 3));
    assert_eq!(pt(&e, bear), (2, 2));

    // Dual land (Badlands = Swamp + Mountain) gives -2/-2 -> kills 2/2 via SBA.
    let mut e2 = lk::game(
        SEED,
        &[badlands()],
        &[grizzly_bears(), plains(), forest()],
        &[exotic_curse()],
        &[],
    );
    let bear2 = lk::obj(&e2, p1, grizzly_bears());
    lk::float(&mut e2, p0, &[(ManaColor::Black, 3)]);
    lk::cast_at(&mut e2, p0, exotic_curse(), &[bear2], &[]);
    assert_eq!(lk::zone_of(&e2, bear2), Zone::Graveyard);

    // Zero basic land types among your lands -> -0/-0.
    let mut e3 = lk::game(
        SEED,
        &[sol_ring()],
        &[grizzly_bears(), plains(), forest()],
        &[exotic_curse()],
        &[],
    );
    let bear3 = lk::obj(&e3, p1, grizzly_bears());
    lk::float(&mut e3, p0, &[(ManaColor::Black, 3)]);
    lk::cast_at(&mut e3, p0, exotic_curse(), &[bear3], &[]);
    assert_eq!(pt(&e3, bear3), (2, 2));
}
