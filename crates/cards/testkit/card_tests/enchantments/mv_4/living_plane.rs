//! `cards/enchantments/mv_4/living_plane.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Living Plane` is a world enchantment costing `{2}{G}{G}` under `Coverage::Implemented`.
/// It prints "All lands are 1/1 creatures that are still lands."
/// While on the battlefield, lands controlled by all players gain the creature type in addition
/// to their land type and have their power and toughness set to 1/1.
#[test]
fn living_plane_turns_all_lands_into_one_one_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[living_plane(), forest()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let p0_land = on_battlefield(&engine, p0, forest()).expect("p0 Forest is present");
    let p1_land = on_battlefield(&engine, p1, mountain()).expect("p1 Mountain is present");

    let chars0 = engine
        .state()
        .object(p0_land)
        .expect("p0 land exists")
        .characteristics();
    assert!(
        chars0.types.contains(TypeSet::LAND) && chars0.types.contains(TypeSet::CREATURE),
        "p0 land is both a land and a creature"
    );
    assert_eq!(
        pt(&engine, p0_land),
        (1, 1),
        "p0 land has 1/1 power and toughness"
    );

    let chars1 = engine
        .state()
        .object(p1_land)
        .expect("p1 land exists")
        .characteristics();
    assert!(
        chars1.types.contains(TypeSet::LAND) && chars1.types.contains(TypeSet::CREATURE),
        "p1 land is both a land and a creature"
    );
    assert_eq!(
        pt(&engine, p1_land),
        (1, 1),
        "p1 land has 1/1 power and toughness"
    );
}
