//! `cards/sorceries/mv_2/finale_of_devastation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Finale of Devastation — {X}{G}{G}: "Search your library and/or graveyard
/// for a creature card with mana value X or less and put it onto the
/// battlefield."
///
/// The same `Filter::CmcAtMostX` bound over creatures, and the assertion
/// beside it is the **mana**: {X}{G}{G} at X = 1 is three, and three Forests
/// is exactly what the board holds, so an engine that fetched without
/// charging the announced number would leave one floating where this reads
/// zero.
///
/// Nothing is in the graveyard here, so the search goes straight to the
/// library, and X is 1; the graveyard and the X-is-10 rider are the three
/// tests after this one.
#[test]
fn finale_of_devastation_fetches_a_creature_within_the_announced_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[finale_of_devastation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests and nothing else on the board"
    );
    cast_with_floating(&mut engine, p0, finale_of_devastation());
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("X = 1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    for id in &options {
        assert!(
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves())),
            "a creature card of mana value 1, which is `X or less` for X = 1"
        );
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{G}}{{G}} with X = 1 is three mana, and three is what was made"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"and put it onto the battlefield\""
    );
}

/// "Search your library and/or graveyard": the graveyard is offered first,
/// only the creature cards in it within X — the Llanowar Elves at X = 1 and
/// not the Air Elemental — and a card taken from there is the whole search,
/// so the library is not shuffled.
#[test]
fn finale_of_devastation_takes_a_creature_from_the_graveyard_without_a_shuffle() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(
            0,
            &[finale_of_devastation(), llanowar_elves(), air_elemental()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let buried = in_hand(&engine, p0, llanowar_elves()).unwrap();
    discard_by_hand(&mut engine, p0, &[llanowar_elves(), air_elemental()]);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let journal_from = engine.state().journal.len();

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = cast_finale(&mut engine, p0, 1)
    else {
        unreachable!("cast_finale stops on a card choice")
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::FromGraveyard);
    assert_eq!(options, vec![buried], "within X = 1, and a creature");
    assert_eq!((min, max), (0, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![buried],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&buried),
        "the buried Elves are on the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)),
        &library,
        "the library was not searched, so it keeps its order"
    );
    assert!(
        !engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|e| matches!(e.event, GameEvent::Shuffled { player, .. } if player == p0)),
        "no shuffle"
    );
}

/// Naming nothing from the graveyard searches the library instead, and
/// that search shuffles.
#[test]
fn finale_of_devastation_searches_the_library_when_the_graveyard_is_passed_over() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[finale_of_devastation(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let buried = in_hand(&engine, p0, llanowar_elves()).unwrap();
    discard_by_hand(&mut engine, p0, &[llanowar_elves()]);
    let Pending::ChooseCards { prompt, .. } = cast_finale(&mut engine, p0, 1) else {
        unreachable!("cast_finale stops on a card choice")
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::FromGraveyard);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("the library search, got {:?}", engine.pending())
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert!(options.iter().all(|id| library.contains(id)));
    let journal_from = engine.state().journal.len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&options[0])
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&buried),
        "one card, from one zone"
    );
    assert!(
        engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|e| matches!(e.event, GameEvent::Shuffled { player, .. } if player == p0)),
        "if you search your library this way, shuffle"
    );
}

/// "If X is 10 or more, creatures you control get +X/+X and gain haste until
/// end of turn." At X = 10 the fetched Elves and the Elves already there are
/// 11/11 with haste, and the opponent's creature is not; at X = 9 nothing is
/// pumped.
#[test]
fn finale_of_devastation_at_ten_or_more_pumps_and_hastes_your_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let finale_at = |x: u32| {
        let forests = vec![forest(); x as usize + 2];
        let mut board = forests.clone();
        board.push(llanowar_elves());
        let mut engine = Duel::new(41, llanowar_elves())
            .battlefield(0, &board)
            .hand(0, &[finale_of_devastation()])
            .battlefield(1, &[llanowar_elves()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0));
        let Pending::ChooseCards {
            options, prompt, ..
        } = cast_finale(&mut engine, p0, x)
        else {
            unreachable!("cast_finale stops on a card choice")
        };
        assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
        let fetched = options[0];
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![fetched],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        (engine, fetched)
    };

    let (engine, fetched) = finale_at(10);
    let standing = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    for mine in [fetched, standing] {
        assert_eq!(pt(&engine, mine), (11, 11), "+10/+10");
        assert!(keywords(&engine, mine).contains(KeywordSet::HASTE));
    }
    assert_eq!(pt(&engine, theirs), (1, 1), "creatures *you* control");
    assert!(!keywords(&engine, theirs).contains(KeywordSet::HASTE));

    let (engine, fetched) = finale_at(9);
    assert_eq!(pt(&engine, fetched), (1, 1), "X = 9 is not 10 or more");
    assert!(!keywords(&engine, fetched).contains(KeywordSet::HASTE));
}
