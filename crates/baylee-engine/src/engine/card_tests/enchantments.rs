//! Cards whose front face is an enchantment, the door
//! `cards/enchantments/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Storm of Saruman: the copy trigger fires on the *second* spell, not the
/// first, and the copy it makes is not itself a cast spell — otherwise each
/// copy would be another "second spell" and the trigger would never stop.
#[test]
fn storm_of_saruman_copies_only_the_second_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(22, forest())
        .battlefield(0, &[plains(), plains(), storm_of_saruman()])
        .hand(0, &[swords_to_plowshares(), swords_to_plowshares()])
        .battlefield(1, &[ondu_cleric(), earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    let cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("cleric deployed");
    let lieutenant =
        on_battlefield(&engine, p1, earth_king_s_lieutenant()).expect("lieutenant deployed");

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }

    // First spell: no trigger, so it simply resolves and exiles the cleric.
    let first = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: first })
        .unwrap();
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected the spell's target choice, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, ondu_cleric()).is_none()
    });

    // Second spell: the trigger copies it, and the copy may be retargeted.
    let second = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .unwrap();
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected the spell's target choice, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();

    let offered = options_offered_including(&mut engine, lieutenant);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![offered[0]],
            },
        )
        .unwrap();

    // The copy resolves and the trigger does not fire again: a copy is put on
    // the stack, never cast, so it is not a third spell.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, earth_king_s_lieutenant()).is_none()
    });
}

/// Wizard Class ({U}, Class): `{2}{U}: Level 2`, and "when this Class
/// becomes level 2, draw two cards".
///
/// The level-up is an `ActivatedConditional` — it may only be activated at
/// level 1 — and that is the whole point of the test. The condition is a
/// restriction on *activating* the ability (CR 602.5), spent the moment the
/// activation is allowed to begin; what goes on the stack afterwards is an
/// ordinary ability. The engine's resolver did not know that: its match over
/// the ability being resolved listed `Activated` and four others and left
/// `ActivatedConditional` out, so levelling a Class put an ability on the
/// stack that panicked the process as it resolved. Two implemented cards
/// print one of these — this and Luminarch Ascension — and neither had ever
/// had its ability resolved by a test.
#[test]
fn a_class_can_be_levelled_and_the_level_up_resolves() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(919, forest())
        .battlefield(0, &[wizard_class(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is on the table");
    tap_mana_except(&mut engine, p0, class);
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: class,
                ability_index: 1,
            },
        )
        .expect("level 2 is affordable at level 1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(class)
            .expect("the Class survived its own ability")
            .counters
            .get(CounterKind::Level),
        1,
        "one level counter, so the Class is level 2"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 2,
        "becoming level 2 drew two cards"
    );
}

/// Luminarch Ascension: a "may" *inside* an intervening-if clause
/// (CR 603.4), which is the shape that put a suspending effect inside a
/// nested branch for the first time.
///
/// The assertion that matters is the count. Running the branch a second
/// time on resume would have asked twice and taken two quest counters —
/// four end steps would have finished the card in two — and nothing in the
/// engine would have complained.
#[test]
fn an_optional_clause_inside_a_condition_is_offered_once_and_taken_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, plains())
        .battlefield(0, &[luminarch_ascension()])
        .start();
    keep_mulligans(&mut engine);
    let ascension =
        on_battlefield(&engine, p0, luminarch_ascension()).expect("the enchantment is out");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(ascension)
            .expect("still on the battlefield")
            .counters
            .get(crate::object::CounterKind::Custom(1)),
        1,
        "one offer, one counter"
    );
}

// oracle_id = "119d719d-e965-45b4-9bc9-ac03211b10c2"
fn survival_of_the_fittest() -> baylee_core::ids::CardIndex {
    card_index("119d719d-e965-45b4-9bc9-ac03211b10c2")
}

