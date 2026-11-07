#[allow(clippy::wildcard_imports)]
use super::*;

/// Deadly Rollick: "If you control a commander, you may cast this spell
/// without paying its mana cost. Exile target creature."
///
/// Both printed sentences in one game, because neither is worth much alone:
/// a free cast that exiles nothing is a card that does nothing, and an exile
/// paid for with `{3}{B}` never reads the first line at all.
///
/// The free half is asserted the way Flawless Maneuver's is — through
/// castability with an **empty mana pool**, which is the only pool this
/// engine ever offers a spell out of. The board is the same on both sides of
/// the commander's arrival: four Swamps that cannot pay `{3}{B}` because
/// nothing is floating, first untapped and then spent on the commander
/// itself. So the one thing that changed between the refusal and the offer
/// is Sheoldred standing on the battlefield, and not the mana — an
/// implementation that read potential mana instead would already have been
/// caught by the first assertion.
///
/// The exile half then has to be pointed by hand: "target creature" names
/// every creature at the table, the caster's commander included, so a test
/// that let the engine pick would be proving whatever came first in the
/// list. The Elves are named, and what is asserted is where they end up —
/// exile and not the graveyard, which is the difference between this card
/// and every "destroy" in the pool.
#[test]
fn a_commander_makes_deadly_rollick_free_and_it_exiles_the_creature_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(73, swamp())
        .commander(0, &[sheoldred_the_apocalypse()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[deadly_rollick()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let rollick = in_hand(&engine, p0, deadly_rollick()).expect("the Rollick is in hand");

    // Four untapped Swamps and nothing floating, with the commander still in
    // the command zone: neither reading of the card pays for it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&rollick),
        "no mana in the pool and no commander on the battlefield: nothing pays for it"
    );

    // The commander comes down for its printed {2}{B}{B}, which is exactly
    // what the four Swamps make — so the pool is empty again behind it.
    tap_all_mana(&mut engine, p0);
    let commander = engine
        .state()
        .zones
        .list(ZoneLocation::Command(p0))
        .first()
        .copied()
        .expect("Sheoldred starts in the command zone");
    engine
        .apply(p0, PlayerAction::CastSpell { card: commander })
        .expect("four Swamps pay {2}{B}{B}");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, sheoldred_the_apocalypse()).is_some()
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let sheoldred =
        on_battlefield(&engine, p0, sheoldred_the_apocalypse()).expect("the commander landed");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the opponent's creature");

    // Same empty pool, one commander later.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&rollick),
        "the Swamps are spent and the pool is empty, so the free clause is \
         the only reading left that offers it"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: rollick })
        .expect("\"you may cast this spell without paying its mana cost\"");

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"Exile target creature\" asked for no target — got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "their spell, their target");
    assert!(
        options.contains(&elves),
        "the opponent's creature is a legal target: {options:?}"
    );
    assert!(
        options.contains(&sheoldred),
        "\"target creature\" names every creature at the table, the caster's \
         own commander included: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves are a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(elves).map(|o| o.zone),
        Some(Zone::Exile),
        "\"Exile target creature\""
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "exiled, and so never in the graveyard a destroy would have left it in"
    );
    assert!(
        in_graveyard(&engine, p0, deadly_rollick()).is_some(),
        "a resolved instant goes to its owner's graveyard, free cast or not"
    );
}

/// The land choice follows the draw/discard and is not targeting.
#[test]
fn frantic_search_untaps_chosen_lands_including_a_shrouded_one() {
    let p0 = PlayerId::new(0);
    let (mut engine, arbor, islands) = a_frantic_search_choosing_its_lands();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no cast-time land targets"
    );
    let library_before = library_size(&engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (2, 2));
    assert_eq!(library_size(&engine, p0), library_before - 2);
    assert!(islands.iter().all(|&id| is_tapped(&engine, id)));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options[..2].to_vec(),
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("untap choice")
    };
    assert_eq!((min, max), (0, 3));
    assert!(
        options.contains(&arbor),
        "shroud does not prevent this choice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![arbor, islands[0], islands[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, arbor));
    assert!(!is_tapped(&engine, islands[0]));
    assert!(!is_tapped(&engine, islands[1]));
    assert!(is_tapped(&engine, islands[2]));
    assert!(in_graveyard(&engine, p0, frantic_search()).is_some());
}

