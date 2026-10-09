//! Instants, the door `cards/instants/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use baylee_core::color::{Color, ColorSet};

mod legends;
mod mv_0;
mod mv_1;
mod mv_2;
mod mv_3;
mod mv_4;
mod mv_5;
mod mv_6;
mod mv_7;
mod mv_8;
mod mv_9;

// oracle_id = "1c747fe2-289e-492a-a846-aa77707e2dc3"
fn abrupt_decay() -> baylee_core::ids::CardIndex {
    card_index("1c747fe2-289e-492a-a846-aa77707e2dc3")
}

// oracle_id = "464c0150-3dbc-403b-9ada-fef25ab1f29d"
fn brain_freeze() -> baylee_core::ids::CardIndex {
    card_index("464c0150-3dbc-403b-9ada-fef25ab1f29d")
}

// oracle_id = "0456ec64-2c81-4763-a352-8ff64a4c3d6b"
fn deadly_rollick() -> baylee_core::ids::CardIndex {
    card_index("0456ec64-2c81-4763-a352-8ff64a4c3d6b")
}

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

// oracle_id = "86bf58f2-7f25-4e10-b797-25e0e8e67769"
fn flusterstorm() -> baylee_core::ids::CardIndex {
    card_index("86bf58f2-7f25-4e10-b797-25e0e8e67769")
}

// oracle_id = "16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c"
fn frantic_search() -> baylee_core::ids::CardIndex {
    card_index("16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c")
}

/// The pool's one land that is also a creature, and therefore the only
/// permanent an Equipment can hand a keyword to that still answers to
/// `Filter::LAND`.
fn dryad_arbor() -> baylee_core::ids::CardIndex {
    card_index("e996cd67-739c-40f4-b276-0042acf26c71")
}

/// Frantic Search cast off exactly the three Islands that pay for it, with
/// Dryad Arbor standing beside them under Lightning Greaves — a *land* with
/// shroud. Answers `(engine, arbor, islands)` on the spell's own target
/// choice.
///
/// The three Islands are what makes the untap the point of the card: they
/// are the whole mana the spell cost, so a resolution that reaches its last
/// effect hands all of it back. The Arbor is deliberately left untapped and
/// out of the payment, so which land the engine spends is not a thing this
/// test can be wrong about.
fn a_frantic_search_choosing_its_lands() -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(77, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                dryad_arbor(),
                lightning_greaves(),
            ],
        )
        .hand(0, &[frantic_search()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Equip {0}, which is how a land comes to have shroud at all.
    let arbor = on_battlefield(&engine, p0, dryad_arbor()).expect("the Arbor is on the table");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves too");
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![arbor], "the only creature p0 controls");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![arbor],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(arbor))
    });
    assert!(
        keywords(&engine, arbor).contains(KeywordSet::SHROUD),
        "the Greaves grant shroud to what they are attached to"
    );
    assert!(
        types(&engine, arbor).contains(TypeSet::LAND),
        "and the thing they are attached to is still a land"
    );

    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 3, "the rest of the board is three Islands");
    tap_all_mana_but(&mut engine, p0, Some(dryad_arbor()));
    let spell = in_hand(&engine, p0, frantic_search()).expect("the Search is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("three Islands pay {2}{U}");
    (engine, arbor, islands)
}

/// Frantic Search: "Draw two cards, then discard two cards. Untap up to
/// three lands."
///
/// Both halves of the card's `Coverage::Partial` are decided by one cast,
/// so they are asserted on one board.
///
/// The half that works is the one the card file asks for by name. This is
/// the first spell whose effect list *suspends* mid-list: `draw(2)` runs,
/// `DiscardForPlayers` stops the resolution on a `ChooseCards`, and the
/// `UntapTarget` behind it happens only when that answer comes back
/// (`Resolution::pc` picks the list up where it left off). So the three
/// Islands are read twice — still tapped while the discard is being asked,
/// untapped once it has been answered. A resolution that dropped its own
/// tail would leave all three tapped and still pass a test that only
/// looked in the graveyard.
///
/// The half that does not work is the `// NOT SUPPORTED:` comment. The
/// printed card names no target, so on paper the lands are picked as it
/// resolves and nothing about them has to be targetable; the DSL says "up
/// to three" with a `TargetReq`, so they are named as the spell is cast
/// (CR 601.2c) and a land with shroud (CR 702.18a) is not offered at all.
/// Dryad Arbor under Lightning Greaves is that land — on the battlefield,
/// a land, and missing from the choice.
#[test]
fn frantic_search_untaps_the_lands_that_paid_for_it_and_cannot_reach_a_shrouded_one() {
    let p0 = PlayerId::new(0);
    let (mut engine, arbor, islands) = a_frantic_search_choosing_its_lands();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the lands are a `TargetReq`, so they are chosen as the spell \
             is cast — got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (0, 3), "\"up to three lands\"");
    assert!(
        !options.contains(&arbor),
        "NOT SUPPORTED, and this is its exact shape: the printed card \
         targets nothing and would untap the shrouded Arbor, but \"up to \
         three\" is spelled as targets, so shroud keeps it off the list"
    );
    assert!(
        islands.iter().all(|l| options.contains(l)),
        "an Island is a land nothing protects, so all three are offered"
    );

    let library_before = library_size(&engine, p0);
    assert!(
        islands.iter().all(|&l| is_tapped(&engine, l)),
        "all three Islands paid for the spell"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: islands.clone(),
            },
        )
        .expect("three lands is what \"up to three\" allows");

    // The resolution stops here, halfway down its own effect list.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on the discard")
    };
    assert_eq!(player, p0, "\"discard two cards\" — the caster's own hand");
    assert_eq!((min, max), (2, 2), "two, and not a choice of how many");
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"draw two cards\" is the effect before this one, and it ran"
    );
    assert!(
        islands.iter().all(|&l| is_tapped(&engine, l)),
        "the untap is a later effect of the same resolution: with the \
         discard still being asked, the three lands are still tapped"
    );

    let discarded: Vec<ObjectId> = options.iter().copied().take(2).collect();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: discarded.clone(),
            },
        )
        .expect("two of the cards the discard itself offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        discarded
            .iter()
            .all(|c| engine.state().object(*c).map(|o| o.zone) == Some(Zone::Graveyard)),
        "both discarded cards are in their owner's graveyard"
    );
    assert!(
        islands.iter().all(|&l| !is_tapped(&engine, l)),
        "\"Untap up to three lands\": the resolution came back from the \
         discard and finished its effect list, so the mana that paid for \
         the spell is standing again"
    );
    assert!(
        in_graveyard(&engine, p0, frantic_search()).is_some(),
        "and the instant itself is in the graveyard"
    );
}

