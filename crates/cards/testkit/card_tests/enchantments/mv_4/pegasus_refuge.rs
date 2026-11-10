//! `cards/enchantments/mv_4/pegasus_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "07462467-e4a3-409e-bcef-9cc92ca4c299"

/// Pegasus Refuge ({3}{W} enchantment) prints one line: "{2}, Discard a card:
/// Create a 1/1 white Pegasus creature token with flying."
///
/// Both halves of that price are the engine's answer rather than the card's,
/// so six Plains are tapped for the cast and read again for the ability: the
/// {2} is a real payment out of a pool those lands actually filled, and the
/// discard is a question the engine has to ask before the cost can be paid
/// (CR 601.2h). The token is claimed only once the stack has emptied, and it
/// is read as a body, a colour, a keyword and a type line — four things a
/// token that merely "arrived" would not tell apart. The hand is counted on
/// both zones, because a discard that emptied the hand without filling a
/// graveyard would satisfy the count alone.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn pegasus_refuge_discards_a_card_for_a_flying_pegasus() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 6])
        // The hand the kit deals is exactly this list, so two Forests are
        // what is left to discard once the Refuge has been cast.
        .hand(0, &[pegasus_refuge(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Plains are the {3}{W} the enchantment costs plus the {2} its ability
    // then charges, both out of one pool inside one main phase (CR 500.5).
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Plains tapped, six white"
    );
    cast_with_floating(&mut engine, p0, pegasus_refuge());
    pass_until(&mut engine, stack_is_empty);

    let refuge = on_battlefield(&engine, p0, pegasus_refuge()).expect("the Refuge resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}}{{W}} is spent and exactly the {{2}} the ability charges is left floating"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the cast took its own card out of hand, and nothing else has moved yet"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — so the claim about the offer is
    // made with the mana already floating, which is where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(refuge, 0)),
        "with {{2}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, pegasus_refuge(), 0);
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
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not a cleanup step: the variant is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options.len(),
        hand_before - 1,
        "every card left in hand is a legal price: {options:?}"
    );
    assert!(
        !options.contains(&refuge),
        "the enchantment is on the battlefield and no longer in hand: {options:?}"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the question offered is a legal answer");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} it charges came out of the pool"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&chosen),
        "a discarded card goes to its owner's graveyard, not merely out of hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 2,
        "exactly one card was given up — the cast's and the discard's, no more"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Pegasus arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Pegasus");
    let printed = engine
        .state()
        .object(tokens[0])
        .expect("the Pegasus is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Pegasus", "the name the card gives it");
    assert_eq!(
        (printed.power, printed.toughness),
        (Some(1), Some(1)),
        "the body the card prints"
    );
    assert!(
        printed.colors.contains(baylee_core::color::Color::White),
        "a 1/1 *white* Pegasus, not merely a 1/1"
    );
    assert!(
        printed.keywords.contains(KeywordSet::FLYING),
        "with flying, which no reading of the body alone shows"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "\"creature token\": {types:?}",
        types = types(&engine, tokens[0])
    );
    assert!(
        on_battlefield(&engine, p0, pegasus_refuge()).is_some(),
        "the price was mana and a card, so the enchantment stays to make another"
    );
}
