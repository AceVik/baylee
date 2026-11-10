//! `cards/lands/surveil/thundering_falls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thundering Falls — Land — Island Mountain: "This land enters tapped.
/// {T}: Add {U} or {R}. When this land enters, surveil 1."
///
/// The land is *played* rather than seeded, because a permanent placed with
/// `starting_battlefield` is a `Cause::Setup` placement that no entry
/// replacement effect ever looks at — so only a real land drop can show the
/// printed tapped entry. The next turn then reads two things at once — the
/// untap step stands the land back up, so the tapped status belonged to the
/// entry and not to the card, and the printed `{T}` ability offers exactly
/// blue and red, which is the choice the card has to print for itself because
/// two basic land types cannot express "or".
///
/// The surveil is the **keeping** direction here, which is the half that is
/// easy to get wrong and impossible to see: answering "none of them" has to
/// leave the card exactly where it was. A surveil that binned on an empty
/// answer, or that dropped the card out of the library into nothing, would
/// look identical from the battlefield. Raucous Theater above bins.
#[test]
fn thundering_falls_enters_tapped_then_taps_for_blue_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[thundering_falls()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let top_before = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .copied()
        .expect("p0 has a library");
    let falls = play_land(&mut engine, p0, thundering_falls());
    assert!(
        entered_tapped(&engine, falls),
        "\"This land enters tapped\" — and it was played, so the entry \
         modifier is the only thing that could have tapped it"
    );

    let (cards, _) = surveil_offer(&mut engine, p0);
    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("keeping everything is an answer (CR 701.25a: \"any number\")");
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "nothing was put into a graveyard, so nothing left the library"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(top_before),
        "and the card that was looked at is still the top one"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "no Forest of the filler library reached a graveyard"
    );

    // A whole turn cycle, so CR 502.3's untap step is what stands the land
    // back up — the reading above is the entry, not a permanent condition.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !entered_tapped(&engine, falls),
        "no effect holds this land down, so the untap step untapped it"
    );

    // Ability 0 is the printed "{T}: Add {U} or {R}."
    activate(&mut engine, p0, thundering_falls(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "two colours and no third thing: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "\"or {{R}}\" is on the menu with the {{U}} an Island would make: \
         {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "the other half of the choice was not handed over as well"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, falls), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}