// oracle_id = "1a0770e6-b093-4439-baff-6889a50ba12e"
fn mental_misstep() -> baylee_core::ids::CardIndex {
    card_index("1a0770e6-b093-4439-baff-6889a50ba12e")
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

// oracle_id = "ededbdae-d9dc-4206-9335-d7158f2d7700"
fn vampiric_tutor() -> baylee_core::ids::CardIndex {
    card_index("ededbdae-d9dc-4206-9335-d7158f2d7700")
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
            .list(crate::zone::ZoneLocation::Library(p0))
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
            .list(crate::zone::ZoneLocation::Library(p0))
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

// oracle_id = "e8863518-0bfa-49c3-8c6e-6c9116a81051"
fn worldly_tutor() -> baylee_core::ids::CardIndex {
    card_index("e8863518-0bfa-49c3-8c6e-6c9116a81051")
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
            crate::zone::ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
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
        (1, 1),
        "one creature card, and the search is not optional"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the one card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
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
            crate::event::GameEvent::Revealed { cards, .. } => Some(cards.clone()),
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

// oracle_id = "5b5bf1fa-6502-4790-b66b-f0f8504ebc7c"
fn cabal_ritual() -> CardIndex {
    card_index("5b5bf1fa-6502-4790-b66b-f0f8504ebc7c")
}

// oracle_id = "133c99c0-3652-410f-8100-68015a47af9f"
fn dispatch() -> baylee_core::ids::CardIndex {
    card_index("133c99c0-3652-410f-8100-68015a47af9f")
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

// oracle_id = "ce19962d-94f9-4b2b-b668-963c0acce308"
fn borne_upon_a_wind() -> CardIndex {
    card_index("ce19962d-94f9-4b2b-b668-963c0acce308")
}

fn deflecting_swat() -> CardIndex {
    card_index("ae120613-97d6-4393-b39d-c3e6c076f5d6")
}

/// `seat` casts Deflecting Swat for its printed `{2}{R}` at `target`, a spell
/// or ability on the stack, and passes until it resolves. Returns its
/// new-target question, or `None` when it asks none.
#[track_caller]
fn swatted(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    target: ObjectId,
) -> Option<Pending> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    cast_from_hand(engine, seat, deflecting_swat());
    let mut aimed = false;
    for _ in 0..24 {
        match engine.pending().clone() {
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let slot = options
                    .iter()
                    .position(|o| !matches!(o.kind, CastModeKind::Alternative(_)))
                    .expect("the printed cost is offered");
                engine
                    .apply(player, PlayerAction::ChooseMode(slot))
                    .unwrap();
            }
            Pending::ChooseTargets { player, .. } if !aimed => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("\"target spell or ability\": the one on the stack");
                aimed = true;
            }
            question @ Pending::ChooseTargets { .. } => return Some(question),
            Pending::Priority { player, .. } => {
                if aimed && on_stack(engine, deflecting_swat()).is_none() {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Swat is cast: {other:?}"),
        }
    }
    panic!("the Swat never resolved")
}

/// Casts Archdruid's Charm off floating mana and answers its mode question
/// with mode `mode`, the way a player presses one of the offered buttons.
/// Returns the modes that were offered; none when only one mode could be
/// chosen, which the engine takes without asking.
#[track_caller]
fn cast_archdruids_charm(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    mode: usize,
) -> Vec<CastModeKind> {
    cast_with_floating(engine, seat, archdruid_s_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        return Vec::new();
    };
    let offered: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
    let slot = offered
        .iter()
        .position(|kind| *kind == CastModeKind::Mode(mode))
        .unwrap_or_else(|| panic!("mode {mode} was not offered: {offered:?}"));
    engine
        .apply(seat, PlayerAction::ChooseMode(slot))
        .expect("an offered mode");
    offered
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

/// Casts Vantress Visions, the Virtue's adventure, off mana already floating
/// and aims it at `ability`, answering as a player would: the face, if the
/// engine asks which (it does not when only the adventure is affordable),
/// then the target, out of a list that must hold it.
#[track_caller]
fn cast_vantress_visions(engine: &mut Engine<RegistryLookup>, seat: PlayerId, ability: ObjectId) {
    cast_with_floating(engine, seat, virtue_of_knowledge());
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let adventure = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
            .expect("Vantress Visions is a way to cast the card");
        engine
            .apply(seat, PlayerAction::ChooseMode(adventure))
            .expect("the face came out of the list");
    }
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Visions asks for its target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ability),
        "an ability you control on the stack: {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![ability],
                players: vec![],
            },
        )
        .expect("the ability");
}

