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
/// and is not, and not one of the four lands on the table is a nonland
/// permanent. A filter one off in either direction, or one that dropped the
/// word "nonland", moves that list.
///
/// The counter half is the Gatherer ruling (2021-03-19): "A spell or ability
/// that counters spells can still target Abrupt Decay. When that spell or
/// ability resolves, Abrupt Decay won't be countered." So p1's Counterspell
/// is offered, points at the Decay, and resolves into the graveyard with the
/// Decay still on the stack under it. This test used to pin the opposite,
/// that the Decay was no target at all, which is how the engine read the
/// keyword until #243.
#[test]
#[allow(clippy::too_many_lines)] // one game, played from the cast to the assertion
fn abrupt_decay_destroys_a_small_permanent_and_the_counterspell_pointed_at_it_does_not_counter_it()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[abrupt_decay()])
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

    tap_all_mana(&mut engine, p0);
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
        "and the four lands on the table are not nonland permanents: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Decay points at the Elf");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1's priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let counter = in_hand(&engine, p1, counterspell()).expect("the Counterspell is in hand");
    assert!(
        legal.castable.contains(&counter),
        "two Islands, and a spell up there to point at: the counter is offered"
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
        vec![decay],
        "\"This spell can't be countered\" does not take the Decay off the list"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![decay],
            },
        )
        .expect("the Counterspell points at the Decay");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, counterspell()).is_some()
    });
    assert!(
        on_stack(&engine, abrupt_decay()).is_some(),
        "the Counterspell resolved and the Decay is still on the stack under it"
    );

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
        // Two Swamps, not one, so the Ritual leaves a mana floating and
        // Flusterstorm's tax is answered straight from the pool. Off a
        // single Swamp the question is still put — CR 605.3a opens a
        // payment window — but the test would then be about the window
        // instead of about Flusterstorm.
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
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
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

/// "Add {B}{B}{B}." The pool is read on both sides of the resolution: two
/// Swamps float {B}{B}, the cast takes both of them, and what stands in the
/// pool afterwards is three black and nothing at all beside it.
///
/// Reading the card file cannot say this. `Effect::mana(ManaColor::Black, 3)`
/// is an amount and a colour on paper; whether the engine puts three *black*
/// into the caster's pool — rather than one, or three colourless, or three
/// carrying a rider `total()` counts and `available()` does not — is visible
/// only by casting the spell and looking. The empty pool between the cast and
/// the resolution is what makes those three the spell's own rather than
/// change the Swamps left behind, and the other five colours read at zero are
/// the half that fails against an engine adding mana generously.
///
/// The threshold line is `Coverage::Partial` and is not what this proves: the
/// graveyard stays empty, so the printed default is the only branch here.
#[test]
fn a_ritual_adds_three_black_and_leaves_nothing_else_floating() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        // Exactly two, so the {1}{B} is paid to the last mana: a third Swamp
        // would leave a black floating that the assertion below could not
        // tell from the spell's own.
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[cabal_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool starts empty, so everything counted below arrived during \
         this test"
    );

    let ritual = in_hand(&engine, p0, cabal_ritual()).expect("the Ritual is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps and nothing else, so {{B}}{{B}} is the whole board's worth"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ritual),
        "{{B}}{{B}} floating against a cost of {{1}}{{B}}: the engine offers \
         the Ritual, and the test presses what was offered"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("the spell the offer just quoted");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{B}} took both Swamps' mana, so the pool is empty while the \
         spell is on the stack — whatever is in it after this is the \
         Ritual's"
    );

    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "\"Add {{B}}{{B}}{{B}}\" is three black mana, and it is black rather \
         than the generic a cost would accept anywhere"
    );
    assert_eq!(
        pool.total(),
        3,
        "three and no fourth, and none of them restricted: `total()` counts \
         the riders `available()` cannot see"
    );
    for color in ManaColor::ALL {
        if color == ManaColor::Black {
            continue;
        }
        assert_eq!(
            pool.available(color),
            0,
            "the Ritual adds one colour, and {color:?} is not it"
        );
    }
    assert!(
        in_graveyard(&engine, p0, cabal_ritual()).is_some(),
        "the mana is there because the instant resolved, and a resolved \
         instant lies in its owner's graveyard (CR 608.2n)"
    );
}

// oracle_id = "133c99c0-3652-410f-8100-68015a47af9f"
fn dispatch() -> baylee_core::ids::CardIndex {
    card_index("133c99c0-3652-410f-8100-68015a47af9f")
}

/// Dispatch, {W}: "Tap target creature." — the half of the card that is
/// built, played against a board that leaves the other half nothing to say.
///
/// The card is `Coverage::Partial`: the metalcraft line that exiles the
/// creature is not modelled. p0 controls one Plains and no artifact at all,
/// so the unbuilt clause has nothing to fire on and this scenario stays the
/// same scenario once it is built — nothing below asserts about it either
/// way.
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

// oracle_id = "ce19962d-94f9-4b2b-b668-963c0acce308"
fn borne_upon_a_wind() -> CardIndex {
    card_index("ce19962d-94f9-4b2b-b668-963c0acce308")
}

/// Borne Upon a Wind ({1}{U}, instant): "You may cast spells this turn as
/// though they had flash. Draw a card." The flash grant is the card's
/// `Coverage::Partial` gap — no modifier hands out a turn-long casting
/// permission — so this plays the half that is written, off two Islands in a
/// first main phase. The draw is asserted on the *card* rather than on a
/// count: the object that was on top of the library is the one that arrives
/// in hand, which is what tells a draw from a spell that merely left the hand.
#[test]
fn borne_upon_a_wind_draws_the_top_card_and_lands_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[borne_upon_a_wind()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    let top = *library_before
        .last()
        .expect("p0 has a library to draw from");
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    assert!(
        in_hand(&engine, p0, borne_upon_a_wind()).is_some(),
        "the instant starts in hand"
    );

    cast_from_hand(&mut engine, p0, borne_upon_a_wind());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, borne_upon_a_wind()).is_some(),
        "an instant that has resolved is put into its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\" is exactly one off the top, and not two"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .contains(&top),
        "the card drawn is the one that was on top of the library"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before,
        "the spell left the hand and one card replaced it — a draw of zero \
         would leave one fewer"
    );
}

fn deflecting_swat() -> CardIndex {
    card_index("ae120613-97d6-4393-b39d-c3e6c076f5d6")
}

/// Deflecting Swat asks `{2}{R}` and none of it while its caster *controls* a
/// commander, and it hands them the target spell to aim somewhere else. Both
/// printed clauses are one scenario: p1 points Swords to Plowshares at p0's
/// Llanowar Elves, and p0 — four spent Swamps, no mana in the pool and a
/// commander standing on the battlefield — casts the Swat for free and turns
/// the Swords onto p1's own Umara Raptor. A card that charged `{2}{R}` would
/// be refused outright here, so the cast itself is the free-cost proof; a Swat
/// that resolved without asking for new targets would exile the Elves, so the
/// two creatures' zones say whether the redirection happened at all.
///
/// The commander is *played* rather than seated, because "you control a
/// commander" is a battlefield sentence (`casting::controls_a_commander`) and
/// one waiting in the command zone is not controlled — which is what the
/// first draft of this test assumed, and the empty `castable` list it got
/// back is exactly what that mistake looks like.
#[allow(clippy::too_many_lines)] // a commander cast, an opponent's spell, and the redirection of it
#[test]
fn deflecting_swat_redirects_a_spell_for_free_while_a_commander_stands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(977, forest())
        .commander(0, &[sheoldred_the_apocalypse()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[deflecting_swat()])
        .battlefield(1, &[plains(), umara_raptor()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The four Swamps pay the commander's {2}{B}{B} and nothing else: the
    // pool p0 answers p1's spell out of is empty.
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
        on_battlefield(e, p0, sheoldred_the_apocalypse()).is_some() && stack_is_empty(e)
    });

    // p1 aims the Swords at the Elves — the play the Swat exists to undo.
    reach_their_main_phase(&mut engine, p1);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let raptor = on_battlefield(&engine, p1, umara_raptor()).expect("the Raptor is out");
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares asks what it is aimed at, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "the creature across the table is a legal target: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    // The caster takes priority back; hand it over so the seat with the Swat
    // gets to answer with the Swords standing on the stack.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!(
            "a spell on the stack hands priority back, got {:?}",
            engine.pending()
        )
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let swords = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == swords_to_plowshares()))
        })
        .expect("the Swords is on the stack");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that is not casting gets to respond");
    let swat = in_hand(&engine, p0, deflecting_swat()).expect("the Swat is in hand");
    assert!(
        legal.castable.contains(&swat),
        "the commander on the battlefield is what offers the free cast, and \
         p0's pool is empty: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: swat })
        .expect("the Swat is cast for nothing");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let free = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
            .expect("the free alternative is one of the modes offered");
        engine.apply(p0, PlayerAction::ChooseMode(free)).unwrap();
    }

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the Swat asks for a spell or ability, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&swords),
        "the Swords on the stack is what there is to turn: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords],
            },
        )
        .unwrap();

    // The Swat resolves, and the question it asks next is where the Swords
    // points now.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "\"you may choose new targets\": the Swat's caster"
    );
    assert!(
        options.contains(&raptor) && options.contains(&elves),
        "both creatures on the table are legal for the spell being aimed: \
         {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![raptor],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Swords was aimed away, so the Elves it was cast at are still there"
    );
    assert!(
        on_battlefield(&engine, p1, umara_raptor()).is_none(),
        "and the Raptor took the exile instead"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == umara_raptor()))),
        "\"exile target creature\": the redirected spell resolved, so the \
         redirection was a real change of target and not a fizzle"
    );
}

/// Swan Song — {U} instant: "Counter target enchantment, instant, or sorcery spell.
/// Its controller creates a 2/2 blue Bird creature token with flying."
///
/// An opponent casts Dark Ritual, Swan Song counters it into the graveyard
/// and the black mana it would have made never arrives. The Bird is the
/// other half, and it is read on the **opponent**: "Its controller" is the
/// countered spell's controller, so a Bird on this side of the table would
/// be the card paying the wrong player. It is also asserted after the
/// counter has already moved the spell to a graveyard, which is where the
/// effect has to find that player.
#[test]
fn swan_song_counters_an_instant_spell_and_pays_its_caster_a_bird() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[swan_song()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    // p0 passes; p1 casts Dark Ritual during p0's main phase.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("dark ritual is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: ritual })
        .unwrap();

    // p1 passes priority with Dark Ritual on the stack.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // p0 answers: tap Island for {U} and cast Swan Song targeting Dark Ritual.
    tap_all_mana(&mut engine, p0);
    let song = in_hand(&engine, p0, swan_song()).expect("swan song is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: song })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for swan song, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![ritual],
        "Dark Ritual is an instant spell and thus a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        in_graveyard(&engine, p1, dark_ritual()),
        Some(ritual),
        "Dark Ritual was countered into the graveyard"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "Dark Ritual never resolved, so no black mana was added"
    );
    assert_eq!(
        in_graveyard(&engine, p0, swan_song()),
        Some(song),
        "Swan Song resolved into its owner's graveyard"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "\"Its controller\" is the countered spell's controller, not the counterer"
    );
    let theirs = tokens_of(&engine, p1);
    assert_eq!(theirs.len(), 1, "one countered spell, one Bird");
    let bird = theirs[0];
    assert_eq!(pt(&engine, bird), (2, 2), "the printed 2/2");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "\"with flying\""
    );
    let printed = engine
        .state()
        .object(bird)
        .expect("the Bird is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Bird");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Blue),
        "\"a 2/2 blue Bird\" — the pool's other Bird token is white"
    );
}

/// Teferi's Protection — {2}{W} instant: "Until your next turn, your life
/// total can't change and you gain protection from everything. All
/// permanents you control phase out. Exile Teferi's Protection."
///
/// One of those four sentences is expressible and three are the
/// `Coverage::Partial` gap, so the test is the exile plus the shape of what
/// is missing. The exile is read in its own right because it is the clause
/// that was *built and then undone*: `Effect::ExileSource` moved the card
/// off the stack and `finalize_spell` fetched it back into the graveyard,
/// which the rules test beside it
/// ([`super::rules`]) now holds shut.
///
/// The three missing clauses are read as one absence rather than three,
/// and deliberately: "your life total can't change" and "you gain
/// protection from everything" would both be continuous effects, and the
/// spell registers none at all. The phase-out is read on the permanent
/// itself, because `Status::PHASED_OUT` exists and nothing set it — an
/// Elf that is still an ordinary untapped creature after the spell
/// resolved is the printed sentence not happening.
#[test]
fn teferis_protection_exiles_itself_and_leaves_everything_else_exactly_as_it_was() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[teferis_protection()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    let effects_before = engine.state().effects.iter().count();
    let life_before = engine.state().players[0].life;

    cast_from_hand(&mut engine, p0, teferis_protection());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == teferis_protection()))
            }),
        "\"Exile Teferi's Protection\" is the one clause the DSL can say"
    );

    assert_eq!(
        engine.state().effects.iter().count(),
        effects_before,
        "\"your life total can't change\" and \"you gain protection from \
         everything\" are both continuous effects, and the spell registered \
         neither"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "and nothing touched the life total the first of them is about"
    );
    assert!(
        engine
            .state()
            .object(elf)
            .is_some_and(|o| !o.status.contains(Status::PHASED_OUT)),
        "\"All permanents you control phase out\" — the status exists and \
         nothing set it, so the Elf is an ordinary creature still"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and it is still on the battlefield, where a phased-out permanent \
         would also be"
    );
}

/// Crop Rotation — {G} instant: "As an additional cost to cast this spell,
/// sacrifice a land. Search your library for a land card, put that card onto
/// the battlefield, then shuffle."
///
/// This played the whole card until the transcoder was caught dropping the
/// additional cost. The reader emitted the search and nothing else, so what
/// shipped was a one-mana tutor that sacrifices no land — and the played
/// test was green over it, because its graveyard assertion sat inside
/// `if let Some(paid) = sacrificed`. The `CostSacrifice` prompt never fired,
/// the binding stayed `None`, and a conditional assertion asserts nothing.
///
/// The card is an honest stub again, and the engine is why it cannot yet be
/// more than one: `cast_wizard::paid_as_a_mandatory_additional_cost` answers
/// for `PayLife` and `PayLifeX` and nothing else, so a
/// `mandatory_additional_costs` entry naming an object would be *skipped* at
/// cast — the same silence one crate over. #52 is both halves, and the day
/// it lands this goes red and the played test comes back with its assertion
/// out of the `if`.
#[test]
fn crop_rotation_is_a_stub_until_a_spell_can_charge_more_than_mana() {
    use baylee_cards_dsl::Coverage;

    let def = baylee_cards::by_oracle_id("28b46183-c62f-47b1-9fee-3ba148202cab")
        .expect("the registry contains the card");
    assert_eq!(def.index, crop_rotation());
    assert_eq!(
        def.coverage,
        Coverage::Unimplemented,
        "a spell whose printed additional cost nothing charges is a stub"
    );
    assert!(
        def.faces[0].mandatory_additional_costs.is_empty(),
        "and it names no cost part the cast wizard would walk past in silence"
    );
}

/// Hero's Downfall — {1}{B}{B} instant: "Destroy target creature or
/// planeswalker." That filter *is* the whole card, so the scenario puts one
/// creature and one planeswalker across the table with four Islands beside
/// them as the control: the offer is exactly those two permanents and never a
/// land, and what leaves the battlefield is the one that was named. Karn
/// arrives by being *cast* rather than seeded — a permanent put down by
/// `starting_battlefield` is a placement and not an entry, so no replacement
/// effect would hand a planeswalker the loyalty it needs to survive the first
/// state-based check.
#[test]
fn heroes_downfall_destroys_the_creature_or_planeswalker_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .battlefield(
            1,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(1, &[karn_the_great_creator()])
        .start();
    keep_mulligans(&mut engine);

    // The planeswalker the Downfall is for arrives first, off four Islands,
    // and the walk stops the moment p0 may answer it.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, karn_the_great_creator());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, karn_the_great_creator()).is_some()
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("Karn resolved");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");
    let islands = all_on_battlefield(&engine, p1, island());
    assert_eq!(islands.len(), 4, "four Islands paid for the planeswalker");

    // {1}{B}{B} off three Swamps. The target is named before the mana is
    // spent (CR 601.2c before CR 601.2h), so the question comes back first.
    cast_from_hand(&mut engine, p0, heroes_downfall());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature or planeswalker\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster names the target");
    assert_eq!(
        (min, max),
        (1, 1),
        "exactly one permanent, as the card reads"
    );
    assert!(
        player_options.is_empty(),
        "a destroy spell targets no player: {player_options:?}"
    );
    assert!(
        options.contains(&karn),
        "the planeswalker half of the filter: {options:?}"
    );
    assert!(options.contains(&elves), "the creature half: {options:?}");
    assert!(
        islands.iter().all(|land| !options.contains(land)),
        "\"creature or planeswalker\" is read and not skipped: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![karn],
            },
        )
        .expect("the planeswalker the question offered");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, karn_the_great_creator()).is_some()
    });

    assert!(
        on_battlefield(&engine, p1, karn_the_great_creator()).is_none(),
        "the named planeswalker left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, karn_the_great_creator()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and only what was named: the creature beside it still stands"
    );
    assert!(
        in_graveyard(&engine, p0, heroes_downfall()).is_some(),
        "the instant itself resolved into its owner's graveyard"
    );
}

/// The game the owner played, and the first rules hole a player walked into.
///
/// An opponent cast Banishing Stroke at Katara, the Fearless; the owner
/// answered with Heroic Intervention; Katara went to the bottom of the
/// library anyway. All three cards read correctly — the printed removal, the
/// printed protection and Katara's own replacement rule — and the engine
/// simply never asked CR 608.2b as the removal began to resolve.
///
/// The rule itself is tested in `rules`, with a synthetic-sized board and
/// both directions of it. This one is the scenario: the three printings that
/// were in front of a person, played in the order they were played in, so
/// that what he reported is what goes red if it comes back.
#[test]
fn heroic_intervention_answers_banishing_stroke() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(76, forest())
        .battlefield(0, &[forest(), forest(), katara_the_fearless()])
        .hand(0, &[heroic_intervention()])
        .battlefield(
            1,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(1, &[banishing_stroke()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let katara = on_battlefield(&engine, p0, katara_the_fearless()).expect("Katara is out");
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p1, banishing_stroke());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the stroke's aim, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![katara],
        "\"target artifact, creature, or enchantment\": Katara is the only \
         one on the table"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![katara],
            },
        )
        .unwrap();

    // The answer, cast in response — which is the half that was broken. A
    // creature that *already* had hexproof could not have been chosen at
    // all, and that path was tested and worked.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_from_hand(&mut engine, p0, heroic_intervention());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(katara).map(|o| o.zone),
        Some(Zone::Battlefield),
        "Katara is on the bottom of the library with hexproof — the owner's \
         bug, exactly as he reported it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and nothing was put under the library either: the count is the \
         other half of the same claim, because a creature that left the \
         battlefield and a creature that arrived in the library are two \
         readings a single zone check cannot tell apart"
    );
    assert!(
        in_graveyard(&engine, p1, banishing_stroke()).is_some(),
        "the spell that did not resolve went to its owner's graveyard \
         (CR 608.2b)"
    );
}

/// Great Defender: "Target creature gets +0/+X until end of turn, where X is its mana value."
/// A 1/2 Ondu Cleric with mana value 2 is targeted by Great Defender.
/// After the instant resolves, its toughness increases by 2 to 4 while its power remains 1.
#[test]
fn great_defender_boosts_toughness_by_target_mana_value() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[plains(), ondu_cleric()])
        .hand(0, &[great_defender()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("Ondu Cleric deployed");
    assert_eq!(pt(&engine, cleric), (1, 1));

    tap_all_mana(&mut engine, p0);
    let gd = in_hand(&engine, p0, great_defender()).expect("Great Defender in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: gd })
        .expect("one Plains pays {W}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&cleric));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, cleric),
        (1, 3),
        "Ondu Cleric has mana value 2 and gets +0/+2"
    );
}

/// Mana Leak: "Counter target spell unless its controller pays {3}."
/// Seat 0 casts Llanowar Elves; Seat 1 responds by casting Mana Leak.
/// When asked to pay {3} for the tax, Seat 0 declines, causing the creature spell to be countered.
#[test]
fn mana_leak_counters_spell_when_tax_is_declined() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(55, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mana_leak()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("Llanowar Elves in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .unwrap();

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let leak = in_hand(&engine, p1, mana_leak()).expect("Mana Leak in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: leak })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Mana Leak, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![elves]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        panic!("expected PayTax prompt, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(mana, 3);

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "countered creature never arrives"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "countered creature is in graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, mana_leak()).is_some(),
        "resolved Mana Leak is in graveyard"
    );
}

/// Strength of Cedars: "Target creature gets +X/+X until end of turn, where X is the number of lands you control."
/// Cast with five Forests on the battlefield targeting a 1/1 Llanowar Elves, X is evaluated as five.
/// Upon resolution, the creature receives +5/+5 and its power and toughness become 6/6.
#[test]
fn strength_of_cedars_pumps_target_creature_by_lands_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(103, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[strength_of_cedars()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, strength_of_cedars()).expect("Strength of Cedars in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "Elves is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elves),
        (6, 6),
        "five Forests give +5/+5 to the 1/1 Elves"
    );
}

// A pump that **counts**. The three below are one reading of the card-script
// reference — `NumAtt$ ±X` where `X` is a `Count$Valid`, which no reader took
// until `Amount::Negated` gave the sign somewhere to live — and they are
// three cards because the reading has three halves that can each be wrong on
// their own: the direction, whose permanents are counted, and whether "on the
// battlefield" means anybody's. `amount_sign_tests` carries the pool-wide
// claim the three of them cannot make.

/// Irradiate: "-1/-1 until end of turn **for each artifact you control**".
///
/// Two claims on one board. The sign — a shrink, and `resolve::counters::signed`
/// is the only thing that decides it, so reverting that call to the
/// `matches!(a, Amount::NegX | Amount::NegXFixed(_))` it replaced makes this
/// a 8/8 instead of a 4/4 and the card hands out the opposite of what it
/// prints. And the count — the opponent's artifact is on the battlefield too,
/// and `Filter::YOUR_ARTIFACT` is the difference between two and three.
#[test]
fn irradiate_shrinks_by_the_artifacts_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                chromatic_lantern(),
                chromatic_lantern(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[irradiate()])
        .battlefield(1, &[chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    assert_eq!(pt(&engine, wurm), (6, 6), "the premise: a 6/6");

    cast_from_hand(&mut engine, p0, irradiate());
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
        pt(&engine, wurm),
        (4, 4),
        "\"-1/-1 for each artifact you control\" with two of them on your \
         side and one on theirs: down by two, not up by two and not down by \
         three"
    );
}

/// Feeding Frenzy: "-X/-X, where X is the number of Zombies **on the
/// battlefield**".
///
/// The mirror of the card above, and the reason both are here. `Count$Valid
/// Zombie` names no controller, and the transcoder writes that as a
/// `CountOf` over `ZoneSel::Battlefield` — which counts everybody's. A reader
/// that narrowed it to "you control" would be wrong in the direction nothing
/// complains about: the spell still shrinks, just by less.
#[test]
fn feeding_frenzy_counts_every_zombie_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(312, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                festering_goblin(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[feeding_frenzy()])
        .battlefield(1, &[festering_goblin(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, feeding_frenzy());
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
        pt(&engine, wurm),
        (3, 3),
        "one Zombie of yours and two of theirs is three Zombies: a 6/6 \
         becomes a 3/3, and a count that stopped at your own side would \
         leave a 5/5"
    );
}

/// Wirewood Pride: the same reading with no sign in front of it.
///
/// The positive side is the larger half of what this transcoder change
/// reaches — 148 reference scripts against 26 — and it shares every line of
/// the reader with the two above except the wrapper. A test only of the
/// negatives would pass with `Amount::Negated` applied unconditionally.
#[test]
fn wirewood_pride_pumps_by_the_elves_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[forest(), llanowar_elves(), rootbreaker_wurm()])
        .hand(0, &[wirewood_pride()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, wirewood_pride());
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
        pt(&engine, wurm),
        (8, 8),
        "two Elves on the battlefield, one each side: +2/+2 upwards"
    );
}

/// CR 608.2f: a counted amount is read **on resolution**, not when the spell
/// was announced.
///
/// The half a fixed number cannot be wrong about, and the one `Amount::CountOf`
/// makes reachable: `resolve::counters::signed` evaluates through
/// `eval::amount`, which walks the battlefield the engine has at the moment
/// the effect runs. Three Zombies are on the board when Feeding Frenzy is
/// cast and two when it resolves, because the opponent exiled one of their
/// own in response — so the wurm loses two, not three. A reader that had
/// evaluated the count at announcement would pass every other test in this
/// file.
#[test]
fn a_counted_pump_is_read_on_resolution_and_not_on_announcement() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(314, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                festering_goblin(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[feeding_frenzy()])
        .battlefield(1, &[plains(), festering_goblin(), festering_goblin()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, feeding_frenzy());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();

    // Three Zombies are on the battlefield right now. The spell is on the
    // stack and has counted nothing yet.
    let theirs = all_on_battlefield(&engine, p1, festering_goblin());
    assert_eq!(theirs.len(), 2, "two Zombies on their side, one on ours");
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster holds priority over their own spell");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // They answer by exiling one of their own — which is not a Zombie dying,
    // so nothing else goes on the stack.
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![theirs[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (4, 4),
        "three Zombies when it was cast and two when it resolved: a 6/6 \
         becomes a 4/4 (CR 608.2f). A count taken at announcement would have \
         made it a 3/3"
    );
}

/// Evasive Action is {1}{U} and counters a spell unless its controller pays
/// {1} for each **basic land type among lands you control** — the domain
/// count belongs to the instant's own controller, not to the spell's. The
/// three lands under p0 hold only two basic land types (Island and Forest),
/// and that number is only readable against the alternatives: one per land
/// would ask for three, one per land of the spell's controller would ask for
/// one off p1's three Swamps, and only "basic land type" asks for two. The
/// declined tax is the other half — the Ritual never adds its three black and
/// its card ends up in its owner's graveyard.
#[test]
fn evasive_action_taxes_by_the_basic_land_types_of_its_own_controllers_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[evasive_action()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // p1 has to be the active player to cast in a main phase, and its spell
    // has to still be on the stack when the counter is aimed at it.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, dark_ritual());
    let ritual = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack");

    // Priority comes back around to p0 while the Ritual waits to resolve.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, evasive_action());

    // Targets are chosen as the spell is cast (CR 601.2c); with one legal
    // spell on the stack the engine may name it itself.
    if let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p0, "the caster names their own target");
        assert!(
            options.contains(&ritual),
            "the Ritual is the spell on the stack: {options:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ritual],
                },
            )
            .unwrap();
    }

    // The tax is asked when Evasive Action *resolves*, not when it is cast:
    // a counterspell goes on the stack above the Ritual and both seats get
    // priority first (CR 117.3c). Reading the pending straight after the
    // target choice reads that priority window and nothing else.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });

    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Ritual's controller is asked for the domain tax, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the controller of the target spell pays");
    assert_eq!(
        mana, 2,
        "Island and Forest are two basic land types among p0's three lands — \
         three would be one per land and one would be the target's own Swamps"
    );

    // Declining is a real counter: the Ritual's three black never arrive.
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "an unpaid tax counters the spell"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "the three Swamps paid for the Ritual and nothing else, so a Ritual \
         that had resolved would have added {{B}}{{B}}{{B}} on top"
    );
    assert!(
        in_graveyard(&engine, p0, evasive_action()).is_some(),
        "and the instant itself resolved and went to its owner's graveyard"
    );
}

/// Gaea's Might prints one sentence: "{G} — Domain — Target creature gets
/// +1/+1 until end of turn for each basic land type among lands you control."
/// The count is of *types* and not of lands, and it reads only lands **you**
/// control, so the board is built to fail either misreading at once: two
/// Forests and an Island under p0 are three lands carrying two basic land
/// types, while a Swamp and a Mountain across the table would make it four if
/// "you control" were skipped. A 1/1 that ends as a 3/3 is the only answer
/// those two traps leave standing — a per-land count would read 4/4 and a
/// table-wide count 5/5.
#[test]
fn gaeas_might_pumps_by_basic_land_types_among_your_own_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island(), quiet_creature()])
        .hand(0, &[gaea_s_might()])
        .battlefield(1, &[swamp(), mountain(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");

    cast_from_hand(&mut engine, p0, gaea_s_might());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Elf was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "Forest and Island are two basic land types — not the three lands \
         that carry them, and not the four types the whole table has"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the creature the spell did not name is untouched"
    );
}

/// Chord of Calling — {X}{G}{G}{G} instant with convoke: "Search your library
/// for a creature card with mana value X or less, put it onto the battlefield,
/// then shuffle."
///
/// X is announced and paid for rather than assumed, and that is most of what
/// the scenario measures: four Forests make exactly the four mana of
/// {1}{G}{G}{G}, and the pool is empty by the time the search asks — an engine
/// that charged only {G}{G}{G} would still have one floating. The library is
/// built out of Llanowar Elves, mana value 1, so "a creature card with mana
/// value X or less" is a bound over real cards rather than an empty offer: the
/// search shows creature cards and nothing else, and the one chosen leaves the
/// library and stands on the battlefield.
#[test]
fn chord_of_calling_announces_x_and_chords_a_creature_of_that_mana_value_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    // The 60-card backing deck is the pool the search reads, so it is a
    // creature card of mana value 1 and not a basic land.
    let mut engine = Duel::new(41, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[chord_of_calling()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Mana before the claim: castability is read off the pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests are four green mana, and nothing else is on the board"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and no creature is — which is also why convoke has nothing to offer"
    );

    cast_with_floating(&mut engine, p0, chord_of_calling());
    // CR 601.2b: X is announced before any cost is paid.
    let Pending::ChooseNumber { player, min, max } = engine.pending().clone() else {
        panic!("a spell with {{X}} asks for X, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster announces the value");
    assert!(
        min <= 1 && 1 <= max,
        "X = 1 is one of the offers: {min}..={max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("the value the question itself enumerated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that searched is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a library search and not a discard, a scry or a cost"
    );
    assert!(
        max >= 1 && min <= 1,
        "up to one card may be found: {min}..={max}"
    );
    assert!(
        options.len() > 1,
        "the backing deck is the library, so the search has cards to show: {}",
        options.len()
    );
    for id in &options {
        assert!(
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves())),
            "a creature card of mana value 1 is `X or less` for X = 1, and the \
             library holds nothing else: {id:?}"
        );
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{G}}{{G}}{{G}} with X = 1 is four mana, and four is what the \
         Forests made: an engine that fetched without charging the X would \
         still have one floating"
    );

    let library_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()).len(),
        1,
        "the found creature card is put onto the battlefield, and it is one \
         card: the Elves drawn into hand are not creatures on the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and it came out of the library it was searched in — a fetch that \
         copied the card would leave this at `library_before`"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "it was put onto the battlefield, not into a graveyard"
    );
}

/// Whir of Invention — {X}{U}{U}{U}: "Search your library for an artifact
/// card with mana value X or less, put it onto the battlefield, then
/// shuffle." The third card in the pool to write `Filter::CmcAtMostX`, and
/// the only one of the three that is an **instant** — so that is what this
/// plays, on the opponent's turn, where a sorcery-speed reading would never
/// have offered the spell at all.
///
/// The file is `Coverage::Partial` for improvise, and the shape of that gap
/// is worth naming: improvise would let artifacts pay part of the cost, so
/// what is missing makes the spell *dearer* and never cheaper. Four Islands
/// pay {1}{U}{U}{U} here with nothing left over, which is the full printed
/// price.
#[test]
fn whir_of_invention_is_an_instant_and_finds_an_artifact_within_its_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, quiet_artifact())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[whir_of_invention()])
        .start();
    keep_mulligans(&mut engine);

    // The opponent's main phase, with priority back on p0: an instant may be
    // cast here and a sorcery may not, which is the half of this card a mode
    // flag could get wrong. The active player holds priority first (CR
    // 117.3a), so the walk is to the pass after that and not to the phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, crate::turn::Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, which is {{1}}{{U}}{{U}}{{U}} exactly"
    );
    cast_with_floating(&mut engine, p0, whir_of_invention());
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("X = 1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        !options.is_empty(),
        "Sol Ring is mana value 1 and the bound announced was 1"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "improvise is the half that is missing, and its absence can only make \
         the spell dearer: the four Islands paid the whole printed cost"
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
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact arrived on the battlefield, on the opponent's turn"
    );
}

/// Archdruid's Charm prints three modes and the file builds **one** — "Exile
/// target artifact or enchantment" — because the other two need a search
/// that forks on the found card's type and a mode that targets one creature
/// you control and one you don't. So there is no mode question at all, and
/// that is asserted rather than assumed: a spell offering a choice of one
/// and a spell offering none are different objects, and only one of them is
/// what this file wrote.
///
/// The target is an artifact an **opponent** controls, because
/// `Filter::ARTIFACT_OR_ENCHANTMENT` carries no controller clause and the
/// printing carries none either.
#[test]
fn archdruids_charm_builds_one_of_its_three_modes_and_asks_no_mode_question() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[archdruid_s_charm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, archdruid_s_charm());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "one built mode means the spell goes straight to its target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&theirs),
        "\"target artifact or enchantment\" names no controller"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .expect("a target the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "\"Exile target artifact or enchantment\""
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_none(),
        "exiled and not destroyed: a graveyard is the wrong zone"
    );
}

/// Silence: "Your opponents can't cast spells this turn."
///
/// Two moments, because a lock that was already on would read the same as
/// one that arrived: while Silence is still on the stack the opponent may
/// answer it, and once it has resolved the same card in the same hand is
/// gone from the offer.
///
/// The opponent's spell is a **Brainstorm** and the choice is the whole
/// test. The first draft gave them a Counterspell, which passed against an
/// engine with this rule removed: with Silence resolved there is nothing on
/// the stack, so a counter is refused for having no target and the negative
/// was true of a game that had never heard of Silence. Brainstorm names
/// nothing, so the only thing that can refuse it is the lock — and the
/// `{U}{U}` it would be paid with is asserted to be still floating, because
/// an offer is gated on affordability too.
#[test]
fn silence_takes_an_opponents_spells_away_once_it_has_resolved_and_not_before() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .battlefield(1, &[island(), island()])
        .hand(0, &[silence()])
        .hand(1, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, silence());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);

    let storm = in_hand(&engine, p1, brainstorm()).expect("the Brainstorm is in hand");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1's priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert!(
        legal.castable.contains(&storm),
        "Silence is on the stack and has not resolved: its own controller \
         may still be answered"
    );

    // Let it resolve, then come back to p1 with the stack empty.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && e.state().zones.stack_is_empty()
    });
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "the {{U}}{{U}} is still floating, so the price is not what refuses \
         the Brainstorm below"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1's priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&storm),
        "\"your opponents can't cast spells this turn\""
    );
    assert!(
        legal.castable.is_empty(),
        "and not this one spell in particular: {:?}",
        legal.castable
    );
}

/// Muscle Burst: three, plus the copies of itself lying in every graveyard.
///
/// Two readings meet here and a fixed number would satisfy neither.
/// `Amount::Plus` is the constant the printed sentence says *before* the
/// count, and `Filter::Named` is what makes the count a count of this card
/// rather than of cards — the two Forests seeded into the same graveyard are
/// there to be ignored, and a reader matching `Filter::Any` would pump by
/// five and then by six.
///
/// The second cast is the half only a played card can show: the first copy
/// is in the graveyard by then, so X has grown by exactly one. It also
/// proves the spell never counts *itself* — it is still on the stack while
/// it resolves (CR 608.2m), so the first cast is +3/+3 and not +4/+4.
#[test]
fn muscle_burst_counts_three_plus_the_copies_in_every_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(371, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), rootbreaker_wurm()],
        )
        .hand(0, &[muscle_burst(), muscle_burst()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Two cards in the graveyard that are not this one.
    seed_graveyard(&mut engine, p0, 2);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, muscle_burst());
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
        pt(&engine, wurm),
        (9, 9),
        "no copy in any graveyard yet, so X is the bare 3: a 6/6 becomes a \
         9/9, and a reader counting the two Forests beside it would say 11/11"
    );

    cast_with_floating(&mut engine, p0, muscle_burst());
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
        pt(&engine, wurm),
        (13, 13),
        "the first copy is in the graveyard now, so the second is +4/+4 on \
         top of the first +3/+3"
    );
}