/// Survival of the Fittest ({1}{G}): "{G}, Discard a creature card: Search
/// your library for a creature card, reveal that card, put it into your
/// hand, then shuffle." A cost that says "a creature card" names none, so
/// the engine has to ask before it may be paid (CR 601.2h) — and this is
/// the board that drives `ChoicePrompt::CostDiscard` and the hand-only
/// option list behind it.
///
/// Reading the card cannot replace playing it, because the card is the half
/// that does not say who may be asked for what. Its filter is a bare
/// `Filter::CREATURE` with no "you control" in it at all, and a hand is a
/// hidden zone the printed line says nothing about; the rule that a player
/// discards from their own hand is CR 701.9a and lives in `cost_wizard`.
/// So three bystanders stand beside the Elves to say what the list is not:
/// a Forest and a Counterspell in the same hand, which are cards but not
/// creature cards, and a Llanowar Elves in the **opponent's** hand, which is
/// a creature card but not one this player may discard. Reading the filter
/// alone over every hand at the table would have offered that last one.
///
/// The prompt is asserted beside the options, because the variant is what
/// tells a client this is a cost and not a search: the same ability publishes
/// a second `Pending::ChooseCards` two steps later carrying
/// `ChoicePrompt::SearchLibrary`, and a test that ignored the field would
/// pass with the two swapped. `ChooseCards` and not `ChooseTargets` is the
/// other half of that: choosing what to discard is not targeting (CR 115.1).
///
/// What the old test pinned — an ability the engine offered to nobody — is
/// gone. The ability is on the list, the question comes before anything is
/// paid, and the green mana and the chosen card are both spent when it is
/// answered.
#[allow(clippy::too_many_lines)] // one activation, from the offer to what the search found
#[test]
fn survival_of_the_fittest_asks_which_creature_card_to_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The library is creature cards, so the search the cost pays for has
    // something to find — and a different creature from the one in hand, so
    // the card that paid and the card that was found cannot be confused.
    let mut engine = Duel::new(41, ondu_cleric())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(
            0,
            &[
                survival_of_the_fittest(),
                llanowar_elves(),
                forest(),
                counterspell(),
            ],
        )
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests pay the {1}{G}. The third is kept back on purpose: the
    // ability's own mana has to be *in the pool* when the offer is read,
    // because `can_afford` asks the pool and never what the board could still
    // tap.
    let forests = mine(&engine, p0, forest(), crate::zone::Zone::Battlefield);
    assert_eq!(forests.len(), 3, "three Forests were dealt to seat 0");
    let spare = forests[2];
    tap_mana_except(&mut engine, p0, spare);
    let spell =
        in_hand(&engine, p0, survival_of_the_fittest()).expect("the enchantment is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("a two-mana enchantment off two Forests");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let survival = on_battlefield(&engine, p0, survival_of_the_fittest())
        .expect("Survival of the Fittest resolved onto the battlefield");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: spare })
        .expect("the Forest that was kept back still taps");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the green mana the ability asks for is in the pool"
    );

    // The one creature card in hand, and the three cards that are not it.
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("a creature card is in hand");
    let land_in_hand = in_hand(&engine, p0, forest()).expect("a land card is in hand");
    let instant_in_hand = in_hand(&engine, p0, counterspell()).expect("an instant is in hand");
    let theirs = in_hand(&engine, p1, llanowar_elves()).expect("seat 1 holds a creature card too");

    // The offer the old gap withheld: a cost that has to ask is now asked of
    // the board instead of refused outright.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(survival, 0)),
        "the enchantment's own ability is on no list: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: survival,
                ability_index: 0,
            },
        )
        .expect("the activation the offer promised");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "a cost that names a card to choose has to ask, and got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "a player discards from their own hand");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature card, no more and no fewer"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "the prompt is what tells a client this is a cost and not a search"
    );
    assert!(
        options.contains(&elves),
        "the creature card in hand pays this cost, and was not offered: {options:?}"
    );
    assert!(
        !options.contains(&land_in_hand),
        "a Forest in hand is a card and not a creature card"
    );
    assert!(
        !options.contains(&instant_in_hand),
        "and neither is a Counterspell"
    );
    assert!(
        !options.contains(&theirs),
        "the opponent's creature card is in the opponent's hand"
    );
    // Seat 0 takes the first turn and skips its draw (CR 103.8), so the hand
    // is exactly what the duel dealt, less the enchantment it cast.
    assert_eq!(options.len(), 1, "and nothing else at all: {options:?}");

    // The question comes before the payment (CR 601.2h): the mana is still in
    // the pool and the card is still in hand, so nobody is made to choose
    // what to give up for an activation that cannot happen.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the mana half of the cost is unspent while the question stands"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves),
        "and so is the card the question is about"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the answer comes off the list the engine published");
    assert_eq!(
        in_graveyard(&engine, p0, llanowar_elves()),
        Some(elves),
        "the discarded card is in its owner's graveyard"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves),
        "and it left the hand to get there"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "the mana half of the cost was paid with it"
    );

    // What the cost bought. The same ability asks again, and the second
    // question carries the prompt a search carries.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the search")
    };
    let found = *options.first().expect("the library holds creature cards");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a creature card off the list the search published");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&found),
        "the card the search found is in hand"
    );
    assert!(
        engine
            .state()
            .object(found)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE)),
        "and it is a creature card, which is all the ability may take"
    );
}