/// The object on top of the stack.
fn top_of_stack(engine: &Engine<RegistryLookup>) -> ObjectId {
    *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .expect("something is on the stack")
}

/// How many times the journal says an ability of `source` was put on the
/// stack by being activated or triggered.
fn put_on_stack_from(engine: &Engine<RegistryLookup>, source: ObjectId) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(e.event, crate::event::GameEvent::AbilityTriggered { source: s, .. } if s == source)
        })
        .count()
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
            .list(crate::zone::ZoneLocation::Exile(p0))
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
            crate::event::GameEvent::BecameTarget { object, target, .. } if target == land => {
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
///
/// The transform half needs a count of what is exiled *with* the permanent
/// and is refused by name; `{3}{W}{B}: Exile target creature` is expressible
/// and is what a game can show.
#[test]
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
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is seated");
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
}

/// Casts Lose Focus off `seat`'s floating mana and answers its replicate
/// question with `times` and its target with `target`, checking the question
/// says what it counts.
#[track_caller]
fn cast_lose_focus_replicated(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    most: u32,
    times: u32,
    target: ObjectId,
) {
    cast_from_hand(engine, seat, lose_focus());
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason,
    } = engine.pending().clone()
    else {
        panic!(
            "replicate is announced with the additional costs, before the \
             target (CR 601.2b), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    assert_eq!(
        (min, max),
        (0, most),
        "any number of times, as many as the floating mana pays for"
    );
    assert_eq!(
        reason,
        crate::choice::NumberPrompt::Replicate {
            cost: baylee_core::mana::ManaCost::parse("{U}")
        },
        "the question says it counts replicate payments of {{U}}, not X"
    );
    assert!(
        engine
            .apply(seat, PlayerAction::ChooseNumber(most + 1))
            .is_err(),
        "a count the pool cannot pay is refused"
    );
    engine
        .apply(seat, PlayerAction::ChooseNumber(times))
        .expect("a count inside the offer");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the spell is a legal target");
}

/// The replicate trigger on the stack, if there is one: a synthetic ability
/// whose source is `spell`.
fn replicate_trigger(engine: &Engine<RegistryLookup>, spell: ObjectId) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.ability.is_some_and(|loc| {
                    loc.source == spell && loc.index == baylee_core::ids::AbilityRef::SYNTHETIC
                })
            })
        })
}

fn ancestral_recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

fn annul() -> CardIndex {
    card_index("d08e9784-75f7-4164-ac48-d06160f8c56b")
}

fn artifact_blast() -> CardIndex {
    card_index("2d4aedc5-31c5-4281-98e1-b0c2233c3c8a")
}

fn battlegrowth() -> CardIndex {
    card_index("650bf82e-7f83-470c-bf4a-34281bfe9341")
}

fn brightstone_ritual() -> CardIndex {
    card_index("08e90e85-4103-4acb-a8a7-e1329b460aa7")
}

fn burst_of_energy() -> CardIndex {
    card_index("b795ecfd-31ac-4a2a-9679-97b088932d93")
}

fn demystify() -> CardIndex {
    card_index("fd591199-9f7a-4147-a150-13279dbb4498")
}

fn enrage() -> CardIndex {
    card_index("0f6e66d5-4f27-485b-999d-ee55c1e218b9")
}

fn envelop() -> CardIndex {
    card_index("30062dd0-c049-4872-8011-8b4810a3fa26")
}

fn erase() -> CardIndex {
    card_index("c2ceb15c-5d02-4cf4-a7c9-c1a40b7ca667")
}

fn heat_ray() -> CardIndex {
    card_index("76ec76b9-da0f-4b9d-ad0a-d734052a5f2b")
}

fn howl_from_beyond() -> CardIndex {
    card_index("403cf6ae-48a9-4ea9-894e-7135cfca4e1b")
}

fn iron_will() -> CardIndex {
    card_index("dc0decb9-32da-47aa-a51a-0d9c623c534a")
}

fn jump() -> CardIndex {
    card_index("f7518456-45ed-41d4-bd3c-5aacea28eb35")
}

