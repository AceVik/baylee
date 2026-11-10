//! `cards/lands/gain/mutant_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mutant Town prints three lines — it enters tapped, it gains its controller
/// 1 life as it enters, and it taps for {G} or {U} — and one game reads all
/// three off the same permanent. The land is *played* and not seeded onto the
/// battlefield, because `starting_battlefield` places a permanent with no
/// entry event at all: a land put down that way would arrive untapped
/// whatever it says. The mana line is read one turn later, since a land that
/// arrived tapped has no `{T}` to pay with in its arrival turn — so the pair
/// of readings, no route while tapped and one route after the untap step, is
/// what tells the printed entry rule from a board that never got there.
#[test]
fn mutant_town_enters_tapped_gains_a_life_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[mutant_town()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );
    let town = play_land(&mut engine, p0, mutant_town());
    assert!(
        entered_tapped(&engine, town),
        "\"this land enters tapped\", and it entered by being played"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        life_before.0 + 1,
        "\"when this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before.1,
        "the trigger belongs to the land's controller and not to the table"
    );

    // The tapped half: the same permanent, in the same main phase, offers no
    // `{T}` route while it is down.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "p0 holds priority in its own main phase, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !deeds(&legal, &[town])
            .iter()
            .any(|(_, deed)| matches!(deed, Deed::Ability(_))),
        "a land that is still tapped has no {{T}} to pay with, so its one \
         activated ability is not offered: {:?}",
        deeds(&legal, &[town])
    );

    // One turn further: the untap step stands it back up...
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, town), "the untap step ran");

    // ...and only then is the mana line a route. Nothing is tapped first:
    // `tap_all_mana` presses every ability whose whole price is its own `{T}`
    // (#159), so it would tap the very land this test is about to activate.
    // The pool is empty, which is what makes "one blue and nothing else" a
    // statement about the town rather than about the board.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no mana is floating on the way in"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "p0 holds priority in its own main phase, got {:?}",
            engine.pending()
        )
    };
    let route = deeds(&legal, &[town]);
    assert_eq!(
        route.len(),
        1,
        "the town prints one activated ability: {route:?}"
    );
    let Deed::Ability(index) = route[0].1 else {
        panic!(
            "a printed mana ability is an ordinary (source, index) entry and \
             not the CR 305.6 shortcut: {route:?}"
        )
    };
    activate(&mut engine, p0, mutant_town(), index);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both colours the card prints are offered: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and no other source on the board"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, town), "the town paid its own {{T}}");
}