/// The same card, on the half of its sentence that says **all** graveyards.
///
/// `ZoneSel::GraveyardAll` is what the card prints and `GraveyardYou` is the
/// neighbouring spelling that passes the test above without a word of
/// difference: here every copy is in the *opponent's* graveyard, so a count
/// that stopped at your own side would pump by three.
#[test]
fn muscle_burst_counts_the_copies_in_an_opponents_graveyard_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The library filler is the card itself, which is how two copies reach a
    // graveyard nobody cast them from.
    let mut engine = Duel::new(372, muscle_burst())
        .battlefield(0, &[forest(), forest(), rootbreaker_wurm()])
        .hand(0, &[muscle_burst()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p1, 2);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, muscle_burst());
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
        pt(&engine, wurm),
        (11, 11),
        "three plus the two copies in the opponent's graveyard: +5/+5, where \
         a count of your own graveyard alone would be +3/+3"
    );
}

/// Virtue of Knowledge: a permanent entering makes an enter trigger happen
/// twice.
///
/// `ReplacementRule::TriggerMultiplier` is the whole front face, and the only
/// way to see a replacement that multiplies a trigger is to count what the
/// trigger did: Lumra mills four on arrival, so it mills eight here. The
/// Adventure half is refused by name and is not on this board.
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

/// Vastwood Fortification, cast as its **front** face: one +1/+1 counter.
///
/// A modal double-faced card is two cards in one, and the half that is not
/// `play_land_face` is this one: the spell is cast out of the same hand the
/// land would have been played from, and the counter is what says which face
/// the engine took.
#[test]
fn vastwood_fortification_puts_a_counter_on_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(413, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[vastwood_fortification()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, vastwood_fortification());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "one +1/+1 counter, which is the whole of the front face"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "so a 1/1 is a 2/2");
}

/// Revitalizing Repast, front face: a counter **and** indestructible, which
/// only a destruction can tell apart from the counter alone.
///
/// The card is `Coverage::Implemented` and the second clause is the one a
/// test that stopped at the counter would never read — so the Elf is shot at
/// after it is pumped, and survives.
#[test]
fn revitalizing_repast_leaves_its_target_indestructible() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(414, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[revitalizing_repast()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[hero_s_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, revitalizing_repast());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "the counter is on"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, hero_s_downfall());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the removal resolves"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"It gains indestructible until end of turn\" — Destroy does nothing \
         (CR 702.12b), and a card that had only put the counter on would have \
         lost the Elf here"
    );
}

/// Spikefield Hazard, front face: one damage, which is the half it claims.
///
/// The "exile it instead of dying" replacement is refused by name, so what a
/// game shows is a 1/1 taking one damage and going to the graveyard — where
/// the printed card would have exiled it.
#[test]
fn spikefield_hazard_deals_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(415, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[spikefield_hazard()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, spikefield_hazard());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "one damage on a 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and it is in the graveyard, which is the half the card refuses: the \
         printing exiles it instead"
    );
}

/// Malakir Rebirth, played as its **back** face: a land that enters tapped
/// and makes black.
///
/// The front face's granted death trigger is refused by name, so the back is
/// where this printing is testable — and the two halves of what a modal
/// double-faced land carries are exactly `EnterModifier::Tapped` and the mana
/// ability behind it.
#[test]
fn malakir_caverns_enters_tapped_and_makes_black() {
    let (engine, land) = play_land_face(malakir_rebirth(), 1).expect("the back face is a land");
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" is an enter modifier and not a line the \
         harness can place around"
    );
}

/// Assassin's Trophy (`Coverage::Partial`): "Destroy target permanent an
/// opponent controls. Its controller may search their library for a basic
/// land card, put it onto the battlefield, then shuffle."
///
/// Both the destroy half and the optional search half are implemented. The
/// `Coverage::Partial` gap is that the found land enters tapped. This test
/// proves that the permanent is destroyed, that the offer goes to the
/// *target's controller* (p1, not p0), and then documents the gap by
/// asserting the fetched land arrived tapped.
#[test]
fn assassins_trophy_destroys_the_target_and_offers_its_controller_a_basic_land_search() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(77, forest())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[assassin_s_trophy()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    let lands_before = lands_of(&engine, p1).len();

    cast_from_hand(&mut engine, p0, assassin_s_trophy());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Trophy asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "their spell, their target choice");
    assert!(
        options.contains(&victim),
        "the opponent's creature is a permanent an opponent controls: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elf is a legal target");

    // Wait for the search question, which belongs to the Elf's controller (p1).
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
        panic!("expected the basic-land search, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p1,
        "\"Its controller\" is the target's controller (p1), not the caster (p0)"
    );
    assert_eq!((min, max), (0, 1), "\"may search\" — zero or one card");
    assert!(
        !options.is_empty(),
        "the library holds basics to search for"
    );

    // The destroy half happened before the search was offered.
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "\"Destroy target permanent\" — the Elf is no longer on the battlefield"
    );

    // Take the land; verify it arrived tapped (the Coverage::Partial gap).
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("p1 takes the basic land");
    pass_until(&mut engine, |e| lands_of(e, p1).len() > lands_before);

    let fetched = *lands_of(&engine, p1)
        .last()
        .expect("the fetched land arrived");
    assert!(
        is_tapped(&engine, fetched),
        "Coverage::Partial gap: the fetched land enters tapped, \
         which the printed Oracle text does not say"
    );
}

/// Beyeen Veil (`Coverage::Implemented`): "Creatures your opponents control
/// get -2/-0 until end of turn."
///
/// One creature on each side confirms "your opponents" is read correctly and
/// that only power is reduced — a 1/1 opponent creature becomes -1/1, while
/// the caster's own 1/1 remains untouched at (1, 1).
#[test]
fn beyeen_veil_shrinks_only_opponent_creatures_by_two_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .hand(0, &[beyeen_veil()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is on the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is on the table");

    assert_eq!(pt(&engine, mine), (1, 1), "a 1/1 before the spell");
    assert_eq!(pt(&engine, theirs), (1, 1), "a 1/1 before the spell");

    cast_from_hand(&mut engine, p0, beyeen_veil());
    // The spell has no targets, so nothing is asked on the way to the stack.
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, theirs),
        (-1, 1),
        "the opponent's 1/1 becomes -1/1 under -2/+0"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"your opponents\" — the caster's own creature is untouched"
    );
}

/// Fire // Ice (`Coverage::Partial`): Ice reads "Tap target permanent. Draw a
/// card." — this is the implemented half. Fire ("deals 2 damage divided as you
/// choose among one or two targets") is not supported because the DSL cannot
/// divide an amount among targets.
///
/// The test casts Ice, confirms the permanent it targets becomes tapped and
/// that exactly one card is drawn. The stack is also checked to be empty
/// afterward — Ice is an instant and belongs in the graveyard.
#[test]
fn ice_taps_a_permanent_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(23, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[fire()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    assert!(!is_tapped(&engine, elf), "the Elf starts untapped");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Cast Ice (face 1) off floating mana. The face question is asked only
    // when there is a choice to make, and here there is not: Fire divides an
    // amount among targets, which the DSL cannot say, so Ice is the only
    // castable half and the engine goes straight to its target. Answering a
    // question nobody asked is how this test first failed.
    cast_from_hand(&mut engine, p0, fire());
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let ice_slot = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Face(1)))
            .expect("Ice (face 1) is one of the options");
        engine
            .apply(p0, PlayerAction::ChooseMode(ice_slot))
            .expect("choosing the Ice face is legal");
    }

    // "Tap target permanent" — the engine asks for a target.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Ice asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "any permanent on the battlefield is a legal target: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, elf),
        "\"Tap target permanent\" — the Elf is tapped"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and \"Draw a card\" put one back — net zero"
    );
    assert!(
        in_graveyard(&engine, p0, fire()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// Ghoul's Feast (`Coverage::Implemented`): "Target creature gets +X/+0 until
/// end of turn, where X is the number of creature cards in your graveyard."
///
/// The library filler is `quiet_creature()` (Llanowar Elves), so one card
/// seeded from the library lands as a creature card in p0's graveyard, giving
/// X = 1. A land card seeded into p1's graveyard confirms "your graveyard"
/// is read and the opponent's creatures are not counted. Power rises by 1,
/// toughness stays unchanged.
#[test]
fn ghouls_feast_pumps_a_creature_by_the_count_of_your_graveyard_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Using quiet_creature() as the library filler so seeded graveyard cards
    // are creature cards, making X = 1 when one is seeded for p0.
    let mut engine = Duel::new(43, quiet_creature())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[ghoul_s_feast()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    assert_eq!(pt(&engine, target), (1, 1), "a 1/1 before the feast");

    // Put one creature card into p0's graveyard so X = 1.
    seed_graveyard(&mut engine, p0, 1);
    // Put a card into p1's graveyard too; it is also a creature card but it
    // is not in *p0's* graveyard, so the spell must not count it.
    seed_graveyard(&mut engine, p1, 1);

    cast_from_hand(&mut engine, p0, ghoul_s_feast());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Feast asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target),
        "the Elf is a legal creature target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target),
        (2, 1),
        "+1/+0 for the one creature card in p0's graveyard; \
         the card in p1's graveyard is not counted"
    );
}

/// Jwari Disruption // Jwari Ruins (`Coverage::Implemented`): "Counter target
/// spell unless its controller pays {1}." The back face enters tapped and
/// taps for {U}.
///
/// p1 casts a Dark Ritual; p0 holds an Island and answers with Jwari
/// Disruption. The tax question goes to p1 (the targeted spell's controller),
/// not to p0. When p1 declines, the Ritual is countered and ends up in p1's
/// graveyard. The mana pool is checked to confirm that declining means nothing
/// was spent — the Ritual itself never resolved either.
#[test]
fn jwari_disruption_counters_unless_its_controller_pays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[jwari_disruption()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 casts Dark Ritual. The pool is read here and again at the end,
    // because `cast_from_hand` taps *every* land: p1 has two Swamps and the
    // Ritual costs one, so a bare `== 0` at the end measures the harness'
    // leftover change rather than the spell. What the card is about is that
    // the pool does not *grow* by three.
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("Ritual is in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    let pool_before = engine.state().players[1].mana_pool.total();

    // p0 responds with Jwari Disruption.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p0, jwari_disruption());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Disruption asks for a target spell, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "their spell, their target choice");
    assert!(
        options.contains(&ritual),
        "the Dark Ritual on the stack is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the Ritual is a legal target");

    // Both pass; Disruption resolves and the tax question goes to p1.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("pass_until stopped on the tax question")
    };
    assert_eq!(*mana, 1, "Jwari Disruption prints a {{1}} tax");
    assert_eq!(
        *player, p1,
        "\"unless its controller pays\" — the tax goes to the targeted spell's controller"
    );

    // p1 declines; the Ritual is countered.
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p0, jwari_disruption()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    // The tax went unpaid and the Ritual was countered, so it added no
    // {B}{B}{B} on the way through.
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before,
        "a countered Dark Ritual adds nothing to its controller's pool"
    );
}

/// Kabira Takedown // Kabira Plateau (`Coverage::Implemented`): "Kabira
/// Takedown deals damage equal to the number of creatures you control to
/// target creature or planeswalker."
///
/// p0 controls two creatures when they cast the spell, so the target takes
/// 2 damage. The opponent's creature (not controlled by p0) must not be
/// counted in the tally — "you control" is the filter. A 1/1 target survives
/// the first hit but dies at 2, which confirms the arithmetic is what the
/// board says it is.
#[test]
fn kabira_takedown_deals_damage_equal_to_creatures_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[kabira_takedown()])
        // The opponent's creature must not count toward p0's total.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");

    // p0 controls two Llanowar Elves, so the spell deals exactly 2 damage.
    cast_from_hand(&mut engine, p0, kabira_takedown());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Takedown asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target),
        "target creature or planeswalker — the Elf qualifies: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "2 damage on a 1/1 is lethal — the Elf is gone from the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a destroyed creature goes to its owner's graveyard"
    );
}

/// Legion Leadership // Legion Stronghold (`Coverage::Implemented`): "Until
/// end of turn, double target creature's power and it gains first strike."
///
/// The spell doubles the target's *current* power (`Amount::TargetPower`)
/// and grants first strike. A 2/2 Llanowar Elves on the table (seeded with a
/// +1/+1 counter via the counter helper) becomes a 4/2 with first strike.
/// The toughness is unchanged, ruling out an accidental toughness doubling.
#[test]
fn legion_leadership_doubles_power_and_grants_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .hand(0, &[legion_leadership()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let creature = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is deployed");

    // Put a +1/+1 counter on the Elf so its current power is 2 rather than
    // the printed 1 — a doubled 1 and a doubled 2 are different numbers,
    // which is what makes this assertion not trivially satisfied.
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::replacement::put_counters(state, creature, CounterKind::P1P1, 1);
    }
    engine.refresh_offer();
    // The Elf is not read here. `record_counters` invalidates the
    // projections, but `refresh_offer` recomputes the *legal actions* and
    // not the layers, so `characteristics()` would still answer 1/1 until
    // the engine runs a pass of its own. The claim this test makes — that
    // the doubling reads the creature's current power and not its printed
    // one — is carried by the (4, 2) at the end instead, which is a
    // different number from the (2, 1) a printed 1 would have doubled to.

    cast_from_hand(&mut engine, p0, legion_leadership());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Legion Leadership asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&creature),
        "the Elf is a legal creature target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, creature),
        (4, 2),
        "power doubled from 2 to 4; toughness is unchanged"
    );
    assert!(
        keywords(&engine, creature).contains(KeywordSet::FIRST_STRIKE),
        "the spell also grants first strike until end of turn"
    );
}

/// Lose Focus (`Coverage::Partial`): "Counter target spell unless its
/// controller pays {2}." (Replicate {U} is the `Coverage::Partial` gap —
/// the card cannot copy itself.)
///
/// The counter clause is fully implemented. p1 casts a Dark Ritual; p0
/// answers with Lose Focus. The tax question goes to the targeted spell's
/// controller (p1). When p1 declines, the Ritual is countered. The mana pool
/// after resolution proves the Ritual never added its {B}{B}{B}.
/// A single copy of Lose Focus sits on the stack before resolution —
/// confirming the replicate gap is real and not just an unfired trigger.
#[test]
fn lose_focus_counters_the_targeted_spell_when_the_controller_declines_to_pay_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[lose_focus()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 casts Dark Ritual.
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual is in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());

    // p0 taps both Islands and counters with Lose Focus.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p0, lose_focus());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Lose Focus asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "their spell, their target choice");
    assert!(
        options.contains(&ritual),
        "the Ritual on the stack is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the Ritual is a legal target");

    // Replicate gap: one spell on the stack, not two.
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(
        stack.len(),
        2,
        "the Ritual and Lose Focus are on the stack — and no replicate copy, \
         which is the Coverage::Partial gap: {stack:?}"
    );

    // Both pass; Lose Focus resolves and the tax is offered to p1.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("pass_until stopped on the tax question")
    };
    assert_eq!(*mana, 2, "Lose Focus prints a {{2}} tax");
    assert_eq!(
        *player, p1,
        "\"unless its controller pays\" — the tax belongs to the targeted spell's controller"
    );

    // p1 declines — the Ritual is countered.
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "the Ritual was countered and goes to p1's graveyard"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the tax went unpaid and the Ritual was countered — no mana floats"
    );
}

/// Razorgrass Ambush // Razorgrass Field — `{1}{W}` Instant // Land.
/// The instant face reads "Razorgrass Ambush deals 3 damage to target
/// attacking or blocking creature." The land face has a life-payment
/// enter trigger and `{T}: Add {W}`.
///
/// Both faces are a GENERATED STUB — no abilities are implemented. The
/// instant face declares no targeting, so the stub casts as a vanilla
/// spell and the test can only confirm that the card reaches the stack and
/// resolves into the graveyard. The 3-damage effect, the mandatory combat
/// target, and the land-face life-payment trigger are all absent; this
/// scenario proves nothing about those clauses.
///
/// SKIP: the instant effect — "deals 3 damage to target attacking or
/// blocking creature" — needs a creature in combat as a target, which
/// requires a full combat phase and a helper to read the damage counter.
/// The land-face enter trigger (pay 3 life or enter tapped) needs
/// Razorgrass Ambush // Razorgrass Field (`Coverage::Partial`): "Razorgrass
/// Ambush deals 3 damage to target attacking or blocking creature."
///
/// The blocking half is the partial — the DSL has no filter for a blocking
/// creature — so the implemented sentence is the attacking one, and reaching
/// it needs a real combat. That is the whole point of the test: the spell is
/// **not castable** with nothing attacking, which is how this card first
/// read as an unimplemented stub to a reader that only tried to cast it on
/// an empty board.
#[test]
fn razorgrass_ambush_burns_a_creature_that_is_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(301, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[razorgrass_ambush()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nothing is attacking yet, so there is no legal target and the spell
    // cannot be cast at all — the engine refuses it rather than offering an
    // empty target list. The mana is tapped first on purpose: without it the
    // refusal would be affordability and this assertion would prove nothing.
    let spell = in_hand(&engine, p0, razorgrass_ambush()).expect("the Ambush is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        engine.state().players[0].mana_pool.total() >= 2,
        "two Plains pay {{1}}{{W}}, so what is refused below is the target"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "with no attacking creature the Ambush has nothing to target"
    );

    // A second game for the combat, because those two Plains are tapped now
    // and a seat's lands untap in its **own** untap step (CR 502.1) — on p1's
    // turn p0 would have had no mana, and "not castable" would have meant
    // something else entirely.
    let mut engine = Duel::new(302, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[razorgrass_ambush()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // p1's turn, and the Elf swings.
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    assert!(
        attackers.contains(&elf),
        "a creature that has been on the battlefield since before the game \
         may attack (CR 302.6): {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("attacking the other seat is legal");

    // Now p0 has a target, and three damage on a 1/1 is lethal.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, razorgrass_ambush());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the Ambush asks for an attacking creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "the attacking Elf is the legal target: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the attacking Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage on a 1/1 is lethal (CR 704.5g)"
    );
}

/// Sejiri Shelter // Sejiri Glacier — `{1}{W}` Instant // Land.
/// The instant face reads "Target creature you control gains protection
/// from the color of your choice until end of turn." The land face enters
/// tapped and taps for `{W}`.
///
/// The card is `Coverage::Partial`: the land face taps for `{W}` and the
/// instant face has no ability at all, because protection from a colour of
/// your choice is not something the DSL can say. So the instant face casts
/// as a vanilla spell, and this test confirms only that the card reaches
/// the stack and resolves into the graveyard. The protection grant and the
/// `ChooseColor` question that must follow the target choice are both
/// absent; this scenario proves nothing about those clauses.
///
/// SKIP: the instant effect — "target creature you control gains
/// protection from the color of your choice" — requires a `ChooseTargets`
/// step for the creature and a `ChooseColor` step for the protection
/// color. Both depend on the targeting and protection-grant ability being
/// present in the card definition.
#[test]
fn sejiri_shelter_stub_casts_and_resolves_into_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(302, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[sejiri_shelter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The stub declares no target, so cast_from_hand succeeds. This only
    // confirms the card is registered and the engine can advance past it.
    // The real test must be written once the protection-grant ability and
    // the color-choice prompt are implemented.
    cast_from_hand(&mut engine, p0, sejiri_shelter());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sejiri_shelter()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// Tear Asunder — `{1}{G}` Instant with kicker `{1}{B}`.
/// "Exile target artifact or enchantment. If this spell was kicked,
/// exile target nonland permanent instead."
///
/// The card is a GENERATED STUB — no abilities are implemented. The stub
/// declares no targeting, so it casts as a vanilla spell. The test can
/// only confirm the card is registered and resolves into the graveyard.
/// The exile effect, the artifact-or-enchantment targeting filter, the
/// kicker mechanic, and the broadened "nonland permanent" targeting when
/// kicked are all absent; this scenario proves nothing about those clauses.
///
/// SKIP: the core effect — "exile target artifact or enchantment" — needs
/// a `ChooseTargets` step filtered to artifacts and enchantments. The
/// kicker variant needs the kicker declaration during casting and a
/// second, broader targeting step. Both depend on the exile ability and
/// Tear Asunder (`Coverage::Partial`): "Exile target artifact or
/// enchantment."
///
/// Kicker `{1}{B}` and the widened "exile target nonland permanent instead"
/// are the partial — a spell carries one target requirement, so the kicked
/// mode cannot widen it — and what is left is a plain exile with a filter on
/// it. The filter is the thing worth playing: an empty board makes the spell
/// uncastable, and a Mox Opal makes it castable, which is the same sentence
/// read from both sides.
#[test]
fn tear_asunder_exiles_an_artifact_and_refuses_an_empty_board() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(303, forest())
        .battlefield(0, &[forest(), swamp()])
        .hand(0, &[tear_asunder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nothing to exile: the target requirement is unsatisfiable and the
    // spell is refused rather than cast into nothing.
    let spell = in_hand(&engine, p0, tear_asunder()).expect("the spell is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "with no artifact and no enchantment on the battlefield there is \
         nothing for it to target"
    );

    // The same spell on a board with a Mox Opal on it.
    let mut engine = Duel::new(303, forest())
        .battlefield(0, &[forest(), swamp(), mox_opal()])
        .hand(0, &[tear_asunder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mox = on_battlefield(&engine, p0, mox_opal()).expect("the Mox is on the battlefield");
    cast_from_hand(&mut engine, p0, tear_asunder());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Tear Asunder asks for an artifact or enchantment, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&mox),
        "an artifact is what the filter admits: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![mox] })
        .expect("the Mox is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, mox_opal()).is_none(),
        "an exiled permanent leaves the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, mox_opal()).is_none(),
        "exile is not the graveyard (CR 406.1)"
    );
}

/// Fell the Profane // Fell Mire (`Coverage::Implemented`): "Destroy target
/// creature or planeswalker. // As this land enters, you may pay 3 life. If
/// you don't, it enters tapped. {T}: Add {B}."
///
/// The front face destroys a creature or planeswalker. The test casts Fell the
/// Profane targeting an opponent's Llanowar Elves, verifies the target is
/// destroyed upon resolution, and checks that both the destroyed creature and
/// the spell card arrive in their owners' graveyards.
#[test]
fn fell_the_profane_destroys_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(48, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[fell_the_profane()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");

    cast_from_hand(&mut engine, p0, fell_the_profane());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target creature or planeswalker — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature is no longer on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the destroyed creature is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, fell_the_profane()).is_some(),
        "the resolved spell is in its caster's graveyard"
    );
}

/// Hagra Mauling // Hagra Broodpit (`Coverage::Partial`): "This spell costs
/// {1} less to cast if an opponent controls no basic lands. Destroy target
/// creature. // This land enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Partial`, the conditional cost reduction is omitted,
/// but the creature destruction is implemented in full. The test casts Hagra
/// Mauling for its printed cost of `{2}{B}{B}`, targets an opponent's creature,
/// and confirms that the creature is destroyed upon resolution.
#[test]
fn hagra_mauling_destroys_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(49, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hagra_mauling()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");

    cast_from_hand(&mut engine, p0, hagra_mauling());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target creature — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature is destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the destroyed creature is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, hagra_mauling()).is_some(),
        "the resolved spell is in its caster's graveyard"
    );
}

/// Inner Calm, Outer Strength (`Coverage::Implemented`): "Target creature
/// gets +X/+X until end of turn, where X is the number of cards in your hand."
///
/// The spell card is on the stack during resolution rather than in hand, so
/// two remaining cards in hand grant +2/+2. The test targets a 1/1 Llanowar
/// Elves and confirms its projected power and toughness become 3/3.
#[test]
fn inner_calm_outer_strength_pumps_target_creature_by_cards_in_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[inner_calm_outer_strength(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is deployed");
    assert_eq!(pt(&engine, elf), (1, 1), "starts as a 1/1 creature");

    cast_from_hand(&mut engine, p0, inner_calm_outer_strength());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target creature — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "two cards remaining in hand give +2/+2, turning the 1/1 into a 3/3"
    );
    assert!(
        in_graveyard(&engine, p0, inner_calm_outer_strength()).is_some(),
        "resolved spell moves to the graveyard"
    );
}

/// Kazuul's Fury // Kazuul's Cliffs (`Coverage::Partial`): "As an additional
/// cost to cast this spell, sacrifice a creature. Kazuul's Fury deals damage
/// equal to the sacrificed creature's power to any target. // This land enters
/// tapped. {T}: Add {R}."
///
/// Under `Coverage::Partial`, the front-face sacrifice cost and damage are
/// not implemented, but the back face (Kazuul's Cliffs) is built in full.
/// The test plays the back face as a land, confirms it enters tapped, advances
/// to the next turn so it untaps, and activates its mana ability to add `{R}`.
#[test]
fn kazuuls_cliffs_enters_tapped_and_taps_for_red_mana() {
    let (mut engine, cliffs) =
        play_land_face(kazuul_s_fury(), 1).expect("plays as Kazuul's Cliffs");
    assert!(is_tapped(&engine, cliffs), "Kazuul's Cliffs enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, cliffs), "untaps on next turn");
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);

    activate(&mut engine, p0, kazuul_s_fury(), 0);

    assert!(
        is_tapped(&engine, cliffs),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        red_before + 1,
        "adds one red mana to the pool"
    );
}

/// Khalni Ambush // Khalni Territory: "Target creature you control fights
/// target creature you don't control. // This land enters tapped. {T}: Add
/// {G}."
///
/// The back face. The test plays it as a land, asserts that it enters
/// tapped, advances to the next turn so it untaps, and activates its mana
/// ability to add `{G}`; the front face is the test below.
#[test]
fn khalni_territory_enters_tapped_and_taps_for_green_mana() {
    let (mut engine, territory) =
        play_land_face(khalni_ambush(), 1).expect("plays as Khalni Territory");
    assert!(
        is_tapped(&engine, territory),
        "Khalni Territory enters tapped"
    );

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, territory), "untaps on next turn");
    let green_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);

    activate(&mut engine, p0, khalni_ambush(), 0);

    assert!(
        is_tapped(&engine, territory),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        green_before + 1,
        "adds one green mana to the pool"
    );
}

/// Khalni Ambush, the front face: "Target creature you control fights target
/// creature you don't control."
///
/// Two instances of the word "target", asked one after the other, each with
/// its own menu — the first holds only my creatures and the second only
/// theirs, which is the printing's "you control" / "you don't control". Then
/// a 4/4 fights a 2/2: each deals damage equal to its power to the other
/// (CR 701.14a), so the 2/2 dies and the 4/4 keeps two damage.
#[test]
fn khalni_ambush_makes_my_creature_fight_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4726, forest())
        .battlefield(0, &[forest(), forest(), forest(), fangren_hunter()])
        .battlefield(1, &[wild_colos()])
        .hand(0, &[khalni_ambush()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, khalni_ambush());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the first target question, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&hunter), "my creature is the fighter");
    assert!(
        !options.contains(&colos),
        "theirs is not \"a creature you control\""
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hunter],
                players: vec![],
            },
        )
        .expect("the Hunter was offered");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the second target question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![colos],
        "only a creature I don't control is the foe"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![colos],
                players: vec![],
            },
        )
        .expect("the Colos was offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wild_colos()).is_some(),
        "four damage kill the 2/2"
    );
    assert!(on_battlefield(&engine, p0, fangren_hunter()).is_some());
    assert_eq!(
        engine.state().object(hunter).map(|o| o.damage),
        Some(2),
        "and the 2/2 dealt its two back"
    );
    assert!(in_graveyard(&engine, p0, khalni_ambush()).is_some());
}

/// Krosan Grip (`Coverage::Partial`): "Split second. Destroy target artifact
/// or enchantment."
///
/// Under `Coverage::Partial`, split second has no DSL representation, but
/// artifact/enchantment destruction is implemented in full. The test casts
/// Krosan Grip targeting the opponent's Sol Ring (`quiet_artifact`), confirms
/// creature permanents are not valid targets, and asserts the artifact is
/// destroyed.
#[test]
fn krosan_grip_destroys_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(43, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[krosan_grip()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("opponent controls Sol Ring");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls an Elf");

    cast_from_hand(&mut engine, p0, krosan_grip());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ring),
        "target artifact or enchantment — Sol Ring qualifies"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("Sol Ring is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the targeted artifact is destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the destroyed artifact is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, krosan_grip()).is_some(),
        "the resolved spell is in its caster's graveyard"
    );
}

/// Rush of Inspiration // Crackling Falls (`Coverage::Partial`): "Draw two
/// cards. Then discard a card at random unless you pay {E}{E}. // This land
/// enters tapped. {T}: Add {U} or {R}."
///
/// Under `Coverage::Partial`, the energy payment and random discard are not
/// expressible in the DSL, but the "draw two cards" effect is implemented.
/// The test casts Rush of Inspiration off three Islands and verifies the
/// caster's hand grows by one card after spending the spell.
#[test]
fn rush_of_inspiration_draws_two_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[rush_of_inspiration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, rush_of_inspiration());
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "casting costs one card from hand and draws two cards, giving a net gain of one"
    );
    assert!(
        in_graveyard(&engine, p0, rush_of_inspiration()).is_some(),
        "resolved spell is in the graveyard"
    );
}

/// Silundi Vision // Silundi Isle (`Coverage::Partial`): "Look at the top six
/// cards of your library. You may reveal an instant or sorcery card from among
/// them and put it into your hand. Put the rest on the bottom of your library in
/// a random order. // This land enters tapped. {T}: Add {U}."
///
/// Under `Coverage::Partial`, `Effect::LookAtTopPick` does not filter by card
/// type and bottoms the rest by player choice. The test casts Silundi Vision,
/// verifies that six cards are offered from the library, puts one into hand,
/// orders the rest to the bottom, and confirms the chosen card reached the hand.
#[test]
fn silundi_vision_looks_at_top_six_and_puts_one_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(45, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[silundi_vision()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, silundi_vision());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected card choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options.len(),
        6,
        "looks at the top six cards of the library"
    );
    assert_eq!((min, max), (1, 1), "picks exactly one card");
    let chosen = options[0];
    let remaining = options[1..].to_vec();

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("picks one card from the looked-at cards");

    let Pending::Arrange {
        player,
        cards,
        piles,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ordering of remaining cards, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(cards.len(), 5, "five remaining cards to put on bottom");
    assert_eq!(
        piles,
        vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, 5)],
        "one pile, the bottom, and every card goes into it"
    );
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![remaining],
            },
        )
        .expect("orders the remaining cards to the bottom");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "the chosen card was put into hand"
    );
    assert!(
        in_graveyard(&engine, p0, silundi_vision()).is_some(),
        "resolved spell is in the graveyard"
    );
}

/// Sink into Stupor // Soporific Springs (`Coverage::Implemented`): "Return
/// target spell or nonland permanent an opponent controls to its owner's hand.
/// // As this land enters, you may pay 3 life. If you don't, it enters tapped.
/// {T}: Add {U}."
///
/// The front face returns a nonland permanent an opponent controls to hand.
/// The test casts Sink into Stupor targeting the opponent's Llanowar Elves,
/// confirms that an opponent land is not offered as a valid target, and
/// verifies the bounced creature returns to the opponent's hand.
#[test]
fn sink_into_stupor_returns_opponent_nonland_permanent_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(46, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[sink_into_stupor()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");
    let opponent_land = on_battlefield(&engine, p1, forest()).expect("opponent's Forest");

    cast_from_hand(&mut engine, p0, sink_into_stupor());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target nonland permanent an opponent controls — the Elf qualifies"
    );
    assert!(
        !options.contains(&opponent_land),
        "an opponent's land is not a nonland permanent"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the bounced creature is no longer on the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "the bounced creature was returned to its owner's hand"
    );
    assert!(
        in_graveyard(&engine, p0, sink_into_stupor()).is_some(),
        "resolved spell moves to the graveyard"
    );
}

/// Sultai Charm (`Coverage::Implemented`): "Choose one — • Destroy target
/// monocolored creature. • Destroy target artifact or enchantment. • Draw two
/// cards, then discard a card."
///
/// The test casts Sultai Charm, selects the first mode ("Destroy target
/// monocolored creature"), targets an opponent's monocolored Llanowar Elves,
/// and verifies that the creature is destroyed upon resolution.
#[test]
fn sultai_charm_destroys_target_monocolored_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, forest())
        .battlefield(0, &[swamp(), forest(), island()])
        .hand(0, &[sultai_charm()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");

    cast_from_hand(&mut engine, p0, sultai_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .expect("mode 0 is offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(slot))
        .expect("chooses mode 0");

    let Pending::ChooseTargets {
        options: targets, ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        targets.contains(&elf),
        "target monocolored creature — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted monocolored creature was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the destroyed creature is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, sultai_charm()).is_some(),
        "the resolved charm is in its caster's graveyard"
    );
}

/// Valakut Awakening // Valakut Stoneforge (`Coverage::Partial`): "Put any number
/// of cards from your hand on the bottom of your library, then draw that many
/// cards plus one. // This land enters tapped. {T}: Add {R}."
///
/// Under `Coverage::Partial`, the front-face hand cycling is not expressible
/// in the DSL, but the back face (Valakut Stoneforge) is built in full. The test
/// plays the back face as a land, confirms it enters tapped, advances to the
/// next turn so it untaps, and activates its mana ability to add `{R}`.
#[test]
fn valakut_stoneforge_enters_tapped_and_taps_for_red_mana() {
    let (mut engine, stoneforge) =
        play_land_face(valakut_awakening(), 1).expect("plays as Valakut Stoneforge");
    assert!(
        is_tapped(&engine, stoneforge),
        "Valakut Stoneforge enters tapped"
    );

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, stoneforge), "untaps on next turn");
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);

    activate(&mut engine, p0, valakut_awakening(), 0);

    assert!(
        is_tapped(&engine, stoneforge),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        red_before + 1,
        "adds one red mana to the pool"
    );
}

/// `Waterlogged Teachings` // `Inundated Archive` (`Coverage::Implemented`): "Search your
/// library for an instant card or a card with flash, reveal it, put it into your hand,
/// then shuffle. // This land enters tapped. {T}: Add {U} or {B}."
///
/// Marked `Coverage::Implemented`, casting the front face for `{3}{U/B}` triggers
/// `Effect::SearchLibrary` for an instant or flash card. With a library filled with
/// `counterspell()`, `Pending::ChooseCards` offers the instant card, which is chosen
/// and added to hand, while `Waterlogged Teachings` moves to the graveyard.
#[test]
fn waterlogged_teachings_searches_library_for_instant_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(471, counterspell())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[waterlogged_teachings()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, waterlogged_teachings());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (1, 1), "mandatory search for one card");
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

    assert!(
        in_hand(&engine, p0, counterspell()).is_some(),
        "the searched instant was put into hand"
    );
    assert!(
        in_graveyard(&engine, p0, waterlogged_teachings()).is_some(),
        "Waterlogged Teachings resolved and moved to graveyard"
    );
}

/// Sejiri Glacier, which is the half of Sejiri Shelter that is implemented.
///
/// The card's own test above can only watch the instant face resolve into a
/// graveyard, because "target creature you control gains protection from the
/// colour of your choice" is not something the DSL can say and the front
/// face carries no ability at all. That leaves the **land** face carrying
/// everything this card actually does — enters tapped, taps for `{W}` — and
/// nothing was playing it. A `Coverage::Partial` card is exactly where that
/// happens: the refusal is written down, so the half that works stops being
/// looked at.
#[test]
fn sejiri_glacier_enters_tapped_and_makes_white() {
    let seat = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(sejiri_shelter(), 1).expect("the back face is a land and may be played");

    assert!(
        is_tapped(&engine, land),
        "Sejiri Glacier prints \"this land enters tapped\""
    );

    // A land untaps in its controller's own untap step (CR 502.1), which is
    // the next turn but one — and then it makes white.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && e.state().turn.active == seat && !is_tapped(e, land)
    });
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("a printed mana ability is offered on the land it is printed on");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "`{{T}}: Add {{W}}` puts one white mana in the pool"
    );
    assert!(is_tapped(&engine, land), "and the land is tapped for it");
}

fn ancestral_recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