fn leap() -> CardIndex {
    card_index("e89a3ce0-6b38-4326-a7af-8575af371baa")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn lose_hope() -> CardIndex {
    card_index("4f6dbd90-f7fe-4adf-a00f-27573c0bd5c4")
}

fn mental_note() -> CardIndex {
    card_index("e8d5f31c-7abf-4fbb-977e-8353a97daf7a")
}

fn opt() -> CardIndex {
    card_index("713332c1-5bd8-400f-bfff-c1ca0697a043")
}

// oracle_id = "0ff78353-a26e-4b5a-948d-1c3b41d2dd1f"
fn oxidize() -> CardIndex {
    card_index("0ff78353-a26e-4b5a-948d-1c3b41d2dd1f")
}

fn quiet_purity() -> CardIndex {
    card_index("3f16dc14-3d3f-4ffa-90bc-9abc67db75cf")
}

fn reach_through_mists() -> CardIndex {
    card_index("c81ca8ff-92f3-481e-82f7-0673b6c74ea0")
}

fn rescue() -> CardIndex {
    card_index("f0ae687a-7222-4521-a514-ba10373fed7e")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

/// Storm Crow, whose printed 1/2 is the whole reason it is here: two
/// damage is exactly lethal on it where one would not be, so the number
/// Shock deals is readable off the graveyard. A 1/1 would have died to
/// either.
fn a_one_two_bird() -> CardIndex {
    card_index("000d5588-5a4c-434e-988d-396632ade42c")
}

fn shrink() -> CardIndex {
    card_index("252330b1-67cf-4d9e-a413-917ba61e731f")
}

fn silk_net() -> CardIndex {
    card_index("58511cb8-348d-409e-8fd6-772be20e57cd")
}

fn spark_spray() -> CardIndex {
    card_index("c37f1921-7ac3-4185-8579-1dfdfe647ea2")
}

fn sprout() -> CardIndex {
    card_index("424c6f5e-b386-47e9-b3fe-25b263097d40")
}

fn stand_firm() -> CardIndex {
    card_index("c0406e70-8131-4e13-b1d5-4e943ad296b8")
}

fn tunnel() -> CardIndex {
    card_index("80559618-9dd9-4987-b3bc-1a1b5537bbc5")
}

/// Wall of Roots — `{1}{G}` 0/5 Plant Wall, the pool's plainest Wall and the
/// only one a test can name by oracle id.
fn wall_of_roots() -> CardIndex {
    card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
}

fn accelerate() -> CardIndex {
    card_index("79189464-9645-4411-8886-68fd40a588ed")
}

fn aggressive_urge() -> CardIndex {
    card_index("43f5a93b-0f8d-48d2-ab9d-275d44cf88b5")
}

fn aura_blast() -> CardIndex {
    card_index("4e3c3bdc-667d-42ec-b960-159a39c53cb3")
}

fn clear() -> CardIndex {
    card_index("d0783be9-e518-431a-8e71-121610da50e1")
}

fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

fn eladamri_s_call() -> CardIndex {
    card_index("4acb6612-54e8-428d-acb6-c7259a5ad6a8")
}

// oracle_id = "b2c9f074-57ca-4709-976d-f432f632483f"
fn extinguish() -> CardIndex {
    card_index("b2c9f074-57ca-4709-976d-f432f632483f")
}

fn false_summoning() -> CardIndex {
    card_index("4f891c68-c959-4210-94e5-94a8e487d5ef")
}

// oracle_id = "fc05e582-e760-4fe2-ba43-e9d8e63f3f85"
fn fists_of_the_anvil() -> CardIndex {
    card_index("fc05e582-e760-4fe2-ba43-e9d8e63f3f85")
}

fn flowstone_strike() -> CardIndex {
    card_index("d67048eb-eaf1-4b5b-a8e3-a004bab438f3")
}

fn gerrards_command() -> CardIndex {
    card_index("de8ebe4b-d86d-478b-946e-f83e1409b63f")
}

fn guided_strike() -> CardIndex {
    card_index("a215e813-df7e-4f21-9718-eb264eb62813")
}

fn hero_s_demise() -> CardIndex {
    card_index("69ec0b59-a0aa-4878-a3b3-130cea18fee0")
}

fn heroes_reunion() -> CardIndex {
    card_index("beab66e0-2b6c-478f-ba42-8c0c216a527e")
}

fn hisoka_s_defiance() -> CardIndex {
    card_index("2ac6f1c7-f4c5-4d45-9644-da49d8fe4758")
}

fn hoodwink() -> CardIndex {
    card_index("a2fe10b8-857e-4881-85ef-24a0f96a3d79")
}

fn mage_s_guile() -> CardIndex {
    card_index("f371cdaf-7ab8-4555-8c96-61dfbf8c0179")
}

/// Magma Jet prints two effects and both are played by one cast: "deals 2
/// damage to any target", then "Scry 2". The damage has to *kill*, so the
/// target is a printed 1/1 — a creature still standing would mean the amount
/// was never read — and the 20 life its controller keeps says the two went to
/// the permanent that was named and not to the seat whose board it stood on
/// (CR 115.4 offers both in one choice). The scry is read as a *move*: the two
/// cards the question offers, the one that is chosen landing on the bottom
/// while the other becomes the new top, with the library exactly as long as it
/// was because scry draws nothing.
fn magma_jet() -> CardIndex {
    card_index("2f292253-64d3-4cf2-881f-a3eea4fda388")
}

fn naturalize() -> CardIndex {
    card_index("bdb3ca68-ec1f-4e16-81cc-d23f8f52c728")
}

fn nourish() -> CardIndex {
    card_index("9d5e82d3-79ac-4dbe-80e8-1db6dd6c8767")
}

fn predators_strike() -> CardIndex {
    card_index("f62e9a9f-9989-4a62-b063-cca32e21fe8b")
}

fn preemptive_strike() -> CardIndex {
    card_index("250f8642-9754-48fd-8f09-70ed13d7a42c")
}

fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

// oracle_id = "b13c0f76-fbda-4911-9442-c3d7e97f1aac"
fn remove_soul() -> CardIndex {
    card_index("b13c0f76-fbda-4911-9442-c3d7e97f1aac")
}

fn turn_to_dust() -> CardIndex {
    card_index("56828166-eaa3-4711-91b6-401a3e3b733f")
}

fn unnatural_speed() -> CardIndex {
    card_index("fa46ced5-b701-4aa3-b4a1-36fb15b80696")
}

fn terminate() -> CardIndex {
    card_index("6257c2fd-005f-41e3-8a72-af76df1eb134")
}

fn fissure() -> CardIndex {
    card_index("c8b1e9f3-b014-4e57-b278-6d84a7e88b23")
}

/// `seat` casts `card` off its whole board, aimed at `objects` and `players`,
/// and the spell on the stack is returned.
#[track_caller]
fn aimed(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> ObjectId {
    cast_from_hand(engine, seat, card);
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: objects.to_vec(),
                players: players.to_vec(),
            },
        )
        .expect("the spell's own target question offered these");
    on_stack(engine, card).expect("the spell is on the stack")
}

/// `seat` answers `spell` with Misdirection, pitching the Counterspell in hand
/// for it, and passes until it resolves (#247). Returns the new-target
/// question it asks, or `None` when it asks none.
///
/// Misdirection's own target question comes first and names a spell; the
/// next target question is the one the card is for.
#[track_caller]
fn misdirected(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    spell: ObjectId,
) -> Option<Pending> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    let pitch = in_hand(engine, seat, counterspell()).expect("a blue card to pitch");
    cast_with_floating(engine, seat, misdirection());
    let mut aimed = false;
    for _ in 0..24 {
        match engine.pending().clone() {
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .expect("the pitch cost is offered");
                engine
                    .apply(player, PlayerAction::ChooseMode(slot))
                    .unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .expect("the Counterspell pays the pitch cost");
            }
            Pending::ChooseTargets { player, .. } if !aimed => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![spell],
                        },
                    )
                    .expect("\"target spell\": the one on the stack");
                aimed = true;
            }
            question @ Pending::ChooseTargets { .. } => return Some(question),
            Pending::Priority { player, .. } => {
                if aimed && on_stack(engine, misdirection()).is_none() {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while Misdirection is cast: {other:?}"),
        }
    }
    panic!("Misdirection never resolved")
}

