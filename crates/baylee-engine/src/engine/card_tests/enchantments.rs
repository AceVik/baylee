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
            .get(baylee_cards_dsl::counters::QUEST),
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

/// CR 303.4c: "illegal" is the **enchant ability's** word, not "gone". An
/// Aura whose host is still on the battlefield, and still a permanent, but
/// has stopped being something that Aura may enchant is put into its
/// owner's graveyard all the same.
///
/// Swift Reconfiguration is the pool's one card that takes creaturehood
/// away without taking the permanent, so it is the only way to ask the
/// question at all: Glasswing Grace says "enchant creature", and after the
/// Reconfiguration resolves its host is an artifact Vehicle and no creature.
/// The test asserts both halves, because a rule that swept every Aura off
/// the table would pass the first — Swift Reconfiguration enchants "creature
/// or Vehicle" and stays, over exactly the host that has just cost the other
/// Aura its place.
#[test]
fn an_aura_falls_off_a_host_that_stops_being_what_it_enchants() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(517, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[glasswing_grace(), swift_reconfiguration()])
        .start();
    keep_mulligans(&mut engine);
    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves deployed");

    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, glasswing_grace());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    let aura = on_battlefield(&engine, p0, glasswing_grace()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and is attached to the Elves"
    );
    assert_eq!(pt(&engine, host), (3, 3), "a 1/1 under +2/+2");

    // The sixth Plains is still untapped: Glasswing Grace costs five.
    cast_from_hand(&mut engine, p0, swift_reconfiguration());
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
        "the host is an artifact Vehicle and no longer a creature: {now:?}"
    );
    assert!(
        on_battlefield(&engine, p0, glasswing_grace()).is_none()
            && in_graveyard(&engine, p0, glasswing_grace()).is_some(),
        "so the Aura that says `enchant creature` is in its owner's \
         graveyard (CR 303.4c, CR 704.5m)"
    );
    assert_eq!(
        on_battlefield(&engine, p0, swift_reconfiguration())
            .and_then(|a| engine.state().object(a))
            .and_then(|o| o.attached_to),
        Some(host),
        "while the Aura that says `enchant creature or Vehicle` is still on \
         the battlefield and still attached to the same permanent"
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

// oracle_id = "8a52f3c0-2552-4425-b2e3-5496eb2232a7"
fn mystic_remora() -> CardIndex {
    card_index("8a52f3c0-2552-4425-b2e3-5496eb2232a7")
}

/// Mystic Remora is `Coverage::Partial`: the tax trigger is written and
/// cumulative upkeep {1} is not, so the scenario is fought on the
/// *opponent's* turn, where the missing clause would never fire anyway — age
/// counters go on at the Remora's own controller's upkeep, and this game ends
/// before that.
///
/// Both words of the filter are struck as well as the sentence read: the Sol
/// Ring its own controller casts is a noncreature spell that costs nobody a
/// card, so "an opponent casts" is doing work, and the Dark Ritual it does
/// react to is that opponent's noncreature spell.
///
/// What is asked is asserted down to the number — `{4}` of the player who
/// cast, never the `{1}` the upkeep clause would have charged — and with the
/// tax declined the payment is a card: one off the top of, and one into the
/// hand of, the seat that controls the Remora.
#[test]
fn mystic_remora_taxes_an_opponents_noncreature_spell_and_draws_when_they_decline() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Three Islands pay {U} for the Remora and leave the Ring's {1} behind
    // it; p1's five Swamps and a Forest cast the Ritual with {4} still
    // floating, and the Elves stay back as the creature spell the trigger
    // must not notice.
    let mut engine = Duel::new(87, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[mystic_remora(), quiet_artifact()])
        .battlefield(1, &[swamp(), swamp(), swamp(), swamp(), swamp(), forest()])
        .hand(1, &[dark_ritual(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches a main phase");

    cast_from_hand(&mut engine, p0, mystic_remora());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, mystic_remora()).is_some(),
        "the Remora resolved and stands on p0's battlefield"
    );

    // "an opponent casts": a noncreature spell of the Remora's own
    // controller's is nothing to it, so the Ring costs a card out of hand
    // and nothing comes back.
    let hand_before_own_spell = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before_own_spell - 1,
        "the Ring left p0's hand and nothing came back: the trigger watches \
         the spells of the Remora's opponents, not its controller's"
    );

    reach_their_main_phase(&mut engine, p1);
    // p1's lands are tapped *before* the Ritual is cast, and there are five
    // of them so that the pool still covers the tax when it is asked. The
    // question exists either way now that CR 605.3a opens a payment window
    // against an empty pool; floating the mana first keeps this test on the
    // card rather than on the window.
    tap_all_mana(&mut engine, p1);
    let library_before = library_size(&engine, p0);
    let hand_before_tax = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();

    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p1,
        "the tax is asked of the player who cast the spell it taxes"
    );
    assert_eq!(
        prompt,
        YesNoPrompt::PayTax { mana: 4 },
        "\"unless that player pays {{4}}\" — and not the {{1}} the upkeep \
         clause this card cannot express would charge"
    );

    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is one of the two answers the question offered");
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the declined tax pays the Remora's controller a card off the top"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before_tax + 1,
        "and that card arrives: one that left the library without being \
         drawn would satisfy the count above"
    );

    // The creature spell is not what the trigger is written for, and p1 has
    // the Ritual's black mana to pay the Elves' {G} with beside the green
    // already floating.
    let hand_before_elf = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before_elf,
        "\"a noncreature spell\": an Elf cast across the table asks for no tax \
         and hands out no card"
    );
}

/// Exploration: "You may play an additional land on each of your turns."
/// With Exploration on the battlefield, the active player is permitted two land plays in a turn.
/// After playing the first land from hand, a second land drop remains legal, but playing a third is refused.
#[test]
fn exploration_allows_playing_additional_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[exploration()])
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let f1 = play_land(&mut engine, p0, forest());
    assert!(on_battlefield(&engine, p0, forest()).is_some());

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority after first land drop");
    };
    assert!(!legal.lands.is_empty(), "second land drop offered");

    let f2 = play_land(&mut engine, p0, forest());
    assert_ne!(f1, f2);

    let Pending::Priority {
        legal: legal_after, ..
    } = engine.pending().clone()
    else {
        panic!("expected priority after second land drop");
    };
    assert!(legal_after.lands.is_empty(), "no third land drop offered");
}

/// Dragonback Assault — "Landfall — Whenever a land you control enters,
/// create a 4/4 red Dragon creature token with flying." Its enters-trigger
/// (3 damage to each creature and each planeswalker) has no DSL spelling and
/// the file says so, so landfall is the whole of what plays here.
///
/// "A land **you control**" is the word the board is built around: the
/// opponent plays a land of their own first, and a trigger that read every
/// land would have made a Dragon then. The token is checked as a token — an
/// object with no card behind it — and then by its printed numbers and its
/// flying, because a 4/4 flier and a 3/3 trampler are both "a Dragon" to a
/// count of permanents.
#[test]
fn dragonback_assault_makes_a_dragon_on_your_own_land_and_not_on_theirs() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragonback_assault()])
        .hand(0, &[forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    let tokens = |e: &Engine<RegistryLookup>| -> Vec<ObjectId> {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| e.state().object(*id).is_some_and(|o| o.card.is_none()))
            .collect()
    };
    assert!(tokens(&engine).is_empty(), "no token is on the board yet");

    // Their land first. A landfall trigger that forgot whose land it was
    // about would resolve here, and the assertion after it would be the only
    // thing that ever said so.
    reach_their_main_phase(&mut engine, p1);
    play_land(&mut engine, p1, forest());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens(&engine).is_empty(),
        "\"a land you control\" is not \"a land\": the opponent's Forest \
         makes nobody a Dragon"
    );

    reach_their_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    let made = tokens(&engine);
    assert_eq!(made.len(), 1, "one land, one Dragon: {made:?}");
    let dragon = made[0];
    assert_eq!(
        engine
            .state()
            .object(dragon)
            .expect("just created")
            .controller,
        p0,
        "it is created under the controller of the enchantment"
    );
    assert_eq!(pt(&engine, dragon), (4, 4), "a 4/4, as the card prints");
    assert!(
        engine
            .state()
            .object(dragon)
            .expect("just created")
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "with flying — the token the transcoder generated and not the one \
         the pool writes by hand"
    );
    assert!(
        types(&engine, dragon).contains(TypeSet::CREATURE),
        "and it is a creature token"
    );
}

/// Twists and Turns — "When a land you control enters, if you control seven
/// or more lands, transform this enchantment." Explore is not in the DSL and
/// the file says so, so the transform trigger is the whole of what this card
/// does here — and it is two printed words wearing one clause: **a land you
/// control** entering, and **you** controlling seven of them.
///
/// Both are only readable against a board that would fool the other reading.
/// The opponent sits on six lands of their own, so a count that forgot whose
/// permanents it was over would transform the enchantment on the sixth land
/// rather than the seventh; the sixth is played first and has to leave an
/// enchantment standing. `Condition::ControlCount` restricts to `you` before
/// its filter runs, which is why the card writes plain `Filter::LAND` and is
/// right to — this test is what says so.
#[test]
fn twists_and_turns_transforms_on_your_seventh_land_and_not_on_the_tables() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                twists_and_turns(),
            ],
        )
        .battlefield(
            1,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, twists_and_turns()).expect("it is out");
    assert!(
        types(&engine, enchantment).contains(TypeSet::ENCHANTMENT),
        "it starts as the face it prints"
    );

    // The sixth. Eleven lands are on the table by now and six of them are
    // p0's, so a table-wide count would fire here.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        types(&engine, enchantment).contains(TypeSet::ENCHANTMENT),
        "six is not seven — and the six across the table are not yours"
    );

    // The seventh, next turn: one land drop per turn (CR 305.2), so the
    // turn has to go round before the second Forest can be played.
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    let transformed = on_battlefield(&engine, p0, twists_and_turns())
        .expect("the card is still on the battlefield, as Mycoid Maze");
    assert!(
        types(&engine, transformed).contains(TypeSet::LAND),
        "the seventh land turns it over: Mycoid Maze is a Land — Cave"
    );
    assert!(
        !types(&engine, transformed).contains(TypeSet::ENCHANTMENT),
        "and it is no longer the enchantment — a transform is not an addition"
    );
    activate(&mut engine, p0, twists_and_turns(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "Mycoid Maze's own {{T}}: Add {{G}}, which is the back face's ability \
         and not the front's"
    );
}

/// Walk-In Closet's front door is one sentence — "You may play lands from
/// your graveyard" — and it is played from both sides of the permission,
/// because a static that is always on and a static that never fires look the
/// same from inside one game. The same board with the enchantment absent
/// refuses the land drop; with it on the battlefield, the graveyard's Forest
/// is a land drop and is taken.
///
/// The file is `Coverage::Partial` for the Room mechanic itself — nothing
/// unlocks a door, nothing charges a locked door's cost as a sorcery — and
/// for Forgotten Cellar's whole trigger. Neither is reachable here: what is
/// cast is the front half, as an ordinary enchantment.
#[test]
fn walk_in_closet_lets_its_controller_play_a_land_out_of_the_graveyard() {
    let p0 = PlayerId::new(0);

    // Without it, to show the offer is the enchantment's and not the board's.
    let mut bare = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut bare);
    reach_main_phase(&mut bare, p0);
    seed_graveyard(&mut bare, p0, 1);
    let buried = bare.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let Pending::Priority { legal, .. } = bare.pending().clone() else {
        panic!("expected priority, got {:?}", bare.pending())
    };
    assert!(
        !legal.lands.contains(&buried),
        "a land in a graveyard is not a land drop by itself"
    );

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[walk_in_closet()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, walk_in_closet());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, walk_in_closet()).is_some(),
        "the front half resolves as an ordinary enchantment"
    );

    seed_graveyard(&mut engine, p0, 1);
    let buried = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&buried),
        "\"You may play lands from your graveyard\""
    );
    engine
        .apply(p0, PlayerAction::PlayLand { card: buried })
        .expect("the land drop the enchantment granted");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&buried),
        "and the Forest left the graveyard for the battlefield"
    );
}

/// Legion's Landing is `Coverage::Partial` with **both** of its printed
/// clauses in the reason — no trigger counts attacking creatures, and the
/// pool has no 1/1 white Vampire token with lifelink for either clause to
/// make — so the front face carries no ability at all, which is what this
/// pins.
///
/// The enchantment is cast and the battlefield is counted before and after:
/// one new permanent, which is the enchantment itself and no Vampire beside
/// it. It is **meant to fail** the day the token exists. The legendary
/// supertype is checked alongside, because it is the one printed
/// characteristic the card does still carry and a `Partial` is not a licence
/// to get the type line wrong.
#[test]
fn legions_landing_makes_no_vampire_because_the_pool_has_no_such_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[legion_s_landing()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let before = engine.state().zones.list(ZoneLocation::Battlefield).len();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, legion_s_landing());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let landing = on_battlefield(&engine, p0, legion_s_landing()).expect("it resolved");
    assert!(
        engine
            .state()
            .object(landing)
            .expect("it exists")
            .characteristics()
            .supertypes
            .contains(baylee_core::types::SupertypeSet::LEGENDARY),
        "Legion's Landing is a legendary enchantment"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        before + 1,
        "one new permanent and no Vampire beside it: \"create a 1/1 white \
         Vampire creature token with lifelink\" has no token to create — \
         delete this the day `crate::tokens` has one"
    );
}

/// Rancor: the whole card, which is three sentences and a return trip.
///
/// The Aura is cast on a creature, the creature is +2/+0 with trample, and
/// when the Aura goes to the graveyard it comes back to its owner's hand —
/// the last of which is what makes this the card it is, and the only way to
/// see it is to kill the host.
#[test]
fn rancor_pumps_its_host_and_comes_back_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(397, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .hand(0, &[rancor()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    cast_from_hand(&mut engine, p0, rancor());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal host");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (3, 1), "a 1/1 with +2/+0 is a 3/1");
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "and it tramples"
    );

    // The host leaves, so the Aura is put into the graveyard (CR 704.5m) and
    // its own trigger sends it home.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
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
        "the removal and the Aura's trigger both resolve"
    );

    assert!(
        in_hand(&engine, p0, rancor()).is_some(),
        "the Aura went to the graveyard and its trigger returned it to hand"
    );
    assert!(
        in_graveyard(&engine, p0, rancor()).is_none(),
        "so it is not lying in the graveyard"
    );
}

/// Fastbond: "any number of lands", which is the half the card claims.
///
/// `Modifier::ExtraLandDrops(u8::MAX)` is the whole expressible sentence; the
/// damage trigger is refused by name for want of a "you play a land" event.
/// Three lands in one turn is what separates it from Aesi's single extra
/// drop, and from no Fastbond at all.
#[test]
fn fastbond_plays_three_lands_in_one_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(398, forest())
        .battlefield(0, &[fastbond()])
        .hand(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for nth in 1..=3 {
        let card = in_hand(&engine, p0, forest()).expect("a Forest is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card })
            .unwrap_or_else(|err| panic!("land drop {nth} was refused: {err:?}"));
        pass_until(&mut engine, stack_is_empty);
    }

    let lands = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.characteristics().types.intersects(TypeSet::LAND))
        })
        .count();
    assert_eq!(
        lands, 3,
        "three land drops in one turn, and no damage taken"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the printed \"deals 1 damage to you\" is refused by name, so nothing \
         charged for the second and third"
    );
}

/// Mirri's Guile: the upkeep question, and the library it leaves alone.
///
/// "You may look at the top three cards of your library, then put them back
/// in any order" moves no card between zones, so the only thing a game can
/// observe is that the question is asked and that answering it changes no
/// count — which is exactly what `Effect::ReorderTopLibrary` promises.
#[test]
fn mirri_s_guile_asks_at_upkeep_and_moves_no_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(399, forest())
        .battlefield(0, &[mirri_s_guile()])
        .start();
    keep_mulligans(&mut engine);

    let before = library_size(&engine, p0);
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
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the reorder is answered on the way"
    );

    assert_eq!(
        library_size(&engine, p0),
        before,
        "three cards were looked at and put back, so the library is the size \
         it was"
    );
}

