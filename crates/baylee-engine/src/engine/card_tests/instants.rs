//! Instants, the door `cards/instants/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Counterspell: the classic — p0's creature spell never arrives.
#[test]
fn counterspell_counters_a_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(12, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[ondu_cleric()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let cleric = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: cleric })
        .unwrap();

    // p0 passes; p1 taps both islands and counters the cleric.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p1, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let cs = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    // After both pass, the cleric is in the graveyard, not on the board.
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| {
                e.state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    });
    assert!(on_battlefield(&engine, p0, ondu_cleric()).is_none());
}

/// CR 601.2c: a spell whose mandatory target has no legal choice cannot be
/// cast at all, so it must not be offered. Counterspell with an empty stack
/// is the clean case — offering it hands a human a button that only errors,
/// and an agent an action it will pick again on every pass, because failing
/// changes nothing about the state.
#[test]
fn a_spell_with_no_legal_target_is_not_offered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.is_empty(),
        "counterspell was offered with nothing on the stack to counter"
    );
}

/// An Offer You Can't Refuse: "Counter target noncreature spell. Its
/// controller creates two Treasure tokens." The Treasures go to the player
/// whose spell was countered, which is the whole cost of the card — and the
/// effect resolves *after* the counter, so it has to find that player
/// through a spell that is already a card in a graveyard.
#[test]
fn an_offer_you_cant_refuse_pays_the_countered_spells_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(33, forest())
        .battlefield(0, &[island()])
        .hand(0, &[an_offer_you_cant_refuse()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // p0 holds; p1 answers with an instant of their own.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let ritual = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    engine
        .apply(p1, PlayerAction::CastSpell { card: ritual })
        .unwrap();

    // p0 answers that: tap the Island, counter the Ritual.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    let offer = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: offer })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![ritual], "the only noncreature spell up there");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    let treasures = |e: &Engine<RegistryLookup>, seat: PlayerId| {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                e.state()
                    .object(**id)
                    .is_some_and(|o| o.token.is_some() && o.controller == seat)
            })
            .count()
    };
    pass_until(&mut engine, |e| treasures(e, p1) == 2);
    assert_eq!(
        treasures(&engine, p0),
        0,
        "the Treasures are the countered player's, not the counterer's"
    );
    assert!(
        on_battlefield(&engine, p1, dark_ritual()).is_none(),
        "the Ritual was countered"
    );
}