/// Casts Pact of Negation at p0's own Llanowar Elves, the one spell a duel
/// with nothing else in hand can offer it, and lets it resolve. What is left
/// is the debt, with `board` to pay it from and `also_in_hand` still held.
fn a_pact_owed_by_p0(board: &[CardIndex], also_in_hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut hand = vec![llanowar_elves(), pact_of_negation()];
    hand.extend_from_slice(also_in_hand);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_with_floating(&mut engine, p0, pact_of_negation());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves are on the stack");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Pact targets the Elves");
    pass_until(&mut engine, stack_is_empty);
    engine
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
            Some(crate::event::LossReason::Effect),
            "has the mana: {has_the_mana}"
        );
        assert!(
            engine.state().players[0].life > 0,
            "and not to its life total"
        );
        assert_eq!(engine.state().players[1].loss, None);
    }
}

/// Clever Concealment on `board`, main phase, every land tapped for mana.
fn concealment_table(board: &[CardIndex]) -> Engine<RegistryLookup> {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(229, plains())
        .hand(0, &[clever_concealment()])
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);
    tap_all_mana(&mut engine, seat);
    engine
}

fn concealment_is_offered(engine: &Engine<RegistryLookup>) -> bool {
    let seat = PlayerId::new(0);
    let card = in_hand(engine, seat, clever_concealment()).expect("in hand");
    matches!(engine.pending(), Pending::Priority { legal, .. } if legal.castable.contains(&card))
}

/// Casts Clever Concealment phasing nothing out, and returns the tap question
/// its convoke asks, if it asks one.
fn concealment_convoke_question(
    engine: &mut Engine<RegistryLookup>,
) -> Option<(Vec<ObjectId>, u32)> {
    let seat = PlayerId::new(0);
    let card = in_hand(engine, seat, clever_concealment()).expect("in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is offered");
    if let Pending::ChooseTargets { reason, .. } = engine.pending()
        && *reason != crate::choice::TargetPrompt::Convoke
    {
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![],
                },
            )
            .expect("\"any number of target\" permanents may be none");
    }
    tap_to_pay_question(engine)
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

/// Convoke's question offers what the payment can take, and the payment
/// takes generic mana only until #230: five creatures are offered, two may be
/// tapped, and the Plains pay `{W}{W}`. #229 asked for up to five, and the
/// third to fifth tap paid for nothing.
///
/// This pins a limitation. #230 teaches the payment to take a white creature
/// for a `{W}` (CR 702.51a), the bound moves to four with it, and this test
/// goes red on purpose.
#[test]
fn clever_concealment_s_convoke_is_asked_for_its_generic_and_no_more_until_230() {
    let seat = PlayerId::new(0);
    let mut board = vec![plains(), plains()];
    board.extend(std::iter::repeat_n(ondu_cleric(), 5));
    let mut engine = concealment_table(&board);
    let (options, max) =
        concealment_convoke_question(&mut engine).expect("convoke asks for its taps");
    assert_eq!(options.len(), 5, "every creature may be the one tapped");
    assert_eq!(max, 2, "the taps pay {{2}} and nothing past it");
    let tapped: Vec<ObjectId> = options.iter().copied().take(2).collect();
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: tapped.clone(),
                players: vec![],
            },
        )
        .expect("two taps and two Plains pay {2}{W}{W}");
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
}

/// `{W}` Aura: "Enchanted creature gets +1/+2."
fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

fn memory_deluge() -> CardIndex {
    card_index("e6fd55f2-7e26-469c-a44a-ea2eb90e19a9")
}

fn consult_the_star_charts() -> CardIndex {
    card_index("e921839f-9d91-41a9-bc89-016af3c757aa")
}

/// Answers a look-and-keep with the first `min` cards offered, after
/// checking how many were looked at and how many are kept, and returns the
/// cards that were not kept.
#[track_caller]
fn keep_first(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    looked: usize,
    kept: u8,
) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, ChoicePrompt::PutIntoHand);
    assert_eq!(options.len(), looked, "how many were looked at");
    assert_eq!((min, max), (kept, kept), "how many are kept");
    let keep = options[..usize::from(kept)].to_vec();
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: keep.clone(),
            },
        )
        .unwrap();
    // "In a random order": nobody is asked to arrange the rest.
    assert!(
        !matches!(engine.pending(), Pending::Arrange { .. }),
        "the rest are not the player's to order"
    );
    pass_until(engine, stack_is_empty);
    for card in &keep {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(seat))
                .contains(card)
        );
    }
    options[usize::from(kept)..].to_vec()
}

