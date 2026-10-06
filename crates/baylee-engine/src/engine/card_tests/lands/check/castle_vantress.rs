//! `cards/lands/check/castle_vantress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Castle Vantress prints three sentences and one turn of play reads all of
/// them: "This land enters tapped unless you control an Island", "{T}: Add
/// {U}", and "{2}{U}{U}, {T}: Scry 2". The checkland half needs the same land
/// dropped onto two boards that differ in nothing but the type it names —
/// five Islands against five Forests — because "arrived untapped" and "was
/// already untapped" look identical once the permanent is standing there.
/// The scry is then paid for out of the Islands it entered beside, and the
/// tap it leaves behind is the only evidence that its own `{T}` was half the
/// cost rather than a rider on the mana.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn castle_vantress_arrives_untapped_beside_an_island_and_scries_two_for_its_own_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[castle_vantress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let castle = play_land(&mut engine, p0, castle_vantress());
    assert_eq!(
        on_battlefield(&engine, p0, castle_vantress()),
        Some(castle),
        "the land drop is a real entry (CR 305.1), not a setup placement"
    );
    assert!(
        !entered_tapped(&engine, castle),
        "\"enters tapped unless you control an Island\" — five Islands under \
         the same seat are one"
    );

    // Mana first: whether the {2}{U}{U} ability is offered is read off the
    // pool, and the Castle is the source this test presses by hand, so it is
    // the one land `tap_all_mana_but` has to leave standing.
    tap_all_mana_but(&mut engine, p0, Some(castle_vantress()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Islands, and the Castle itself is kept back for its own ability"
    );

    // Ability 0 is the land's printed `{T}: Add {U}`, ability 1 the scry.
    activate(&mut engine, p0, castle_vantress(), 1);
    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the activating seat does the looking");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );
    assert_eq!(
        cards,
        vec![
            *library_before.last().expect("p0 has a library"),
            library_before[library_before.len() - 2],
        ],
        "the top two cards of the library, topmost first"
    );

    let bottomed = cards[0];
    let kept = cards[1];
    engine
        .apply(p0, look_answer(&cards, &[bottomed]))
        .expect("a card the scry itself put on the menu");
    pass_until(&mut engine, stack_is_empty);

    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(library.len(), library_before.len(), "scry draws nothing");
    assert_eq!(
        library.first().copied(),
        Some(bottomed),
        "the card that was chosen is on the bottom of the library"
    );
    assert_eq!(
        library.last().copied(),
        Some(kept),
        "and the one left alone is the new top card"
    );
    assert!(
        is_tapped(&engine, castle),
        "{{T}} is the other half of the scry's cost"
    );

    // The other side of the check: five lands under the same seat, not one of
    // them the type the card names, so this is the printed sentence being
    // read rather than the entry being skipped.
    let mut dry = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[castle_vantress()])
        .start();
    keep_mulligans(&mut dry);
    reach_main_phase(&mut dry, p0);
    let dry_castle = play_land(&mut dry, p0, castle_vantress());
    assert!(
        on_battlefield(&dry, p0, castle_vantress()).is_some(),
        "the same land arrives on the second board too"
    );
    assert!(
        entered_tapped(&dry, dry_castle),
        "\"unless you control an Island\" — a table of Forests is not one, so \
         the land enters tapped"
    );
}