/// "Exile **any number of** target creatures you control", with none to exile.
///
/// The spell half of what the Apparition's trigger settles above, and it
/// arrives by a different door: `min` is 0 and `max` is 255, so the cast
/// wizard's `max == 0` branch never sees this spell and the *board* is what
/// leaves it with nothing to choose. One legal answer is not a choice, so the
/// spell is cast with no targets rather than the caster being held at
/// `ChooseTargets { options: [], min: 0, max: 255 }` — a stop that can only
/// be answered one way.
#[test]
fn a_spell_that_may_target_any_number_is_not_asked_with_nothing_to_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(202, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[eerie_interlude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase hands priority back");
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&engine, p0, eerie_interlude()).expect("the Interlude is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("three Plains pay {2}{W}");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no creatures to exile, so no question: {:?}",
        engine.pending()
    );
    assert!(!stack_is_empty(&engine), "and the spell is on the stack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
}

/// Path to Exile: "Exile target creature. **Its controller** may search their
/// library for a basic land card…"
///
/// The ramp is the half of the card that is not the removal, and it is asked
/// of the seat whose creature just died — never of the seat who cast the
/// spell. `PlayerRel::ControllerOfTarget` is how the card says that, and a
/// site that resolved it through `eval::players` got no seats at all and
/// returned early, so the card shipped as a strictly better Swords to
/// Plowshares that also gave the opponent nothing.
///
/// The assertion therefore names the seat, not just the question: an
/// implementation that offered the search to the *caster* would be exactly as
/// wrong and would pass a test that only counted a `ChooseCards`.
#[test]
fn path_to_exile_offers_the_ramp_to_the_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[plains()])
        .hand(1, &[path_to_exile()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let victim = on_battlefield(&engine, p0, quiet_creature()).expect("p0's creature");
    let lands_before = lands_of(&engine, p0).len();

    cast_from_hand(&mut engine, p1, path_to_exile());
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("Path asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "their spell, their target");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("Path points at the creature");

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "the ramp never asked — got {:?}. `OptionalBasicLandSearchFor` \
                 resolves `ControllerOfTarget`, which only `players_of` can \
                 answer.",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the basic-land search, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the search belongs to the creature's controller, not to the caster"
    );
    assert_eq!((min, max), (0, 1), "\"may search\" — one card at most");
    assert!(
        !options.is_empty(),
        "p0's library is sixty Forests and every one of them is basic"
    );

    // The removal half happened too, and on the right card.
    assert_eq!(
        engine.state().object(victim).map(|o| o.zone),
        Some(Zone::Exile),
        "the creature is exiled, not destroyed"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("p0 takes the land");
    pass_until(&mut engine, |e| lands_of(e, p0).len() > lands_before);
    let fetched = *lands_of(&engine, p0).first().expect("the fetched land");
    assert!(
        engine
            .state()
            .object(fetched)
            .is_some_and(|o| o.status.contains(Status::TAPPED)),
        "\"put that card onto the battlefield tapped\""
    );
}

/// A tutor to the top of the library leaves the card it found on top.
///
/// Mystical Tutor prints "search your library for an instant or sorcery
/// card, reveal it, then shuffle **and put that card on top**", and the
/// search resolution shuffled *after* placing — so the card went on top and
/// was immediately shuffled back into sixty others. Every tutor-to-top in
/// the pool returned a random card.
#[test]
fn a_tutor_to_the_top_leaves_its_card_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(47, counterspell())
        .battlefield(0, &[island()])
        .hand(0, &[mystical_tutor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, mystical_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let found = *options
        .first()
        .expect("the library is full of Counterspells");
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
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(found),
        "the card the tutor found is the one on top",
    );
}

/// Mycosynth Lattice says "all **permanents** are artifacts", and an instant
/// is not one. It read `Filter::Any` and so reached the stack, where a
/// Brainstorm became an artifact spell — an artifact spell is a permanent
/// spell, and `finalize_spell` put it onto the battlefield and left it
/// there. CR 304.4: an instant cannot enter the battlefield at all.
///
/// Three cards went to the owner's battlefield this way in one game, and
/// Ephemerate's rebound was eaten with them: the card never reached the
/// resolution path that exiles it.
#[test]
fn an_instant_does_not_land_on_the_battlefield_under_mycosynth_lattice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[island(), mycosynth_lattice()])
        .hand(0, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lattice = on_battlefield(&engine, p0, mycosynth_lattice()).expect("lattice deployed");
    let land = on_battlefield(&engine, p0, island()).expect("island deployed");
    assert!(
        types(&engine, land).intersects(TypeSet::ARTIFACT),
        "a permanent still is an artifact — the card's own rules text"
    );
    assert!(types(&engine, lattice).intersects(TypeSet::ARTIFACT));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let bolt = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    // Sampled with the spell still on the stack and the layer pass behind
    // it — p0 has passed, p1 holds priority. "All permanents" does not reach
    // a spell, and this is the reading `finalize_spell` goes on to make.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the spell should still be on the stack here"
    );
    assert!(
        !types(&engine, bolt).intersects(TypeSet::ARTIFACT),
        "the Lattice reached the stack: an instant spell became an artifact spell"
    );

    let rest = drive_to_rest(&mut engine, p0);
    assert!(matches!(rest, Rest::Reached), "the duel stalled: {rest:?}");
    assert!(
        on_battlefield(&engine, p0, brainstorm()).is_none(),
        "CR 304.4: an instant card cannot enter the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, brainstorm()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// The other half of the same sentence, and the reason the fix is not only
/// in the card: no type-adding effect may make an instant a permanent, so
/// `TypeSet::is_permanent` answers the question once for every future
/// Lattice. Enlightened Tutor is what noticed — it searches for "an artifact
/// or enchantment card", and every card in the library matched.
#[test]
fn a_library_card_is_not_an_artifact_under_mycosynth_lattice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, forest())
        .battlefield(0, &[plains(), mycosynth_lattice()])
        .hand(0, &[enlightened_tutor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let tutor = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: tutor })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();

    // The library is nothing but Forests (the filler), so a correct search
    // finds nothing and the engine asks nothing. Under the old reading every
    // card in it was an artifact and the whole library was on the list.
    assert!(
        !matches!(engine.pending(), Pending::ChooseCards { .. }),
        "the tutor offered a search among Forests: {:?}",
        engine.pending()
    );
    let land = on_battlefield(&engine, p0, plains()).expect("plains deployed");
    assert!(
        types(&engine, land).intersects(TypeSet::ARTIFACT),
        "a permanent is still an artifact"
    );
    for &id in engine.state().zones.list(ZoneLocation::Library(p0)) {
        assert!(
            !types(&engine, id).intersects(TypeSet::ARTIFACT),
            "a card in the library is not a permanent and gains nothing"
        );
    }
}

