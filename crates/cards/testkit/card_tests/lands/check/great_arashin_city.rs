//! `cards/lands/check/great_arashin_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Great Arashin City prints three lines and this scenario plays all three.
/// It enters tapped unless its controller has a Forest or a Plains, so the
/// City is played once beside a Forest — where it must arrive untapped with
/// its `{T}` still free for the activated ability — and once beside an Island,
/// a land this seat controls that is neither named type, where it must arrive
/// tapped and therefore offers nothing until a later turn. The `{1}{B}` is a
/// real price, claimed with the Forest and the Swamp already tapped and paid
/// off with an empty pool; and "exile a **creature** card from **your**
/// graveyard" is a cost question whose menu has to hold my Llanowar Elves and
/// not the same card across the table.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn great_arashin_city_enters_by_its_own_condition_and_exiles_a_creature_card_for_a_spirit() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // The library is nothing but creature cards: `seed_graveyard` moves cards
    // off the top of it, and the activated ability's price is a creature
    // *card*, which a basic land would never be.
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), swamp()])
        .hand(0, &[great_arashin_city()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    let city = play_land(&mut engine, p0, great_arashin_city());
    assert!(
        !entered_tapped(&engine, city),
        "\"enters tapped unless you control a Forest or a Plains\": the Forest \
         is already on the battlefield, so the City arrives untapped"
    );

    // The City prints its own `{T}: Add {B}`, so `tap_all_mana` would spend the
    // very tap the activated ability is about to charge. It is named as the one
    // source kept back, and the two basic lands pay exactly the printed price:
    // the Forest's {G} for the {1} and the Swamp's {B} for the {B}.
    tap_all_mana_but(&mut engine, p0, Some(great_arashin_city()));
    {
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(
            pool.total(),
            2,
            "a Forest and a Swamp, and no mana off the City: {{G}} and {{B}}"
        );
        assert_eq!(
            pool.available(ManaColor::Green),
            1,
            "the Forest's {{G}}, which is what pays the generic {{1}}"
        );
        assert_eq!(pool.available(ManaColor::Black), 1, "and the Swamp's {{B}}");
    }

    let fodder =
        in_graveyard(&engine, p0, llanowar_elves()).expect("my creature card is in my graveyard");
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("and one is in theirs");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(city, 1)),
        "ability 0 is the mana line and ability 1 is the {{1}}{{B}} one, offered \
         now that its mana is in the pool and a creature card is in the yard: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, great_arashin_city(), 1);
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
            "the exile is a cost and the engine asks which card, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostExile,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature card, no more and no fewer"
    );
    assert!(
        options.contains(&fodder),
        "a creature card in my graveyard is the whole of the answer: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"from *your* graveyard\": the Elf across the table is the same card \
         and is not mine to exile: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that one card is the whole menu: {options:?}"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "nothing is exiled while the cost question is still open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("a card the cost question offered is a legal answer");

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "the exiled card left the graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&fodder),
        "and it is in exile, which is where that price sends it"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "while the card the cost did not name never moved"
    );
    assert!(
        is_tapped(&engine, city),
        "{{T}} is the other half of the price, paid by the City itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{B}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "creating a token is no mana ability, so the ability is on the stack"
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
    assert_eq!(spirit.name, "Spirit", "the token the card names");
    assert_eq!(
        (spirit.power, spirit.toughness),
        (Some(1), Some(1)),
        "the printed 1/1 body"
    );
    assert!(
        spirit.colors.contains(baylee_core::color::Color::White),
        "\"a 1/1 white Spirit creature token\""
    );
    let kinds = types(&engine, tokens[0]);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "and it is a creature: {kinds:?}"
    );
    assert!(
        on_battlefield(&engine, p0, great_arashin_city()).is_some(),
        "the price was a tap and no sacrifice, so the City is still standing"
    );

    // The other branch of the printed entry condition needs a board where the
    // Forest is replaced by a land that is neither a Forest nor a Plains: an
    // Island satisfies "a land you control" and must still leave the City down,
    // which a rule that had lost the subtypes would not do.
    let mut second = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[great_arashin_city()])
        .start();
    keep_mulligans(&mut second);
    reach_main_phase(&mut second, p0);
    let late_city = play_land(&mut second, p0, great_arashin_city());
    assert!(
        entered_tapped(&second, late_city),
        "\"unless you control a Forest or a Plains\": an Island is a land this \
         seat controls and neither named type, so the City enters tapped"
    );

    // A land that came in tapped makes no mana until its controller's next
    // untap step, so the printed `{T}: Add {B}` can only be read a turn later.
    reach_their_main_phase(&mut second, p1);
    reach_their_main_phase(&mut second, p0);
    assert!(
        !is_tapped(&second, late_city),
        "the untap step stood the City back up"
    );
    assert_eq!(
        second.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // The Island is named as the printing kept back so that the City's own tap
    // is still there to spend: the {B} that lands afterwards has no other
    // source on this board, since an Island makes {U} and nothing else.
    tap_all_mana_but(&mut second, p0, Some(great_arashin_city()));
    assert_eq!(
        second.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the Island's {{U}}, and no black anywhere yet"
    );
    activate(&mut second, p0, great_arashin_city(), 0);
    assert!(
        matches!(second.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{B}}` is fixed, so nothing is asked on the way (CR 605.1), got {:?}",
        second.pending()
    );
    {
        let pool = &second.state().players[0].mana_pool;
        assert_eq!(pool.available(ManaColor::Black), 1, "\"{{T}}: Add {{B}}\"");
        assert_eq!(
            pool.total(),
            2,
            "one blue from the Island and one black off the City"
        );
    }
    assert!(is_tapped(&second, late_city), "the City paid its own {{T}}");
    assert!(
        stack_is_empty(&second),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
}