/// Ancestral Recall — {U} — Instant: "Target player draws three cards."
///
/// The word that decides the card is "target": the three cards belong to the
/// player the spell names, so the scenario aims it across the table and reads
/// both libraries afterwards. The caster's own library is the control that
/// keeps "a draw" from passing as "a draw for everybody", and the target
/// question is read while it still stands, which is where "target player"
/// has to offer both seats rather than only the one across the table.
#[test]
fn ancestral_recall_draws_three_for_the_player_it_names_and_not_for_the_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[ancestral_recall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine_before = library_size(&engine, p0);
    let theirs_before = library_size(&engine, p1);
    let my_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let their_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, ancestral_recall());

    // The announcement is **atomic in this engine**, and that is worth
    // pinning rather than working around. CR 601.2a moves the card to the
    // stack before CR 601.2c asks for a target; `cast_wizard` asks first and
    // moves at the end, so while the question stands the card is still in
    // hand. Nothing can see the difference: no player is given priority
    // until CR 601.2i, and CR 115.5 makes a spell an illegal target of
    // itself, so the one list that would have held the card holds nothing
    // either way. If this assertion ever fails, the wizard has started
    // moving the card first and the reason above is the thing to re-read.
    assert!(
        on_stack(&engine, ancestral_recall()).is_none()
            && in_hand(&engine, p0, ancestral_recall()).is_some(),
        "the card is where it was until the wizard finishes"
    );

    // A target that is only ever a player asks `ChoosePlayer` rather than
    // `ChooseTargets`: two pendings, and the DSL spec picks between them —
    // `TargetSpec::AnyPlayer` here, against `AnyTarget`, which offers both
    // lists at once because CR 115.4 lets it name a creature too.
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "\"target player\" is a player choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat names the player");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "\"target player\" reaches either seat, its own included: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent was one of the players the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        theirs_before - 3,
        "\"target player draws three cards\": three off the top of that player's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand_before + 3,
        "and the cards are in the target's hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand_before - 1,
        "the caster's hand only lost the spell it cast"
    );
    assert_eq!(
        library_size(&engine, p0),
        mine_before,
        "and the caster's library never moved, which is what tells \"target \
         player\" from \"you\""
    );
    assert!(
        in_graveyard(&engine, p0, ancestral_recall()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
    assert!(
        engine.state().players[0].mana_pool.total() == 0,
        "the {{U}} was paid"
    );
}

fn annul() -> CardIndex {
    card_index("d08e9784-75f7-4164-ac48-d06160f8c56b")
}

/// Annul costs `{U}` and prints a single line: "Counter target artifact
/// or enchantment spell". Both halves of this filter are played in *the same*
/// first main phase of the opponent — first a Sol Ring spell,
/// then an Exploration —, and because CR 500.5 empties the mana pool only at
/// the end of the step, two tapped Islands pay both `{U}`. The two
/// target lists are the actual proof: an ability without a matching
/// filter could not restrict the stack to exactly one spell, and
/// a countered spell that nevertheless stood on the battlefield would not be
/// a counter.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn annul_counters_an_artifact_spell_and_an_enchantment_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[annul(), annul()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[quiet_artifact(), exploration()])
        .start();
    keep_mulligans(&mut engine);

    // The opponent casts on their own Main: two Forests, two green mana.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "two Forests, two green mana"
    );
    cast_with_floating(&mut engine, p1, quiet_artifact());
    let ring = on_stack(&engine, quiet_artifact()).expect("der Sol Ring liegt auf dem Stapel");

    // p0 responds and taps both Islands at once: the pool outlasts
    // the resolution of the first Annul and also pays for the second.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, two blue mana"
    );

    cast_with_floating(&mut engine, p0, annul());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Annul targets a spell, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat chooses the target");
    assert_eq!(
        options,
        vec![ring],
        "ein Artefaktspruch und sonst nichts auf dem Stapel"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the Sol Ring spell was one of the offered options");

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "a countered spell goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and never onto the battlefield"
    );

    // Die andere Hälfte des gedruckten Filters: eine Verzauberung.
    cast_with_floating(&mut engine, p1, exploration());
    let enchantment = on_stack(&engine, exploration()).expect("Exploration liegt auf dem Stapel");
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_with_floating(&mut engine, p0, annul());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an enchantment is also a target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![enchantment],
        "ein Verzauberungsspruch und sonst nichts auf dem Stapel"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchantment],
            },
        )
        .expect("the Exploration spell was one of the offered options");

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, exploration()).is_some(),
        "auch die Verzauberung wurde gekontert"
    );
    assert!(
        on_battlefield(&engine, p1, exploration()).is_none(),
        "and never reached the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "both Annuls resolved and landed in their graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Islands both paid {{U}}"
    );
}

fn artifact_blast() -> CardIndex {
    card_index("2d4aedc5-31c5-4281-98e1-b0c2233c3c8a")
}

/// Artifact Blast costs {R} and prints a single line: „Counter target
/// artifact spell." The scenario first casts a Sol Ring — an artifact
/// that comes onto the stack as a *spell* and never touches the battlefield —
/// and responds to it while the caster has priority (CR 601.2i);
/// „target artifact spell" does not ask who controls the spell. The
/// counterproof lies in the target list: the Sol Ring that is already on
/// the opponent's side has the same card name and still does not appear
/// on it, because the target is a spell and not a permanent. After
/// resolution the artifact card is in its owner's graveyard instead of on
/// the battlefield, and the Sol Ring already on the battlefield is untapped
/// and untouched — the counter hit the stack and not the battlefield.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn artifact_blast_counters_a_sol_ring_on_the_stack_and_never_touches_the_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[quiet_artifact(), artifact_blast()])
        // The twin on the board: the same card name, but a permanent.
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 erreicht seinen Main");
    let standing =
        on_battlefield(&engine, p1, quiet_artifact()).expect("bei p1 liegt schon ein Sol Ring");

    // Mana before the assertion: `LegalActions` reads the pool and not the
    // two untapped Mountains.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red mana"
    );

    // The spell that is to be countered: {1} paid, and the card lies
    // afterwards on the stack — in none of the three zones in which a test
    // otherwise looks for it.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    let spell = on_stack(&engine, quiet_artifact()).expect("Sol Ring is a spell on the stack");
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "an artifact spell is on the stack until resolution"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the {{1}} is paid, the {{R}} for the counterspell is still floating"
    );

    // CR 601.2i: the caster gets priority back, so they can respond to
    // their own spell.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("after casting, p0 holds priority: {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    cast_with_floating(&mut engine, p0, artifact_blast());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "„target artifact spell\" is a target choice: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat chooses the target");
    assert_eq!((min, max), (1, 1), "exactly one spell");
    assert!(
        options.contains(&spell),
        "the artifact spell on the stack is the target: {options:?}"
    );
    assert!(
        !options.contains(&standing),
        "ein Artefakt auf dem Schlachtfeld ist kein Spruch, auch wenn es \
         derselbe Kartenname ist: {options:?}"
    );
    assert_eq!(options.len(), 1, "und sonst liegt nichts auf dem Stapel");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the target that the question itself offered");
    // CR 601.2h: the costs are the last step of the announcement, so
    // the target question is still there while the mana is floating.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "the {{R}} is paid"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the counterspell is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "and never onto the battlefield"
    );
    assert_eq!(
        on_battlefield(&engine, p1, quiet_artifact()),
        Some(standing),
        "der Sol Ring, der schon lag, ist derselbe wie vorher: keiner ist \
         aufgetaucht, keiner verschwunden"
    );
    assert!(
        !is_tapped(&engine, standing),
        "and he was tapped for nothing — the counterspell never touched the board"
    );
    assert!(
        in_graveyard(&engine, p0, artifact_blast()).is_some(),
        "the counterspell itself is in the graveyard after its resolution"
    );
}

fn battlegrowth() -> CardIndex {
    card_index("650bf82e-7f83-470c-bf4a-34281bfe9341")
}

/// Battlegrowth is `{G}` for one sentence — "Put a +1/+1 counter on target
/// creature" — and the board makes each of its words answerable somewhere
/// different. "Target creature" is any creature, so the menu holds both of my
/// Elves *and* the one across the table while the Forest beside them is no
/// target at all; the named creature is the only one that changes, with the
/// second Elf and the Elf across the table as the two controls that say so;
/// and the counter is read twice — as a projected (2, 2) and as the `+1/+1`
/// counter on the permanent — because a pump and a counter are different
/// claims. The activation order is the card's too: CR 601.2c names the target
/// and CR 601.2h pays the `{G}` afterwards, so the mana is spent by the pool
/// before the question is answered rather than after it.
#[test]
fn battlegrowth_puts_its_counter_on_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[battlegrowth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which stays a printed 1/1"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");

    // One Forest into the pool, and both Elves named as kept back: the {@G}
    // is read off the pool rather than off the untapped lands, and a creature
    // that tapped for it would leave one fewer 1/1 to read the filter against.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest, one green"
    );
    cast_with_floating(&mut engine, p0, battlegrowth());

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
    assert_eq!(player, p0, "the caster is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"target creature\" reaches either of my own: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and any creature across the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a land is no creature, so it is no target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered is the one it is aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, host, baylee_cards_dsl::CounterKind::P1P1),
        1,
        "one +1/+1 counter, on the permanent that was named"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "and the counter is a body: 1/1 plus one of each"
    );
    assert_eq!(
        counters_on(&engine, bystander, baylee_cards_dsl::CounterKind::P1P1),
        0,
        "the Elf nobody named never got one"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the counter lands only where the spell was aimed, never across the table"
    );
}

fn brightstone_ritual() -> CardIndex {
    card_index("08e90e85-4103-4acb-a8a7-e1329b460aa7")
}

/// Brightstone Ritual ({R}, Instant): "Add {R} for each Goblin on the
/// battlefield." The count is the whole card, so the board is built to make
/// three readings disagree at once: two Goblins under the caster, one across
/// the table, and an Elf that is a creature but no Goblin. Three red in the
/// pool is the only answer that reads "Goblin" over the *whole* battlefield —
/// two would drop the opponent's Goblin, four would count the Elf — and the
/// Elf is kept untapped for exactly that, since its own `{T}: Add {G}` would
/// otherwise leave green beside the red this test is counting.
#[test]
fn brightstone_ritual_adds_one_red_for_each_goblin_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                festering_goblin(),
                festering_goblin(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[brightstone_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf is named as the printing kept back: its whole price is its own
    // {T}, so `tap_all_mana` would have tapped it too and the pool would hold
    // a green that has nothing to do with the card under test.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one Mountain pays the {{R}}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "and the Elf stayed out of it"
    );

    cast_with_floating(&mut engine, p0, brightstone_ritual());
    assert!(
        !stack_is_empty(&engine),
        "an instant goes on the stack, so the mana it adds is not here yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} is spent, so whatever the pool holds next the Ritual put there"
    );
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        3,
        "one red for each of the three Goblins on the battlefield — two of \
         this seat's and one of the opponent's"
    );
    assert_eq!(pool.total(), 3, "and nothing else came with it");
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some()
            && on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "the Ritual adds mana and moves no Goblin"
    );
}

fn burst_of_energy() -> CardIndex {
    card_index("b795ecfd-31ac-4a2a-9679-97b088932d93")
}

/// Burst of Energy — {W} instant: "Untap target permanent." The word worth
/// playing is **permanent**, because the target spec is `Filter::Any`: anything
/// on either battlefield, not merely a creature of your own. So the board
/// carries two Plains under p0 — one of them tapped to pay the {W} — and a
/// Forest across the table, and the offer has to name all three. The untap
/// itself is then read off the board rather than off the question: the Plains
/// the spell named stands back up and the Plains nobody named stays down.
/// Both of them are tapped first, which is what makes the second half a
/// claim at all: a land that was never tapped is standing afterwards whether
/// or not the spell did anything.
#[test]
fn burst_of_energy_untaps_the_permanent_it_names_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[forest()])
        .hand(0, &[burst_of_energy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands = all_on_battlefield(&engine, p0, plains());
    assert_eq!(lands.len(), 2, "two Plains were dealt");
    let (spent, kept) = (lands[0], lands[1]);
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");

    // **Both** Plains are tapped, and that is the whole control. A Plains
    // that was never tapped is untapped afterwards whatever the spell did,
    // so "and no other" would have been satisfied by a spell that did
    // nothing at all. Two down, one named, one not.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two white floating, one of which is the {{W}} the spell prints"
    );
    assert!(
        is_tapped(&engine, spent) && is_tapped(&engine, kept),
        "both Plains paid for it and both are down"
    );

    cast_with_floating(&mut engine, p0, burst_of_energy());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
    assert!(
        options.contains(&spent),
        "the tapped Plains is a permanent: {options:?}"
    );
    assert!(
        options.contains(&kept),
        "and so is the untapped one: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target permanent\" reaches across the table: {options:?}"
    );
    assert!(
        is_tapped(&engine, spent),
        "`CR 601.2h`: nothing is untapped while the target question still stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spent],
            },
        )
        .expect("the permanent the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, spent),
        "\"Untap target permanent\" — the permanent the spell named is standing"
    );
    assert!(
        is_tapped(&engine, kept),
        "and the one nobody named is still the tapped Plains it was"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "the Forest across the table was never tapped and still is not"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one {{W}} paid for the spell and the other is still floating — the \
         phase has not ended (CR 500.4)"
    );
    assert!(
        in_graveyard(&engine, p0, burst_of_energy()).is_some(),
        "an instant resolves into its owner's graveyard"
    );
}

fn demystify() -> CardIndex {
    card_index("fd591199-9f7a-4147-a150-13279dbb4498")
}

/// Demystify prints one line — "Destroy target enchantment" — for {W}, and
/// the scenario reads each printed word off a different place. The *filter* is
/// the offer: an opponent's Underworld Breach is on it while the Llanowar
/// Elves beside it are not, so `Filter::ENCHANTMENT` is read rather than
/// skipped, and "target" is why the enchantment only dies because it was
/// named. The destruction is the graveyard the card lands in — its owner's,
/// the seat that controlled it — while the creature the spell did not name
/// never moves, and the {W} off the only Plains on this board is what paid.
#[test]
fn demystify_destroys_the_enchantment_it_names_and_leaves_the_creature_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[demystify()])
        // An enchantment across the table, and a creature beside it that
        // "target enchantment" has to decline.
        .battlefield(1, &[underworld_breach(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doomed = on_battlefield(&engine, p1, underworld_breach()).expect("the enchantment is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("the creature is out");

    cast_from_hand(&mut engine, p0, demystify());
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
    assert_eq!(player, p0, "the caster aims its own spell");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&doomed),
        "the enchantment across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "a creature is no enchantment: {options:?}"
    );
    assert_eq!(options.len(), 1, "and that enchantment is the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the enchantment the question offered is the one it destroys");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, underworld_breach()).is_none(),
        "the named enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, underworld_breach()).is_some(),
        "and it is in its owner's graveyard — the seat that controlled it, \
         not the seat that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, demystify()).is_some(),
        "and the instant itself resolved into its caster's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} the only Plains made is what paid for it"
    );
}

fn enrage() -> CardIndex {
    card_index("0f6e66d5-4f27-485b-999d-ee55c1e218b9")
}

/// Enrage — `{X}{R}` instant: "Target creature gets +X/+0 until end of turn."
///
/// One casting asks the two questions CR 601.2 puts before the payment, and the
/// board is built so each answer is worth reading: X stops at what the pool
/// pays (three Mountains, so 2 — the Elf beside them is named as the source kept
/// back, because a mana creature counted into the pool would move that number),
/// and the target menu holds the Elf *across* the table as well as the one this
/// side, which is "target creature" and not "target creature you control",
/// while the Sol Ring over there is no creature at all. `(3, 1)` on a printed
/// 1/1 is the only body that reads both halves of the pump, and the same Elf
/// reading `(1, 1)` again on the opponent's turn is what makes "until end of
/// turn" a duration rather than a counter.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn enrage_pumps_the_target_by_x_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), quiet_artifact()])
        .hand(0, &[enrage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // The Elf is named as the source kept back: its whole price is its own
    // `{T}`, so `tap_all_mana` would have drunk it too (#159) and the three
    // red X is measured against would have been four.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains tapped and the Elf still standing"
    );

    cast_with_floating(&mut engine, p0, enrage());

    // CR 601.2b then CR 601.2c, before a single mana is spent (CR 601.2h). The
    // two questions are answered in whichever order they arrive rather than in
    // the order they are expected: a spell that asked only one of them would
    // otherwise look the same as one that asked the other first.
    let mut named_x = false;
    let mut aimed = false;
    for _ in 0..20 {
        if named_x && aimed && at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber { player, min, max } => {
                assert_eq!(player, p0, "the caster names X");
                // **Not** bounded by the pool: `cast_wizard` offers up to
                // `X_CEILING` and validates the mana when the wizard
                // finishes, because a printed `{X}` has no legality of its
                // own (CR 601.2b) — the payment is where an unpayable
                // announcement is refused. So the floor is what this
                // assertion is about, and the three the Mountains pay is
                // asserted below, where it is actually spent.
                assert_eq!(min, 0, "nothing is a legal X");
                assert!(max >= 2, "the two the Mountains pay is offered: {max}");
                engine
                    .apply(p0, PlayerAction::ChooseNumber(2))
                    .expect("the value the question itself offered");
                named_x = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the caster aims it");
                assert_eq!((min, max), (1, 1), "exactly one creature");
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "\"target creature\" is any creature, on either side of the \
                     table: {options:?}"
                );
                assert!(
                    !options.contains(&rock),
                    "the Sol Ring is an artifact and no creature: {options:?}"
                );
                assert_eq!(options.len(), 2, "and those two are the whole menu");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .expect("the Elf the question offered was chosen");
                aimed = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while Enrage is being cast: {other:?}"),
        }
    }
    assert!(
        named_x && aimed,
        "the cast asked for both its value and its target"
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{R}} with X two is the three Mountains, and they are spent"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 1),
        "+X/+0: the two the caster named, and the toughness the card never pumps"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );

    // The duration. The opponent's main phase lies past this turn's cleanup
    // step, which is where "until end of turn" ends — an Elf still reading
    // `(3, 1)` there would be a counter wearing a one-shot's name.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"until end of turn\": the +2 is gone with the turn that paid for it"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the creature nobody aimed at was never anything but a 1/1"
    );
}

fn envelop() -> CardIndex {
    card_index("30062dd0-c049-4872-8011-8b4810a3fa26")
}

/// Envelop costs {U} and, according to its own text, counters a
/// sorcery spell. A counter can only be read from what *does not happen*
/// afterwards: Wheel of Fortune would have made each player discard their
/// hand and draw seven cards — so p0's unchanged library and hand are one
/// half of the claim and p1's graveyard the other. Dark Ritual lies as
/// an instant over the same sorcery on the same stack and must be absent
/// from the target menu; otherwise `Filter::HasType(SORCERY)` reads
/// nothing at all and the spell counters every spell. The {U} is paid only
/// after the target choice (CR 601.2c before CR 601.2h), which the pool
/// shows after the response.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn envelop_counters_the_sorcery_and_declines_the_instant_beside_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[envelop()])
        .battlefield(1, &[mountain(), mountain(), mountain(), swamp()])
        .hand(1, &[wheel_of_fortune(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // Three Mountains pay {2}{R}; the Swamp stays untapped so that the
    // instant above it does not fail because of the color of the rest.
    let swamp_obj = on_battlefield(&engine, p1, swamp()).expect("the Swamp is untapped");
    tap_mana_where(&mut engine, p1, |id| id != swamp_obj);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "three Mountains, three mana for the sorcery"
    );
    cast_with_floating(&mut engine, p1, wheel_of_fortune());
    assert!(
        on_stack(&engine, wheel_of_fortune()).is_some(),
        "the sorcery waits on the stack"
    );

    // The active player retains priority after casting (CR 117.3c) and
    // casts another instant in response to the same sorcery.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_mana_where(&mut engine, p1, |id| id == swamp_obj);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the Swamp, one black mana"
    );
    cast_with_floating(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, |e| {
        on_stack(e, dark_ritual()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let wheel_spell = on_stack(&engine, wheel_of_fortune()).expect("the sorcery is still there");
    let ritual_spell = on_stack(&engine, dark_ritual()).expect("der Instant liegt darüber");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // First mana into the pool, then the assertion: `castable` reads the pool
    // and not the untapped Islands.
    tap_mana_where(&mut engine, p0, |_| true);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "two Islands, two blue mana"
    );
    let counterspell = in_hand(&engine, p0, envelop()).expect("Envelop is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("p0 holds priority, not {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&counterspell),
        "with {{U}} in the pool the counterspell is playable: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: counterspell })
        .expect("der Konter ist angekündigt");
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target sorcery spell\" is a target choice, not {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the casting player chooses the target (CR 601.2c)"
    );
    assert_eq!(
        options,
        vec![wheel_spell],
        "the sorcery and only it: the instant above is a spell, not a sorcery"
    );
    assert!(
        !options.contains(&ritual_spell),
        "HasType(SORCERY) is read, not skipped"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wheel_spell],
            },
        )
        .expect("the sorcery was one of the options");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the cost comes only after the target (CR 601.2h): the {{U}} is paid"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, envelop()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, wheel_of_fortune()).is_some(),
        "the countered spell is in p1's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "der Instant unter dem Konter ist unberührt verrechnet — er zeigt auf niemanden"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the Wheel would have drawn seven cards each: none of that happened"
    );
    // One card fewer, and the one it lost is the counter it cast: the Wheel
    // would have taken the whole hand away and dealt seven back (CR 701.5a
    // is why none of it happened), so "the same size" would have been the
    // wrong claim as well as a false one.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the hand lost the Envelop and nothing else — no discard, no draw"
    );
}

fn erase() -> CardIndex {
    card_index("c2ceb15c-5d02-4cf4-a7c9-c1a40b7ca667")
}

/// Erase costs {W} and prints a line: "Exile target enchantment."
/// Exactly one enchantment is on the table — and exactly it is on the
/// target list, while the Sol Ring next to it and the Elf across as
/// permanents are still not targets, which separates
/// `Filter::ENCHANTMENT` from `Filter::Any`. The second word that matters
/// is "exile": the card must end up in its owner's exile and must *not*
/// be in the graveyard, where a destruction effect would have left it.
/// Payment is made from mana that was in the pool beforehand — the engine
/// reads payability there and not at untapped lands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn erase_exiles_the_enchantment_it_names_and_leaves_the_rest_of_the_board() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), quiet_artifact()])
        .battlefield(1, &[their_enchantment(), llanowar_elves()])
        .hand(0, &[erase()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let victim = on_battlefield(&engine, p1, their_enchantment()).expect("the enchantment is out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let exiled_before = engine.state().zones.list(ZoneLocation::Exile(p1)).len();

    // First mana into the pool: whether a spell is castable, the engine reads
    // from the pool and not from what could still be tapped (#159).
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, erase()).expect("Erase is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{W}} is in the pool and an enchantment is on the battlefield, so \
         the spell is playable: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the {{W}} already in the pool pays for it");

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the applying seat chooses the target");
    assert_eq!((min, max), (1, 1), "exactly one enchantment");
    assert_eq!(
        options,
        vec![victim],
        "\"target enchantment\" offers the enchantment and nothing else: the \
         Sol Ring next to it and the Elf opposite are permanents and yet not \
         targets"
    );
    assert!(
        !options.contains(&rock) && !options.contains(&elf),
        "the two Bystanders are not enchantments: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "an enchantment is not a player: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("die angebotene Verzauberung ist die Antwort");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the named enchantment has left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_none(),
        "\"Exile\" and not \"destroy\": a destruction effect would have put \
         it into its owner's graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&victim),
        "and it is in exile — under its owner, not under the caster"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        exiled_before + 1,
        "one target, one exiled card"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some()
            && on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "die Beisitzer stehen unberührt"
    );
    assert!(
        in_graveyard(&engine, p0, erase()).is_some(),
        "the spell itself is in its caster's graveyard after resolution"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "the {{W}} from the pool is paid"
    );
}

fn heat_ray() -> CardIndex {
    card_index("76ec76b9-da0f-4b9d-ad0a-d734052a5f2b")
}

/// Heat Ray — {X}{R} Instant: "Heat Ray deals X damage to target creature."
///
/// X is the whole card, so the scenario answers a number no fixed amount could
/// have produced: five, which is exactly lethal to the printed 7/5 across the
/// table. The number is read out of the question the engine asks at announce
/// time (CR 601.2b) and the target off the list published for CR 601.2c — a
/// creature and no player, which is what "target creature" is worth telling
/// from "any target". The six Mountains the cast drains are what says the
/// {{5}}{{R}} was paid rather than assumed.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn heat_ray_deals_the_x_its_controller_names_to_the_creature_it_names() {
    // oracle_id = "d3a5a830-cd14-49da-9412-c50049c74c92"
    fn fleshgorger() -> CardIndex {
        card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
    }

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        // A 7/5 for five damage to be exactly lethal to, and an artifact that
        // "target creature" has to decline.
        .battlefield(1, &[fleshgorger(), quiet_artifact()])
        .hand(0, &[heat_ray()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let target = on_battlefield(&engine, p1, fleshgorger()).expect("the 7/5 is out");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(
        pt(&engine, target),
        (7, 5),
        "five damage is exactly lethal to the body the spell is aimed at"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Mountains, and six is exactly {{5}}{{R}}"
    );
    cast_with_floating(&mut engine, p0, heat_ray());

    // X is chosen at announce time (CR 601.2b) and the target after it
    // (CR 601.2c); each answer is lifted out of the enumeration its own
    // question carried rather than assumed.
    let mut asked_x = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut a_player_was_on_the_menu = true;
    for _ in 0..8 {
        if asked_x && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber { player, min, max } => {
                assert!(
                    min <= 5 && 5 <= max,
                    "X = 5 must be one of the values six Mountains can pay: {min}..={max}"
                );
                engine
                    .apply(player, PlayerAction::ChooseNumber(5))
                    .expect("the answer came out of the question");
                asked_x = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                menu = options;
                a_player_was_on_the_menu = !player_options.is_empty();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("the 7/5 was one of the options");
            }
            other => panic!("unexpected while casting Heat Ray: {other:?}"),
        }
    }
    assert!(
        asked_x,
        "{{X}} is a question the engine asks and not a number the card fixes"
    );
    assert!(
        menu.contains(&target),
        "\"target creature\" offers the creature across the table: {menu:?}"
    );
    assert!(
        !menu.contains(&ring),
        "an artifact is no creature: {menu:?}"
    );
    assert!(
        !a_player_was_on_the_menu,
        "\"target creature\" is not \"any target\" (CR 115.4): both seats \
         would otherwise be on the menu"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{5}}{{R}} is the last step of the activation (CR 601.2h), and it is \
         paid rather than labelled"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, fleshgorger()).is_some(),
        "five damage to a 7/5 is lethal (CR 704.5f) — a fixed three would have \
         left it standing, which is what makes this an X and not a number"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the spell did not name never moved"
    );
}

fn howl_from_beyond() -> CardIndex {
    card_index("403cf6ae-48a9-4ea9-894e-7135cfca4e1b")
}

/// Howl from Beyond prints one line — "Target creature gets +X/+0 until end
/// of turn" — and X is the whole of the card, so the scenario names 2 out of
/// a pool four Swamps actually filled: the printed +2/+0 on a 1/1 has to read
/// `(3, 1)`, which neither a fixed pump nor a toughness half could produce.
/// The Elf across the table is the control that keeps the pump on the target
/// that was named, and the Elf beside the Swamps is kept untapped so the pool
/// afterwards is "three black spent and nothing else" rather than a count a
/// mana creature quietly paid into.
#[test]
fn howl_from_beyond_pumps_the_target_it_names_for_the_x_it_was_given() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(811, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[howl_from_beyond()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // Four Swamps into the pool, and the Elf named as the printing kept back:
    // it is the creature the spell is aimed at, and an Elf tapped for its own
    // mana would make every number below a claim about five.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps, and nothing off the Elf beside them"
    );
    cast_with_floating(&mut engine, p0, howl_from_beyond());

    // CR 601.2b asks for X and CR 601.2c for the target, and the two arrive in
    // whichever order the engine is written to ask them: both are answered out
    // of what the question enumerated rather than in an assumed order.
    let mut aimed = false;
    let mut sized = false;
    for _ in 0..12 {
        if aimed && sized {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the casting seat aims it");
                assert_eq!((min, max), (1, 1), "\"target creature\": exactly one");
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "\"target creature\" is any creature, on either side of the \
                     table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            Pending::ChooseNumber { player, min, max } => {
                assert_eq!(player, p0, "the casting seat names X");
                assert!(min <= 2 && 2 <= max, "X = 2 is not in {min}..={max}");
                engine.apply(player, PlayerAction::ChooseNumber(2)).unwrap();
                sized = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Howl is cast: {other:?}"),
        }
    }
    assert!(
        aimed && sized,
        "casting the Howl asks for both X and a target"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, mine),
        (3, 1),
        "+2/+0 for the X that was named — a (1, 3) would be a toughness pump \
         the card does not print"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump lands on the creature that was named and never across the table"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{X}}{{B}} with X = 2 is three black out of the four the Swamps made"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else is in the pool, so the Elf beside them never paid"
    );

    // "Until end of turn": the only reading that tells a duration from a
    // permanent +2/+0 is the same creature on the far side of the turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the pump ended with the turn it was cast in"
    );
}

fn iron_will() -> CardIndex {
    card_index("dc0decb9-32da-47aa-a51a-0d9c623c534a")
}

/// Iron Will is `{W}` for "Target creature gets +0/+4 until end of turn" and
/// cycles for `{2}`. One creature stands under each seat so the pump is read
/// as a target and not as a board buff — `(1, 5)` against `(1, 1)` — and the
/// second copy is cycled out of the same floating mana, which is the only way
/// to see that the other printed line discards *this* card and draws one: the
/// card ends in its owner's graveyard and the library is a card shorter.
#[test]
fn iron_will_pumps_one_creature_and_cycles_its_other_copy_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[iron_will(), iron_will()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one across the table"
    );

    // The Elves are named as the printing kept back: `tap_all_mana` presses a
    // creature's own `{T}: Add {G}` too (#159), and the green it would float
    // has nothing to do with the `{W}` this spell costs.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        4,
        "four Plains, four white, and neither Elf tapped for it"
    );

    cast_with_floating(&mut engine, p0, iron_will());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    // CR 601.2c before CR 601.2h: the target is named while the {W} is still
    // in the pool, so the payment below really is the last step of the cast.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        4,
        "targets are chosen before costs are paid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is a legal target");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        3,
        "the {{W}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (1, 5),
        "+0/+4 on the creature the spell named — a (1, 1) would mean the pump \
         never resolved, and a (5, 5) that the power was read too"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );

    // The second printed line: Cycling {2} ({2}, Discard this card: Draw a
    // card). The card is a hand object, so the ability is offered on the card
    // itself and on nothing standing on the battlefield.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, iron_will(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, iron_will()).is_some(),
        "\"Discard this card\": the card goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discarded card left the hand and a drawn one replaced it"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "and the {{2}} came out of the same pool the {{W}} did"
    );
}

fn jump() -> CardIndex {
    card_index("f7518456-45ed-41d4-bd3c-5aacea28eb35")
}

/// Jump costs {U} and prints a line: "Target creature gains flying until
/// end of turn." The test plays both halves. The offer is the one proof
/// you cannot read from the card: the card says "target creature" and not
/// "target creature you control", so two of your own Elves *and* the one
/// on the battlefield are in the same selection, and only the named one
/// gets flying. The turn change checks the duration: the same Elf is
/// grounded again in the opponent's main phase.
#[test]
fn jump_grants_flying_to_the_creature_it_targets_and_only_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[jump()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays grounded");
    let (target, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FLYING),
        "nothing has been granted anything yet"
    );

    // Mana before the assertion: the Island is tapped before the spell is
    // considered playable. The Elves are exempt because they are the
    // creatures this is about in a moment.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one Island, one blue, and neither Elf of mine paid in"
    );
    cast_with_floating(&mut engine, p0, jump());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&target) && options.contains(&bystander),
        "both creatures under my control are creatures: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"target creature you control\": {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the creature the question offered is the one that gains flying");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, target).contains(KeywordSet::FLYING),
        "the Elf the instant named is flying"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the Elf nobody named is still grounded"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the Elf across the table was a legal target and was not the target"
    );

    // "until end of turn": the next turn reads the same Elf grounded.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FLYING),
        "the grant lasts until end of turn and not a step longer"
    );
}

fn leap() -> CardIndex {
    card_index("e89a3ce0-6b38-4326-a7af-8575af371baa")
}

/// Leap is `{U}` for two printed lines — "Target creature gains flying until
/// end of turn" and "Draw a card" — and both are only themselves together: a
/// card that granted flying without drawing is a different card, and one that
/// drew without granting reads the same on a library count. The board carries
/// two Elves under one seat and a third across the table, so the keyword has
/// to land on the creature the target question *named* and on no other —
/// "target creature" is neither "creatures you control" nor the whole table —
/// and the draw is read as a library one card shorter, which a reveal or a
/// scry could not produce.
#[test]
fn leap_grants_flying_to_the_creature_it_names_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[leap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays grounded");
    let (chosen, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, chosen).contains(KeywordSet::FLYING),
        "nothing has been cast yet"
    );

    // The Island is the whole price, and the Elves are named as the printing
    // kept back: a creature tapped for mana is a creature whose status has
    // already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Island and neither Elf: the {{U}} is a real payment"
    );

    let library_before = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, leap());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell aims it");
    assert!(
        options.contains(&chosen) && options.contains(&bystander),
        "both creatures you control may be the target: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("the creature the question offered was chosen");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, chosen).contains(KeywordSet::FLYING),
        "\"target creature gains flying until end of turn\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the spell reaches the creature it named and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor across the table, where nobody was named"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card left the top of the library"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}} came out of the pool"
    );
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

/// Lightning Bolt prints a line: "{R} — Instant: Lightning Bolt deals 3
/// damage to any target." Exactly that is played twice here, because
/// `any target` (CR 115.4) means both lists of *one* target question:
/// the first Bolt kills the printed 1/1 Elf on the battlefield, the
/// second goes to the player. The 20 life after the creature hit are the
/// control — they rule out that the damage went to both targets at once
/// —, and the two Mountains are the entire cost of both spells in one
/// main phase (CR 500.5), so that "two red" before the first `apply`
/// is a statement about the pool and not about untapped lands.
#[test]
fn lightning_bolt_deals_three_to_a_creature_or_a_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 erreicht seine eigene Main"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf on the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "ein gedruckter 1/1 für drei Schaden"
    );

    // Both Mountains, both red mana: `legal.castable` reads the pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, two red, and nothing else on the board"
    );

    cast_with_floating(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("der Blitz zielt, got {:?}", engine.pending())
    };
    assert_eq!(
        (player, min, max),
        (p0, 1, 1),
        "one target, and the caster chooses it"
    );
    assert!(
        options.contains(&elf),
        "the creature above the table is a target: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: `any target` counts players in the same choice: {player_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage to a 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the player got no point from it"
    );

    // The second Bolt at the other target: the same line, and the damage
    // lands where the response carries it.
    cast_with_floating(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("auch der zweite Blitz zielt, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a target for `any target`: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "exactly three life, and no more"
    );
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "the Blitz landed in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both {{R}} are paid, so the cost of the second Bolt was real"
    );
}

fn lose_hope() -> CardIndex {
    card_index("4f6dbd90-f7fe-4adf-a00f-27573c0bd5c4")
}

