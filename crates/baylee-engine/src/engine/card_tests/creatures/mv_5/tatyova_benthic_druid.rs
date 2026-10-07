//! `cards/creatures/mv_5/tatyova_benthic_druid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tatyova, Benthic Druid — {3}{G}{U} 3/3 with Landfall: "Whenever a land you
/// control enters, you gain 1 life and draw a card."
///
/// The order is the test: five lands stand on the battlefield before the Druid
/// resolves and must pay nothing, so the one Forest played *after* her is the
/// only thing the ability can be reading. Exactly one life and exactly one
/// card is the load-bearing number, because she arrives as a creature and the
/// trigger is about lands — a `Trigger::EntersBattlefield` that had lost its
/// filter would have paid for the Druid herself, and one collected per
/// permanent on the board would have paid five times.
///
/// The card drawn is named rather than counted: the card that was on top of the
/// library before the land drop has to be the one in hand afterwards, which is
/// what separates a real draw from the land merely leaving the hand.
#[test]
fn tatyova_pays_one_life_and_one_card_for_the_land_that_enters_after_her() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), island(), island(), island()])
        .hand(0, &[tatyova_benthic_druid(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let library_before = library_size(&engine, p0);

    // {3}{G}{U} off the two Forests and the three Islands. She is cast and not
    // seeded: `starting_battlefield` is a placement rather than an entry, and a
    // landfall trigger reads entries.
    cast_from_hand(&mut engine, p0, tatyova_benthic_druid());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, tatyova_benthic_druid()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the Druid's own arrival is not a land, so nothing has triggered yet"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and she drew nothing for herself"
    );

    // Measured *after* she is cast: she left the hand herself, and what
    // this is about is the land leaving it and a drawn card taking its
    // place.
    let hand_before_land = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // The one land drop this turn, played after her, which is the land the
    // ability is written about.
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");
    let land = play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().players[0].life == life_before + 1
    });

    assert!(
        engine
            .state()
            .object(land)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield),
        "the Forest really is the land that entered"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"you gain 1 life\" — once, for the one land that entered and not for \
         the five that were already there"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the land's controller, not to the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one card, off the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the card that was on top before the land was played"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before_land,
        "the land left the hand and the drawn card took its place"
    );
}