/// Mental Misstep: "Counter target spell with mana value 1." The pool's first
/// stack target filtered on mana value (CR 202.3), and a filter is only worth
/// anything if it refuses — so both halves are read off one board, one phase,
/// one pool of floating blue: a {2} artifact spell on the stack must leave the
/// Misstep uncastable (a spell with no legal target is not offered), and the
/// {G} creature spell that follows it must be the only thing the target choice
/// names.
///
/// Holding the two halves on the same board is the point. Asserting only that
/// the card is unoffered would pass just as well if the Misstep were unplayable
/// for a reason of its own — the mana is already floating and the second half
/// casts it off the same pool, so what changed between the two is the stack and
/// nothing else.
#[test]
fn a_two_mana_spell_is_no_target_and_the_one_mana_spell_behind_it_is_countered() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[fellwar_stone(), llanowar_elves()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mental_misstep()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // p0 floats {G}{G}{G} once and spends it over both spells. Once, because
    // the Stone's own mana ability must never be pressed: it would ask which
    // colour an opponent's lands could make, which is a question this test has
    // no business answering.
    tap_all_mana(&mut engine, p0);
    let stone = in_hand(&engine, p0, fellwar_stone()).expect("the Stone is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: stone })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 answers by emptying both Islands into the pool. The mana stays there
    // for the rest of the phase (CR 500.5 empties a pool at the end of a
    // *step*), which is what lets the same board be asked twice.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    tap_all_mana(&mut engine, p1);
    let misstep = in_hand(&engine, p1, mental_misstep()).expect("the Misstep is in hand");

    // Half one. A {2} artifact spell has mana value 2, so the Misstep has no
    // legal target — and with the blue already floating, the filter is the
    // only thing standing in the way.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&misstep),
        "a two-mana spell is no target for `counter target spell with mana value 1`"
    );
    assert!(
        engine
            .apply(p1, PlayerAction::CastSpell { card: misstep })
            .is_err(),
        "and naming it anyway is refused rather than quietly allowed"
    );

    // The Stone resolves; p0 follows it with a one-mana creature off the same
    // pool, so the stack now holds exactly one spell and it is a legal target.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, fellwar_stone()).is_some()
    });
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p0 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("the Elves are in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // Half two. Same hand, same floating blue, a one-mana spell on the stack:
    // now the card is offered, and the Elves are the whole of what it may be
    // pointed at.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&misstep),
        "a one-mana spell is the target this card is printed for"
    );
    engine
        .apply(p1, PlayerAction::CastSpell { card: misstep })
        .unwrap();
    // Mana and life both pay the {U/P} here, so the caster is asked
    // (CR 601.2b); it is paid with its mana, as this test was written for.
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![elves],
        "the one-mana spell, and nothing else that was ever on this stack"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    // It resolves: the Elves never arrive, the Misstep is spent, and the
    // two-mana spell it could not point at is still standing.
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, llanowar_elves()).is_some()
    });
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the countered spell never reached the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, mental_misstep()).is_some(),
        "the Misstep resolved and is in its own controller's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, fellwar_stone()).is_some(),
        "the two-mana spell the Misstep could not point at resolved"
    );
}