/// Lose Hope is a black instant with two sentences — "Target
/// creature gets -1/-1 until end of turn" and "Scry 2" — and both are
/// played. The target is the Wurm on the table, and *both* of its
/// numbers are read: the difference `(power - 1, toughness - 1)` is the
/// only pair that evidences a -1/-1, while a -0/-1 would leave the power
/// unchanged; that the Wurm survives is also the reason why the numbers
/// are readable at all — a 1/1 would have died according to CR 704.5f
/// before anyone could have checked its power. The Elf next to it is the
/// other half of "Target creature": exactly one target is available, and
/// the creature that the spell did not name does not move. The
/// Scry half is read as a question and not as an answer — the two
/// topmost cards are up for choice, topmost first, and sending none of
/// them to the bottom leaves the library exactly as it was.
#[test]
fn lose_hope_shrinks_the_creature_it_names_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[lose_hope()])
        .battlefield(1, &[rootbreaker_wurm(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("their Wurm is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf beside it");
    let (power, toughness) = pt(&engine, wurm);
    let untouched = pt(&engine, bystander);

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    cast_from_hand(&mut engine, p0, lose_hope());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&wurm) && options.contains(&bystander),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and the two creatures over there are the whole menu: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options");

    // Die Auflösung führt durch den ersten Satz hindurch zur Frage des
    // zweiten: erst schrumpfen, dann schauen.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(
        prompt,
        ArrangePrompt::Scry,
        "scry is its own question and not a search or a discard"
    );
    assert_eq!(
        piles,
        scry_piles(2),
        "Scry 2: either, both or neither of the top two, and none of them forced"
    );
    assert_eq!(cards, vec![top, second], "the top two cards, top first");

    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("looking is not moving: keeping both on top is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        pt(&engine, wurm),
        (power - 1, toughness - 1),
        "-1/-1 on the creature the spell named — a (power, toughness - 1) \
         would mean only the toughness was ever read"
    );
    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_some(),
        "and it survives: the spell shrinks a creature rather than destroying one"
    );
    assert_eq!(
        pt(&engine, bystander),
        untouched,
        "the creature the spell did not name never moved"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).clone(),
        library_before,
        "scry 2 looks and reorders: nothing was drawn and nothing was bottomed"
    );
}

fn mental_note() -> CardIndex {
    card_index("e8d5f31c-7abf-4fbb-977e-8353a97daf7a")
}

/// Mental Note ({U}, Instant) prints exactly two lines: "Mill two cards" and
/// "Draw a card." Both move the same library into two different zones, so
/// no single length counter can tell them apart — a card that only milled
/// or only drew would satisfy every single delta. Therefore each zone is
/// read and the objects are named: the top two cards from before the spell
/// are now in the graveyard, and the card in hand is exactly the third from
/// the top, which nails down the printed order (first mill, then draw).
#[test]
fn mental_note_mills_the_top_two_and_then_draws_the_next_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[mental_note()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The library is `forest()` all the way down — sixty copies of one
    // printing — so the *objects* are the only thing that can say which card
    // went where.
    let library_before: Vec<ObjectId> =
        engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];
    let third = library_before[library_before.len() - 3];
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, mental_note());
    pass_until(&mut engine, stack_is_empty);

    let graveyard = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert!(
        graveyard.contains(&top) && graveyard.contains(&second),
        "\"mill two cards\": the top two of the library, and neither of them \
         is anywhere else: {graveyard:?}"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 3,
        "two milled and one drawn is three cards off the top"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&third),
        "\"draw a card\": the mill moved the first two, so the card that \
         reaches the hand is exactly the third from the top"
    );
    // The hand is the size it was, and that is arithmetic rather than a
    // disappointment: the Note left the hand as the drawn card entered it.
    // The claim that a card was drawn is the identity above — a mill alone
    // would have left the hand one card *shorter*, which is what this
    // number rules out.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card out (the Note) and one card in (the draw)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the spell's {{U}} came out of the pool the Island filled"
    );
    assert!(
        in_graveyard(&engine, p0, mental_note()).is_some(),
        "and the instant itself is in its owner's graveyard once it has \
         resolved"
    );
}

fn opt() -> CardIndex {
    card_index("713332c1-5bd8-400f-bfff-c1ca0697a043")
}

/// Opt — {U} Instant: "Scry 1. Draw a card."
///
/// The two halves can only be read together: Scry 1 moves exactly
/// one card and leaves the library the same length, the draw takes
/// exactly the then-topmost and makes it one shorter. Therefore the
/// topmost and the second-topmost cards are pinned before the effect
/// via their `ObjectId`: the first must land on the bottom, the second
/// in hand. Via card indices that would not be visible, because the
/// filler is an Island and every copy is the same.
#[test]
fn opt_scries_the_top_card_to_the_bottom_and_draws_the_one_beneath_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[opt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert!(
        library_before.len() >= 2,
        "Scry 1 needs a second card to separate the topmost from the next"
    );
    let top = *library_before.last().expect("the library is not empty");
    let second = library_before[library_before.len() - 2];
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, opt());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("die Bedingung hat gerade gematcht")
    };
    assert_eq!(player, p0, "der Wirkende schaut");
    assert_eq!(
        prompt,
        crate::choice::ArrangePrompt::Scry,
        "ein Scry, kein Surgeil"
    );
    assert_eq!(
        cards,
        vec![top],
        "Scry 1 sees exactly one card, and it is the topmost"
    );
    assert_eq!(piles, scry_piles(1), "oben lassen oder nach unten legen");

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("the offered card is a legal response, the costs are long since paid");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .first()
            .copied(),
        Some(top),
        "the scried card is on the bottom — moved, not removed"
    );
    assert_eq!(
        engine.state().object(top).expect("das Objekt lebt").zone,
        Zone::Library,
        "and it is still in the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "Scry moves, Draw takes: exactly one card fewer than before"
    );
    assert_eq!(
        engine.state().object(second).expect("das Objekt lebt").zone,
        Zone::Hand,
        "the card drawn was the one under the scried card, not another"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the instant left the hand and the drawn card replaced it"
    );
    assert!(
        in_graveyard(&engine, p0, opt()).is_some(),
        "a resolved instant is in its owner's graveyard"
    );
}

// oracle_id = "0ff78353-a26e-4b5a-948d-1c3b41d2dd1f"
fn oxidize() -> CardIndex {
    card_index("0ff78353-a26e-4b5a-948d-1c3b41d2dd1f")
}

/// Oxidize — {G} instant: "Destroy target artifact. It can't be regenerated."
///
/// The filter is the whole card, and it needs a witness on each side: an
/// artifact under the caster's own control and one across the table are both
/// on the offer, because the printing names no controller — while the Forest
/// beside them is a permanent and no artifact, which is what says
/// `Filter::ARTIFACT` was read rather than skipped. Only the artifact the
/// question was answered with leaves, and it goes to its *owner's* graveyard,
/// so a resolve that had swept the type would fail the survivor below. The
/// second printed sentence has nothing to act on: no card in the pool
/// regenerates anything.
#[test]
fn oxidize_destroys_the_artifact_it_names_and_leaves_its_neighbour_standing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), quiet_artifact()])
        .battlefield(1, &[quiet_artifact(), forest()])
        .hand(0, &[oxidize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my artifact is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert!(
        types(&engine, mine).contains(TypeSet::ARTIFACT),
        "the permanent the filter has to read is the artifact it prints"
    );

    // `{G}` comes off the two Forests; the offer at the target question is
    // what the card is, so nothing is asserted before the mana is floating.
    cast_from_hand(&mut engine, p0, oxidize());
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
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "\"target artifact\" is one artifact");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target artifact\" names no controller, so both sides of the table are \
         on the menu: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a land is a permanent and no artifact: {options:?}"
    );
    assert_eq!(options.len(), 2, "those two artifacts are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the artifact the question offered is the one that is named");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the destroyed artifact is in its *owner's* graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and no longer on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "nor did anything that is not an artifact"
    );
    assert!(
        in_graveyard(&engine, p0, oxidize()).is_some(),
        "the instant itself resolved and is in its caster's graveyard"
    );
}

fn quiet_purity() -> CardIndex {
    card_index("3f16dc14-3d3f-4ffa-90bc-9abc67db75cf")
}

/// Quiet Purity — {W} Instant (Arcane): "Destroy target enchantment."
///
/// One enchantment and one creature stand on each side of the table, so the
/// target offer is the whole card in a single question: it holds the
/// enchantment across the table and neither creature, which tells "target
/// enchantment" from "target permanent" and from "target creature". The {W}
/// is read off the pool rather than off the untapped Plains, and the
/// enchantment is followed into its *owner's* graveyard rather than merely
/// off the battlefield — an exile or a bounce would satisfy the first
/// reading on its own.
#[test]
fn quiet_purity_destroys_the_enchantment_across_the_table_and_no_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .battlefield(1, &[their_enchantment(), llanowar_elves()])
        .hand(0, &[quiet_purity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doom = on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment stands");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves stand");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");

    // The offer is read off the pool and not off the untapped lands, so the
    // mana goes in before anything is claimed about it.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, quiet_purity()).expect("the spell is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{W}} floating pays for it: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, quiet_purity());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!(
        options,
        vec![doom],
        "the one enchantment on the table: a creature is no enchantment and \
         a land is none either"
    );
    assert!(
        !options.contains(&my_elf) && !options.contains(&their_elf),
        "the two creatures are the control: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "an enchantment is an object target, so no seat is on the menu: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doom],
            },
        )
        .expect("the enchantment the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and a destroyed permanent goes to its owner's graveyard — an exile \
         or a bounce would leave this empty"
    );
    assert!(
        in_graveyard(&engine, p0, their_enchantment()).is_none(),
        "under the seat that owns it and not under the one that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature beside it never moved"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nor the one on this side, which \"destroy target enchantment\" could \
         not have named at all"
    );
}

fn reach_through_mists() -> CardIndex {
    card_index("c81ca8ff-92f3-481e-82f7-0673b6c74ea0")
}

/// Reach Through Mists costs `{U}`, is an instant and prints exactly
/// one line: "Draw a card." It is therefore played where a sorcery
/// could no longer be played — in the end step of its own turn, in which
/// only the active player gets priority (CR 117.3a) —, and the one card
/// it draws is the previously top card of the library, which afterwards
/// lies in hand as the same object. The spell itself goes to the graveyard
/// after resolution, the library is shorter by exactly one, and the `{U}` has
/// disappeared from the pool, because the assertion about `castable` came
/// only after the tapping.
#[test]
fn reach_through_mists_draws_the_top_card_at_instant_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, island())
        .battlefield(0, &[island()])
        .hand(0, &[reach_through_mists()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 erreicht seine eigene erste Hauptphase"
    );

    // Into the end step of its own turn: a sorcery would no longer be
    // playable here, an instant would.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let spell = in_hand(&engine, p0, reach_through_mists()).expect("the instant is in hand");

    // First the mana into the pool, then the assertion: `LegalActions` is
    // filtered behind `can_afford` and reads the pool, not the untapped
    // lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one Island, one blue mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{U}} is in the pool, so the spell is playable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, reach_through_mists());
    assert!(
        on_stack(&engine, reach_through_mists()).is_some(),
        "the spell is on the stack before anyone passes"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}} is paid"
    );
    assert!(
        in_graveyard(&engine, p0, reach_through_mists()).is_some(),
        "a resolved instant goes to the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "exactly one card has left the library"
    );
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert!(
        hand.contains(&top),
        "\"Draw a card\": the previously top card of the library is now in \
         hand"
    );
    assert!(
        !hand.contains(&spell),
        "and the spell itself has gone — if it remained in hand, the hand \
         size would be unchanged and nothing would have left it"
    );
}

fn rescue() -> CardIndex {
    card_index("f0ae687a-7222-4521-a514-ba10373fed7e")
}

/// Rescue — {U} Instant: "Return target permanent you control to its owner's
/// hand." The load-bearing word is *permanent*: the offer has to hold the
/// Island this seat controls — a creature-only filter would have dropped it —
/// and it has to decline the Elf across the table, which only
/// `Filter::ControlledByYou` keeps off the menu. The bounce is then read as the
/// Elf arriving in its owner's hand while the land beside it never moves, and
/// the {{U}} as the one blue the single Island put in the pool.
#[test]
fn rescue_returns_a_permanent_you_control_to_the_hand_of_the_seat_that_owns_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), llanowar_elves()])
        .hand(0, &[rescue()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let land = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // Only the Island is tapped: `tap_all_mana` would also have pressed the
    // Elves' own printed `{T}: Add {G}` (#159), and one blue is the whole of
    // the price the card prints.
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Island, one blue — the Elves stayed untapped and made nothing"
    );

    cast_with_floating(&mut engine, p0, rescue());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent you control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&mine),
        "the creature under my own control is on the menu: {options:?}"
    );
    assert!(
        options.contains(&land),
        "\"target permanent\" is not \"target creature\": the Island I control \
         is a permanent too, and a creature-only filter would have dropped it: \
         {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" declines the Elf across the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "\"return ... to its owner's hand\": the Elf is in the hand of the seat \
         that owns it"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and it has left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, island()).is_some(),
        "the permanent Rescue did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor did the creature across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}} it charges came out of the pool"
    );
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

/// Shock prints one line — "Shock deals 2 damage to any target" — and the test
/// plays both halves of "any" off one card. The first cast is aimed at the
/// opposing player from a `ChooseTargets` whose two lists are both populated
/// (CR 115.4), and 20 life becoming 18 is what pins the number: a one-damage
/// Shock leaves 19 and a three-damage one leaves 17. The second is aimed at a
/// printed 1/2, which dies to exactly the same two and would have survived one,
/// so the object half of the spec is read with the amount already fixed.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn shock_deals_two_to_a_player_and_to_a_printed_one_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[shock(), shock()])
        .battlefield(1, &[a_one_two_bird()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bird = on_battlefield(&engine, p1, a_one_two_bird()).expect("the Bird is out");
    assert_eq!(
        pt(&engine, bird),
        (1, 2),
        "a printed 1/2: two damage is exactly lethal where one would not be"
    );

    // Mana before the claim: the offer is read off the pool, and the two
    // Mountains are exactly the {R}{R} two Shocks cost in this one main phase
    // (CR 500.5, which ends at the step and not before).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red"
    );

    cast_with_floating(&mut engine, p0, shock());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "`any target` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&bird),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice, both of \
         them: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player the prompt enumerated is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"deals 2 damage\": 19 would be one and 17 would be three"
    );
    assert!(
        on_battlefield(&engine, p1, a_one_two_bird()).is_some(),
        "the damage went to the player who was named and not to their board"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one of the two red paid the first Shock"
    );

    // The object half of the same spec, off the red still floating.
    cast_with_floating(&mut engine, p0, shock());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the second Shock asks for a target too, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bird),
        "the same 1/2 is offered again: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bird],
                players: vec![],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, a_one_two_bird()).is_some(),
        "two damage to a printed 1/2 is lethal (CR 704.5f)"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "and the second Shock went to the creature: the life total never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} it charged came out of the pool"
    );
}

fn shrink() -> CardIndex {
    card_index("252330b1-67cf-4d9e-a413-917ba61e731f")
}

/// Shrink is `{G}` for a single printed sentence: "Target creature gets
/// -5/-0 until end of turn." On the battlefield stands a printed 6/6 Wurm, so
/// that the −5 lands on a number that remains positive: `(1, 6)` reads both
/// halves of the calculation — a `(1, 1)` would mean that a toughness
/// penalty was paid along with it, which the card does not print. The Elves
/// next to it and the Elves across the table are the two controls: one
/// shows that the static only affects the *named* creature, the other that
/// "target creature" was not narrowed to "your creature". The second main
/// phase visit reads the duration, which is half the card: "until end of
/// turn" is over as soon as the turn in which it was cast is over.
#[test]
fn shrink_takes_five_power_from_the_creature_it_names_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), rootbreaker_wurm(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[shrink()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the spell");

    // The Forest pays the {G}; the Elves are named as the printing that
    // remains, because they are the creature that the control below reads.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest tapped, one green floating"
    );
    cast_with_floating(&mut engine, p0, shrink());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&wurm) && options.contains(&elves) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (1, 6),
        "−5/−0 on the creature it named — a (1, 1) would be a toughness \
         penalty the card never prints"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and nothing at all for the creature it did not name"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor for the creature across the table, which it merely could have named"
    );

    // The other half of the sentence. The next main phase lies behind
    // the end of p0's turn, and exactly there "until end of turn" expires.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, wurm),
        (6, 6),
        "the pump lasts until end of turn, and this is no longer that turn"
    );
}

fn silk_net() -> CardIndex {
    card_index("58511cb8-348d-409e-8fd6-772be20e57cd")
}

/// Silk Net is one sentence — "Target creature gets +1/+1 and gains reach
/// until end of turn" — and both halves of it land on the creature the offer
/// named and on no other. That is why the board holds two Elves under the
/// caster and one across the table: the target reads `(2, 2)` with `REACH`
/// out of the layer projection, which is the only reading that can see a
/// *granted* keyword, while the two Elves nobody aimed at stay printed
/// `(1, 1)`s without it. The offer is the other half of "target creature" —
/// the Elf across the table is on it, the Forest beside it is not, and the
/// `{G}` is read as spent at the end.
#[test]
fn silk_net_pumps_and_grants_reach_to_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[silk_net()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which the Net never names"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Net");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::REACH),
        "reach is granted and not printed, so it is not there yet"
    );

    // The Forest pays and the Elves are kept standing: their own `{T}: Add
    // {G}` is a mana route `tap_all_mana` would take, and a host tapped for
    // mana reads like a host the Net never reached.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest tapped and neither Elf: the pool is the {{G}} and nothing else"
    );
    cast_with_floating(&mut engine, p0, silk_net());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that cast the Net is the one that aims it"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is a permanent and no creature: {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered is the one it is aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the Net targets"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::REACH),
        "and the printed reach reaches it through the layers"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody aimed at is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::REACH),
        "\"target creature\" is not \"creatures you control\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf across the table was a legal target and was not the one \
         named, so the pump never crossed on its own"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::REACH),
        "nor did the keyword"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} came out of the pool the Forest filled"
    );
}

fn spark_spray() -> CardIndex {
    card_index("c37f1921-7ac3-4185-8579-1dfdfe647ea2")
}

/// Spark Spray is called that, but it is two cards: "{R} — Instant: Spark
/// Spray deals 1 damage to any target" and in hand "Cycling {R} ({R},
/// Discard this card: Draw a card.)". The battlefield plays both halves from
/// a single tapping of two Mountains, because a pool only empties at the end
/// of the step (CR 500.5): the second copy is cycled first and must discard
/// exactly itself to draw a card, and afterwards the remaining copy kills a
/// printed 1/1 opponent — while the second Elf remains and p1's life stays
/// at 20, which proves the targeting question "any target" (CR 115.4) on
/// both lists instead of assuming it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn spark_spray_cycles_itself_for_a_card_and_burns_a_creature_for_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[spark_spray(), spark_spray()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "zwei 1/1er, einer davon der Zuschauer");
    let (doomed, bystander) = (elves[0], elves[1]);

    // Both halves of the card cost {R}, so the two Mountains pay
    // once for the cycle and once for the spell.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red mana, and the creatures above them are tapped"
    );

    // The hand half: the cycling ability is offered on the card in hand,
    // not on a permanent.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == spark_spray()))
        })
        .expect("Cycling is offered for the card in hand");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the offered ability is payable");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        mine(&engine, p0, spark_spray(), Zone::Graveyard).len(),
        1,
        "\"Discard this card\" is the cost, and it takes the card that prints it"
    );
    assert!(
        in_hand(&engine, p0, spark_spray()).is_some(),
        "the other copy is still in hand: Cycling doesn't eat the card"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — a card from the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discard and the turn cancel each other out"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the first {{R}} is paid, the second is still floating for the spell"
    );

    // The spell half: 1 damage to a target that may be a creature.
    cast_with_floating(&mut engine, p0, spark_spray());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops only on a target question")
    };
    assert_eq!(player, p0, "der wirkende Sitz zielt");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        options.contains(&doomed) && options.contains(&bystander),
        "\"any target\" reaches either creature: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: a player is an \"any target\" too: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("der Elf war eine der angebotenen Optionen");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "1 damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and only the named creature: the other Elf is still standing"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature and not to the player behind it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{R}} was the spell's price"
    );
    assert_eq!(
        mine(&engine, p0, spark_spray(), Zone::Graveyard).len(),
        2,
        "both halves of the card are played: once canceled, once cast"
    );
}

fn sprout() -> CardIndex {
    card_index("424c6f5e-b386-47e9-b3fe-25b263097d40")
}

/// Sprout is one line — `{G}` for "Create a 1/1 green Saproling creature
/// token" — so the whole card is the token, and the only way to tell a token
/// from a spell that did nothing is to read the permanent that arrived. One
/// Forest pays for it, which makes the pool reading exact: the Saproling
/// stands on a board that held nothing before the cast, it is a *creature*
/// (an Effect that merely announced a token would leave the type line empty),
/// and its 1/1 green body is the printing rather than a default. The spell
/// itself is looked for in the graveyard afterwards, which is where a resolved
/// instant goes and where a case that never resolved would not be.
#[test]
fn sprout_creates_a_one_one_green_saproling_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[sprout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the board holds no token before the spell is cast"
    );

    cast_from_hand(&mut engine, p0, sprout());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sprout()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation of the spell, one token");
    let saproling = engine
        .state()
        .object(tokens[0])
        .expect("the token is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(saproling.name, "Saproling");
    assert_eq!(
        (saproling.power, saproling.toughness),
        (Some(1), Some(1)),
        "a 1/1, and not a bodyless token the state-based checks would eat"
    );
    assert!(
        saproling.colors.contains(baylee_core::color::Color::Green),
        "green, which no other part of the board could have supplied"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "and it is a creature, not merely a permanent that appeared"
    );
}

fn stand_firm() -> CardIndex {
    card_index("c0406e70-8131-4e13-b1d5-4e943ad296b8")
}

/// Stand Firm prints two sentences: "Target creature gets +1/+1 until end of
/// turn." and "Scry 2." Both are only worthwhile together, so next to the
/// bearer there is a second Elf on the table — "target creature" is any
/// creature, and `(2, 2)` versus `(1, 1)` rules out "creatures you control"
/// at the same time. The scry is read as *movement* and not as a posed
/// question: the chosen card is then on the bottom, the other is the new top,
/// and the library is as long as before — scry looks and sorts, it draws
/// nothing.
#[test]
fn stand_firm_pumps_the_creature_it_names_and_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .hand(0, &[stand_firm()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // The two cards the scry is about to look at, named before anything is
    // cast. The list's last entry is the top of the library — the order
    // `Effect::Scry` reads the top `n` in, and the end `ZonePosition::Bottom`
    // writes to.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    cast_from_hand(&mut engine, p0, stand_firm());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    // CR 601.2c picks the target while the spell is still being cast, so
    // nothing has resolved yet: the Elf is still the 1/1 it was printed as.
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the target is named before the spell resolves"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("a card the scry put on the menu is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "scry draws nothing, so the library is the length it was"
    );

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 until end of turn on the creature the spell named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the Elf it did not"
    );
}

fn tunnel() -> CardIndex {
    card_index("80559618-9dd9-4987-b3bc-1a1b5537bbc5")
}

/// Wall of Roots — `{1}{G}` 0/5 Plant Wall, the pool's plainest Wall and the
/// only one a test can name by oracle id.
fn wall_of_roots() -> CardIndex {
    card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
}

/// Tunnel prints one sentence — "Destroy target Wall. It can't be
/// regenerated." — and the pool implements no regeneration anywhere, so the
/// whole card is the destroy plus the word *Wall*. The board therefore puts a
/// Wall and a creature that is not one under the same opponent: the offer has
/// to hold exactly the first, and the Elf beside it is the control that says
/// the subtype filter was read rather than that a lone creature turned out to
/// be legal. Afterwards the Wall is in its owner's graveyard and the Elf is
/// untouched — a spell that killed whatever it was aimed at would still leave
/// the first assertion green and the second one red.
#[test]
fn tunnel_destroys_a_wall_and_declines_a_creature_that_is_not_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[tunnel()])
        .battlefield(1, &[wall_of_roots(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p1, wall_of_roots()).expect("the Wall is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");

    // {R} off the one Mountain, and the spell asks for its target as it is
    // cast (CR 601.2c) rather than when it resolves.
    cast_from_hand(&mut engine, p0, tunnel());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Wall\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one target, and no \"up to\"");
    assert!(
        options.contains(&wall),
        "a Wall is the one thing Tunnel may point at: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "a creature that is no Wall is not a legal target: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the Wall is the whole menu, on a board that holds four creatures: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .expect("the Wall was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wall_of_roots()).is_some(),
        "the targeted Wall went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, wall_of_roots()).is_none(),
        "and it left the battlefield rather than merely changing hands"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, tunnel()).is_some(),
        "an instant that resolved is in the graveyard of the seat that cast it"
    );
}

fn accelerate() -> CardIndex {
    card_index("79189464-9645-4411-8886-68fd40a588ed")
}

/// Accelerate — {1}{R} instant: "Target creature gains haste until end of
/// turn. Draw a card."
///
/// The two printed sentences are one card, and the board makes each of them
/// separately readable: "target creature" is any creature and not "you
/// control", so the Elf across the table is on the menu while the Elf that is
/// named is the only one of the three to end up with haste, and the draw is
/// read off the library where a card that only pumped could not show it. The
/// opponent's turn is walked to afterwards because the keyword is "until end
/// of turn" — reading haste in the turn it was granted cannot tell a duration
/// from a static.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn accelerate_grants_haste_to_the_creature_it_targets_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[accelerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    for id in [host, bystander, theirs] {
        assert!(
            !keywords(&engine, id).contains(KeywordSet::HASTE),
            "a printed Llanowar Elves has no haste of its own"
        );
    }

    // Both Mountains, and the Elves named as the printing kept back: they are
    // the creatures the spell is about, and a mana creature tapped for the mana
    // would leave a board this test no longer reads.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, and nothing off either Elf"
    );
    let card = in_hand(&engine, p0, accelerate()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with {{1}}{{R}} in the pool the spell is castable: {:?}",
        legal.castable
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_with_floating(&mut engine, p0, accelerate());

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "exactly one creature, no more and no fewer"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "no player is a creature: {player_options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2c before CR 601.2h: the target is chosen while the mana is \
         still floating"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::HASTE),
        "\"target creature gains haste until end of turn\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::HASTE),
        "the Elf nobody named is untouched: the pump reaches the target and \
         no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "nor across the table, where the same Elf was on the menu and was not \
         named"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, accelerate()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card left the hand with the spell and one came back with the draw"
    );

    // "Until end of turn" is a duration and not a static, which the turn it
    // was cast in cannot tell: the keyword is read again on the opponent's
    // turn, past the cleanup step that ends it.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::HASTE),
        "the granted haste expired with the turn it was granted in"
    );
}

fn aggressive_urge() -> CardIndex {
    card_index("43f5a93b-0f8d-48d2-ab9d-275d44cf88b5")
}

/// Aggressive Urge prints two effects on one instant for `{1}{G}`: "Target
/// creature gets +1/+1 until end of turn" and "Draw a card."
///
/// The two printed effects are read where each one leaves a mark a test can
/// see. The pump is read off the board after the spell has resolved, on the
/// creature the target question named and on no other — so a second Elf
/// under the same seat and an Elf across the table are both standing there
/// to catch a `Filter` that had widened. The draw is read off the library
/// and the hand together, because a count alone would be satisfied by a
/// card that left the library without arriving anywhere.
///
/// The four Forests, and neither Elf tapped for any of it: a creature that
/// paid its own `{T}` toward the spell it is about to be the target of is a
/// creature whose board state already changed for a reason of its own.
#[test]
fn aggressive_urge_pumps_the_target_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(83, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[aggressive_urge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the pump");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Two of the four Forests pay the {1}{G}; the Elves are named as the
    // printing kept back, since they are the creatures this test reads back.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped and neither Elf"
    );
    cast_with_floating(&mut engine, p0, aggressive_urge());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    // CR 601.2c names the target and CR 601.2h pays afterwards, so the mana is
    // still floating while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the spell is pointed at"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody pointed at is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the target and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}}{{G}} came out of the four Forests"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and it is in hand: the spell left the hand and the card it drew \
         took its place, so the count is where it started"
    );
}

fn aura_blast() -> CardIndex {
    card_index("4e3c3bdc-667d-42ec-b960-159a39c53cb3")
}

/// Aura Blast costs `{1}{W}` and prints two sentences: "Destroy target
/// enchantment" and "Draw a card."
///
/// Two Plains pay the cost, and the only enchantment in the game is across the
/// table, so the spell has to reach over and take it — while the Sol Ring
/// standing beside it is an artifact and the whole reason the target filter is
/// read rather than skipped. The draw is asserted as a *move* rather than as a
/// question that was asked: the card on top of the library before the cast is
/// the card in hand afterwards, which is the half a destroyed enchantment
/// alone could never prove.
#[test]
fn aura_blast_destroys_an_enchantment_across_the_table_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[their_enchantment(), quiet_artifact()])
        .hand(0, &[aura_blast()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let theirs = on_battlefield(&engine, p1, their_enchantment()).expect("the enchantment is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("the Sol Ring is out");
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");

    // Two Plains, tapped before the cast: the card leaves the hand as a
    // payment, and nothing about how it is paid is what this test reads.
    cast_from_hand(&mut engine, p0, aura_blast());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one enchantment");
    assert!(
        options.contains(&theirs),
        "\"target enchantment\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "an artifact is not an enchantment: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that one is the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the enchantment the question offered is the one it named");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and it went to its owner's graveyard, which is the seat that had it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, aura_blast()).is_some(),
        "and the instant itself is spent: a resolved instant goes to its \
         owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top before the cast, not merely \
         one card fewer in the library"
    );
}

fn boomerang() -> CardIndex {
    card_index("dc4a4996-108a-4aac-850f-2d9f76403446")
}

/// Boomerang — {U}{U} instant: "Return target permanent to its owner's hand."
///
/// The two words that carry the card are "permanent" and "owner's", so both
/// sides of the table are read: the offer has to name an Island this seat
/// controls as well as the Elf across it, the Elf is the one that is answered,
/// and it has to land in *its owner's* hand rather than in the hand of the seat
/// that aimed the spell. Casting it in the opponent's own main phase is the
/// other half — the card is an instant, and a bounce that could only be played
/// on its controller's turn would look identical on every board above.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn boomerang_returns_a_permanent_across_the_table_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[boomerang()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // The non-active seat holding priority in the active seat's own main phase
    // is the half of "instant" no board on p0's turn can show.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let mine = on_battlefield(&engine, p0, island()).expect("my Island is on the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is on the table");
    let permanents = engine.state().zones.list(ZoneLocation::Battlefield).len();

    // Mana before the claim: the offer is read off the pool, not off the two
    // untapped Islands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "the two Islands are exactly the {{U}}{{U}} the card costs"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, boomerang()).expect("the Boomerang is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{U}}{{U}} is in the pool and an instant may be cast in an opponent's \
         main phase: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the mana already floating pays for it");

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat casting the spell is the one aiming it"
    );
    assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
    assert!(
        player_options.is_empty(),
        "\"target permanent\" is not \"any target\": no player may be named \
         (CR 115.4): {player_options:?}"
    );
    assert_eq!(
        options.len(),
        permanents,
        "\"target permanent\" is every permanent in the game: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "an Island of mine and the Elf across the table are both permanents \
         (CR 110.1): {options:?}"
    );
    // Still in **hand**, and that is this engine's announcement rather than a
    // bug: `cast_wizard` asks every question CR 601.2b–h poses and moves the
    // card to the stack last, at CR 601.2i, so the whole announcement is
    // atomic from the outside. Nobody can tell: no player gets priority
    // until 601.2i, and CR 115.5 makes a spell an illegal target for
    // itself, so there is no legal question whose answer differs.
    assert!(
        in_hand(&engine, p0, boomerang()).is_some(),
        "the card has not reached the stack yet (CR 601.2i comes last)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the permanent it is aimed at has not moved: the effect resolves \
         off the stack"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf was one of the permanents the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes to the seat that owns it, not to \
         the seat that aimed the spell"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and nothing of the opponent's reached the caster's hand"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "a bounce is not a destroy: the Elf is in a hand and in no graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, boomerang()).is_some(),
        "and the instant that resolved is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}}{{U}} came out of the pool"
    );
}

fn clear() -> CardIndex {
    card_index("d0783be9-e518-431a-8e71-121610da50e1")
}

/// Clear — {1}{W} Instant: "Destroy target enchantment."
///
/// The whole spell is that one sentence, so the board is built to make the
/// printed word the only thing that can decide the outcome: one enchantment
/// stands *across* the table and one Forest stands beside it. The Forest is
/// the witness — a filter widened to "target permanent" would offer it, and
/// one narrowed to "enchantment you control" would offer nothing at all — and
/// the card's own graveyard entry is what separates destruction from an exile
/// or a bounce that also took the permanent off the battlefield.
#[test]
fn clear_destroys_the_enchantment_across_the_table_and_nothing_beside_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[clear()])
        .battlefield(1, &[their_enchantment(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doomed = on_battlefield(&engine, p1, their_enchantment()).expect("the enchantment is out");
    let witness = on_battlefield(&engine, p1, forest()).expect("and a Forest beside it");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the cast: the two Plains are the whole price"
    );

    // Two Plains pay {1}{W} and nothing else on this board can — the Forest
    // belongs to the other seat — so the pool read afterwards is the printed
    // cost and not a stray source.
    cast_from_hand(&mut engine, p0, clear());

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
    assert_eq!(player, p0, "the caster aims their own spell");
    assert_eq!((min, max), (1, 1), "\"target enchantment\" is exactly one");
    assert!(
        options.contains(&doomed),
        "\"target enchantment\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&witness),
        "a Forest is a permanent and no enchantment: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and those two readings are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the enchantment the question offered is the one it destroys");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and it is in its owner's graveyard, which is where a destroyed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, clear()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{W}} came out of the pool the two Plains filled"
    );
}

fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

/// Disenchant — {1}{W} instant: "Destroy target artifact or enchantment."
///
/// The offer is the half a file reading cannot see: `Filter::ARTIFACT_OR_ENCHANTMENT` has to
/// name both an artifact and an enchantment across the table and decline the Elf standing
/// beside them, which is what says the two words are one filter rather than "target
/// permanent". Only the artifact is then named, so the enchantment is the bystander that
/// proves one target died and not the offer's whole menu — and the caster's own graveyard is
/// where the resolved instant has to land.
#[test]
fn disenchant_destroys_the_artifact_it_names_and_leaves_the_rest_of_the_table_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[disenchant()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let artifact = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let enchantment =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Two Plains into the pool first: whether a {1}{W} spell is castable is
    // read off the pool and not off the lands that are still standing.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, disenchant());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Disenchant targets, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&artifact),
        "an artifact is a legal target: {options:?}"
    );
    assert!(
        options.contains(&enchantment),
        "\"or enchantment\" reads both halves of the filter: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![artifact],
            },
        )
        .expect("the artifact was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact the spell named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and a destroyed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "the enchantment it did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and neither did the creature the filter declined"
    );
    assert!(
        in_graveyard(&engine, p0, disenchant()).is_some(),
        "the resolved instant is in its caster's graveyard"
    );
}

fn eladamri_s_call() -> CardIndex {
    card_index("4acb6612-54e8-428d-acb6-c7259a5ad6a8")
}

