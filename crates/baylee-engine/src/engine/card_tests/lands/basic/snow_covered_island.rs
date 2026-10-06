//! `cards/lands/basic/snow_covered_island.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Snow-Covered Island prints "Basic Snow Land — Island" and `{T}: Add {U}`.
/// The scenario plays it as a real land drop rather than seeding it with
/// `starting_battlefield`, because a seeded permanent arrives by
/// `Cause::Setup` and no replacement effect ever looks at it — only an actual
/// entry makes "enters untapped" a claim about the card. Two readings then
/// separate it from a plain Island: the projected supertypes carry Basic
/// *and* Snow, and the land is the table's only permanent, so a single tap
/// has to be the single blue mana in the pool.
#[test]
fn snow_covered_island_lands_untapped_as_a_basic_snow_land_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[snow_covered_island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, snow_covered_island());
    assert!(
        !is_tapped(&engine, land),
        "a basic land has nothing standing between the drop and the battlefield"
    );
    let kinds = types(&engine, land);
    assert!(
        kinds.contains(TypeSet::LAND),
        "the type line is Land: {kinds:?}"
    );
    let supertypes = engine
        .state()
        .object(land)
        .expect("the land was just played")
        .characteristics()
        .supertypes;
    assert!(
        supertypes.contains(SupertypeSet::BASIC) && supertypes.contains(SupertypeSet::SNOW),
        "Basic and Snow are both printed, and the Snow is exactly what this \
         card is where a plain Island is not: {supertypes:?}"
    );

    // Its `{T}: Add {U}` is a mana ability (CR 605.1) and needs no mana to
    // be offered, so the route is on the offer while the pool is empty —
    // read before the tap, because the tap is what spends it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        !deeds(&legal, &[land]).is_empty(),
        "an untapped land offers the route its own text prints: {legal:?}"
    );

    // One route taken and one blue in the pool are the same claim read
    // twice, and `total` makes it exact: the colour is printed on the card,
    // not chosen by the seat.
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        1,
        "the land is the table's whole mana base"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "{{U}} was the tap's whole yield"
    );
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(
        is_tapped(&engine, land),
        "and the {{T}} is what paid for it"
    );
}