// oracle_id = "3a3e8c9b-e458-4661-980d-0a84a4c2452b"

/// Glasswing Grace // Age-Graced Chapel ({3}{W/B}{W/B}, MH3): a modal
/// double-faced card whose front is an Aura and whose back, Age-Graced
/// Chapel, is a land that enters tapped and taps for {W} or {B}.
fn glasswing_grace() -> CardIndex {
    card_index("3a3e8c9b-e458-4661-980d-0a84a4c2452b")
}

/// Casts the Aura on seat 0's Llanowar Elves and hands back the board it
/// left: the engine, seat 0's enchanted Elves, seat 1's untouched Elves and
/// the Aura permanent. A Swords to Plowshares is left in hand for the caller
/// that wants the host gone.
///
/// The second Elves is the control the whole thing rests on. "Enchanted
/// creature gets +2/+2" is one `Filter::AttachedToBySource`, and a filter that
/// had come out as "every creature" would read exactly the same on a board
/// with only one creature on it.
#[track_caller]
fn a_glasswing_on_the_elves() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(517, forest())
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
        .hand(0, &[glasswing_grace(), swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elves deployed");
    assert_eq!(pt(&engine, mine), (1, 1), "Llanowar Elves prints 1/1");

    reach_main_phase(&mut engine, p0);
    // Five Plains, and the Elves is not among the mana `cast_from_hand` taps:
    // `legal.mana_abilities` is the CR 305.6 shortcut, which only a land with
    // a basic land type is on. So the pool is five white against
    // {3}{W/B}{W/B}, and `mana_pay::pay` spends the hybrids before the
    // generic — white twice, then three more for the {3}.
    cast_from_hand(&mut engine, p0, glasswing_grace());

    // "Enchant creature" (CR 303.4a): an Aura spell requires a target, which
    // its enchant ability defines, so the question is asked as it is cast and
    // not on the way onto the battlefield. The Chapel is never offered
    // beside it: a land back face is played and never cast
    // (`casting::castable_back_faces` drops it), so the wizard has one option
    // and asks no mode at all.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura spell targets as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "any creature may be enchanted, and the board has two: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| at_rest(e, p0));
    let aura =
        on_battlefield(&engine, p0, glasswing_grace()).expect("the Aura resolved onto the table");
    (engine, mine, theirs, aura)
}

/// "Enchanted creature gets +2/+2 and has flying and lifelink." One printed
/// sentence and two layers — CR 613.1 applies layer 6 (the keywords) before
/// layer 7c (the P/T change) — so the only reading that can see both at once
/// is the creature's projected characteristics, taken after the layers ran.
///
/// It also pins where the effect lands. The Aura arrives attached to the
/// creature its spell targeted, the grant follows that attachment, and
/// neither half reaches the identical Elves the opponent controls.
#[test]
fn glasswing_grace_gives_the_creature_it_enchants_plus_two_two_flying_and_lifelink() {
    let (engine, mine, theirs, aura) = a_glasswing_on_the_elves();

    assert_eq!(
        engine
            .state()
            .object(aura)
            .and_then(|o| o.attached_to)
            .expect("the Aura is attached to something"),
        mine,
        "an Aura enters attached to the creature its spell targeted"
    );
    assert_eq!(pt(&engine, mine), (3, 3), "+2/+2 on a 1/1");
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING),
        "the enchanted creature has flying"
    );
    assert!(
        kw.contains(KeywordSet::LIFELINK),
        "the enchanted creature has lifelink"
    );

    // The Aura grants; it does not keep.
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::FLYING),
        "the Aura itself is no flier"
    );
    // And the creature it is not attached to is the card it always was.
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the opponent's Elves is enchanted by nothing"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and picked up no keyword either"
    );
}

/// CR 704.5m: an Aura attached to an illegal permanent — or to nothing at all
/// — is put into its owner's graveyard as a state-based action.
///
/// Exiling the host is the cleanest way to ask it, because nothing else in
/// the scenario touches the Aura: it is on the battlefield, its creature
/// leaves, and the only rule that can move it is that one. An Aura that
/// stayed would go on granting +2/+2 to an object that is no longer there.
#[test]
fn a_glasswing_grace_falls_into_the_graveyard_when_its_creature_is_exiled() {
    let p0 = PlayerId::new(0);
    let (mut engine, mine, _theirs, aura) = a_glasswing_on_the_elves();

    // Back to an own first main phase — the phase is named rather than left
    // to `at_rest`, which is already true in turn three's upkeep — where five
    // Plains have untapped and the Swords left in hand is castable.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && at_rest(e, p0)
    });
    assert!(
        engine
            .state()
            .object(aura)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the Aura is still on the battlefield before the removal"
    );

    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&mine),
        "the enchanted creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        engine
            .state()
            .object(mine)
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "the host was exiled"
    );
    assert!(
        on_battlefield(&engine, p0, glasswing_grace()).is_none(),
        "the Aura did not stay on the battlefield with nothing to enchant"
    );
    assert!(
        in_graveyard(&engine, p0, glasswing_grace()).is_some(),
        "an Aura enchanting nothing goes to its owner's graveyard (CR 704.5m)"
    );
}

