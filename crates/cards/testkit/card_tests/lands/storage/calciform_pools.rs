//! `cards/lands/storage/calciform_pools.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Calciform Pools: `{1}, Remove X storage counters from this land: Add X
/// mana in any combination of {W} and/or {U}.`
///
/// The Mercadian Masques cycle pays a tap and pours one colour; this one
/// pays **mana instead of the tap** and pours a *combination*, and each of
/// those is a thing no test has held yet.
///
/// - **No `{T}` in the cost.** The land is tapped for the whole of this
///   test — it spent its tap banking the counter — and spends them anyway,
///   in the same turn it banked the second one. A cost read as "the
///   announcement plus a tap" would refuse every activation here.
/// - **A pick per mana**, which the rules have no number for: "in any
///   combination" appears nowhere in the Comprehensive Rules, so it is card
///   text and the choices are made while the effect is applied like any
///   other (CR 608.2d). `combination: true` is where that lives here. Two
///   counters ask *twice*, not once, and the two answers may differ — which
///   is the whole difference from Harabaz Druid's "add X mana of any one
///   color", one pick for the whole amount. The assertion that catches a
///   reader confusing them is that the pool ends with one white *and* one
///   blue: one pick for both would make two of whichever was named.
/// - **The options are the card's own two colours**, which is where the
///   reader is struck. `colors_of` hands `ManaSource::Choice` straight back,
///   so `Pending::ChooseColor.options` is the list `landgen` emitted.
#[test]
fn a_storage_land_that_pays_mana_pours_its_counters_into_two_colours() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(911, forest())
        .battlefield(0, &[calciform_pools(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pools = on_battlefield(&engine, p0, calciform_pools()).expect("the land is on the table");

    // Two turns of `{1}, {T}: Put a storage counter on this land.`
    for banked in 1..=2u16 {
        if banked > 1 {
            cross_into_the_next_own_main(&mut engine, p0);
        }
        tap_mana_except(&mut engine, p0, pools);
        store_a_counter(&mut engine, p0, pools, 1);
        assert_eq!(counters_on(&engine, pools, counters::STORAGE), banked);
        assert!(
            is_tapped(&engine, pools),
            "which is what its `{{T}}` pays for"
        );
    }

    // The same turn the second counter was banked in, with the land tapped.
    assert_eq!(
        spend_storage(&mut engine, p0, pools, 2, 2),
        (0, 2),
        "two counters banked, two announceable — and the tap the banking \
         line spent is not part of this cost"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "three Forests paid two {{1}}s, and the third is still floating"
    );

    // Two picks, and they may differ.
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("a combination asks a colour: {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "and the colours offered are the two the card prints, in its order"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("a colour the engine offered");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "`in any combination` is one pick per mana, so the second is \
             still to come: {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![ManaColor::White, ManaColor::Blue]);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("and the second answer need not be the first");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        (
            pool.available(ManaColor::White),
            pool.available(ManaColor::Blue)
        ),
        (1, 1),
        "one of each: a reader that took this for `any one color` would \
         have made two of whichever was named first"
    );
    assert_eq!(counters_on(&engine, pools, counters::STORAGE), 0);
}