// oracle_id = "1c747fe2-289e-492a-a846-aa77707e2dc3"
fn abrupt_decay() -> baylee_core::ids::CardIndex {
    card_index("1c747fe2-289e-492a-a846-aa77707e2dc3")
}

/// Abrupt Decay: "This spell can't be countered. Destroy target nonland
/// permanent with mana value 3 or less."
///
/// Both printed sentences are read off one board, and each is read against
/// something that *is* allowed, so neither half can pass by being empty.
///
/// The cap is pinned from both sides in one target list: Skyclave
/// Apparition costs exactly three and is offered, Karn costs exactly four
/// and is not, and not one of the five lands on the table is a nonland
/// permanent. A filter one off in either direction, or one that dropped the
/// word "nonland", moves that list.
///
/// The counter half needs the same care, because "not offered" is also what
/// a seat who could not have cast a counter at all looks like. So p0 puts a
/// second spell on the stack *under* the Decay: when p1's Counterspell asks
/// what it may point at, the Brainstorm is on the list and the Decay — up
/// there beside it, and payable by the same two Islands — is not.
#[test]
#[allow(clippy::too_many_lines)] // one game, played from the cast to the assertion
fn abrupt_decay_destroys_a_small_permanent_and_is_not_a_target_the_counterspell_may_point_at() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[swamp(), forest(), island()])
        .hand(0, &[abrupt_decay(), brainstorm()])
        .battlefield(
            1,
            &[
                island(),
                island(),
                llanowar_elves(),
                skyclave_apparition(),
                karn_the_great_creator(),
            ],
        )
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    let apparition =
        on_battlefield(&engine, p1, skyclave_apparition()).expect("the Apparition is deployed");
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("Karn is deployed");
    let library_before = library_size(&engine, p0);

    // The Swamp and the Forest pay for the Decay; the Island is held back for
    // the spell that goes on the stack underneath it.
    tap_all_mana_but(&mut engine, p0, Some(island()));
    let decay = in_hand(&engine, p0, abrupt_decay()).expect("the Decay is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: decay })
        .expect("a Swamp and a Forest pay {B}{G}");

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the Decay asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "their spell, their target");
    assert_eq!(
        (min, max),
        (1, 1),
        "\"target nonland permanent\" is one target, and not an optional one"
    );
    assert!(
        options.contains(&victim),
        "a one-mana creature is a nonland permanent with mana value 3 or less"
    );
    assert!(
        options.contains(&apparition),
        "the Apparition costs three, and \"3 or less\" includes three"
    );
    assert!(!options.contains(&karn), "Karn costs four, which is more");
    assert_eq!(
        options.len(),
        2,
        "and the five lands on the table are not nonland permanents: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Decay points at the Elf");

    // A second, ordinary spell on top of it, off the Island held back. It is
    // the control for everything asserted below about the counter.
    let storm = in_hand(&engine, p0, brainstorm()).expect("the Brainstorm is in hand");
    cast_from_hand(&mut engine, p0, brainstorm());
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert!(
        stack.len() == 2 && stack.contains(&decay) && stack.contains(&storm),
        "both spells are on the stack, the Decay among them: {stack:?}"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1's priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let counter = in_hand(&engine, p1, counterspell()).expect("the Counterspell is in hand");
    assert!(
        legal.castable.contains(&counter),
        "two Islands, and a counterable spell up there: the counter is offered"
    );
    engine
        .apply(p1, PlayerAction::CastSpell { card: counter })
        .expect("two Islands pay {U}{U}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the Counterspell asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![storm],
        "\"This spell can't be countered\": the Decay is up there beside the \
         Brainstorm and is not on the list"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![storm],
            },
        )
        .expect("the Counterspell points at the Brainstorm");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "\"Destroy target nonland permanent\": the Elf is off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and a destroyed permanent is its owner's card in their graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, abrupt_decay()).is_some(),
        "the Decay resolved, and a resolved instant is a card in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the same Counterspell did counter the spell it was allowed to point \
         at: a Brainstorm that had resolved would have drawn three cards"
    );
    assert!(
        on_battlefield(&engine, p1, skyclave_apparition()).is_some(),
        "the Apparition was offered and not chosen, so it is still there"
    );
    assert!(
        on_battlefield(&engine, p1, karn_the_great_creator()).is_some(),
        "and Karn was never on offer at all"
    );
}