fn realms_uncharted() -> CardIndex {
    card_index("e21c8fc6-d4ef-42b6-b11e-d9c931da1387")
}

/// Moves the named cards from the seat's hand into its library, the harness
/// way, so a search has lands of several names to find among the Forests.
#[track_caller]
fn hide_in_library(engine: &mut Engine<RegistryLookup>, seat: PlayerId, cards: &[CardIndex]) {
    for &card in cards {
        let id = in_hand(engine, seat, card).expect("the card starts in hand");
        engine
            .dev_state_mut(seat)
            .expect("the harness may set boards up")
            .move_object(
                id,
                ZoneLocation::Library(seat),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("into the library");
    }
}

/// Casts Realms Uncharted and finds the first `n` lands it offers, after
/// checking the offer is one card of each name, at most four of them.
#[track_caller]
fn realms_search(engine: &mut Engine<RegistryLookup>, seat: PlayerId, n: usize) -> Vec<ObjectId> {
    cast_from_hand(engine, seat, realms_uncharted());
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    // "Up to four", and a counted choice is fitted to what it offers
    // (CR 609.3): four of five names, two of two.
    assert_eq!(
        (min, usize::from(max)),
        (0, options.len().min(4)),
        "up to four"
    );
    let mut names: Vec<_> = options
        .iter()
        .map(|id| engine.state().object(*id).unwrap().characteristics().name)
        .collect();
    let offered = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), offered, "one card of each name is offered");
    let found = options[..n].to_vec();
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: found.clone(),
            },
        )
        .unwrap();
    found
}

// oracle_id = "437b2dab-15e0-4b9a-a204-58622d37a3b3"
fn fact_or_fiction() -> CardIndex {
    card_index("437b2dab-15e0-4b9a-a204-58622d37a3b3")
}

/// Fact or Fiction cast with three known cards on top of the library, up to
/// the opponent's question. Returns the engine and the five revealed cards,
/// top first: Ondu Cleric, Counterspell, Llanowar Elves, then two Forests.
fn fact_or_fiction_revealed(seed: u64) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(
            0,
            &[
                fact_or_fiction(),
                llanowar_elves(),
                counterspell(),
                ondu_cleric(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mut on_top = Vec::new();
    for card in [llanowar_elves(), counterspell(), ondu_cleric()] {
        on_top.insert(0, hand_to_library_top(&mut engine, p0, card));
    }
    let journal_from = engine.state().journal.entries().len();
    cast_from_hand(&mut engine, p0, fact_or_fiction());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert_eq!(player, p1, "\"an opponent separates those cards\"");
    assert_eq!(options.len(), 5, "\"the top five cards\"");
    assert_eq!(
        options[..3],
        on_top[..],
        "the top of the library, top first"
    );
    assert_eq!((min, max), (0, 5), "either pile may hold any of them");
    assert_eq!(prompt, ChoicePrompt::FirstPile);
    assert!(
        engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards == &options)),
        "\"reveal the top five\", before anyone separates them"
    );
    (engine, options)
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
            Some(crate::zone::Zone::Graveyard)
        );
    }
    assert!(engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty());
}

/// Seat 0 casts Dark Ritual and, holding priority, Three Steps Ahead with
/// its counter and copy modes, {4}{U}{U}: the Ritual is the spell's first
/// instance of "target", its own Llanowar Elves the second (CR 700.2c).
/// Seat 1 holds `theirs` and the lands `lands`. Returns the engine with the
/// Three Steps on the stack and seat 0 holding priority, the Ritual and the
/// Elves.
fn three_steps_ahead_of_a_ritual(
    lands: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                swamp(),
                island(),
                island(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, lands)
        .hand(0, &[dark_ritual(), three_steps_ahead()])
        .hand(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, dark_ritual());
    let ritual = on_stack(&engine, dark_ritual()).unwrap();
    cast_with_floating(&mut engine, p0, three_steps_ahead());
    let offered = choose_modes(&mut engine, p0, 0b011);
    assert!(offered.iter().any(|o| o.kind == CastModeKind::Modes(0b011)
        && o.cost == baylee_core::mana::ManaCost::parse("{4}{U}{U}")));
    // Each instance is explained as its own mode's sentence: the counter's,
    // then the copy's. Neither is "the second target" of one sentence, and
    // what the counter chose is nothing for the copy to weigh.
    let context = engine.decision_context();
    assert_eq!((context.mode, context.second_instance), (Some(0), false));
    let _ = aim_at(&mut engine, p0, ritual);
    let context = engine.decision_context();
    assert_eq!(
        (context.mode, context.second_instance, context.first_targets),
        (Some(1), false, &[][..])
    );
    let _ = aim_at(&mut engine, p0, elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    (engine, ritual, elves)
}

// ---------------------------------------------------------------------------
// Alpha cards, played by their Oracle text.
// ---------------------------------------------------------------------------

/// A seat's current life total, read the way most of this batch reads it.
fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// Passes priority, declaring nothing, until `seat` is the one asked — the
/// shape of every response window in this batch where the other seat has
/// nothing left to add and only needs to get out of the way.
#[track_caller]
fn pass_until_priority(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == seat => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected a priority round on the way to {seat:?}, got {other:?}"),
        }
    }
}

