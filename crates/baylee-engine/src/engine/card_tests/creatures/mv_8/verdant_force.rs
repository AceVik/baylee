//! `cards/creatures/mv_8/verdant_force.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Verdant Force — "At the beginning of **each** upkeep, create a 1/1 green
/// Saproling creature token": the opponent's upkeep as well as its
/// controller's. The reader read a phase trigger naming no player as "your
/// upkeep", and the card made a Saproling on one upkeep in two.
#[test]
fn verdant_force_makes_a_saproling_on_every_players_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let force = card_index("7a21ea22-3cd7-4c11-8895-5943c0d93a0d");
    let mut engine = Duel::new(7, forest()).battlefield(0, &[force]).start();
    keep_mulligans(&mut engine);
    let saprolings = |e: &Engine<RegistryLookup>| {
        e.state()
            .battlefield_view()
            .iter()
            .filter(|id| {
                e.state()
                    .object(**id)
                    .is_some_and(|o| o.card.is_none() && o.controller == p0)
            })
            .count()
    };
    reach_main_phase(&mut engine, p0);
    let before = saprolings(&engine);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(saprolings(&engine), before + 1, "the opponent's upkeep");
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(saprolings(&engine), before + 2, "and its controller's");
}

/// Verdant Force costs {5}{G}{G}{G} and prints a 7/7 Elemental whose whole
/// text is a beginning-of-upkeep trigger making a 1/1 green Saproling token.
///
/// Eight Forests are the entire board, so the cast spends every mana source on
/// the table and the pool reads empty the moment the creature lands: the "no
/// token yet" below is therefore a statement about a step trigger rather than
/// an enters-trigger, and not about a board that had a Saproling on it all
/// along. The token is then read on the Force's controller's own upkeep, with
/// the intervening turn walked by the kit's cross-turn primitive, and counted
/// again a turn cycle later — the sentence recurs, which one count alone cannot
/// show.
#[test]
fn verdant_force_makes_a_green_saproling_on_each_of_its_controllers_upkeeps() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[verdant_force()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    cast_from_hand(&mut engine, p0, verdant_force());
    pass_until(&mut engine, stack_is_empty);

    let force = on_battlefield(&engine, p0, verdant_force()).expect("the Force resolved");
    assert_eq!(pt(&engine, force), (7, 7), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{5}}{{G}}{{G}}{{G}} out of exactly eight tapped Forests, and no land \
         on this board makes anything but green"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the token is a step trigger and not an enters-trigger: the Force \
         resolved inside a main phase and has made nothing"
    );

    // The next upkeep that belongs to the Force's controller is a whole turn
    // away. The intervening turn is walked with the kit's own cross-turn
    // primitive, which answers the empty combat declarations on the way.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let tokens = tokens_of(&engine, p0);
    assert!(
        !tokens.is_empty(),
        "the Force stood through the cast and a whole turn, so its upkeep has \
         been and gone without making anything"
    );
    assert!(
        on_battlefield(&engine, p0, verdant_force()).is_some(),
        "the Force is still standing, so the token came off its upkeep and not \
         off its own departure"
    );

    let token = tokens[0];
    let printed = engine
        .state()
        .object(token)
        .expect("the Saproling is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Saproling");
    assert_eq!(
        (printed.power, printed.toughness),
        (Some(1), Some(1)),
        "a 1/1 body"
    );
    assert!(
        printed.colors.contains(baylee_core::color::Color::Green),
        "and a *green* one: {:?}",
        printed.colors
    );
    assert!(
        types(&engine, token).contains(TypeSet::CREATURE),
        "the token is a creature, under the seat whose upkeep made it"
    );

    // "each upkeep": one turn cycle later the same board holds more Saprolings
    // than it did, which a single count cannot show.
    let before = tokens_of(&engine, p0).len();
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let after = tokens_of(&engine, p0).len();
    assert!(
        after > before,
        "the trigger fires again on the next upkeep: {before} token(s) before \
         the turn cycle and {after} after"
    );
}