// oracle_id = "464c0150-3dbc-403b-9ada-fef25ab1f29d"
fn brain_freeze() -> baylee_core::ids::CardIndex {
    card_index("464c0150-3dbc-403b-9ada-fef25ab1f29d")
}

/// Brain Freeze, {1}{U}: "Target player mills three cards." — and under it,
/// "Storm (When you cast this spell, copy it for each spell cast before it
/// this turn. You may choose new targets for the copies.)"
///
/// The card is `Coverage::Partial`, so both halves are played here. A test
/// that only counted the three cards would be green on a card that prints two
/// sentences and plays one — and the unplayed one is the sentence the card is
/// famous for.
///
/// Dark Ritual is cast first for exactly one reason: it makes the turn's
/// spell count one, which the engine already keeps in `per_turn.spells_cast`
/// and which is asserted below. So the copy that never appears is a copy
/// storm would have been owed, and not one that had nothing to count. The
/// Ritual is asked nothing on the way — its whole text is "Add {B}{B}{B}" —
/// and it is cast off the Swamp alone, leaving both Islands for the spell
/// under test.
///
/// The mill half is aimed across the table, which is what "target player"
/// buys: the three cards have to leave the named seat's library and no other.
#[test]
fn brain_freeze_mills_three_and_the_storm_it_prints_never_copies_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[swamp(), island(), island()])
        .hand(0, &[dark_ritual(), brain_freeze()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let graveyard = |e: &Engine<RegistryLookup>, seat: PlayerId| {
        e.state().zones.list(ZoneLocation::Graveyard(seat)).len()
    };

    // The turn's first spell, so that storm would have something to count.
    tap_all_mana_but(&mut engine, p0, Some(island()));
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("one Swamp pays {B}");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.spells_cast[p0.get() as usize],
        1,
        "a spell was cast before the next one this turn — the number storm reads"
    );

    let their_library = library_size(&engine, p1);
    let their_graveyard = graveyard(&engine, p1);
    let my_graveyard = graveyard(&engine, p0);

    // Cast off the engine's own offer rather than at it: the two Islands are
    // tapped first, because a spell is castable here only once its mana is
    // already floating.
    let freeze = in_hand(&engine, p0, brain_freeze()).expect("the spell is in hand");
    tap_all_mana_but(&mut engine, p0, None);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.castable.contains(&freeze),
        "the engine offers the spell before the test presses it"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: freeze })
        .expect("two Islands pay {1}{U}");

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "\"target player\" is named as the spell is cast, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "their spell, their choice");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "either player may be milled, the caster included: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the other seat is one of the options just offered");

    // The storm half, at the one moment a copy would be visible: it is put on
    // the stack as the spell is cast, and is offered new targets of its own.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "storm is not written: the turn's second spell went on the stack alone"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "and nothing is asked to be re-aimed, because there is no copy: {:?}",
        engine.pending()
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 3,
        "three cards off the top of the named seat's library — three, not six"
    );
    assert_eq!(
        graveyard(&engine, p1),
        their_graveyard + 3,
        "milling puts them into the graveyard, not into exile"
    );
    assert_eq!(
        graveyard(&engine, p0),
        my_graveyard + 1,
        "the caster milled nothing: the one card added is Brain Freeze itself"
    );
    assert!(
        in_graveyard(&engine, p0, brain_freeze()).is_some(),
        "and the instant resolved once and went to its owner's graveyard"
    );
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