/// A vanilla {6} 4/6 artifact creature — Obsianus Golem, no printed
/// keywords or abilities. Used here only as an ordinary body to attack or
/// block with; its own Oracle text is checked in `artifacts.rs`.
fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

fn death_ward() -> CardIndex {
    card_index("0544b707-ec67-43e3-a25d-fd7005ab673d")
}

fn healing_salve() -> CardIndex {
    card_index("8da8644c-75a1-4fe9-8e94-900d948d631c")
}

fn reverse_damage() -> CardIndex {
    card_index("eaaf7c30-f463-4115-a40e-7dc717063413")
}

fn righteousness() -> CardIndex {
    card_index("3f6b2f76-0364-415e-a6a2-e9e5bf31b745")
}

fn blue_elemental_blast() -> CardIndex {
    card_index("65e1558c-6b09-4ddc-b520-f19f4fb972af")
}

fn mana_short() -> CardIndex {
    card_index("48207d1c-448a-4e1b-974a-642dfea75933")
}

fn psionic_blast() -> CardIndex {
    card_index("7f221ad6-7ec4-483d-a6b5-1456c95c1cad")
}

fn thoughtlace() -> CardIndex {
    card_index("6452b6a6-6235-46a3-a712-a26592450438")
}

fn twiddle() -> CardIndex {
    card_index("773ad2ef-5acc-49ea-8d85-056330e87039")
}

fn deathlace() -> CardIndex {
    card_index("fb80aaba-352a-4b58-8db2-1e02d542819c")
}

fn terror() -> CardIndex {
    card_index("b81f041d-98db-4408-9472-c483e4a502bc")
}

fn chaoslace() -> CardIndex {
    card_index("08842aa3-f923-46e9-a106-f542331e9cc1")
}

/// The opponent answers `spell` (cast by seat 0 off `spell_land`) with `lace`
/// (cast by seat 1 off `lace_land`): "Target spell or permanent becomes
/// <color>. (Its mana symbols remain unchanged.)" aimed at the *spell*.
/// Asserts the spell is its printed color on the stack first, the lace's
/// color once the lace has resolved, the mana cost untouched, and the
/// permanent it becomes still that color afterwards: the change has no end,
/// so it is not the stack's alone.
#[track_caller]
fn a_lace_recolours_a_spell(
    lace: CardIndex,
    lace_land: CardIndex,
    spell: CardIndex,
    spell_land: CardIndex,
    printed: Color,
    becomes: Color,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1002, forest())
        .battlefield(0, &[spell_land])
        .battlefield(1, &[lace_land])
        .hand(0, &[spell])
        .hand(1, &[lace])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    cast_from_hand(&mut engine, p0, spell);
    let on_the_stack = on_stack(&engine, spell).expect("the creature spell is on the stack");
    let before = engine
        .state()
        .object(on_the_stack)
        .unwrap()
        .characteristics()
        .clone();
    assert_eq!(
        before.colors,
        ColorSet::of(printed),
        "its printed color before the lace"
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, lace);
    let offered = aim_at(&mut engine, p1, on_the_stack);
    assert!(
        offered.contains(&on_the_stack),
        "a spell on the stack is a legal target: {offered:?}"
    );
    pass_until(&mut engine, |e| on_stack(e, lace).is_none());
    let after = engine
        .state()
        .object(on_the_stack)
        .expect("the creature spell is still on the stack")
        .characteristics()
        .clone();
    assert_eq!(after.colors, ColorSet::of(becomes), "\"becomes\"");
    assert_eq!(
        after.mana_cost, before.mana_cost,
        "\"its mana symbols remain unchanged\""
    );

    pass_until(&mut engine, stack_is_empty);
    let permanent = on_battlefield(&engine, p0, spell).expect("the spell resolved");
    assert_eq!(
        engine
            .state()
            .object(permanent)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(becomes),
        "the permanent it became keeps the color"
    );
}

fn red_elemental_blast() -> CardIndex {
    card_index("bb329a5c-b9f9-4973-a53f-090024146325")
}

fn fog() -> CardIndex {
    card_index("27e9db49-7af7-4bef-ad4c-bf5dfb92030d")
}

fn purelace() -> CardIndex {
    card_index("3773001a-8868-49ec-a406-298cf72359c2")
}

fn lifelace() -> CardIndex {
    card_index("eec1de80-4b3d-481d-a235-c299e0381830")
}

fn natural_selection() -> CardIndex {
    card_index("57f90b30-bcb0-447e-8788-5c5ded187207")
}

fn power_sink() -> CardIndex {
    card_index("39412e6d-2837-4729-abf9-e64a5ba87e40")
}

/// Casts Power Sink for `x` at `target` (an object on the stack), answering
/// the announce-time questions (CR 601.2b, 601.2c) in whichever order the
/// engine asks them, and returns the target menu it published.
#[track_caller]
fn cast_power_sink_at(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
    x: u32,
    target: ObjectId,
) -> Vec<ObjectId> {
    let sink = in_hand(engine, caster, power_sink()).expect("Power Sink in hand");
    engine
        .apply(caster, PlayerAction::CastSpell { card: sink })
        .expect("Power Sink is castable");
    let mut asked_x = false;
    let mut targeted = false;
    let mut menu = Vec::new();
    for _ in 0..8 {
        if asked_x && targeted {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(
                    min <= x && x <= max,
                    "X = {x} must be one of the values on offer: {min}..={max}"
                );
                engine
                    .apply(player, PlayerAction::ChooseNumber(x))
                    .expect("the answer came out of the question");
                asked_x = true;
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("the named spell was on the menu");
                targeted = true;
            }
            other => panic!("unexpected while casting Power Sink: {other:?}"),
        }
    }
    assert!(
        asked_x && targeted,
        "both questions were asked and answered"
    );
    menu
}

