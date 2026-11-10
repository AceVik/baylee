//! `cards/lands/manlands/needle_spires.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Needle Spires prints three sentences and each one gates the next: it
/// enters tapped, its `{T}` adds `{R}` or `{W}`, and `{2}{R}{W}` turns the
/// land itself into a 2/1 red and white Elemental with double strike that is
/// still a land. A land lying tapped cannot pay `{T}` (CR 502.1 hands the tap
/// back only in its controller's untap step), so the mana sentence is read a
/// turn later — and the untapped-again check is what keeps that from being
/// taken on faith. The animation costs only mana, and the offer is read off
/// the pool, so the four basics beside it are tapped before the ability is
/// asked for.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn needle_spires_enters_tapped_taps_for_red_or_white_and_animates_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), plains(), plains()])
        .hand(0, &[needle_spires()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    // A real land drop, not a seeded board: `starting_battlefield` places a
    // permanent with `Cause::Setup`, which no replacement effect looks at, so
    // a Spires set up that way would be standing untapped and this line would
    // be measuring the harness instead of the card.
    let spires = play_land(&mut engine, p0, needle_spires());
    assert!(is_tapped(&engine, spires), "\"This land enters tapped\"");

    // The other half of the same sentence: a tapped land has no `{T}` to
    // spend, so its own mana ability is not on the menu at all this turn. The
    // control is the same ability below, offered the moment the land stands.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert!(
        !legal.abilities.contains(&(spires, 0)),
        "a land lying tapped has no {{T}} to pay with: {:?}",
        legal.abilities
    );

    // An untap step is the only thing that gives the tap back, so the reading
    // waits a turn — and says so, because a walk that never happened would
    // leave the mana sentence below green for the wrong reason.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, spires),
        "the untap step stood it back up"
    );

    // Ability 0 is the printed mana ability; ability 1 is the animation.
    activate(&mut engine, p0, needle_spires(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped it names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "{{R}} or {{W}}: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana off one tap"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, spires), "the tap was the cost");

    // `{2}{R}{W}` is read off the pool, so the mana comes first: the Spires is
    // named as the source kept back because it is already tapped and its `{T}`
    // belongs to the line above.
    tap_all_mana_but(&mut engine, p0, Some(needle_spires()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "one white already floating, and four more from the lands beside it"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(
        legal.abilities.contains(&(spires, 1)),
        "{{2}}{{R}}{{W}} is affordable off the floating five, so the animation \
         is on the menu: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, needle_spires(), 1);
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, spires);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "\"… this land becomes a 2/1 red and white Elemental creature …\": \
         {kinds:?}"
    );
    assert!(
        kinds.contains(TypeSet::LAND),
        "\"It's still a land\": {kinds:?}"
    );
    assert_eq!(
        pt(&engine, spires),
        (2, 1),
        "the body the card prints, set rather than added to anything"
    );
    assert!(
        keywords(&engine, spires).contains(KeywordSet::DOUBLE_STRIKE),
        "\"… with double strike\""
    );
}
