//! `cards/artifacts/mv_1/skycloud_egg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skycloud Egg prints a single line — "{2}, {T}, Sacrifice this artifact:
/// Add {W}{U}. Draw a card." — and every part of it is somewhere a test could
/// lose it. Two colours out of one price has to be read in the pool as *both*
/// a white and a blue (a card that only added one would still show a full pool
/// of two), the draw has to leave the library rather than merely be announced,
/// and the sacrifice has to take the Egg itself off the battlefield into its
/// owner's graveyard. `tap_all_mana` must leave the Egg standing: its price is
/// the mana *and* the tap *and* the sacrifice, so it is not a route that
/// helper may press (#159) — the count of two says so out loud.
#[test]
fn skycloud_egg_trades_two_mana_and_itself_for_white_blue_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), skycloud_egg()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let egg = on_battlefield(&engine, p0, skycloud_egg()).expect("the Egg is on the table");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // `{2}` is read off the pool and not off the untapped lands, so the Plains
    // are tapped before anything is claimed about the offer. The Egg is not
    // one of the two: its whole price is not its own tap, so the helper does
    // not press it and it is still standing to be sacrificed below.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 2, "the two Plains, and the Egg left alone");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two white floating from the Plains"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "with {{2}} in the pool the Egg's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skycloud_egg(), 0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert!(in_graveyard(&engine, p0, skycloud_egg()).is_some());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        stack_is_empty(&engine),
        "the activated ability has finished resolving"
    );
    assert!(
        on_battlefield(&engine, p0, skycloud_egg()).is_none(),
        "sacrificing itself is part of the price, not a rider"
    );
    assert!(
        in_graveyard(&engine, p0, skycloud_egg()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the {{W}} half of `Add {{W}}{{U}}`"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and the {{U}} half, which a pool of two alone would not have told apart"
    );
    assert_eq!(
        pool.total(),
        2,
        "the {{2}} went out of the pool to pay, so nothing else is in it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so a library that emptied would not satisfy the count above"
    );
}