fn fork() -> CardIndex {
    card_index("50c53ae0-51ba-4046-ac74-87c65e688032")
}

fn berserk() -> CardIndex {
    card_index("8b67d192-9a05-4a47-82ae-5fc4b7834d88")
}

fn siren_s_call() -> CardIndex {
    card_index("269fc857-a052-4f0a-9759-467ccf42bebb")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Taps whichever untapped basic land `seat` still has, floating one mana
/// of its color. `legal.castable` reads the pool that is already floating
/// (CR 106.4 empties it at the end of every step), never a land that merely
/// *could* be tapped, so a "not castable" reading is only about timing when
/// this was called first — otherwise it is indistinguishable from "no mana".
///
/// Filtered for an actual land rather than taking `mana_abilities.first()`
/// blind: that list also carries a granted mana ability with no printed
/// source (`docs/protocol.md` §"Granted mana"), which is not a land this
/// helper could have meant to tap.
#[track_caller]
fn float_one_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let source = *priority_offer(engine)
        .mana_abilities
        .iter()
        .find(|&&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.characteristics().types.contains(TypeSet::LAND))
        })
        .expect("an untapped basic land is still available");
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("the land taps for mana");
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

// ---------------------------------------------------------- Blaze of Glory

fn blaze_of_glory() -> CardIndex {
    card_index("b330ac89-790e-4cc9-96a5-532c48252088")
}

fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

fn fire_sprites() -> CardIndex {
    card_index("fc5e42b5-4da2-4777-828b-138c0a5d234f")
}

fn pearled_unicorn() -> CardIndex {
    card_index("c071be90-0531-40cc-af46-0cbe80c4ddd4")
}

fn hurloon_minotaur() -> CardIndex {
    card_index("8f1dae40-b307-446e-bbd2-86aa35813871")
}

/// What `actions.rs`' block-requirement check refuses a declaration with.
const MUST_BLOCK: &str = "a creature that must block if able does not";

/// Asserts `result` is the engine's own refusal `why`, naming the message
/// rather than merely that something failed.
#[track_caller]
fn refused(result: Result<(), EngineError>, why: &str) {
    match result {
        Err(EngineError::IllegalAction(message)) => assert_eq!(message, why),
        other => panic!("expected the refusal {why:?}, got {other:?}"),
    }
}

/// Taps one of `seat`'s untapped Plains for `{W}`, and asserts that exactly
/// one white mana is now floating — so a "not castable" refusal checked
/// right after this can never be read as "not enough mana" instead.
#[track_caller]
fn float_one_white(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let source = all_on_battlefield(engine, seat, plains())
        .into_iter()
        .find(|id| !is_tapped(engine, *id))
        .expect("an untapped Plains remains");
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("a Plains taps for white");
    assert_eq!(
        engine.state().players[seat.get() as usize]
            .mana_pool
            .available(ManaColor::White),
        1,
        "exactly the one white this float put there"
    );
}

fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

// -------------------------------------------------------------- Simulacrum

fn simulacrum() -> CardIndex {
    card_index("20d69989-7250-40c7-a064-8ed78ccbe556")
}

fn white_knight() -> CardIndex {
    card_index("ddb021df-ae4a-4ac1-8353-d0b375761714")
}

fn flawless_maneuver() -> CardIndex {
    card_index("4e183439-17d2-47ff-9d99-5e22821d91e3")
}

// oracle_id = "7bb41690-f8ec-462a-ba29-be453eb86fca"
fn sandstorm() -> CardIndex {
    card_index("7bb41690-f8ec-462a-ba29-be453eb86fca")
}

// oracle_id = "3483946d-8645-4c22-b0ba-a65a44456324"
fn army_of_allah() -> CardIndex {
    card_index("3483946d-8645-4c22-b0ba-a65a44456324")
}

// oracle_id = "0c017406-7fc3-4701-93ec-ddb02044c12a"
fn piety() -> CardIndex {
    card_index("0c017406-7fc3-4701-93ec-ddb02044c12a")
}

fn crumble() -> CardIndex {
    card_index("8d6e39b0-a190-40a0-a8e1-ee82f477376f")
}

/// Living Wall: an artifact creature of mana value 4 whose own `{1}` ability
/// can raise the shield every "it can't be regenerated" test has to ignore.
fn living_wall() -> CardIndex {
    card_index("4844312c-3c9d-4ca1-986d-4ad35e68454e")
}

/// p0 at `life`, with Llanowar Elves on the stack (paid by a Forest) and
/// Mental Misstep in hand; `islands` Islands are tapped into the pool too.
/// Returns the engine at p0's priority with the Elves on the stack.
fn misstep_board(life: i32, islands: usize) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut lands = vec![forest()];
    lands.extend(std::iter::repeat_n(island(), islands));
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &lands)
        .hand(0, &[llanowar_elves(), mental_misstep()])
        .life(0, life)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, llanowar_elves());
    assert!(on_stack(&engine, llanowar_elves()).is_some());
    engine
}

fn misstep_is_offered(engine: &Engine<RegistryLookup>) -> bool {
    let p0 = PlayerId::new(0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    in_hand(engine, p0, mental_misstep()).is_some_and(|card| legal.castable.contains(&card))
}

fn aim_misstep_at_the_elves(engine: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    let elves = on_stack(engine, llanowar_elves()).expect("the Elves are on the stack");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the target question, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
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
        assert_eq!(prompt, crate::choice::YesNoPrompt::PayLife { amount: 2 });
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
