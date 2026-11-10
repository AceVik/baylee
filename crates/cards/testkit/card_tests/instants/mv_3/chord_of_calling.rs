//! `cards/instants/mv_3/chord_of_calling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Chord of Calling — {X}{G}{G}{G} instant with convoke: "Search your library
/// for a creature card with mana value X or less, put it onto the battlefield,
/// then shuffle."
///
/// X is announced and paid for rather than assumed, and that is most of what
/// the scenario measures: four Forests make exactly the four mana of
/// {1}{G}{G}{G}, and the pool is empty by the time the search asks — an engine
/// that charged only {G}{G}{G} would still have one floating. The library is
/// built out of Llanowar Elves, mana value 1, so "a creature card with mana
/// value X or less" is a bound over real cards rather than an empty offer: the
/// search shows creature cards and nothing else, and the one chosen leaves the
/// library and stands on the battlefield.
#[allow(clippy::too_many_lines)] // X announced, paid, then the search: one scenario
#[test]
fn chord_of_calling_announces_x_and_chords_a_creature_of_that_mana_value_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    // The 60-card backing deck is the pool the search reads, so it is a
    // creature card of mana value 1 and not a basic land.
    let mut engine = Duel::new(41, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[chord_of_calling()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Mana before the claim: castability is read off the pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests are four green mana, and nothing else is on the board"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and no creature is — which is also why convoke has nothing to offer"
    );

    cast_with_floating(&mut engine, p0, chord_of_calling());
    // CR 601.2b: X is announced before any cost is paid.
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("a spell with {{X}} asks for X, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster announces the value");
    assert!(
        min <= 1 && 1 <= max,
        "X = 1 is one of the offers: {min}..={max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("the value the question itself enumerated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that searched is the one asked");
    assert_eq!(
        prompt,
        baylee_engine::choice::ChoicePrompt::SearchLibrary,
        "a library search and not a discard, a scry or a cost"
    );
    assert!(
        max >= 1 && min <= 1,
        "up to one card may be found: {min}..={max}"
    );
    assert!(
        options.len() > 1,
        "the backing deck is the library, so the search has cards to show: {}",
        options.len()
    );
    for id in &options {
        assert!(
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves())),
            "a creature card of mana value 1 is `X or less` for X = 1, and the \
             library holds nothing else: {id:?}"
        );
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{G}}{{G}}{{G}} with X = 1 is four mana, and four is what the \
         Forests made: an engine that fetched without charging the X would \
         still have one floating"
    );

    let library_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()).len(),
        1,
        "the found creature card is put onto the battlefield, and it is one \
         card: the Elves drawn into hand are not creatures on the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and it came out of the library it was searched in — a fetch that \
         copied the card would leave this at `library_before`"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "it was put onto the battlefield, not into a graveyard"
    );
}

/// Chord of Calling is convoked with its green creatures: each tapped
/// creature pays a green mana of `{G}{G}{G}` (CR 702.51a), not only generic
/// mana. One Forest and three Grizzly Bears cast it at X = 0, two Bears and
/// the Forest paying `{G}{G}{G}`; before #230 the taps paid generic mana
/// alone, so this board was never offered the spell, and the models that
/// played Chord reported it as uncastable whenever their lands alone could
/// not make the green.
#[test]
fn chord_of_calling_is_convoked_with_green_creatures_for_its_green_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, llanowar_elves())
        .battlefield(
            0,
            &[forest(), grizzly_bears(), grizzly_bears(), grizzly_bears()],
        )
        .hand(0, &[chord_of_calling()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest's green, and the Bears make no mana"
    );
    let chord = in_hand(&engine, p0, chord_of_calling()).expect("in hand");
    assert!(
        matches!(engine.pending(), Pending::Priority { legal, .. } if legal.castable.contains(&chord)),
        "one green floating and three green creatures pay {{G}}{{G}}{{G}}"
    );

    cast_with_floating(&mut engine, p0, chord_of_calling());
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("X is announced first, got {:?}", engine.pending())
    };
    engine
        .apply(p0, PlayerAction::ChooseNumber(0))
        .expect("X = 0 is a value the question offers");
    let Pending::ChooseTargets {
        options,
        max,
        reason,
        ..
    } = engine.pending().clone()
    else {
        panic!("convoke asks for its taps, got {:?}", engine.pending())
    };
    assert_eq!(reason, baylee_engine::choice::TargetPrompt::Convoke);
    assert_eq!(options.len(), 3, "every Bear may be the one tapped");
    assert_eq!(max, 3, "the Bears may pay all three green symbols");
    let tapped: Vec<ObjectId> = options.iter().copied().take(2).collect();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: tapped.clone(),
                players: vec![],
            },
        )
        .expect("two Bears and the Forest's green pay {G}{G}{G}");
    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell reached the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Forest's green paid the third symbol"
    );
    assert!(
        options
            .iter()
            .all(|id| is_tapped(&engine, *id) == tapped.contains(id)),
        "the taps chosen are the taps spent"
    );
}
