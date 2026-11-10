//! `cards/lands/white_lotus_hideout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// White Lotus Hideout is a land printing three mana abilities, and the
/// difference between the second and the third is the whole card: "`{T}`: Add
/// one mana of any color. Spend this mana only to cast a Lesson or Shrine
/// spell" against "`{1}`, `{T}`: Add one mana of any color". All three lines
/// are played in one game on a board whose only other source is a single
/// Forest, so every pool reading has exactly one possible origin — and the
/// card's own word, "only", is read as the two pools it produces: the black
/// the `{1}` line makes is in the pool a spell pays out of, and the black the
/// rider makes is not.
#[test]
#[allow(clippy::too_many_lines)]
fn white_lotus_hideout_plays_all_three_mana_lines_and_restricts_only_the_second() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[white_lotus_hideout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hideout = play_land(&mut engine, p0, white_lotus_hideout());
    assert!(
        !is_tapped(&engine, hideout),
        "a land with no enter modifier arrives standing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land spends nothing and floats nothing"
    );

    // Ability 0 — "{{T}}: Add {{C}}". Its whole price is its own tap, and the
    // colour is printed rather than asked for.
    activate(&mut engine, p0, white_lotus_hideout(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`Add {{C}}` names its mana, so there is nothing to choose: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, hideout), "the tap was the whole price");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );

    // A turn round the table, because the untap step is what gives the land
    // its {{T}} back — three lines and one tap symbol can only be read across
    // more than one turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, hideout),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 2 — "{{1}}, {{T}}: Add one mana of any color", with no rider on
    // it. The Forest is tapped by name and *not* the Hideout: `tap_all_mana`
    // would have pressed both of the land's own `{{T}}` lines (#159), spending
    // the very activation this half of the test is about.
    let plot = on_battlefield(&engine, p0, forest()).expect("the Forest is on the table");
    tap_mana_where(&mut engine, p0, |id| id == plot);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one green off the Forest, and nothing else on this board makes mana"
    );
    assert!(
        !is_tapped(&engine, hideout),
        "and the Hideout is still standing for the {{T}} half of the price"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hideout, 2)),
        "`can_afford` reads the pool, and the {{1}} is floating in it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, white_lotus_hideout(), 2);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and it is in the pool a spell pays out of"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forest's green bought the {{1}}, so the black is all that is left"
    );
    assert_eq!(pool.total(), 1, "one mana, and no third source of it");
    assert!(
        is_tapped(&engine, hideout),
        "and the {{T}} was the other half of the price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, hideout), "the untap step again");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(hideout, 2)),
        "the same board with an empty pool, and the {{1}} line is not offered — \
         which is what made the offer above a payment rather than a label: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(hideout, 1)),
        "while the line whose whole price is its own tap is offered: {:?}",
        legal.abilities
    );

    // Ability 1 — "{{T}}: Add one mana of any color. Spend this mana only to
    // cast a Lesson or Shrine spell." The mana arrives, and the plane it
    // arrives in is the point: the pool a spell pays out of finds none of it.
    activate(&mut engine, p0, white_lotus_hideout(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "\"Spend this mana only to cast a Lesson or Shrine spell\": the plain \
         pool a spell pays out of holds none of it"
    );
    assert!(
        !pool.restricted().is_empty(),
        "and the mana is not gone — it is tracked apart, under the rider the \
         card prints"
    );
    assert!(
        is_tapped(&engine, hideout),
        "the tap was the price either way"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