/// The back face. "Age-Graced Chapel — Land. This land enters tapped.
/// {T}: Add {W} or {B}." CR 712.12: a player playing a modal double-faced
/// card as a land chooses one of its faces that's a land — it is *played*
/// as a land drop rather than cast, and what arrives is that land, with
/// none of the Aura's printed statics on it.
///
/// The pool-wide land sweep cannot speak for this card: `land_mana_tests`
/// sweeps only the faces that arrive untapped and counts the rest, and this
/// one is exactly that.
#[test]
fn age_graced_chapel_is_played_as_a_tapped_land_that_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let (mut engine, chapel) =
        play_land_face(glasswing_grace(), 1).expect("the back face is a land and can be played");

    let kinds = types(&engine, chapel);
    assert!(
        kinds.contains(TypeSet::LAND),
        "the face that was played is the land"
    );
    assert!(
        !kinds.contains(TypeSet::ENCHANTMENT),
        "and it is not the Aura on the other side"
    );
    assert!(
        entered_tapped(&engine, chapel),
        "\"This land enters tapped.\""
    );

    // Its own untap step gives it back, which is the first moment the {T} in
    // the ability's cost is payable at all.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && at_rest(e, p0)
    });
    assert!(
        !is_tapped(&engine, chapel),
        "the untap step untapped the Chapel"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&chapel),
        "the Chapel prints no basic land type, so there is no CR 305.6 shortcut"
    );
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(who, _)| *who == chapel)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        vec![0],
        "the land face offers its own mana ability and nothing the Aura printed"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: chapel,
                ability_index: 0,
            },
        )
        .expect("an untapped land activates its own {T} ability");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" asks which, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Black],
        "the two colours the Chapel prints, and only those"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black came out of its own list");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "one black mana in the pool"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and nothing of the colour that was not chosen"
    );
    assert!(
        is_tapped(&engine, chapel),
        "the {{T}} in the ability's cost"
    );
}

// oracle_id = "1a8c996d-ca93-4c17-ace5-66ecd6b99317"

/// Strength of the Harvest // Haven of the Harvest ({2}{G/W}) — a modal
/// double-faced card whose front is an Aura and whose back is a land.
fn strength_of_the_harvest() -> CardIndex {
    card_index("1a8c996d-ca93-4c17-ace5-66ecd6b99317")
}

/// "Enchant creature. Enchanted creature gets +1/+1 for each creature
/// and/or enchantment you control."
///
/// Three printed claims, and the board is built so that each one moves a
/// number the others cannot. The Aura spell *targets* as it is cast — an
/// Aura spell requires a target, defined by its enchant ability
/// (CR 303.4a) — and the offer is asserted as a whole rather than searched:
/// four Forests stand beside the one creature, so "enchant **creature**"
/// is the difference between one option and five.
///
/// The permanent then arrives attached to what it chose, which is read off
/// `attached_to` and confirmed a second way by the Aura still being on the
/// battlefield at all: an Aura attached to nothing is put into its owner's
/// graveyard by a state-based action (CR 704.5m).
///
/// The count is the half that says which two types are read. A Llanowar
/// Elves alone would leave a 2/2 whether the Aura counted enchantments or
/// not; the Aura **is** an enchantment its controller controls, so it counts
/// itself and the Elf comes out a 3/3. Four Forests are on the table and
/// none of them counts, which is what "creature and/or enchantment" excludes
/// — counting every permanent would read 7/7 here.
///
/// And casting a second Elf afterwards is what makes it "for each" rather
/// than a number fixed as the Aura entered: the projection is read again and
/// the enchanted creature goes to 4/4, while the newcomer — which the Aura
/// is not attached to — stays the 1/1 it was printed as.
#[test]
fn the_harvest_aura_swells_only_its_own_creature_and_recounts_the_board_each_time() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[strength_of_the_harvest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // Crosses a turn boundary if the seed puts p0 on the draw, which
    // `reach_main_phase` cannot: the combat on the way asks for attackers.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a Llanowar Elves is a 1/1 before anything enchants it"
    );

    // Four Forests: three pay {2}{G/W} — the hybrid takes green — and the
    // fourth stays floating for the second Elf, which has to be cast in this
    // same main phase because a pool empties when the step ends (CR 500.5).
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, strength_of_the_harvest()).expect("the Aura is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("{2}{G/W} off four Forests");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura spell requires a target defined by its enchant ability \
             (CR 303.4a) — got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        [elf],
        "\"enchant creature\" reaches the one creature on the table and none \
         of the four Forests standing beside it"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, strength_of_the_harvest())
        .expect("the Aura resolved onto the battlefield and stayed there");
    assert_eq!(
        engine
            .state()
            .object(aura)
            .and_then(|o| o.attached_to)
            .expect("the Aura is attached to something"),
        elf,
        "an Aura enters the battlefield attached to the object its spell \
         targeted; attached to nothing it would already be in its owner's \
         graveyard (CR 704.5m)"
    );
    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "one creature (the Elf) and one enchantment (the Aura itself) — and \
         not one of the four Forests"
    );

    let second = in_hand(&engine, p0, llanowar_elves()).expect("the second Elf is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .expect("{G} off the Forest left floating");
    pass_until(&mut engine, stack_is_empty);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "the second Elf resolved");
    assert_eq!(
        pt(&engine, elf),
        (4, 4),
        "\"for each\" is read on every projection, so a creature arriving \
         after the Aura counts too"
    );
    let newcomer = elves
        .into_iter()
        .find(|id| *id != elf)
        .expect("the one that is not the enchanted Elf");
    assert_eq!(
        pt(&engine, newcomer),
        (1, 1),
        "only the *enchanted* creature gets the bonus — the filter is the \
         Aura's own host, not every creature you control"
    );
}