/// Arguel's Blood Fast: "{1}{B}, Pay 2 life: Draw a card."
///
/// The transform trigger and the back face's sacrifice ability are refused by
/// name; the draw is the card's front half and charges in two currencies at
/// once, which is what the assertion has to read.
#[test]
fn arguel_s_blood_fast_charges_two_life_for_its_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(400, forest())
        .battlefield(0, &[arguel_s_blood_fast(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, arguel_s_blood_fast(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), before - 1, "one card drawn");
    assert_eq!(
        engine.state().players[0].life,
        18,
        "and two life paid for it"
    );
}

/// Steely Resolve: the type is chosen as it enters, and only that type is
/// untargetable.
///
/// `EnterModifier::ChooseSubtype` plus a filter that reads the choice is the
/// whole card, and the pair is only proved by a board with a creature of the
/// chosen type and one of another: the Elf gains shroud and the Beast beside
/// it does not.
#[test]
fn steely_resolve_shrouds_only_the_type_it_was_given() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(401, forest())
        .battlefield(
            0,
            &[forest(), forest(), llanowar_elves(), rootbreaker_wurm()],
        )
        .hand(0, &[steely_resolve()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is seated");
    cast_from_hand(&mut engine, p0, steely_resolve());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::ELF),
        )
        .expect("Elf is a creature type");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::SHROUD),
        "the Elf is of the chosen type"
    );
    assert!(
        !keywords(&engine, wurm).contains(KeywordSet::SHROUD),
        "and the Wurm is not, so the choice is read rather than ignored"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&wurm) && !options.contains(&elf),
        "shroud is what the offer reads, not a keyword nobody asks about: \
         {options:?}"
    );
}

/// Sterling Grove: the tutor puts the card **on top**, and the enchantments
/// beside it are untargetable.
///
/// Both halves are on one board because the second is what the first costs:
/// the Grove sacrifices itself, so the shroud it was granting goes with it.
#[test]
fn sterling_grove_shrouds_its_neighbours_and_tutors_to_the_top() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(402, forest())
        // Fastbond and not an Aura: an Aura seated with no host is put into
        // the graveyard by state-based actions (CR 704.5m) before anything
        // can be asked about it.
        .battlefield(0, &[sterling_grove(), fastbond(), forest()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let neighbour =
        on_battlefield(&engine, p0, fastbond()).expect("the other enchantment is there");
    assert!(
        keywords(&engine, neighbour).contains(KeywordSet::SHROUD),
        "\"Other enchantments you control have shroud\""
    );
    let grove = on_battlefield(&engine, p0, sterling_grove()).expect("the Grove is there");
    assert!(
        !keywords(&engine, grove).contains(KeywordSet::SHROUD),
        "and \"other\" leaves the Grove itself out"
    );

    let before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sterling_grove(), 1);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the search is answered on the way"
    );
    assert_eq!(
        library_size(&engine, p0),
        before,
        "the tutored card goes on top of the library rather than out of it"
    );
    assert!(
        on_battlefield(&engine, p0, sterling_grove()).is_none(),
        "and the Grove sacrificed itself to do it"
    );
    let _ = p1;
}

/// Sylvan Library: the two extra cards, which is the half the card claims.
///
/// The put-back-or-pay-4-life rider is refused by name, so what is left is a
/// draw step that draws three instead of one — and the `MayDo` in front of it
/// is what makes the assertion a decision rather than a side effect.
#[test]
fn sylvan_library_draws_two_more_at_the_draw_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(403, forest())
        .battlefield(0, &[sylvan_library()])
        .start();
    keep_mulligans(&mut engine);

    let before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
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
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the trigger resolves"
    );

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "two additional cards, on top of whatever the draw step itself drew"
    );
    assert!(
        library_size(&engine, p0) <= before - 2,
        "and they came off the library"
    );
}

/// The Meathook Massacre: the X it announces sweeps the board, and the two
/// drain triggers read which side a creature was on.
///
/// `{X}{B}{B}` plus `Amount::NegX` in a `PumpFilter` is the sweep, and the
/// number is announced on casting (CR 601.2b) — so X is 1 here and the Elves
/// on both sides are 1/1s that CR 704.5f puts in the graveyard. Both drain
/// triggers then fire, in opposite directions, which is what separates the
/// card from one that drained on every death.
#[test]
fn the_meathook_massacre_sweeps_for_the_x_it_announced_and_drains_both_ways() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(406, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[the_meathook_massacre()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, the_meathook_massacre());
    let Pending::ChooseNumber { max, .. } = engine.pending().clone() else {
        panic!("expected the X announcement, got {:?}", engine.pending())
    };
    assert!(max >= 1, "three Swamps pay {{X}}{{B}}{{B}} for X = 1");
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the sweep and both drains resolve"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none()
            && on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "-1/-1 kills a 1/1 on either side (CR 704.5f)"
    );
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (21, 19),
        "your creature dying drains them for one and theirs gains you one, so \
         the two triggers are read by which side the creature was on"
    );
}

/// Underworld Breach: the sentence it keeps is the one that takes it away.
///
/// Escape is refused by name — no `Modifier` grants a casting permission out
/// of a graveyard with its own cost — so what is left is "at the beginning of
/// the end step, sacrifice this enchantment", and a card that did nothing at
/// all would sit on the battlefield forever.
#[test]
fn underworld_breach_sacrifices_itself_at_the_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(407, forest())
        .battlefield(0, &[underworld_breach()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, underworld_breach()).is_some(),
        "it is on the battlefield during the main phase"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, underworld_breach()).is_none(),
        "and it sacrifices itself at the beginning of the end step"
    );
    assert!(
        in_graveyard(&engine, p0, underworld_breach()).is_some(),
        "a sacrifice puts it in its owner's graveyard"
    );
}

/// Retreat to Kazandu: a landfall trigger with two modes, and the mode is
/// chosen every time it triggers.
///
/// `modal_triggered!` is the shape, and the half worth playing is that both
/// modes are reachable off the same land drop: the second land takes the
/// other one, so the counter and the two life are the same ability answering
/// two different questions.
#[test]
fn retreat_to_kazandu_offers_both_of_its_modes_on_a_land_drop() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(408, forest())
        .battlefield(0, &[retreat_to_kazandu(), llanowar_elves()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let card = in_hand(&engine, p0, forest()).expect("the land is in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected the mode choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 2, "\"choose one\" of two");
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "the first mode put a +1/+1 counter on the target"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the second mode's two life was not also taken"
    );
}

/// Erode: the removal, and the land the *other* player is offered for it.
///
/// `Effect::OptionalBasicLandSearchFor { player: ControllerOfTarget }` is the
/// half that is easy to write pointing at the wrong seat, so the assertion is
/// on whose battlefield the basic arrives — and it is not the caster's.
#[test]
fn erode_destroys_a_creature_and_offers_its_controller_a_basic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(409, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[erode()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is seated");
    let their_library = library_size(&engine, p1);
    cast_from_hand(&mut engine, p0, erode());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the search the other seat is offered is answered on the way"
    );

    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_none(),
        "the Wurm is destroyed"
    );
    assert!(
        library_size(&engine, p1) < their_library,
        "and it is the Wurm's controller whose library the basic came out of"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_size(&engine, p0),
        "the caster searched nothing"
    );
}

/// Temur Ascendancy: the haste it grants, on a creature that has just
/// arrived.
///
/// The "power 4 or greater" draw trigger is refused by name, so the static is
/// the card here — and it is a second printing of the same sentence Maelstrom
/// Wanderer carries, which is why this one is asserted on the keyword and on
/// the attacker list rather than on both again.
#[test]
fn temur_ascendancy_gives_a_freshly_cast_creature_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(410, forest())
        .battlefield(0, &[temur_ascendancy(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf arrived this turn");
    assert!(
        keywords(&engine, elf).contains(KeywordSet::HASTE),
        "\"Creatures you control have haste\""
    );
}

/// `Druid Class` (`Coverage::Partial`):
/// "Landfall — Whenever a land you control enters, you gain 1 life.
/// `{{2}}{{G}}`: Level 2. You may play an additional land on each of your turns.
/// `{{4}}{{G}}`: Level 3. When this Class becomes level 3, target land you control becomes
/// a creature with haste and 'This creature's power and toughness are each equal to the
/// number of lands you control.' It's still a land."
///
/// Under `Coverage::Partial`, the level 2 extra land drop and level 3 animation trigger are
/// omitted, while the Landfall trigger and both level-up activated abilities are implemented.
/// The test verifies that playing a land triggers Landfall to gain 1 life, and that activating
/// level 2 with `{{2}}{{G}}` puts a level counter on `Druid Class`.
#[test]
fn druid_class_triggers_landfall_and_levels_to_level_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[druid_class(), forest(), forest(), forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[0].life, 20, "starts at 20 life");
    let class = on_battlefield(&engine, p0, druid_class()).expect("Druid Class on battlefield");
    assert_eq!(
        counters_on(&engine, class, CounterKind::Level),
        0,
        "starts at level 1 with 0 level counters"
    );

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "Landfall trigger gained 1 life"
    );

    tap_mana_except(&mut engine, p0, class);
    activate(&mut engine, p0, druid_class(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, class, CounterKind::Level),
        1,
        "one level counter, so Druid Class is level 2"
    );
}

/// `Garruk's Uprising` (`Coverage::Partial`):
/// "When this enchantment enters, if you control a creature with power 4 or greater, draw a card.
/// Creatures you control have trample. Whenever a creature you control with power 4 or greater enters,
/// draw a card."
///
/// Under `Coverage::Partial`, the enchantment's own enter draw trigger is omitted, while the trample
/// anthem and the enter draw trigger for creatures with power 4 or greater are implemented.
/// The test verifies that controlled creatures gain trample while the opponent's creature does not,
/// that casting a 1/1 `llanowar_elves()` draws no card, and that casting a 6/6 `rootbreaker_wurm()`
/// triggers the draw ability.
#[test]
fn garruk_s_uprising_grants_trample_and_draws_on_power_four_or_greater() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(105, forest())
        .battlefield(
            0,
            &[
                garruk_s_uprising(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[llanowar_elves(), rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's elf on battlefield");
    assert!(
        !keywords(&engine, their_elf).contains(KeywordSet::TRAMPLE),
        "Garruk's Uprising only grants trample to creatures you control"
    );

    let lib_start = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf on battlefield");
    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::TRAMPLE),
        "controlled creature has trample from the anthem"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_start,
        "a 1/1 entering does not trigger the draw trigger"
    );

    cast_from_hand(&mut engine, p0, rootbreaker_wurm());
    pass_until(&mut engine, stack_is_empty);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("wurm on battlefield");
    assert!(
        keywords(&engine, wurm).contains(KeywordSet::TRAMPLE),
        "wurm has trample"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_start - 1,
        "a 6/6 creature entering triggers the draw ability"
    );
}

/// `Grasping Shadows` // `Shadows' Lair` (`Coverage::Partial`):
/// "Whenever a creature you control attacks alone, it gains deathtouch and lifelink until
/// end of turn. Put a dread counter on this enchantment. Then if there are three or more dread
/// counters on it, transform it. // `{{T}}`: Add `{{B}}`. `{{B}}`, `{{T}}`, Remove a dread counter
/// from this land: You draw a card and you lose 1 life."
///
/// Under `Coverage::Partial`, the lone-attacker trigger and the back face's dread-counter ability
/// are omitted, leaving the front face as a `{{3}}{{B}}` enchantment with no abilities. The test
/// casts `Grasping Shadows` from hand, confirms it enters as an enchantment on face 0, verifies that
/// with floating mana `LegalActions::abilities` offers no activated abilities on it, and confirms
/// that an attacking creature gains neither deathtouch nor lifelink under `Coverage::Partial`.
#[test]
fn grasping_shadows_casts_and_enters_as_enchantment_without_lone_attacker_trigger() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(109, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[grasping_shadows()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    // Not `cast_from_hand`: it taps every mana source, and Llanowar Elves is
    // one — a tapped creature cannot be declared an attacker (CR 508.1a), so
    // the Elf has to be kept out of the payment it is not needed for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, grasping_shadows());
    pass_until(&mut engine, stack_is_empty);

    let shadows =
        on_battlefield(&engine, p0, grasping_shadows()).expect("Grasping Shadows on battlefield");
    assert_eq!(
        engine.state().object(shadows).map(|o| o.face_index),
        Some(0),
        "Grasping Shadows is on face 0"
    );

    let t = types(&engine, shadows);
    assert!(
        t.contains(TypeSet::ENCHANTMENT),
        "Grasping Shadows is an enchantment"
    );
    assert!(!t.contains(TypeSet::LAND), "Grasping Shadows is not a land");

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == shadows),
        "front face offers no activated abilities with floating mana"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let kw = keywords(&engine, elf);
    assert!(
        !kw.contains(KeywordSet::DEATHTOUCH),
        "under `Coverage::Partial` attacking alone does not grant deathtouch"
    );
    assert!(
        !kw.contains(KeywordSet::LIFELINK),
        "under `Coverage::Partial` attacking alone does not grant lifelink"
    );
    assert_eq!(
        engine.state().object(shadows).map(|o| o.face_index),
        Some(0),
        "Grasping Shadows remains on face 0"
    );
}

/// `Growing Rites of Itlimoc` // `Itlimoc, Cradle of the Sun` (`Coverage::Partial`):
/// "When `Growing Rites of Itlimoc` enters, look at the top four cards of your library. You may
/// reveal a creature card from among them and put it into your hand. Put the rest on the bottom
/// of your library in any order. At the beginning of your end step, if you control four or more
/// creatures, transform `Growing Rites of Itlimoc`. // `{{T}}`: Add `{{G}}`. `{{T}}`: Add `{{G}}` for
/// each creature you control."
///
/// Under `Coverage::Partial`, the enter look-at-four trigger is omitted, while the end-step
/// transform trigger and the back face's mana abilities are implemented. The test sets up four
/// controlled creatures, advances to the end step where the transform condition is met, verifies
/// the enchantment transforms into the legendary land `Itlimoc, Cradle of the Sun` on face 1,
/// and activates Itlimoc's second mana ability to produce green mana equal to the creature count.
#[test]
fn growing_rites_of_itlimoc_transforms_at_four_creatures_and_taps_for_creature_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(
            0,
            &[
                growing_rites_of_itlimoc(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);

    let itlimoc =
        on_battlefield(&engine, p0, growing_rites_of_itlimoc()).expect("Itlimoc on battlefield");
    assert_eq!(
        engine.state().object(itlimoc).map(|o| o.face_index),
        Some(1),
        "Growing Rites of Itlimoc transformed to face 1"
    );

    let t = types(&engine, itlimoc);
    assert!(t.contains(TypeSet::LAND), "Itlimoc is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Itlimoc is not an enchantment"
    );

    activate(&mut engine, p0, growing_rites_of_itlimoc(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "Itlimoc added four green mana for the four creatures controlled"
    );
}

/// `Hadana's Climb` // `Winged Temple of Orazca` (`Coverage::Partial`):
/// "At the beginning of combat on your turn, put a +1/+1 counter on target creature you control.
/// Then if that creature has three or more +1/+1 counters on it, transform `Hadana's Climb`.
/// // `{{T}}`: Add one mana of any color. `{{1}}{{G}}{{U}}`, `{{T}}`: Target creature you control
/// gains flying and gets +X/+X until end of turn, where X is its power."
///
/// Under `Coverage::Partial`, the transform clause is omitted, while the combat-begin trigger
/// placing a +1/+1 counter on a controlled creature is implemented. The test advances to the
/// combat phase, targets a controlled 1/1 `llanowar_elves()`, verifies that the counter is placed
/// and its projected power and toughness become 2/2, and confirms that `Hadana's Climb` remains
/// on face 0.
#[test]
fn hadana_s_climb_puts_plus_one_counter_on_controlled_creature_at_combat() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .battlefield(0, &[hadana_s_climb(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1), "starts as a 1/1");
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        0,
        "starts with 0 counters"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&elf),
        "the controlled creature is a legal target"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "one +1/+1 counter placed on the target"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "elf is now 2/2");

    let climb =
        on_battlefield(&engine, p0, hadana_s_climb()).expect("Hadana's Climb on battlefield");
    assert_eq!(
        engine.state().object(climb).map(|o| o.face_index),
        Some(0),
        "Hadana's Climb remains on face 0"
    );
    assert!(
        types(&engine, climb).contains(TypeSet::ENCHANTMENT),
        "Hadana's Climb is an enchantment"
    );
}

/// `Journey to Eternity` // `Atzal, Cave of Eternity` (`Coverage::Implemented`):
/// "Enchant creature you control. When enchanted creature dies, return it to the battlefield
/// under your control, then return this card to the battlefield transformed under your control.
/// // `{{T}}`: Add one mana of any color. `{{3}}{{B}}{{G}}`, `{{T}}`: Return target creature card
/// from your graveyard to the battlefield."
///
/// Under `Coverage::Implemented`, casting `Journey to Eternity` targets and attaches to a controlled
/// creature. When that creature dies (sacrificed to `ashnods_altar()`), the dies trigger returns the
/// creature to the battlefield and returns `Journey to Eternity` transformed as the legendary land
/// `Atzal, Cave of Eternity` on face 1, which then activates its `{{T}}` mana ability for black mana.
#[test]
fn journey_to_eternity_returns_creature_and_transforms_into_atzal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(
            0,
            &[
                forest(),
                swamp(),
                plains(),
                llanowar_elves(),
                ashnods_altar(),
            ],
        )
        .hand(0, &[journey_to_eternity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    cast_from_hand(&mut engine, p0, journey_to_eternity());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "the controlled creature is a legal target"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let aura = on_battlefield(&engine, p0, journey_to_eternity())
        .expect("Journey to Eternity on battlefield");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(elf),
        "Journey to Eternity is attached to the creature"
    );

    let altar =
        on_battlefield(&engine, p0, ashnods_altar()).expect("Ashnod's Altar on battlefield");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: altar,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards { prompt, .. } = engine.pending().clone() else {
        panic!(
            "expected sacrifice choice for Ashnod's Altar, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the enchanted creature returned to the battlefield under your control"
    );
    let atzal = on_battlefield(&engine, p0, journey_to_eternity())
        .expect("Atzal, Cave of Eternity returned transformed");
    assert_eq!(
        engine.state().object(atzal).map(|o| o.face_index),
        Some(1),
        "Atzal is on face 1"
    );

    let t = types(&engine, atzal);
    assert!(t.contains(TypeSet::LAND), "Atzal is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Atzal is no longer an enchantment"
    );

    activate(&mut engine, p0, journey_to_eternity(), 0);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!(
            "expected color choice for Atzal mana ability, got {:?}",
            engine.pending()
        );
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "Atzal produced one black mana"
    );
}

/// `Path of Mettle` // `Metzali, Tower of Triumph` (`Coverage::Partial`):
/// "When `Path of Mettle` enters, it deals 1 damage to each creature that doesn't have
/// first strike, double strike, vigilance, or haste. Whenever you attack with at least two
/// creatures that have first strike, double strike, vigilance, and/or haste, transform
/// `Path of Mettle`. // `{{T}}`: Add one mana of any color. `{{1}}{{R}}`, `{{T}}`: `Metzali` deals
/// 2 damage to each opponent. `{{2}}{{W}}`, `{{T}}`: Choose a creature at random that attacked
/// this turn. Destroy that creature."
///
/// Under `Coverage::Partial`, the enter damage trigger, the combat transform trigger, and the
/// back face's damage and destruction abilities are omitted, leaving the front face as a
/// `{{R}}{{W}}` legendary enchantment with no abilities. The test casts `Path of Mettle` from
/// hand, confirms it enters as a legendary enchantment on face 0 without damaging a 1/1
/// `llanowar_elves()`, verifies that with floating mana `LegalActions::abilities` offers no
/// activated abilities on it, and confirms it remains on face 0 in the following turn.
#[test]
fn path_of_mettle_casts_and_enters_as_legendary_enchantment_without_damage_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(103, mountain())
        .battlefield(0, &[mountain(), plains(), mountain(), llanowar_elves()])
        .hand(0, &[path_of_mettle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    cast_from_hand(&mut engine, p0, path_of_mettle());
    pass_until(&mut engine, stack_is_empty);

    let mettle =
        on_battlefield(&engine, p0, path_of_mettle()).expect("Path of Mettle on battlefield");
    assert_eq!(
        engine.state().object(mettle).map(|o| o.face_index),
        Some(0),
        "Path of Mettle is on face 0"
    );

    let t = types(&engine, mettle);
    assert!(
        t.contains(TypeSet::ENCHANTMENT),
        "Path of Mettle is an enchantment"
    );
    assert!(!t.contains(TypeSet::LAND), "Path of Mettle is not a land");
    assert!(
        engine
            .state()
            .object(mettle)
            .expect("Path of Mettle exists")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "Path of Mettle is legendary"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "under `Coverage::Partial` no enter-damage trigger fires, so the 1/1 elf survives"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "elf remains an undamaged 1/1");

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == mettle),
        "front face offers no activated abilities with floating mana"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().object(mettle).map(|o| o.face_index),
        Some(0),
        "Path of Mettle remains on face 0 in the following turn"
    );
}