/// "Search your library for a card, then shuffle and put that card on top.
/// You lose 2 life."
///
/// Three printed clauses, and each is a different way the card could be
/// wrong. **A card** narrows nothing, so the search has to offer the whole
/// library and not the instants and sorceries a Mystical Tutor would leave of
/// it. **Put that card on top** has to survive the shuffle the same sentence
/// asks for first, which is why the card chosen here is the one at the
/// *bottom*: a search that placed before it shuffled would leave it wherever
/// the shuffle dropped it, and only by coincidence on top. And **you** lose
/// the life — a `PlayerRel` read one seat over would take it off the opponent
/// instead, and nothing else about the board would look any different.
#[test]
fn vampiric_tutor_puts_any_card_on_top_and_takes_the_two_life_from_its_caster() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(23, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[vampiric_tutor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );
    cast_from_hand(&mut engine, p0, vampiric_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    let library_before = library_size(&engine, p0);
    assert_eq!(player, p0, "the caster searches their own library");
    assert_eq!(max, 1, "\"a card\" is one card");
    assert_eq!(
        options.len(),
        library_before,
        "\"a card\" filters nothing: every card in the library is offered",
    );

    // `list(Library)` runs bottom to top, so `options[0]` is the card furthest
    // from where the tutor has to put it.
    let found = options.first().copied().expect("the library is not empty");
    assert_ne!(
        engine
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(found),
        "the card being searched for is not already the one on top",
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(found),
        "\"put that card on top\", after the shuffle the same sentence asks for",
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the find stayed in the library: this tutor puts it on top, not in hand",
    );
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (life_before.0 - 2, life_before.1),
        "\"you lose 2 life\" is paid by the caster and by nobody else",
    );
    assert!(
        in_graveyard(&engine, p0, vampiric_tutor()).is_some(),
        "the instant resolved rather than being left on the stack",
    );
}

