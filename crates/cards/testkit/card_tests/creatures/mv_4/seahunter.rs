//! `cards/creatures/mv_4/seahunter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "59e1899f-a9e8-48aa-b3fb-0b7d9fd6859c"

/// Seahunter — {2}{U}{U}, a 2/2 Human Mercenary: "{3}, {T}: Search your
/// library for a Merfolk permanent card, put it onto the battlefield, then
/// shuffle."
///
/// Three words in that sentence each need their own reading, and one board
/// supplies all of them. `{3}` is a real price: `LegalActions::abilities` is
/// filtered through `can_afford`, which reads the *pool* and not the seven
/// untapped Islands, so the line is absent until they are tapped and present
/// the moment the seven are floating. The search is the engine's own
/// `ChoicePrompt::SearchLibrary` and its menu is read back against the
/// library, because "search your library" is a claim about where the cards
/// come from. And `Find::BATTLEFIELD` is the word that tells a tutor from a
/// hand-filler: the card the search offered is a permanent in play
/// afterwards, the library is exactly one shorter, and the hand has not grown.
///
/// The deck is sixty Tishana's Tidebinders — a Merfolk Wizard, and the
/// quietest Merfolk in the pool — so "for a Merfolk permanent card" has
/// something to find, and the board is seven Islands plus the Seahunter
/// itself, since the ability and not the body is the card.
#[test]
#[allow(clippy::too_many_lines)]
fn seahunter_spends_three_and_its_tap_to_put_a_merfolk_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, tishanas_tidebinder())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                seahunter(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, seahunter()).expect("the Seahunter is on the table");

    // `can_afford` reads the pool and not the untapped lands: nothing floats
    // yet, so the {3} is unpayable and the line is not offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(hunter, 0)),
        "{{3}} is not three, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    assert_eq!(
        tap_all_mana(&mut engine, p0),
        7,
        "seven Islands, and nothing else on this board makes mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Islands, seven blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hunter, 0)),
        "with the seven floating the whole price is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, seahunter(), 0);
    assert!(
        is_tapped(&engine, hunter),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "searching is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that activated does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(!options.is_empty(), "the library holds Merfolk to find");
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a card the search offered is a legal answer");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the search resolves and hands the seat a quiet priority back"
    );

    let landed = on_battlefield(&engine, p0, tishanas_tidebinder())
        .expect("\"put it onto the battlefield\": the found card is a permanent in play");
    assert_eq!(
        landed, found,
        "the very card the search offered is the permanent that is in play"
    );
    assert!(
        types(&engine, landed).contains(TypeSet::CREATURE),
        "a Merfolk *permanent* card: a creature, read off the board"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and it went onto the battlefield rather than into a hand — a tutor \
         that put it in hand would satisfy every count above"
    );
}