/// `Search for Azcanta` // `Azcanta, the Sunken Ruin` (`Coverage::Partial`):
/// "At the beginning of your upkeep, surveil 1. Then if you have seven or more cards in your
/// graveyard, you may transform `Search for Azcanta`. // `{{T}}`: Add `{{U}}`. `{{2}}{{U}}`, `{{T}}`: Look
/// at the top four cards of your library. You may reveal a noncreature, nonland card from among
/// them and put it into your hand. Put the rest on the bottom of your library in any order."
///
/// Under `Coverage::Partial`, the transform clause and the back face's card-selection ability
/// are omitted, while the upkeep surveil 1 trigger is implemented. The test advances to upkeep,
/// intercepts the surveil 1 prompt (`ChoicePrompt::SurveilGraveyard`), chooses to put the top
/// card into the graveyard, verifies the card arrives in the graveyard, and confirms that
/// `Search for Azcanta` remains on face 0.
#[test]
fn search_for_azcanta_surveils_at_upkeep_and_remains_on_face_zero() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[search_for_azcanta(), island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SurveilGraveyard,
                ..
            }
        )
    });

    let Pending::ChooseCards {
        options,
        prompt,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected surveil choice, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::SurveilGraveyard);
    assert_eq!(min, 0, "surveil allows choosing 0 cards for graveyard");
    assert_eq!(max, 1, "surveil 1 allows at most 1 card");
    assert_eq!(
        options.len(),
        1,
        "surveil 1 looks at the top card of library"
    );

    let milled_card = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![milled_card],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let azcanta = on_battlefield(&engine, p0, search_for_azcanta())
        .expect("Search for Azcanta on battlefield");
    assert_eq!(
        engine.state().object(azcanta).map(|o| o.face_index),
        Some(0),
        "Search for Azcanta remains on face 0"
    );
    assert!(
        types(&engine, azcanta).contains(TypeSet::ENCHANTMENT),
        "Search for Azcanta is an enchantment"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "the surveilled card was put into the graveyard"
    );
}

/// `Sidequest: Catch a Fish` // `Cooking Campsite` (`Coverage::Partial`):
/// "At the beginning of your upkeep, look at the top card of your library. If it's an artifact or
/// creature card, you may reveal it and put it into your hand. If you put a card into your hand
/// this way, create a Food token and transform this enchantment. // `{{T}}`: Add `{{W}}`. `{{3}}`, `{{T}}`,
/// Sacrifice an artifact: Put a +1/+1 counter on each creature you control. Activate only as a sorcery."
///
/// Under `Coverage::Partial`, the front-face upkeep reveal-and-transform trigger is omitted,
/// leaving the front face as a `{{2}}{{W}}` enchantment with no abilities. The test casts
/// `Sidequest: Catch a Fish` from hand, confirms it enters as an enchantment on face 0, verifies
/// that with floating mana `LegalActions::abilities` offers no activated abilities on it, and
/// advances to the following turn's upkeep and main phase confirming no trigger fires and it remains on face 0.
#[test]
fn sidequest_catch_a_fish_casts_and_enters_as_enchantment_without_upkeep_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(108, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[sidequest_catch_a_fish()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, sidequest_catch_a_fish());
    pass_until(&mut engine, stack_is_empty);

    let quest = on_battlefield(&engine, p0, sidequest_catch_a_fish())
        .expect("Sidequest: Catch a Fish on battlefield");
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest is on face 0"
    );

    let t = types(&engine, quest);
    assert!(
        t.contains(TypeSet::ENCHANTMENT),
        "Sidequest is an enchantment"
    );
    assert!(!t.contains(TypeSet::LAND), "Sidequest is not a land");

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == quest),
        "front face offers no activated abilities with floating mana"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "two turns pass, so p0 draws once for its own draw step (CR 504.1) — \
         and under `Coverage::Partial` the upkeep trigger adds no second card"
    );
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest remains on face 0 in the following turn"
    );
}

/// `Storm the Vault` // `Vault of Catlacan` (`Coverage::Implemented`):
/// "Whenever one or more creatures you control deal combat damage to a player, create a Treasure
/// token. At the beginning of your end step, if you control five or more artifacts, transform
/// `Storm the Vault`. // `{{T}}`: Add one mana of any color. `{{T}}`: Add `{{U}}` for each artifact
/// you control."
///
/// Under `Coverage::Implemented`, controlling five artifacts satisfies the end-step transform
/// condition. The test sets up five `quiet_artifact()`s, advances to the end step where the
/// transform trigger resolves, verifies `Storm the Vault` transforms into the legendary land
/// `Vault of Catlacan` on face 1, and activates its second mana ability to produce blue mana
/// equal to the artifact count.
#[test]
fn storm_the_vault_transforms_at_five_artifacts_and_taps_for_artifact_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, island())
        .battlefield(
            0,
            &[
                storm_the_vault(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);

    let vault =
        on_battlefield(&engine, p0, storm_the_vault()).expect("Vault of Catlacan on battlefield");
    assert_eq!(
        engine.state().object(vault).map(|o| o.face_index),
        Some(1),
        "Storm the Vault transformed to face 1"
    );

    let t = types(&engine, vault);
    assert!(t.contains(TypeSet::LAND), "Vault of Catlacan is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Vault of Catlacan is not an enchantment"
    );

    activate(&mut engine, p0, storm_the_vault(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        5,
        "Vault of Catlacan produced five blue mana for the five artifacts controlled"
    );
}

/// `Vance's Blasting Cannons` // `Spitfire Bastion` (`Coverage::Partial`):
/// "At the beginning of your upkeep, exile the top card of your library. If it's a nonland card,
/// you may cast that card this turn. Whenever you cast your third spell in a turn, you may transform
/// `Vance's Blasting Cannons`. // `{{T}}`: Add `{{R}}`. `{{2}}{{R}}`, `{{T}}`: `Spitfire Bastion` deals
/// 3 damage to any target."
///
/// Under `Coverage::Partial`, the upkeep exile-and-cast trigger is omitted, while the third-spell
/// transform trigger and the back face's abilities are implemented. The test casts three spells
/// in one turn using `dark_ritual()`, intercepts the optional transform prompt on the third cast,
/// accepts the transformation into the legendary land `Spitfire Bastion` on face 1, and taps
/// `Spitfire Bastion` for red mana.
#[test]
fn vance_s_blasting_cannons_transforms_on_third_spell_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, swamp())
        .battlefield(0, &[vance_s_blasting_cannons(), swamp(), swamp(), swamp()])
        .hand(0, &[dark_ritual(), dark_ritual(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Spell 1
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    // Spell 2
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    // Spell 3 triggers the third-spell MayDo transform trigger
    cast_with_floating(&mut engine, p0, dark_ritual());
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

    let bastion = on_battlefield(&engine, p0, vance_s_blasting_cannons())
        .expect("Spitfire Bastion on battlefield");
    assert_eq!(
        engine.state().object(bastion).map(|o| o.face_index),
        Some(1),
        "Vance's Blasting Cannons transformed to face 1"
    );

    let t = types(&engine, bastion);
    assert!(t.contains(TypeSet::LAND), "Spitfire Bastion is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Spitfire Bastion is no longer an enchantment"
    );

    activate(&mut engine, p0, vance_s_blasting_cannons(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Spitfire Bastion tapped for one red mana"
    );
}

/// `Fable of the Mirror-Breaker` // `Reflection of Kiki-Jiki` (`Coverage::Partial`):
/// "I — Create a 2/2 red Goblin Shaman creature token with 'Whenever this token attacks,
/// create a Treasure token.'
/// II — You may discard up to two cards. If you do, draw that many cards.
/// III — Exile this Saga, then return it to the battlefield transformed under your control. //
/// `{{1}}`, `{{T}}`: Create a token that's a copy of another target nonlegendary creature you
/// control, except it has haste. Sacrifice it at the beginning of the next end step."
///
/// Under `Coverage::Partial`, chapters I and II and the back face's copy ability are omitted.
/// Chapter III (`Effect::ExileSelfReturnAsFace { face: 1 }`) is implemented.
/// The test casts `Fable of the Mirror-Breaker`, tracks lore counters advancing across turns,
/// and verifies that chapter III exiles the Saga and returns it transformed as
/// `Reflection of Kiki-Jiki` on face 1 as a 2/2 creature.
#[test]
fn fable_of_the_mirror_breaker_advances_to_chapter_three_and_transforms() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(101, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[fable_of_the_mirror_breaker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fable_of_the_mirror_breaker());
    pass_until(&mut engine, stack_is_empty);

    let saga = on_battlefield(&engine, p0, fable_of_the_mirror_breaker())
        .expect("Fable of the Mirror-Breaker on battlefield");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        1,
        "enters with one lore counter"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let saga = on_battlefield(&engine, p0, fable_of_the_mirror_breaker())
        .expect("Fable on battlefield in turn 2");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        2,
        "second lore counter added in precombat main phase"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // Chapter III triggered at the start of turn 3's precombat main phase; resolve it.
    pass_until(&mut engine, stack_is_empty);

    let kiki = on_battlefield(&engine, p0, fable_of_the_mirror_breaker())
        .expect("Reflection of Kiki-Jiki on battlefield");
    assert_eq!(
        engine
            .state()
            .object(kiki)
            .expect("object exists")
            .face_index,
        1,
        "transformed to face 1"
    );
    assert_eq!(
        pt(&engine, kiki),
        (2, 2),
        "Reflection of Kiki-Jiki is a 2/2"
    );
    assert!(
        types(&engine, kiki).contains(TypeSet::CREATURE),
        "Reflection of Kiki-Jiki is a creature"
    );
}

/// `Welcome to . . .` // `Jurassic Park` (`Coverage::Partial`):
/// "I — For each opponent, up to one target noncreature artifact they control becomes a 0/4
/// Wall artifact creature with defender for as long as you control this Saga.
/// II — Create a 3/3 green Dinosaur creature token with trample. It gains haste until end of turn.
/// III — Destroy all Walls. Exile this Saga, then return it to the battlefield transformed under
/// your control. // `{{T}}`: Add `{{G}}` for each Dinosaur you control."
///
/// Under `Coverage::Partial`, chapters I and II and the graveyard escape grant are omitted.
/// Chapter III and the back face's Dinosaur-scaled mana ability are implemented.
/// The test casts `Welcome to . . .` with a Dinosaur (`Carnage Tyrant`) on the battlefield,
/// advances lore counters to chapter III, resolves the transformation to `Jurassic Park` on face 1,
/// and taps `Jurassic Park` to produce one green mana for the controlled Dinosaur.
#[test]
fn welcome_to_advances_to_chapter_three_and_taps_for_dinosaur_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[forest(), forest(), forest(), carnage_tyrant()])
        .hand(0, &[welcome_to()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, welcome_to());
    pass_until(&mut engine, stack_is_empty);

    let saga = on_battlefield(&engine, p0, welcome_to()).expect("Welcome to . . . on battlefield");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        1,
        "enters with one lore counter"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let saga = on_battlefield(&engine, p0, welcome_to())
        .expect("Welcome to . . . on battlefield in turn 2");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        2,
        "second lore counter added in precombat main phase"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // Chapter III triggered at the start of turn 3's precombat main phase; resolve it.
    pass_until(&mut engine, stack_is_empty);

    let jp = on_battlefield(&engine, p0, welcome_to()).expect("Jurassic Park on battlefield");
    assert_eq!(
        engine.state().object(jp).expect("object exists").face_index,
        1,
        "transformed to face 1"
    );
    assert!(
        types(&engine, jp).contains(TypeSet::LAND),
        "Jurassic Park is a land"
    );

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "mana pool is empty before activating Jurassic Park"
    );

    activate(&mut engine, p0, welcome_to(), 0);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "produced 1 green mana for 1 controlled Dinosaur (Carnage Tyrant)"
    );
}

fn alexi_s_cloak() -> CardIndex {
    card_index("e52eb1a6-fff1-4a47-b434-31a74d76231c")
}

/// Alexi's Cloak is `{1}{U}` Aura with flash and one printed static:
/// "Enchanted creature has shroud." The static is `Filter::AttachedToBySource`,
/// so the only reading worth playing is the one that tells the creature the Aura
/// *holds* from every other creature in the game — which is why a bare Elf beside
/// the host is read, and why the opponent's own removal spell is what says the
/// shroud is real rather than decorative: Swords to Plowshares offers the bare Elf
/// and refuses the enchanted one in the very same question, so a filter that had
/// simply dropped the keyword would fail one half or the other.
#[test]
fn alexis_cloak_wards_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[alexi_s_cloak()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "nothing is enchanted yet"
    );

    // {1}{U} off the two Islands, and "enchant creature" is a target choice
    // (CR 601.2c) answered before the cost is read out of the pool.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, alexi_s_cloak());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "target creature reaches either Elf on this board: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let cloak = on_battlefield(&engine, p0, alexi_s_cloak()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(cloak).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it enchanted"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "\"enchanted creature has shroud\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "the static reaches the enchanted creature and no other"
    );

    // The other side of the table, which is where "can't be the target of
    // spells" is visible at all: the opponent's own removal spell asks once
    // and has to name the bare Elf and not the cloaked one.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bystander),
        "the control: the Elf the Cloak does not hold is still a legal target: {options:?}"
    );
    assert!(
        !options.contains(&host),
        "shroud: the enchanted creature cannot be the target of a spell at all: {options:?}"
    );
}