/// "Haven of the Harvest — Land. This land enters tapped. {T}: Add {G} or
/// {W}."
///
/// A player playing a modal double-faced card as a land chooses one of its
/// faces that's a land before putting it onto the battlefield, and it enters
/// with that face up (CR 712.12). Only the back face of this card is a land,
/// so there is exactly one choice to make and the engine makes it: the card
/// arrives as face 1, a Land with none of the Aura's text on it.
///
/// It has to be a real land drop rather than a seeded battlefield, because
/// "this land enters tapped" is a replacement effect and a permanent placed
/// on the board never enters at all — the tapped assertion would pass on a
/// card that printed nothing of the kind.
///
/// Then the mana ability, which is the rest of the face: the land untaps on
/// its controller's next turn, and what it offers is asked of the offer
/// first — one printed ability and no CR 305.6 shortcut, because the Haven
/// prints no basic land type — and then of the pool, which ends up holding
/// one mana of the colour chosen and none of the other. "{G} **or** {W}" is
/// the claim that a single {T} producing both would pass.
#[test]
fn the_harvest_land_face_enters_tapped_and_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) = play_land_face(strength_of_the_harvest(), 1)
        .unwrap_or_else(|why| panic!("Haven of the Harvest {why}"));

    let printed = types(&engine, land);
    assert!(
        printed.contains(TypeSet::LAND),
        "the face that was played is the Land: {printed:?}"
    );
    assert!(
        !printed.contains(TypeSet::ENCHANTMENT),
        "and it carries none of the Aura face's types: {printed:?}"
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped.\""
    );

    // Its own untap step is the first moment it can be tapped for mana.
    pass_until(&mut engine, |e| !is_tapped(e, land));
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&land),
        "the Haven prints no basic land type, so nothing offers it the \
         intrinsic tap of CR 305.6"
    );
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(who, _)| *who == land)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        [0],
        "the land face offers its own printed mana ability and nothing the \
         Aura face wrote"
    );

    activate(&mut engine, p0, strength_of_the_harvest(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a choice of two colours — got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        [ManaColor::Green, ManaColor::White],
        "the two the card prints, in the order it prints them, and no third"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white is one of the two offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "one white mana in the pool, which is what the ability produces"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and none of the colour that was not chosen — the card prints \
         \"or\", not both"
    );
    assert!(
        is_tapped(&engine, land),
        "and the {{T}} in the ability's cost spent the land"
    );
}

// oracle_id = "5d47e820-913f-441a-a6cc-37ab3181d79a"
fn swift_reconfiguration() -> CardIndex {
    card_index("5d47e820-913f-441a-a6cc-37ab3181d79a")
}

/// `Artifact — Vehicle`, 3/3, and — uncrewed — no creature at all.
///
/// It is on this board for the half of "enchant creature or Vehicle" that a
/// creature cannot stand for: a Vehicle is a legal host precisely because it
/// is *not* a creature, so a filter that read the printed line as "enchant
/// creature" would still pass every other assertion here.
fn smugglers_copter() -> CardIndex {
    card_index("49136bdc-bc50-49a2-999a-1ef9c16ea130")
}