/// Eladamri's Call — {G}{W} instant: "Search your library for a creature
/// card, reveal that card, put it into your hand, then shuffle."
///
/// The board makes the filter legible rather than assumed: the 60-card deck is
/// Forests, so the only creature card in the game is one Llanowar Elves moved
/// out of the hand into the library, and an offer holding exactly that card is
/// "creature card" being read while a library of lands is passed over — a
/// search that had dropped the filter would have offered all of it. The move
/// afterwards is read off the zones, because the reveal is not a state: one
/// card leaves the library (the shuffle reorders what is left without changing
/// how much of it there is) and lands in the hand, while the {G}{W} is paid out
/// of a pool the Forest and the Plains filled and nothing else could have.
#[test]
fn eladamri_s_call_finds_the_one_creature_card_in_a_library_of_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[eladamri_s_call(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The one creature card anywhere, put where the Call has to go looking for
    // it: the harness' own dev capability, used exactly as `seed_graveyard`
    // uses it, since `SeatSpec` has no field for a library.
    let elf = in_hand(&engine, p0, llanowar_elves()).expect("the Elf is in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            elf,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    // The offer was computed when priority was granted, which was before the
    // Elf left the hand, so it has to be recomputed before it is read again.
    engine.refresh_offer();

    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, eladamri_s_call());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{W}} out of the two basic lands, and nothing left floating"
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
    assert_eq!(player, p0, "the caster searches their own library");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        options,
        vec![elf],
        "one creature card in a library of lands: \"creature card\" is read, \
         not skipped"
    );

    // Captured with the instant already off the hand and on the stack, so the
    // only thing that can move the count is the card the search is about to
    // hand over.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        in_hand(&engine, p0, llanowar_elves()),
        Some(elf),
        "\"put that card into your hand\": the very card the search offered, \
         and not another copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the hand is one card up — a reveal that left the card where it was \
         could not do that"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what remains without changing how much of it there is"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .iter()
            .all(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
            }),
        "the library is the filler deck again: the one creature card in the \
         game is the one that moved"
    );
    assert!(
        in_graveyard(&engine, p0, eladamri_s_call()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
}

// oracle_id = "b2c9f074-57ca-4709-976d-f432f632483f"
fn extinguish() -> CardIndex {
    card_index("b2c9f074-57ca-4709-976d-f432f632483f")
}

/// Extinguish — {1}{U} instant: "Counter target sorcery spell."
///
/// Both words of the restriction are the engine's answer rather than the
/// card's, so a sorcery has to actually be on the stack: p0 casts Vindicate
/// at p1's Sol Ring, and the counter is then read off what becomes of both
/// cards. The target question is the first half — the spell p0 just cast is
/// on the menu, and the permanent that spell is aimed at is not — and the
/// resolution is the second: the sorcery lies in its owner's graveyard
/// without ever destroying anything, while the Sol Ring is still standing
/// and the stack is empty.
#[test]
fn extinguish_counters_the_sorcery_on_the_stack_and_leaves_its_target_standing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), swamp()])
        .hand(0, &[vindicate()])
        .battlefield(1, &[island(), island(), quiet_artifact()])
        .hand(1, &[extinguish()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A sorcery belongs in its caster's own main phase (CR 307.1), so this is
    // where the card under test gets something legal to counter.
    let victim = on_battlefield(&engine, p1, quiet_artifact()).expect("the Sol Ring stands");
    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Vindicate targets a permanent, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster names its own target");
    assert!(
        options.contains(&victim),
        "the Sol Ring is a permanent and so a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the permanent the cast offered was named");
    let spell = on_stack(&engine, vindicate()).expect("the sorcery is on the stack, unresolved");

    // The active player holds priority first after casting (CR 117.3c), so p1
    // has to be handed it before it can answer.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );

    // Two Islands and nothing else: the Sol Ring is the spell's target and is
    // left untapped, so the {1}{U} is paid entirely out of the lands.
    tap_all_mana_but(&mut engine, p1, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "two Islands, exactly the {{1}}{{U}} Extinguish charges"
    );
    cast_with_floating(&mut engine, p1, extinguish());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target sorcery spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it chooses");
    assert!(
        options.contains(&spell),
        "the sorcery p0 just cast is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&victim),
        "the Sol Ring is a permanent and no spell on the stack: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the spell on the stack was one of the options");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, vindicate()).is_none(),
        "the countered spell left the stack rather than resolving"
    );
    assert!(
        in_graveyard(&engine, p0, vindicate()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent Vindicate aimed at is untouched: the counter stopped \
         the effect and not merely the card"
    );
    assert!(
        in_graveyard(&engine, p1, extinguish()).is_some(),
        "and the instant that did it resolved into its caster's graveyard"
    );
}

fn false_summoning() -> CardIndex {
    card_index("4f891c68-c959-4210-94e5-94a8e487d5ef")
}

/// False Summoning — {1}{U} instant: "Counter target creature spell."
///
/// The whole card is one target spec, and the two words a wider filter would
/// lose are "spell" and "creature": an Elf already standing on the battlefield
/// is the permanent that a bare `Filter::CREATURE` would have offered, so the
/// counter's menu has to name the Elf *card* on the stack and decline the Elf
/// permanent while it does. Playing it means playing both sides of a real
/// stack — p0 casts the Elf, p1 answers it — and the {1}{U} leaves the pool only
/// after the target is named (CR 601.2c, then CR 601.2h), which is why the
/// mana is read on both sides of that answer.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn false_summoning_counters_a_creature_spell_and_never_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(409, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[false_summoning()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real creature spell on the stack, with p1 to answer it: the walk stops
    // on p1's priority rather than letting the Elf resolve, because a resolved
    // Elf is no longer a spell and the card has nothing left to target.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        on_stack(e, llanowar_elves()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let spell = on_stack(&engine, llanowar_elves()).expect("the Elf is a spell on the stack");
    let permanent =
        on_battlefield(&engine, p0, llanowar_elves()).expect("an Elf is a permanent already");

    // Mana before the claim: `castable` is filtered through `can_afford`,
    // which reads the pool and not the untapped Islands.
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the answering seat holds priority");
    let card = in_hand(&engine, p1, false_summoning()).expect("the counter is in hand");
    assert!(
        legal.castable.contains(&card),
        "{{1}}{{U}} is floating, so the counter is castable: {:?}",
        legal.castable
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "two Islands, two blue"
    );

    engine
        .apply(p1, PlayerAction::CastSpell { card })
        .expect("the counter is cast");
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        player_options.is_empty(),
        "\"target creature spell\" counts objects and no players: {player_options:?}"
    );
    assert!(
        options.contains(&spell),
        "the creature spell on the stack is the target the card prints: {options:?}"
    );
    assert!(
        !options.contains(&permanent),
        "\"spell\" is not \"permanent\": the Elf already on the battlefield is a \
         creature and never a legal target: {options:?}"
    );
    assert_eq!(options.len(), 1, "and that spell is the whole menu");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "CR 601.2h pays last: the target is named first, so the {{1}}{{U}} is \
         still floating while the question stands"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the spell the question offered is the one that was named");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "and the cost came out of the pool with the answer"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "\"counter\" sends the spell to its owner's graveyard: the countered Elf \
         is the card that was on the stack, and a spell that had resolved would \
         have left nothing there"
    );
    assert!(
        in_graveyard(&engine, p1, false_summoning()).is_some(),
        "and the counter itself resolved into its caster's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()).len(),
        1,
        "the Elf already on the battlefield never became a target and is still \
         the one permanent it was"
    );
}

// oracle_id = "fc05e582-e760-4fe2-ba43-e9d8e63f3f85"
fn fists_of_the_anvil() -> CardIndex {
    card_index("fc05e582-e760-4fe2-ba43-e9d8e63f3f85")
}

/// Fists of the Anvil — {1}{R} Instant: "Target creature gets +4/+0 until
/// end of turn."
///
/// A pure pump is only itself when both halves of the printed sentence are
/// read off the board, so the creatures beside it are the controls: the Elf
/// across the table is a printed 1/1 that a filter which had lost "target"
/// would also have offered, and `(5, 1)` is the one body that reads both
/// printed numbers — a `(5, 5)` would mean a toughness the card never
/// prints, and `(1, 1)` that the pump never landed at all. The "until end of
/// turn" half is asserted a turn later, where the same Elf has to be back to
/// its printed body.
#[test]
fn fists_of_the_anvil_pumps_the_creature_it_targets_for_power_and_no_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fists_of_the_anvil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // Mana before the claim: `castable` is read off the pool and not off the
    // untapped Mountains (a pool survives until the step ends, CR 500.5, and
    // this whole cast happens inside one main phase).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Mountains and the Elf's own {{G}}, which is every source on this board"
    );
    cast_with_floating(&mut engine, p0, fists_of_the_anvil());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (5, 1),
        "+4/+0 on the creature it targeted — a (5, 5) would be a toughness \
         the card does not print"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf the spell did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{R}} came out of the pool the three sources filled"
    );
    assert!(
        in_graveyard(&engine, p0, fists_of_the_anvil()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );

    // "Until end of turn" is the half nothing about the offer or the pump
    // could show: a turn later the body is the printed one again.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the pump lasted the turn it was cast and no longer"
    );
}

fn flowstone_strike() -> CardIndex {
    card_index("d67048eb-eaf1-4b5b-a8e3-a004bab438f3")
}

/// Flowstone Strike prints one line — "Target creature gets +1/-1 and gains
/// haste until end of turn" — and one casting reads all of it. `(7, 5)` on a
/// printed 6/6 is the only body that applies a power *up* and a toughness
/// *down*: a symmetric pump would leave `(7, 7)` and a swapped pair would read
/// `(5, 7)`. The haste is read through the layers rather than off the card
/// file, and the Elf across the table is the control — the offer names it,
/// because "target creature" is not "target creature you control", and the
/// seat whose creature was not named keeps its printed body and no keyword.
#[test]
fn flowstone_strike_gives_one_creature_plus_one_minus_one_and_haste() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), rootbreaker_wurm(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flowstone_strike()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let bystander =
        on_battlefield(&engine, p0, llanowar_elves()).expect("a second creature of mine");
    let theirs =
        on_battlefield(&engine, p1, llanowar_elves()).expect("a creature across the table");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the spell");
    assert!(
        !keywords(&engine, wurm).contains(KeywordSet::HASTE),
        "and no haste until the sentence grants it"
    );

    // The two Mountains and no creature: the Elves are named as the printing
    // kept back, so "exactly two" is two red rather than a green the pool
    // would have to account for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two tapped Mountains, and neither Elf paid in"
    );
    cast_with_floating(&mut engine, p0, flowstone_strike());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&wurm) && options.contains(&bystander),
        "both creatures you control are on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"target creature you control\": {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (7, 5),
        "+1/-1 on the creature that was named — a (7, 7) would be a toughness \
         the card never lowers, and a (5, 7) would have the two halves swapped"
    );
    assert!(
        keywords(&engine, wurm).contains(KeywordSet::HASTE),
        "and the same sentence grants haste until end of turn"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the creature the spell did not name keeps its printed body"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::HASTE),
        "and the granted keyword reaches the target and no other"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor across the table: one creature was named, and it was not this one"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "the pump is one creature, wherever it stood"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool"
    );
}

fn gerrards_command() -> CardIndex {
    card_index("de8ebe4b-d86d-478b-946e-f83e1409b63f")
}

/// Gerrard's Command — {G}{W} instant: "Untap target creature. It gets +3/+3
/// until end of turn."
///
/// One target carries both printed sentences, so the board is built to show
/// they land on the creature the question named and on nothing else: two
/// Llanowar Elves under p0 are tapped for the mana that pays for the spell
/// (so the one it untaps is genuinely tapped rather than untapped anyway), and
/// a third Elf stands across the table because "target creature" (CR 115.1)
/// reaches all three. Only the Elf that was answered for comes back up and
/// reads 4/4 — an untap without the pump would leave it a 1/1, and a pump
/// without the untap would leave it lying down.
#[test]
fn gerrards_command_untaps_and_pumps_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[gerrards_command()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays out of it");
    let (named, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // Every mana source on the board, counted rather than assumed: the Forest
    // and the Plains, plus each Elf's own printed `{T}: Add {G}` — which is
    // what leaves the creature the spell is about to untap lying down.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "one green and one white off the lands, and two more green off the Elves"
    );
    assert!(
        is_tapped(&engine, named) && is_tapped(&engine, bystander),
        "both Elves paid for the spell with their own tap"
    );
    assert_eq!(pt(&engine, named), (1, 1), "a printed 1/1 while tapped");

    cast_with_floating(&mut engine, p0, gerrards_command());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it is the seat that aims it");
    assert!(
        options.contains(&named) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "\"target creature\" names no player (CR 115.1): {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![named],
            },
        )
        .expect("the creature the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, named),
        "\"untap target creature\" — the Elf that had just paid for the spell"
    );
    assert_eq!(
        pt(&engine, named),
        (4, 4),
        "+3/+3 on the creature the spell named, and nothing else"
    );
    assert!(
        is_tapped(&engine, bystander),
        "the Elf nobody named stays exactly where it tapped"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and unpumped — the effect does not read \"creatures you control\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor does it reach across the table, however wide the target filter is"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{G}}{{W}} came out of the pool, and the two green left are all that remains"
    );
}

fn guided_strike() -> CardIndex {
    card_index("a215e813-df7e-4f21-9718-eb264eb62813")
}

/// Guided Strike — {1}{W} instant: "Target creature gets +1/+0 and gains first
/// strike until end of turn. Draw a card."
///
/// Both sentences are one resolution, so the card has to be cast to be read at
/// all. "Target creature" names no controller, so the offer is taken with an
/// Elf standing on *each* side of the table: a 1/1 becoming `(2, 1)` is what
/// says the `+1/+0` was applied once and the toughness left alone, while the
/// Elf across the table staying a printed, keywordless 1/1 is what says the
/// pump reached the creature that was named and not the board. The draw is the
/// half no characteristic can show, so it is read as the library getting
/// shorter and the instant landing in its owner's graveyard.
#[test]
fn guided_strike_pumps_one_creature_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[guided_strike()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "and it has no first strike of its own"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Two Plains pay {1}{W}. The Elf is kept back because it is the creature
    // the spell is about to name, and a source tapped for its own mana has
    // already changed for a reason this test is not reading.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains in the pool, and the Elf paid nothing"
    );
    cast_with_floating(&mut engine, p0, guided_strike());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("my own Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (2, 1),
        "+1/+0 on the creature the spell named: power up, toughness untouched"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike reaches it through the layers"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "nor does the keyword cross the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw replaced it, so the hand is the \
         size it was"
    );
    assert!(
        in_graveyard(&engine, p0, guided_strike()).is_some(),
        "and the instant itself is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{W}} came out of the pool"
    );
}

fn hero_s_demise() -> CardIndex {
    card_index("69ec0b59-a0aa-4878-a3b3-130cea18fee0")
}

/// Hero's Demise — {1}{B} Instant: "Destroy target legendary creature."
///
/// The whole card is one filter, so the scenario is built to tell
/// `LEGENDARY_CREATURE` from every wider reading it could have been: a
/// legendary creature stands across the table with a plain Llanowar Elves
/// beside it, and the target question must offer exactly the first. That the
/// legend is the *opponent's* and is still on the menu is the other half —
/// nothing on the card says "you control" — and the Elves standing unharmed
/// afterwards is the control that the destruction came from the spell and
/// not from a legend-rule accident or a board-wide wipe.
#[test]
fn heros_demise_destroys_the_legend_and_leaves_the_plain_creature_standing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[katara_the_fearless(), llanowar_elves()])
        .hand(0, &[hero_s_demise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let legend = on_battlefield(&engine, p1, katara_the_fearless())
        .expect("the legendary creature stands across the table");
    let elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("and a plain creature beside it");
    assert_eq!(
        pt(&engine, legend),
        (3, 3),
        "a body for the removal to be read off"
    );

    // Two Swamps for {1}{B}, and `cast_from_hand` taps them before the cast
    // so the offer is read against a pool that can actually pay.
    cast_from_hand(&mut engine, p0, hero_s_demise());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target legendary creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        options.contains(&legend),
        "\"target legendary creature\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a plain Llanowar Elves is a creature and no legend, so it is not on \
         the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the legend is the whole of what the filter names on this board: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .expect("the creature the question offered is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, katara_the_fearless()).is_none(),
        "the targeted legend left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, katara_the_fearless()).is_some(),
        "and it is in its owner's graveyard, which is where a destroyed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, hero_s_demise()).is_some(),
        "the instant itself resolved and went to its caster's graveyard"
    );
}

fn heroes_reunion() -> CardIndex {
    card_index("beab66e0-2b6c-478f-ba42-8c0c216a527e")
}

/// Heroes' Reunion costs {G}{W} and prints one sentence: "Target player gains
/// 7 life." Both halves of that need a witness. The *player* half is read by
/// aiming the two copies this seat holds at opposite seats — the question has
/// to offer both, so "target player" is not "target opponent" — and the
/// printed seven is read off two seats that start at different totals, so
/// 30 → 37 and 20 → 27 are numbers neither seat could have reached through
/// the other's cast. The four lands are tapped before anything is claimed
/// (CR 500.5 keeps the pool through this one main phase), and two cards in
/// the graveyard are what says each cast resolved rather than waiting.
#[test]
fn heroes_reunion_gives_seven_life_to_the_player_it_names_and_no_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), forest(), forest()])
        .hand(0, &[heroes_reunion(), heroes_reunion()])
        .life(0, 30)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana before the claim: `castable` is read off the pool and not off the
    // untapped lands, so the spell is this seat's to cast only once the four
    // lands have paid in.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Plains and two Forests: {{G}}{{W}} twice over"
    );

    // A target that is a player is one question in either of the shapes the
    // engine gives it — a target choice whose object list is empty and whose
    // player list is the whole of it, or the bare player choice — and both
    // enumerate the same answers (CR 115.4), so the answer is read out of
    // whichever shape arrives.
    let aim_at = |engine: &mut Engine<RegistryLookup>, at: PlayerId| -> Vec<PlayerId> {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![at],
                        },
                    )
                    .expect("the player aimed at was one of the options offered");
                player_options
            }
            Pending::ChoosePlayer { player, options } => {
                engine
                    .apply(player, PlayerAction::ChoosePlayer(at))
                    .expect("the player aimed at was one of the options offered");
                options
            }
            other => panic!("\"target player\" is a question about players, got {other:?}"),
        }
    };

    cast_with_floating(&mut engine, p0, heroes_reunion());
    let offered = aim_at(&mut engine, p1);
    assert!(
        offered.contains(&p0) && offered.contains(&p1),
        "\"target player\" is any player, on either side of the table: {offered:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        27,
        "the named seat gained exactly seven"
    );
    assert_eq!(
        engine.state().players[0].life,
        30,
        "and the seat that was not named did not"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{G}}{{W}} came out of the pool"
    );

    // The second copy, aimed the other way: "target player" is not "target
    // opponent", so the same card has to be able to hand its seven back to
    // the seat that cast it.
    cast_with_floating(&mut engine, p0, heroes_reunion());
    let offered = aim_at(&mut engine, p0);
    assert!(
        offered.contains(&p0),
        "the caster is a legal target for their own spell: {offered:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 37, "seven for the caster");
    assert_eq!(
        engine.state().players[1].life,
        27,
        "and the first seven are still the other seat's"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{G}}{{W}} came out of the pool"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "one resolved instant per cast — a spell stuck on the stack would \
         leave this at one"
    );
    assert!(
        in_graveyard(&engine, p0, heroes_reunion()).is_some(),
        "and the cards are where a resolved instant goes"
    );
}

fn hisoka_s_defiance() -> CardIndex {
    card_index("2ac6f1c7-f4c5-4d45-9644-da49d8fe4758")
}

/// Hisoka's Defiance prints one sentence — "Counter target Spirit or Arcane
/// spell" — so the card is only itself when the *filter* decides what may be
/// aimed at, and the scenario plays both sides of it in one main phase. A Kor
/// Spirit spell is countered while it is still a spell; an Elf spell cast
/// afterwards is the control, and p1's second copy is read while the same
/// {1}{U} is floating, so where the offer disappears it disappears for the
/// subtype and for nothing else.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn hisokas_defiance_counters_a_spirit_spell_and_declines_an_elf() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[plains(), plains(), forest(), forest()])
        .hand(0, &[skyclave_apparition(), llanowar_elves()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[hisoka_s_defiance(), hisoka_s_defiance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{W}{W} off both Plains and one Forest. The other Forest is named as
    // the source kept back: it is what pays for the Elf in the second half,
    // and tapping it here would leave that cast with nothing behind it.
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        forests.len(),
        2,
        "a Forest to pay with and a Forest to keep"
    );
    tap_mana_except(&mut engine, p0, forests[1]);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Plains and one Forest is exactly the {{1}}{{W}}{{W}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spirit = in_hand(&engine, p0, skyclave_apparition()).expect("the Spirit is in hand");
    assert!(
        legal.castable.contains(&spirit),
        "the three mana in the pool pay for it: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, skyclave_apparition());

    // p1 gets priority while the Spirit spell is still on the stack: a spell
    // is countered while it is a spell, and once it resolves there is nothing
    // left to point at.
    pass_until(&mut engine, |e| {
        on_stack(e, skyclave_apparition()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let apparition_spell =
        on_stack(&engine, skyclave_apparition()).expect("the Spirit spell is on the stack");

    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        4,
        "four Islands, four blue"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "and it is p1 being asked");
    let defiance = in_hand(&engine, p1, hisoka_s_defiance()).expect("the Defiance is in hand");
    assert!(
        legal.castable.contains(&defiance),
        "a Spirit spell on the stack and {{1}}{{U}} floating is a castable \
         Hisoka's Defiance: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p1, hisoka_s_defiance());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Spirit or Arcane spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "one spell, no more and no fewer");
    assert!(
        options.contains(&apparition_spell),
        "the Kor Spirit is one of the options: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the only spell on the stack, so the filter is the whole menu: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![apparition_spell],
            },
        )
        .expect("the spell the question offered was one of its options");

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        in_graveyard(&engine, p0, skyclave_apparition()).is_some(),
        "a countered spell goes to its owner's graveyard, not to the counter's"
    );
    assert!(
        on_battlefield(&engine, p0, skyclave_apparition()).is_none(),
        "and the Spirit never arrived, so its enters-trigger never fired"
    );
    assert!(
        in_graveyard(&engine, p1, hisoka_s_defiance()).is_some(),
        "the Defiance itself resolved and went to the graveyard"
    );

    // The control, and the half of the printed sentence that is a filter. The
    // Elf spell is no Spirit and no Arcane spell, and p1's pool still holds
    // the second {1}{U} out of the four Islands — so the price is no reason to
    // decline, and a Defiance that had stopped reading the subtype would be
    // offered here.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("p1 holds priority again, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "and it is p1 being asked");
    let second = in_hand(&engine, p1, hisoka_s_defiance()).expect("the second copy is in hand");
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the Elf spell is on the stack, which is where a counter would aim"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "and the second {{1}}{{U}} is still floating, so the price is no \
         reason to decline"
    );
    assert!(
        !legal.castable.contains(&second),
        "an Elf spell is neither a Spirit nor an Arcane spell, so the second \
         Defiance has nothing it may be pointed at: {:?}",
        legal.castable
    );
}

fn hoodwink() -> CardIndex {
    card_index("a2fe10b8-857e-4881-85ef-24a0f96a3d79")
}

/// Hoodwink — {1}{U} instant: "Return target artifact, enchantment, or land to
/// its owner's hand."
///
/// The filter is three types wide, so the board carries one permanent of each
/// legible kind across the table — a land and an artifact — plus a creature
/// under the caster's own control. The Elf is the counter-half: it is a
/// permanent a "target permanent" reading would have offered, and this card
/// must not, which no comparison of `CardDef` fields can separate from a filter
/// that quietly lost its type list. Aiming the spell at the opponent's Forest
/// is what reads the rest of the sentence: the card has to reach the hand of
/// the seat that *owns* it and not the seat that cast the spell, and the
/// artifact beside it and the caster's own board are the controls that exactly
/// one card moved.
#[test]
fn hoodwink_bounces_an_opponents_land_to_its_owners_hand_and_never_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[forest(), quiet_artifact()])
        .hand(0, &[hoodwink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let my_land = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");

    // {1}{U} off two Islands; the Elf is tapped by the helper too, which is
    // harmless here — nothing below counts the pool or the Elves' status.
    cast_from_hand(&mut engine, p0, hoodwink());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster is the seat that aims the spell");
    assert!(
        options.contains(&their_land) && options.contains(&their_rock),
        "an opponent's land and an opponent's artifact are both \"target \
         artifact, enchantment, or land\": {options:?}"
    );
    assert!(
        options.contains(&my_land),
        "and so is the caster's own land — the filter names no controller: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is none of the three types the card prints, which is what \
         tells this filter from \"target permanent\": {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "the two Islands, their Forest and their artifact are the whole menu; \
         the Elf is the only permanent on the table the card declines: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the Forest was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the targeted land left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, forest()).is_some(),
        "\"to its owner's hand\": the Forest goes back to the seat that owns \
         it, not to the seat that cast the spell"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_none(),
        "and not to the caster's hand, which is where a bounce misread as \
         \"return it to your hand\" would have put it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "one target moved one card: the artifact beside it never moved"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature the spell could not name is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, hoodwink()).is_some(),
        "an instant that resolves goes to its caster's graveyard"
    );
}

fn mage_s_guile() -> CardIndex {
    card_index("f371cdaf-7ab8-4555-8c96-61dfbf8c0179")
}

/// Mage's Guile prints two lines: "{1}{U}: Target creature gains shroud until
/// end of turn" and "Cycling {U}". The shroud half is read twice, because a
/// projected keyword is only half the sentence — "it can't be the target of
/// spells or abilities" is a targeting restriction, so the same creature has
/// to be missing from the menu of an ability that could otherwise equip it,
/// while the Elf beside it stays offered. The cycling half is then paid out of
/// hand, which is a cost of its own: the card discards itself and the draw
/// replaces it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn mages_guile_shrouds_the_creature_it_names_and_cycles_itself_away() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                lightning_greaves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[mage_s_guile(), mage_s_guile()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which is the control");
    let (host, bystander) = (elves[0], elves[1]);
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are out");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "a printed 1/1 has no shroud before the spell"
    );

    // Three Islands and both Elves are five mana, and a pool survives until the
    // step ends (CR 500.5) — one main phase pays `{1}{U}` now and the cycling
    // `{U}` afterwards.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "three Islands and both Elves tapped for it"
    );

    cast_with_floating(&mut engine, p0, mage_s_guile());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both Elves are creatures it may name: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "\"target creature gains shroud\" — the creature it named"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "and the Elf standing beside it gains nothing"
    );

    // The grant is a restriction as well as a keyword, and `Equip {0}` is the
    // cheapest question that can see it: it costs no mana at all, so the offer
    // below turns on the shroud and on nothing else.
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !options.contains(&host),
        "\"it can't be the target of spells or abilities\": {options:?}"
    );
    assert!(
        options.contains(&bystander),
        "while the creature without shroud is still offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .expect("the creature the equip question offered");
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(bystander))
    });

    // Cycling {U} (CR 702.29a): a second copy is still in hand, and both the
    // mana and the card itself are the price.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let cycler = in_hand(&engine, p0, mage_s_guile()).expect("the second copy is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cycler, 0)),
        "cycling is an activated ability of the card in hand: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, mage_s_guile(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        mine(&engine, p0, mage_s_guile(), Zone::Graveyard).len(),
        2,
        "the resolved copy and the cycled copy are both in the graveyard — \
         discarding the card is the cost, not a rider"
    );
    assert!(
        in_hand(&engine, p0, mage_s_guile()).is_none(),
        "and no copy of it is left in hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one discarded and one drawn leave the hand the size it was, so an \
         empty library would not satisfy the count above"
    );
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

#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn magma_jet_kills_a_one_one_across_the_table_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[magma_jet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, elf), (1, 1), "a 1/1 for two damage to kill");

    // The two cards the scry is about to look at, named before anything is
    // cast: the list's last entry is the top of the library and its first is
    // the bottom, which is the order `Effect::Scry` reads the top `n` in.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red: nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, magma_jet()).expect("the Jet is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{1}}{{R}} is in the pool, so the Jet is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, magma_jet());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&elf),
        "the Elf across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    // The spell resolves and the scry is the last thing it does, so this is
    // where the card asks its second question — the walk stops on the
    // question rather than answering it, because the *answer* is the subject.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("one of the two just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and what died left the battlefield"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the \
         player whose board it stood on"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the two red Mountains"
    );

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(library.len(), library_before.len(), "scry draws nothing");
}

fn naturalize() -> CardIndex {
    card_index("bdb3ca68-ec1f-4e16-81cc-d23f8f52c728")
}

/// Naturalize — {1}{G} instant: "Destroy target artifact or enchantment."
///
/// Three words carry the card and the board gives each of them a witness: a
/// Sol Ring under the caster, a Sol Ring and an enchantment across the table,
/// and a creature and two lands that "artifact or enchantment" must decline —
/// so the offer read while the spell is on the stack is a filter and not the
/// whole board. The enchantment is the one that dies, which is the half an
/// artifact-only reading would have skipped, and the permanents beside it are
/// what say one named target was destroyed rather than a sweep.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn naturalize_destroys_the_artifact_or_enchantment_its_caster_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), quiet_artifact()])
        .hand(0, &[naturalize()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my artifact is out");
    let my_land = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let doomed =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Mana before the claim: `legal.castable` is filtered through
    // `can_afford`, which reads the pool and not the untapped lands. The Sol
    // Ring is named as the thing kept back so that "two" is the two Forests
    // and nothing else.
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Forests, and the artifact left standing"
    );
    cast_with_floating(&mut engine, p0, naturalize());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact or enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster is the one that chooses");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&mine),
        "\"target artifact\" reaches this seat's own board too: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and the artifact across the table: {options:?}"
    );
    assert!(
        options.contains(&doomed),
        "and the enchantment beside it — the second word of the filter: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment: {options:?}"
    );
    assert!(
        !options.contains(&my_land),
        "nor is a land, whatever an unfiltered target spec would have offered: {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");

    // Still in **hand**, and that is this engine's announcement rather than a
    // bug: `cast_wizard` asks every question CR 601.2b–h poses and moves the
    // card to the stack last, at CR 601.2i, so the whole announcement is
    // atomic from the outside. Nobody can tell: no player gets priority
    // until 601.2i, and CR 115.5 makes a spell an illegal target for
    // itself, so there is no legal question whose answer differs.
    assert!(
        in_hand(&engine, p0, naturalize()).is_some(),
        "the card has not reached the stack yet (CR 601.2i comes last)"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "and nothing has been destroyed yet — the removal happens on resolution"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the enchantment was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and it is in its owner's graveyard, not merely gone"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "nor did this seat's own artifact"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor the creature that was never a legal target"
    );
    assert!(
        in_graveyard(&engine, p0, naturalize()).is_some(),
        "and the instant itself went to its owner's graveyard as it resolved"
    );
}

fn nourish() -> CardIndex {
    card_index("9d5e82d3-79ac-4dbe-80e8-1db6dd6c8767")
}

/// Nourish is `{G}{G}` for one printed line: "You gain 6 life." The number is
/// the whole card, so the board is built so that six cannot be borrowed from
/// anywhere else — two Forests pay the cost and nothing else on the table can
/// move a life total — and the life is read on *both* seats, because a drain
/// and a gain are told apart by which side of the table moved. The spell is
/// read on the stack first (nothing is gained until it resolves) and in its
/// owner's graveyard afterwards, so the six life is the resolution of a spell
/// that actually was cast rather than a state somebody set.
#[test]
fn nourish_pays_two_green_for_six_life_on_its_casters_side_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[nourish()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The two Forests are the whole of `{G}{G}`; `cast_from_hand` taps them and
    // spends the pool in one step, so an empty pool afterwards is the printed
    // price really paid and not a free spell.
    cast_from_hand(&mut engine, p0, nourish());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "an instant on the stack hands priority back to its caster, got {:?}",
        engine.pending()
    );
    assert!(
        on_stack(&engine, nourish()).is_some(),
        "the spell is on the stack: an instant resolves when both seats pass"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing is gained until the spell resolves, so the life is read twice"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{G}} came out of the pool the two Forests filled"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        26,
        "\"You gain 6 life\" — six, and not two per Forest or one per mana"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the spell's controller: the opponent's total never moved"
    );
    assert!(
        in_graveyard(&engine, p0, nourish()).is_some(),
        "an instant that resolved goes to its owner's graveyard"
    );
}

fn predators_strike() -> CardIndex {
    card_index("f62e9a9f-9989-4a62-b063-cca32e21fe8b")
}

/// Predator's Strike prints one sentence: "Target creature gets +3/+3 and gains
/// trample until end of turn." Both halves are read off the creature the spell
/// *named*, while a second creature stands across the table — "target creature"
/// is neither "target creature you control" nor "creatures you control", so the
/// offer holding both is what makes the 4/4 and the printed 1/1 beside it a
/// filter rather than a board-wide buff. The land on that same menu is the other
/// word struck: `Filter::CREATURE` is read and not skipped. Two Forests pay the
/// `{1}{G}` and are read empty afterwards, so the pump is something that was
/// bought rather than a board that happened to be big.
#[test]
fn predators_strike_pumps_and_tramples_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[predators_strike()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let land = on_battlefield(&engine, p0, forest()).expect("a Forest is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "and nothing has granted it a keyword yet"
    );

    // The two Forests pay the {1}{G}, and the Elves are named as the printing
    // kept back: the creature this spell is about is the one that must not have
    // been tapped for its own mana, and two Forests are exactly the cost.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped, and no creature of mine paid in"
    );
    cast_with_floating(&mut engine, p0, predators_strike());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two creatures on the table and nothing else: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is a permanent and no creature: `Filter::CREATURE` is read, \
         not skipped: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (4, 4),
        "+3/+3 on the creature the spell targeted"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "and trample, which the layers have to project: the Elves print none"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the creature the spell did not name is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the spell reaches the creature it targeted and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{G}} came out of the two Forests the pool was read from"
    );
    assert!(
        in_graveyard(&engine, p0, predators_strike()).is_some(),
        "an instant that finished resolving goes to its owner's graveyard"
    );
}

fn preemptive_strike() -> CardIndex {
    card_index("250f8642-9754-48fd-8f09-70ed13d7a42c")
}

/// Preemptive Strike — {1}{U} instant: "Counter target creature spell."
///
/// The scenario is the one the card exists for: an opponent casts a creature,
/// so the spell sits on the stack with no creature anywhere near the
/// battlefield. The counter is read in the *zone* the card lands in — a
/// countered spell goes to its owner's graveyard (CR 701.6a) and never enters,
/// so one resolution has to show both: the Elves in p1's graveyard and no Elf
/// under p1's control. The target question is the other half: the spell on the
/// menu is the object that was just cast, and the {1}{U} leaves the pool only
/// after that answer (CR 601.2c before CR 601.2h), which is where the payment
/// is checked.
#[test]
fn preemptive_strike_counters_a_creature_spell_that_is_still_on_the_stack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[preemptive_strike()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // p1's turn, because the creature spell this instant answers has to be
    // theirs: the seat holding the counter is not the active one.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "one Forest pays the Elves' {{G}}"
    );
    cast_with_floating(&mut engine, p1, llanowar_elves());

    // Back to p0 with the creature spell waiting and nothing resolved yet.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let elves = on_stack(&engine, llanowar_elves())
        .expect("the Elves are a spell on the stack, not a permanent");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "a spell that has not resolved is no creature on the battlefield"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, one more blue than {{1}}{{U}} asks for"
    );
    cast_with_floating(&mut engine, p0, preemptive_strike());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat casting the instant is the one aiming it"
    );
    assert!(
        options.contains(&elves),
        "the creature spell just cast is the spell on the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the spell the question offered is the one being countered");

    assert!(
        on_stack(&engine, preemptive_strike()).is_some(),
        "the instant is on the stack beside the spell it answered"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{U}} is the last step of the cast (CR 601.2h), leaving one \
         of the three Islands' blue behind"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a countered spell is put into its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and it never resolved, so the battlefield never sees it"
    );
    assert!(
        in_graveyard(&engine, p0, preemptive_strike()).is_some(),
        "the instant itself resolved and is in its owner's graveyard"
    );
}

fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

/// Raise the Alarm prints one line — "Create two 1/1 white Soldier creature
/// tokens" — and "two" is the half a single-token reading would lose. The
/// board is two Plains and nothing else, so the {1}{W} is a real payment out
/// of a pool the lands actually filled, and the count is read off the
/// battlefield after the spell has resolved rather than off the stack: one
/// token and two tokens are the same card until the effect has finished.
#[test]
fn raise_the_alarm_makes_two_white_soldier_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[raise_the_alarm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing is on the board before the spell is cast"
    );

    cast_from_hand(&mut engine, p0, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        2,
        "one printed line, two Soldiers: {tokens:?}"
    );
    for token in tokens {
        let def = engine
            .state()
            .object(token)
            .expect("the token is an object")
            .token
            .expect("the token knows which token it is");
        assert_eq!(def.name, "Soldier", "the token the card names");
        assert_eq!(
            (def.power, def.toughness),
            (Some(1), Some(1)),
            "a 1/1, and not the two Bodies a token with no numbers would have"
        );
        assert!(
            def.colors.contains(baylee_core::color::Color::White),
            "a *white* Soldier"
        );
        assert!(
            types(&engine, token).contains(TypeSet::CREATURE),
            "and a creature, not merely a permanent"
        );
        assert_eq!(
            engine
                .state()
                .object(token)
                .expect("the token is still there")
                .controller,
            p0,
            "under the control of the seat that cast the spell"
        );
    }
    assert!(
        in_graveyard(&engine, p0, raise_the_alarm()).is_some(),
        "an instant that has resolved goes to its owner's graveyard"
    );
}

// oracle_id = "b13c0f76-fbda-4911-9442-c3d7e97f1aac"
fn remove_soul() -> CardIndex {
    card_index("b13c0f76-fbda-4911-9442-c3d7e97f1aac")
}