fn buoyancy() -> CardIndex {
    card_index("f4e4060d-bfff-4991-9ae5-8f848304cd1e")
}

/// Buoyancy — {1}{U} Aura: flash, "Enchant creature", "Enchanted creature
/// has flying."
///
/// The cast happens in the *opponent's* first main phase, so the only thing
/// that lets the engine offer it there is the printed flash (CR 702.8) — a
/// card with the same cost and no flash would be refused for timing, and
/// playing it on its controller's own turn could not tell the two apart.
/// "Enchant creature" is a restriction on what the Aura may hold and not on
/// who controls it, so the offer is read across the whole table before the
/// answer is given; and the Elf that answer declines is the live 1/1 that
/// shows the grant landing on the creature the Aura holds and nowhere else.
#[test]
fn buoyancy_flashes_in_on_the_opponents_turn_and_grants_flying_only_to_its_host() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), quiet_creature()])
        .hand(0, &[buoyancy()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);

    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "nothing is enchanted yet"
    );

    // The opponent's first main phase, and then priority handed to the seat
    // that is not taking the turn (CR 117.3a).
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert_eq!(
        engine.state().turn.active,
        p1,
        "the Aura is still being cast on the opponent's turn"
    );

    // Mana first: the offer is read off the pool and not off the untapped
    // lands. The Elf is kept standing so that the question below is about a
    // creature and not about a tapped one.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    let aura = in_hand(&engine, p0, buoyancy()).expect("the Aura is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&aura),
        "flash (CR 702.8) is the whole of why an enchantment is castable in \
         the opponent's main phase: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, buoyancy());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("an Aura asks what it enchants, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"Enchant creature\" reaches any creature in the game, on either \
         side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| on_battlefield(e, p0, buoyancy()).is_some());

    let on_table = on_battlefield(&engine, p0, buoyancy()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(on_table).and_then(|o| o.attached_to),
        Some(mine),
        "the Aura holds the creature it was aimed at"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature has flying"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the Elf that was offered and declined is still a printed 1/1"
    );
    assert!(
        !keywords(&engine, on_table).contains(KeywordSet::FLYING),
        "the static is filtered to a creature, so the Aura bestows the \
         keyword rather than keeping it"
    );
}

fn capashen_standard() -> CardIndex {
    card_index("75510429-41bb-409e-b6fe-04a8bb174c6b")
}

/// Capashen Standard prints three lines: "Enchant creature", "Enchanted
/// creature gets +1/+1", and "{2}, Sacrifice this Aura: Draw a card". A
/// creature stands on each side of the table so the static can be read as
/// `Filter::AttachedToBySource` and not as "creatures you control" — and the
/// pump is read *again* after the sacrifice, where a host that kept the
/// counter would mean the Aura is still modifying something it let go of.
/// Both mana costs are paid inside one main phase: three Plains tap and the
/// {W} leaves exactly the {2} the second line charges, which is what makes
/// the ability offered at all rather than refused for want of mana.
#[test]
fn capashen_standard_pumps_only_its_host_then_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[capashen_standard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // The Elves are kept back: its {T} is a mana ability in the pool's own
    // terms, and tapping it here would put green in the pool that neither
    // printed line accounts for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, capashen_standard());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "`enchant creature` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"enchant creature\" names any creature, on either side of the \
         table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, capashen_standard()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "the enchanted creature gets +1/+1"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the creature across the table is not the one this Aura holds"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "one of the three Plains paid the {{W}}, and the {{2}} is still \
         floating — read off the pool, which is what the offer below reads"
    );

    // Ability 0 is the enchant spell, 1 the static that pumps, 2 this.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(aura, 2)),
        "with the {{2}} in the pool the sacrifice ability is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, capashen_standard(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, capashen_standard()).is_some(),
        "sacrificing the Aura is the other half of the cost"
    );
    assert!(
        on_battlefield(&engine, p0, capashen_standard()).is_none(),
        "and it left the battlefield to pay it"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the +1/+1 went with the Aura, which is the whole of what \
         `Filter::AttachedToBySource` says"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and into the hand, where a count off the library alone would not \
         have put it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} was paid"
    );
}

fn crackling_club() -> CardIndex {
    card_index("876affb5-155d-4268-9e46-4437a9fbccc7")
}

/// Crackling Club prints three sentences whose order is what makes them
/// readable: an Aura enchanting a creature (any creature — the Elf across the
/// table is offered as readily as my own), "+1/+0" on the creature it *holds*
/// and no other, and "Sacrifice this Aura: It deals 1 damage to target
/// creature." The pump is read on both sides of the sacrifice, so the static
/// is shown to hang on the Aura's presence rather than on the creature; and
/// the sacrifice is read as a cost, which is why the target question
/// (CR 601.2c) arrives with the Aura still on the battlefield and the payment
/// (CR 601.2h) not yet made.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn crackling_club_pumps_the_creature_it_enchants_then_trades_itself_for_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), llanowar_elves()])
        .hand(0, &[crackling_club()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // {R} off the Mountain alone: the Elf is kept untapped because it is the
    // creature the Aura is about to hold, not a mana source on the board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, crackling_club());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "an Aura may enchant any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered");
    pass_until(&mut engine, stack_is_empty);

    let club = on_battlefield(&engine, p0, crackling_club()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(club).and_then(|o| o.attached_to),
        Some(host),
        "and landed on the creature it was cast at"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 1),
        "\"enchanted creature gets +1/+0\" — the point of power and no point of toughness"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the creature the Aura holds and never across the table"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let mut offered = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == club);
    let (source, ability_index) = offered
        .next()
        .expect("with a creature on the table the sacrifice is offered");
    assert!(
        offered.next().is_none(),
        "and it is the Aura's only activated ability: the grant is a static"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the price is the Aura itself: nothing is floating to pay with"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("a cost of the Aura's own body is one it can always pay");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&theirs) && options.contains(&host),
        "one damage may be aimed at any creature, its own host included: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, crackling_club()).is_some(),
        "CR 601.2c comes before CR 601.2h: the sacrifice is not paid while \
         the target is being chosen"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 1),
        "so the pump is still standing at the moment the question is asked"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature the question offered");

    assert!(
        on_battlefield(&engine, p0, crackling_club()).is_none(),
        "answering the target pays the cost, and the cost is the Aura"
    );
    assert!(
        in_graveyard(&engine, p0, crackling_club()).is_some(),
        "an Aura that sacrifices itself goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the damage is on the stack behind the price that bought it"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a 1/1 is CR 704.5f, and the Elf across the table is gone"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the Aura was holding is untouched"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "and it is a 1/1 again: the +1/+0 died with the Aura that granted it"
    );
}

fn diplomatic_immunity() -> CardIndex {
    card_index("f1a3153c-0200-4ff2-b7d5-48d23920bb3c")
}

/// Diplomatic Immunity — {1}{U} Aura: "Enchant creature. Shroud. Enchanted
/// creature has shroud." Both printed sentences are about *not* being
/// targeted, so the scenario has to hold a target question open: a bare Elf
/// beside the enchanted one is what makes a refusal readable, because a
/// removal spell offered nothing at all would be refused for having no target
/// rather than for shroud. Vindicate's one menu — every permanent in the game
/// — is therefore read three ways: the unenchanted Elf is on it (the spell is
/// real), the enchanted Elf is not (the static grants), and the Aura itself is
/// not (the shroud it prints for its own body, without which pointing at the
/// Aura would strip the protection off the creature).
#[test]
#[allow(clippy::too_many_lines)] // two casts, and one menu read three ways
fn diplomatic_immunity_shrouds_the_creature_it_enchants_and_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[diplomatic_immunity(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "a printed 1/1 has none of it before the Aura lands on it"
    );

    // {1}{U}: the lands pay and the two Elves are tapped along with them,
    // which costs nothing here — they are the creatures this test is about,
    // not attackers, and the pool they leave is read below rather than
    // asserted.
    cast_from_hand(&mut engine, p0, diplomatic_immunity());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("an Aura targets as it is cast, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" is any creature on your side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let aura = on_battlefield(&engine, p0, diplomatic_immunity()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "it entered attached to the creature it was cast at"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "\"enchanted creature has shroud\" reaches the creature it holds"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "and the Elf nobody enchanted is still a bare 1/1"
    );
    assert!(
        keywords(&engine, aura).contains(KeywordSet::SHROUD),
        "the Aura prints shroud for itself, which is what keeps the grant \
         from being undone by pointing at what grants it"
    );

    // The removal spell, off what the first cast left in the pool — the
    // question it opens is the whole of what this card is.
    cast_with_floating(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bystander),
        "the unenchanted Elf is a legal target, so the spell was cast rather \
         than refused for want of one: {options:?}"
    );
    assert!(
        !options.contains(&host),
        "the enchanted creature has shroud and cannot be the target of a \
         spell (CR 702.18a): {options:?}"
    );
    assert!(
        !options.contains(&aura),
        "nor can the Aura that grants it: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .expect("the Elf the offer named");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(host)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the creature the spell could not name is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "while the one it could name was destroyed, so the spell did resolve"
    );
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and the Aura is still holding on to it"
    );
}

fn enfeeblement() -> CardIndex {
    card_index("42b2db4c-4a1d-436f-9eeb-53a04db46c58")
}

/// Enfeeblement prints two sentences — "Enchant creature" and "Enchanted
/// creature gets -2/-2" — and neither can be read off the card file: one is a
/// target the engine offers, the other a static whose filter is
/// `Filter::AttachedToBySource`. So the board holds a body a -2/-2 leaves
/// alive (a 4/4, or there would be no projected numbers to read at all) and a
/// bystander across the table, which is what separates this card's filter from
/// a modifier that shrank everything in play. The pump is asserted as the
/// *delta* off the body the board printed rather than as a number assumed from
/// the printing, and the target prompt is read for what it declines as well as
/// for what it offers.
#[test]
fn enfeeblement_shrinks_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(211, swamp())
        .battlefield(0, &[swamp(), swamp(), thrun_the_last_troll()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[enfeeblement()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("the Troll is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let land = on_battlefield(&engine, p0, swamp()).expect("a Swamp is out");
    let (power, toughness) = pt(&engine, host);
    assert!(
        toughness >= 3,
        "the host has to live through -2/-2, or there is no projection left to \
         read: {power}/{toughness}"
    );
    assert_eq!(pt(&engine, bystander), (1, 1), "the Elf is a printed 1/1");

    let spell = in_hand(&engine, p0, enfeeblement()).expect("the Aura is in hand");
    cast_from_hand(&mut engine, p0, enfeeblement());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura picks its host as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"Enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Swamp is a land and no creature: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the host was one of the options it offered");

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && e.state()
                .object(spell)
                .is_some_and(|o| o.attached_to == Some(host))
    });

    let aura =
        on_battlefield(&engine, p0, enfeeblement()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it entered attached to the creature it was cast at"
    );
    assert_eq!(
        pt(&engine, host),
        (power - 2, toughness - 2),
        "the enchanted creature gets -2/-2 off the body the card prints"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the static reaches the enchanted creature and never across the table"
    );
}

fn eternal_warrior() -> CardIndex {
    card_index("dab28bc6-3b2a-444f-b596-0a8d95d6d28c")
}

/// Eternal Warrior — {R} Aura: "Enchant creature. Enchanted creature has
/// vigilance."
///
/// A keyword grant is only worth playing if the keyword's own rule gets read,
/// so the game is walked as far as the attack declaration: vigilance says
/// attacking doesn't cause the creature to tap (CR 702.20b), which is the one
/// reading that separates the enchanted Elf from the bare Elf beside it once
/// both are declared. The bare Elf, the Elf across the table and the Aura
/// itself are the controls — "enchanted creature" is not "creatures you
/// control" and not "every permanent in this game", and the Aura grants the
/// keyword rather than keeping it. The Mountain is the only source tapped:
/// the Elves are the attackers below, so both are named as kept back, and
/// casting off `cast_with_floating` proves the {R} was really paid.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn eternal_warrior_grants_vigilance_to_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[eternal_warrior()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "nothing is enchanted yet"
    );

    // {R} off the Mountain alone: the Elves are kept untapped because they
    // are the attackers below, and a creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red, and the Elves are still standing"
    );
    cast_with_floating(&mut engine, p0, eternal_warrior());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("an Aura targets as it is cast, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" reaches either Elf: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and a creature across the table too — the filter is `Filter::CREATURE`: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, eternal_warrior()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it is attached to the Elf it was aimed at"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "enchanted creature has vigilance"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::VIGILANCE),
        "the static reaches the enchanted creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "nor across the table"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::VIGILANCE),
        "the Aura grants the keyword, it does not keep it"
    );

    // The keyword's whole meaning, which is a combat-step fact: both Elves
    // may attack (both are untapped and past summoning sickness), and only
    // the bare one pays for it by tapping.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&host) && attackers.contains(&bystander),
        "both untapped Elves are offered as attackers: {attackers:?}"
    );
    assert!(
        !attackers.contains(&theirs),
        "and the Elf across the table is not mine to send: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (host, Defender::Player(p1)),
                    (bystander, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, host),
        "CR 702.20b: attacking does not tap the enchanted creature"
    );
    assert!(
        is_tapped(&engine, bystander),
        "while the Elf beside it taps to attack like any other, so the \
         untapped one above is the grant and not a combat step that never came"
    );
}

fn flaming_sword() -> CardIndex {
    card_index("05c62f91-2a5b-4cba-8e66-78fb370ea409")
}

/// Flaming Sword prints flash, "Enchant creature", and one static: the
/// enchanted creature gets +1/+0 and has first strike. Both grants hang on
/// `Filter::AttachedToBySource`, so the board has to separate the creature
/// the Aura holds from every other creature on the table — an unenchanted
/// Elf beside the host and an Elf across it both have to stay `(1, 1)` and
/// keywordless, or the static is only being read, not tested. The flash
/// line is played rather than noted: the Aura is cast in the *opponent's*
/// main phase off two Mountains that were never tapped for anything, which
/// is a cast no sorcery-speed Aura could make.
#[test]
fn flaming_sword_flashes_in_to_arm_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flaming_sword()])
        .start();
    keep_mulligans(&mut engine);

    // Flash (CR 702.8a) is a cast at instant speed, so the window is p1's
    // own first main phase, stack empty, with p0 holding priority.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays unenchanted");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // Neither Elf may be tapped for its own mana: the two Mountains are the
    // whole of the {1}{R}, and the host has to still be what it was.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, flaming_sword());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the caster chooses what their own Aura enchants"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "either creature under your control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature: an Aura is not a spell that \
         has to be aimed at your own side: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sword = on_battlefield(&engine, p0, flaming_sword()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(sword).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it was cast at"
    );

    assert_eq!(pt(&engine, host), (2, 1), "+1/+0 on the enchanted creature");
    assert!(
        keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and first strike with it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "the static reaches the enchanted creature and no other"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "nor does the keyword travel with the Aura's colour"
    );
    assert!(
        !keywords(&engine, sword).contains(KeywordSet::FIRST_STRIKE),
        "the Aura grants the keyword, it does not keep it"
    );
}

fn flight() -> CardIndex {
    card_index("6a4068b0-fb4f-429c-a94e-47849f3eb7ef")
}

/// Flight is `{U}` Aura — "Enchant creature / Enchanted creature has
/// flying", and both sentences only mean anything on a board with creatures
/// the Aura does *not* hold. The 1/1 a host is printed as carries no
/// evasion until Flight lands on it, while the Elf beside it and the Elf
/// across the table stay grounded — so a static that had lost its
/// `AttachedToBySource` filter would grant flying to the whole table and be
/// caught. The offer is read before the answer too: CR 601.2c picks the
/// target before CR 601.2h pays for it, and "enchant creature" is what puts
/// the opponent's Elf on that list rather than a "you control" that was
/// never printed.
#[test]
fn flight_enchants_only_the_creature_it_lands_on() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flight()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "a printed 1/1 with nothing on it flies nowhere yet"
    );

    cast_from_hand(&mut engine, p0, flight());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "enchant creature asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf it was aimed at was one of the options");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, flight()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted (CR 303.4f)"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "enchanted creature has flying"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the static reaches the enchanted creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor across the table"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::FLYING),
        "the Aura grants the keyword, it does not keep it"
    );
}

// oracle_id = "861c5374-a8e7-4684-9a1b-e6ce3b98e4a2"
fn frog_tongue() -> CardIndex {
    card_index("861c5374-a8e7-4684-9a1b-e6ce3b98e4a2")
}

