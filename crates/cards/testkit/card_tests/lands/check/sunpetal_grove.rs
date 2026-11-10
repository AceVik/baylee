//! `cards/lands/check/sunpetal_grove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunpetal Grove prints a check land's pair: "This land enters tapped unless
/// you control a Forest or a Plains", and "{T}: Add {G} or {W}".
///
/// Three boards hold it to one word each. Alone it comes in tapped and its
/// `{T}` is not even offered, which is the "unless" failing rather than a
/// permanent that merely happens to be down. With the Forest *and* the Plains
/// across the table it is tapped again, so the clause that decides is "you
/// control" and not `Filter::LAND`. And with a Forest of its own controller's
/// it stands up and its one ability is a real two-colour question, answered
/// green out of a pool that was empty before the tap.
#[test]
fn sunpetal_grove_checks_its_controllers_own_forest_or_plains_and_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);

    // Alone: nothing excuses the entrance.
    let mut alone = Duel::new(SEED, forest())
        .hand(0, &[sunpetal_grove()])
        .start();
    keep_mulligans(&mut alone);
    assert!(walk_to_own_main(&mut alone, p0), "p0 reaches its own main");
    let lonely = play_land(&mut alone, p0, sunpetal_grove());
    assert!(
        entered_tapped(&alone, lonely),
        "no Forest and no Plains anywhere: \"enters tapped unless…\" has \
         nothing to be excused by"
    );
    let Pending::Priority { legal, .. } = alone.pending().clone() else {
        panic!(
            "a land drop leaves a quiet priority, got {:?}",
            alone.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == lonely)
            && !legal.mana_abilities.contains(&lonely),
        "and it offers no {{T}} at all — the half of the sentence a board of \
         untapped lands would have hidden: {legal:?}"
    );

    // The same two subtypes, under the other seat's control.
    let mut theirs = Duel::new(SEED, forest())
        .battlefield(1, &[forest(), plains()])
        .hand(0, &[sunpetal_grove()])
        .start();
    keep_mulligans(&mut theirs);
    assert!(walk_to_own_main(&mut theirs, p0), "p0 reaches its own main");
    let neighbourly = play_land(&mut theirs, p0, sunpetal_grove());
    assert!(
        entered_tapped(&theirs, neighbourly),
        "\"unless *you* control\" — a Forest and a Plains across the table are \
         exactly the two subtypes the filter names, and neither is mine"
    );

    // Mine, and then the mana.
    let mut mine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[sunpetal_grove()])
        .start();
    keep_mulligans(&mut mine);
    assert!(walk_to_own_main(&mut mine, p0), "p0 reaches its own main");
    let grove = play_land(&mut mine, p0, sunpetal_grove());
    assert!(
        !entered_tapped(&mine, grove),
        "a Forest I control is the excuse the printing asks for, so this one \
         stands up"
    );
    assert_eq!(
        mine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating: whatever the tap makes came off the tap"
    );

    // Ability 0 is the printed "{T}: Add {G} or {W}" — a mana ability the
    // card prints itself, so it is an `(source, index)` entry and not the
    // CR 305.6 shortcut (this land has no basic land type).
    activate(&mut mine, p0, sunpetal_grove(), 0);
    let Pending::ChooseColor { player, options } = mine.pending().clone() else {
        panic!("\"or\" is a question, got {:?}", mine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both halves of the printing are on the menu: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing else is: {options:?}");

    mine.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &mine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::White), 0, "and not white as well");
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(
        stack_is_empty(&mine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&mine, grove), "the {{T}} is what paid for it");
    let forest = on_battlefield(&mine, p0, forest()).expect("the Forest is still out");
    assert!(
        !is_tapped(&mine, forest),
        "and the Forest beside it never moved, so the green has no other \
         source on this board"
    );
}