/// Worldly Tutor ({G}, instant): "Search your library for a creature card,
/// reveal it, then shuffle and put the card on top."
///
/// Three clauses, and the one that is this card's own is the first. Demonic
/// Tutor searches for "a card" and Mystical Tutor for an instant or sorcery,
/// so a filter that had quietly widened to `Filter::Any` would still pass
/// every other tutor test in this file: the library those are played over is
/// sixty copies of one filler, where every filter offers the same sixty
/// options and none of them can be told apart. This one is therefore played
/// over a library that holds *both* kinds — sixty Forests and exactly one
/// creature card — and the assertion is that the Forests are not on the list.
///
/// The library has to be made that way, because `SeatSpec` builds one out of
/// a single filler and has no field for a seeded card. That is the harness'
/// own dev capability doing one zone over what `seed_graveyard` does.
///
/// The other two clauses are read off the outcome. The shuffle comes
/// **before** the placement, which is what the printed sentence says and what
/// `resolve::resume` does, so the Elf is the top card when the spell has
/// finished rather than a random one; and the reveal is no card data at all —
/// the engine derives it from a search narrower than "a card" ending in a
/// hidden zone — so the journal has to name what was found, exactly once.
#[test]
fn a_creature_tutor_passes_over_sixty_forests_and_leaves_the_elf_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[worldly_tutor(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One creature card among the Forests, put there after the draw step so
    // that nothing but the tutor can find it.
    let drafted = in_hand(&engine, p0, llanowar_elves()).expect("the Elf starts in hand");
    let elf = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            drafted,
            baylee_engine::zone::ZoneLocation::Library(p0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .expect("the harness puts the one creature card into the library");
    let library_before = library_size(&engine, p0);
    assert!(
        library_before >= 60,
        "the Elf is hiding among sixty Forests rather than standing alone in \
         the library: {library_before}"
    );

    cast_from_hand(&mut engine, p0, worldly_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass above waited for exactly this")
    };
    assert_eq!(
        options,
        vec![elf],
        "\"a creature card\": sixty Forests are in the same library and none \
         of them is offered"
    );
    assert_eq!(
        (min, max),
        (0, 1),
        "CR 701.23b permits failing to find a card of a stated quality"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the one card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(elf),
        "\"shuffle and put the card on top\" — the shuffle happens first, so \
         the creature is the card p0 draws next"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "this is not Demonic Tutor: what it found stays in the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "nothing left the library"
    );
    assert!(
        in_graveyard(&engine, p0, worldly_tutor()).is_some(),
        "and the instant itself resolved into the graveyard"
    );

    let shown: Vec<Vec<ObjectId>> = engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match &e.event {
            baylee_engine::event::GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        shown,
        vec![vec![elf]],
        "\"reveal it\": a search this narrow, ending somewhere hidden, shows \
         what it found — and shows only that"
    );
}

/// Dispatch, {W}: "Tap target creature." — played against a board with no
/// artifact at all, so the metalcraft line has nothing to fire on; the test
/// after this one is about that line.
///
/// Reading the card cannot replace this, because every assertion here is
/// about what the *engine* offered and did. "Target creature" is a filter,
/// and the two mistakes it can make are both visible in one target list: the
/// Plains p0 just tapped for the spell is a permanent and is not offered,
/// and both of p1's Elves are, so naming one is a real choice rather than
/// the only legal answer. Then the tap has to be a change: both Elves are
/// read untapped before the cast, and afterwards exactly the named one is
/// tapped. A resolution that tapped everything, or that tapped nothing on a
/// board that had started tapped, passes neither half.
#[test]
fn dispatch_taps_the_creature_it_names_and_leaves_the_one_beside_it_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, plains())
        .battlefield(0, &[plains()])
        .hand(0, &[dispatch()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, plains()).expect("p0's one Plains");
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves were seeded across the table: {elves:?}"
    );
    let (named, bystander) = (elves[0], elves[1]);
    assert!(
        !is_tapped(&engine, named) && !is_tapped(&engine, bystander),
        "both creatures stand untapped, so the tap below is a change and \
         not the state they were seeded in"
    );

    // The one Plains is the whole of p0's mana, so this is exactly {W}.
    cast_from_hand(&mut engine, p0, dispatch());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"Tap target creature\" asked for no target — got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "their spell, their target");
    assert!(
        options.contains(&named) && options.contains(&bystander),
        "both of the opponent's creatures are legal targets, so naming one \
         is a choice: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "\"target creature\" — the Plains that paid for the spell is a \
         permanent and not one of them: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![named],
            },
        )
        .expect("an untapped creature across the table is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, named),
        "\"Tap target creature.\" — the creature the spell named"
    );
    assert!(
        !is_tapped(&engine, bystander),
        "and only that one: the creature beside it was never a target"
    );
    assert!(
        in_graveyard(&engine, p0, dispatch()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// Dispatch's second line: "Metalcraft — If you control three or more
/// artifacts, exile that creature."
///
/// Read on both sides of the number, because "three or more" is the whole
/// sentence: two Sol Rings leave the Elf tapped where it stood, and a third
/// exiles it. The Sol Rings are tapped for mana on the way in, which is
/// deliberate — a tapped artifact is still one you control.
#[test]
fn dispatch_exiles_the_creature_it_tapped_at_three_artifacts_and_not_at_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for artifacts in [2, 3] {
        let mut board = vec![plains()];
        board.extend(std::iter::repeat_n(quiet_artifact(), artifacts));
        let mut engine = Duel::new(97, plains())
            .battlefield(0, &board)
            .hand(0, &[dispatch()])
            .battlefield(1, &[llanowar_elves()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1's Elf");

        cast_from_hand(&mut engine, p0, dispatch());
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
            .expect("the Elf is a legal target");
        pass_until(&mut engine, stack_is_empty);

        let exiled = engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))
            });
        if artifacts == 3 {
            assert!(exiled, "three artifacts: \"exile that creature\"");
            assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
        } else {
            assert!(!exiled, "two artifacts are not metalcraft");
            let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf stayed");
            assert!(is_tapped(&engine, elf), "and it was still tapped");
        }
    }
}