/// Swift Reconfiguration ({W}, Aura): "Flash. Enchant creature or Vehicle.
/// Enchanted permanent is a Vehicle artifact with crew 5 and it loses all
/// other card types."
///
/// The half that works, played the way the card is played: held through the
/// opponent's turn and flashed onto one of their creatures in their own main
/// phase, which is a cast no sorcery-speed Aura could make (CR 702.8a). The
/// second sentence is struck on the offer rather than on the answer, on all
/// three of its words: the two Elves are on the list, the uncrewed Smuggler's
/// Copter is on it because a *Vehicle* is the other half and not because it
/// is a creature — it is not one — and the Plains standing beside them is on
/// neither, which is the difference between "enchant creature or Vehicle"
/// and "enchant permanent". What the third sentence then does is read off the layer
/// system — layer 4, where the artifact type is added, Vehicle is added as a
/// subtype and every other card type is taken away (CR 613.1d) — and off
/// combat, because an uncrewed Vehicle is not a creature and CR 508.1a only
/// ever declares creatures as attackers.
///
/// The second Elf is the control and carries the whole assertion: both of
/// them start the turn identical and only one is enchanted, so "the engine
/// did not offer it" cannot be a summoning-sick, tapped or otherwise
/// uninteresting board. One is offered as an attacker and the other is not.
#[test]
#[allow(clippy::too_many_lines)] // one scenario, read in order: cast, resolve, combat
fn a_flashed_reconfiguration_makes_an_attacker_a_vehicle_that_cannot_be_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[plains(), smugglers_copter()])
        .hand(0, &[swift_reconfiguration()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let my_plains = on_battlefield(&engine, p0, plains()).expect("one Plains, and it pays {W}");
    let copter = on_battlefield(&engine, p0, smugglers_copter()).expect("an uncrewed Vehicle");
    let elves = mine(&engine, p1, llanowar_elves(), Zone::Battlefield);
    assert_eq!(elves.len(), 2, "two Elves, one enchanted and one not");
    let (enchanted, bystander) = (elves[0], elves[1]);
    assert!(
        types(&engine, enchanted).contains(TypeSet::CREATURE)
            && !types(&engine, enchanted).contains(TypeSet::ARTIFACT),
        "it begins the turn as a plain creature"
    );

    // Their turn, their main phase, and the Aura is still in hand: that is
    // the only window in which flash is the reason it can be cast at all.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let card = in_hand(&engine, p0, swift_reconfiguration()).expect("held through their turn");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "flash puts the Aura on offer while the other seat is the active player"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("one Plains pays {W}");

    // "Enchant creature or Vehicle": the Aura picks its host as it is cast,
    // and a land is on neither half of that.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura chooses what it enchants as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&enchanted) && options.contains(&bystander),
        "both creatures are legal hosts: {options:?}"
    );
    assert!(
        options.contains(&copter),
        "and so is the Vehicle, which is the half of the line no creature can \
         stand for — it is on the list because it is a Vehicle and not \
         because it is a creature, which it is not: {options:?}"
    );
    assert!(
        !options.contains(&my_plains),
        "while the land is on neither half, which is the word the printed \
         line does not say: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchanted],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, swift_reconfiguration()).is_some()
    });

    let aura = on_battlefield(&engine, p0, swift_reconfiguration()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(enchanted),
        "and arrived attached to the creature it targeted"
    );
    let now = types(&engine, enchanted);
    assert!(
        now.contains(TypeSet::ARTIFACT) && !now.contains(TypeSet::CREATURE),
        "it is an artifact and has lost every other card type: {now:?}"
    );
    assert!(
        engine
            .state()
            .object(enchanted)
            .expect("the host is still on the battlefield")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::artifact::VEHICLE),
        "and a Vehicle"
    );

    // CR 508.1a declares creatures, so an uncrewed Vehicle is not on the list
    // — while the Elf beside it, identical in every other way, is.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(enchanted),
        "and it is still attached after the state-based actions have run, \
         with its host no longer a creature (CR 704.5m)"
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attacker declaration")
    };
    assert!(
        attackers.contains(&bystander),
        "the untouched Elf attacks, so the board itself is not the reason"
    );
    assert!(
        !attackers.contains(&enchanted),
        "the enchanted one is no creature and may not be declared: {attackers:?}"
    );
}

