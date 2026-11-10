//! `cards/lands/utility/turtle_lair.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Turtle Lair is a land whose three lines all hang off one `{T}`: `{T}: Add
/// {C}`, `{T}: Add one mana of any color. Spend this mana only to cast a Ninja
/// or Turtle spell`, and `{3}, {T}: Target Ninja or Turtle can't be blocked
/// this turn`. It is played here as the turn's land drop, so the untapped
/// permanent the rest of the test reads is a real entry and not a setup
/// placement, and the two mana lines are then measured against a hand that can
/// weigh the rider: Dark Ritual costs `{B}`, is neither a Ninja nor a Turtle,
/// and is uncastable on the restricted black alone while one unrestricted
/// black from the Swamp is enough to pay for it. The `{3}, {T}` line needs a
/// Ninja or Turtle to point at and this board has none, so what is pinned here
/// is the land and the two mana lines.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn turtle_lair_lands_untapped_and_its_any_colour_mana_pays_only_for_a_ninja_or_turtle() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[turtle_lair(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A land, and played as one: out of the hand, as the turn's land drop,
    // and it prints no enters-tapped clause.
    let lair = play_land(&mut engine, p0, turtle_lair());
    assert!(
        types(&engine, lair).contains(TypeSet::LAND),
        "the card is a land: {:?}",
        types(&engine, lair)
    );
    assert!(!is_tapped(&engine, lair), "it enters untapped");
    assert!(
        in_hand(&engine, p0, turtle_lair()).is_none(),
        "and the card the hand gave up is the permanent standing there"
    );

    // Index 1, the second printed line: "Add one mana of any color" is a
    // question, and the five colours of the game are its whole menu —
    // colourless is no colour at all (CR 105.4).
    activate(&mut engine, p0, turtle_lair(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`one mana of any color` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller is the one who names it");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(options.len(), 5, "five colours and no sixth: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana off one tap");
    // `available` reads the plain pool and this land makes none of it: the
    // printed restriction — "spend this mana only to cast a Ninja or Turtle
    // spell" — is what makes it `RestrictedMana`, and a plain read would
    // report a land that produces nothing.
    assert_eq!(
        pool.restricted()
            .iter()
            .filter(|m| m.color == ManaColor::Black)
            .map(|m| u32::from(m.amount))
            .sum::<u32>(),
        1,
        "the colour that was named, and not a default"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, lair), "the {{T}} was its price");

    // "Spend this mana only to cast a Ninja or Turtle spell." Dark Ritual
    // costs {B} and is neither, so the black mana standing in the pool cannot
    // pay for it. The offer is read off the pool: the Swamp beside it is
    // untapped and counts for nothing until it has been tapped.
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&ritual),
        "the pool holds one black and it is restricted to Ninja and Turtle \
         spells: {:?}",
        legal.castable
    );

    // The control, one tap later on the same board: unrestricted black from
    // the Swamp pays {{B}}, so the refusal above was the rider and not a
    // Ritual that was never castable at all.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ritual),
        "one unrestricted black is enough to pay {{B}}: {:?}",
        legal.castable
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the restricted black is still in the pool beside it"
    );

    // The first printed line wants the same {{T}}, so it takes a fresh untap
    // step. The walk is also what shows the Lair comes back: it prints no
    // "doesn't untap" clause.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Lair's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, lair),
        "the untap step gave the Lair back"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 500.5: the pool emptied when the phase it was filled in ended"
    );

    // Index 0: "{T}: Add {C}" — one colourless and nothing to ask, which is
    // the whole difference between the two mana lines.
    activate(&mut engine, p0, turtle_lair(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "a mana ability with nothing to ask hands priority straight back, got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the {{C}} the first line prints"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "and the untapped Swamp beside it put nothing in the pool"
    );
    assert!(is_tapped(&engine, lair), "the {{T}} was its price again");
}

/// Turtle Lair: "{3}, {T}: Target Ninja or Turtle can't be blocked this
/// turn."
#[test]
fn turtle_lair_makes_a_turtle_unblockable() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4206, mountain())
        .battlefield(
            0,
            &[
                turtle_lair(),
                mountain(),
                mountain(),
                mountain(),
                lair_tortoise(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let turtle = on_battlefield(&engine, p0, lair_tortoise()).expect("the Turtle");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    tap_all_mana_but(&mut engine, p0, Some(turtle_lair()));
    assert!(!keywords(&engine, turtle).contains(KeywordSet::UNBLOCKABLE));
    activate(&mut engine, p0, turtle_lair(), 2);
    answer_target(&mut engine, turtle, &[elves]);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, turtle).contains(KeywordSet::UNBLOCKABLE));
    assert!(!keywords(&engine, elves).contains(KeywordSet::UNBLOCKABLE));
}
