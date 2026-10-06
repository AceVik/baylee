//! `cards/creatures/mv_3/endurance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Endurance — {1}{G}{G} — 3/4 Elemental Incarnation with flash and reach,
/// and "Evoke—Exile a green card from your hand", which this pool models as
/// an alternative cost plus the trigger that sacrifices what entered for it.
///
/// The scenario is p0's own end step, because that is the one moment that
/// tells flash apart from the timing every creature already has: the Llanowar
/// Elves in the same hand print no flash and must not be castable while the
/// Elemental is. The evoke cast is then read off the whole table — the Elves
/// leave the hand for *exile* and not the graveyard, the three Forests stay
/// untapped because the printed {1}{G}{G} was never paid, and the Elemental
/// that lands a moment later is eaten by its own enters-evoked trigger, with
/// flash and reach projected onto it while it is there. The unsupported enter
/// trigger asks nothing: no target choice interrupts the walk at all.
#[allow(clippy::too_many_lines)] // flash, the cost menu, the exile it pays with, and the trigger after it
#[test]
fn endurance_flashes_in_on_its_evoke_cost_and_is_sacrificed_by_its_own_trigger() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7719, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[endurance(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // Walk to the end step, where a creature spell is legal only on the
    // strength of its own printed flash.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let elemental = in_hand(&engine, p0, endurance()).expect("Endurance is in hand");
    let fodder = in_hand(&engine, p0, llanowar_elves()).expect("the green card is in hand");
    // The Forests are tapped *before* the cast, because affordability here
    // is read off the mana pool and not off what could still be tapped: with
    // an empty pool the printed {1}{G}{G} is not an option at all, the evoke
    // cost is the only one left, and a wizard with one option asks nothing.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the end step hands p0 priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&elemental),
        "flash: three untapped Forests pay {{1}}{{G}}{{G}}, and a creature \
         may be cast in an end step only when it prints flash: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&fodder),
        "the Elves print no flash, so the end step is not theirs: {:?}",
        legal.castable
    );

    // Cast it for the evoke cost rather than for its mana cost.
    engine
        .apply(p0, PlayerAction::CastSpell { card: elemental })
        .expect("the Elemental may be cast in this end step");
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "evoke is an alternative cost, so the cast asks which one: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat chooses its own cost");
    let evoke = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
        .expect("the evoke cost is offered beside the printed {1}{G}{G}");
    engine
        .apply(p0, PlayerAction::ChooseMode(evoke))
        .expect("the alternative cost may be chosen");

    // The cost names a green *card* in hand; the Forests the seat drew are
    // colourless lands, so the Elves are what the question is about.
    if let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p0, "the cost is paid by the seat paying it");
        assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
        assert!(
            options.contains(&fodder),
            "the Llanowar Elves are the green card in hand: {options:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .expect("the cost is paid with what the question offered");
    }

    // The Elemental itself: the body it prints, with both keyword bits, on
    // the table for as long as its own trigger takes to resolve.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, endurance()).is_some()
    });
    let body = on_battlefield(&engine, p0, endurance()).expect("the Elemental resolved");
    assert_eq!(pt(&engine, body), (3, 4), "the body the card prints");
    let kw = keywords(&engine, body);
    assert!(kw.contains(KeywordSet::FLASH), "flash");
    assert!(kw.contains(KeywordSet::REACH), "reach");

    // "When this creature enters [evoked], sacrifice it."
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, endurance()).is_none() && stack_is_empty(e)
    });
    assert!(
        in_graveyard(&engine, p0, endurance()).is_some(),
        "the enters-evoked trigger sacrifices the Elemental it just made"
    );

    // And the cost that bought it is in exile rather than the graveyard.
    let exile = engine.state().zones.list(ZoneLocation::Exile(p0));
    assert!(
        exile.iter().any(|id| engine
            .state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))),
        "`exile a green card from your hand`: {exile:?}"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "the evoke cost exiles; it does not discard"
    );
    // The three Forests were tapped to make the printed cost *offerable*,
    // and the mana they made is still sitting in the pool: an evoke cast
    // spends the card in exile and nothing else.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "the printed {{1}}{{G}}{{G}} was never paid: evoke is a *different* cost"
    );
}
