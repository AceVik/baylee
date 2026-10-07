//! `cards/lands/utility/nivix_aerie_of_the_firemind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nivix, Aerie of the Firemind prints two abilities and only one of them is
/// written: `{T}: Add {C}` is there, while `{2}{U}{R}, {T}: Exile the top
/// card of your library …` is the `Coverage::Partial` gap. The board is two
/// Islands and two Mountains — exactly `{U}{U}{R}{R}`, which pays that cost —
/// and the card itself is left standing, because a second ability skipped for
/// want of mana is indistinguishable from one the DSL cannot write. The four
/// basics make no colourless at all, so the `{C}` in the pool can only have
/// come off the card's own tap, and the exile half is pinned as a non-event:
/// the top of the library stays put and nothing reaches exile.
#[test]
fn nivix_taps_for_colorless_and_never_offers_the_exile_the_dsl_cannot_write() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), mountain(), mountain()])
        .hand(0, &[nivix_aerie_of_the_firemind()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, nivix_aerie_of_the_firemind());
    assert!(
        on_battlefield(&engine, p0, nivix_aerie_of_the_firemind()).is_some(),
        "the land drop put it onto the battlefield"
    );
    assert!(
        !is_tapped(&engine, land),
        "a land enters untapped unless it prints otherwise, so the tap below \
         is the card's own and not the entry"
    );

    let library_before = library_size(&engine, p0);
    let exiled_before = engine.state().zones.list(ZoneLocation::Exile(p0)).len();

    // Mana first: `{2}{U}{R}` is in the pool, so an ability the offer does
    // not carry was not declined for want of mana.
    tap_all_mana_but(&mut engine, p0, Some(nivix_aerie_of_the_firemind()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Islands and two Mountains are exactly {{U}}{{U}}{{R}}{{R}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(
        offered,
        vec![(land, 0)],
        "the card prints two abilities and only the mana one exists — the \
         exile-and-cast line is the `Coverage::Partial` gap"
    );

    // The half that is written.
    activate(&mut engine, p0, nivix_aerie_of_the_firemind(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the {{C}} the card prints, off the card's own {{T}}"
    );
    assert_eq!(pool.total(), 5, "and the four basics made nothing extra");
    assert!(is_tapped(&engine, land), "which tapped it");

    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the gap: no card was exiled off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exiled_before,
        "and the exile zone is the empty one the game started with"
    );
}