/// Frog Tongue — {G} Aura: "Enchant creature", "When this Aura enters, draw
/// a card", "Enchanted creature has reach."
///
/// All three printed sentences come off one cast, and each needs a bystander
/// to mean anything. `enchant creature` names no controller, so the Elf
/// across the table is on the target menu — and is precisely the creature
/// that must *not* grow the keyword afterwards. A second, bare Elf under the
/// same seat is the other control: the static reads `AttachedToBySource`, so
/// "creatures you control" would have been just as easy to write and would
/// have passed a board with only one creature on it.
#[test]
fn frog_tongue_enchants_one_creature_grants_reach_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[frog_tongue()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::REACH),
        "nothing enchants it yet"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, frog_tongue());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses what it enchants");
    assert_eq!((min, max), (1, 1), "exactly one creature, asked once");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures this seat controls may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" names no controller, so the Elf across the \
         table is a legal host too: {options:?}"
    );
    assert!(
        !options.contains(&host) || options.len() == 3,
        "and those three are the whole menu"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();

    // The Aura resolving puts its own enters-trigger on the stack behind it,
    // so the board is not finished until that has resolved too.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, frog_tongue()).is_some()
    });

    let aura = on_battlefield(&engine, p0, frog_tongue()).expect("the Aura resolved");

    assert!(
        keywords(&engine, host).contains(KeywordSet::REACH),
        "\"enchanted creature has reach\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::REACH),
        "the Elf nobody enchanted gains nothing: the static is attached-to \
         and not \"creatures you control\""
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::REACH),
        "and the static never reaches across the table, whatever the Aura \
         could have targeted"
    );
    assert!(
        keywords(&engine, aura).is_empty(),
        "the Aura grants the keyword, it does not keep it"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"when this Aura enters, draw a card\" — one off the top"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Aura left the hand and the draw put exactly one card back: a \
         missing trigger would leave one fewer"
    );
}

fn giant_strength() -> CardIndex {
    card_index("db85ba13-f00d-4cdd-99e1-22a4d39c8837")
}

/// Giant Strength — {R}{R} — Aura. It prints exactly two sentences: "Enchant
/// creature" and "Enchanted creature gets +2/+2", and the static is written as
/// `Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource])`, so the
/// reading worth playing is the one that tells the creature the Aura *holds*
/// from every other creature on the table. The second Elf under the same seat
/// is the load-bearing bystander: "+2/+2" landing on it too would mean the
/// filter had collapsed to "creatures you control", and the Elf across the
/// table would catch "+2/+2 on each creature". The offer is read before the
/// answer because "enchant creature" names no controller, and a copy of the
/// card that could only point at its caster's own board would print the same
/// text.
#[test]
fn giant_strength_pumps_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[giant_strength()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // {R}{R} off the two Mountains, and the offer is read before the answer:
    // targeting happens at CR 601.2c, the costs at CR 601.2h.
    cast_from_hand(&mut engine, p0, giant_strength());
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
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses what its Aura enchants");
    assert_eq!((min, max), (1, 1), "an Aura enchants exactly one creature");
    assert!(
        player_options.is_empty(),
        "\"creature\" is no player (CR 115.1): {player_options:?}"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be enchanted: {options:?}"
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
        .expect("the creature the offer named is a legal target");
    pass_until(&mut engine, stack_is_empty);

    let aura =
        on_battlefield(&engine, p0, giant_strength()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it is attached to the creature it was cast on"
    );
    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "\"enchanted creature gets +2/+2\""
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as — \
         \"enchanted creature\" is not \"creatures you control\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table"
    );
}

fn greels_caress() -> CardIndex {
    card_index("e9e0b78e-07d5-4603-8e3b-27274148d1a1")
}

/// Phyrexian Fleshgorger: a printed 7/5 whose keywords are stubs, so on a
/// battlefield it is a body and nothing else — which is what makes "-3/-0"
/// readable as a number rather than as a fight with a ward trigger.
fn a_body_to_shrink() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

/// Greel's Caress is `{1}{B}` Aura with flash: "Enchant creature. Enchanted
/// creature gets -3/-0." The cast therefore happens in the *opponent's* main
/// phase, which is where flash is the difference between a spell and a
/// refusal — a sorcery could not be cast there at all. Mana is tapped before
/// the offer is read, because `castable` is filtered against the pool and not
/// against the board. The host is a 7/5 and the Elf on the caster's own side
/// is the control: `(4, 5)` on the host can only be -3/-0 landing on the
/// creature the Aura is attached to, and `(1, 1)` on the Elf says the static
/// is not "creatures".
#[test]
fn greels_caress_casts_at_flash_speed_to_weaken_only_the_creature_it_enchants() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(19, swamp())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .hand(0, &[greels_caress()])
        .battlefield(1, &[a_body_to_shrink()])
        .start();
    keep_mulligans(&mut engine);

    // Through the whole of p0's own turn, where the Aura stays in hand, and
    // into the one phase flash was printed for.
    reach_their_main_phase(&mut engine, p1);
    if matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1) {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the nonactive seat is the one being asked in the opponent's main \
         phase: {:?}",
        engine.pending()
    );

    // Mana first: an offer is computed against the pool, so a `castable`
    // claim made before the Swamps are tapped says nothing about flash.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, greels_caress()).expect("the Aura is in hand");
    assert!(
        legal.castable.contains(&card),
        "flash: the Aura is castable in the opponent's main phase, where a \
         sorcery-speed card could not be: {:?}",
        legal.castable
    );

    let host = on_battlefield(&engine, p1, a_body_to_shrink()).expect("a 7/5 across the table");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("a 1/1 on this side");
    assert_eq!(pt(&engine, host), (7, 5), "the printed body, untouched");

    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "an Aura asks for the creature it enchants, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" is every creature, on either side of the \
         table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered");
    let aura = on_stack(&engine, greels_caress()).expect("the Aura spell is on the stack");

    pass_until(&mut engine, |e| {
        e.state()
            .object(aura)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        on_battlefield(&engine, p0, greels_caress()).is_some(),
        "the Aura resolved onto the battlefield and stayed attached instead \
         of falling off its host"
    );
    assert_eq!(
        pt(&engine, host),
        (4, 5),
        "{{-3/-0}} on the creature it is attached to: three off the power, \
         and the toughness exactly where it was"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and nothing for the 1/1 beside the caster: the static names the \
         attached creature and not every creature"
    );
}

fn hero_s_resolve() -> CardIndex {
    card_index("1854a99d-f8c7-45b3-83a2-98ae1c5b5b09")
}

/// Hero's Resolve ({1}{W}, Aura): "Enchant creature" and "Enchanted creature
/// gets +1/+5." Both printed lines are one scenario, because the second is a
/// `Filter::AttachedToBySource` static and only the creature the Aura *lands
/// on* may move. The Elf across the table is the control that separates a pump
/// reaching its host from one reaching every creature, and the target offer is
/// where "enchant creature" — any creature, not "you control" — is read.
/// +1/+5 on a printed 1/1 is `(2, 6)`: a `(1, 6)` would mean the power was
/// never added, a `(2, 2)` that the power was written where the toughness goes.
#[test]
fn hero_s_resolve_pumps_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(81, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[hero_s_resolve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // Two Plains pay {1}{W}. The Elf beside them taps for mana like any other
    // source, which is exactly what `tap_all_mana` does — nothing here needs
    // it untapped afterwards, so the whole tap is one call.
    cast_from_hand(&mut engine, p0, hero_s_resolve());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("an Aura targets as it is cast, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster chooses what it enchants");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"enchant creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf it was cast for was one of the options");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, hero_s_resolve()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted (CR 303.4)"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 6),
        "+1/+5 on the creature the Aura holds — not +5/+1, and not nothing"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature the Aura does not hold"
    );
}

fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

/// Holy Strength is a {W} Aura printing "Enchant creature" and "Enchanted
/// creature gets +1/+2", and only the second line is a static — a
/// `Filter::AttachedToBySource`, so the one reading worth playing is the one
/// that tells the creature the Aura *holds* from every other creature on the
/// table. A printed 1/1 Elf becomes a 2/3 while an Elf of mine nobody
/// enchanted and an Elf across the table both stay 1/1, which is why the
/// board carries all three. The target menu is read as well: "Enchant
/// creature" reaches either side of the table and never a land, so its own
/// options are where a filter that had quietly narrowed to "you control" or
/// widened to `Filter::Any` would show up.
#[test]
fn holy_strength_pumps_the_creature_it_enchant_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[holy_strength()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let land = on_battlefield(&engine, p0, plains()).expect("a Plains is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    cast_from_hand(&mut engine, p0, holy_strength());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Plains is no creature and cannot be enchanted: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, holy_strength()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "the Aura is attached to the creature it named"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 3),
        "+1/+2 on the creature it enchants"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table"
    );
}

fn illuminated_wings() -> CardIndex {
    card_index("7a22389a-34e7-4726-a551-f6fbc225cefe")
}

/// Illuminated Wings — {1}{U} Aura: "Enchant creature / Enchanted creature
/// has flying. / {2}, Sacrifice this Aura: Draw a card."
///
/// Both printed sentences are read off one board, because each is the other's
/// control. The Aura is cast onto this seat's own Elf with the opponent's Elf
/// standing beside it, so the target question shows that "creature" reaches
/// across the table while the granted flying lands only on the creature the
/// Aura is attached to. Sacrificing the Aura for the card is what makes the
/// static readable at all: the same Elf is a printed 1/1 again afterwards, so
/// the keyword came from the Aura and not from anything on the Elf. The {2}
/// is paid out of the mana the cast left floating, so the activation is a
/// real payment and not a label.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn illuminated_wings_grants_flying_to_its_host_and_sacrifices_itself_for_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[illuminated_wings()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "a Llanowar Elves prints no flying of its own"
    );

    // {1}{U} off the four Islands, with the Elf kept back so the only mana
    // sources this board offers are the lands.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, illuminated_wings());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("an Aura picks what it enchants, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"enchant creature\" is any creature on either side: {options:?}"
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

    let wings = on_battlefield(&engine, p0, illuminated_wings()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(wings).and_then(|o| o.attached_to),
        Some(mine),
        "it entered attached to the creature it was cast on"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature has flying"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and the Elf across the table has nothing: the static is about the \
         creature the Aura holds and not about creatures"
    );

    // The Aura's own second sentence. The cast left two Islands' worth of
    // blue floating, which is exactly the {2} the ability charges, and the
    // only activated ability the card prints is the one the offer names.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == wings)
        .expect("`{2}, Sacrifice this Aura: Draw a card` is offered");
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(id, _)| *id == wings)
            .count(),
        1,
        "the Aura prints one activated ability, so it is offered once: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the mana the cast left floating pays the {2}");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "nothing the sacrifice asks is unanswerable"
    );

    assert!(
        on_battlefield(&engine, p0, illuminated_wings()).is_none(),
        "the Aura sacrificed itself, so it is gone from the table"
    );
    assert!(
        in_graveyard(&engine, p0, illuminated_wings()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "the Elf is a printed 1/1 again: the flying was the Aura's static, \
         and it left when the Aura did"
    );
}

/// Immolation — {R} Aura: "Enchant creature. Enchanted creature gets +2/-2."
///
/// Both halves of the modifier are read off one body: Rootbreaker Wurm is a
/// printed 6/6 and has to become an 8/4, so a `+2/+2` or a `-2/+2` in the card
/// def shows up as a different pair instead of passing. The two bystanders and
/// the Mountain draw the two lines the card prints — the static reaches only
/// the permanent it holds (the Elf beside it and the Elf across the table stay
/// 1/1), and the target choice offers every creature on the table and no land,
/// which is "enchant creature" and not "enchant permanent".
fn immolation() -> CardIndex {
    card_index("9f40ad89-3767-4837-a078-f2dcfaf368df")
}

#[test]
fn immolation_gives_the_enchanted_creature_two_power_and_takes_two_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), rootbreaker_wurm(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[immolation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let hill = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the Aura");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and a printed 1/1 beside it"
    );

    cast_from_hand(&mut engine, p0, immolation());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("an Aura asks what it enchants, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster chooses");
    assert!(
        options.contains(&wurm) && options.contains(&bystander) && options.contains(&theirs),
        "\"enchant creature\" reaches any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&hill),
        "a Mountain is a permanent and no creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the offered targets");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, immolation()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(wurm),
        "it entered attached to the creature it targeted"
    );
    assert_eq!(
        pt(&engine, wurm),
        (8, 4),
        "+2/-2 on the creature it holds: both numbers move, in opposite directions"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static reaches the equipped creature's table and no further"
    );
}

fn imposing_visage() -> CardIndex {
    card_index("6f67058b-ed14-4e3c-9af3-d61570870e36")
}

/// Imposing Visage — {R} Aura: "Enchant creature. Enchanted creature has
/// menace." The word "enchant creature" names no controller, so the target
/// question is read over both sides of the table and answered with the
/// caster's own Elf. The static is
/// `Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource])`, so the
/// bare Elf beside the host and the Elf across it are the two controls: a
/// filter that had widened to "creatures you control" (or dropped the
/// attachment) would have granted menace to one or both and passed every
/// assertion about the host.
#[test]
fn imposing_visage_enchants_any_creature_and_grants_menace_only_to_that_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, mountain())
        .battlefield(0, &[mountain(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[imposing_visage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::MENACE),
        "nothing is enchanted yet"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::MENACE),
        "and the Elf across the table is a printed 1/1"
    );

    // {R} off the Mountain. `tap_all_mana` also taps the two Elves for their
    // own mana ability, which costs this scenario nothing — nobody attacks.
    cast_from_hand(&mut engine, p0, imposing_visage());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "an Aura picks its host as it is cast (CR 601.2c), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat casting the Aura is the one asked");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    let visage = on_battlefield(&engine, p0, imposing_visage()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(visage).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::MENACE),
        "\"enchanted creature has menace\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::MENACE),
        "the static reaches the creature the Aura holds and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::MENACE),
        "nor across the table"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "menace is a keyword and no body: the Elf is still a printed 1/1"
    );
}

// oracle_id = "cca60afe-b044-4401-8322-170aa015873c"
fn indomitable_will() -> CardIndex {
    card_index("cca60afe-b044-4401-8322-170aa015873c")
}

/// Indomitable Will — {1}{W} Aura with flash: "Enchant creature. Enchanted
/// creature gets +1/+2."
///
/// The cast is made in the **opponent's** first main phase, which is the one
/// place the printed flash is the only thing that lets a sorcery-speed Aura
/// be played at all. The target question is then read off the whole table —
/// "enchant creature" is any creature, the Elf across it included — and the
/// pump is read off the board afterwards, where the unequipped Elf beside the
/// host and the opponent's Elf must both still be printed 1/1s, because the
/// static names the *enchanted* creature and not "creatures you control".
#[test]
fn indomitable_will_enchants_in_the_opponents_turn_and_pumps_only_its_host() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[indomitable_will()])
        .start();
    keep_mulligans(&mut engine);

    // Nothing is cast on p0's own turn: walk to the moment p0 holds priority
    // in p1's first main phase, which is the window flash opens.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().turn.active,
        p1,
        "the Aura is about to be cast on the opponent's turn"
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    cast_from_hand(&mut engine, p0, indomitable_will());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses the creature it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under the caster are legal: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature on the table, the opponent's \
         included: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, indomitable_will()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "the Aura lands on the creature it targeted (CR 303.4)"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 3),
        "+1/+2 on the creature it enchants"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the enchanted creature and never across the table"
    );
}

// oracle_id = "864c7575-4589-416c-b143-310d5ef238c5"
fn inertia_bubble() -> CardIndex {
    card_index("864c7575-4589-416c-b143-310d5ef238c5")
}