/// Virtue of Knowledge: a permanent entering makes an enter trigger happen
/// twice.
///
/// `ReplacementRule::TriggerMultiplier` is the whole front face, and the only
/// way to see a replacement that multiplies a trigger is to count what the
/// trigger did: Lumra mills four on arrival, so it mills eight here. The
/// Adventure half, Vantress Visions, is played by the tests below.
#[test]
fn virtue_of_knowledge_makes_an_enter_trigger_happen_twice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(411, forest())
        .battlefield(
            0,
            &[
                virtue_of_knowledge(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[lumra_bellow_of_the_woods()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, lumra_bellow_of_the_woods());
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "both copies of the enter trigger resolve"
    );

    assert_eq!(
        library_size(&engine, p0),
        before - 8,
        "\"that ability triggers an additional time\": mill four, twice"
    );
}

/// Vantress Visions copies an **activated** ability and its controller
/// chooses a new target for the copy (CR 707.10c). Ba Sing Se's earthbend 2
/// aims at one Forest; the copy is turned onto another, and both come out of
/// it 2/2 land creatures with haste. The question is CR 115.7d's, one target
/// at a time: the copy's current Forest is not offered (keeping it is naming
/// nothing, `min` 0), the other lands are. The copy was not activated
/// (CR 707.10), so the journal says Ba Sing Se's ability went on the stack
/// once. The card then goes on an adventure in exile (CR 715.3d).
#[test]
fn vantress_visions_copies_an_activated_ability_onto_a_new_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ba_sing_se(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
            ],
        )
        .hand(0, &[virtue_of_knowledge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let city = on_battlefield(&engine, p0, ba_sing_se()).expect("Ba Sing Se");
    let forests = all_on_battlefield(&engine, p0, forest());
    let islands = all_on_battlefield(&engine, p0, island());
    let (first, second) = (forests[0], forests[1]);

    for &land in &forests {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .expect("a Forest taps for {G}");
    }
    activate(&mut engine, p0, ba_sing_se(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![first],
                players: vec![],
            },
        )
        .expect("target land you control");
    let earthbend = top_of_stack(&engine);

    for &land in &islands {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .expect("an Island taps for {U}");
    }
    cast_vantress_visions(&mut engine, p0, earthbend);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the copy's controller chooses");
    assert_eq!((min, max), (0, 1), "keep it, or name one new land");
    assert!(options.contains(&second), "another land you control");
    assert!(!options.contains(&first), "not the one it already targets");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![second],
                players: vec![],
            },
        )
        .expect("the other Forest");
    pass_until(&mut engine, stack_is_empty);

    for land in [first, second] {
        assert!(types(&engine, land).contains(TypeSet::CREATURE));
        assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 2);
        assert_eq!(pt(&engine, land), (2, 2));
        assert!(keywords(&engine, land).contains(KeywordSet::HASTE));
    }
    assert!(
        !types(&engine, forests[2]).contains(TypeSet::CREATURE),
        "two earthbends, two lands"
    );
    assert_eq!(
        put_on_stack_from(&engine, city),
        1,
        "the copy was not activated"
    );
    assert!(
        engine
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Exile(p0))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == virtue_of_knowledge()))),
        "the card is on an adventure in exile"
    );
}

/// Vantress Visions copies a **triggered** ability, and its controller keeps
/// the copy's target (CR 707.10c: "may leave any number of the targets
/// unchanged"). Badgermole Cub's enters trigger earthbends one Forest; the
/// copy earthbends the same one again, so it ends a 2/2 with two counters.
/// The copy is a new object that targets the Forest, so the Forest becomes
/// its target, journalled once as the copy's own (`BecameTarget`); the copy
/// was not triggered, so no second `AbilityTriggered` announces it.
#[test]
fn vantress_visions_copies_a_triggered_ability_that_keeps_its_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), island(), island()])
        .hand(0, &[badgermole_cub(), virtue_of_knowledge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    let islands = all_on_battlefield(&engine, p0, island());
    let land = forests[2];

    for &forest in &forests[..2] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: forest })
            .expect("a Forest taps for {G}");
    }
    cast_with_floating(&mut engine, p0, badgermole_cub());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .expect("target land you control");
    let trigger = top_of_stack(&engine);
    let cub = on_battlefield(&engine, p0, badgermole_cub()).expect("the Cub");

    for &island in &islands {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: island })
            .expect("an Island taps for {U}");
    }
    cast_vantress_visions(&mut engine, p0, trigger);
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
        .expect("naming nothing keeps the target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, land, CounterKind::P1P1),
        2,
        "earthbend 1, twice"
    );
    assert_eq!(pt(&engine, land), (2, 2));
    let became_target: Vec<ObjectId> = engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            baylee_engine::event::GameEvent::BecameTarget { object, target, .. }
                if target == land =>
            {
                Some(object)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        became_target.len(),
        1,
        "the copy targets it once: {became_target:?}"
    );
    assert_ne!(became_target[0], trigger, "and it is the copy that does");
    assert_eq!(
        put_on_stack_from(&engine, cub),
        1,
        "the copy did not trigger"
    );
}

