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
    let cleric = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
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
    let cs = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p1))[0];
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
            .list(crate::zone::ZoneLocation::Graveyard(p0))
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
    let ritual = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p1))[0];
    engine
        .apply(p1, PlayerAction::CastSpell { card: ritual })
        .unwrap();

    // p0 answers that: tap the Island, counter the Ritual.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    let offer = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
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
            .list(crate::zone::ZoneLocation::Battlefield)
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
        Some(crate::zone::Zone::Exile),
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
            .list(crate::zone::ZoneLocation::Library(p0))
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
        types(&engine, land).intersects(baylee_core::types::TypeSet::ARTIFACT),
        "a permanent still is an artifact — the card's own rules text"
    );
    assert!(types(&engine, lattice).intersects(baylee_core::types::TypeSet::ARTIFACT));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let bolt = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    // Sampled with the spell still on the stack and the layer pass behind
    // it — p0 has passed, p1 holds priority. "All permanents" does not reach
    // a spell, and this is the reading `finalize_spell` goes on to make.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        1,
        "the spell should still be on the stack here"
    );
    assert!(
        !types(&engine, bolt).intersects(baylee_core::types::TypeSet::ARTIFACT),
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
    let tutor = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
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
        types(&engine, land).intersects(baylee_core::types::TypeSet::ARTIFACT),
        "a permanent is still an artifact"
    );
    for &id in engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
    {
        assert!(
            !types(&engine, id).intersects(baylee_core::types::TypeSet::ARTIFACT),
            "a card in the library is not a permanent and gains nothing"
        );
    }
}