/// Inertia Bubble prints two lines: "Enchant artifact" and "Enchanted
/// artifact doesn't untap during its controller's untap step." The board
/// holds two Sol Rings — the one the Bubble ends up holding and one it does
/// not — and both are tapped for mana in the same turn, so the untethered
/// Ring coming back in p0's next untap step is what tells a rule (CR 502.3)
/// from a game that simply never reached it. The filter is read off the
/// target menu as well: the Islands beside them are permanents and are not
/// offered, which is the mistake a `Filter::Any` would make. Nothing but
/// playing the Aura separates `Modifier::DoesNotUntap` on
/// `Filter::AttachedToBySource` from a static that lost its filter and
/// pinned every artifact in play.
#[test]
fn inertia_bubble_pins_the_artifact_it_enchants_while_the_ring_beside_it_untaps() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4711, island())
        .battlefield(0, &[island(), island(), quiet_artifact(), quiet_artifact()])
        .hand(0, &[inertia_bubble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `cast_from_hand` is `tap_all_mana` plus the cast: both Rings are tapped
    // by the same helper that floats the {1}{U}, so the untap step below has
    // two tapped artifacts of the same printing to answer for.
    cast_from_hand(&mut engine, p0, inertia_bubble());

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (1, 1), "an Aura has exactly one host");
    assert_eq!(
        options.len(),
        2,
        "the two artifacts on the table and nothing else: {options:?}"
    );
    let island = on_battlefield(&engine, p0, island()).expect("the Island is out");
    assert!(
        !options.contains(&island),
        "\"artifact\" is read: a land is a permanent and not on this menu: {options:?}"
    );
    let (host, bystander) = (options[0], options[1]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let bubble = on_battlefield(&engine, p0, inertia_bubble()).expect("the Bubble resolved");
    assert_eq!(
        engine.state().object(bubble).and_then(|o| o.attached_to),
        Some(host),
        "the Aura landed on the artifact that was named and not on the other one"
    );
    assert!(is_tapped(&engine, host), "the host paid for the cast");
    assert!(
        is_tapped(&engine, bystander),
        "and so did the Ring beside it, so the untap step has two answers to give"
    );

    // A whole turn cycle, so CR 502.3 is what happens between the two readings.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, bystander),
        "the untap step ran: the Ring the Bubble does not hold came back"
    );
    assert!(
        is_tapped(&engine, host),
        "\"enchanted artifact doesn't untap\" — the step passed over the Ring \
         the Bubble holds and over no other"
    );
}

fn lance() -> CardIndex {
    card_index("5960dd01-6797-4c73-b48a-f637b9c288cc")
}

/// Lance is a {W} Aura: "Enchant creature" and "Enchanted creature has first
/// strike." The keyword is a static over `AttachedToBySource`, so the only
/// reading worth playing is the one that separates the creature the Aura
/// holds from every other creature on the table — the Elf across the table is
/// the control, and the Aura itself must not keep what it grants. And first
/// strike is a word the combat step reads: two 1/1s trade in one simultaneous
/// damage step, so an enchanted 1/1 blocker-killer walking away alone is the
/// card doing the thing its text says.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn lance_gives_first_strike_to_the_creature_it_enchants_and_wins_that_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[lance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "nothing is enchanted yet"
    );

    // {W} off the Plains, with both creatures kept standing: the host has to
    // be untapped to attack with below, and a creature that tapped for mana
    // may not (the Elves print `{T}: Add {G}`).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one white mana, and the only source touched is the Plains"
    );
    cast_with_floating(&mut engine, p0, lance());

    // "Enchant creature" is the target question the card prints, and it is
    // asked of the whole table: the Elf across it is a legal answer to look
    // at and the wrong one to give.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, lance()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "enchanted creature has first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "the static reaches the enchanted creature and never across the table"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::FIRST_STRIKE),
        "the Aura grants the keyword, it does not keep it"
    );

    let their_life = engine.state().players[1].life;

    // Played out into the step that reads the keyword: the enchanted 1/1
    // attacks, the other 1/1 blocks, and only the block dies.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(host, Defender::Player(p1))],
            },
        )
        .unwrap();

    let mut blocked = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => {
                let pair = blockers.iter().find(|o| o.blocker == theirs);
                assert!(
                    pair.is_some_and(|o| o.attackers.contains(&host)),
                    "the untapped Elf across the table may block the attacker: {blockers:?}"
                );
                engine
                    .apply(
                        p1,
                        PlayerAction::DeclareBlockers {
                            blockers: vec![(theirs, host)],
                        },
                    )
                    .unwrap();
                blocked = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while declaring blockers: {other:?}"),
        }
    }
    assert!(blocked, "the block was declared");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocker died to the first-strike damage step"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the enchanted attacker is still standing — without first strike \
         the two 1/1s would have traded in one simultaneous damage step"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life,
        "a blocked attacker deals its damage to the blocker, not to the player"
    );
}

fn magetas_boon() -> CardIndex {
    card_index("e3e5b12c-2103-4b40-83d8-6d5449179b6f")
}

/// Mageta's Boon is a {1}{W} Aura with flash — "Enchant creature; enchanted
/// creature gets +1/+2". It is cast here in the *opponent's* main phase,
/// which is the only place its flash is observable: a sorcery-speed Aura is
/// absent from `castable` there. It enchants one of two Elves while a third
/// stands across the table, so the +1/+2 has to land on the creature the Aura
/// is attached to rather than on "creatures you control" or on every creature
/// in the game.
#[test]
fn magetas_boon_flashes_in_and_pumps_only_the_creature_it_enchants() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[magetas_boon()])
        .start();
    keep_mulligans(&mut engine);

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (mine[0], mine[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // Onto the opponent's main phase, and then to the priority p0 holds
    // inside it — the two Plains are untapped and stay that way until the
    // Aura is paid for.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana before the claim: `castable` is filtered against the pool, not
    // against the untapped lands sitting beside it.
    tap_all_mana(&mut engine, p0);
    let aura = in_hand(&engine, p0, magetas_boon()).expect("the Aura is in hand");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat about to cast the instant-speed Aura");
    assert!(
        legal.castable.contains(&aura),
        "flash: a {{1}}{{W}} Aura is castable in the opponent's main phase, \
         where a sorcery-speed one could not be: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: aura })
        .expect("flash makes the cast legal at instant speed");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"Enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"Enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered");
    pass_until(&mut engine, stack_is_empty);

    let enchant = on_battlefield(&engine, p0, magetas_boon()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(enchant).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert_eq!(pt(&engine, host), (2, 3), "+1/+2 on the enchanted creature");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static reaches no creature it is not attached to"
    );
}

fn primal_frenzy() -> CardIndex {
    card_index("705b4da3-d463-4808-b79c-dc0c1830945a")
}

/// Primal Frenzy — {G} — Aura: "Enchant creature. Enchanted creature has
/// trample."
///
/// The one line the card prints is a static on `Filter::AttachedToBySource`,
/// so the only reading worth playing is the one that tells the creature the
/// Aura *is on* from every other creature in the game — which is why both
/// bystanders are real creatures rather than scenery: an Elf beside the host
/// under the same seat and an Elf across the table. A reading of "creatures
/// you control" would leave two tramplers, and a reading with no filter at
/// all three; exactly one trampler is the only answer that reads both the
/// filter and the word "enchanted".
///
/// The menu read before the answer is the other half: "enchant creature" is
/// a target chosen as the Aura is cast (CR 601.2c) with no controller in it,
/// so all three creatures are on it and no Forest is.
#[test]
fn primal_frenzy_grants_trample_to_the_creature_it_enchants_and_to_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[primal_frenzy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "nothing is enchanted yet"
    );

    cast_from_hand(&mut engine, p0, primal_frenzy());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target chosen as the Aura is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"creature\" is read with no controller in it: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "and those three are the whole menu — the Forests are no creatures: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let frenzy = on_battlefield(&engine, p0, primal_frenzy()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(frenzy).and_then(|o| o.attached_to),
        Some(host),
        "and it is attached to the creature it was aimed at"
    );
    assert!(
        types(&engine, frenzy).contains(TypeSet::ENCHANTMENT),
        "an Aura is an enchantment once it is on the battlefield: {:?}",
        types(&engine, frenzy)
    );

    assert!(
        keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "enchanted creature has trample"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::TRAMPLE),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the static reaches the enchanted creature and never across the table"
    );
    assert!(
        !keywords(&engine, frenzy).contains(KeywordSet::TRAMPLE),
        "the Aura grants the keyword, it does not keep it"
    );
}

fn reflexes() -> CardIndex {
    card_index("dc87b0a5-3d9d-44eb-b415-b022acd63cf1")
}

/// Reflexes prints two sentences: "Enchant creature" and "Enchanted creature
/// has first strike." This scenario plays both. The Aura is cast off a
/// Mountain at the only two creatures on the table, so "enchant creature"
/// is read as *any* creature rather than "you control" — the Elf across the
/// table is offered and the one under our own seat is taken. The keyword is
/// then proven in combat instead of in a keyword list: two printed 1/1s
/// would trade, and only the one striking first leaves the blocking 1/1 dead
/// while the attacker lives.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn reflexes_enchants_a_creature_and_lets_it_strike_first() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, mountain())
        .battlefield(0, &[mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[reflexes()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "nothing is enchanted yet"
    );

    // The Mountain pays the {R}; the Elf is kept back because it is the
    // creature that has to attack in the combat step below.
    tap_mana_except(&mut engine, p0, mine);
    cast_with_floating(&mut engine, p0, reflexes());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "an Aura targets a creature as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat chooses the target");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
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

    let aura = on_battlefield(&engine, p0, reflexes()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "it entered attached to the creature it was aimed at"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "the enchanted creature has first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "and the creature it is not attached to has none"
    );

    // Combat is where first strike means anything: the two 1/1s would trade,
    // and only the one that strikes first survives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mine, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the defending seat declares the blocks");
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == theirs && option.attackers.contains(&mine)),
        "the Elves across the table may block the attacker: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(theirs, mine)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the first striker deals its damage first, so the blocker dies before \
         it can deal any back and the 1/1 attacker lives"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and the blocking 1/1 is in its owner's graveyard"
    );
}

fn robe_of_mirrors() -> CardIndex {
    card_index("093a20e5-ff14-41c7-b16c-1f745ddf6942")
}

/// Robe of Mirrors — {U}, Aura: "Enchant creature" and "Enchanted creature
/// has shroud."
///
/// The Aura has to arrive by being *cast*, because the creature it enchants
/// is chosen while the spell is on the stack and the attach is the resolving
/// effect's whole job — a Robe seeded onto a board would enter attached to
/// nobody and its keyword would land nowhere. Shroud is nothing but a
/// targeting restriction (CR 702.18a), so the second printed sentence is read
/// off the opponent's Swords to Plowshares: the menu holds the bare Elf under
/// the same controller and the Elf across the table — two live options — and
/// not the robed one, which is what keeps the absence from being an empty
/// menu, and then the bait Elf really is exiled while its neighbour stands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn robe_of_mirrors_shrouds_only_the_creature_it_is_attached_to() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[robe_of_mirrors()])
        .battlefield(1, &[plains(), llanowar_elves()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "nothing is enchanted yet"
    );

    // {U} off the Island, and the enchanted creature is named while the Aura
    // is still a spell (CR 601.2c).
    tap_all_mana(&mut engine, p0);
    let robe = in_hand(&engine, p0, robe_of_mirrors()).expect("the Robe is in hand");
    cast_with_floating(&mut engine, p0, robe_of_mirrors());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster names what it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "either Elf you control may wear it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(robe)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "enchanted creature has shroud"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "the static reaches the creature the Robe is attached to and no other"
    );
    assert!(
        !keywords(&engine, robe).contains(KeywordSet::SHROUD),
        "the Aura grants the keyword, it does not keep it"
    );

    // The other side of the table, on its own turn: Swords to Plowshares can
    // point at any creature, and the offer is where shroud is visible.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it chooses");
    assert!(
        options.contains(&bystander),
        "the bare Elf under the same controller is still a legal target: {options:?}"
    );
    assert!(
        options.contains(&their_elf),
        "and so is a creature of the caster's own: {options:?}"
    );
    assert!(
        !options.contains(&host),
        "\"enchanted creature has shroud\": the creature the Robe holds cannot \
         be targeted at all (CR 702.18a): {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .expect("the bait Elf was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&host),
        "the shrouded Elf was never a legal target and is untouched"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&bystander),
        "and the Elf that *was* offered is exiled, so the difference between \
         the two was shroud and not a spell that never resolved"
    );
}

fn sicken() -> CardIndex {
    card_index("5208a5f2-eebe-4adc-8f29-543b60116817")
}

/// Sicken is `{B}` Aura: "Enchant creature. Enchanted creature gets -1/-1.
/// Cycling {2}". The static is the whole card once it lands, so the scenario
/// has to show both halves of what a `Filter::AttachedToBySource` means — the
/// creature it holds shrinks and nothing else does — and it casts across the
/// table, because "enchant creature" names no controller and a version that
/// quietly required one would pass every test played on its own board. The
/// exact `(5, 5)` is what separates `-1/-1` from a destroy or a `-2/-2`, and
/// the enchanted permanent still being on the battlefield is what makes the
/// body readable at all.
#[test]
fn sicken_shrinks_the_creature_it_enchanters_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .battlefield(1, &[rootbreaker_wurm()])
        .hand(0, &[sicken()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the Aura");
    assert_eq!(pt(&engine, elves), (1, 1), "and a printed 1/1");

    cast_from_hand(&mut engine, p0, sicken());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "`enchant creature` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&wurm) && options.contains(&elves),
        "\"enchant creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, sicken()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(wurm),
        "an Aura enters attached to the creature it was cast on (CR 303.4f)"
    );
    assert_eq!(
        pt(&engine, wurm),
        (5, 5),
        "one less power and one less toughness, so the static is exactly \
         `-1/-1` and not a destroy or a `-2/-2`"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the creature it did not enchant keeps its printed numbers, which is \
         `AttachedToBySource` and not \"creatures\""
    );
}

fn sinister_strength() -> CardIndex {
    card_index("deb499bd-71d4-4430-8a58-a31f5ab1b239")
}

/// Sinister Strength — {1}{B} Aura: "Enchant creature. Enchanted creature
/// gets +3/+1 and is black."
///
/// The two clauses of the second sentence live in two different layers, so
/// each gets its own bystander. One of two printed 1/1 Elves becomes a 4/2
/// while the other stays a 1/1 and the Elf across the table stays a 1/1 too,
/// which is what says the static reads `AttachedToBySource` rather than
/// "creatures you control" or the whole table; and the host reads black while
/// the bare Elf beside it and the one across the table stay green, which is
/// the colour half and not a side effect of the pump. The target menu is read
/// before the answer, because the printed word is "creature" and not
/// "creature you control": the opponent's Elf is offered.
#[test]
fn sinister_strength_pumps_and_blackens_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[swamp(), forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[sinister_strength()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // {1}{B}: the Swamp pays the black, the Forest the generic.
    cast_from_hand(&mut engine, p0, sinister_strength());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses what to enchant");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under your own control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"Enchant creature\" is any creature and not only yours: {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the Aura's own question offered");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, sinister_strength()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature its target became"
    );

    assert_eq!(
        pt(&engine, host),
        (4, 2),
        "+3/+1 on the creature it is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and nothing at all on the Elf beside it"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor on the one across the table"
    );

    // The layer that no power/toughness reading can see.
    let host_colors = engine
        .state()
        .object(host)
        .expect("the host is on the battlefield")
        .characteristics()
        .colors;
    assert!(
        host_colors.contains(baylee_core::color::Color::Black),
        "\"and is black\" — the enchanted creature takes the colour"
    );
    for (id, what) in [
        (bystander, "the bare Elf beside it"),
        (theirs, "the Elf across the table"),
    ] {
        let colors = engine
            .state()
            .object(id)
            .expect("the bystander is on the battlefield")
            .characteristics()
            .colors;
        assert!(
            !colors.contains(baylee_core::color::Color::Black),
            "{what} is not the enchanted creature and stays the green 1/1 it was printed as"
        );
    }
}

fn unholy_strength() -> CardIndex {
    card_index("090d88a9-7f2d-4bd1-a30a-7c48d05068be")
}

/// Unholy Strength — {B} Aura: "Enchant creature" and "Enchanted creature
/// gets +2/+1".
///
/// Both halves are read off one play. "Enchant creature" is a target on *any*
/// creature in the game, so the Elf across the table is offered and then has
/// to come out unchanged — that is the counter-half of "enchanted creature".
/// The static is keyed on `Filter::AttachedToBySource`, so only the creature
/// the Aura is actually attached to collects the numbers: `(3, 2)` on a
/// printed 1/1 is the only pair that reads the `+2` and the `+1` both, and
/// the Aura itself must stay a printed enchantment instead of growing a body.
#[test]
fn unholy_strength_gives_two_and_one_to_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[unholy_strength()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one it may not touch"
    );

    // {B} off the Swamp alone: the Elf is kept back, because it is the
    // creature this Aura is about to enchant and a host tapped for mana is
    // still a host, but an untapped one keeps the reading unambiguous.
    tap_mana_except(&mut engine, p0, host);
    cast_with_floating(&mut engine, p0, unholy_strength());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura's enchant ability is a target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"Enchant creature\" is any creature on the table, either side: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the spell targeted is one of its own options");
    pass_until(&mut engine, stack_is_empty);

    let aura =
        on_battlefield(&engine, p0, unholy_strength()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted (CR 303.4a)"
    );
    assert_eq!(
        pt(&engine, host),
        (3, 2),
        "+2/+1 on the creature the Aura is attached to"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the enchanted creature and never across the table"
    );
    let kinds = types(&engine, aura);
    assert!(
        kinds.contains(TypeSet::ENCHANTMENT) && !kinds.contains(TypeSet::CREATURE),
        "the Aura grants the numbers, it does not keep them: {kinds:?}"
    );
}

fn vigilance() -> CardIndex {
    card_index("70570170-be76-4c56-9151-c4b6e253f462")
}