/// Profane Procession: the exile it claims, which is the front face's
/// activated ability.
/// `{3}{W}{B}: Exile target creature. Then if there are three or more cards
/// exiled with this permanent, transform it.`
#[test]
#[allow(clippy::too_many_lines)]
fn profane_procession_exiles_a_creature_for_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(412, plains())
        .battlefield(
            0,
            &[
                profane_procession(),
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[rootbreaker_wurm(), grizzly_bears(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is seated");
    let c2 = on_battlefield(&engine, p1, grizzly_bears()).expect("bears seated");
    let c3 = on_battlefield(&engine, p1, ondu_cleric()).expect("cleric seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, profane_procession(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_none(),
        "the Wurm is off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, rootbreaker_wurm()).is_none(),
        "and exiled rather than destroyed, so it is not in the graveyard"
    );

    // Exile two more creatures so Profane Procession has 3 linked exiled cards and transforms.
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 1);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    engine.refresh_offer();

    activate(&mut engine, p0, profane_procession(), 0);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![c2] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let tomb = on_battlefield(&engine, p0, profane_procession()).unwrap();
    assert_eq!(
        engine.state().object(tomb).unwrap().face_index,
        0,
        "2 exiled cards does not transform Profane Procession yet"
    );

    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 1);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    engine.refresh_offer();
    activate(&mut engine, p0, profane_procession(), 0);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![c3] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(tomb).unwrap().face_index,
        1,
        "3rd exiled card transforms Profane Procession into Tomb of the Dusk Rose"
    );

    // Activate Tomb of the Dusk Rose's {T}: Add one mana of any color (ability 0 on back face).
    activate(&mut engine, p0, profane_procession(), 0);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    // Untap Tomb of the Dusk Rose, add {1}{B}{C} (plus the 1 White just added = {2}{W}{B}), and activate ability 1.
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(tomb)
        .unwrap()
        .status
        .remove(baylee_engine::object::Status::TAPPED);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 1);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    engine.refresh_offer();
    activate(&mut engine, p0, profane_procession(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(wurm).unwrap().zone,
        Zone::Battlefield,
        "Tomb of the Dusk Rose puts the chosen exiled creature onto the battlefield"
    );
    assert_eq!(
        engine.state().object(wurm).unwrap().controller,
        p0,
        "under your control, even though p1 owns it"
    );
}