/// Remove Soul — {1}{U} instant: "Counter target creature spell."
///
/// The whole card is one question about a zone no other test in this file
/// reads from: the target is a *spell*, so the scenario has to stop while the
/// Elf is a stack object and in neither of the two zones a creature card
/// lives in, and the offer that names it can only have come off the stack. The
/// counter is then read as a move rather than as a question — a countered
/// creature spell was never a permanent, so the Elf can only end up in its
/// owner's graveyard — and the two Islands that paid for the counterspell are
/// spent with it.
#[test]
fn remove_soul_counters_the_creature_spell_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(307, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[remove_soul()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // p1 puts a creature *spell* on the stack — the only thing Remove Soul can
    // name — and the Elf is nowhere a permanent yet.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, llanowar_elves());
    let elf_spell =
        on_stack(&engine, llanowar_elves()).expect("the Elves are a spell on the stack");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none()
            && in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "an unresolved creature spell is in neither of the two zones that hold \
         a creature card"
    );

    // p0's window: the creature spell is still waiting and p0 holds priority.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana into the pool before anything is claimed about the offer: the
    // engine reads `castable` off the pool and not off untapped lands.
    let soul = in_hand(&engine, p0, remove_soul()).expect("the counterspell is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands pay {{1}}{{U}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&soul),
        "with the mana floating and a creature spell on the stack, the card is \
         castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, remove_soul());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!(
        options,
        vec![elf_spell],
        "the creature spell on the stack is the whole menu: this card counters \
         a spell, so nothing that is not on the stack can be named"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elf_spell],
            },
        )
        .expect("the creature spell the offer named is a legal target");

    assert!(
        on_stack(&engine, remove_soul()).is_some() && on_stack(&engine, llanowar_elves()).is_some(),
        "choosing the target does not counter anything: the counterspell is on \
         the stack above the spell it named, and both are still there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, llanowar_elves()).is_none(),
        "the creature spell left the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and it never resolved, so it never became a permanent"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p0, remove_soul()).is_some(),
        "the counterspell resolved and was put into its caster's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{U}} came out of the pool"
    );
}

fn turn_to_dust() -> CardIndex {
    card_index("56828166-eaa3-4711-91b6-401a3e3b733f")
}

/// Turn to Dust — {G} instant: "Destroy target Equipment. Add {G}."
///
/// Two Equipments stand on the table, one under each seat, and one tapped
/// Forest is the whole board's mana: the offer naming exactly those two
/// artifacts is the printed word "Equipment" being read — it reaches across
/// the table (CR 115.1) and leaves the land beside it alone — and answering
/// with the one across the table is what says so. The single Forest is also
/// what makes "Add {G}" legible: it is spent paying for the spell, so once
/// the target question closes the pool is empty and the green standing in it
/// after resolution has no source on this board but the spell itself.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn turn_to_dust_destroys_the_equipment_it_names_and_adds_a_green_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), basilisk_collar()])
        .hand(0, &[turn_to_dust()])
        // An Equipment on the other side of the table: "target Equipment" is
        // not "an Equipment you control".
        .battlefield(1, &[lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, basilisk_collar()).expect("my Equipment is out");
    let theirs = on_battlefield(&engine, p1, lightning_greaves()).expect("their Equipment is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        1,
        "the Forest is the only mana source on the board"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one Forest, one green"
    );

    // Mana before the claim: `castable` is filtered through `can_afford`,
    // which reads the pool and not the untapped land.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, turn_to_dust()).expect("the instant is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{G}} is in the pool, so the instant is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, turn_to_dust());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Equipment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one Equipment");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "an Equipment on either side of the table is a legal target: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is no Equipment, so the filter is read and not skipped: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2c before CR 601.2h: the target is named while the {{G}} is still floating"
    );
    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_some(),
        "and nothing has been destroyed while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Equipment across the table was one of the options");

    // CR 601.2h: the cost is the last step, so the {G} is gone the moment
    // the target question closes — and the spell is on the stack, not done.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} is paid on announcement, after the target"
    );
    assert!(
        !stack_is_empty(&engine),
        "Turn to Dust is no mana ability, so it is waiting to resolve"
    );
    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_some(),
        "and the targeted Equipment is still on the table"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_none(),
        "the targeted Equipment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, lightning_greaves()).is_some(),
        "a destroyed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, basilisk_collar()).is_some(),
        "and only the Equipment that was named: mine still stands"
    );
    assert!(
        in_graveyard(&engine, p0, turn_to_dust()).is_some(),
        "the instant itself went to its owner's graveyard"
    );
    assert!(
        is_tapped(&engine, land),
        "the Forest paid the {{G}}, so it is tapped and can produce nothing more"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"Add {{G}}\" — one green in the pool, and the board's only land is spent"
    );
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
}

fn unnatural_speed() -> CardIndex {
    card_index("fa46ced5-b701-4aa3-b4a1-36fb15b80696")
}

/// Unnatural Speed is one red instant — Arcane — reading "Target creature
/// gains haste until end of turn", and the keyword alone proves nothing: haste
/// exists to beat summoning sickness (CR 302.6), so the only board that reads
/// the card is one where a creature *arrived this turn*. Two identical Elves
/// are cast in the same first main phase and only one of them is aimed at, so
/// the pair has to part company in the attack declaration — the hasted Elf is
/// offered although it just entered, and the one beside it is not. The Elf
/// across the table is the control for the second half of the target line:
/// "target creature" is not "target creature you control".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn unnatural_speed_lets_a_creature_that_arrived_this_turn_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(197, forest())
        .battlefield(0, &[mountain(), mountain(), forest(), forest()])
        .hand(0, &[quiet_creature(), quiet_creature(), unnatural_speed()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // Four lands and no creature of p0's yet, so the pool is exactly these
    // four and every number below is one of them being spent.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Mountains and two Forests, and nothing else on this board makes mana"
    );

    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    let fast = on_battlefield(&engine, p0, quiet_creature()).expect("the first Elf resolved");
    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);

    let pair = all_on_battlefield(&engine, p0, quiet_creature());
    assert_eq!(
        pair.len(),
        2,
        "two Elves, and both arrived in this same turn"
    );
    let slow = pair
        .iter()
        .copied()
        .find(|id| *id != fast)
        .expect("the second Elf is on the table too");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, fast).contains(KeywordSet::HASTE),
        "nothing has been aimed at yet"
    );

    cast_with_floating(&mut engine, p0, unnatural_speed());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert!(
        options.contains(&fast) && options.contains(&slow),
        "both Elves of mine are creatures: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"target creature you control\": {options:?}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "the two Elves spent the green, so the {{R}} is still in the pool while \
         the target question stands (CR 601.2c before CR 601.2h)"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fast],
            },
        )
        .expect("the Elf the question offered is the one that was aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, unnatural_speed()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
    assert!(
        keywords(&engine, fast).contains(KeywordSet::HASTE),
        "the Elf the spell named has haste until end of turn"
    );
    assert!(
        !keywords(&engine, slow).contains(KeywordSet::HASTE),
        "and the identical Elf beside it does not: the pump reaches one target \
         and no other"
    );

    // The functional half. Haste is only worth anything against summoning
    // sickness, and this is where the two Elves have to disagree.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&fast),
        "an untapped creature that arrived this turn may attack because the \
         spell gave it haste: {attackers:?}"
    );
    assert!(
        !attackers.contains(&slow),
        "while the Elf that arrived on the same turn and was not aimed at is \
         still sick (CR 302.6): {attackers:?}"
    );
}

fn unsummon() -> CardIndex {
    card_index("837182db-1bf3-4a2c-bd01-1af9d9873561")
}

/// Unsummon — {U} instant: "Return target creature to its owner's hand."
///
/// Two printed words carry the card, and each gets its own witness. "target
/// creature" is read off what the engine offers: three creatures stand on the
/// table — mine and both of the opponent's — and a filter that had narrowed to
/// one side, or to something that is no creature, could not list three. "its
/// owner's hand" is read by aiming it at the *opponent's* Elf, so the card has
/// to arrive in p1's hand and nowhere else, which a test that only watched the
/// creature leave the battlefield could never tell.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn unsummon_returns_the_creature_it_names_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(917, island())
        .battlefield(0, &[island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .hand(0, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves stand across the table");
    let (target, bystander) = (theirs[0], theirs[1]);

    // `legal.castable` is filtered through `can_afford`, which reads the mana
    // *pool* and not the untapped lands, so the mana is made first. The Island
    // taps for {U} and the Elf beside it prints its own `{T}: Add {G}`, which
    // is a route whose whole price is its own tap (#159) — so two routes is
    // every source on this board and the pool has to say two.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 2,
        "the Island and the Elf, and nothing else makes mana"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue, which is exactly what {{U}} costs"
    );
    assert_eq!(
        pool.total(),
        2,
        "and the Elves' own {{G}} is beside it, so both sources are counted"
    );

    cast_with_floating(&mut engine, p0, unsummon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast the spell aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        3,
        "\"target creature\" is every creature in the game: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&target) && options.contains(&bystander),
        "both sides of the table are on the menu: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "a creature is no player, so there is no face to choose: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the Elf the question enumerated is a legal answer");

    // CR 601.2c chose the target and CR 601.2h paid afterwards, so the mana is
    // gone now and the spell is what is waiting.
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the {{U}} came out of the pool to pay for the spell"
    );
    assert_eq!(
        pool.total(),
        1,
        "and the green the Elves made is all that is left of the two"
    );
    assert!(
        !stack_is_empty(&engine),
        "returning a creature is no mana ability, so the spell waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p1, llanowar_elves()),
        vec![bystander],
        "only the creature that was named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf is in p1's hand, the seat that owns it"
    );
    assert_eq!(
        engine
            .state()
            .object(target)
            .expect("the bounced Elf is still an object")
            .zone,
        crate::zone::Zone::Hand,
        "and the very card that was aimed at is the one that moved to a hand"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and never to the hand of the seat that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature nobody named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, unsummon()).is_some(),
        "the spell itself went to its owner's graveyard once it resolved"
    );
}

fn terminate() -> CardIndex {
    card_index("6257c2fd-005f-41e3-8a72-af76df1eb134")
}

/// Terminate: "Destroy target creature. It can't be regenerated."
///
/// The second sentence is the half that used to cost nothing. While this
/// engine had no regeneration at all, `destroy` and `destroy_no_regen` were
/// the same function and every card printing the clause was correct for
/// free — so a shield standing on the target when the spell resolves is the
/// only thing that has ever told the two apart (CR 701.19c).
///
/// Its own controller's Troll, because a shield has to be *bought* to mean
/// anything and the Troll's `{B}` is the cheapest one there is. A
/// Terminate wired to the ordinary door would leave the Troll standing
/// here, tapped and unhurt.
#[test]
fn terminate_kills_through_a_regeneration_shield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(511, forest())
        .battlefield(0, &[lotleth_troll(), swamp(), swamp(), mountain()])
        .hand(0, &[terminate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);

    cast_with_floating(&mut engine, p0, terminate());
    let menu = aim_at(&mut engine, p0, troll);
    assert!(
        menu.contains(&troll),
        "a shielded creature is a legal target like any other: {menu:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, lotleth_troll()).is_none(),
        "the shield did not save it"
    );
    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "and it is in its owner's graveyard"
    );
}

fn fissure() -> CardIndex {
    card_index("c8b1e9f3-b014-4e57-b278-6d84a7e88b23")
}

/// Fissure: "Destroy target creature or land. It can't be regenerated."
///
/// The same clause on a wider filter, and the filter is the half worth
/// asserting beside it: the menu holds the Mountains as well as the Troll,
/// which is what "creature **or land**" means and what a copy of Terminate
/// would not do.
#[test]
fn fissure_reaches_a_land_as_well_and_still_ignores_a_shield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(512, forest())
        .battlefield(
            0,
            &[
                lotleth_troll(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[fissure()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is seated");
    let a_mountain = on_battlefield(&engine, p0, mountain()).expect("a Mountain is seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);

    cast_with_floating(&mut engine, p0, fissure());
    let menu = aim_at(&mut engine, p0, troll);
    assert!(menu.contains(&troll), "the creature half of the filter");
    assert!(
        menu.contains(&a_mountain),
        "and the land half, which is the whole difference from Terminate: {menu:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "the shield did not save it"
    );
}

/// Inspiration — {3}{U} Instant: "Target player draws two cards."
///
/// "Target player" is the whole card, so the target named here is the seat
/// that did *not* cast it: p1's hand grows by two and p1's library shrinks by
/// two, while p0 — who paid the {3}{U} — draws nothing and only loses the card
/// itself to its own graveyard. A printing that read "you draw two" would
/// satisfy every count taken on p0's side of the table and fail both of the
/// ones taken here, and a printing that read "target creature's controller"
/// would have no creature on this board to name. The four Islands are tapped
/// before the cast, because the offer reads the pool and not the untapped
/// lands, and they are the only mana on the board.
#[test]
fn inspiration_draws_two_cards_for_the_player_it_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[inspiration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_library = library_size(&engine, p0);
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let their_library = library_size(&engine, p1);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, inspiration());

    // CR 601.2c before CR 601.2h: the cost is the last step of the cast, so
    // the four blue are still floating while the target question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands tapped for four blue, and the {{3}}{{U}} is not paid yet"
    );

    // A player-only target is a target question whose menu is the seats of
    // the game. Whichever of the two forms the engine publishes it under, the
    // answer is taken out of that question's own enumeration.
    let offered = match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the seat that cast it names the target");
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the other seat was one of the options");
            player_options
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the seat that cast it names the target");
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the other seat was one of the options");
            options
        }
        other => panic!("expected a target choice, got {other:?}"),
    };
    assert!(
        offered.contains(&p0) && offered.contains(&p1),
        "\"target player\" is either seat of the game: {offered:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 2,
        "\"target player draws two cards\": two off the targeted seat's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand + 2,
        "and both are in that seat's hand, so an emptied library would not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "the caster draws nothing — the cards belong to the player it named"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand - 1,
        "p0's hand is one smaller: the Inspiration itself left it and no draw replaced it"
    );
    assert!(
        in_graveyard(&engine, p0, inspiration()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}}{{U}} was paid rather than merely printed"
    );
}

/// Lightning Blast — {3}{R} instant: "Lightning Blast deals 4 damage to any
/// target."
///
/// Two casts off one pool of eight Mountains read the whole card inside a
/// single main phase. The first is aimed at the opponent, so the life they are
/// missing afterwards is the printed number *exactly* — sixteen, not the
/// seventeen a three-damage spell would leave nor the fifteen a five-damage
/// one — and the menu it was chosen from carries a creature and both players
/// in one choice, which is what CR 115.4's "any target" means. The second cast
/// is aimed at that creature and the same four damage kills it, so the object
/// half of the target spec is played and not merely offered; the pool read in
/// between says a {3}{R} really costs four of the eight.
#[test]
#[allow(clippy::too_many_lines)]
fn lightning_blast_deals_four_to_a_player_and_to_a_creature_off_the_mana_it_charges() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 8])
        .hand(0, &[lightning_blast(), lightning_blast()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is across the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1 for four damage to kill"
    );

    // Eight Mountains are the whole board, and whether a `{3}{R}` is castable
    // is read off the pool rather than off the untapped lands — so the mana
    // goes in before anything is cast.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Mountains tapped for eight red"
    );

    // The first cast, at the player. `any target` is a question with two lists,
    // and both of them are read before it is answered.
    cast_with_floating(&mut engine, p0, lightning_blast());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "\"any target\" reaches a creature: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4 counts players in the same choice: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target for `any target`");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "\"deals 4 damage\": sixteen, and not the seventeen a three-damage \
         spell would leave nor the fifteen a five-damage one"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage went to the seat that was named and not to the caster"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the first {{3}}{{R}} came out of the eight, and the pool did not \
         empty between the two casts (CR 500.5)"
    );

    // The second cast, at the creature: the other half of `AnyTarget`, read on
    // a body instead of on a life total.
    cast_with_floating(&mut engine, p0, lightning_blast());
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
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "the creature across the table is on the menu: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "four damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        mine(&engine, p0, lightning_blast(), Zone::Graveyard).len(),
        2,
        "both spells resolved: one copy in the graveyard would mean one of them \
         never left the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "eight Mountains paid two {{3}}{{R}} and not one mana is left over"
    );
}

#[test]
fn searing_wind_deals_ten_damage_to_the_face_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 9])
        .hand(0, &[searing_wind()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // {8}{R} is nine mana, and the offer is read off the pool: nine tapped
    // Mountains are what makes the spell castable at all.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "nine Mountains, nine red — exactly the printed cost"
    );
    cast_with_floating(&mut engine, p0, searing_wind());

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the face was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        10,
        "ten damage at once — one point would leave it at nineteen, and only \
         the printed ten can leave it at ten"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that cast it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "and took nothing on the way past");
    assert!(
        in_graveyard(&engine, p0, searing_wind()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
}

// oracle_id = "fa9b6be2-b88c-4302-b7e2-faf25a60bcb9"

/// Altar's Light — {2}{W}{W} instant: "Exile target artifact or
/// enchantment."
///
/// The whole card is one sentence with two nouns in it, so both halves are
/// played off one board that carries a witness for each word: an artifact and
/// an enchantment across the table are both offered, a creature is not, and
/// the two casts leave the artifact and then the enchantment sitting in their
/// owner's exile while the Elf never moves. Exile rather than graveyard is the
/// reading that tells the printed verb from a destroy, and the empty pool
/// before the first cast is what makes the missing `castable` a claim about
/// the {2}{W}{W} instead of about an absent target.
#[test]
fn altars_light_exiles_an_artifact_and_an_enchantment_and_declines_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 8])
        .hand(0, &[altars_light(), altars_light()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let chant = on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Nothing floats, and both nouns the card prints are already standing on
    // the other side of the table — so an uncastable spell here is the cost
    // and not "there is nothing for it to point at".
    let spell = in_hand(&engine, p0, altars_light()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&spell),
        "an empty pool pays no {{2}}{{W}}{{W}}, though a legal artifact and a \
         legal enchantment both stand across the table: {:?}",
        legal.castable
    );

    // Four of the eight Plains pay the first cast; the rest stay in the pool
    // for the second, because a pool survives inside one main phase (CR 500.5).
    cast_from_hand(&mut engine, p0, altars_light());
    let options = aim_at(&mut engine, p0, ring);
    assert!(
        options.contains(&ring) && options.contains(&chant),
        "\"target artifact or enchantment\" reaches either noun, on either \
         side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a creature and neither noun the card prints: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact the spell named left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&ring),
        "\"exile\": the card is in its owner's exile"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_none(),
        "and not in a graveyard, which is the word the card does not print"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "one target, one card: the enchantment nobody named is untouched"
    );

    // The other half of the disjunction, read off the board the first cast
    // left behind: the exiled artifact is gone from the menu and the
    // enchantment is still on it.
    cast_from_hand(&mut engine, p0, altars_light());
    let options = aim_at(&mut engine, p0, chant);
    assert!(
        options.contains(&chant) && !options.contains(&ring),
        "the enchantment is the target still standing, and the exiled artifact \
         is no longer one: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "and a creature is still neither noun: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the enchantment left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&chant),
        "and the second noun is exiled the same way the first was"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_none(),
        "never into a graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell could not name is still standing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two casts of {{2}}{{W}}{{W}} out of eight Plains spend the pool exactly"
    );
}

/// Might of Oaks prints one sentence — "Target creature gets +7/+7 until end
/// of turn" — and every word of it needs a witness: "target" is a single
/// choice out of a menu that reaches both sides of the table, "creature" is
/// what tells the pump from a board-wide grant, and "until end of turn" is a
/// duration nothing on the battlefield carries. The Elf across the table and
/// the turn walked afterwards are those witnesses: both printed numbers land
/// on the creature that was named and on no other, and they are gone again by
/// the next main phase while the creature is still standing.
#[test]
fn might_of_oaks_pumps_the_creature_it_names_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[might_of_oaks()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the pump");

    // Five Forests tapped and the Elves named as the printing kept back: the
    // creature this test targets is one of them, and its own `{T}` would have
    // been spent for mana the {3}{G} does not need.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests tapped, five green, and neither Elf paid in"
    );
    cast_with_floating(&mut engine, p0, might_of_oaks());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (8, 8),
        "+7/+7 on the creature the spell named, and on nothing else"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf the spell did not name never moved — \"target creature\" is \
         not \"creatures\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{3}}{{G}} came out of the five green: one mana is the change left"
    );

    // "until end of turn": the pump is a duration and not a static, so the
    // same Elf reads its printed body a turn later.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the numbers left rather than \
         the creature"
    );
}

/// Opportunity is `{4}{U}{U}` for a single printed sentence: "Target player
/// draws four cards." That sentence is `PlayerRel::Chosen`, so the four cards
/// belong to the seat the spell *named* and not to the seat that cast it, and
/// the only way to say so is to name the opponent and read both sides: the
/// targeted library is four shorter and that hand four longer, while the
/// caster's library never moves. The target menu is the other half of the
/// claim — "target player" reaches either chair, so a spell quietly narrowed to
/// its own controller would still resolve and still draw four.
#[test]
fn opportunity_draws_four_for_the_player_it_targets_and_nothing_for_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 6])
        .hand(0, &[opportunity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Islands pay the {4}{U}{U}. Every count is taken before the cast, so
    // that the cards below are read as a move and not as a board.
    let mine = library_size(&engine, p0);
    let theirs = library_size(&engine, p1);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, opportunity());

    // The target is named at CR 601.2c, while the spell is still on the stack.
    // A player-only target is published either as the player half of a target
    // choice or as a bare player choice, so both questions are read.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets { .. } | Pending::ChoosePlayer { .. }
        )
    });
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            min,
            max,
            ..
        } => {
            assert_eq!(player, p0, "the seat that cast the spell names the target");
            assert_eq!((min, max), (1, 1), "one player, and the spell asks once");
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" is any player at the table, the caster \
                 included: {player_options:?}"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the opponent was one of the players the question offered");
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the seat that cast the spell names the target");
            assert!(
                options.contains(&p0) && options.contains(&p1),
                "\"target player\" is any player at the table, the caster \
                 included: {options:?}"
            );
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the opponent was one of the players the question offered");
        }
        other => panic!("Opportunity never asked for a target player: {other:?}"),
    }

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        theirs - 4,
        "\"target player draws four cards\" — four off the top of the targeted \
         seat's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand + 4,
        "and the four are in that player's hand, so a library that merely \
         emptied would not satisfy the count above"
    );
    assert_eq!(
        library_size(&engine, p0),
        mine,
        "`PlayerRel::Chosen`: the caster's library never moved — the four cards \
         belong to the player the spell named and not to the one who paid for it"
    );
    assert!(
        in_graveyard(&engine, p0, opportunity()).is_some(),
        "the resolved instant is in its owner's graveyard"
    );
}

// oracle_id = "2fa763ad-4d94-4c3a-a099-13ac22c09ce4"

/// Regress — {2}{U} instant: "Return target permanent to its owner's hand."
///
/// Two printed words are the whole card and each needs its own witness. The
/// target says *permanent* and not creature, so lands stand on the menu beside
/// the creatures; and the destination says *owner's* hand, so the spell is
/// aimed across the table and the Elf has to come back to the seat that owns
/// it rather than to the seat that cast the spell. My own Elf is the control
/// that the return is the named target and not a sweep of the board, and the
/// three Islands are read as an exactly emptied pool so the {2}{U} is a
/// payment rather than a label.
#[test]
fn regress_returns_the_permanent_it_names_to_its_owners_hand_and_not_to_the_casters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[regress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let my_land = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // Three Islands are the whole of {2}{U}, and the Elf is named as the
    // printing kept back: it is a creature this test reads off the board
    // afterwards, and a mana creature tapped for the spell would be a
    // different board than the one the assertions below describe.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands in the pool, and the Elf contributed nothing"
    );

    cast_with_floating(&mut engine, p0, regress());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert_eq!(player, p0, "the seat that cast the spell names the target");
    assert!(
        options.contains(&my_elf) && options.contains(&their_elf),
        "\"target permanent\" is any permanent, on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&my_land) && options.contains(&their_land),
        "\"target permanent\" is not \"target creature\": lands are on the \
         menu too: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while the Elf it \
         names is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("the creature across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and not to the caster's hand, which is the reading the printed word \
         forbids — the two differ only by which side of the table the card \
         lands on"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}} came out of the three Islands: one spell, and nothing \
         left floating"
    );
}

/// Volcanic Geyser — {X}{R}{R} instant: "Volcanic Geyser deals X damage to any
/// target."
///
/// X is the whole card, so the amount has to be read off a number only one
/// value produces: five Mountains pay {3}{R}{R} exactly, and the opponent's
/// twenty life becomes seventeen — not the five a card reading the mana it ate
/// would deal, and not the zero a card that never asked for X would. The other
/// printed word is "any target" (CR 115.4), so the question is read while it
/// stands: one choice carrying the Elf across the table and both seats, aimed at
/// the player, with the Elf still standing afterwards.
#[test]
#[allow(clippy::too_many_lines)]
fn volcanic_geyser_deals_three_damage_for_x_three_to_the_seat_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        // A creature across the table, so "any target" has an object to offer
        // beside the two seats and the damage has somewhere to *not* go.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[volcanic_geyser()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains, five red — the board has no other source on it"
    );

    // `can_afford` reads the pool and not the untapped lands, so the claim
    // about the offer is made with the mana already floating.
    let card = in_hand(&engine, p0, volcanic_geyser()).expect("the Geyser is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "five red pays {{3}}{{R}}{{R}}: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, volcanic_geyser());

    // X at CR 601.2b, the target at CR 601.2c, the mana at CR 601.2h. The
    // questions are answered in the order they arrive rather than in the order
    // they are expected.
    let mut asked_for_x = false;
    let mut aimed = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut seats: Vec<PlayerId> = Vec::new();
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::Priority { .. } => break,
            Pending::ChooseNumber { player, max, .. } => {
                assert_eq!(player, p0, "the caster of the spell names X");
                assert!(
                    max >= 3,
                    "the cost is {{X}}{{R}}{{R}} and five red is floating, so \
                     three has to be on the menu: max is {max}"
                );
                engine
                    .apply(p0, PlayerAction::ChooseNumber(3))
                    .expect("three came out of the range the question published");
                asked_for_x = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the caster aims it");
                assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
                assert_eq!(
                    options,
                    vec![elf],
                    "the only object \"any target\" names here is the creature \
                     across the table — the Mountains are lands: {options:?}"
                );
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "CR 115.4: \"any target\" counts players in the same choice: \
                     {player_options:?}"
                );
                assert_eq!(
                    engine.state().players[0].mana_pool.total(),
                    5,
                    "CR 601.2c before CR 601.2h: the cost is the last step of \
                     the cast, so nothing is spent while the question stands"
                );
                menu = options;
                seats = player_options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("the seat was one of the targets it enumerated");
                aimed = true;
            }
            other => panic!("unexpected while the Geyser is cast: {other:?}"),
        }
    }
    assert!(
        asked_for_x,
        "X is a choice: a spell that skipped it would deal a fixed amount and \
         leave the pool untouched"
    );
    assert!(aimed, "\"any target\" is a target choice");
    assert_eq!(menu, vec![elf], "the menu the target question published");
    assert!(seats.contains(&p1), "and it carried both seats");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}} and {{R}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the Geyser is waiting on the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing has happened while it is still there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "\"deals X damage\" with X = 3 — five would mean the mana spent was \
         read instead of the number chosen"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage belongs to the target, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved, so the three points \
         went to the target and not to the board"
    );
    assert!(
        in_graveyard(&engine, p0, volcanic_geyser()).is_some(),
        "and the instant resolved into its owner's graveyard"
    );
}

/// Zap — {2}{R} instant: "Zap deals 1 damage to any target. Draw a card."
/// Both printed sentences are read in one cast, and the target question is
/// where CR 115.4 is checked: `options` carries the Elf across the table and
/// `player_options` both seats, because "any target" is one choice over
/// objects and players together. The damage is read on a life total rather
/// than on a body, since 20 to 19 is exactly the printed one where a dead 1/1
/// would only say "at least one", and the draw is read off the library and the
/// hand together so an emptied library could not stand in for a card.
#[test]
#[allow(clippy::too_many_lines)]
fn zap_deals_one_damage_to_the_target_it_names_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[zap()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `legal.castable` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands, so the claim is made before anything is
    // tapped: {{2}}{{R}} is not payable on an empty pool.
    let card = in_hand(&engine, p0, zap()).expect("Zap is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{R}}, so the instant is not offered: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains and nothing else on this board makes mana"
    );
    cast_with_floating(&mut engine, p0, zap());

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "nothing has happened yet: the target is chosen before anything resolves"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");

    assert!(
        !stack_is_empty(&engine),
        "an instant is a spell and waits on the stack rather than resolving on announcement"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage is the resolution, not the cost"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" — exactly one, read on the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that paid for it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        in_graveyard(&engine, p0, zap()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved, so the one point went where it was aimed"
    );
}

/// Mystic Denial is a {1}{U}{U} instant under `Coverage::Implemented` that counters target creature or sorcery spell.
/// When an opponent casts a creature spell, Mystic Denial targets that spell while it is on the stack.
/// Resolving Mystic Denial counters the spell, sending the creature card directly to its owner's graveyard.
/// The creature never enters the battlefield.
#[test]
fn mystic_denial_counters_a_creature_spell_on_the_stack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[mystic_denial()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());

    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "caster holds priority");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    let Pending::Priority {
        player: responding, ..
    } = engine.pending().clone()
    else {
        panic!("expected opponent priority, got {:?}", engine.pending())
    };
    assert_eq!(responding, p1, "priority passes to opponent");

    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "three Islands produce three blue mana"
    );
    cast_with_floating(&mut engine, p1, mystic_denial());

    let Pending::ChooseTargets {
        player: targeting_player,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        targeting_player, p1,
        "opponent selects target for Mystic Denial"
    );

    let elf_spell = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elf_spell],
            },
        )
        .expect("targeting creature spell on stack is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "countered creature never reached the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "countered creature is in owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, mystic_denial()).is_some(),
        "Mystic Denial went to graveyard upon resolution"
    );
}

/// Pull Under is a {5}{B} instant under `Coverage::Implemented` that gives target creature -5/-5 until end of turn.
/// When cast, creatures on the battlefield are presented as legal targets while non-creatures are excluded.
/// Targeting a 6/6 creature reduces its projected power and toughness down to 1/1 without destroying it.
/// Bystanders not selected by the spell remain unaffected.
#[test]
fn pull_under_gives_target_creature_minus_five_minus_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[pull_under()])
        .battlefield(1, &[rootbreaker_wurm(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, pull_under()).expect("Pull Under is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{5}}{{B}}"
    );

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("their Wurm is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(pt(&engine, wurm), (6, 6), "Wurm starts as 6/6");
    assert_eq!(pt(&engine, elf), (1, 1), "Elf starts as 1/1");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Swamps produce six mana"
    );
    cast_with_floating(&mut engine, p0, pull_under());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "caster chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&wurm) && options.contains(&elf),
        "both creatures are valid targets: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("targeting Wurm is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (1, 1),
        "Wurm received -5/-5, becoming a 1/1"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "Elf was not targeted and remains a 1/1"
    );
    assert!(
        in_graveyard(&engine, p0, pull_under()).is_some(),
        "Pull Under went to graveyard upon resolution"
    );
}

/// Smash is a {2}{R} instant under `Coverage::Implemented` that destroys target artifact and draws a card.
/// When cast targeting an artifact controlled across the table, the target is destroyed and sent to the graveyard.
/// The resolving spell draws a card for its controller, verified by hand and library size changes.
/// An artifact is required as a target, leaving creatures excluded from the offered options.
#[test]
fn smash_destroys_target_artifact_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[smash()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, smash()).expect("Smash is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{R}}, so the instant is not offered: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains produce three mana"
    );
    cast_with_floating(&mut engine, p0, smash());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(player, p0, "the casting seat selects the target");
    assert_eq!((min, max), (1, 1), "one target artifact is required");
    assert!(
        options.contains(&rock),
        "the artifact is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the creature is not an artifact: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("target artifact selected");

    // Counted once the target is chosen and Smash is on the stack: while the
    // question stood, the spell being cast was still counted in the hand.
    let lib_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the targeted artifact is destroyed and in graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact is no longer on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the bystander creature is untouched"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "one card was drawn from library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the drawn card reached the hand"
    );
    assert!(
        in_graveyard(&engine, p0, smash()).is_some(),
        "the instant resolved and went to graveyard"
    );
}

/// Verdigris is a {2}{G} instant under `Coverage::Implemented` that destroys target artifact.
/// When cast, artifacts are offered as legal targets while non-artifact creatures are excluded.
/// Upon resolution, the targeted artifact is destroyed and placed into its owner's graveyard.
/// Other permanents on the battlefield remain unaffected.
#[test]
fn verdigris_destroys_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[verdigris()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, verdigris()).expect("Verdigris is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{2}}{{G}}"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests produce three mana"
    );
    cast_with_floating(&mut engine, p0, verdigris());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(player, p0, "the casting player chooses the target");
    assert_eq!((min, max), (1, 1), "one target artifact is required");
    assert!(
        options.contains(&rock),
        "artifact is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "creature is not an artifact: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("target artifact selected");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "targeted artifact is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "targeted artifact is no longer on battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "bystander creature remains on battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, verdigris()).is_some(),
        "Verdigris is in graveyard"
    );
}

/// Absorb — {W}{U}{U} instant: "Counter target spell. You gain 3 life."
///
/// Both printed sentences are read in one exchange, because the counterspell
/// is only itself if the spell it names actually leaves the stack *and* the
/// caster's life total moves. The Elf is the spell being countered: the target
/// question must offer it before any cost is paid (CR 601.2c before
/// CR 601.2h), so the three mana is still floating while that menu stands, and
/// afterwards the card lies in its owner's graveyard, the battlefield is empty,
/// and the three life belongs to the seat that cast the Absorb rather than to
/// the seat whose spell died.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn absorb_counters_the_spell_it_names_and_gains_three_life_for_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[quiet_creature()])
        .battlefield(1, &[plains(), island(), island()])
        .hand(1, &[absorb()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, quiet_creature());
    let elf = on_stack(&engine, quiet_creature()).expect("the Elf is a spell on the stack");
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "a creature spell is not a permanent until it resolves"
    );

    // p0 passes, and p1 answers with priority while that spell is still on the
    // stack — exactly the window an instant is for.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && !stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 20, "nothing gained yet");

    // `LegalActions` is filtered through `can_afford`, which reads the pool and
    // not the untapped lands, so an empty pool is the only difference between
    // the offer below and the one after the tap.
    let spell = in_hand(&engine, p1, absorb()).expect("the Absorb is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the responding seat holds priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.castable.contains(&spell),
        "{{W}}{{U}}{{U}} is not three untapped lands: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p1);
    let pool = &engine.state().players[1].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "the Plains");
    assert_eq!(pool.available(ManaColor::Blue), 2, "and the two Islands");
    cast_with_floating(&mut engine, p1, absorb());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast the Absorb names the target");
    assert_eq!((min, max), (1, 1), "one spell, and the spell asks once");
    assert!(
        options.contains(&elf),
        "the spell waiting on the stack is the whole menu: {options:?}"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "CR 601.2c before CR 601.2h: the target is named while the mana that \
         pays for it is still floating"
    );

    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the spell the menu offered is the one it counters");
    assert!(
        !stack_is_empty(&engine),
        "and the Absorb itself is what is waiting there now"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, quiet_creature()).is_none(),
        "the countered spell left the stack"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "and it never becomes the permanent it was cast as"
    );
    assert!(
        in_graveyard(&engine, p1, absorb()).is_some(),
        "the Absorb resolved and is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1].life,
        23,
        "\"You gain 3 life\" — the caster's life, and exactly three of it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the seat whose spell died loses nothing"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the {{W}}{{U}}{{U}} came out of the pool"
    );
}