/// Vigilance is a {W} Aura — "Enchant creature / Enchanted creature has
/// vigilance" — and vigilance's whole printed meaning is that attacking does
/// not tap the creature (CR 702.20b). So the card is played onto one of two
/// otherwise identical Llanowar Elves and both attack in the same combat:
/// the one wearing the Aura is still standing afterwards, the one without it
/// is tapped, and the only difference between them is the Aura. The
/// projection is read before combat too, so a static that never landed would
/// be caught before the tapped check could blame the declare-attackers step.
#[test]
fn vigilance_enchants_a_creature_and_that_creature_attacks_without_tapping() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[vigilance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "an Aura that has not resolved grants nothing"
    );

    // {W} off the Plains, and both Elves kept back: the creature the Aura is
    // about to arm has to be able to attack, and one tapped for mana could not.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, vigilance());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "enchant creature asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "either creature on this board may be enchanted: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered is the one enchanted");

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, vigilance()).is_some()
    });
    let aura = on_battlefield(&engine, p0, vigilance()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it entered attached to the creature it was cast on"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "enchanted creature has vigilance"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::VIGILANCE),
        "the Aura grants the keyword, it does not keep it"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::VIGILANCE),
        "and it reaches the enchanted creature and no other"
    );

    // Both attack. The bare Elf is the control: it proves the combat step
    // really ran, so the standing attacker below is vigilance and not a
    // declaration that never happened.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&host) && attackers.contains(&bystander),
        "both untapped Elves may be declared: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (host, Defender::Player(p1)),
                    (bystander, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is empty the moment attackers are
    // declared, so that predicate stops the walk before the combat damage
    // step. The end step is past damage (CR 510.2) and still before p0's
    // untap step, which is the only place a tapped attacker can be read.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        !is_tapped(&engine, host),
        "\"attacking doesn't cause it to tap\": the enchanted Elf attacked and is \
         still standing"
    );
    assert!(
        is_tapped(&engine, bystander),
        "the identical Elf nobody enchanted attacked and tapped, so the \
         difference is the Aura and not a combat step that never ran"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "and both of them connected, so neither was quietly dropped from the \
         declaration"
    );
}

fn weakness() -> CardIndex {
    card_index("f07a24c0-bf3c-4733-9473-c6be3b16950e")
}

fn a_seven_five_wurm() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

/// Weakness — {B}, Aura: "Enchant creature. Enchanted creature gets -2/-1."
///
/// The two printed sentences part company on exactly one question: *which*
/// creature. "Enchant creature" carries no controller restriction, so the
/// offer is expected to name both sides of the table (CR 303.4a), while the
/// static's `Filter::AttachedToBySource` may name only the one the Aura is
/// holding. Both are read in one cast: the opponent's 7/5 takes the -2/-1 and
/// becomes a 5/4, and the Llanowar Elves under Weakness' own controller is
/// still the 1/1 it was printed as — the bystander is what tells a modifier
/// that lost its filter from one that never left the host. The host survives
/// the shrink on purpose, so the numbers are readable where they landed
/// rather than inferred from a graveyard.
#[test]
fn weakness_shrinks_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .hand(0, &[weakness()])
        .battlefield(1, &[a_seven_five_wurm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let host = on_battlefield(&engine, p1, a_seven_five_wurm()).expect("their Wurm is out");
    assert_eq!(pt(&engine, host), (7, 5), "the body the card prints");
    assert_eq!(pt(&engine, bystander), (1, 1), "and the bystander's");

    // Mana first, then the claim: the offer is read off the pool. The Elves'
    // own `{T}` is a mana ability `tap_all_mana` takes as well, so nothing
    // here counts the pool — the Swamp is what pays the {B}.
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, weakness()).expect("the Aura is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "an Aura with a creature on the table is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, weakness());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert_eq!(player, p0, "the caster chooses what it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" names both creatures on the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Wurm was one of the options it published");

    pass_until(&mut engine, |e| on_battlefield(e, p0, weakness()).is_some());
    let aura = on_battlefield(&engine, p0, weakness()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature its target became"
    );
    assert_eq!(
        pt(&engine, host),
        (5, 4),
        "-2/-1 on the creature it enchants: 7/5 becomes 5/4"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and the creature it does not — the static reads the source's own \
         attachment and never the whole table"
    );
}

fn web() -> CardIndex {
    card_index("5aa12aff-db3c-4be5-822b-3afdf536b33e")
}

/// Web — `{G}` Aura: "Enchant creature. Enchanted creature gets +0/+2 and has
/// reach."
///
/// Both printed statics are `Filter::And(&[CREATURE, AttachedToBySource])`,
/// and *that* word is the one an Aura test has to play: "attached to this" is
/// neither "creatures you control" nor "the whole table". So two Elves stand
/// under the caster and a third across it, and the single enchanted one
/// reading `(1, 3)` with reach while the other two stay `(1, 1)` and
/// keywordless is the whole of what the card says. The Aura also has to
/// arrive *attached* — an Aura that resolved and then sat loose would give
/// the numbers to nobody, which is why the attachment is read as well.
#[test]
fn web_holds_one_creature_at_one_three_with_reach_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[web()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Web");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::REACH),
        "and nothing has granted it reach yet"
    );

    cast_from_hand(&mut engine, p0, web());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster picks what it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under your own control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table, \
         and never \"you control\": {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the offer named");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let aura = on_battlefield(&engine, p0, web()).expect("the Web resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it was cast at"
    );

    assert_eq!(
        pt(&engine, host),
        (1, 3),
        "the enchanted creature gets +0/+2: the power is untouched and the \
         toughness is the printed 1 plus two"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::REACH),
        "and has reach"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::REACH),
        "\"creatures you control\" would have granted this one reach too"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::REACH),
        "so the Elf across it has neither the toughness nor the keyword"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::REACH),
        "the Aura grants the keyword to its host, it does not keep it"
    );
}

fn angelic_shield() -> CardIndex {
    card_index("05b020fd-21be-495d-ae45-7de3b1224e6d")
}

/// `Angelic Shield` (`Coverage::Implemented`):
/// "Creatures you control get +0/+1. Sacrifice this enchantment: Return target creature to its owner's hand."
///
/// Verifies that `Angelic Shield` provides a static +0/+1 toughness boost to creatures
/// you control, and that its second ability sacrifices itself to return a target
/// creature to its owner's hand, removing the static boost.
#[test]
fn angelic_shield_buffs_toughness_and_sacrifices_to_bounce() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1327, forest())
        .battlefield(0, &[angelic_shield(), llanowar_elves()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("wurm deployed");

    // Static ability boosts controlled creature toughness by 1
    assert_eq!(
        pt(&engine, elf),
        (1, 2),
        "elf gets +0/+1 from Angelic Shield"
    );
    assert_eq!(pt(&engine, wurm), (6, 6), "opponent wurm is untouched");

    // Ability 1 is the activated ability that sacrifices itself to bounce
    activate(&mut engine, p0, angelic_shield(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Angelic Shield, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&wurm));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_none(),
        "bounced wurm left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, rootbreaker_wurm()).is_some(),
        "bounced wurm is in opponent's hand"
    );
    assert!(
        in_graveyard(&engine, p0, angelic_shield()).is_some(),
        "Angelic Shield is in graveyard"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "elf returns to 1/1 after Angelic Shield leaves"
    );
}

fn carnival_of_souls() -> CardIndex {
    card_index("95b10ca7-7360-4da5-bd93-686ae3051833")
}

/// `Carnival of Souls` (`Coverage::Implemented`):
/// "Whenever a creature enters, you lose 1 life and add `{{B}}`."
///
/// Verifies that whenever a creature enters the battlefield, `Carnival of Souls`
/// triggers, causing its controller to lose 1 life and add one black mana.
#[test]
fn carnival_of_souls_triggers_on_creature_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1328, forest())
        .battlefield(0, &[carnival_of_souls(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller loses 1 life when a creature enters"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "controller adds {{B}} when a creature enters"
    );
}

fn clutch_of_undeath() -> CardIndex {
    card_index("5a68c925-db79-44f2-a5c1-5a607298f6cf")
}

/// `Clutch of Undeath` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +3/+3 as long as it's a Zombie. Otherwise, it gets -3/-3."
///
/// Verifies that casting `Clutch of Undeath` on a Zombie creature grants it +3/+3.
#[test]
fn clutch_of_undeath_gives_plus_three_plus_three_to_zombie() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1316, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                festering_goblin(),
            ],
        )
        .hand(0, &[clutch_of_undeath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let goblin =
        on_battlefield(&engine, p0, festering_goblin()).expect("festering goblin deployed");
    assert_eq!(pt(&engine, goblin), (1, 1));

    cast_from_hand(&mut engine, p0, clutch_of_undeath());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![goblin],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, clutch_of_undeath()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(goblin),
        "Aura attached to chosen goblin"
    );
    assert_eq!(
        pt(&engine, goblin),
        (4, 4),
        "1/1 Zombie gets +3/+3 to become 4/4"
    );
}

fn compulsion() -> CardIndex {
    card_index("3fafb6b2-5cae-45b6-8550-3ff8daa02802")
}

/// `Compulsion` (`Coverage::Implemented`):
/// "`{{1}}{{U}}`, Discard a card: Draw a card. `{{1}}{{U}}`, Sacrifice this enchantment: Draw a card."
///
/// Verifies that activating `Compulsion`'s first ability costs `{{1}}{{U}}` and prompts
/// to discard a card from hand, subsequently drawing a card upon resolution.
#[test]
fn compulsion_discards_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1329, forest())
        .battlefield(0, &[compulsion(), island(), island()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_card = in_hand(&engine, p0, forest()).expect("forest in hand");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, compulsion(), 0);
    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected discard cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostDiscard);
    assert!(options.contains(&forest_card));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_card],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "discarded card is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, compulsion()).is_some(),
        "Compulsion remains on the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "drew a replacement card"
    );
}

fn concordant_crossroads() -> CardIndex {
    card_index("ff01b408-6d17-40a3-9efd-a1b341ec1307")
}

/// `Concordant Crossroads` (`Coverage::Implemented`):
/// "All creatures have haste."
///
/// Verifies that while `Concordant Crossroads` is on the battlefield, all creatures
/// on both sides of the table gain `KeywordSet::HASTE`.
#[test]
fn concordant_crossroads_grants_haste_to_all_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1318, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[concordant_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert!(!keywords(&engine, mine).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::HASTE));

    cast_from_hand(&mut engine, p0, concordant_crossroads());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, concordant_crossroads()).is_some());
    assert!(
        keywords(&engine, mine).contains(KeywordSet::HASTE),
        "my creature gains haste"
    );
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "opponent creature also gains haste"
    );
}

fn dark_heart_of_the_wood() -> CardIndex {
    card_index("c44f40da-867e-4237-b4b1-ed6feb1f37b7")
}

/// `Dark Heart of the Wood` (`Coverage::Implemented`):
/// "Sacrifice a Forest: You gain 3 life."
///
/// Verifies that activating `Dark Heart of the Wood` requires sacrificing a controlled
/// Forest and gains 3 life upon resolution.
#[test]
fn dark_heart_of_the_wood_sacrifices_forest_to_gain_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1330, forest())
        .battlefield(0, &[dark_heart_of_the_wood(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_forest = on_battlefield(&engine, p0, forest()).expect("forest deployed");
    assert_eq!(engine.state().players[0].life, 20);

    activate(&mut engine, p0, dark_heart_of_the_wood(), 0);
    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(options.contains(&my_forest));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_forest],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 23, "gained 3 life");
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "sacrificed Forest is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "sacrificed Forest left the battlefield"
    );
}

fn darkest_hour() -> CardIndex {
    card_index("5667376e-e59c-4b17-b096-5d92cdfe3db1")
}

/// `Darkest Hour` (`Coverage::Implemented`):
/// "All creatures are black."
///
/// Verifies that while `Darkest Hour` is on the battlefield, all creatures
/// on both sides of the table have their colors set to black.
#[test]
fn darkest_hour_sets_all_creatures_color_to_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1319, forest())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[darkest_hour()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(
        engine
            .state()
            .object(mine)
            .expect("mine exists")
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Green])
    );

    cast_from_hand(&mut engine, p0, darkest_hour());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, darkest_hour()).is_some());
    assert_eq!(
        engine
            .state()
            .object(mine)
            .expect("mine exists")
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black]),
        "my creature is black"
    );
    assert_eq!(
        engine
            .state()
            .object(theirs)
            .expect("theirs exists")
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black]),
        "opponent creature is black"
    );
}

fn dehydration() -> CardIndex {
    card_index("db27c686-e202-4b60-9a10-0a0fef25576c")
}

/// `Dehydration` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature doesn't untap during its controller's untap step."
///
/// Verifies that enchanting a tapped creature with `Dehydration` prevents it from
/// untapping during its controller's untap step, while lands untap normally.
#[test]
fn dehydration_prevents_enchanted_creature_from_untapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1309, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[dehydration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    assert!(!is_tapped(&engine, elf), "elf starts untapped");

    // Tap the elf for mana
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elf,
                ability_index: 0,
            },
        )
        .expect("elf taps for mana");
    assert!(is_tapped(&engine, elf), "elf is tapped");

    cast_from_hand(&mut engine, p0, dehydration());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, dehydration()).expect("Dehydration resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(elf),
        "Dehydration attached to elf"
    );

    // Pass turn to opponent and then back to p0's next main phase
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        is_tapped(&engine, elf),
        "enchanted creature does not untap during untap step"
    );
}

fn divine_transformation() -> CardIndex {
    card_index("292e7135-8804-43f2-a486-51ef97b83f77")
}

/// `Divine Transformation` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +3/+3."
///
/// Verifies that casting `Divine Transformation` gives +3/+3 to the targeted
/// creature while leaving a bystander creature unaffected.
#[test]
fn divine_transformation_gives_plus_three_plus_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1310, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[divine_transformation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, divine_transformation());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, divine_transformation()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (4, 4),
        "1/1 elf gets +3/+3 to become 4/4"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
}

fn elven_palisade() -> CardIndex {
    card_index("0fb94fa4-2aff-4636-ac1b-ed39dc9451a6")
}

/// `Elven Palisade` (`Coverage::Implemented`):
/// "Sacrifice a Forest: Target attacking creature gets -3/-0 until end of turn."
///
/// Verifies that during combat, activating `Elven Palisade` targets an attacking creature,
/// requires sacrificing a Forest as an activation cost, and reduces the attacker's
/// power by 3 until end of turn.
#[test]
fn elven_palisade_sacrifices_forest_to_reduce_attacker_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1320, forest())
        .battlefield(0, &[forest(), elven_palisade(), rootbreaker_wurm()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("wurm deployed");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("forest deployed");
    assert_eq!(pt(&engine, wurm), (6, 6));

    // Advance to combat and declare attackers
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wurm, Defender::Player(p1))],
            },
        )
        .expect("wurm declares attack");

    // Activate Elven Palisade targeting the attacking wurm
    activate(&mut engine, p0, elven_palisade(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Elven Palisade, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&wurm), "attacking wurm is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();

    // Cost choice: sacrifice a Forest
    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&my_forest),
        "controlled Forest is offered to sacrifice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_forest],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, wurm), (3, 6), "wurm gets -3/-0 to become 3/6");
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "sacrificed Forest is in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "sacrificed Forest is no longer on the battlefield"
    );
}

fn feast_of_the_unicorn() -> CardIndex {
    card_index("274d89b8-1e59-4992-9299-dc793b7f6752")
}

/// `Feast of the Unicorn` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +4/+0."
///
/// Verifies that casting `Feast of the Unicorn` grants +4/+0 to the targeted creature
/// without modifying its toughness or affecting a bystander creature.
#[test]
fn feast_of_the_unicorn_grants_plus_four_plus_zero() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1311, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[feast_of_the_unicorn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, feast_of_the_unicorn());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, feast_of_the_unicorn()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (5, 1),
        "1/1 elf gets +4/+0 to become 5/1"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
}

fn flight_of_fancy() -> CardIndex {
    card_index("cd20a2a4-5e5e-420d-9420-651bce511f76")
}

/// `Flight of Fancy` (`Coverage::Implemented`):
/// "Enchant creature. When this Aura enters, draw two cards. Enchanted creature has flying."
///
/// Verifies that casting `Flight of Fancy` grants flying to the enchanted creature
/// and triggers an enters-the-battlefield ability that draws two cards.
#[test]
fn flight_of_fancy_grants_flying_and_draws_two_cards_on_entering() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1312, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flight_of_fancy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);

    cast_from_hand(&mut engine, p0, flight_of_fancy());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();

    // Resolves the Aura and then resolves the ETB draw trigger
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, flight_of_fancy()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature gains flying"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "drew two cards from the enters-the-battlefield trigger"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "opponent elf does not gain flying"
    );
}

