//! `cards/lands/utility/moorland_haunt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moorland Haunt prints two lines: "{T}: Add {C}" and "{W}{U}, {T}, Exile a
/// creature card from your graveyard: Create a 1/1 white Spirit creature token
/// with flying."
///
/// One board plays both, and each price is read where it lands.
/// `legal.abilities` is filtered through `can_afford`, which reads the mana
/// pool rather than the untapped lands — so the `{W}{U}` is floated *first*,
/// which leaves the empty graveyard as the only thing that can be withholding
/// the second line from the offer. The creature card seeded into the
/// opponent's graveyard is then the control for "your graveyard", and the mana
/// line has to wait a turn, because both printed lines cost the same `{T}`.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn moorland_haunt_taps_for_colorless_and_exiles_a_creature_card_for_a_spirit() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The library is the deck the graveyard is seeded out of, so its filler
    // has to be a *creature card*: a basic land would leave the exile price
    // unpayable for ever and this test could never reach the Spirit at all.
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[moorland_haunt(), island(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let haunt = on_battlefield(&engine, p0, moorland_haunt()).expect("the Haunt is on the table");
    assert!(!is_tapped(&engine, haunt), "a land enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before a land is tapped"
    );

    // The {W}{U} is floated before anything is claimed about the offer, and
    // the Haunt is named as the printing kept back: it prints its own
    // `{T}: Add {C}`, so `tap_all_mana` would have spent the very activation
    // the second line is about (#159).
    tap_all_mana_but(&mut engine, p0, Some(moorland_haunt()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "one Plains, one white");
    assert_eq!(pool.available(ManaColor::Blue), 1, "one Island, one blue");
    assert_eq!(
        pool.total(),
        2,
        "two lands tapped and nothing else on the board makes mana"
    );

    // With the {W}{U} already in the pool, affordability can no longer be the
    // reason the second line is missing: `can_afford` refuses the exile price
    // the way it refuses a mana cost it cannot pay.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(haunt, 0)),
        "the {{T}}: Add {{C}} line is paid by its own tap and is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(haunt, 1)),
        "no creature card in the graveyard, so the exile cannot be paid and \
         the second line is absent from the offer rather than refused: {:?}",
        legal.abilities
    );

    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let fodder = in_graveyard(&engine, p0, llanowar_elves()).expect("p0's graveyard was seeded");
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("p1's graveyard was seeded");
    assert_ne!(fodder, theirs, "two cards of the same printing, two owners");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(haunt, 1)),
        "a creature card in the graveyard and the {{W}}{{U}} floating: the \
         whole printed price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, moorland_haunt(), 1);
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
            "the exile is a cost and the engine asks which card pays it, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostExile,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature card, and the cost asks once"
    );
    assert_eq!(
        options,
        vec![fodder],
        "the creature card in *your* graveyard is the whole menu — the same \
         printing across the table is no price of yours"
    );
    assert!(
        !options.contains(&theirs),
        "a seat exiles from its own graveyard, whatever the filter says"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered pays the cost");

    assert!(is_tapped(&engine, haunt), "{{T}} is part of the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{W}}{{U}} it charges came out of the pool"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&fodder),
        "the exiled card left its graveyard for its owner's exile"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "and it is no longer in the graveyard it was paid from"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Spirit arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Spirit");
    let spirit = engine
        .state()
        .object(tokens[0])
        .expect("the Spirit is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(spirit.name, "Spirit", "the name the card gives it");
    assert_eq!(
        (spirit.power, spirit.toughness),
        (Some(1), Some(1)),
        "the 1/1 body the card prints"
    );
    assert!(
        spirit.keywords.contains(KeywordSet::FLYING),
        "the printed \"with flying\" reaches the token"
    );
    assert!(
        spirit.colors.contains(baylee_core::color::Color::White),
        "\"white\" is the first word of the token's type line"
    );
    assert!(
        on_battlefield(&engine, p0, moorland_haunt()).is_some(),
        "the land paid its tap and is still on the battlefield"
    );

    // The other printed line wants an untapped land, and the untap step is the
    // only thing that gives it one back: both lines cost the same `{T}`.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, haunt),
        "the untap step stood the Haunt back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    activate(&mut engine, p0, moorland_haunt(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, haunt), "the Haunt paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — no basic land type on this board could have made it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
