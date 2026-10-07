//! `cards/lands/pain/shivan_reef.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shivan Reef — Land: "{T}: Add {C}" and "{T}: Add {U} or {R}. This land
/// deals 1 damage to you."
///
/// Both printed lines are played, because the card *is* the difference between
/// them: the colourless one is a plain mana ability that costs its own tap and
/// nothing else, while the coloured one is the same tap plus a point of damage
/// to the land's own controller — so p0's life is read with p1's as the
/// control. The colour question names exactly the two colours the card prints,
/// which is what separates "{U} or {R}" from "any colour" and from the {C} the
/// line above makes.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn shivan_reef_taps_for_colorless_and_charges_one_life_for_blue_or_red() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    // Two Reefs, one seated and one played: a single land can only be tapped
    // once, and the whole test is both lines of the same card.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[shivan_reef()])
        .hand(0, &[shivan_reef()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Played rather than merely seated: `starting_battlefield` places a
    // permanent without an entry, and a land that could not be tapped the turn
    // it arrived would not be this card.
    let reef = play_land(&mut engine, p0, shivan_reef());
    assert!(
        !entered_tapped(&engine, reef),
        "nothing on the card makes it enter tapped"
    );

    // No basic land type stands behind either line, so both are printed mana
    // abilities: `(source, index)` entries in `abilities`, each of which costs
    // its own tap and no mana, and so is offered on an empty pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(reef, 0)) && legal.abilities.contains(&(reef, 1)),
        "an untapped Reef offers both of its printed lines: {:?}",
        legal.abilities
    );

    // Index 0 — "{T}: Add {C}". No question and no life: the damage belongs to
    // the second line alone.
    activate(&mut engine, p0, shivan_reef(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "one {{C}}");
    assert_eq!(pool.total(), 1, "one tap, one mana, and no other source");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"Add {{C}}\" is not the pain line"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );

    // Index 1 — "{T}: Add {U} or {R}. This land deals 1 damage to you." Two
    // questions belong to the one activation, so they are answered in the order
    // they arrive rather than in the order they are expected.
    let mut offered: Vec<ManaColor> = Vec::new();
    activate(&mut engine, p0, shivan_reef(), 1);
    for _ in 0..8 {
        if at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseColor { player, options } => {
                assert_eq!(player, p0, "the activating seat names the colour");
                offered = options;
                engine
                    .apply(player, PlayerAction::ChooseColor(ManaColor::Blue))
                    .expect("blue was one of the colours offered");
            }
            // "This land deals 1 damage to **you**" names a player rather than
            // asking for one, so the seat is the Reef's controller whichever
            // way the script routes it.
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&p0),
                    "\"you\" is the Reef's controller: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p0],
                        },
                    )
                    .expect("the seat the damage is aimed at");
            }
            other => panic!("unexpected while the Reef pays its coloured line: {other:?}"),
        }
    }
    assert!(at_rest(&engine, p0), "and the coloured line finishes");

    assert_eq!(
        offered,
        vec![ManaColor::Blue, ManaColor::Red],
        "the two colours the card prints — not any colour, and not the {{C}} of \
         the line above"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the {{C}} is still floating: one main phase does not empty a pool \
         (CR 500.5)"
    );
    assert_eq!(pool.total(), 2, "two taps, two mana, and nothing else");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the Reef's controller pays it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the seat across the table pays nothing"
    );
}