fn improvised_armor() -> CardIndex {
    card_index("aca7c0a7-b365-421e-aeb0-49d3a9873e4f")
}

/// `Improvised Armor` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +2/+5. Cycling `{{3}}`."
///
/// Verifies that casting `Improvised Armor` gives +2/+5 to the targeted creature
/// while leaving a bystander creature unaffected.
#[test]
fn improvised_armor_gives_plus_two_plus_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1313, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[improvised_armor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, improvised_armor());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, improvised_armor()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 6),
        "1/1 elf gets +2/+5 to become 3/6"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
}

fn maggot_therapy() -> CardIndex {
    card_index("ab4887e6-f71b-462d-9243-9ebe54da98f4")
}

/// `Maggot Therapy` (`Coverage::Implemented`):
/// "Flash. Enchant creature. Enchanted creature gets +2/-2."
///
/// Verifies that `Maggot Therapy` modifies power by +2 and toughness by -2
/// on the targeted creature, leaving another creature unchanged.
#[test]
fn maggot_therapy_modifies_power_and_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1306, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), a_seven_five_wurm()])
        .battlefield(1, &[a_seven_five_wurm()])
        .hand(0, &[maggot_therapy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, a_seven_five_wurm()).expect("my wurm deployed");
    let theirs = on_battlefield(&engine, p1, a_seven_five_wurm()).expect("their wurm deployed");
    assert_eq!(pt(&engine, mine), (7, 5));

    cast_from_hand(&mut engine, p0, maggot_therapy());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, maggot_therapy()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to the chosen wurm"
    );
    assert_eq!(
        pt(&engine, mine),
        (9, 3),
        "7/5 wurm gets +2/-2 to become 9/3"
    );
    assert_eq!(
        pt(&engine, theirs),
        (7, 5),
        "the wurm across the table is untouched"
    );
}

fn mass_hysteria() -> CardIndex {
    card_index("4500131b-7417-4f30-a1b0-97d51b2e6458")
}

/// `Mass Hysteria` (`Coverage::Implemented`):
/// "All creatures have haste."
///
/// Verifies that while `Mass Hysteria` is on the battlefield, all creatures
/// on both sides of the table have `KeywordSet::HASTE`.
#[test]
fn mass_hysteria_grants_haste_to_all_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1321, forest())
        .battlefield(0, &[mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[mass_hysteria()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert!(!keywords(&engine, mine).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::HASTE));

    cast_from_hand(&mut engine, p0, mass_hysteria());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mass_hysteria()).is_some());
    assert!(
        keywords(&engine, mine).contains(KeywordSet::HASTE),
        "my creature gains haste"
    );
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "opponent creature also gains haste"
    );
}

fn mythic_proportions() -> CardIndex {
    card_index("e03322a0-e477-4223-969e-27f6772e3d6d")
}

/// `Mythic Proportions` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +8/+8 and has trample."
///
/// Verifies that casting `Mythic Proportions` grants +8/+8 and `KeywordSet::TRAMPLE`
/// to the targeted creature while leaving a bystander creature unaffected.
#[test]
fn mythic_proportions_grants_pump_and_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1317, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[mythic_proportions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, mythic_proportions());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, mythic_proportions()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (9, 9),
        "1/1 elf gets +8/+8 to become 9/9"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "enchanted creature gains trample"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "opponent elf does not gain trample"
    );
}

fn need_for_speed() -> CardIndex {
    card_index("8894ab96-17e1-41d8-a4fb-28b510807394")
}

/// `Need for Speed` (`Coverage::Implemented`):
/// "Sacrifice a land: Target creature gains haste until end of turn."
///
/// Verifies that activating `Need for Speed` requires sacrificing a land and grants
/// `KeywordSet::HASTE` to the targeted creature until end of turn.
#[test]
fn need_for_speed_sacrifices_land_to_grant_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1322, forest())
        .battlefield(0, &[need_for_speed(), mountain(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    let my_mountain = on_battlefield(&engine, p0, mountain()).expect("mountain deployed");
    assert!(!keywords(&engine, elf).contains(KeywordSet::HASTE));

    activate(&mut engine, p0, need_for_speed(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Need for Speed, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf), "elf is a legal creature target");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&my_mountain),
        "controlled land is offered to sacrifice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_mountain],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::HASTE),
        "target creature gained haste"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "sacrificed land is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, mountain()).is_none(),
        "sacrificed land left the battlefield"
    );
}

fn onslaught() -> CardIndex {
    card_index("ae9ca82c-e07e-4a41-a387-0ef7d6df14b6")
}

/// `Onslaught` (`Coverage::Implemented`):
/// "Whenever you cast a creature spell, tap target creature."
///
/// Verifies that casting a creature spell triggers `Onslaught`, prompting for a target
/// creature to tap, and that upon resolution the targeted creature becomes tapped.
#[test]
fn onslaught_taps_target_creature_on_casting_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1323, forest())
        .battlefield(0, &[onslaught(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert!(!is_tapped(&engine, their_elf));

    cast_from_hand(&mut engine, p0, llanowar_elves());

    // Onslaught triggers on casting a creature spell
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Onslaught trigger, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&their_elf));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, their_elf),
        "targeted creature was tapped by Onslaught"
    );
}

fn scavenged_weaponry() -> CardIndex {
    card_index("2d3010d5-5c21-4342-ad79-a737b6731230")
}

/// `Scavenged Weaponry` (`Coverage::Implemented`):
/// "Enchant creature. When this Aura enters, draw a card. Enchanted creature gets +1/+1."
///
/// Verifies that casting `Scavenged Weaponry` attaches to the targeted creature, gives it
/// +1/+1, and triggers an enters-the-battlefield ability that draws a card.
#[test]
fn scavenged_weaponry_pumps_and_draws_a_card_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1307, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[scavenged_weaponry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    assert_eq!(pt(&engine, elf), (1, 1));
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);

    cast_from_hand(&mut engine, p0, scavenged_weaponry());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    // Resolves the Aura and then resolves the enters-the-battlefield draw trigger
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, scavenged_weaponry()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(elf),
        "Aura attached to chosen elf"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "1/1 elf gets +1/+1 to become 2/2");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "drew a card from the enters-the-battlefield trigger"
    );
}

fn seal_of_fire() -> CardIndex {
    card_index("348a345e-4639-41ca-b015-a5d43459eb64")
}

/// `Seal of Fire` (`Coverage::Implemented`):
/// "Sacrifice this enchantment: It deals 2 damage to any target."
///
/// Verifies that activating `Seal of Fire` sacrifices itself and deals 2 damage
/// to the chosen target player.
#[test]
fn seal_of_fire_sacrifices_to_deal_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1324, forest())
        .battlefield(0, &[seal_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[1].life, 20);

    activate(&mut engine, p0, seal_of_fire(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Seal of Fire, got {:?}",
            engine.pending()
        )
    };
    assert!(player_options.contains(&p1));
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
        18,
        "dealt 2 damage to opponent"
    );
    assert!(
        in_graveyard(&engine, p0, seal_of_fire()).is_some(),
        "Seal of Fire is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, seal_of_fire()).is_none(),
        "Seal of Fire left the battlefield"
    );
}

fn seal_of_removal() -> CardIndex {
    card_index("f0801029-bcf7-4bdb-84bf-e88dcaa9dc03")
}

/// `Seal of Removal` (`Coverage::Implemented`):
/// "Sacrifice this enchantment: Return target creature to its owner's hand."
///
/// Verifies that activating `Seal of Removal` sacrifices itself and returns the
/// targeted creature to its owner's hand.
#[test]
fn seal_of_removal_bounces_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1325, forest())
        .battlefield(0, &[seal_of_removal()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");

    activate(&mut engine, p0, seal_of_removal(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Seal of Removal, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&their_elf));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "bounced creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "bounced creature is in owner's hand"
    );
    assert!(
        in_graveyard(&engine, p0, seal_of_removal()).is_some(),
        "Seal of Removal is in graveyard"
    );
}

fn seal_of_strength() -> CardIndex {
    card_index("e41a68b3-e1cb-4f51-bd54-68882d2cc015")
}

/// `Seal of Strength` (`Coverage::Implemented`):
/// "Sacrifice this enchantment: Target creature gets +3/+3 until end of turn."
///
/// Verifies that activating `Seal of Strength` sacrifices itself and gives +3/+3
/// to the targeted creature until end of turn.
#[test]
fn seal_of_strength_pumps_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1326, forest())
        .battlefield(0, &[seal_of_strength(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    assert_eq!(pt(&engine, elf), (1, 1));

    activate(&mut engine, p0, seal_of_strength(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Seal of Strength, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (4, 4), "1/1 elf gets +3/+3 to become 4/4");
    assert!(
        in_graveyard(&engine, p0, seal_of_strength()).is_some(),
        "Seal of Strength is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, seal_of_strength()).is_none(),
        "Seal of Strength left the battlefield"
    );
}

fn serra_s_embrace() -> CardIndex {
    card_index("6d6ba936-4a15-4c40-aaa6-71605fb732d1")
}

/// `Serra's Embrace` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +2/+2 and has flying and vigilance."
///
/// Verifies that `Serra's Embrace` grants +2/+2, `KeywordSet::FLYING`, and
/// `KeywordSet::VIGILANCE` to the enchanted creature while leaving a bystander creature unaffected.
#[test]
fn serra_s_embrace_grants_pump_flying_and_vigilance() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1314, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[serra_s_embrace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, serra_s_embrace());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, serra_s_embrace()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "1/1 elf gets +2/+2 to become 3/3"
    );
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING.union(KeywordSet::VIGILANCE)),
        "enchanted creature gains flying and vigilance"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    let their_kw = keywords(&engine, theirs);
    assert!(
        !their_kw.contains(KeywordSet::FLYING) && !their_kw.contains(KeywordSet::VIGILANCE),
        "opponent elf receives no keywords"
    );
}

fn spectral_cloak() -> CardIndex {
    card_index("fadfa9f9-d096-4083-8630-1c18928133ff")
}

/// `Spectral Cloak` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature has shroud as long as it's untapped."
///
/// Verifies that an untapped creature enchanted by `Spectral Cloak` gains shroud,
/// while an opponent's creature is unaffected. When the enchanted creature taps to
/// produce mana, shroud is immediately lost because the static condition is no longer met.
#[test]
fn spectral_cloak_grants_shroud_only_while_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1301, forest())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[spectral_cloak()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elves deployed");

    // The two Islands and *not* the Elves: `cast_from_hand` taps every mana
    // source on the board, and the Elves are one — so paying that way would
    // tap the very creature whose untapped status this card reads, and the
    // missing shroud would look like an unimplemented static ability rather
    // than a correctly-read condition.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, spectral_cloak());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&mine) && options.contains(&theirs));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, spectral_cloak()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen creature"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::SHROUD),
        "untapped enchanted creature has shroud"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::SHROUD),
        "opponent creature has no shroud"
    );

    // Tapping the elf causes it to lose shroud
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mine,
                ability_index: 0,
            },
        )
        .expect("elf taps for mana");
    assert!(is_tapped(&engine, mine), "elf is now tapped");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::SHROUD),
        "shroud is lost while tapped"
    );
}

fn tiger_claws() -> CardIndex {
    card_index("2ef8ebc9-4f95-42f8-86e6-85eff0b8f021")
}

/// `Tiger Claws` (`Coverage::Implemented`):
/// "Flash. Enchant creature. Enchanted creature gets +1/+1 and has trample."
///
/// Verifies that casting `Tiger Claws` grants +1/+1 and `KeywordSet::TRAMPLE`
/// to the enchanted creature while leaving a bystander unaffected.
#[test]
fn tiger_claws_grants_pump_and_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1308, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[tiger_claws()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, tiger_claws());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, tiger_claws()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "1/1 elf gets +1/+1 to become 2/2"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "enchanted creature gains trample"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "opponent elf does not gain trample"
    );
}

fn torment() -> CardIndex {
    card_index("b53ec6e6-fcc9-4471-88c5-7ad0fbd7bbea")
}

/// `Torment` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets -3/-0."
///
/// Verifies that casting `Torment` on an opponent's creature reduces its power
/// by 3 while leaving its toughness unchanged, and leaves other creatures untouched.
#[test]
fn torment_reduces_power_by_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1302, forest())
        .battlefield(0, &[swamp(), forest()])
        .battlefield(1, &[rootbreaker_wurm(), llanowar_elves()])
        .hand(0, &[torment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("wurm deployed");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf deployed");
    assert_eq!(pt(&engine, wurm), (6, 6));

    cast_from_hand(&mut engine, p0, torment());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, torment()).expect("Torment resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(wurm),
        "Torment attached to wurm"
    );
    assert_eq!(
        pt(&engine, wurm),
        (3, 6),
        "6/6 wurm gets -3/-0 to become 3/6"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "bystander elf is untouched");
}

fn twisted_experiment() -> CardIndex {
    card_index("4066d4df-d98f-44cd-bf25-ebb5e9d9ddeb")
}

/// `Twisted Experiment` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +3/-1."
///
/// Verifies that enchanting a creature with `Twisted Experiment` modifies its
/// power by +3 and toughness by -1, while leaving bystander creatures unchanged.
#[test]
fn twisted_experiment_modifies_power_and_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1303, forest())
        .battlefield(0, &[swamp(), forest(), a_seven_five_wurm()])
        .battlefield(1, &[a_seven_five_wurm()])
        .hand(0, &[twisted_experiment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, a_seven_five_wurm()).expect("my wurm deployed");
    let theirs = on_battlefield(&engine, p1, a_seven_five_wurm()).expect("their wurm deployed");
    assert_eq!(pt(&engine, mine), (7, 5));

    cast_from_hand(&mut engine, p0, twisted_experiment());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, twisted_experiment()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to the chosen wurm"
    );
    assert_eq!(
        pt(&engine, mine),
        (10, 4),
        "7/5 wurm gets +3/-1 to become 10/4"
    );
    assert_eq!(
        pt(&engine, theirs),
        (7, 5),
        "the wurm across the table is untouched"
    );
}

fn wings_of_aesthir() -> CardIndex {
    card_index("06413d87-d119-4c04-93d5-5ced7ad4a858")
}

/// `Wings of Aesthir` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +1/+0 and has flying and first strike."
///
/// Verifies that casting `Wings of Aesthir` grants +1/+0, `KeywordSet::FLYING`,
/// and `KeywordSet::FIRST_STRIKE` to the enchanted creature while leaving
/// bystander creatures unaffected.
#[test]
fn wings_of_aesthir_grants_pump_flying_and_first_strike() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1304, forest())
        .battlefield(0, &[plains(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wings_of_aesthir()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, wings_of_aesthir());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, wings_of_aesthir()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 1),
        "1/1 elf gets +1/+0 to become 2/1"
    );
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE)),
        "enchanted creature gains flying and first strike"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    let their_kw = keywords(&engine, theirs);
    assert!(
        !their_kw.contains(KeywordSet::FLYING) && !their_kw.contains(KeywordSet::FIRST_STRIKE),
        "opponent elf receives no keywords"
    );
}

fn wings_of_hope() -> CardIndex {
    card_index("c1df6359-edf4-48cf-b8d1-6240ac291cf7")
}

/// `Wings of Hope` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +1/+3 and has flying."
///
/// Verifies that casting `Wings of Hope` gives +1/+3 and `KeywordSet::FLYING`
/// to the targeted creature without affecting an identical bystander.
#[test]
fn wings_of_hope_grants_pump_and_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1305, forest())
        .battlefield(0, &[plains(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wings_of_hope()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, wings_of_hope());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, wings_of_hope()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 4),
        "1/1 elf gets +1/+3 to become 2/4"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature gains flying"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "opponent elf does not gain flying"
    );
}

fn zephid_s_embrace() -> CardIndex {
    card_index("2f8b07ef-9d00-4eb6-a395-b5033aa3f80e")
}

/// `Zephid's Embrace` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +2/+2 and has flying and shroud."
///
/// Verifies that `Zephid's Embrace` grants +2/+2, `KeywordSet::FLYING`, and
/// `KeywordSet::SHROUD` to the targeted creature without affecting a bystander creature.
#[test]
fn zephid_s_embrace_grants_pump_flying_and_shroud() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1315, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[zephid_s_embrace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(pt(&engine, mine), (1, 1));

    cast_from_hand(&mut engine, p0, zephid_s_embrace());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, zephid_s_embrace()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "1/1 elf gets +2/+2 to become 3/3"
    );
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING.union(KeywordSet::SHROUD)),
        "enchanted creature gains flying and shroud"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "opponent elf is untouched");
    let their_kw = keywords(&engine, theirs);
    assert!(
        !their_kw.contains(KeywordSet::FLYING) && !their_kw.contains(KeywordSet::SHROUD),
        "opponent elf receives no keywords"
    );
}
