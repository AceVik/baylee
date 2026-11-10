//! `cards/creatures/mv_5/meloku_the_clouded_mirror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Meloku the Clouded Mirror is a 2/4 flier whose one printed line is
/// "{1}, Return a land you control to its owner's hand: Create a 1/1 blue
/// Illusion creature token with flying."
///
/// Both halves of that price are the engine's answer rather than the card's,
/// so both are played on one board: seven Islands pay the {4}{U} and leave
/// exactly the {1} floating beside it, and the cost that names no land has to
/// ask which one is being given up — a menu holding every land this seat
/// controls and neither the Wizard that prints the ability nor the Forest
/// across the table. Nothing in the price is a tap symbol, so Meloku is still
/// standing when the land it ate goes back to its owner's hand and the
/// Illusion arrives.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn meloku_the_clouded_mirror_returns_a_land_for_a_blue_illusion_token() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 7])
        .hand(0, &[meloku_the_clouded_mirror()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4}{U} out of seven Islands, which leaves exactly the {1} the ability
    // charges floating beside it: the whole scenario stays inside this one main
    // phase, and CR 500.5 empties a pool only when a step ends.
    cast_from_hand(&mut engine, p0, meloku_the_clouded_mirror());
    pass_until(&mut engine, stack_is_empty);
    let mirror = on_battlefield(&engine, p0, meloku_the_clouded_mirror())
        .expect("Meloku resolved onto the table");
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");
    assert_eq!(pt(&engine, mirror), (2, 4), "the body the card prints");
    assert!(
        keywords(&engine, mirror).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "seven Islands less the {{4}}{{U}} the cast cost is exactly the {{1}} \
         the ability charges and one to spare"
    );
    assert!(
        !is_tapped(&engine, mirror),
        "nothing about this card asks for a tap symbol"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool rather than the untapped lands — which is why the claim is made
    // with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mirror, 0)),
        "with {{1}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let lands = lands_of(&engine, p0);
    assert_eq!(lands.len(), 7, "every Island is still on the battlefield");
    let given_up = lands[0];
    let standing = lands[1];

    activate(&mut engine, p0, meloku_the_clouded_mirror(), 0);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which land goes back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostReturn,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert_eq!(
        options.len(),
        lands.len(),
        "the lands this seat controls are the whole menu, tapped or not — a \
         return takes either (CR 118.3 is the tap's rule, not this one's): {options:?}"
    );
    assert!(
        lands.iter().all(|id| options.contains(id)),
        "every land of mine is on it: {options:?}"
    );
    assert!(
        !options.contains(&mirror),
        "Meloku is a creature and no land, though he is the permanent asking: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"a land *you* control\": the Forest across the table is not mine to \
         give up (CR 701.21a): {options:?}"
    );
    assert_eq!(
        engine
            .state()
            .object(given_up)
            .expect("the land is still an object")
            .zone,
        Zone::Battlefield,
        "CR 601.2h pays last: while the question stands the land is still in play"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![given_up],
            },
        )
        .expect("the land the question offered pays the cost");

    assert_eq!(
        engine
            .state()
            .object(given_up)
            .expect("the returned land is still an object")
            .zone,
        Zone::Hand,
        "\"return a land you control to its owner's hand\" — the card is in a \
         hand, not merely off the battlefield"
    );
    assert_eq!(
        mine(&engine, p0, island(), Zone::Battlefield).len(),
        6,
        "exactly one land was given up: the same spell cannot buy two tokens"
    );
    assert_eq!(
        engine
            .state()
            .object(standing)
            .expect("the land nobody named is still an object")
            .zone,
        Zone::Battlefield,
        "and the Island the answer did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}} it charges came out of the pool"
    );
    assert!(
        !is_tapped(&engine, mirror),
        "the price was a land and a mana: the {{T}} symbol is not on the card, \
         so the Wizard is still standing"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Illusion arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Illusion");
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "the token is the creature the card prints: {:?}",
        types(&engine, tokens[0])
    );
    let illusion = engine
        .state()
        .object(tokens[0])
        .expect("the Illusion is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(illusion.name, "Illusion");
    assert_eq!(
        (illusion.power, illusion.toughness),
        (Some(1), Some(1)),
        "the printed 1/1 body"
    );
    assert!(
        illusion.colors.contains(baylee_core::color::Color::Blue),
        "a 1/1 *blue* Illusion"
    );
    assert!(
        illusion.keywords.contains(KeywordSet::FLYING),
        "with flying"
    );
    assert!(
        on_battlefield(&engine, p0, meloku_the_clouded_mirror()).is_some(),
        "Meloku outlives the land he ate, so the same Wizard can do it again"
    );
}