/// The two halves the card's `Coverage::Partial` is about, on a board built
/// so that both of them are visible at once.
///
/// **`with crew 5` is not written.** Five untapped Llanowar Elves stand
/// beside the enchanted permanent — total power exactly 5, which is what the
/// printed crew cost asks for — and the offer the engine makes on that
/// permanent is *exactly* its host's own printed "`{T}`: Add `{G}`", at index
/// 0. An equality and not an emptiness, because an emptiness here would have
/// been wrong rather than weak: a **printed** mana ability is enumerated into
/// `LegalActions::abilities` like any other activated ability —
/// `mana_abilities` is the CR 305.6 land shortcut plus what a continuous
/// effect *granted* — so the Elf under the Aura was never going to offer
/// nothing. The equality keeps what the emptiness was reaching for (the
/// engine is looking at this object) and still fails the moment a second
/// entry appears. The card's own `NOT SUPPORTED` note says why none does: no
/// `CostPart` chooses a set of other creatures and reads a total power off
/// it, so nothing grants a crew ability in the first place. Crew would arrive
/// through `Modifier::GrantActivated`, offered at a `choice::granted_ability`
/// index beside the printed one — offered, which is to say *payable*, since
/// `can_afford` gates the offer. That is what the five untapped Elves are
/// for: with total power exactly 5 standing by, the day crew is written is
/// the day it is affordable, and this assertion is red.
///
/// **A card type does not take its subtypes with it.** `Modifier::RemoveType`
/// clears bits in the projected `types` and nothing subtracts a subtype, so
/// the Elf Druid under the Aura projects as `Artifact — Elf Druid Vehicle`.
/// It is not a creature, which is what every rule this card reaches asks
/// first, but a count of Elves would still find it.
#[test]
#[allow(clippy::too_many_lines)] // both halves of `Coverage::Partial` on one board
fn a_reconfigured_elf_is_a_vehicle_nobody_can_crew_and_keeps_its_creature_types() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(62, forest())
        .battlefield(
            0,
            &[
                plains(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[swift_reconfiguration()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = mine(&engine, p0, llanowar_elves(), Zone::Battlefield);
    assert_eq!(elves.len(), 6, "one to enchant and five to crew with");
    let (host, crew) = (elves[0], &elves[1..]);

    // Only the Plains pays: the Elves have to stay untapped, because an
    // already-tapped crew would be a second reason for the offer to be absent.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let card = in_hand(&engine, p0, swift_reconfiguration()).expect("the Aura is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("one Plains pays {W}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the Aura's host choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&host), "my own Elf is a legal host");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let now = types(&engine, host);
    assert!(
        now.contains(TypeSet::ARTIFACT) && !now.contains(TypeSet::CREATURE),
        "the Aura resolved and rewrote what the permanent is: {now:?}"
    );

    // The board could pay crew 5 twice over if there were a crew cost to pay.
    assert!(
        crew.iter().all(|id| !is_tapped(&engine, *id)),
        "every creature that would crew it is untapped"
    );
    let total: i16 = crew.iter().map(|id| pt(&engine, *id).0).sum();
    assert_eq!(total, 5, "five untapped 1/1s: total power exactly 5");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(who, _)| *who == host)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        [0_u32],
        "the permanent still offers exactly one thing, and it is the Elf's \
         own printed ability at index 0 — `{{T}}: Add {{G}}`, which a printed \
         mana ability is listed under here rather than in `mana_abilities`. \
         `with crew 5` is the card's NOT SUPPORTED clause, so no second entry \
         at a `choice::granted_ability` index stands beside it"
    );
    assert!(
        !types(&engine, host).contains(TypeSet::CREATURE),
        "and with nothing to crew it, it never becomes a creature again"
    );

    // The other half of `Coverage::Partial`: losing the card type leaves the
    // creature types behind.
    let c = engine
        .state()
        .object(host)
        .expect("the host is still on the battlefield")
        .characteristics();
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::artifact::VEHICLE),
        "Vehicle was added"
    );
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::ELF)
            && c.subtypes
                .contains(baylee_core::generated::subtypes::creature::DRUID),
        "and Elf and Druid were not taken away with the creature type: no \
         `Modifier` subtracts a subtype. When this fires, the clause has \
         become expressible and the card is no longer Coverage::Partial"
    );
}

// oracle_id = "50aa7aff-1f01-4224-9a83-01f74d703ec2"
fn earthcraft() -> baylee_core::ids::CardIndex {
    card_index("50aa7aff-1f01-4224-9a83-01f74d703ec2")
}