/// Flusterstorm: "Counter target instant or sorcery spell unless its
/// controller pays {1}" — and, printed beside it, storm, which is the
/// `Coverage::Partial` this card declares.
///
/// A Partial card owes both halves, so both are here.
///
/// The half it has: the tax is put to the *countered spell's* controller
/// and not to the seat that cast Flusterstorm, and declining it is a
/// counter rather than a fizzle. The mana pool is the whole proof of that
/// last word — a Dark Ritual that resolved and a Dark Ritual that was
/// countered both end in p1's graveyard, so the zone distinguishes nothing
/// and the number does: it would be three higher had the Ritual resolved,
/// and one lower had the tax been paid.
///
/// The half it has not: Dark Ritual was cast this turn before Flusterstorm,
/// so storm would copy it once and put its own "when you cast this spell"
/// trigger on the stack above the two spells. The stack holds the two
/// spells and nothing else, which is the card playing as though the line
/// were not printed — and the assertion that stops passing the day storm
/// is written.
#[test]
fn a_declined_flusterstorm_counters_the_spell_and_never_copies_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(59, forest())
        .battlefield(0, &[island()])
        .hand(0, &[flusterstorm()])
        // Two Swamps, not one. `PlayerMayPayOr` only ever *asks* a seat that
        // could pay out of its floating pool, so a Ritual cast off a single
        // Swamp would leave nothing behind and be countered without the
        // question ever being put — the right outcome for the wrong reason.
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // p0 holds; p1 answers with an instant of their own, one Swamp's worth
    // of mana still floating behind it.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual is in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // p0 taps the Island and points Flusterstorm at it.
    let fluster = in_hand(&engine, p0, flusterstorm()).expect("Flusterstorm is in hand");
    cast_from_hand(&mut engine, p0, flusterstorm());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ritual],
        "the only instant or sorcery on the stack for it to point at"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    // The gap, measured rather than asserted about the card file: one spell
    // was cast before this one this turn, so storm would have copied it once
    // and its trigger would be sitting on top of both of these.
    let stack = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .clone();
    assert_eq!(
        stack,
        vec![ritual, fluster],
        "the two spells, bottom to top, and nothing above them: storm prints \
         a \"when you cast this spell\" trigger this card does not have"
    );

    // Both pass, Flusterstorm resolves, and the tax is put to somebody.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: crate::choice::YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("the walk above stops on the tax question")
    };
    assert_eq!(*mana, 1, "Flusterstorm prints a {{1}} tax");
    assert_eq!(
        *player, p1,
        "\"unless its controller pays\" is the countered spell's controller, \
         not the seat that cast Flusterstorm"
    );

    let pool_before = engine.state().players[1].mana_pool.total();
    assert!(
        pool_before >= 1,
        "the second Swamp is what leaves the tax payable, and so what makes \
         the question askable at all"
    );
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is an answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before,
        "the Ritual was countered, so it added no {{B}}{{B}}{{B}} — and the \
         tax went unpaid, so nothing left the pool for it either"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.5a)"
    );
    assert!(
        in_graveyard(&engine, p0, flusterstorm()).is_some(),
        "and Flusterstorm, having resolved, follows it there"
    );
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
/// (CR 601.2c) and a land with shroud (CR 702.18b) is not offered at all.
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
    // for the rest of the phase (CR 500.4 empties a pool at the end of a
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
