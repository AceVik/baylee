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
/// flash and reach projected onto it while it is there. Decline the optional
/// graveyard target before resolving its sacrifice trigger.
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

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();

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

#[test]
fn endurance_randomizes_only_the_targeted_graveyard_on_the_library_bottom() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[endurance()])
        .hand(
            1,
            &[index::GRIZZLY_BEARS, index::MIND_STONE, index::MOUNTAIN],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mut grave = vec![
        hand_to_graveyard(&mut e, p1, index::GRIZZLY_BEARS),
        hand_to_graveyard(&mut e, p1, index::MIND_STONE),
        hand_to_graveyard(&mut e, p1, index::MOUNTAIN),
    ];
    let library = e.state().zones.list(ZoneLocation::Library(p1)).clone();
    cast_from_hand(&mut e, p0, endurance());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let after = e.state().zones.list(ZoneLocation::Library(p1));
    assert_eq!(&after[3..], &library);
    let mut bottom = after[..3].to_vec();
    bottom.sort_unstable();
    grave.sort_unstable();
    assert_eq!(bottom, grave);
    assert!(e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty());
}

/// p0 casts Endurance for its mana cost with both players' graveyards holding
/// two cards and answers its "up to one target player" with `chosen`. Returns
/// the engine with the trigger resolved, the cards of each graveyard before it
/// and the two libraries before it.
#[allow(clippy::type_complexity)] // a fixture's snapshot, read once by each test
fn endurance_over_two_graveyards(
    chosen: Option<PlayerId>,
) -> (
    Engine<RegistryLookup>,
    Vec<ObjectId>,
    Vec<ObjectId>,
    Vec<ObjectId>,
    Vec<ObjectId>,
) {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[endurance(), index::GRIZZLY_BEARS, index::MIND_STONE])
        .hand(1, &[index::GIANT_GROWTH, index::MOUNTAIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mine = vec![
        hand_to_graveyard(&mut e, p0, index::GRIZZLY_BEARS),
        hand_to_graveyard(&mut e, p0, index::MIND_STONE),
    ];
    let theirs = vec![
        hand_to_graveyard(&mut e, p1, index::GIANT_GROWTH),
        hand_to_graveyard(&mut e, p1, index::MOUNTAIN),
    ];
    let library0 = e.state().zones.list(ZoneLocation::Library(p0)).clone();
    let library1 = e.state().zones.list(ZoneLocation::Library(p1)).clone();
    cast_from_hand(&mut e, p0, endurance());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        min,
        max,
        player_options,
        ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on a target choice")
    };
    assert_eq!((min, max), (0, 1), "\"up to one target player\"");
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "any player, yourself included: {player_options:?}"
    );
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: chosen.into_iter().collect(),
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    (e, mine, theirs, library0, library1)
}

fn sorted(mut cards: Vec<ObjectId>) -> Vec<ObjectId> {
    cards.sort_unstable();
    cards
}

/// "up to one target player": naming nobody leaves every graveyard where it is.
#[test]
fn endurance_with_no_target_chosen_leaves_both_graveyards_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (e, mine, theirs, library0, library1) = endurance_over_two_graveyards(None);
    assert_eq!(
        sorted(e.state().zones.list(ZoneLocation::Graveyard(p0)).clone()),
        sorted(mine),
        "the caster's graveyard is untouched"
    );
    assert_eq!(
        sorted(e.state().zones.list(ZoneLocation::Graveyard(p1)).clone()),
        sorted(theirs),
        "and so is the opponent's"
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Library(p0)), &library0);
    assert_eq!(e.state().zones.list(ZoneLocation::Library(p1)), &library1);
}

/// Targeting the opponent puts THEIR graveyard on the bottom of THEIR library
/// and does not touch the caster's.
#[test]
fn endurance_aimed_at_the_opponent_moves_only_the_opponents_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (e, mine, theirs, library0, library1) = endurance_over_two_graveyards(Some(p1));
    assert!(
        e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty(),
        "the target player's graveyard is emptied"
    );
    let after = e.state().zones.list(ZoneLocation::Library(p1));
    assert_eq!(&after[theirs.len()..], &library1, "on the bottom");
    assert_eq!(sorted(after[..theirs.len()].to_vec()), sorted(theirs));
    assert_eq!(
        sorted(e.state().zones.list(ZoneLocation::Graveyard(p0)).clone()),
        sorted(mine),
        "the caster's graveyard is untouched"
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Library(p0)), &library0);
}

/// Targeting yourself moves your own graveyard and leaves the opponent's.
#[test]
fn endurance_aimed_at_its_caster_moves_only_the_casters_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (e, mine, theirs, library0, library1) = endurance_over_two_graveyards(Some(p0));
    assert!(
        e.state().zones.list(ZoneLocation::Graveyard(p0)).is_empty(),
        "the target player's graveyard is emptied"
    );
    let after = e.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(&after[mine.len()..], &library0, "on the bottom");
    assert_eq!(sorted(after[..mine.len()].to_vec()), sorted(mine));
    assert_eq!(
        sorted(e.state().zones.list(ZoneLocation::Graveyard(p1)).clone()),
        sorted(theirs),
        "the opponent's graveyard is untouched"
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Library(p1)), &library1);
}

/// "Exile a green card from your hand": a card of another colour, or none, is
/// not on the menu, however many of the hand's cards there are.
#[test]
fn endurance_evoke_offers_the_green_cards_of_the_hand_and_nothing_else() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                endurance(),
                llanowar_elves(),
                index::GRIZZLY_BEARS,
                index::LIGHTNING_BOLT,
                index::MIND_STONE,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let elemental = in_hand(&e, p0, endurance()).unwrap();
    let green = sorted(vec![
        in_hand(&e, p0, llanowar_elves()).unwrap(),
        in_hand(&e, p0, index::GRIZZLY_BEARS).unwrap(),
    ]);
    // No mana at all: the only cost left to pay is the evoke cost.
    e.apply(p0, PlayerAction::CastSpell { card: elemental })
        .expect("a green card in hand pays the evoke cost");
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("the cost asks which green card to exile: {:?}", e.pending())
    };
    assert_eq!(
        sorted(options),
        green,
        "only the two green cards: not the red Lightning Bolt, not the \
         colourless Mind Stone, and not the Elemental being cast"
    );
}

/// With no green card in hand, and no mana, the evoke cost cannot be paid: a
/// hand of other colours (and the Elemental itself, which is on the stack and
/// no longer "in hand") offers nothing.
#[test]
fn endurance_cannot_be_evoked_without_another_green_card_in_hand() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[endurance(), index::LIGHTNING_BOLT, index::MIND_STONE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let elemental = in_hand(&e, p0, endurance()).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("priority expected, got {:?}", e.pending())
    };
    assert!(
        !legal.castable.contains(&elemental),
        "no green card besides itself to exile and no mana: {:?}",
        legal.castable
    );
}

/// Only the evoked Elemental is sacrificed: one cast for {1}{G}{G} stays.
#[test]
fn endurance_cast_for_its_mana_cost_is_not_sacrificed() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[endurance()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, endurance());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, p0, endurance()).is_some(),
        "\"when this creature enters [evoked], sacrifice it\" does not apply to a hard cast"
    );
    assert!(in_graveyard(&e, p0, endurance()).is_none());
}