/// "If you don't, you lose the game" is an effect saying the seat loses
/// (CR 104.3e), and the seat keeps that as its reason. It used to be recorded
/// as a loss to life, which the table is now shown: a player on twenty life
/// told they lost to their life total.
///
/// Both ways of not paying are played. With the mana floating the question is
/// put and declined; with an empty pool it is never put, because a payment
/// the pool cannot cover is one that is not made.
#[test]
fn a_pact_nobody_pays_for_loses_the_game_to_its_own_effect() {
    let p0 = PlayerId::new(0);
    let mut board = vec![island(); 8];
    board.push(forest());
    for has_the_mana in [true, false] {
        let poor_board = [forest()];
        let mut engine = a_pact_owed_by_p0(if has_the_mana { &board } else { &poor_board }, &[]);
        let mut asked = false;
        for _ in 0..600 {
            match engine.pending().clone() {
                Pending::GameOver(_) => break,
                Pending::YesNo { player, .. } => {
                    assert_eq!(player, p0, "the debt is the caster's");
                    asked = true;
                    engine
                        .apply(p0, PlayerAction::YesNo(false))
                        .expect("declining is an answer");
                }
                Pending::Priority { player, .. } => {
                    engine
                        .apply(player, PlayerAction::PassPriority)
                        .expect("passing priority is always legal");
                }
                Pending::ChooseAttackers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                        .unwrap();
                }
                Pending::ChooseBlockers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                        .unwrap();
                }
                other => panic!("unexpected on the way to the deferred upkeep cost: {other:?}"),
            }
        }
        assert!(
            asked,
            "declining a pact must be an explicit choice, even with an empty pool"
        );
        assert_eq!(
            engine.state().players[0].loss,
            Some(baylee_engine::event::LossReason::Effect),
            "has the mana: {has_the_mana}"
        );
        assert!(
            engine.state().players[0].life > 0,
            "and not to its life total"
        );
        assert_eq!(engine.state().players[1].loss, None);
    }
}

/// Convoke taps creatures (CR 702.51a), and an artifact is not one. #229:
/// convoke and waterbend shared one walk over creatures *and* artifacts, so a
/// Darksteel Pendant paid Clever Concealment's `{1}` and was offered in its
/// tap question.
#[test]
fn clever_concealment_is_convoked_by_creatures_and_never_by_an_artifact() {
    assert!(
        concealment_is_offered(&concealment_table(&[
            plains(),
            plains(),
            ondu_cleric(),
            ondu_cleric()
        ])),
        "the control: two Plains and two creatures pay {{2}}{{W}}{{W}}"
    );
    assert!(
        !concealment_is_offered(&concealment_table(&[
            plains(),
            plains(),
            ondu_cleric(),
            darksteel_pendant()
        ])),
        "an artifact's tap was counted toward convoke"
    );

    let seat = PlayerId::new(0);
    let mut engine = concealment_table(&[
        plains(),
        plains(),
        ondu_cleric(),
        ondu_cleric(),
        darksteel_pendant(),
    ]);
    let pendant = on_battlefield(&engine, seat, darksteel_pendant()).expect("on the battlefield");
    let (options, _) =
        concealment_convoke_question(&mut engine).expect("convoke asks for its taps");
    assert!(!options.contains(&pendant), "convoke offered an artifact");
    assert_eq!(options.len(), 2, "both creatures are offered");
}

/// Convoke's question offers what the payment can take: five white creatures
/// are offered for `{2}{W}{W}`, and four may be tapped, a white creature
/// paying a `{W}` as well as a `{1}` (CR 702.51a). Until #230 the payment
/// took generic mana only and the bound was two; #229 asked for up to five,
/// and the taps past what the cost could take paid for nothing.
///
/// Two taps and the two Plains' white pay it; four taps pay all of it, and
/// the Plains' white is still floating.
#[test]
fn clever_concealment_s_convoke_pays_its_white_symbols_too() {
    let seat = PlayerId::new(0);
    let mut board = vec![plains(), plains()];
    board.extend(std::iter::repeat_n(ondu_cleric(), 5));
    for taps in [2_usize, 4] {
        let mut engine = concealment_table(&board);
        let (options, max) =
            concealment_convoke_question(&mut engine).expect("convoke asks for its taps");
        assert_eq!(options.len(), 5, "every creature may be the one tapped");
        assert_eq!(max, 4, "the taps pay {{2}} and both {{W}}, and no more");
        let tapped: Vec<ObjectId> = options.iter().copied().take(taps).collect();
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: tapped.clone(),
                    players: vec![],
                },
            )
            .unwrap_or_else(|e| panic!("{taps} taps and the Plains pay {{2}}{{W}}{{W}}: {e:?}"));
        assert!(
            !engine.state().zones.stack_is_empty(),
            "the spell never reached the stack"
        );
        assert!(
            options
                .iter()
                .all(|id| is_tapped(&engine, *id) == tapped.contains(id)),
            "the taps chosen are the taps spent"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            if taps == 4 { 2 } else { 0 },
            "{taps} taps leave the Plains' white floating only when they paid it all"
        );
    }
}