/// Ember Shot prints two sentences — "Ember Shot deals 3 damage to any
/// target" and "Draw a card" — and one cast has to show both, so the seat
/// across the table is aimed at rather than the creature on it: a life total
/// that lands exactly on seventeen is the only reading that tells the printed
/// three from a one or a two, while the Llanowar Elves standing on that board
/// is what makes "any target" (CR 115.4) a claim about both option lists
/// instead of about having nothing to point at. Seven Mountains pay the
/// {6}{R} down to an empty pool, so the mana that is gone afterwards is the
/// printed cost really paid rather than a card that resolved for free, and
/// reading the draw as a library one shorter *and* a Shot in the graveyard is
/// what separates a drawn card from a hand that merely changed size.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn ember_shot_deals_three_to_any_target_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[ember_shot()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let shot = in_hand(&engine, p0, ember_shot()).expect("the Shot is in hand");

    // `LegalActions::castable` is filtered through `can_afford`, which reads
    // the *pool* and not the untapped lands — so the claim is made twice, once
    // on an empty pool and once with the mana really floating. The Elf is the
    // control for the other reason a cast is withheld: the spell has a legal
    // target on this board either way.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&shot),
        "an empty pool pays no {{6}}{{R}}, and `can_afford` reads the pool \
         rather than the seven untapped Mountains: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Mountains tapped, seven red, and no creature of mine makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&shot),
        "the seven floating pay the whole cost, so the Shot is castable: {:?}",
        legal.castable
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let my_life = engine.state().players[0].life;
    let their_life = engine.state().players[1].life;

    cast_with_floating(&mut engine, p0, ember_shot());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // mana is still floating while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the cost is the last step of the cast, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{6}}{{R}} came out of the seven the Mountains made"
    );
    assert!(
        !stack_is_empty(&engine),
        "an instant resolves off the stack, so it is waiting there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        their_life - 3,
        "\"deals 3 damage to any target\" — three, and not one per mana spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        my_life,
        "the damage went to the target that was named, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and it still carries the body it was printed with"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        in_graveyard(&engine, p0, ember_shot()).is_some(),
        "the resolved instant went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Shot left the hand and the draw put exactly one card back"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&shot),
        "and the card in hand is the drawn one, not the Shot that was cast"
    );
}

/// Rend Flesh — {2}{B} Instant, Arcane: "Destroy target non-Spirit creature."
///
/// The word that carries the card is `non-Spirit`, and no reading of the card
/// file can tell that filter from a promise — the menu is the claim. So the
/// board puts a Llanowar Elves and a Kami of Tattered Shoji under the same
/// opponent: a creature and a Spirit, which is the one distinction the printed
/// sentence draws, and a Sol Ring beside them that is no creature at all.
///
/// The answer is then taken twice — once with the Spirit, refused without the
/// other seat paying for it, and once with the Elf — and the target is named
/// while both creatures are still standing (CR 601.2c), because the destruction
/// is the resolution and not the announcement.
///
/// The three Swamps pay the printed {2}{B} down to an empty pool, so the spell
/// was really cast rather than announced, and the graveyard reads the instant
/// itself as well as the creature it killed.
#[test]
fn rend_flesh_destroys_a_non_spirit_creature_and_leaves_a_spirit_standing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(
            1,
            &[llanowar_elves(), kami_of_tattered_shoji(), quiet_artifact()],
        )
        .hand(0, &[rend_flesh()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let spirit =
        on_battlefield(&engine, p1, kami_of_tattered_shoji()).expect("their Spirit is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 to kill");
    assert_eq!(pt(&engine, spirit), (2, 5), "and a printed 2/5 Spirit");

    cast_from_hand(&mut engine, p0, rend_flesh());

    let options = pass_until_targets(&mut engine, p0);
    // CR 601.2c takes the target before CR 601.2h pays, so while this question
    // stands the three Swamps' mana is still floating.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    assert!(
        options.contains(&elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&spirit),
        "\"non-Spirit\" is the whole filter: a Spirit is a creature and still \
         not a legal target for this spell: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "the Sol Ring is an artifact and no creature: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "CR 601.2c before the effect resolves: the creature is named while it \
         is still standing"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![spirit],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, kami_of_tattered_shoji()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered is the one that dies");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Swamps are exactly {{2}}{{B}}, so the cost was paid and not \
         merely printed"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "\"Destroy target … creature\": the Elf is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the destruction itself"
    );
    assert!(
        on_battlefield(&engine, p1, kami_of_tattered_shoji()).is_some(),
        "the Spirit the filter declined never moved: one target, one death"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the Sol Ring was never on the menu to begin with"
    );
    assert!(
        in_graveyard(&engine, p0, rend_flesh()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );
}

/// Reviving Dose — {2}{W} instant: "You gain 3 life" and "Draw a card."
///
/// Two printed sentences that land in two different places, so one cast reads
/// both: the life total is the caster's alone (the opponent stays at twenty,
/// which is what tells "you" from "each player"), and the draw is read as a
/// move — the very card that was on top of the library is in hand afterwards
/// while the library is one shorter, so an effect that merely emptied the top
/// of the library could not pass. The three Plains are spent to the last mana
/// and the empty stack afterwards says the {2}{W} was paid and the spell
/// resolved rather than sitting on the stack.
#[test]
fn reviving_dose_gains_three_life_and_draws_the_top_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[reviving_dose()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The list's last entry is the top of the library, so this is the card the
    // draw is about; it is named before anything is cast.
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library to draw from");
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, reviving_dose());
    assert!(
        !stack_is_empty(&engine),
        "an instant is a spell: it goes on the stack rather than resolving as it is cast"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing has been gained while it is still waiting there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 23, "\"You gain 3 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the seat that cast it, not the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and the very card that was on top is in hand, so the draw is a move \
         and not an emptied library"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Plains paid the {{2}}{{W}} to the last mana"
    );
}

/// Scrap is `{2}{R}` for "Destroy target artifact" and "Cycling {2} ({2},
/// Discard this card: Draw a card.)" — two halves that each consume the card
/// carrying them, so two copies are dealt and played in one main phase. The
/// first is aimed at the Sol Ring across the table, which is the game's only
/// artifact and therefore the whole of what `Filter::ARTIFACT` may offer: the
/// Llanowar Elves beside it is a permanent of another kind and has to stay off
/// that menu. The second is cycled out of **hand**, the zone
/// `ActivationZone::Hand` puts the ability in, and because `can_afford` reads
/// the mana already floating rather than the five untapped Mountains the pool
/// is taken first: `{2}{R}` leaves exactly the `{2}` the cycling then charges,
/// and the pool reads empty once both prices are paid.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn scrap_destroys_the_artifact_it_names_and_cycles_itself_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[scrap(), scrap()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // Five Mountains into the pool: `{2}{R}` for the cast and the `{2}` the
    // cycling charges are one payment inside one main phase (CR 500.5), and
    // `can_afford` reads the pool rather than the untapped lands. Nothing else
    // on this board makes mana — the Sol Ring belongs to the other seat, and
    // cycling's own price is not a tap, so the helper leaves the card alone.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains tapped, and five red in the pool"
    );

    // The first half: "Destroy target artifact." Targets are named before
    // costs are paid (CR 601.2c, then CR 601.2h), so the artifact is still on
    // the battlefield and the pool is still full while the question stands.
    let spell = in_hand(&engine, p0, scrap()).expect("the first Scrap is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{2}}{{R}} is payable out of the pool, so the instant is castable: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, scrap());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one artifact, and the spell asks once");
    assert!(
        options.contains(&ring),
        "the Sol Ring is an artifact and the spell may point at it: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is the only one in the game: the Elf beside it is a permanent \
         of another kind, and every other permanent on the table is a Mountain \
         — {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "`Filter::ARTIFACT` is read and not skipped: a creature is no artifact: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while its price is \
         still unpaid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\": the named artifact is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the permanent the spell never named is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, scrap()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five mana less the {{2}}{{R}} the cast cost: exactly the {{2}} the \
         cycling charges is still floating"
    );

    // The second half. Cycling is an ability the card offers *from hand*
    // (`ActivationZone::Hand`), so the ability's source is the card in the
    // hand rather than a permanent, and its `{2}` is read off the pool the
    // cast left behind.
    let discard = in_hand(&engine, p0, scrap()).expect("the second Scrap is in hand");
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == discard)
        .expect("cycling is offered on the card in hand, which is where it lives");
    assert_eq!(
        source, discard,
        "the ability belongs to the very card that pays for it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("{{2}} is in the pool and the card is in hand, so cycling is activatable");

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&discard),
        "\"Discard this card\" is half the cost and is paid on announcement \
         (CR 601.2h), which for a card is its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the cycling ability waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert_eq!(
        hand_after.len(),
        hand_before.len(),
        "one card discarded and one card drawn: the hand is the length it was"
    );
    assert!(
        !hand_after.contains(&discard),
        "the card that paid the cost is not in the hand it left"
    );
    let drawn: Vec<ObjectId> = hand_after
        .iter()
        .copied()
        .filter(|id| !hand_before.contains(id))
        .collect();
    assert_eq!(
        drawn.len(),
        1,
        "exactly one new card, so the draw happened once and not twice"
    );
    assert!(
        library_before.contains(&drawn[0]),
        "and it came off the library rather than out of nowhere"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&drawn[0]),
        "\"Draw a card\": the card left the library for the hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "one card off the top of the library, which is the whole of the effect"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cycling's {{2}} came out of the pool the Mountains filled, so both \
         printed prices were really paid"
    );
}

/// Sudden Strength prints two sentences: "Target creature gets +3/+3 until end
/// of turn" and "Draw a card". Three readings make the card itself: the pump
/// lands on the creature the spell named and on no other, so an Elf beside the
/// host and an Elf across the table both stay printed 1/1s; the draw is a move
/// off the library and back into a hand the spell had just left; and the
/// printed duration is real, so a turn later the host is a 1/1 again with the
/// creature still standing. "Target creature" is any creature on either side of
/// the table, so the offer names the opponent's Elf as readily as mine.
#[test]
fn sudden_strength_pumps_only_the_creature_it_names_and_draws_a_card_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[sudden_strength()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays the control");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, sudden_strength());
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
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be chosen: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (4, 4),
        "+3/+3 on the creature the spell named"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody named is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        in_graveyard(&engine, p0, sudden_strength()).is_some(),
        "an instant resolves into its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw put one card back"
    );

    // "until end of turn": a turn later the Elf is the 1/1 it was printed as,
    // and it is still on the battlefield — so what expired was the pump.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the grant lasted the turn it was cast in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}

/// Tel-Jilad Justice — {1}{G} Instant: "Destroy target artifact. Scry 2."
///
/// Both printed sentences are the engine's answer rather than the card's, so
/// both are played on one board: the {1}{G} comes out of a pool only the three
/// Forests filled, the target question names the artifact across the table and
/// must decline the Elf standing beside it, and the scry is read as a *move*
/// of the top two cards rather than as a question that was asked. The destroy
/// is asserted while the scry question is already open, which is the one moment
/// that tells the two effects apart: the artifact is in its owner's graveyard
/// before the looking starts, so the scry cannot be what removed it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn tel_jilad_justice_destroys_an_artifact_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[tel_jilad_justice()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The two cards the scry is about to look at, named before anything is
    // cast: the list's last entry is the top of the library and its first is
    // the bottom, which is the order `Effect::Scry` reads the top `n` in.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests tapped for three green, and nothing else on this board makes mana"
    );
    cast_with_floating(&mut engine, p0, tel_jilad_justice());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell names the target");
    assert_eq!((min, max), (1, 1), "one artifact, and the spell asks once");
    assert!(
        options.contains(&ring),
        "the artifact across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is not an artifact, and \"target artifact\" is read rather \
         than skipped: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the destroy is an effect and not a cost: the artifact is still on the \
         battlefield while the target is being named"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] },)
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact the question offered is the one that dies");

    // The spell's two effects resolve in the order they are printed, so the
    // destroy has already happened when the scry's question arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the artifact is in its owner's graveyard before the looking starts"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and has left the battlefield, which is what destroying it means"
    );

    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("one of the two cards just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top card"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "scry reorders and draws nothing"
    );
    assert!(
        in_graveyard(&engine, p0, tel_jilad_justice()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and nothing else on the table moved: the spell destroyed one artifact \
         and looked at two cards"
    );
}

/// Wipe Clean prints two lines and both want a card in hand, so the test plays
/// them with two copies in one main phase: {1}{W} exiles an enchantment, and
/// {3} cycles the other copy away for a card. The board carries both
/// enchantments plus a creature and an artifact, so the target question is a
/// filter rather than a count — "target enchantment" reaches either side of the
/// table and neither of the other two permanents. Five Plains pay for both
/// lines out of one pool, which makes the emptied pool a price, and the cycled
/// copy landing in the graveyard beside the resolved one is the discard
/// cycling charges.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn wipe_clean_exiles_an_enchantment_and_cycles_its_other_copy_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                exploration(),
            ],
        )
        .hand(0, &[wipe_clean(), wipe_clean()])
        .battlefield(
            1,
            &[their_enchantment(), llanowar_elves(), quiet_artifact()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_enchantment =
        on_battlefield(&engine, p0, exploration()).expect("my own enchantment is out");
    let theirs =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // How many copies of this printing are in p0's graveyard, which is where
    // both halves of the card are read: a resolved instant lands there and a
    // discarded card is put there.
    let in_yard = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .filter(|id| {
                engine
                    .state()
                    .object(**id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == wipe_clean()))
            })
            .count()
    };
    assert_eq!(
        in_yard(&engine),
        0,
        "nothing has been cast or discarded yet"
    );

    // Mana first: five Plains are the whole board and five white is what the
    // two printed lines cost together.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains tapped and five white floating — the Exploration beside \
         them makes no mana"
    );
    cast_with_floating(&mut engine, p0, wipe_clean());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"exile target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!(
        (min, max),
        (1, 1),
        "one enchantment, and the spell asks once"
    );
    assert!(
        options.contains(&my_enchantment) && options.contains(&theirs),
        "\"target enchantment\" is any enchantment, on either side of the \
         table: {options:?}"
    );
    assert!(
        !options.contains(&elf) && !options.contains(&rock),
        "a creature and an artifact are no enchantments, so the filter is read \
         and not skipped: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the enchantment across the table was one of the options");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the exile resolves and the board comes back to a quiet priority"
    );

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the enchantment the spell named left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&theirs),
        "\"exile\" is exile and not the graveyard: the card is in its owner's \
         exile zone"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&my_enchantment),
        "the enchantment on this side of the table was never named"
    );
    assert_eq!(
        in_yard(&engine),
        1,
        "and the spell itself is a resolved instant in its owner's graveyard"
    );

    // The second printed line. Its price is {3} and the card, and the three
    // white the cast left behind are exactly that {3}.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "five white less the {{1}}{{W}} the spell cost"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let cycler = in_hand(&engine, p0, wipe_clean()).expect("the second copy is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == cycler),
        "cycling is taken from hand, so the card itself is the source of the \
         ability being offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, wipe_clean(), 0);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the cycled ability resolves and the seat is back at a quiet priority"
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} came out of the pool the cast left it in"
    );
    assert_eq!(
        in_yard(&engine),
        2,
        "discarding the card is cycling's own price, so the cycled copy joined \
         the resolved one in the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the cycled card left the hand and the drawn one replaced it, so the \
         count is unchanged — and only a real draw holds it there"
    );
}

/// `Ferocious Charge` is a `{2}{G}` Instant under `Coverage::Implemented`.
/// It gives target creature +4/+4 until end of turn and scries 2.
/// Casting it targets a friendly creature, prompting for the target upon casting,
/// and during resolution asks a `Pending::Arrange` with `ArrangePrompt::Scry` for the
/// scry 2 before pumping the creature from 1/1 to 5/5 until end of turn and moving to
/// the graveyard.
#[test]
fn ferocious_charge_pumps_target_creature_and_scries_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[ferocious_charge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1));

    cast_from_hand(&mut engine, p0, ferocious_charge());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Ferocious Charge, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&elf), "elf is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Scry,
                ..
            }
        )
    });

    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("expected a scry arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "active player scries");
    assert_eq!(
        piles,
        scry_piles(2),
        "Scry 2 allows choosing 0 to 2 cards to bottom"
    );
    assert_eq!(prompt, ArrangePrompt::Scry);

    engine.apply(p0, look_answer(&cards, &[])).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (5, 5),
        "target creature receives +4/+4 until end of turn"
    );
    assert!(
        in_graveyard(&engine, p0, ferocious_charge()).is_some(),
        "Ferocious Charge is in graveyard after resolution"
    );
}

/// Daring Leap — {1}{W}{U} instant: "Target creature gets +1/+1 and gains
/// flying and first strike until end of turn."
///
/// The word "creature" only shows in the menu, so the board carries three
/// lands, one Elf under the caster and one across the table: two options is
/// a claim about the CR 115.1 target the card prints, and a wildcard filter
/// would have offered the lands too. The pump and both keywords have to land
/// on the creature that was named and on no other — a grant that swept the
/// board would leave the bystander flying — and a whole turn cycle later the
/// 2/2 flier with first strike is a printed 1/1 again, which is the printed
/// duration read off the board rather than taken from the card file.
#[test]
fn daring_leap_pumps_and_arms_only_the_creature_it_names_and_only_for_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), forest(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[daring_leap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "nothing has granted anything yet"
    );

    // Mana before the claim: the cast is a real {1}{W}{U}, and what is
    // castable is filtered through the pool rather than through the untapped
    // lands, so the three sources are tapped first.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, daring_leap());

    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else on this board is one — the three lands are permanents \
         and no creatures: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the spell named"
    );
    let granted = keywords(&engine, host);
    assert!(granted.contains(KeywordSet::FLYING), "and flying");
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "and first strike"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor does the grant: one target, one creature"
    );

    // "until end of turn": a turn cycle later the Elf is the 1/1 it was
    // printed as, so the body and the keywords were a duration and not a
    // permanent change.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the creature is still standing, so the grant left rather than the creature"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the +1/+1 lasted the turn it was made in and no longer"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "and so did the flying"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and the first strike, which a permanent grant would have kept"
    );
}

// oracle_id = "a8c9f91a-b1e7-451d-b6db-ae865e2b853c"

/// Exclude prints two sentences — "Counter target creature spell" and "Draw a
/// card" — and a stack holding two spells proves both in one resolution. p0's
/// Llanowar Elves is the creature spell the counter is for, and the Dark Ritual
/// cast on top of it is the spell the printed filter has to decline, so the
/// target menu reads "creature" instead of assuming it. Countering is a move
/// between zones — the Elves ends in its owner's graveyard and never reaches
/// the battlefield — and the draw is read as the card that was on top of p1's
/// library arriving in p1's hand, which an Exclude that merely emptied the
/// stack could not produce.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn exclude_counters_a_creature_spell_and_draws_its_controller_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), swamp()])
        .hand(0, &[llanowar_elves(), dark_ritual()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[exclude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two spells on the stack at once, and only one of them is a creature: p0
    // holds priority after the Elves (CR 601.2i) and puts the instant on top of
    // it with the black the Swamp pays for.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and a Swamp, and no creature on the board to make more"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    cast_with_floating(&mut engine, p0, dark_ritual());
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves spell is on the stack");
    let ritual = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack beside it");
    assert_ne!(elves, ritual, "two spells, two objects");

    // p1's window. The mana is tapped before anything is claimed about the
    // offer, because `LegalActions` is filtered through `can_afford`, which
    // reads the pool and not the untapped Islands.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && on_stack(e, llanowar_elves()).is_some()
    });
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        4,
        "four Islands, four blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p1, exclude()).expect("the Exclude is in hand");
    assert!(
        legal.castable.contains(&card),
        "{{2}}{{U}} is payable out of the pool, so the Exclude is castable: {:?}",
        legal.castable
    );

    // The top of p1's library, named before the draw, so the card that moves is
    // the one this test is about rather than any card that happens to be
    // missing afterwards.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    let top = *library_before.last().expect("p1's library has a top card");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_with_floating(&mut engine, p1, exclude());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast the Exclude aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elves),
        "the creature spell waiting on the stack is the target it is for: {options:?}"
    );
    assert!(
        !options.contains(&ritual),
        "\"creature spell\" is read, not skipped: the instant above it is a \
         spell and no creature: {options:?}"
    );
    assert_eq!(options.len(), 1, "and the Elves are the whole menu");

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the creature spell was one of the options it enumerated");
    assert!(
        on_stack(&engine, dark_ritual()).is_some(),
        "the Ritual is untouched by the target that was named"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, llanowar_elves()).is_none(),
        "the countered spell left the stack"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and it never arrived: a countered spell resolves into a graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the card is in its owner's graveyard, which is where a countered spell goes"
    );
    assert!(
        in_graveyard(&engine, p1, exclude()).is_some(),
        "and the Exclude followed it there once it had resolved"
    );
    assert_eq!(
        library_size(&engine, p1),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&top),
        "and the card that was on top is in hand, so the draw is not merely a \
         library that got shorter"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before,
        "one card out (the Exclude) and one card in (the draw)"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "the {{2}}{{U}} came out of the pool"
    );
}

/// Repulse — {2}{U} instant: "Return target creature to its owner's hand.
/// Draw a card." Each printed half is where the other could hide, so one cast
/// reads both: the bounce lands the *opponent's* Elf in the *opponent's* hand
/// — a spell that tucked it into the caster's hand would pass every count taken
/// on this side of the table — while the draw is read as the very card that was
/// on top of the library, which a library that merely shrank by one could not
/// stand in for. The Sol Ring is the filter's witness: it is an artifact, so
/// "target creature" may not name it, and it is not what moved.
#[test]
fn repulse_bounces_the_named_creature_to_its_owners_hand_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[repulse()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");

    // The top of p0's library, named before anything is cast: the list's last
    // entry is the top, and a draw is a move rather than a count.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, repulse());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "the Sol Ring is an artifact and no creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it, \
         not to the seat that cast the spell"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and the caster's hand is not where it went"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, repulse()).is_some(),
        "the instant resolved and is in its owner's graveyard"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top of the library, which a count \
         alone would not say"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card cast and one card drawn: the hand is the size it was"
    );
}

/// Rescind prints two lines and one board plays both: "{1}{U}{U} — Return
/// target permanent to its owner's hand" and its cycling, "{2}, Discard this
/// card: Draw a card." The bounce is read across the table, where "its
/// owner's hand" is the word under test — the Sol Ring that leaves goes to the
/// hand of the seat that owns it and not to the caster's, while the second
/// permanent nobody named never moves — and the cycling is read on the copy
/// still in hand, whose `{2}` comes out of the same pool the spell left behind.
/// Five Islands are exactly both printed costs, so the pool reads five, then
/// two, then nothing, and each half's price is a payment rather than a label.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn rescind_bounces_a_permanent_to_its_owners_hand_and_cycles_a_copy_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[rescind(), rescind()])
        // A permanent to bounce and a second one to leave alone, both on the
        // far side of the table: "target permanent" crosses it, "its owner's
        // hand" is the seat that owns it, and nothing claims the board.
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before a single Island is tapped"
    );

    // `LegalActions` is filtered through `can_afford`, and that reads the pool
    // rather than the five untapped Islands. The spell has targets standing on
    // the table, so the only thing between the seat and the cast is mana.
    let card = in_hand(&engine, p0, rescind()).expect("the Rescind is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{1}}{{U}}{{U}}: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Islands, five blue, and no other source on this board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with the mana floating the same card is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, rescind());
    let options = aim_at(&mut engine, p0, rock);
    assert!(
        options.contains(&rock) && options.contains(&elf),
        "\"target permanent\" is any permanent, on either side of the table: {options:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the permanent the Rescind named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, quiet_artifact()).is_some(),
        "\"to its owner's hand\": the Sol Ring goes back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_none(),
        "and nowhere near the hand of the seat that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the permanent nobody named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, rescind()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}}{{U}}{{U}} is spent and exactly the {{2}} the cycling charges is left"
    );

    // The other printed line: "{2}, Discard this card: Draw a card." The second
    // copy is still in hand, and its activation is taken out of the offer
    // rather than guessed at — the card in hand is the source the engine names,
    // because that is the zone the ability lives in.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == rescind()))
        })
        .expect("the Rescind left in hand offers its own cycling");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two mana already floating pay for the cycling");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cycling's {{2}} came out of the pool the Islands filled"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and the discard pays for it, so the hand is the size it was"
    );
    assert_eq!(
        mine(&engine, p0, rescind(), Zone::Graveyard).len(),
        2,
        "one Rescind resolved and one was discarded to its own cycling"
    );
}

/// `Afflict` is an instant costing `{2}{B}` under `Coverage::Implemented`.
/// It prints "Target creature gets -1/-1 until end of turn. Draw a card."
/// When cast from hand off three Swamps targeting an opponent's 2/2 creature (such as `Desert Drake`),
/// the target creature's power and toughness become 1/1 until end of turn and the caster draws a card.
#[test]
fn afflict_shrinks_target_creature_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let drake_card = card_index("ce8f4eb4-08b8-404b-9147-1e28c1b14a65");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[afflict()])
        .battlefield(1, &[drake_card])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let initial_hand_len = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let target_drake =
        on_battlefield(&engine, p1, drake_card).expect("opponent controls Desert Drake");
    assert_eq!(pt(&engine, target_drake), (2, 2), "initial body is 2/2");

    cast_from_hand(&mut engine, p0, afflict());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Afflict, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&target_drake),
        "target creature is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_drake],
            },
        )
        .expect("targeting Desert Drake is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target_drake),
        (1, 1),
        "target creature received -1/-1 until end of turn"
    );
    let final_hand_len = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        final_hand_len, initial_hand_len,
        "caster spent Afflict and drew 1 card, leaving hand size unchanged"
    );
    assert!(
        in_graveyard(&engine, p0, afflict()).is_some(),
        "resolved Afflict is in caster's graveyard"
    );
}

/// `Negate` is an instant costing `{1}{U}` under `Coverage::Implemented` that counters target noncreature spell.
/// When the opponent casts a noncreature spell (such as `Rejuvenate`) and passes priority, `Negate`
/// can be cast in response off two Islands. Targeting the spell on the stack counters it, placing both
/// cards into their respective owners' graveyards without the countered spell taking effect.
#[test]
fn negate_counters_target_noncreature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let rejuvenate_card = card_index("35069cdb-e5bd-4224-a1a8-b329054e003b");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[negate()])
        .battlefield(1, &[forest(), forest(), forest(), forest()])
        .hand(1, &[rejuvenate_card])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, rejuvenate_card);
    let spell_on_stack = on_stack(&engine, rejuvenate_card).expect("Rejuvenate is on the stack");

    // p1 passes priority; p0 responds with Negate.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority for p0, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "p0 receives priority while spell is on stack");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, negate());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Negate, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&spell_on_stack),
        "Rejuvenate on stack is offered as target noncreature spell: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell_on_stack],
            },
        )
        .expect("targeting Rejuvenate with Negate is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        20,
        "countered Rejuvenate never resolved; opponent gained no life"
    );
    assert!(
        in_graveyard(&engine, p1, rejuvenate_card).is_some(),
        "countered spell was put into owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, negate()).is_some(),
        "resolved Negate is in caster's graveyard"
    );
}

/// `Rend Spirit` is an instant costing `{2}{B}` under `Coverage::Implemented`.
/// It prints "Destroy target Spirit."
/// When cast against an opponent's board containing both a Spirit (`Skyclave Apparition`) and a non-Spirit
/// (`Llanowar Elves`), targeting filters exclusively for Spirit creatures. Choosing the Spirit destroys it
/// upon resolution, leaving non-Spirit creatures on the battlefield.
#[test]
fn rend_spirit_destroys_target_spirit_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[rend_spirit()])
        .battlefield(1, &[skyclave_apparition(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let spirit =
        on_battlefield(&engine, p1, skyclave_apparition()).expect("opponent controls Spirit");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls non-Spirit");

    cast_from_hand(&mut engine, p0, rend_spirit());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Rend Spirit, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&spirit),
        "Spirit creature is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "non-Spirit creature is excluded from target options: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spirit],
            },
        )
        .expect("targeting Spirit is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, skyclave_apparition()).is_none(),
        "targeted Spirit was destroyed from the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, skyclave_apparition()).is_some(),
        "destroyed Spirit is in opponent's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "non-Spirit creature remains on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, rend_spirit()).is_some(),
        "resolved Rend Spirit sits in caster's graveyard"
    );
}

/// Flash Counter — {1}{U} instant: "Counter target instant spell."
///
/// The printed filter is a *type*, so the scenario puts exactly one spell on
/// the stack and reads the menu the cast publishes: p1's Giant Growth is on it
/// while the Elf that Growth was aimed at — a permanent, and the only other
/// object in the game — is not. The counter then has to do the whole of its
/// work rather than merely leave the stack alone: the Growth is in its owner's
/// graveyard, the +3/+3 never reached the creature, and the {1}{U} really left
/// the pool.
#[test]
fn flash_counter_counters_the_instant_spell_it_names_and_leaves_the_creature_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[flash_counter()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(1, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    // The instant under test is cast on the *opponent's* turn, which is the
    // only turn an instant needs.
    assert!(
        walk_to_own_main(&mut engine, p1),
        "p1 reaches their own main"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before the Growth");

    // p1's Forest pays {G}; the Elf is kept off the tap so that the creature
    // the pump is about to name is still the board this test reads back.
    tap_all_mana_but(&mut engine, p1, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "one Forest, one green, and the Elf kept its own {{T}}"
    );
    cast_with_floating(&mut engine, p1, giant_growth());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Giant Growth targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert_eq!(options, vec![elf], "the only creature in the game");
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf came out of the list that offered it");

    // The Growth is on the stack and p0 holds priority with it there.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && !stack_is_empty(e)
    });
    let growth = on_stack(&engine, giant_growth()).expect("the Growth is on the stack");

    cast_from_hand(&mut engine, p0, flash_counter());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target instant spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one spell, and the cast asks once");
    assert_eq!(
        options,
        vec![growth],
        "the instant on the stack is the whole menu: the creature the Growth \
         was aimed at is a permanent and no spell"
    );
    // CR 601.2c before CR 601.2h: the target is named while the {1}{U} the two
    // Islands made is still in the pool.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Islands are tapped and the {{1}}{{U}} is not yet paid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![growth],
            },
        )
        .expect("the spell the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, giant_growth()).is_some(),
        "a countered spell goes to its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"counter\": the +3/+3 never resolved, so the Elf is the 1/1 it was"
    );
    assert!(
        in_graveyard(&engine, p0, flash_counter()).is_some(),
        "and the counter itself resolved rather than being countered or fizzling"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{U}} it charged came out of the pool"
    );
}

// oracle_id = "41bfef9f-6eb7-49c7-9b90-ff9f385ba670"

/// Second Thoughts — {4}{W} instant: "Exile target attacking creature. Draw
/// a card."
///
/// Both printed sentences are read off one combat phase, and each needs a
/// witness the other does not give it. The board carries an attacking Elf and
/// an Elf that stayed home, so the menu the target question publishes *is* the
/// first word of the card — a filter that had lost `ATTACKING` would offer the
/// bystander just as readily. The five Plains are tapped before anything is
/// claimed, because `can_afford` reads the pool and not the untapped lands,
/// and the creature is then read in the **exile** zone rather than in a
/// graveyard, which is the one word that separates this card from every
/// destroy spell. The draw is read as a move: the card that was on top of the
/// library is the card in hand afterwards, and the spell leaving the hand and
/// the drawn card replacing it cancel out of the hand count exactly.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn second_thoughts_exiles_an_attacking_creature_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Five Plains are exactly {4}{W}. One Elf is this seat's attacker, and the
    // one across the table is the creature that must stay off the menu.
    let mut engine = Duel::new(31, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[second_thoughts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, attacker), (1, 1), "a body that may attack");
    assert_eq!(pt(&engine, bystander), (1, 1), "and one that stays home");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the active seat declares its own attackers");
    assert!(
        attackers.contains(&attacker),
        "an untapped 1/1 of this seat is offered as an attacker: {attackers:?}"
    );
    assert_eq!(
        attackers.len(),
        1,
        "and it is the only creature this seat has: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // The combat phase keeps a priority round open with the Elf already
    // attacking and the damage step still ahead — the only place the card's
    // own filter can find a target. Answering the round by hand is also what
    // reads the price honestly: the mana is tapped for the question that
    // actually needs it and not a step earlier.
    let mut window = false;
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == p0 => {
                window = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing priority is always legal");
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .expect("declaring no blockers is always legal");
            }
            other => panic!("unexpected in the combat phase: {other:?}"),
        }
    }
    assert!(
        window,
        "the seat holding the instant is offered priority while its Elf is \
         still attacking"
    );

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Mana before the claim (the offer is read off the pool, not off the
    // board), and the attacking Elf named as the source kept back: it prints
    // its own `{T}: Add {G}`, so tapping it would put six mana in the pool for
    // a five-mana spell (#159).
    tap_mana_except(&mut engine, p0, attacker);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains, five white, and nothing off the Elf that is attacking"
    );
    cast_with_floating(&mut engine, p0, second_thoughts());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target attacking creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert_eq!(
        options,
        vec![attacker],
        "the attacking creature is the whole menu — the Elf on the other side \
         of the table is a creature and is not attacking, which is the word \
         this filter is read on"
    );
    // CR 601.2c names the target before CR 601.2h pays the cost, so the mana
    // is still floating and the creature still on the battlefield here.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the exile has not happened either: it is the spell's resolution"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .expect("the attacking creature was the one option the question offered");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "the spell is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&attacker),
        "\"Exile target attacking creature\": the Elf is in exile, under the \
         player who owned it"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "and not in a graveyard — a destroy spell would have put it there, and \
         nothing about the board above would look different"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the attacker left the battlefield"
    );
    assert_eq!(
        on_battlefield(&engine, p1, llanowar_elves()),
        Some(bystander),
        "and the creature the spell did not name never moved"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "still the printed 1/1 of a body that stayed home"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top of the library, not merely \
         some card that appeared"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw replaced it, so the count is \
         where it was"
    );
}

/// Seething Song prints one line — "Add {R}{R}{R}{R}{R}" — on an instant that
/// costs {2}{R}, so the card is a price and a colour and nothing else. The end
/// step is where the two halves meet: a sorcery could not be cast there at all,
/// so the engine naming the Song in `legal.castable` with an empty stack is the
/// printed "Instant" rather than a reading of the card file, and the five red
/// that land in the pool afterwards come out of exactly the three Mountains the
/// {2}{R} cost — which is why the pool reads five and not eight.
#[test]
fn seething_song_trades_three_mana_for_five_red_in_the_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[seething_song()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let song = in_hand(&engine, p0, seething_song()).expect("the Song is in hand");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the board is three untapped Mountains and nothing floating"
    );

    // Through combat and into the end step of p0's own turn: the active player
    // opens that priority round (CR 117.3a) and no sorcery may be cast in it.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, and nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&song),
        "\"Instant\": with an empty stack in the end step the Song is castable, \
         which is precisely the moment a sorcery is not: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, seething_song());
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        5,
        "\"Add {{R}}{{R}}{{R}}{{R}}{{R}}\" — five of the one colour the card names"
    );
    assert_eq!(
        pool.total(),
        5,
        "and nothing beside them: the {{2}}{{R}} was spent out of the three \
         Mountains rather than left floating under the new mana"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Green,
    ] {
        assert_eq!(
            pool.available(color),
            0,
            "a Song that added \"one mana of any color\" five times would leave \
             {color:?} in the pool here"
        );
    }
    assert!(
        in_graveyard(&engine, p0, seething_song()).is_some(),
        "an instant that resolved is in its owner's graveyard, not merely gone \
         from the hand"
    );
    assert!(
        stack_is_empty(&engine),
        "and nothing it started is still waiting to resolve"
    );
}