/// Earthcraft ({1}{G}): "Tap an untapped creature you control: Untap target
/// basic land."
///
/// The pool's first `CostPart::TapOther`, and the one asking cost whose
/// answer is still on the battlefield afterwards. That is the half a reader
/// of the card cannot settle and the reason the cost has its own prompt: a
/// player shown `ChoicePrompt::CostSacrifice` over their own creatures would
/// decline a cost that only taps one.
///
/// Both enumerations are struck, and the menu is asserted by exact equality
/// rather than by what it contains. The tapped Elf is mine and is refused by
/// CR 118.3 — a permanent already tapped cannot be tapped to pay a cost,
/// whether or not the card thought to say "untapped". The opponent's Elf is
/// untapped and is refused because no rule makes a cost reach across the
/// table, so `cost_wizard::options` draws that line itself. The Earthcraft
/// is neither, and is on nobody's menu.
///
/// The creature that *does* pay arrived this turn, which is the claim the
/// card is silent about and the rules are not: CR 302.6 restricts a
/// creature's own `{T}` ability and says nothing about it being tapped to
/// pay for somebody else's, so a Bird cast this turn can already work the
/// land. Reading `cost_wizard` alone would not settle it — `can_afford`'s
/// `TapSelf` arm *does* ask about summoning sickness, one arm away.
///
/// The target list is the other half. "Target basic land" carries no "you
/// control", so the opponent's Forest is on it and the Badlands beside it is
/// not — a dual land is no basic land however many basic types it has.
#[allow(clippy::too_many_lines)] // one activation, two enumerations, and a tap that is not a sacrifice
#[test]
fn earthcraft_taps_a_summoning_sick_creature_to_untap_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[earthcraft(), forest(), llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), forest(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let craft = on_battlefield(&engine, p0, earthcraft()).expect("the Earthcraft is out");
    let veteran = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf that was here");
    let land = on_battlefield(&engine, p0, forest()).expect("my Forest");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let their_dual = on_battlefield(&engine, p1, badlands()).expect("their Badlands");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");

    // The older Elf is tapped by hand and the Forest by the cast, which is
    // what puts a tapped creature of my own on the board to be refused and
    // leaves the newcomer as the only untapped one. By hand because a
    // printed `mana_ability!` is an ordinary `(source, index)` in
    // `legal.abilities` — `legal.mana_abilities` is the CR 305.6 shortcut,
    // lands and granted abilities, and `cast_from_hand` walks only that.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: veteran,
                ability_index: 0,
            },
        )
        .expect("the Elf taps for {G}");
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let rookie = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            *id != veteran
                && engine.state().object(*id).is_some_and(|o| {
                    o.controller == p0 && o.card.is_some_and(|c| c.index == llanowar_elves())
                })
        })
        .expect("the Elf cast this turn is on the battlefield");
    assert!(
        is_tapped(&engine, veteran),
        "the older Elf paid for the cast"
    );
    assert!(is_tapped(&engine, land), "and so did my Forest");
    assert!(!is_tapped(&engine, rookie), "the newcomer arrived untapped");

    activate(&mut engine, p0, earthcraft(), 0);

    // CR 601.2c: targets first, and "basic land" says nothing about whose.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the untap asks which land: {:?}", engine.pending())
    };
    assert!(
        options.contains(&land) && options.contains(&their_land),
        "either side of the table prints a basic land: {options:?}"
    );
    assert!(
        !options.contains(&their_dual),
        "a dual land is no basic land, whatever types it has: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: Vec::new(),
            },
        )
        .expect("my own tapped Forest is one of the legal targets");

    // CR 601.2h, and the one step of it the player takes.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which creature to tap: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "not `CostSacrifice`: the creature named here is still standing \
         afterwards, and the two questions cannot share a word"
    );
    assert_eq!((min, max), (1, 1), "one creature, and exactly one");
    assert_eq!(
        options,
        vec![rookie],
        "the only untapped creature this seat controls. The older Elf \
         {veteran:?} is tapped (CR 118.3), the Earthcraft {craft:?} is no \
         creature, and the Elf {their_elf:?} across the table is not mine \
         to tap"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rookie],
            },
        )
        .expect("a creature that arrived this turn may still be tapped for a cost");

    assert!(
        is_tapped(&engine, rookie),
        "the cost is paid, so the creature named is tapped"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some()
            && engine.state().object(rookie).is_some(),
        "and it is still on the battlefield: a tap is not a sacrifice"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is not a mana ability (CR 605.1), so it uses the stack"
    );
    assert!(
        is_tapped(&engine, land),
        "and nothing has untapped yet — the effect happens on resolution"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, land),
        "the targeted basic land is untapped"
    );
    assert!(
        is_tapped(&engine, rookie),
        "and the creature that paid stays tapped: the cost is not refunded"
    );
}