/// The opponent may put all five in one pile, and the caster may still take
/// the other: "A pile can contain zero or more objects" (CR 700.3d).
/// Everything revealed goes to the graveyard.
#[test]
fn fact_or_fiction_may_take_an_empty_pile() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, revealed) = fact_or_fiction_revealed(43);
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    let Pending::ChoosePile { piles, .. } = engine.pending().clone() else {
        panic!(
            "expected the caster's pile choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(piles, vec![vec![], revealed.clone()]);
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    for card in &revealed {
        assert_eq!(
            engine.state().object(*card).map(|o| o.zone),
            Some(baylee_engine::zone::Zone::Graveyard)
        );
    }
    assert!(engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty());
}

/// Mental Misstep — `{U/P}`: "({U/P} can be paid with either {U} or 2
/// life.) Counter target spell with mana value 1."
///
/// With no blue mana at all, the Phyrexian symbol is paid with 2 life
/// (CR 107.4f): the spell is offered, nothing is asked (only one answer
/// pays), the caster drops from 20 to 18, and the Elves are countered.
/// Before the fix a spell's Phyrexian symbol could be paid only with its
/// colour, so the offer left the Misstep out.
#[test]
fn mental_misstep_is_paid_with_two_life_when_no_blue_mana_is_there() {
    let p0 = PlayerId::new(0);
    let mut engine = misstep_board(20, 0);
    assert!(misstep_is_offered(&engine), "two life pays for {{U/P}}");
    cast_with_floating(&mut engine, p0, mental_misstep());
    aim_misstep_at_the_elves(&mut engine);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "2 life paid the {{U/P}}"
    );
    assert!(on_stack(&engine, mental_misstep()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
}

/// CR 119.4: life can be paid only up to the life total. At 1 life with no
/// blue mana the Misstep cannot be paid, so it is not offered; at exactly 2
/// it is, and the payment takes the caster to 0.
#[test]
fn mental_misstep_needs_two_life_to_pay_its_phyrexian_symbol_with_life() {
    let p0 = PlayerId::new(0);
    let engine = misstep_board(1, 0);
    assert!(!misstep_is_offered(&engine), "1 life cannot pay 2");

    let mut engine = misstep_board(2, 0);
    assert!(misstep_is_offered(&engine));
    cast_with_floating(&mut engine, p0, mental_misstep());
    aim_misstep_at_the_elves(&mut engine);
    assert_eq!(engine.state().players[0].life, 0);
}

/// With blue mana floating and life to spare both answers pay, so the
/// caster is asked (CR 601.2b announces it): yes keeps the Island's mana in
/// the pool and costs 2 life, no spends the mana and keeps the life.
#[test]
fn mental_misstep_asks_life_or_mana_when_both_can_pay() {
    let p0 = PlayerId::new(0);
    for pay_life in [true, false] {
        let mut engine = misstep_board(20, 1);
        assert!(misstep_is_offered(&engine));
        cast_with_floating(&mut engine, p0, mental_misstep());
        let Pending::YesNo { prompt, source, .. } = engine.pending().clone() else {
            panic!(
                "expected the Phyrexian question, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(
            prompt,
            baylee_engine::choice::YesNoPrompt::PayLife { amount: 2 }
        );
        assert_eq!(source.map(|s| s.card), Some(mental_misstep()));
        engine.apply(p0, PlayerAction::YesNo(pay_life)).unwrap();
        aim_misstep_at_the_elves(&mut engine);
        let blue = engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Blue);
        if pay_life {
            assert_eq!((engine.state().players[0].life, blue), (18, 1));
        } else {
            assert_eq!((engine.state().players[0].life, blue), (20, 0));
        }
    }
}