/// Shatter — {1}{R} instant: "Destroy target artifact."
///
/// The target is the whole card, so the menu is what proves the printed word
/// `artifact` was read and not skipped: two Sol Rings — one under each seat —
/// are on it while the Llanowar Elves beside them and every land on the table
/// are not, because the filter names a type and no side of the battlefield.
/// Answering with the opponent's Ring and letting the spell resolve reads the
/// other half of the sentence — the card in its owner's graveyard, my own Ring
/// untouched, the declined creature still standing — and the battlefield read
/// while the question is open is CR 601.2c before CR 601.2h: the cost is the
/// last step of the cast, so nothing has moved yet when the target is named.
#[test]
fn shatter_destroys_the_artifact_it_names_and_no_other_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), quiet_artifact()])
        // An artifact and a creature across the table, so the filter has to
        // reach one and decline the other, and a land on my own side so
        // "artifact" is also read against a permanent I control.
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .hand(0, &[shatter()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, shatter()).expect("the spell is in hand");
    // `castable` is filtered through `can_afford`, which reads the pool and
    // not the untapped lands: with nothing floating the {1}{R} is unpayable,
    // so the claim is made only once the mana is really there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{1}}{{R}}: {:?}",
        legal.castable
    );

    // `tap_all_mana` presses every mana ability whose whole price is its own
    // `{T}` (#159), and the Sol Ring beside the Mountains prints one — so it
    // is named as the thing kept back, which keeps it standing as the control
    // below and makes "two red" a count of the Mountains alone.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains tapped and the Sol Ring kept back: {{R}}{{R}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with {{R}}{{R}} floating the spell is offered: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, shatter());

    let options = pass_until_targets(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target artifact\" reaches either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is no artifact, whatever its controller: {options:?}"
    );
    for land in all_on_battlefield(&engine, p0, mountain()) {
        assert!(
            !options.contains(&land),
            "a land is no artifact either: {options:?}"
        );
    }
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "costs are paid last (CR 601.2h), so the Ring is still standing while \
         the target question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Sol Ring the question offered is a legal target");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool as the spell was announced"
    );
    assert!(
        !stack_is_empty(&engine),
        "the spell is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\": the Ring is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and has left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and neither did the creature the filter declined"
    );
    assert!(
        in_graveyard(&engine, p0, shatter()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// Strength in Numbers is `{1}{G}` for "Until end of turn, target creature
/// gains trample and gets +X/+X, where X is the number of attacking
/// creatures." X is the whole card, so the spell is aimed at the one Elf that
/// stayed home while two others attack: a home 1/1 reading 3/3 can only have
/// counted the attackers, where aiming at an attacker itself could not tell
/// X = 1 from X = 2 or even from 0. The two attackers and the Elf across the
/// table are the controls for "target creature" — neither may change — and
/// walking a whole turn afterwards is the control for the printed "until end of
/// turn", which no single-moment reading can see.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn strength_in_numbers_counts_the_attackers_for_a_creature_that_is_not_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // A creature across the table, so "target creature" has a second side
        // to reach and a bystander that must stay a printed 1/1.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[strength_in_numbers()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 3, "three Elves: two attack, one stays home");
    let (first, second) = (elves[0], elves[1]);
    let home = elves[2];
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, home), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, home).contains(KeywordSet::TRAMPLE),
        "and it starts with no trample for the spell to be given credit for"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the active seat declares its attackers");
    assert!(
        attackers.contains(&first) && attackers.contains(&second),
        "the two untapped Elves are on the offer: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (first, Defender::Player(p1)),
                    (second, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attackers came out of the list that offered them");

    // The instant is played in the step it counts, and the mana is made there
    // too: a pool empties when a step ends (CR 500.5), so the Forests cannot be
    // tapped in the main phase and carried into the attack. Both attackers are
    // tapped by now and the third Elf is named as the printing kept back — it is
    // the creature the spell is about to be aimed at.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests and no Elf of mine: {{G}}{{G}} is the whole pool"
    );

    let spell = in_hand(&engine, p0, strength_in_numbers()).expect("the instant is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{1}}{{G}} is in the pool, so the instant is playable in the middle of \
         the combat step: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, strength_in_numbers());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&home) && options.contains(&first) && options.contains(&second),
        "\"target creature\" is any creature, the two attackers included: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and it reaches across the table, which a filter carrying \
         `ControlledByYou` would not do: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // mana is still floating while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cost is the last step of the cast, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![home],
            },
        )
        .expect("the creature the question offered was chosen");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and then the {{1}}{{G}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, home),
        (3, 3),
        "+X/+X with X the number of attacking creatures, which is two — on a \
         creature that is not one of them, so a 1 or a 0 would have to have come \
         from somewhere other than the printed count"
    );
    assert!(
        keywords(&engine, home).contains(KeywordSet::TRAMPLE),
        "and the printed trample reaches the same creature"
    );
    assert_eq!(
        pt(&engine, first),
        (1, 1),
        "the attackers the count was made of are untouched: the effect targets, \
         it does not sweep the board"
    );
    assert_eq!(pt(&engine, second), (1, 1), "and neither is the other one");
    assert!(
        !keywords(&engine, first).contains(KeywordSet::TRAMPLE)
            && !keywords(&engine, second).contains(KeywordSet::TRAMPLE),
        "nor was either of them granted a keyword it never asked for"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "and nothing crosses the table");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the Elf on the other side is a target the spell could have named and \
         did not"
    );

    // "until end of turn": a whole turn later the Elf is a printed 1/1 with no
    // trample, which is the only reading that tells the printed duration from a
    // permanent grant.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, home),
        (1, 1),
        "the pump lasted the turn it was cast in and no longer"
    );
    assert!(
        !keywords(&engine, home).contains(KeywordSet::TRAMPLE),
        "and the trample left with it"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature is still standing, so the grant left rather than the creature"
    );
    assert!(
        in_graveyard(&engine, p0, strength_in_numbers()).is_some(),
        "an instant is in its owner's graveyard once it has resolved"
    );
}

/// `Dismiss` is an instant costing `{2}{U}{U}` under `Coverage::Implemented`.
/// It prints "Counter target spell. Draw a card."
/// When an opponent casts a spell (such as `Llanowar Elves`), `Dismiss` can be cast
/// targeting that spell on the stack, countering it to the graveyard and drawing a card.
#[test]
fn dismiss_counters_target_spell_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[dismiss()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let initial_p1_cards = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    let elf_spell =
        on_stack(&engine, llanowar_elves()).expect("Llanowar Elves spell is on the stack");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    let dismiss_card = in_hand(&engine, p1, dismiss()).expect("Dismiss is in p1's hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: dismiss_card })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Dismiss, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf_spell),
        "target spell is offered: {options:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elf_spell],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "countered spell is placed into p0's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "countered spell never entered the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, dismiss()).is_some(),
        "Dismiss resolved and went to p1's graveyard"
    );
    assert_eq!(
        library_size(&engine, p1),
        initial_p1_cards - 1,
        "Dismiss caused p1 to draw a card"
    );
}

/// `Fanatical Fever` is an instant costing `{2}{G}{G}` under `Coverage::Implemented`.
/// It prints "Target creature gets +3/+0 and gains trample until end of turn."
/// When cast from hand off four Forests targeting `Llanowar Elves`, the target gets +3/+0
/// (growing from 1/1 to 4/1) and gains trample until end of turn.
#[test]
fn fanatical_fever_pumps_power_and_grants_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[fanatical_fever()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves is on the battlefield");
    assert_eq!(pt(&engine, elf), (1, 1), "base body is 1/1");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "target does not have trample initially"
    );

    cast_from_hand(&mut engine, p0, fanatical_fever());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Fanatical Fever, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "Llanowar Elves is offered: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (4, 1),
        "creature gets +3/+0 until end of turn"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "creature gains trample until end of turn"
    );
    assert!(
        in_graveyard(&engine, p0, fanatical_fever()).is_some(),
        "Fanatical Fever resolves to the graveyard"
    );
}

/// `Soothing Balm` is an instant costing `{1}{W}` under `Coverage::Implemented`.
/// It prints "Target player gains 5 life."
/// Because its target specification is `TargetSpec::AnyPlayer`, casting it prompts
/// with `Pending::ChoosePlayer`. Answering with `PlayerAction::ChoosePlayer` increases
/// the target player's life total by 5 upon resolution.
#[test]
fn soothing_balm_gains_five_life_for_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[soothing_balm()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, soothing_balm());

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "expected ChoosePlayer prompt for Soothing Balm, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "caster chooses the target player");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "target player reaches either seat: {options:?}"
    );

    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        25,
        "target player gained 5 life, increasing total from 20 to 25"
    );
    assert!(
        in_graveyard(&engine, p0, soothing_balm()).is_some(),
        "Soothing Balm resolves and goes to graveyard"
    );
}

// oracle_id = "39213de3-6a4a-4879-a7f9-70f45013765e"

/// Everybody Lives! — {1}{W} Instant: "All creatures gain hexproof and
/// indestructible until end of turn. Players gain hexproof until end of turn.
/// Players can't lose life this turn and players can't lose the game or win
/// the game this turn."
///
/// A grant that lasts until end of turn is only worth playing in the turn it
/// refuses something, so the instant is cast in the **opponent's** main phase,
/// where their sorcery is waiting to be aimed: the Vindicate's target menu then
/// holds my artifacts and none of my creatures, while the Elf across the table
/// carries the same two keywords ("all creatures" and not my half of it). The
/// indestructible half is read off a destroy that *does* name my creature — my
/// own Despotic Scepter, which hexproof never shielded from its controller's
/// own ability — and the same ability aimed at a Plains takes the land down, so
/// the Elf left standing is the keyword's doing and not a destroy that quietly
/// never worked.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn everybody_lives_hides_and_arms_every_creature_through_the_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                despotic_scepter(),
                despotic_scepter(),
                llanowar_elves(),
                forest(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[everybody_lives()])
        .battlefield(1, &[llanowar_elves(), plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HEXPROOF),
        "nothing has granted anything yet"
    );

    // Into the opponent's main phase: the spell is an instant, and the turn it
    // has to refuse is the one whose sorcery is about to be aimed at my board.
    reach_their_main_phase(&mut engine, p1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the active seat opens its own main phase");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the instant is cast in the opponent's turn, so the seat holding it \
         needs priority: got {:?}",
        engine.pending()
    );
    cast_from_hand(&mut engine, p0, everybody_lives());
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });

    let granted = keywords(&engine, mine);
    assert!(
        granted.contains(KeywordSet::HEXPROOF) && granted.contains(KeywordSet::INDESTRUCTIBLE),
        "the creature under my own control has both printed keywords: {granted:?}"
    );
    let across = keywords(&engine, theirs);
    assert!(
        across.contains(KeywordSet::HEXPROOF) && across.contains(KeywordSet::INDESTRUCTIBLE),
        "\"All creatures\" is the whole table and not my half of it: {across:?}"
    );

    let scepters = all_on_battlefield(&engine, p0, despotic_scepter());
    assert_eq!(
        scepters.len(),
        2,
        "two Scepters, one for each destroy this scenario needs"
    );

    // The opponent's sorcery, aimed while the grant is live.
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert!(
        !options.contains(&mine),
        "hexproof is the opponents' word (CR 702.11b): the spell cast across the \
         table has no room for the Elf it was brought for: {options:?}"
    );
    assert!(
        options.contains(&scepters[0]) && options.contains(&scepters[1]),
        "and the board it declines to name is not an empty one — everything I \
         control that is no creature is still on that menu: {options:?}"
    );

    let doomed = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert!(
        options.contains(&doomed),
        "a land of mine is a permanent the menu really offers: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the land was one of the options the question enumerated");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the Vindicate resolved against the one permanent it was allowed to name"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "and the Forest left the battlefield, so the cast was a real one"
    );

    // The indestructible half, read off a destroy hexproof never blocked: the
    // Scepter is mine, so my own creature is a permanent its controller owns.
    activate(&mut engine, p0, despotic_scepter(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Scepter destroys a permanent its controller owns, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own ability");
    assert!(
        options.contains(&mine),
        "hexproof shields a creature from its controller's *opponents* only, so \
         my own destroy may still name my own Elf: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Elf was one of the options the question enumerated");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"indestructible\": the destroy resolved and the Elf is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "and it was not put into a graveyard instead"
    );

    // The same ability aimed at something with no keyword at all: the destroy
    // really works, so the Elf above stood on indestructible.
    let lands = all_on_battlefield(&engine, p0, plains());
    assert_eq!(lands.len(), 2, "both Plains are still standing");
    activate(&mut engine, p0, despotic_scepter(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the second Scepter asks the same question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that paid the tap names the target");
    assert!(
        options.contains(&lands[0]),
        "a Plains is a permanent this seat owns: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lands[0]],
            },
        )
        .expect("the land the question offered is destroyed");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        1,
        "the destroy takes a land down, so nothing was wrong with the ability"
    );
    assert!(
        in_graveyard(&engine, p0, plains()).is_some(),
        "and the land is where a destroyed permanent goes"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf is still there beside it, which only the keyword can account for"
    );
}

// oracle_id = "d09c9cba-fdd2-479b-ad5d-d05181c3e3f9"

/// Fierce Guardianship prints "{2}{U} — Instant: If you control a commander, you
/// may cast this spell without paying its mana cost" and "Counter target
/// noncreature spell".
///
/// The free half is played on a seat that controls its commander and **no
/// untapped land**. The commander has to be *cast*: CR 903.6 starts it in the
/// command zone, and a second copy of the card seated on the battlefield is
/// not a commander at all. So seat 0 spends every land it has on it, and on
/// seat 1's turn the pool is empty and the alternative cost is the only way
/// the card can be offered; a board without a commander refuses it until the
/// printed {2}{U} is really floating — which is what separates the
/// printed condition from a card that is simply free. Seat 1 casts a creature
/// and then, still holding priority (CR 117.3c), an instant on top of it — an
/// artifact could not follow, because CR 301.1 wants an empty stack — so the
/// target menu reads the filter: the Brainstorm is offered, the Elves is not,
/// and the Elves is the spell that resolves behind the counter.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn fierce_guardianship_casts_free_under_a_commander_and_counters_only_the_noncreature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .commander(0, &[katara_the_fearless()])
        .battlefield(0, &[forest(), plains(), island()])
        .hand(0, &[fierce_guardianship()])
        .battlefield(1, &[forest(), island(), island()])
        .hand(1, &[llanowar_elves(), brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The commander comes down for its printed {G}{W}{U}, which is exactly
    // what the three lands make. They stay tapped through seat 1's turn
    // (CR 502.3 untaps only the active player's permanents) and the pool
    // empties with the step (CR 106.4).
    tap_all_mana(&mut engine, p0);
    let commander = engine
        .state()
        .zones
        .list(ZoneLocation::Command(p0))
        .first()
        .copied()
        .expect("Katara starts in the command zone");
    engine
        .apply(p0, PlayerAction::CastSpell { card: commander })
        .expect("a Forest, a Plains and an Island pay {G}{W}{U}");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, katara_the_fearless()).is_some()
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        on_battlefield(&engine, p0, katara_the_fearless()),
        Some(engine.state().commanders[0][0].object),
        "the Katara on the battlefield is the commander itself, not another copy of the card"
    );
    reach_their_main_phase(&mut engine, p1);

    // Seat 1 stacks two spells without letting either resolve: the creature is
    // the control for the counter's own filter, not a second target. It goes
    // first because it wants an empty stack (CR 302.1), and the instant follows
    // while seat 1 still holds priority.
    cast_from_hand(&mut engine, p1, llanowar_elves());
    cast_with_floating(&mut engine, p1, brainstorm());
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the creature spell is waiting on the stack for seat 0's answer, underneath"
    );
    assert!(
        on_stack(&engine, brainstorm()).is_some(),
        "and so is the instant seat 1 cast on top of it while it still held priority"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && on_stack(e, brainstorm()).is_some()
    });
    let seat_1_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: every land seat 0 has was spent on the commander"
    );
    let spell = in_hand(&engine, p0, fierce_guardianship()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "a commander is standing and there is no mana anywhere: the free \
         alternative is the only way this is offerable: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the commander's alternative cost pays for it");

    let storm = on_stack(&engine, brainstorm()).expect("the Brainstorm is on the stack");
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves is on the stack");
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target noncreature spell\" names one, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one spell, and the counter asks once");
    assert!(
        options.contains(&storm),
        "the instant spell is a noncreature spell: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the creature spell on the same stack is not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![storm],
                players: vec![],
            },
        )
        .expect("the Brainstorm was one of the options the question enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price was the commander: not one mana was paid"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, brainstorm()).is_some(),
        "a countered spell goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        seat_1_hand,
        "and never resolves: a Brainstorm that did would have left seat 1 a card up"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature spell was no legal target and resolved behind the counter"
    );
    assert!(
        in_graveyard(&engine, p0, fierce_guardianship()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );

    // The control for the free cast: the same spell on a seat that controls no
    // commander, where the printed {2}{U} is the only way in.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[fierce_guardianship()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, quiet_artifact());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && on_stack(e, quiet_artifact()).is_some()
    });

    let spell = in_hand(&engine, p0, fierce_guardianship()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "no commander and an empty pool: the free cast is not on offer: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, three blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "with the printed {{2}}{{U}} in the pool the spell is castable: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("three blue pays the printed {{2}}{{U}}");
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that paid aims it");
    let countered = on_stack(&engine, quiet_artifact()).expect("the Sol Ring is on the stack");
    assert!(options.contains(&countered), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![countered],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed {{2}}{{U}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the paid cast counters exactly the same way"
    );
}

/// Heliod's Intervention is a modal instant — `{X}{W}{W}` for "destroy X
/// target artifacts and/or enchantments", or "target player gains twice X
/// life". This scenario plays the destroy mode at X = 1 with exactly three
/// permanents its filter names on the table — my Sol Ring, my Exploration
/// and the opponent's Sol Ring — so the single target the cast names is the
/// only permanent destroyed. The two survivors prove both halves at once:
/// X governs how many, and "artifacts and/or enchantments" governs which
/// kinds appear on the menu (the four Plains do not).
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn heliods_intervention_destroys_exactly_the_x_artifacts_or_enchantments_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                quiet_artifact(),
                exploration(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[heliods_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let my_chant = on_battlefield(&engine, p0, exploration()).expect("my Exploration is out");
    let their_ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // Mana first: `can_afford` reads the pool and not the untapped lands, so
    // the spell's own costs are what the offer is filtered against.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White)
            >= 2,
        "the four Plains are the white the {{W}}{{W}} needs"
    );
    cast_with_floating(&mut engine, p0, heliods_intervention());

    // The three questions the cast asks — X, the mode and the target — each
    // in whichever order the engine raises them.
    let mut chose_x = false;
    let mut chose_mode = false;
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseNumber { player, min, max } => {
                assert_eq!(player, p0, "the caster names X");
                assert!(
                    min <= 1 && 1 <= max,
                    "X = 1 is a legal value: {min}..={max}"
                );
                engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
                chose_x = true;
            }
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the caster names the mode");
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
                    .expect("the destroy mode is one of the ways to cast this card");
                engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
                chose_mode = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the caster aims the destroy");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "X = 1: exactly one target, no more and no fewer"
                );
                assert!(
                    options.contains(&their_ring)
                        && options.contains(&my_ring)
                        && options.contains(&my_chant),
                    "\"artifacts and/or enchantments\" is any of the three, on \
                     either side of the table: {options:?}"
                );
                assert_eq!(
                    options.len(),
                    3,
                    "the four Plains are lands, neither artifacts nor \
                     enchantments: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![their_ring],
                        },
                    )
                    .expect("the Sol Ring X named is one of the options");
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while casting Heliod's Intervention: {other:?}"),
        }
    }
    assert!(chose_x, "X is a question the caster answers");
    assert!(
        chose_mode,
        "with two modes printed, the caster picks one of them"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy X target artifacts and/or enchantments\" — the Sol Ring the \
         target question named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and it is no longer on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact X did not name is untouched: X is a count, not a sweep"
    );
    assert!(
        on_battlefield(&engine, p0, exploration()).is_some(),
        "and the enchantment beside it survives, which is the half of the \
         filter only \"and/or\" can account for: destroying every permanent \
         the filter matched would empty the board"
    );
}

// oracle_id = "74d3277a-38e5-4732-afed-084a56148f20"

/// Mana Drain is `{U}{U}` and prints two sentences: "Counter target spell",
/// and "At the beginning of your next main phase, add an amount of {C} equal
/// to that spell's mana value."
///
/// The victim is Tidings — `{3}{U}{U}`, five mana and no target — so the
/// delayed mana has to be **five** colourless. That is the whole second
/// sentence: the amount is read off the spell that was countered, which a
/// one-mana victim could not tell from the Drain's own cost or from a card
/// that always adds one. Five Islands pay Tidings' price to the last mana, so
/// the five that arrive later are not leftovers, and reading them at p1's
/// *next* first main phase is the first sentence too: mana made during p0's
/// main phase would have emptied with it (CR 500.5) and the walk would never
/// see it at all.
#[test]
fn mana_drain_counters_a_five_mana_spell_and_pays_five_colorless_a_turn_later() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[tidings()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mana_drain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a first main phase of its own"
    );

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, tidings());
    let victim = on_stack(&engine, tidings()).expect("Tidings went on the stack");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Islands are exactly {{3}}{{U}}{{U}}, so nothing is left floating"
    );

    // p0 passes, and the opponent gets the window an instant needs.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && on_stack(e, tidings()).is_some()
    });
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "the two Islands are {{U}}{{U}}, and the Drain asks for no more"
    );
    cast_with_floating(&mut engine, p1, mana_drain());
    let menu = aim_at(&mut engine, p1, victim);
    assert!(
        menu.contains(&victim),
        "the only spell on the stack is the one being countered: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, tidings()).is_some(),
        "\"Counter target spell\": the sorcery is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, mana_drain()).is_some(),
        "and the counterspell itself resolved rather than fizzling"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "Tidings draws four cards only if it resolves — a countered spell does nothing"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the Drain's price was all p1 had, so the pool starts this walk blank"
    );

    // The second sentence, at the only moment it can be read: p1's own next
    // first main phase, with the mana already made.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().players[1].mana_pool.total() > 0
    });

    let pool = &engine.state().players[1].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        5,
        "the countered spell's mana value is five, so five {{C}} — not one, \
         and not the two the Drain itself cost"
    );
    assert_eq!(
        pool.total(),
        5,
        "and nothing else is floating: the two Islands paid for the Drain"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "\"add an amount of {{C}}\": the mana is colourless, and no blue land \
         on this board made any of it"
    );
}

/// Misdirection is {3}{U}{U} for two printed sentences — "You may exile a blue
/// card from your hand rather than pay this spell's mana cost" and "Change the
/// target of target spell with a single target" — and neither is readable off
/// the card file. p1 therefore owns **no land at all**: the pitched Counterspell
/// is the whole of the price, so the empty pool is how clause one is read. The
/// Swords to Plowshares p0 aimed across the table is then turned onto p0's own
/// Elf, which is clause two: the exile says the target really moved, and the one
/// life says the spell is still p0's, because a redirected spell pays its own
/// controller and not the seat that re-aimed it.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn misdirection_pitches_a_blue_card_to_turn_the_swords_onto_my_own_elves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .hand(0, &[swords_to_plowshares()])
        // No land under p1: the printed cost is unpayable here, so the
        // alternative cost is the only way this spell can arrive at all.
        .battlefield(1, &[llanowar_elves()])
        .hand(1, &[misdirection(), counterspell()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // (1) The spell that is about to be redirected. p0's Swords names the Elf
    // across the table, which is where its life payment would have shown.
    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    let swords_spell =
        on_stack(&engine, swords_to_plowshares()).expect("the Swords is waiting on the stack");

    // (2) The response, and the card under test. `pass_until` hands priority
    // to the seat that may answer the spell that was just cast.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "p1 owns no land at all, so the pitch cost is the whole of what this \
         spell can cost"
    );
    let pitch = in_hand(&engine, p1, counterspell()).expect("the blue card to pitch is in hand");
    cast_with_floating(&mut engine, p1, misdirection());

    // Clause one's cost, clause two's own target and clause two's new target
    // arrive in whatever order the engine asks them; each is answered where it
    // is, and the two target questions are told apart by what they enumerate.
    let mut turned = false;
    for _ in 0..24 {
        match engine.pending().clone() {
            // The redirect's own question, asked of the seat that cast
            // Misdirection. A menu holding a creature is that question: the
            // spell's own target choice below enumerates a spell instead.
            Pending::ChooseTargets {
                player, options, ..
            } if options.contains(&mine) || options.contains(&theirs) => {
                assert_eq!(
                    player, p1,
                    "the controller of the redirect names the new target, not \
                     the controller of the spell"
                );
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "the new target may be a creature either side of the table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .expect("the new target came out of the menu it published");
                turned = true;
                break;
            }
            // {3}{U}{U} against a pitch cost, and only one of them payable on
            // a board with no mana.
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .expect("the pitch cost is one of the ways to cast this card");
                engine
                    .apply(player, PlayerAction::ChooseMode(slot))
                    .expect("the pitch cost is a legal way to pay it");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p1, "the seat paying the cost answers it");
                assert_eq!((min, max), (1, 1), "one blue card, no more and no fewer");
                assert!(
                    options.contains(&pitch),
                    "the blue card in hand is the whole of the price: {options:?}"
                );
                for id in &options {
                    assert!(
                        engine.state().object(*id).is_some_and(|o| {
                            o.characteristics()
                                .colors
                                .contains(baylee_core::color::Color::Blue)
                        }),
                        "\"exile a **blue** card from your hand\": {id:?} is not one"
                    );
                }
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .expect("the card the cost offered pays it");
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p1, "the seat casting Misdirection aims it");
                assert!(
                    options.contains(&swords_spell),
                    "\"target spell\" is the Swords that is on the stack: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![swords_spell],
                        },
                    )
                    .expect("the Swords was one of the spells it offered");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing priority is always legal");
            }
            other => panic!("unexpected while Misdirection is being cast: {other:?}"),
        }
    }
    assert!(
        turned,
        "the redirect asked for the spell's new target, which is the whole card"
    );
    pass_until(&mut engine, stack_is_empty);

    // (3) Clause two, read where it lands rather than where it was asked: the
    // spell now names p0's Elf, so that is the creature that goes to exile.
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the creature the Swords was turned onto left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))),
        "and it is in *exile*, which is where Swords to Plowshares sends a creature"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell was originally aimed at never moved, so the \
         target really changed rather than the spell merely resolving"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"its controller gains life equal to its power\": the Swords is still \
         p0's, so p0 gains the one the Elf was worth"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the seat that re-aimed the spell pays nothing for it"
    );

    // (4) Clause one, read the same way: a blue card out of the hand into
    // exile, and no mana produced anywhere.
    assert!(
        in_hand(&engine, p1, counterspell()).is_none(),
        "the pitched card left the hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == counterspell()))),
        "and it is exiled, which is the whole of the alternative cost"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "no mana was ever produced, because p1 owns no land at all"
    );
    assert!(
        in_graveyard(&engine, p1, misdirection()).is_some(),
        "the instant resolved into its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, swords_to_plowshares()).is_some(),
        "and so did the spell it re-aimed"
    );
}

// oracle_id = "f3e213a4-ba5a-468a-93b3-c0a34e1bd725"

/// Pact of Negation is a free counterspell paid for later: "Counter target
/// spell", and then "At the beginning of your next upkeep, pay {3}{U}{U}. If
/// you don't, you lose the game."
///
/// Both printed sentences are played here, because the first is what makes the
/// second reachable at all. The Pact answers a Llanowar Elves its own
/// controller has just cast — "target spell" names no controller, so the card
/// in the graveyard instead of on the battlefield is the whole of what
/// countered means — and the deferred price is then demanded in a *later*
/// turn's upkeep, where the engine's question is the only thing that proves the
/// second sentence is written at all. The five mana the answer spends are
/// floated in that same upkeep (CR 500.5 empties a pool only when a step ends),
/// and the seat is still in the game once the cost is paid.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn pact_of_negation_counters_a_spell_now_and_charges_for_it_at_the_next_upkeep() {
    let p0 = PlayerId::new(0);
    let mut board = vec![island(); 8];
    board.push(forest());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[llanowar_elves(), pact_of_negation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A spell is put on the stack only so the Pact has something to answer: an
    // Elf is the cheapest card whose resolution leaves a mark in a different
    // zone than its countering does.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "eight Islands and one Forest: every source on the board makes one"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // {0}: the Pact is cast off a cost it does not pay into, and the caster
    // still holds priority with its own spell waiting below.
    cast_with_floating(&mut engine, p0, pact_of_negation());
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
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one spell, and the Pact asks once");
    let elves_spell =
        on_stack(&engine, llanowar_elves()).expect("the Elves spell is waiting on the stack");
    assert!(
        options.contains(&elves_spell),
        "\"target spell\" puts the spell that has not resolved yet on the menu, \
         whichever seat cast it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves_spell],
            },
        )
        .expect("the spell the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the spell was countered, so the creature never reached the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and a countered spell goes to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, pact_of_negation()).is_some(),
        "the Pact itself resolved, which is where its second sentence starts"
    );
    let cast_turn = engine.state().turn.number;

    // The deferred half. Tapping every mana ability whose whole price is its
    // own {T} each time this seat holds priority leaves a pool ready for the
    // question wherever it lands, since CR 500.5 only empties a pool when a
    // step ends — and the question is the whole reading, so a walk that never
    // meets one is a finding.
    let mut asked = false;
    for _ in 0..600 {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                assert_eq!(
                    player, p0,
                    "the debt belongs to the seat that cast the Pact"
                );
                assert_ne!(
                    engine.state().turn.number,
                    cast_turn,
                    "\"at the beginning of your next upkeep\": the price is not \
                     demanded in the turn the Pact was cast"
                );
                let before = engine.state().players[0].mana_pool.total();
                assert!(
                    before >= 5,
                    "{{3}}{{U}}{{U}} is five mana and the pool holds {before}"
                );
                engine
                    .apply(p0, PlayerAction::YesNo(true))
                    .expect("the pool covers the printed cost");
                assert_eq!(
                    engine.state().players[0].mana_pool.total(),
                    before - 5,
                    "{{3}}{{U}}{{U}} took exactly five mana out of the pool"
                );
                assert!(
                    !matches!(engine.pending(), Pending::GameOver(_)),
                    "paying is the other half of \"if you don't, you lose the \
                     game\", so the seat is still in it"
                );
                asked = true;
                break;
            }
            Pending::Priority { player, .. } => {
                if player == p0 {
                    tap_all_mana(&mut engine, p0);
                }
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
    assert!(asked, "the deferred pay-or-lose cost was never demanded");
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
        let mut engine = a_pact_owed_by_p0(&board, &[]);
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
                    if has_the_mana && player == p0 {
                        tap_all_mana(&mut engine, p0);
                    }
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
        assert_eq!(
            asked, has_the_mana,
            "the question is put exactly when the pool could pay"
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

/// "Players can't lose the game" (Everybody Lives!) stops a pact's "you lose
/// the game" just as it stops a life total at zero: both are the game saying
/// a player loses, and an effect saying so (CR 104.3e) is one of them (#237).
///
/// Cast in the upkeep the debt comes due in, it keeps the seat in the game on
/// both roads the pact has. With the mana there the question is put and
/// declined; with a pool that cannot cover it the question never comes. Either
/// way the debt is simply gone, because the trigger that demanded it has
/// resolved.
#[test]
fn a_pact_left_unpaid_under_everybody_lives_costs_nothing() {
    let p0 = PlayerId::new(0);
    for has_the_mana in [true, false] {
        let board = if has_the_mana {
            let mut board = vec![island(); 8];
            board.extend([forest(), plains()]);
            board
        } else {
            vec![island(), forest(), plains()]
        };
        let mut engine = a_pact_owed_by_p0(&board, &[everybody_lives()]);
        let cast_turn = engine.state().turn.number;
        let (mut protected, mut asked) = (false, false);
        for _ in 0..600 {
            if protected && engine.state().turn.step != crate::turn::Step::Upkeep {
                break;
            }
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
                    let turn = &engine.state().turn;
                    let the_upkeep_it_is_due = player == p0
                        && turn.active == p0
                        && turn.number > cast_turn
                        && turn.step == crate::turn::Step::Upkeep;
                    if the_upkeep_it_is_due && !protected {
                        tap_all_mana(&mut engine, p0);
                        cast_with_floating(&mut engine, p0, everybody_lives());
                        pass_until(&mut engine, stack_is_empty);
                        protected = true;
                        continue;
                    }
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
        assert!(protected, "the upkeep the debt is due in was never reached");
        assert_eq!(
            asked, has_the_mana,
            "the question is put exactly when the pool could pay"
        );
        assert_eq!(
            engine.state().players[0].loss,
            None,
            "players can't lose the game this turn (has the mana: {has_the_mana})"
        );
        assert!(
            !matches!(engine.pending(), Pending::GameOver(_)),
            "and the game goes on"
        );
    }
}

/// Vanishing Verse is a `{W}{B}` instant whose whole text is "Exile target
/// monocolored permanent", and "monocolored" names a **count of colours**
/// rather than a type line: an object with exactly one colour in it qualifies
/// (CR 105.2c), so a gold permanent and a colourless one are both off the menu.
///
/// The defending board therefore carries all three readings at once — a green
/// Llanowar Elves, a black-and-green Lotleth Troll and a colourless Sol Ring —
/// and the test reads both the menu and the move: the one permanent that was a
/// legal target leaves for *exile*, which a spell that had quietly resolved
/// against nothing could not show.
#[test]
fn vanishing_verse_exiles_the_monocolored_permanent_and_leaves_the_other_two_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), swamp(), swamp()])
        .hand(0, &[vanishing_verse()])
        .battlefield(1, &[llanowar_elves(), lotleth_troll(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    let troll = on_battlefield(&engine, p1, lotleth_troll()).expect("the Troll is out");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("the Sol Ring is out");

    cast_from_hand(&mut engine, p0, vanishing_verse());
    let options = pass_until_targets(&mut engine, p0);
    assert_eq!(
        options,
        vec![elf],
        "\"monocolored permanent\" on a board holding a green creature, a \
         black-and-green one and a colourless artifact: one colour is the whole menu"
    );
    assert!(
        !options.contains(&troll),
        "two colours is not one, whatever the other creature is: {options:?}"
    );
    assert!(
        !options.contains(&ring),
        "and no colour at all is not one either (CR 105.2c): {options:?}"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![troll],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the permanent the question offered is the one it exiles");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "and exile is not the graveyard, which is where a destroy would have put it"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .copied()
            .any(|id| engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))),
        "\"exile\": the card is in exile under its owner, not merely off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, vanishing_verse()).is_some(),
        "the instant itself resolved rather than being countered or fizzling"
    );
    assert!(
        on_battlefield(&engine, p1, lotleth_troll()).is_some()
            && on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the two permanents the filter declined never moved"
    );
}

/// Void Rend prints two sentences: "This spell can't be countered" and
/// "Destroy target nonland permanent". The destroy half is played on a board
/// carrying every word of the filter — a creature and an artifact across the
/// table are the nonland permanents, and the Forest beside them plus the three
/// lands paying for the spell are what "nonland" has to decline — so the menu
/// reads exactly those two and the named creature is in its owner's graveyard
/// once the spell has resolved.
///
/// The other sentence is about the spell and not about any permanent, so it is
/// read off the object the engine puts on the stack: that object and that key
/// are what a counter effect would have to read before it could do anything.
#[test]
fn void_rend_destroys_a_nonland_permanent_and_carries_its_uncounterable_clause() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[plains(), island(), swamp()])
        .hand(0, &[void_rend()])
        .battlefield(1, &[forest(), llanowar_elves(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let my_land = on_battlefield(&engine, p0, island()).expect("my Island is out");

    // {W}{U}{B} out of the Plains, the Island and the Swamp: `cast_from_hand`
    // taps the three and pays for it, and the spell stops on its target
    // question on the way in (CR 601.2c).
    cast_from_hand(&mut engine, p0, void_rend());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target nonland permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell names the target");
    assert_eq!((min, max), (1, 1), "one permanent, and the spell asks once");
    assert!(
        options.contains(&victim) && options.contains(&rock),
        "\"target nonland permanent\" reaches a creature and an artifact alike: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and it reaches nothing else on this board — neither the Forest across \
         the table nor the three lands paying for the spell: {options:?}"
    );
    assert!(
        !options.contains(&their_land) && !options.contains(&my_land),
        "\"nonland\" is the word under test: a land is a permanent and never a \
         legal target for this spell: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    // The target is named, so the spell is on the stack now — and nothing has
    // happened to the board yet.
    let spell = on_stack(&engine, void_rend()).expect("the spell is waiting on the stack");
    assert!(
        keywords(&engine, spell).contains(KeywordSet::UNCOUNTERABLE),
        "\"This spell can't be countered\" rides the object the engine put on \
         the stack, which is where a counter would have to read it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the permanent it names is untouched while the spell is merely cast"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "\"Destroy target nonland permanent\": the named creature left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and it is destroyed rather than merely moved somewhere else"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the nonland permanent the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and no land was touched, since none of them was ever on the menu"
    );
    assert!(
        in_graveyard(&engine, p0, void_rend()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );
}
