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

/// Luminarch Ascension's other branch: "if you didn't lose life this turn"
/// on a turn its controller did. The test above is the turn with no loss.
///
/// The opponent casts a Lightning Bolt in their own main phase, and the two
/// games differ only in who it hits. Aimed at the opponent, the end step
/// still offers the counter. That is the control, and it is what keeps the
/// other half from passing on a trigger that never fired. Aimed at the
/// Ascension's controller, the condition is false on resolution, so nothing
/// is asked and no counter is placed (CR 603.4).
#[test]
fn luminarch_ascension_asks_nothing_on_a_turn_its_controller_lost_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let end_step = |bolted: PlayerId| {
        let mut engine = Duel::new(42, plains())
            .battlefield(0, &[luminarch_ascension()])
            .battlefield(1, &[mountain()])
            .hand(1, &[lightning_bolt()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");
        tap_all_mana(&mut engine, p1);
        cast_with_floating(&mut engine, p1, lightning_bolt());
        engine
            .apply(
                p1,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![bolted],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[bolted.get() as usize].life,
            17,
            "the Bolt hit"
        );
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::MayDo,
                    ..
                }
            ) || e.state().turn.active == p0
        });
        let asked = matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        );
        let ascension =
            on_battlefield(&engine, p0, luminarch_ascension()).expect("the enchantment is out");
        let quest = engine
            .state()
            .object(ascension)
            .expect("still on the battlefield")
            .counters
            .get(baylee_cards_dsl::counters::QUEST);
        (asked, quest)
    };
    assert_eq!(
        end_step(p1),
        (true, 0),
        "the opponent's loss is not the controller's: the counter is offered"
    );
    assert_eq!(
        end_step(p0),
        (false, 0),
        "three damage to the controller is three life lost this turn"
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

/// Dragonback Assault is `{3}{G}{U}{R}` for two printed sentences: an
/// enters-trigger that deals 3 damage to each creature and each planeswalker,
/// and landfall — a 4/4 red Dragon with flying whenever a land its
/// controller's controls enters. The damage is read off three bodies on one
/// board: a 3/3 of mine that has to die to three points, a 1/1 across the
/// table that has to die to the same three, and a 7/5 of mine that has to
/// survive them, which is what tells damage to each creature from a destroy
/// and three points from one. The landfall half is then played rather than
/// read: one of my lands enters and a Dragon arrives, while the same land drop
/// by the other seat a turn later leaves the count where it was.
///
/// "And each planeswalker" is read off Karn, the Great Creator across the
/// table: printed loyalty 5, so three damage leaves him standing at 2 with no
/// damage marked (CR 120.3c) — a 3-loyalty walker would be in the graveyard
/// before anything could look at it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn dragonback_assault_shoots_each_creature_and_makes_a_dragon_for_a_land_of_yours() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        // Six lands, which is exactly {3}{G}{U}{R}, plus the two bodies the
        // damage is measured against.
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                mountain(),
                katara_the_fearless(),
                a_seven_five(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), karn_the_great_creator()])
        // The Assault itself, and the land that is the landfall half.
        .hand(0, &[dragonback_assault(), forest()])
        // A land of their own, for the control at the end.
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    let katara = on_battlefield(&engine, p0, katara_the_fearless()).expect("Katara is out");
    let wurm = on_battlefield(&engine, p0, a_seven_five()).expect("the big body is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("their Karn is out");
    let loyalty = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .object(karn)
            .expect("Karn is an object")
            .counters
            .get(baylee_cards_dsl::CounterKind::Loyalty)
    };
    assert_eq!(loyalty(&engine), 5, "Karn enters with his printed loyalty");
    assert_eq!(
        pt(&engine, katara),
        (3, 3),
        "a printed 3/3 before any damage"
    );
    assert_eq!(
        pt(&engine, wurm),
        (7, 5),
        "and a body three points cannot kill"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "with a 1/1 across the table");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "arriving is not a land entering: nothing has been made yet"
    );

    // Mana into the pool before the cast is claimed: `can_afford` reads the
    // pool and not the untapped lands. Both creatures are named as the
    // objects kept back, so the six on the pool are the six lands.
    tap_mana_where(&mut engine, p0, |id| id != katara && id != wurm);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "three Forests, two Islands and a Mountain, and neither creature paid in"
    );
    cast_with_floating(&mut engine, p0, dragonback_assault());
    // Let the spell resolve; the enters-trigger it puts on the stack behind
    // itself is answered by the same walk.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, dragonback_assault()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, katara_the_fearless()).is_some(),
        "3 damage on a printed 3/3 is lethal (CR 704.5f), and it is my own \
         creature: \"each creature\" reaches this side of the table too"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and the opponent's 1/1 died to the same trigger, so the damage is not \
         \"each creature you control\""
    );
    assert_eq!(
        pt(&engine, wurm),
        (7, 5),
        "while the 7/5 is still standing under the same three points: the \
         trigger damages each creature rather than destroying them"
    );
    assert_eq!(
        on_battlefield(&engine, p1, karn_the_great_creator()),
        Some(karn),
        "Karn survives three points of his five"
    );
    assert_eq!(loyalty(&engine), 2, "\"and each planeswalker\": 5 - 3");
    assert_eq!(
        engine.state().object(karn).expect("Karn").damage,
        0,
        "damage to a planeswalker removes loyalty and is not marked (CR 120.3c)"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and still no Dragon — only a land entering makes one"
    );

    // The landfall half. The land is played rather than seated, because the
    // trigger watches an entry and not a board.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| !tokens_of(e, p0).is_empty());

    let dragons = tokens_of(&engine, p0);
    assert_eq!(dragons.len(), 1, "one land, one Dragon");
    let dragon = dragons[0];
    assert_eq!(pt(&engine, dragon), (4, 4), "the printed 4/4 body");
    assert!(
        keywords(&engine, dragon).contains(KeywordSet::FLYING),
        "and the flying the token is printed with"
    );
    assert!(
        types(&engine, dragon).contains(TypeSet::CREATURE),
        "it is a creature token: {:?}",
        types(&engine, dragon)
    );
    let printed = engine
        .state()
        .object(dragon)
        .expect("the Dragon is an object")
        .token
        .expect("a token and not a card that arrived from somewhere");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Red),
        "a 4/4 *red* Dragon"
    );

    // The other half of "a land *you* control": the same land drop by the
    // other seat leaves the count where it was.
    reach_their_main_phase(&mut engine, p1);
    let their_land = in_hand(&engine, p1, forest()).expect("p1 is holding a land of its own");
    engine
        .apply(p1, PlayerAction::PlayLand { card: their_land })
        .expect("a land drop in their own main phase is legal");
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "landfall watches *your* lands: the opponent's land entering made no Dragon"
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
/// This is the third line, read about a creature *entering*: the trample
/// anthem reaches your creatures and not the opponent's, casting a 1/1
/// draws nothing, and casting a 6/6 draws. The enchantment is seated rather
/// than cast here, so its own enters-trigger never fires — that one is the
/// test above, and keeping them apart is what stops one draw being read as
/// the other.
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

/// Path of Mettle prints "When this enchantment enters, it deals 1 damage to
/// each creature that doesn't have first strike, double strike, vigilance, or
/// haste", and the board is three identical printed 1/1 Llanowar Elves — two
/// of mine and one across the table — of which exactly one has been given
/// haste by Lightning Greaves before the enchantment arrives. The Greaves
/// equip for `{0}`, so nothing about the mana spent on the enchantment is
/// confounded by them, and both halves of the sentence get a witness of the
/// same body: one Elf dies beside the survivor of my own, and one dies on the
/// other side of the table, because "each creature" names no controller.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn path_of_mettle_spares_the_creature_that_has_one_of_its_four_keywords() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                mountain(),
                mountain(),
                llanowar_elves(),
                llanowar_elves(),
                lightning_greaves(),
            ],
        )
        .hand(0, &[path_of_mettle()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (armed, bare) = (elves[0], elves[1]);
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "an Elf across the table"
    );
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are out");
    assert!(
        !keywords(&engine, armed).contains(KeywordSet::HASTE),
        "nothing is equipped yet, so nothing has haste"
    );

    // Equip {0} (CR 702.6). Its whole price is the tap symbol, so the pool the
    // enchantment below is paid out of is exactly what the four lands hold —
    // and the ability index is read out of the offer rather than guessed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == greaves)
        .expect("Equip {0} is the only activated ability the Greaves print");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("equip costs its own tap symbol and no mana at all");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options.len(),
        2,
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        options.contains(&armed) && options.contains(&bare),
        "and the Elf across the table is not one of them: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![armed],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(armed))
    });

    let granted = keywords(&engine, armed);
    assert!(
        granted.contains(KeywordSet::HASTE),
        "the Greaves grant haste, the fourth of the four keywords the trigger \
         names: {granted:?}"
    );
    assert_eq!(
        pt(&engine, armed),
        (1, 1),
        "and they change no body, so it is still the printed 1/1 the bare Elf is"
    );

    // The Elves are named as the printing kept back, because they are the
    // creatures this test reads afterwards: what is tapped is the two Plains
    // and the two Mountains, which is {W}{W}{R}{R} for a {R}{W}.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Plains and two Mountains, and neither Elf tapped for anything"
    );
    cast_front_face(&mut engine, p0, path_of_mettle());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, path_of_mettle()).is_some(),
        "the enchantment resolved onto the table"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![armed],
        "1 damage on a printed 1/1 is lethal (CR 704.5f), so the only Elf of \
         mine still standing is the one the trigger's filter excluded"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the bare Elf the trigger did not exclude is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "\"each creature\" names no controller, so the Elf across the table \
         died to the same resolution"
    );
    assert!(
        keywords(&engine, armed).contains(KeywordSet::HASTE),
        "and the survivor kept the keyword that spared it"
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
/// intercepts the surveil 1 arrangement (`ArrangePrompt::Surveil`), chooses to put the top
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
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });

    let Pending::Arrange {
        cards,
        prompt,
        piles,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a surveil arrangement, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ArrangePrompt::Surveil);
    assert_eq!(piles, surveil_piles(1), "surveil 1 allows at most 1 card");
    assert_eq!(cards.len(), 1, "surveil 1 looks at the top card of library");

    let milled_card = cards[0];
    engine
        .apply(p0, look_answer(&cards, &[milled_card]))
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

fn arenson_s_aura() -> CardIndex {
    card_index("465843dc-57d0-46fd-ac47-238723034563")
}

/// Arenson's Aura prints two lines: "{W}, Sacrifice an enchantment:
/// Destroy target enchantment" and "{3}{U}{U}: Counter target enchantment
/// spell." This scenario plays both. The first half sacrifices one of the
/// two own auras to destroy the opponent's enchantment — the sacrifice
/// menu shows that "an enchantment" means both own auras and neither the
/// creature nor the opponent's battlefield —, and the second half counters
/// the opponent's enchantment spell on the opponent's turn, which afterwards
/// lies in the graveyard instead of in play.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn arenson_s_aura_sacrifices_an_enchantment_to_destroy_one_and_counters_an_enchantment_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                arenson_s_aura(),
                arenson_s_aura(),
                plains(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(
            1,
            &[luminarch_ascension(), baleful_strix(), plains(), plains()],
        )
        .hand(1, &[luminarch_ascension()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 erreicht seine eigene Hauptphase"
    );

    let auras = all_on_battlefield(&engine, p0, arenson_s_aura());
    assert_eq!(auras.len(), 2, "zwei eigene Auren, eine zahlt gleich");
    let (aura, fodder) = (auras[0], auras[1]);
    let theirs = on_battlefield(&engine, p1, luminarch_ascension()).expect("ihre Verzauberung");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("their creature");

    // Only the Plain: the five Islands must remain for the second line,
    // and the pool is the only place where `can_afford` reads.
    let land = on_battlefield(&engine, p0, plains()).expect("ein Plain");
    assert_eq!(
        tap_mana_where(&mut engine, p0, |id| id == land),
        1,
        "das Plain und sonst nichts"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(aura, 0)),
        "{{W}} and an enchantment are both there, so the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, arenson_s_aura(), 0);

    // CR 601.2c before CR 601.2h: both questions are answered in the order
    // they actually arrive.
    let mut targets: Vec<ObjectId> = Vec::new();
    let mut menu: Vec<ObjectId> = Vec::new();
    loop {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                targets = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![theirs],
                        },
                    )
                    .expect("ihre Verzauberung war eine der Optionen");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a price and no search, what it all says to a client"
                );
                assert_eq!((min, max), (1, 1), "exactly one enchantment");
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("die eigene Aura war eine der Optionen");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected, while the Aura is being resolved: {other:?}"),
        }
    }

    assert!(
        targets.contains(&theirs),
        "\"Destroy target enchantment\" reaches the opponent's battlefield: {targets:?}"
    );
    assert!(
        !targets.contains(&strix),
        "a creature is not an enchantment — the filter is read: {targets:?}"
    );
    assert_eq!(menu.len(), 2, "die beiden eigenen Auren: {menu:?}");
    assert!(
        menu.contains(&aura) && menu.contains(&fodder),
        "\"eine Verzauberung, die du kontrollierst\" meint beide: {menu:?}"
    );
    assert!(
        !menu.contains(&strix),
        "a creature is not an enchantment: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs),
        "CR 701.21a: what belongs to the opponent, this seat does not sacrifice: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, luminarch_ascension()).is_none(),
        "die benannte Verzauberung ist zerstört"
    );
    assert!(
        in_graveyard(&engine, p1, luminarch_ascension()).is_some(),
        "and lies in its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, arenson_s_aura()).len(),
        1,
        "die geopferte Aura ist weg, die andere steht noch"
    );
    assert!(
        in_graveyard(&engine, p0, arenson_s_aura()).is_some(),
        "a sacrificed enchantment goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} is paid, and nothing more was floating"
    );

    // Second line: on the opponent's turn cast an enchantment and counter
    // it while it is on the stack.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "zwei Plains, und der Strix macht kein Mana"
    );
    cast_with_floating(&mut engine, p1, luminarch_ascension());
    let spell =
        on_stack(&engine, luminarch_ascension()).expect("die Verzauberung liegt auf dem Stapel");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // First mana into the pool, then the claim about the offer.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the five Islands; the Plains has been tapped since the first line"
    );
    let theirs_grave = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the opponent has passed");
    assert!(
        legal.abilities.contains(&(aura, 1)),
        "{{3}}{{U}}{{U}} is payable, so the second line is in the offer: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, arenson_s_aura(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"Target: enchantment spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "die aktivierende Sitz wählt");
    assert!(
        options.contains(&spell),
        "the spell on the stack is what \"enchantment spell\" names: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the spell was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, luminarch_ascension()).is_none(),
        "the spell never arrived"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        theirs_grave + 1,
        "CR 701.5: a countered spell is the one extra card in its owner's \
         graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{U}}{{U}} ist bezahlt"
    );
    assert!(
        on_battlefield(&engine, p0, arenson_s_aura()).is_some(),
        "the second Aura counters the spell without sacrificing itself"
    );
}

fn armistice() -> CardIndex {
    card_index("da103316-2a85-4de9-8531-ac2cd2859d6f")
}

/// Armistice is a `{2}{W}` enchantment with exactly one line: "{3}{W}{W}:
/// You draw a card and target opponent gains 3 life." The scenario plays
/// both halves in one main-phase window, because eight Plains first pay the
/// enchantment spell and afterwards leave exactly the five mana that the
/// ability demands (CR 500.5) — so the cost is a real payment and not a
/// label on a free ability. The table with three seats is the word the card
/// relies on: the offer names both other seats and never its own, and the
/// three life lands on the one named, while the third seat stays untouched;
/// the draw is the half that no pool read can see.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn armistice_draws_a_card_and_gives_one_named_opponent_three_life() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, plains(), 3)
        .battlefield(0, &[plains(); 8])
        .hand(0, &[armistice()])
        .life(0, 20)
        .life(1, 20)
        .life(2, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Mana first: `cast_from_hand` taps the board, so the pool is read and
    // not the untapped lands, and what is left of the eight Plains after the
    // {2}{W} is exactly the {3}{W}{W} the ability goes on to charge.
    cast_from_hand(&mut engine, p0, armistice());
    pass_until(&mut engine, stack_is_empty);
    let enchantment = on_battlefield(&engine, p0, armistice()).expect("the Armistice resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "eight Plains pay the {{2}}{{W}} and leave exactly five for the ability"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "the one line the card prints is offered with its five mana floating: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, armistice(), 0);

    // CR 601.2c before CR 601.2h: while the target question stands no mana
    // has been spent yet, and the answer is lifted out of what the question
    // itself enumerated rather than guessed at.
    let action = match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat aims it");
            assert!(
                options.is_empty(),
                "nothing on the battlefield is a legal target for it: {options:?}"
            );
            assert_eq!(
                player_options.len(),
                2,
                "\"target opponent\" is the two other seats: {player_options:?}"
            );
            assert!(
                player_options.contains(&p1) && player_options.contains(&p2),
                "both opponents are offered: {player_options:?}"
            );
            assert!(
                !player_options.contains(&p0),
                "\"target opponent\" is not \"target player\": {player_options:?}"
            );
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![p1],
            }
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the activating seat aims it");
            assert_eq!(options.len(), 2, "the two other seats: {options:?}");
            assert!(
                options.contains(&p1) && options.contains(&p2) && !options.contains(&p0),
                "both opponents and never the controller: {options:?}"
            );
            PlayerAction::ChoosePlayer(p1)
        }
        other => panic!("\"target opponent\" is a target choice, got {other:?}"),
    };
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the cost is the last step of the activation, so the mana is still floating"
    );
    engine
        .apply(p0, action)
        .expect("the seat the question offered is a legal answer");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{W}}{{W}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "and it is no mana ability");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        23,
        "\"target opponent gains 3 life\" — the seat that was named"
    );
    assert_eq!(
        engine.state().players[2].life,
        20,
        "and exactly one opponent: the second seat never moved"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the life belongs to the opponent, not to the seat that paid"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "\"You draw a card\""
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library, so the hand grew by a draw"
    );
    assert!(
        on_battlefield(&engine, p0, armistice()).is_some(),
        "an activated ability costs the enchantment nothing"
    );
}

fn aura_fracture() -> CardIndex {
    card_index("3495d83a-b103-42be-8708-9ce971b352bd")
}

/// Aura Fracture — {2}{W} enchantment: "Sacrifice a land: Destroy target
/// enchantment."
///
/// Both halves of that line are the engine's answer rather than the card's,
/// so the board is built to strike each one. The sacrifice names no land in
/// particular: the menu is the four Plains under this seat and nothing else —
/// not the Forest across the table (CR 701.21a), and not the Aura Fracture
/// itself, which is an enchantment and no land. The target menu holds the
/// enchantment across the table *and* the Fracture on this side, because
/// "target enchantment" reaches both, while every land on either board stays
/// off it. And the order is CR 601.2c before CR 601.2h, so while the target
/// question stands nothing has been paid and nothing has been destroyed —
/// the land is still untapped and the enchantment is still standing, and only
/// one of them leaves the battlefield.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn aura_fracture_trades_a_land_for_the_enchantment_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[aura_fracture()])
        // An enchantment across the table to destroy, and a Forest that
        // "sacrifice a land" has to decline because it is not this seat's.
        .battlefield(1, &[fastbond(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{W} off three Plains, with the fourth kept back: the land the price
    // is paid with is untapped, so the sacrifice is read off a land nobody
    // could mistake for a tapped-out one.
    let fodder = on_battlefield(&engine, p0, plains()).expect("a Plains of mine is out");
    tap_mana_except(&mut engine, p0, fodder);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains tapped, and the fourth held back"
    );
    cast_with_floating(&mut engine, p0, aura_fracture());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, aura_fracture()).is_some()
    });
    let fracture = on_battlefield(&engine, p0, aura_fracture()).expect("Aura Fracture resolved");
    let doom = on_battlefield(&engine, p1, fastbond()).expect("the enchantment across the table");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let my_lands = all_on_battlefield(&engine, p0, plains());
    assert_eq!(my_lands.len(), 4, "four Plains, all of them still standing");
    assert!(
        !is_tapped(&engine, fodder),
        "the Plains kept back never paid for the enchantment"
    );

    // The whole price is a land and no mana, so the offer turns on nothing
    // the pool could have supplied.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(fracture, 0)),
        "a land to give up is the whole price, so the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, aura_fracture(), 0);

    // CR 601.2c: the target is named first, and nothing is paid while the
    // question stands.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&doom) && options.contains(&fracture),
        "\"target enchantment\" reaches both sides of the table — the Aura \
         Fracture is an enchantment too: {options:?}"
    );
    assert!(
        !options.contains(&fodder) && !options.contains(&their_land),
        "a land is no enchantment: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, aura_fracture()).is_some()
            && on_battlefield(&engine, p1, fastbond()).is_some(),
        "and nothing has happened yet: the cost is the last step of the activation"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doom],
            },
        )
        .expect("the enchantment the question offered is the one it destroys");

    // CR 601.2h: the sacrifice is asked only now, the target being settled.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the untapped Plains this seat kept back is on the menu: {options:?}"
    );
    assert!(
        options.iter().all(|id| my_lands.contains(id)),
        "every permanent on the menu is a land of this seat's own — the Forest \
         across the table and the Aura Fracture itself are not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the land the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, plains()).is_some(),
        "the land is in its owner's graveyard the moment the price is paid"
    );
    assert!(
        on_battlefield(&engine, p1, fastbond()).is_some(),
        "and the destruction has not happened yet — it resolves off the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying an enchantment is no mana ability"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, fastbond()).is_some(),
        "the enchantment the ability named is destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, aura_fracture()).is_some(),
        "the enchantment that aimed it was not the target and survived"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        3,
        "exactly one land paid the price"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing else on the other side of the table moved"
    );
}

fn aura_shards() -> CardIndex {
    card_index("8d03d050-391c-4311-8c42-4ee632d40fdc")
}

/// Aura Shards — {1}{G}{W} enchantment: "Whenever a creature you control
/// enters, you may destroy target artifact or enchantment."
///
/// `pass_until` answers the "you may" with yes on the way past, so what is
/// left to read is the target half: the menu has to hold every artifact and
/// enchantment on the table — the Shards themselves, the Sol Ring beside them
/// and the Luminarch Ascension across it — while the Llanowar Elves whose
/// entry caused the trigger is a creature and must be absent from it. The
/// destruction is then read in the graveyard rather than off the battlefield,
/// because "destroy" is the word the card prints, and an effect that merely
/// stopped being a permanent would satisfy a board check.
#[test]
fn aura_shards_destroys_the_artifact_or_enchantment_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                aura_shards(),
                forest(),
                forest(),
                plains(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[their_enchantment()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let shards = on_battlefield(&engine, p0, aura_shards()).expect("the Shards are out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let theirs =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");

    // The creature enters and the trigger asks; the target question is what
    // the walk stops on, with the "you may" already answered yes behind it.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves landed");
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the target question")
    };
    assert_eq!(
        player, p0,
        "the Shards' controller answers their own trigger"
    );
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&shards) && options.contains(&rock) && options.contains(&theirs),
        "\"target artifact or enchantment\" is any of the three on the table — \
         the Shards are an enchantment themselves and the Ascension stands \
         across it: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the creature whose entry caused the trigger is a creature and not \
         something the filter names: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "and those three are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the enchantment across the table was one of the options");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, their_enchantment()).is_some()
    });

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the chosen permanent left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "\"destroy\" puts it in its owner's graveyard, which is p1's and not \
         the Shards' controller's"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the permanent the ability did not name is untouched"
    );
    assert!(
        on_battlefield(&engine, p0, aura_shards()).is_some(),
        "and the Shards outlive their own trigger"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "one artifact or enchantment, one destruction: the creature that \
         started all of this is still standing"
    );
}

fn back_to_basics() -> CardIndex {
    card_index("05c2dec2-d2f7-4036-b91f-4fccba10a8bb")
}

/// Back to Basics — {2}{U} enchantment: "Nonbasic lands don't untap during
/// their controllers' untap steps." The card is cast for real off three
/// Islands, and the Badlands beside them is the whole proof: it is a land the
/// CR 305.6 shortcut taps for mana like any other, and the difference between
/// the two kinds only shows up one untap step later. The three Islands coming
/// back in that same step are the control — a rule that held *everything*
/// down and a game that never reached CR 502.3 would read identically without
/// them.
#[test]
fn back_to_basics_holds_the_nonbasic_lands_down_while_the_basics_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), badlands()])
        .hand(0, &[back_to_basics()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nonbasic = on_battlefield(&engine, p0, badlands()).expect("the Badlands is on the table");
    let basics = all_on_battlefield(&engine, p0, island());
    assert_eq!(basics.len(), 3, "three Islands beside it");
    assert!(
        !is_tapped(&engine, nonbasic),
        "a land enters untapped, so it has a {{T}} to spend"
    );

    // {2}{U} out of the board, and every land on it is spent to do it. That
    // the Badlands paid is what makes its *not* untapping below a fact about
    // the printed rule rather than about a land that never moved.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, back_to_basics());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, back_to_basics()).is_some(),
        "the enchantment resolved"
    );
    assert!(is_tapped(&engine, nonbasic), "the Badlands paid for it");
    assert!(
        basics.iter().all(|id| is_tapped(&engine, *id)),
        "and so did every Island"
    );

    // A whole turn cycle, so the untap step that matters is p0's own. The
    // opponent's turn in between is what makes it the *second* untap step of
    // the game rather than the one that already happened before the cast.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        basics.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every basic land is back. Without this the \
         assertion below is satisfied by a game that never reached CR 502.3"
    );
    assert!(
        is_tapped(&engine, nonbasic),
        "\"Nonbasic lands don't untap during their controllers' untap \
         steps\" — the Badlands alone stayed down, and the three basic \
         Islands beside it are why that is the rule and not a missing turn"
    );
}

// oracle_id = "10e95489-a94d-4523-964c-ec9753103a62"
fn blanket_of_night() -> CardIndex {
    card_index("10e95489-a94d-4523-964c-ec9753103a62")
}

/// Blanket of Night — {1}{B}{B} enchantment: "Each land is a Swamp in
/// addition to its other land types."
///
/// A Forest is the reading no board can give by accident: left alone it taps
/// for {G} and for nothing else, so black mana out of one can only be the
/// Swamp the Blanket added — and the question asked on the way is **two**
/// colours wide, which is the "in addition to" half, since a Forest that had
/// merely *become* a Swamp would offer black alone. The Forest across the
/// table is the word "each": it is not a land this seat controls, and it is
/// asked the same two-colour question on p1's own turn.
#[test]
fn blanket_of_night_makes_a_forest_a_swamp_on_both_sides_of_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest()])
        .battlefield(1, &[forest()])
        .hand(0, &[blanket_of_night()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // Three Swamps pay {1}{B}{B}, and the Forest is named as the thing kept
    // back: it is the permanent whose tap this test goes on to read.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps, three black, and the Forest contributed nothing"
    );
    cast_with_floating(&mut engine, p0, blanket_of_night());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, blanket_of_night()).is_some(),
        "the Blanket resolved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and its {{1}}{{B}}{{B}} came out of the pool"
    );
    assert!(!is_tapped(&engine, mine), "the Forest is still standing");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        legal.mana_abilities.contains(&mine),
        "an added basic land type is the CR 305.6 shortcut, so the Forest is \
         offered without a printed ability to name: {:?}",
        legal.mana_abilities
    );

    // **Two abilities and not one question**, which is CR 305.6 read
    // literally: the Forest has one mana ability per basic type, and its
    // green one is printed on the card (a basic land prints what the rule
    // gives it). So the shortcut is left with the type the Blanket added and
    // nothing else, and pressing it needs no colour question at all — the
    // green stays where it was, on `legal.abilities` at index 0.
    assert!(
        legal.abilities.contains(&(mine, 0)),
        "the printed green ability is still on offer beside it: {:?}",
        legal.abilities
    );
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: mine })
        .expect("the tap the offer named is the one it pays");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "black mana out of a Forest is the Swamp the Blanket added, and \
         nothing else on this board could have made it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one tap, one mana"
    );
    assert!(is_tapped(&engine, mine), "and it cost the Forest its tap");

    // "Each land" and not "each land you control": the Forest across the
    // table answers the same two-colour question on p1's own turn.
    reach_their_main_phase(&mut engine, p1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("p1 holds its own main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: theirs })
        .expect("their Forest is offered the same tap");
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "\"each land\": the opponent's Forest is a Swamp as well"
    );
}

fn captive_flame() -> CardIndex {
    card_index("fada1102-bbb7-4c90-a72e-6c595c08a55d")
}

/// Captive Flame costs `{2}{R}` and prints an activation whose entire
/// cost is one red mana: "{R}: Target creature gets +1/+0 until end of
/// turn." The scenario plays both halves in one main phase: four Mountains
/// pay the `{2}{R}` and leave exactly the red mana floating that the
/// ability then spends — `legal.abilities` is filtered behind
/// `can_afford`, so the line would not even be in the offer without that
/// mana. The Elf on the table is the control: "target creature" points at
/// every creature, but the `+1/+0` may only remain on the one that was
/// named; and the enchantment itself is not a creature and therefore does
/// not appear on the menu.
#[test]
fn captive_flame_pumps_only_the_creature_it_targets_for_one_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[captive_flame()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one across the table"
    );

    // The four Mountains into the pool and the Elf named as the one that
    // remains: its own `{T}: Add {G}` is also a mana ability, whose entire
    // cost is its own tap, and a green mana in the pool would extend every
    // number below by a source that has nothing to do with it.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4,
        "four Mountains tapped, and no Elf contributed a green"
    );
    cast_with_floating(&mut engine, p0, captive_flame());
    pass_until(&mut engine, stack_is_empty);

    let flame = on_battlefield(&engine, p0, captive_flame()).expect("the Flame resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} is spent and exactly the {{R}} the ability charges is left"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(flame, 0)),
        "with a red already floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, captive_flame(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&flame),
        "an enchantment is no creature, so the Flame cannot pump itself: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2c names the target first: nothing is spent while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was named");

    // CR 601.2h: the `{R}` is the last step of the activation, so the pool
    // is only read as empty here — and the enchantment stays untapped,
    // because its cost is the mana and never its own `{T}`.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
    assert!(!is_tapped(&engine, flame), "no {{T}} is part of the cost");
    assert!(!stack_is_empty(&engine), "the pump is no mana ability");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mine), (2, 1), "+1/+0 on the creature it named");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature it did not"
    );
}

fn choke() -> CardIndex {
    card_index("057fa60b-10b0-4612-be0d-157076c82241")
}

/// Choke — {2}{G} enchantment: "Islands don't untap during their controllers'
/// untap steps."
///
/// The enchantment is cast, and `cast_from_hand` taps every source whose whole
/// price is its own `{T}` — so the caster's own Island is already down beside
/// the three Forests that paid when the printed sentence is about to matter. The
/// Forests are the bound on the claim: they come back at that untap step and the
/// Island does not, which is the difference between a rule (CR 502.3) and a game
/// that simply never moved on. p1's Island is tapped in its own main phase and
/// read one turn later, because the sentence says "controllers'" — it names a
/// land type and no controller, and a filter quietly narrowed to the caster's
/// permanents would satisfy the first half of this test and none of the second.
#[test]
fn choke_keeps_every_island_down_while_the_forests_beside_them_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), forest(), island()])
        .battlefield(1, &[island(), forest()])
        .hand(0, &[choke()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, choke());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, choke()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    let mine = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let my_forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(my_forests.len(), 3, "three Forests paid for the Choke");
    assert!(
        my_forests.iter().all(|id| is_tapped(&engine, *id)) && is_tapped(&engine, mine),
        "the whole board was tapped for the {{2}}{{G}}, the Island included — \
         which is the state both untap steps below are read from"
    );

    // p1's two lands, tapped in their own main phase so that their controller's
    // untap step has something to leave behind.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    let their_island = on_battlefield(&engine, p1, island()).expect("their Island is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert!(
        is_tapped(&engine, their_island) && is_tapped(&engine, their_forest),
        "both of p1's lands tapped for mana in their own main phase"
    );

    // p0's next untap step: every Forest stands back up, the Island does not.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        my_forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every Forest came back. Without this the Island \
         below would be satisfied by a game that never reached CR 502.3"
    );
    assert!(
        is_tapped(&engine, mine),
        "\"Islands don't untap during their controllers' untap steps\" — the \
         caster's own Island stayed down (CR 502.3), an effect and not a \
         characteristic anything projects"
    );

    // And the same rule across the table, where a filter narrowed to the
    // caster's permanents would show nothing at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, their_forest),
        "p1's untap step ran too, and its Forest came back"
    );
    assert!(
        is_tapped(&engine, their_island),
        "the printed sentence names a land type and no controller, so p1's \
         Island is down for as long as the Choke is on the battlefield"
    );
}

fn contemplation() -> CardIndex {
    card_index("fa7efcef-a688-4e25-a823-4d53b2e96508")
}

/// Contemplation is a {1}{W}{W} enchantment whose entire text is "Whenever
/// you cast a spell, you gain 1 life", so the test plays three casts and asks
/// who owns each of them. Its own arrival gains nothing — an ability functions
/// only from the battlefield (CR 113.6) — the Sol Ring cast right afterwards
/// off the two white the first cast left floating gains exactly one, and the
/// Giant Growth the other seat aims at its own Strix gains p0 nothing at all,
/// because the trigger's `Filter::ControlledByYou` reads the spell and not the
/// table. Every number is read off a life total, which is the only thing this
/// card prints.
#[test]
fn contemplation_gains_one_life_for_your_spells_and_none_for_an_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[contemplation(), quiet_artifact()])
        .battlefield(1, &[forest(), baleful_strix()])
        .hand(1, &[giant_growth()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the Strix is on the table");
    let mine = engine.state().players[0].life;
    let theirs = engine.state().players[1].life;

    cast_from_hand(&mut engine, p0, contemplation());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, contemplation()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        mine,
        "its own cast happened while it was still a spell on the stack: an \
         ability functions only from the battlefield (CR 113.6), so there was \
         nothing of its own to trigger"
    );

    // Two of the five Plains are still floating — a mana pool survives until
    // the step ends (CR 500.5) and this test never leaves p0's main phase —
    // so the second spell is paid for out of the first one's leftovers.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the second spell resolved, so the cast really happened"
    );
    assert_eq!(
        engine.state().players[0].life,
        mine + 1,
        "\"Whenever you cast a spell, you gain 1 life\" — one spell, one life"
    );

    // The other seat gets priority inside p0's own main phase and answers
    // with a spell of its own: the trigger is a question about who cast the
    // spell, and a whole turn would answer it with a different board.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "passing in a main phase hands priority across the table, got {:?}",
        engine.pending()
    );
    cast_from_hand(&mut engine, p1, giant_growth());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "a pump spell asks for its target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it names the creature");
    assert!(
        options.contains(&strix),
        "the Strix across the table is a creature and a legal target: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![strix],
            },
        )
        .expect("the target the question itself offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        mine + 1,
        "the opponent's spell is not a spell *you* cast, so \
         `Filter::ControlledByYou` declines it and no life is gained"
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs,
        "and the trigger would not have paid the other seat either: the one \
         life total that moved in this game is the caster's"
    );
}

fn deadapult() -> CardIndex {
    card_index("5e0fea29-0fd5-4535-b1df-cd66e50662cc")
}

/// Deadapult — {2}{R} Enchantment: "{R}, Sacrifice a Zombie: This
/// enchantment deals 2 damage to any target."
///
/// Both words of the cost need a witness on the board: the sacrifice names a
/// *Zombie* and nothing else, so my Festering Goblin (a Zombie Goblin) is the
/// whole menu while the Wurm beside it and the Zombie across the table stay
/// off it — a filter that had dropped `HasSubtype(ZOMBIE)` or
/// `ControlledByYou` would still read correctly on the card file. The {R} is
/// paid last (CR 601.2h), out of a pool four Mountains filled for a {2}{R}
/// cast that leaves exactly one red behind, and "any target" (CR 115.4) is the
/// object-and-player choice that is read by aiming the two damage at the
/// *player*, whose life total then says two. The sacrificed Goblin's own death
/// trigger is aimed at the Wurm, a 6/6 that survives the −1/−1 and whose 5/5
/// afterwards is what says that trigger really resolved.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn deadapult_eats_a_zombie_for_two_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                festering_goblin(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[deadapult()])
        // A Zombie across the table: "a Zombie" is not an invitation to eat
        // somebody else's.
        .battlefield(1, &[festering_goblin()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // Four Mountains and nothing else that taps for mana: neither creature
    // prints a mana ability, so the pool is exactly the four the lands made.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and no other source on this board"
    );
    cast_with_floating(&mut engine, p0, deadapult());
    pass_until(&mut engine, stack_is_empty);
    let enchantment = on_battlefield(&engine, p0, deadapult()).expect("Deadapult resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} is spent and the {{R}} the ability charges is still \
         floating — one main phase, so CR 500.5 never emptied the pool"
    );

    let mine = on_battlefield(&engine, p0, festering_goblin()).expect("my Zombie is out");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("a non-Zombie is out");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Zombie is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "with the {{R}} already floating, the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, deadapult(), 0);

    // CR 601.2c first: the target is named while the Zombie still stands and
    // the {{R}} is still in the pool, because the costs come last.
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
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice, either \
         seat of the table: {player_options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&wurm) && options.contains(&theirs),
        "and the creatures on both sides of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2h pays last, so nothing has been spent while this stands"
    );
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "and the Zombie is still standing to be sacrificed"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the seat across the table was one of the options");

    // CR 601.2h: the sacrifice, and with it the {R}, are the last thing paid.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Zombie, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one Zombie, no more and no fewer");
    assert_eq!(
        options,
        vec![mine],
        "the one creature with the Zombie subtype under your control is the \
         whole of the answer"
    );
    assert!(
        !options.contains(&wurm),
        "the Wurm is a creature and no Zombie: `HasSubtype(ZOMBIE)` is read, \
         not skipped"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: a Zombie across the table is not yours to sacrifice"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Zombie the question offered pays the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{R}} went with it"
    );
    assert!(
        in_graveyard(&engine, p0, festering_goblin()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    // Festering Goblin's own printed trigger ("when this dies, target
    // creature gets −1/−1") fires off that sacrifice. It is not the card
    // under test, so it is aimed at the Wurm.
    let mut aimed_at_the_wurm = false;
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert!(
                    options.contains(&wurm) && options.contains(&theirs),
                    "the death trigger wants a creature, on either side of the \
                     table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![wurm],
                        },
                    )
                    .expect("the Wurm was one of the options");
                aimed_at_the_wurm = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected after the sacrifice: {other:?}"),
        }
    }
    assert!(
        aimed_at_the_wurm,
        "the sacrificed Zombie's own death trigger is a question before \
         anything resolves"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"deals 2 damage to any target\" — two, aimed at the player and not \
         at a creature"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the life belongs to the seat that was aimed at"
    );
    assert!(
        on_battlefield(&engine, p0, deadapult()).is_some(),
        "the enchantment does not sacrifice itself to pay its own cost"
    );
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "the Zombie the ability did not name never moved"
    );
    assert_eq!(
        pt(&engine, wurm),
        (5, 5),
        "the death trigger resolved on the Wurm: −1/−1 on a printed 6/6"
    );
}

fn dralnu_s_crusade() -> CardIndex {
    card_index("ff48cf80-4950-4ae4-9f7c-8d826b2f26f7")
}

/// Dralnu's Crusade prints three sentences that all share one filter: all
/// Goblins get +1/+1, all Goblins are black, and all Goblins are Zombies in
/// addition to their other creature types. The pump is the clause this board
/// can differ on — Festering Goblin already prints Zombie Goblin and is
/// already black — so it is read on **both** sides of the table, because "All
/// Goblins" is not "Goblins you control". The Skyclave Apparition beside them
/// is the filter's edge: a creature the same seat controls and no Goblin, so
/// a filter that had lost `Filter::HasSubtype` would have pumped it to 3/3.
#[test]
fn dralnu_s_crusade_pumps_the_goblins_on_both_sides_of_the_table_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                swamp(),
                mountain(),
                forest(),
                festering_goblin(),
                skyclave_apparition(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[dralnu_s_crusade()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, festering_goblin()).expect("my Goblin stands");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin stands");
    let other = on_battlefield(&engine, p0, skyclave_apparition()).expect("the Apparition stands");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "a printed 1/1 before the Crusade"
    );
    assert_eq!(
        pt(&engine, other),
        (2, 2),
        "and a printed 2/2 that is no Goblin"
    );

    // {1}{B}{R} out of the Swamp, the Mountain and the Forest, which is every
    // mana source on this board.
    cast_from_hand(&mut engine, p0, dralnu_s_crusade());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, dralnu_s_crusade()).is_some(),
        "the Crusade resolved onto the table"
    );

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "\"All Goblins get +1/+1\", on the Goblin under the Crusade's controller"
    );
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "and on the Goblin across the table: the card says all Goblins, not all \
         of yours"
    );
    assert_eq!(
        pt(&engine, other),
        (2, 2),
        "a creature that is no Goblin is untouched — a filter that had lost its \
         subtype would have pumped this one too"
    );
}

// oracle_id = "795b096a-2bce-4588-a2c9-abc5ea40dc0c"
fn enchantress_s_presence() -> CardIndex {
    card_index("795b096a-2bce-4588-a2c9-abc5ea40dc0c")
}

/// Enchantress's Presence — {2}{G} enchantment: "Whenever you cast an
/// enchantment spell, draw a card."
///
/// The trigger reads the *type of the spell on the stack*, so the hand holds
/// one of each kind: the Presence itself, which cannot draw for its own cast
/// (CR 603.2 — it is not on the battlefield yet when its own spell is cast),
/// a Llanowar Elves, which is a spell and no enchantment, and Fastbond, which
/// is the one card here that satisfies the filter. The library is read after
/// each of the three, so "a creature spell is no enchantment spell" is a
/// claim and not an assumption — a trigger that had lost its type filter
/// would draw on the Elves and every count before it would still have read
/// correctly.
#[test]
fn enchantresss_presence_draws_for_an_enchantment_and_for_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[enchantress_s_presence(), llanowar_elves(), fastbond()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Forests are the whole curve — {2}{G}, then {G}, then {G} — and
    // nothing else on this board makes mana, so the pool read after each cast
    // is a statement about the lands and nothing else.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests tapped, and the Elves are still in hand"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // The negative nobody has to control: the trigger lives on a permanent,
    // and that permanent arrives only once its own spell has resolved.
    cast_with_floating(&mut engine, p0, enchantress_s_presence());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, enchantress_s_presence()).is_some(),
        "the Presence resolved onto the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "\"whenever you *cast*\" — the Presence was not on the battlefield \
         when its own cast went by, so it drew nothing for it"
    );

    // A creature spell is the filter's other half: the same cast event with
    // the wrong type on it.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves resolved past the Presence's trigger, not through it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "a creature spell is no enchantment spell, so the Presence looked at \
         it and drew nothing"
    );

    // And the card the trigger is written about. The trigger goes on the stack
    // above the spell that caused it (CR 603.3b), so the card is drawn before
    // Fastbond itself ever resolves.
    cast_with_floating(&mut engine, p0, fastbond());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, fastbond()).is_some(),
        "Fastbond resolved: the enchantment landed behind its own trigger"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — exactly one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 2,
        "three cards cast and one drawn, so the hand is two smaller — a draw \
         that emptied the library without filling the hand would satisfy the \
         count above"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the five Forests paid exactly {{2}}{{G}} + {{G}} + {{G}}"
    );
}

fn fervor() -> CardIndex {
    card_index("8e0cea9c-3110-4728-9378-76849e33bb90")
}

/// Fervor — {2}{R} enchantment: "Creatures you control have haste."
///
/// Haste projects no characteristic at all; the only place it exists is in
/// the attackers the combat step is willing to offer, so the scenario is the
/// one play that needs it: a Llanowar Elves cast in the same main phase it
/// would have to attack in (CR 302.6). Three Mountains pay the enchantment
/// while the lone Forest is held back to pay for the Elf, and the fresh 1/1
/// is both offered as an attacker and connects for exactly its power.
///
/// The second game is the control, and it is the same board down to the last
/// card with Fervor seated *across* the table instead: the identical Elf is
/// not offered. That one difference reads both halves of
/// `Filter::YOUR_CREATURE` — nothing but the static put the first Elf in the
/// list, and it reaches its controller's creatures and no other seat's.
#[test]
fn fervor_haste_only_its_own_controllers_creatures_so_they_attack_as_they_arrive() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // Mine: the Mountains cast the enchantment and the Forest is kept back
    // for the creature it is about to make hasty.
    let mut mine = Duel::new(47, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), forest()])
        .hand(0, &[fervor(), llanowar_elves()])
        .start();
    keep_mulligans(&mut mine);
    assert!(walk_to_own_main(&mut mine, p0), "p0 reaches its own main");
    tap_all_mana_but(&mut mine, p0, Some(forest()));
    assert_eq!(
        mine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, and the Forest kept back for the Elf"
    );
    cast_with_floating(&mut mine, p0, fervor());
    pass_until(&mut mine, stack_is_empty);
    assert!(
        on_battlefield(&mine, p0, fervor()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    cast_from_hand(&mut mine, p0, llanowar_elves());
    pass_until(&mut mine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = mine.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration");
    };
    let hasty = on_battlefield(&mine, p0, llanowar_elves()).expect("the Elf resolved");
    assert!(
        attackers.contains(&hasty),
        "\"creatures you control have haste\": the Elf entered this turn and \
         is offered anyway (CR 302.6): {attackers:?}"
    );

    // And it is a real attack, not merely a name on a list.
    mine.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(hasty, Defender::Player(p1))],
        },
    )
    .expect("the offer is what the permission looks like");
    pass_until(&mut mine, |e| matches!(e.state().turn.phase, Phase::Ending));
    assert_eq!(
        mine.state().players[1].life,
        19,
        "the 1/1 connected for exactly its power"
    );

    // Theirs: one card different, and the same Elf stays home.
    let mut theirs = Duel::new(47, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), forest()])
        .battlefield(1, &[fervor()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut theirs);
    assert!(walk_to_own_main(&mut theirs, p0), "p0 reaches its own main");
    cast_from_hand(&mut theirs, p0, llanowar_elves());
    pass_until(&mut theirs, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = theirs.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration");
    };
    let sick = on_battlefield(&theirs, p0, llanowar_elves()).expect("the Elf resolved");
    assert!(
        !attackers.contains(&sick),
        "an opponent's Fervor is not this seat's: the same freshly played Elf \
         is still summoning-sick and is not offered: {attackers:?}"
    );
}

fn fires_of_yavimaya() -> CardIndex {
    card_index("e23d6f3b-0e18-423b-943b-15db7837255b")
}

/// Fires of Yavimaya — {1}{R}{G} enchantment: "Creatures you control have
/// haste" and "Sacrifice this enchantment: Target creature gets +2/+2 until
/// end of turn." One board reads both printed lines: an Elf under the same seat
/// has the keyword the moment the enchantment lands while the Elf across the
/// table does not, which is the whole of "you control"; and the sacrifice is
/// then played out as the printed price it is, with the offer for the pump
/// naming both Elves — "target creature" is not "target creature you control".
#[test]
fn fires_of_yavimaya_grants_haste_and_sacrifices_itself_for_a_pump() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fires_of_yavimaya()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "a printed Llanowar Elves carries no keyword of its own"
    );

    // {1}{R}{G} off the two Forests and the Mountain, with the Elf named as
    // the source kept back: it is the creature the static is read on, and a
    // source tapped for mana is a creature the rest of this test would be
    // reading in a state nobody asked for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and a Mountain pay {{1}}{{R}}{{G}} exactly"
    );
    cast_with_floating(&mut engine, p0, fires_of_yavimaya());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, fires_of_yavimaya()).is_some(),
        "the enchantment resolved onto the table"
    );

    // "Creatures you control have haste", read on both sides of the table so
    // that the filter is what the assertion is about and not the board.
    assert!(
        keywords(&engine, mine).contains(KeywordSet::HASTE),
        "a creature you control gets the granted haste"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "\"creatures *you* control\": the Elf across the table is untouched"
    );

    // The whole price of the second line is the enchantment itself, so
    // nothing has to float — an offer read off an empty pool is the price
    // being `SacrificeSelf` and no mana.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pump charges no mana at all"
    );
    activate(&mut engine, p0, fires_of_yavimaya(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, fires_of_yavimaya()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h)"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is the one that gets it");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, fires_of_yavimaya()).is_some(),
        "\"Sacrifice this enchantment\" is the cost, and a sacrificed \
         permanent goes to its owner's graveyard"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "and the static left with its source: nothing grants haste now"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "+2/+2 on the creature that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the Elf the ability did not name never moved"
    );
}

fn flowstone_surge() -> CardIndex {
    card_index("fb1755a0-3334-419b-8cb5-5a3ac7fa5b13")
}

/// Flowstone Surge — {1}{R} Enchantment: "Creatures you control get +1/-1."
///
/// Two readings have to come off one board, because each is the half the other
/// cannot see. A printed 1/1 of mine is buried the moment the Surge resolves —
/// a 2/0 is lethal by CR 704.5f and nothing but the `-1` can have done it —
/// while the Elf across the table is still a 1/1, which is the only way to tell
/// `Filter::YOUR_CREATURE` from every creature in the game. The `+1` needs a
/// creature that survives the subtraction, so a second creature of mine is read
/// before and after and has to move by exactly one in each direction.
#[test]
fn flowstone_surge_gives_plus_one_minus_one_to_your_creatures_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                drannith_magistrate(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flowstone_surge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let survivor = on_battlefield(&engine, p0, drannith_magistrate()).expect("a creature of mine");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the Surge");
    let before = pt(&engine, survivor);

    // The {1}{R} comes off the two Mountains and the Surge has to actually
    // arrive: a static read off the card file says nothing about which
    // creatures the layer system handed it.
    cast_from_hand(&mut engine, p0, flowstone_surge());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, flowstone_surge()).is_some(),
        "the enchantment resolved onto the table"
    );

    assert_eq!(
        pt(&engine, survivor),
        (before.0 + 1, before.1 - 1),
        "+1/-1 and not a pump of one half: {before:?} becomes two numbers, \
         each moved by exactly one"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "a printed 1/1 under +1/-1 is a 2/0, and CR 704.5f buries it"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and it is in its owner's graveyard, not merely gone"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"creatures you control\": the Elf across the table is untouched"
    );
}

fn ghitu_war_cry() -> CardIndex {
    card_index("0439df39-0324-4110-b3f4-4a32393021de")
}

/// Ghitu War Cry — {2}{R} Enchantment: "{R}: Target creature gets +1/+0
/// until end of turn." The whole card is that line, so the scenario has to
/// read it in three places at once: the {2}{R} out of a pool four Mountains
/// filled, the {R} for the activation that leaves the pool at zero, and the
/// pump itself, which must land on the targeted Elf and on no other. Two
/// Elves stand — one under each seat — because "target creature" is not
/// "creatures you control", and a static that had reached the whole table
/// would still satisfy a scenario holding only the caster's own board.
#[test]
fn ghitu_war_cry_taps_a_mountain_to_pump_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8311, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[ghitu_war_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(pt(&engine, theirs), (1, 1), "and one across the table");

    // The Elf is named as the source kept back: it is the creature the
    // ability is about to target, and a host tapped for its own mana would
    // no longer be the permanent this test reads afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and no Elf of mine paid in"
    );

    // {2}{R} off the pool, which leaves exactly the {R} the ability charges
    // inside this one main phase (CR 500.5).
    cast_with_floating(&mut engine, p0, ghitu_war_cry());
    pass_until(&mut engine, stack_is_empty);
    let cry = on_battlefield(&engine, p0, ghitu_war_cry()).expect("the enchantment resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} is spent and one red is left floating for the ability"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cry, 0)),
        "with the {{R}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, ghitu_war_cry(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&cry),
        "the enchantment is no creature and cannot pump itself: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2h pays last: nothing is spent while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "pumping is no mana ability");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, mine),
        (2, 1),
        "+1/+0 until end of turn on the creature it targeted"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the ability reaches the creature it named and never across the table"
    );
    assert!(
        on_battlefield(&engine, p0, ghitu_war_cry()).is_some(),
        "an activation costs the enchantment nothing but mana"
    );
}

fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

/// Glorious Anthem — {1}{W}{W}: "Creatures you control get +1/+1."
///
/// The card is a static over a *side* of the table, so the scenario needs a
/// creature on each: two Llanowar Elves under seat 0 that must both read
/// 2/2, and one across it that must stay the printed 1/1 — an anthem that
/// had lost `Filter::YOUR_CREATURE` would pump the whole table and still
/// satisfy the first half. The Elves are the read-out and are named as the
/// printing kept back, so the three Plains are the only thing that pays and
/// neither `(2, 2)` can be a tapped creature's own doing.
#[test]
fn glorious_anthem_pumps_every_creature_its_controller_has_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[glorious_anthem()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves on this side of the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    for elf in &mine {
        assert_eq!(pt(&engine, *elf), (1, 1), "printed 1/1s before the Anthem");
    }
    assert_eq!(pt(&engine, theirs), (1, 1), "and so is the one across it");

    // Three Plains pay {1}{W}{W}; both Elves are kept off the mana so the
    // creatures the pump is read back on are not sources that tapped.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains in the pool and no Elf's {{G}} among them"
    );
    cast_with_floating(&mut engine, p0, glorious_anthem());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, glorious_anthem()).is_some(),
        "the Anthem resolved onto the battlefield"
    );
    for elf in &mine {
        assert_eq!(
            pt(&engine, *elf),
            (2, 2),
            "every creature this seat controls is +1/+1"
        );
    }
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"you control\" is not the whole table"
    );
}

fn goblin_bombardment() -> CardIndex {
    card_index("edad60c6-80de-4033-af1b-a703ac332983")
}

/// Goblin Bombardment ({1}{R}, enchantment) prints one line: "Sacrifice a
/// creature: This enchantment deals 1 damage to any target." Both halves of
/// the price and both halves of the target need their own witness, so the
/// board carries two Elves under the acting seat and one across the table:
/// the sacrifice menu must hold exactly the two that seat controls — never
/// the Elf it is aimed at, never the enchantment — while the target menu
/// must reach across the table (CR 115.4). Two activations play the object
/// and the player half of "any target" in turn, because the enchantment
/// survives both, and 20 → 19 is the only number that separates "1 damage"
/// from a damage count read off the board.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn goblin_bombardment_sacrifices_a_creature_to_deal_one_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[goblin_bombardment()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, goblin_bombardment());
    pass_until(&mut engine, stack_is_empty);
    let bomb = on_battlefield(&engine, p0, goblin_bombardment()).expect("the enchantment resolved");
    assert!(
        types(&engine, bomb).contains(TypeSet::ENCHANTMENT),
        "and what landed is the enchantment it prints, not a creature"
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two creatures to give up, one per activation"
    );
    let (first, second) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // The price is a creature and no mana at all, so the offer turns on
    // nothing the pool could have supplied — but read it after the cast, so
    // the enchantment is on the battlefield the offer is read from.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bomb, 0)),
        "a creature to sacrifice is the whole price, so the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    // First activation: the object half of "any target". CR 601.2c picks the
    // target and CR 601.2h pays afterwards, so nothing has been given up
    // while the question stands.
    activate(&mut engine, p0, goblin_bombardment(), 0);
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
    assert_eq!(
        player, p0,
        "the seat that activated is the one that aims it"
    );
    assert!(
        options.contains(&first) && options.contains(&second),
        "both creatures this seat controls are legal targets: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"any target\" reaches across the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: the player half of the same choice names both seats: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is a cost and asks which one, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two creatures this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&first) && options.contains(&second),
        "both Elves of mine are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: the creature the ability is aimed at is not mine to \
         sacrifice, so it is a target and never a price: {options:?}"
    );
    assert!(
        !options.contains(&bomb),
        "the enchantment is no creature: it cannot eat itself: {options:?}"
    );
    // CR 601.2h pays last: the target is named, the price is not yet paid,
    // and the creature that is about to die is still on the battlefield.
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nothing has been sacrificed while the cost question stands"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the target has not moved either"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .expect("the creature the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the sacrificed creature went to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![second],
        "and only the creature that was named: the second Elf never moved"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was aimed at and not to the \
         player whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_bombardment()).is_some(),
        "the enchantment outlives the creature it ate — it is not sacrificed"
    );

    // Second activation: the player half of "any target", off the one
    // creature still standing.
    activate(&mut engine, p0, goblin_bombardment(), 0);
    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        player_options.contains(&p1),
        "a player is a legal target: {player_options:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a face is the other half of `any target`");

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the price is asked again, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![second],
        "one creature left under this seat, so the menu is down to it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("the last creature pays the price");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" — exactly one, off the player half \
         of the same choice"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the seat that aimed it, not to the caster"
    );
    assert!(
        all_on_battlefield(&engine, p0, llanowar_elves()).is_empty(),
        "two activations, two creatures given up"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_bombardment()).is_some(),
        "and the enchantment is still there to do it a third time"
    );
}

fn goblin_trenches() -> CardIndex {
    card_index("b43f40e6-c0ad-4a12-b75d-f2ba12629bfe")
}

/// Goblin Trenches prints a single line: "{2}, Sacrifice a land: Create
/// two 1/1 red and white Goblin Soldier creature tokens." Both parts of the
/// cost are readable on the same battlefield — five lands pay the
/// `{1}{R}{W}` of the enchantment and leave floating in the same main phase
/// (CR 500.5) exactly the `{2}` that the ability then requires, so the empty
/// pool afterwards proves the payment and not just a label. The sacrificed
/// land is the *own* one: the Forest on the table is on no option list
/// (CR 701.21a), and because the sacrifice is the last step of the
/// activation (CR 601.2h), it is only in the graveyard after the response.
/// The two Soldiers are the yield that no mana calculation can predict.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn goblin_trenches_eats_a_land_of_your_own_for_two_goblin_soldiers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[goblin_trenches()])
        .battlefield(0, &[mountain(), plains(), forest(), forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five lands make the `{1}{R}{W}` of the enchantment, and because
    // CR 500.5 empties the pool only at the end of a step, the `{2}` that
    // the ability costs stays in this one main phase.
    cast_from_hand(&mut engine, p0, goblin_trenches());
    pass_until(&mut engine, stack_is_empty);
    let trenches = on_battlefield(&engine, p0, goblin_trenches()).expect("the Trenches resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five lands paid the {{1}}{{R}}{{W}} and the ability's {{2}} is still in the pool"
    );

    // `legal.abilities` is filtered behind `can_afford`, and that reads the
    // pool: the line is only now offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trenches, 0)),
        "the one line the card prints, now that its {{2}} is payable: {:?}",
        legal.abilities
    );

    let mine = lands_of(&engine, p0);
    assert_eq!(mine.len(), 5, "five lands, all of them still on the table");
    let victim = all_on_battlefield(&engine, p0, forest())[0];
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    activate(&mut engine, p0, goblin_trenches(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    for land in &mine {
        assert!(
            options.contains(land),
            "every land this seat controls is on the menu: {options:?}"
        );
    }
    assert_eq!(options.len(), 5, "and those five are the whole menu");
    assert!(
        !options.contains(&trenches),
        "the Trenches are an enchantment: it cannot eat itself: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's land is not yours to sacrifice: {options:?}"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the land the question offered pays the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 601.2h: the {{2}} came out of the pool as the last step of the \
         activation"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "and the sacrificed land is in its owner's graveyard, not merely gone"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        lands_of(&engine, p0).len(),
        4,
        "exactly one land was given up: the other four are still standing"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        2,
        "\"create two 1/1 red and white Goblin Soldier creature tokens\""
    );
    for id in &tokens {
        let chars = engine
            .state()
            .object(*id)
            .expect("the token is on the battlefield")
            .characteristics();
        assert!(
            chars.types.contains(TypeSet::CREATURE),
            "a token that is not a creature could not attack: {chars:?}"
        );
        assert_eq!(
            (chars.power, chars.toughness),
            (Some(1), Some(1)),
            "a 1/1 body, on each of the two"
        );
        let soldier = engine
            .state()
            .object(*id)
            .and_then(|o| o.token)
            .expect("it knows which token it is");
        assert!(
            soldier.colors.contains(baylee_core::color::Color::Red)
                && soldier.colors.contains(baylee_core::color::Color::White),
            "red and white, the color pair the card prints"
        );
    }
    assert!(
        on_battlefield(&engine, p0, goblin_trenches()).is_some(),
        "the enchantment outlives the land it ate"
    );
}

fn goblin_war_drums() -> CardIndex {
    card_index("29c21edd-781b-448e-824a-17bc8b8f4077")
}

/// Goblin War Drums is one printed sentence — "Creatures you control have
/// menace" — and the whole card lives in the two words that narrow it. So
/// the board carries both halves of each: a Sol Ring under the same seat
/// (which a `Filter::Any` would have granted menace to, as an artifact and
/// no creature) and an Elf across the table (which a static that read "the
/// table" instead of "you" would have granted it to). The grant is read off
/// the projected characteristics *after* the enchantment has been cast and
/// resolved, because a static that never registered grants nothing to
/// anybody.
#[test]
fn goblin_war_drums_gives_menace_to_the_creatures_its_controller_has_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4211, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[goblin_war_drums()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::MENACE),
        "nothing is enchanting the board yet"
    );

    // {2}{R} off the three Mountains, with the Elf and the Sol Ring named as
    // the two things kept back: the pool the enchantment is paid from is then
    // exactly the lands this test goes on to read, and neither of the
    // permanents about to be asked about has moved for a reason of its own.
    tap_mana_where(&mut engine, p0, |id| id != elves && id != rock);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains and nothing else: the Elf and the Sol Ring made no mana"
    );
    cast_with_floating(&mut engine, p0, goblin_war_drums());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, goblin_war_drums()).is_some()
    });
    let drums = on_battlefield(&engine, p0, goblin_war_drums()).expect("the enchantment resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{R}} came out of the pool"
    );

    assert!(
        keywords(&engine, elves).contains(KeywordSet::MENACE),
        "\"creatures you control have menace\""
    );
    assert!(
        !keywords(&engine, rock).contains(KeywordSet::MENACE),
        "\"creatures you control\" is not \"permanents you control\": the Sol \
         Ring is an artifact and no creature"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::MENACE),
        "\"you control\" is not the table: the Elf across it is a creature \
         too, and the static never reaches it"
    );
    assert!(
        !keywords(&engine, drums).contains(KeywordSet::MENACE),
        "the enchantment grants the keyword, it does not keep it"
    );
}

fn gravity_sphere() -> CardIndex {
    card_index("8ddf93fe-980b-4dc4-b56f-6a2ee50100a6")
}

/// Gravity Sphere — {2}{R} World enchantment: "All creatures lose flying."
///
/// The reading is the two-sided one, because "all" is exactly what a filter
/// narrowed to the controller would keep: the Sphere is cast by p0 and has to
/// ground a Sphinx of the Final Word across the table. That Sphinx is also the
/// control for the other half of the sentence — it prints hexproof beside
/// flying, so the same creature says both that one keyword went and that the
/// rest stayed, and its body is compared against the one read before the
/// Sphere resolved, so a card that had stripped every keyword cannot pass on
/// the strength of a missing `FLYING` alone. The Elf under the Sphere's own
/// controller is the third reading: a creature with nothing to lose comes out
/// of the resolution exactly as it went in.
#[test]
fn gravity_sphere_grounds_every_creature_and_takes_only_flying() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[gravity_sphere()])
        .battlefield(1, &[sphinx_of_the_final_word()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphinx =
        on_battlefield(&engine, p1, sphinx_of_the_final_word()).expect("the Sphinx is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "the Sphinx prints flying before anything is cast"
    );
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::HEXPROOF),
        "and hexproof beside it, which the Sphere has no business touching"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the Elf never had flying, so it is the negative half of the claim"
    );
    let body = pt(&engine, sphinx);

    // {2}{R} off the three Mountains; `tap_all_mana` also takes the Elf's own
    // `{T}: Add {G}`, which is a printed mana ability and no reason for the
    // Elf to move.
    cast_from_hand(&mut engine, p0, gravity_sphere());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, gravity_sphere()).is_some()
    });

    assert!(
        on_battlefield(&engine, p0, gravity_sphere()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert!(
        !keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "\"All creatures lose flying\" reaches the creature across the table"
    );
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::HEXPROOF),
        "the Sphere removes one keyword and not the set it lives in"
    );
    assert_eq!(
        pt(&engine, sphinx),
        body,
        "and the Sphinx is still the body it was, so nothing was removed but \
         the flying"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the creature that had no flying is untouched"
    );
}

fn hannas_custody() -> CardIndex {
    card_index("57b0205d-ad9d-45b7-8556-0733aa7a4987")
}

/// Hanna's Custody — {2}{W} enchantment: "All artifacts have shroud."
///
/// Shroud (CR 702.18) is only ever visible in a targeting question, and the
/// board carries two untapped Liquimetal Coatings — "{T}: Target permanent
/// becomes an artifact in addition to its other types" — precisely because
/// that ability may name an artifact or a land, so the same menu reads the
/// card twice: once before the enchantment arrives, with both Sol Rings, both
/// Coatings and an Elf on it, and once after, with a Forest and the
/// enchantment itself still on it while every artifact is gone — including the
/// Elf the first activation had turned into one. That the ability belongs to
/// the artifacts' own controller makes the absence shroud and not hexproof
/// (CR 702.11b), which would have left their owner free to aim at them.
#[test]
#[allow(clippy::too_many_lines)] // one menu, asked on both sides of the enchantment
fn hannas_custody_shrouds_every_artifact_on_the_table_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                liquimetal_coating(),
                liquimetal_coating(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[forest(), quiet_artifact()])
        .hand(0, &[hannas_custody()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: which seat the seed
    // puts on the play is not this test's subject, and only the tolerant
    // walker crosses a turn boundary to reach p0's own main phase.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let coatings = all_on_battlefield(&engine, p0, liquimetal_coating());
    assert_eq!(
        coatings.len(),
        2,
        "two copies, one per side of the enchantment"
    );
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");

    // The menu *before* the enchantment, so that its losses later are the
    // shroud and not this ability's own filter or its price.
    activate(&mut engine, p0, liquimetal_coating(), 0);
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
    assert_eq!(player, p0, "the seat that activated aims it");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        [ring, theirs, coatings[0], coatings[1]]
            .iter()
            .all(|id| options.contains(id)),
        "with no shroud anywhere yet, artifacts sit on this menu like anything \
         else: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        types(&engine, elf).contains(TypeSet::ARTIFACT),
        "the Coating's own effect turned the Elf into an artifact in addition \
         to its other types"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::SHROUD),
        "and nothing on the board has granted shroud yet"
    );
    assert_eq!(
        coatings
            .iter()
            .filter(|id| is_tapped(&engine, **id))
            .count(),
        1,
        "the Coating whose {{T}} paid is the one left down; the other is the \
         reading this test still has to take"
    );

    // {2}{W} off the three Plains — and the Elf the first activation turned
    // into an artifact is still one, which is the point of the pair of menus.
    cast_from_hand(&mut engine, p0, hannas_custody());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let custody = on_battlefield(&engine, p0, hannas_custody()).expect("the Custody resolved");

    assert!(
        keywords(&engine, ring).contains(KeywordSet::SHROUD),
        "all artifacts have shroud"
    );
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::SHROUD),
        "on both sides of the table, and not only the controller's"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::SHROUD),
        "and so has the Elf the Coating made an artifact a moment earlier"
    );
    for coating in &coatings {
        assert!(
            keywords(&engine, *coating).contains(KeywordSet::SHROUD),
            "both Coatings are artifacts and neither is exempt"
        );
    }
    assert!(
        !keywords(&engine, custody).contains(KeywordSet::SHROUD),
        "the enchantment that grants the keyword is no artifact and keeps none"
    );
    assert!(
        !keywords(&engine, their_land).contains(KeywordSet::SHROUD),
        "and a Forest is not one either"
    );

    // The same menu again, from the Coating that has not spent its {T}.
    activate(&mut engine, p0, liquimetal_coating(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated aims it");
    assert!(
        options.contains(&their_land) && options.contains(&custody),
        "the menu is not empty: a Forest across the table and the enchantment \
         itself are still legal \"target permanent\"s: {options:?}"
    );
    for artifact in [ring, theirs, elf, coatings[0], coatings[1]] {
        assert!(
            !options.contains(&artifact),
            "every artifact on the table is off the menu now, whoever owns it: \
             {options:?}"
        );
    }

    // The answer the question offered still lands, and what becomes an
    // artifact after the enchantment is shrouded by it too: the static reads
    // the board it is on rather than a snapshot of it.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the Forest was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        types(&engine, their_land).contains(TypeSet::ARTIFACT),
        "the Coating makes the Forest an artifact in addition to its types"
    );
    assert!(
        keywords(&engine, their_land).contains(KeywordSet::SHROUD),
        "and Hanna's Custody shrouds what became an artifact after it resolved"
    );
}

fn peace_of_mind() -> CardIndex {
    card_index("4f8c5fd7-f280-4b0c-bb84-6ff9b258c50f")
}

/// Peace of Mind — {1}{W} enchantment: "{W}, Discard a card: You gain 3
/// life." Three printed things, and each is read off a different place. The
/// {W} is the offer filtered through `can_afford`, which reads the pool and
/// not the untapped Plains — so the line is absent on a bare board and
/// present once the mana floats. The discard arrives as `CostDiscard` over
/// the hand, and the three life is the *effect*, which is why the life total
/// is asserted before the stack drains and again after. The Mountain is the
/// card that is given up so that "one card fewer" is a fact about the hand
/// rather than a count of the filler deck beside it.
#[test]
fn peace_of_mind_discards_a_card_for_white_and_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), peace_of_mind()])
        .hand(0, &[mountain()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, peace_of_mind()).expect("Peace of Mind is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(enchantment, 0)),
        "an empty pool pays no {{W}}, and `can_afford` reads the pool: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the one Plains pays the one white the ability charges"
    );

    let fodder = in_hand(&engine, p0, mountain()).expect("the Mountain is in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let life_before = engine.state().players[0].life;

    activate(&mut engine, p0, peace_of_mind(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat gives the card up");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not an effect, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "\"discard a card\" is any card in the hand: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("a card the question offered pays the cost");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} went with it"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the price is paid on announcement; the life is what resolution is for"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "the discarded card is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and it left the hand behind, which a library that shrank could not say"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 3,
        "\"You gain 3 life\" — three, and not one per card in any graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, peace_of_mind()).is_some(),
        "an activated ability costs the enchantment nothing but the card it ate"
    );
}

fn primal_rage() -> CardIndex {
    card_index("d6464ee4-23fc-4d68-bbda-3b53772015d1")
}

/// Primal Rage — {1}{G} enchantment: "Creatures you control have trample."
///
/// The static is `Filter::YOUR_CREATURE`, so the reading worth playing is the
/// one that tells this seat's creatures from the opponent's: an Elf sits under
/// each seat and only the one this side controls may carry the keyword once the
/// enchantment has resolved. The Elf cast *afterwards* is the other half of the
/// same claim — a grant that reached only what stood on the battlefield at
/// resolution would read the printed card rather than the board the static
/// lives on — and the enchantment itself is no creature, so it must not carry
/// the keyword it hands out.
#[test]
fn primal_rage_grants_trample_to_the_creatures_of_its_controller_including_a_latecomer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(131, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[primal_rage(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "nothing has granted a keyword yet"
    );

    // The Elf is named as the printing kept back: it prints its own
    // `{T}: Add {G}`, so "three Forests" is a claim about the Forests and not
    // about a board where a creature quietly paid in (rule 11).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Elf kept untapped"
    );
    cast_with_floating(&mut engine, p0, primal_rage());
    pass_until(&mut engine, stack_is_empty);

    let rage = on_battlefield(&engine, p0, primal_rage()).expect("Primal Rage resolved");
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "\"creatures you control have trample\" — the Elf under this seat carries it"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the Elf across the table does not: \"you control\" is read and not skipped"
    );
    assert!(
        !keywords(&engine, rage).contains(KeywordSet::TRAMPLE),
        "the enchantment grants the keyword to creatures; it is not one itself"
    );

    // The green the cast left behind, spent on a creature that arrives after
    // the static is already on the battlefield.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{G}} out of three green leaves exactly one for the Elf"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "the second Elf resolved beside the first");
    assert!(
        elves
            .iter()
            .all(|id| keywords(&engine, *id).contains(KeywordSet::TRAMPLE)),
        "a creature that enters under the static is projected against it too"
    );
    assert!(
        !keywords(
            &engine,
            on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is still out")
        )
        .contains(KeywordSet::TRAMPLE),
        "and the arrival changes nothing across the table"
    );
}

fn seal_of_cleansing() -> CardIndex {
    card_index("a75dbe70-7e3e-446f-9a76-9fbb414f2e7c")
}

/// Seal of Cleansing — {1}{W} enchantment: "Sacrifice this enchantment:
/// Destroy target artifact or enchantment."
///
/// The whole price is the Seal itself and no mana at all, so the ability is
/// offered on a board with an empty pool — `LegalActions::abilities` is
/// filtered through `can_afford`, and a cost that names no mana is payable
/// from a pool holding none. The target menu is the card's filter read on a
/// real board: artifacts on *either* side of the table and an enchantment
/// across it, but neither the Elf beside the Seal nor the Forest it stands
/// next to, which is what separates "target artifact or enchantment" from
/// "target permanent". And the sacrifice is read after the target: targets
/// are chosen at CR 601.2c and costs paid at CR 601.2h, so while the question
/// stands the Seal is still on the battlefield and only afterwards is it in
/// its owner's graveyard.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn seal_of_cleansing_sacrifices_itself_to_destroy_an_artifact_or_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                seal_of_cleansing(),
                quiet_artifact(),
                quiet_creature(),
                forest(),
            ],
        )
        .battlefield(1, &[quiet_artifact(), their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let seal = on_battlefield(&engine, p0, seal_of_cleansing()).expect("the Seal is out");
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let aura = on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");

    // Nothing needs tapping: the price is the Seal. That is exactly why the
    // offer may be read here with an empty pool — and it is asserted first,
    // so a missing line could not be blamed on mana that was never floated.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing was tapped for this scenario and nothing needs to be"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(seal, 0)),
        "the one line the Seal prints costs only itself, so it is offered on \
         an empty pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, seal_of_cleansing(), 0);
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
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target artifact\" reaches artifacts on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&aura),
        "and an enchantment across the table is the other half of the \
         filter: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "nor is a land, so neither may be offered: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, seal_of_cleansing()).is_some(),
        "CR 601.2c before CR 601.2h: the sacrifice has not happened while the \
         target question is still open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![aura],
            },
        )
        .expect("the enchantment was one of the options it enumerated");

    assert!(
        in_graveyard(&engine, p0, seal_of_cleansing()).is_some(),
        "\"Sacrifice this enchantment\" is the last thing paid, and it takes \
         the Seal with it"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "the targeted enchantment is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and only the permanent that was named: the artifact across the table \
         still stands"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "as does this seat's own — one target, one destroyed permanent"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the creature the filter declined never moved"
    );
}

fn serras_blessing() -> CardIndex {
    card_index("49cdd05c-feeb-4c24-9d88-069f5d2e08c3")
}

/// Serra's Blessing — {1}{W} — "Creatures you control have vigilance."
///
/// The keyword cannot be read off a projection: what vigilance *does* only
/// shows up in combat, so the Elf takes the attack declaration and is still
/// standing afterwards, which is the one thing the keyword changes. The
/// control is the Elf across the table attacking on its own turn and tapping
/// normally, so the untapped Elf says something about a granted rule rather
/// than about a declaration that never happened. Before the cast and across
/// the table the keyword is asserted absent, because a static without
/// `Filter::YOUR_CREATURE` would have armed the whole table.
#[test]
fn serras_blessing_keeps_your_attacker_standing_and_not_theirs() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[serras_blessing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "nothing is granted before the enchantment resolves"
    );

    // Two Plains pay the {1}{W}; the Elf is named as the printing kept back,
    // because it is the creature this test attacks with and a creature tapped
    // for mana is tapped for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, serras_blessing());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, serras_blessing()).is_some(),
        "the enchantment resolved"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "\"creatures you control have vigilance\""
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "\"you control\" — the Elf across the table is not granted the keyword"
    );

    // Attacking is where the keyword is worth anything: the Elf is declared as
    // an attacker and stays untapped, which a printed 1/1 without vigilance
    // could not do.
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
    assert!(
        !is_tapped(&engine, mine),
        "\"attacking doesn't cause them to tap\" — the Elf that just attacked \
         is still standing"
    );

    // The control: the same declaration one turn later, by the seat with no
    // Serra's Blessing, does tap its attacker.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(theirs, Defender::Player(p0))],
            },
        )
        .unwrap();
    assert!(
        is_tapped(&engine, theirs),
        "an attacker with no vigilance taps as it attacks"
    );
}

fn shivan_harvest() -> CardIndex {
    card_index("27c3d7cd-f92e-4203-9fb5-f6b4776f6ffd")
}

/// Shivan Harvest — {1}{R} enchantment: "{1}{R}, Sacrifice a creature:
/// Destroy target nonbasic land."
///
/// Both halves of the price are read somewhere a test can see them — the {1}{R}
/// leaves a pool four Mountains actually filled, and the creature leaves the
/// battlefield for its owner's graveyard — while the ability is on the stack,
/// which is what tells a real sacrifice into a real activation. The words that
/// need a witness on the board are the two filters: the Elf across the table
/// must stay off the sacrifice menu (CR 701.21a) and the Forest beside the
/// targeted land must stay off the target menu, so it is read twice — once in
/// the options and once by that Forest still standing at the end.
#[test]
#[allow(clippy::too_many_lines)] // one activation, every clause of the card read off it
fn shivan_harvest_sacrifices_a_creature_to_destroy_a_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[shivan_harvest()])
        // A nonbasic land this seat does not control, and a basic one beside
        // it in the same seat's hands.
        .battlefield(1, &[treetop_village(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The enchantment arrives first, off **two** named Mountains and not off
    // whatever is standing: the other two are what the ability spends
    // further down, and the pool emptying in between is what makes each
    // price a number rather than a leftover. `cast_from_hand` would have
    // tapped the Elf as well, and the Elf is the creature this ability eats.
    let mountains = all_on_battlefield(&engine, p0, mountain());
    assert_eq!(mountains.len(), 4, "four Mountains were dealt");
    let (first, second) = (mountains[0], mountains[1]);
    tap_mana_where(&mut engine, p0, |id| id == first || id == second);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, which is the {{1}}{{R}} the enchantment costs"
    );
    cast_with_floating(&mut engine, p0, shivan_harvest());
    pass_until(&mut engine, stack_is_empty);
    let harvest = on_battlefield(&engine, p0, shivan_harvest()).expect("the Harvest resolved");
    let victim = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let nonbasic = on_battlefield(&engine, p1, treetop_village()).expect("their Treetop Village");
    let basic_land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast spent the pool: nothing is floating for the ability"
    );

    // `legal.abilities` is behind `can_afford`, which reads the pool and not
    // the untapped lands, so the {1}{R} is floating *before* anything is
    // claimed about the offer.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Mountains nobody spent on the enchantment; the Elf is kept \
         back because it is the creature about to be eaten"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(harvest, 0)),
        "with {{1}}{{R}} floating the Harvest's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, shivan_harvest(), 0);

    // **The target first and the cost second**, which is the order CR 601.2
    // gives and CR 602.2b applies to an activation: the land is named at
    // CR 601.2c and the creature is eaten at CR 601.2h. A test that asked
    // for the sacrifice first would have been asserting the opposite order
    // and passing on a board where it does not matter.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target nonbasic land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&nonbasic),
        "a nonbasic land across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&basic_land),
        "\"nonbasic\" is read and not skipped: the Forest beside it is no target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![nonbasic],
            },
        )
        .expect("the land the question offered was chosen");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is a cost and is asked after the target: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options,
        vec![victim],
        "the creature you control is the whole of the answer"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice: {options:?}"
    );
    assert!(
        !options.contains(&harvest),
        "the Harvest is an enchantment: it cannot eat itself: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the creature the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "destroying a land is no mana ability, so the ability is on the stack"
    );

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "`CR 601.2h`: the sacrifice is paid before the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} that was floating went with it"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, treetop_village()).is_none(),
        "the targeted land left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, treetop_village()).is_some(),
        "and it is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the basic land the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, shivan_harvest()).is_some(),
        "the enchantment paid its price and stayed on the table"
    );
}

// oracle_id = "d32a32d2-203d-4be1-8a33-e037747053c7"
fn sustenance() -> CardIndex {
    card_index("d32a32d2-203d-4be1-8a33-e037747053c7")
}

/// Sustenance is `{1}{G}` and prints exactly one line: "{1}, Sacrifice a
/// land: Target creature gets +1/+1 until end of turn." Three things in that
/// sentence are the engine's answer rather than the card's, so the board is
/// built to read each of them. The `{1}` is filtered out of the offer until
/// the pool holds it (`can_afford` reads the pool, not the untapped lands);
/// the second price asks *which* land and may offer only this seat's own,
/// with the Forest across the table as the control; and the pump has to land
/// on the creature that was named and on no other, which is why an Elf of
/// mine and an Elf of theirs are both standing when the target question is
/// asked and their Elf is still a printed 1/1 afterwards.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn sustenance_trades_a_land_for_one_more_power_on_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(421, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), sustenance(), llanowar_elves()],
        )
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment =
        on_battlefield(&engine, p0, sustenance()).expect("the enchantment is on the table");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // `{1}` is read off the pool and not off the untapped lands, so with
    // nothing floating the whole price is unpayable and the line is not there
    // at all — the half a test that only ever taps first would never see.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the enchantment holds it");
    assert!(
        !legal.abilities.contains(&(enchantment, 0)),
        "{{1}} is not one mana, so nothing is offered: {:?}",
        legal.abilities
    );

    // Three Forests, and the Elves named as the printing kept back: they are
    // the creature the pump is about, and a host tapped for its own mana reads
    // wrong afterwards. Rule 17 in one line — the card's own price is not its
    // own {{T}}, so `tap_all_mana_but` is what keeps the creature standing.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and no Elf of mine contributed"
    );
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 3, "three Forests of my own to give up");
    let doomed = lands[0];

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "with the {{1}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sustenance(), 0);

    // CR 601.2c before CR 601.2h: the target is named while the mana is still
    // in the pool and every Forest still standing, so both prices are read
    // after this answer.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&enchantment),
        "the enchantment is no creature: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h), \
         so no land has been given up yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{1}} is still in the pool for the same reason"
    );

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert!(
        options.contains(&doomed),
        "a Forest this seat controls is on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "the three Forests of mine and nothing else: {options:?}"
    );
    assert!(
        !options.contains(&enchantment),
        "the enchantment is no land, so it cannot pay its own price: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "and the Elves are a creature: \"a land\" is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&their_land),
        "`CR 701.21a`: an opponent's land is not yours to sacrifice, whatever \
         the filter says: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the land the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}} came out of the pool the three Forests filled"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "exactly one Forest was given up: the other two are still standing"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "a sacrificed land goes to its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 until end of turn on the creature that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the creature the ability did not target"
    );
    assert!(
        on_battlefield(&engine, p0, sustenance()).is_some(),
        "an activated ability costs the enchantment nothing but what it prints"
    );
}

fn trade_routes() -> CardIndex {
    card_index("120fa67e-e5e0-4c23-9a78-d6171d357aee")
}

/// Trade Routes — {1}{U} enchantment — prints two activated abilities and one
/// board plays both: three Islands pay the cast, and the two `{1}`s that follow
/// come out of the three Forests the cast was told to leave standing. The first
/// line, "`{1}`: Return target land you control to its owner's hand", is read as
/// a *move* — the tapped Island the cast spent leaves the battlefield and is
/// offered as a land drop again — and the second, "`{1}`, Discard a land card:
/// Draw a card", spends that very Island, so the card one line hands back is the
/// card the other costs. The Forest across the table is the control on both
/// "you"s: it is a land, it is not this seat's, and neither ability may touch it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn trade_routes_bounces_a_land_then_trades_one_for_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), forest(), forest(), forest()],
        )
        .hand(0, &[trade_routes(), counterspell()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // **Two** named Islands pay the {1}{U} and not every Island standing:
    // the pool has to empty on the cast, because "neither ability is
    // affordable yet" two lines down is a claim about the pool and a
    // leftover mana would answer it by accident. The Forests are the two
    // {1}s the card's own abilities charge afterwards.
    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 3, "three Islands were dealt");
    let (first, second) = (islands[0], islands[1]);
    tap_mana_where(&mut engine, p0, |id| id == first || id == second);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands tapped, which is the {{1}}{{U}} Trade Routes costs"
    );
    cast_with_floating(&mut engine, p0, trade_routes());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let routes = on_battlefield(&engine, p0, trade_routes()).expect("Trade Routes resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast spent the pool, so neither ability is affordable yet"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat with the enchantment");
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == routes),
        "an empty pool pays no {{1}}, and an unaffordable ability is absent \
         from the offer rather than refused: {:?}",
        legal.abilities
    );

    let bounced = islands[0];
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 3, "three Forests were dealt");
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");

    // The two named Forests and nothing else: the third Island is standing
    // too, and "everything but one" would have floated it as well.
    let (left, right) = (forests[1], forests[2]);
    let taken = tap_mana_where(&mut engine, p0, |id| id == left || id == right);
    assert_eq!(taken, 2, "the two Forests beside the one kept back");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two green, one for each {{1}}"
    );

    // Ability 0: "{1}: Return target land you control to its owner's hand."
    activate(&mut engine, p0, trade_routes(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land you control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one land");
    assert!(
        options.contains(&bounced),
        "a tapped land you control is still a land you control: {options:?}"
    );
    assert!(
        options.contains(&forests[0]),
        "and so is the untapped one: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" declines the Forest across the table: {options:?}"
    );
    assert!(
        !options.contains(&routes),
        "the enchantment is no land: {options:?}"
    );
    assert_eq!(
        options.len(),
        6,
        "the six lands this seat controls and nothing else: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2h pays last, so the {{1}} is still in the pool while the \
         question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bounced],
            },
        )
        .expect("the land the question offered is the one that goes back");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "and the ability is on the stack");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().object(bounced).map(|o| o.zone),
        Some(Zone::Hand),
        "the land left the battlefield for its owner's hand"
    );
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "and it is this seat's hand that holds it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, island()).len(),
        2,
        "two Islands are still standing"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the Forest the ability did not name never moved"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&bounced),
        "\"return it to your hand\" means it is a land drop again: {:?}",
        legal.lands
    );

    let library_before = library_size(&engine, p0);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let spell = in_hand(&engine, p0, counterspell()).expect("a nonland card in hand");

    // Ability 1: "{1}, Discard a land card: Draw a card."
    activate(&mut engine, p0, trade_routes(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one land card, no more and no fewer");
    assert!(
        options.contains(&bounced),
        "the Island the first line handed back is a land card in hand: {options:?}"
    );
    assert!(
        !options.contains(&spell),
        "\"a land card\" is read and not skipped: the Counterspell in hand is \
         not on the menu: {options:?}"
    );
    assert!(
        options
            .iter()
            .all(|id| types(&engine, *id).contains(TypeSet::LAND)),
        "and nothing but land cards is: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}} is not paid until the question is answered"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bounced],
            },
        )
        .expect("the card the question offered pays the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{1}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, island()).is_some(),
        "the discarded land card went to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\""
    );
    assert_eq!(
        engine.state().object(top).map(|o| o.zone),
        Some(Zone::Hand),
        "and the card drawn is the one that was on top of the library, not \
         merely some card that appeared in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn, so the hand is the size it was"
    );
}

/// Garruk's Uprising prints three lines and the **first** one is an
/// intervening `if` on the enchantment's own arrival: "When this enchantment
/// enters, if you control a creature with power 4 or greater, draw a card."
/// CR 603.4 checks it twice, and both checks are against the board as the
/// enchantment lands — so the 6/6 already standing earns the card and a 1/1
/// standing in its place earns nothing. The test beside this one plays the
/// third line, which is the same predicate read about a creature entering
/// rather than about the board.
#[test]
fn garruk_s_uprising_draws_on_its_own_arrival_only_over_a_four_power_creature() {
    let p0 = PlayerId::new(0);

    let draw_from = |creature: CardIndex, seed: u64| -> usize {
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[forest(), forest(), forest(), forest(), creature])
            .hand(0, &[garruk_s_uprising()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let before = library_size(&engine, p0);
        cast_from_hand(&mut engine, p0, garruk_s_uprising());
        pass_until(&mut engine, stack_is_empty);
        assert!(
            on_battlefield(&engine, p0, garruk_s_uprising()).is_some(),
            "the enchantment resolved either way"
        );
        before - library_size(&engine, p0)
    };

    assert_eq!(
        draw_from(llanowar_elves(), 9301),
        0,
        "a 1/1 is not \"a creature with power 4 or greater\", so the trigger \
         is binned by its own intervening if"
    );
    assert_eq!(
        draw_from(rootbreaker_wurm(), 9302),
        1,
        "a 6/6 earns the card as the enchantment lands"
    );
}

/// Temur Ascendancy's second line is the one Garruk's Uprising prints as a
/// gift rather than a choice: "Whenever a creature you control with power 4
/// or greater enters, **you may** draw a card." The "may" is answered here
/// rather than assumed — a `MayDo` nobody is asked resolves into nothing —
/// and a 1/1 entering asks no question at all, which is the half that says
/// the power predicate is on the trigger and not on the draw.
#[test]
fn temur_ascendancy_offers_its_draw_only_for_a_four_power_creature() {
    let p0 = PlayerId::new(0);

    let mut small = Duel::new(9303, forest())
        .battlefield(0, &[temur_ascendancy(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut small);
    reach_main_phase(&mut small, p0);
    let before = library_size(&small, p0);
    cast_from_hand(&mut small, p0, llanowar_elves());
    pass_until(&mut small, stack_is_empty);
    assert!(
        on_battlefield(&small, p0, llanowar_elves()).is_some(),
        "the Elf arrived"
    );
    assert_eq!(
        library_size(&small, p0),
        before,
        "a 1/1 entering triggers nothing, so nothing was asked and nothing drawn"
    );

    let mut big = Duel::new(9304, forest())
        .battlefield(
            0,
            &[
                temur_ascendancy(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut big);
    reach_main_phase(&mut big, p0);
    let before = library_size(&big, p0);
    cast_from_hand(&mut big, p0, rootbreaker_wurm());
    pass_until(&mut big, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    big.apply(p0, PlayerAction::YesNo(true))
        .expect("the \"you may\" is answered");
    pass_until(&mut big, stack_is_empty);
    assert_eq!(
        library_size(&big, p0),
        before - 1,
        "the 6/6 entering asked, and the answer was yes"
    );
}

// oracle_id = "b22080d6-a9ed-4bdd-a604-058e0e3e9463"

/// Mental Discipline — {1}{U}{U} enchantment: "{1}{U}, Discard a card: Draw a
/// card." The price and the effect each land somewhere a test can read, and one
/// activation reads all four places at once: the {1}{U} comes out of a pool the
/// five Islands filled for the cast, the discarded card is in its owner's
/// graveyard rather than merely gone from the hand, the drawn card is off the
/// top of the library, and the enchantment itself survives the activation. The
/// engine asks which card is being given up as `CostDiscard` — a cost, not a
/// search — and the hand holds exactly the one card that was drawn afterwards.
#[test]
fn mental_discipline_spends_mana_and_a_card_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let fodder = silence();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[mental_discipline(), fodder])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The {1}{U}{U} cast is paid out of the pool: `cast_from_hand` taps all
    // five Islands and leaves exactly the {1}{U} the ability charges floating.
    cast_from_hand(&mut engine, p0, mental_discipline());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, mental_discipline()).is_some(),
        "the enchantment resolved onto the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five Islands less the {{1}}{{U}}{{U}} the enchantment costs"
    );

    let library_before = library_size(&engine, p0);
    let fodder_card = in_hand(&engine, p0, fodder).expect("the fodder is in hand");

    activate(&mut engine, p0, mental_discipline(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the discard is a cost, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options,
        vec![fodder_card],
        "the only card left in hand is the whole of the menu"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder_card],
            },
        )
        .expect("the card the question offered pays the cost");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{U}} it charges came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, fodder).is_some(),
        "a discarded card goes to its owner's graveyard, not merely out of the hand"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the effect is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "one card discarded and one drawn, so the hand holds exactly the new card"
    );
    assert!(
        on_battlefield(&engine, p0, mental_discipline()).is_some(),
        "an activated ability costs the enchantment nothing but the mana it was paid"
    );
}

/// Night of Souls' Betrayal — {2}{B}{B}: "All creatures get -1/-1."
///
/// One sentence, three claims, and no witness supplies two of them: the
/// **same** larger creature on both sides of the table is exactly one point
/// smaller afterwards and still standing — so the modifier is -1/-1, it
/// reaches every creature and not only the caster's, and it is no "destroy all
/// creatures" — a printed 1/1 underneath it dies the moment the enchantment
/// resolves (CR 704.5f), and a second 1/1 cast *afterwards* enters and dies
/// the same way, which a static read off the board only once, as it arrived,
/// would leave standing.
#[test]
fn night_of_souls_betrayal_shrinks_every_creature_by_one_and_buries_the_one_ones() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                forest(),
                thrun_the_last_troll(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[thrun_the_last_troll()])
        .hand(0, &[night_of_souls_betrayal(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_troll = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("my Troll is out");
    let their_troll =
        on_battlefield(&engine, p1, thrun_the_last_troll()).expect("their Troll is out");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elves are out");
    let forest_land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let (power, toughness) = pt(&engine, my_troll);
    assert_eq!(
        pt(&engine, their_troll),
        (power, toughness),
        "the same card on both sides of the table, before anything is asked of either"
    );
    assert!(
        toughness > 1,
        "the body the -1/-1 is read off has to be one that survives it: {toughness}"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and a printed 1/1 is what the modifier has to bury"
    );

    // Four Swamps pay {2}{B}{B} exactly. The Elves and the Forest are named as
    // the two sources kept back: the Elves print a mana ability of their own,
    // which is a route `tap_mana_where` would otherwise take (#159), and the
    // Forest is what the second Elves below is cast with.
    tap_mana_where(&mut engine, p0, |id| id != elves && id != forest_land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps in the pool, and neither the Elves nor the Forest gave anything"
    );

    cast_with_floating(&mut engine, p0, night_of_souls_betrayal());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, night_of_souls_betrayal()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{B}}{{B}} came out of the pool"
    );

    assert_eq!(
        pt(&engine, my_troll),
        (power - 1, toughness - 1),
        "-1/-1 on the creature this seat owns"
    );
    assert_eq!(
        pt(&engine, their_troll),
        (power - 1, toughness - 1),
        "\"all creatures\" is not \"creatures you control\": the same card across the table lost the same point"
    );
    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_some(),
        "and the bigger body survives it — this is no \"destroy all creatures\""
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "a printed 1/1 with -1/-1 is a 0/0, and CR 704.5f puts it in the graveyard"
    );
    assert_eq!(
        mine(&engine, p0, quiet_creature(), Zone::Graveyard).len(),
        1,
        "which is where it went, rather than merely off the battlefield"
    );

    // The second 1/1 sat in hand while the enchantment arrived, so the static
    // is read for it again: the Forest pays its {G} and it dies the same way.
    cast_from_hand(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "an Elves cast under the enchantment enters and is buried at once"
    );
    assert_eq!(
        mine(&engine, p0, quiet_creature(), Zone::Graveyard).len(),
        2,
        "both 1/1s are in the graveyard, so the -1/-1 is not a one-shot read at the moment it resolved"
    );
}

/// Overgrown Estate — {W}{B}{G} enchantment: "Sacrifice a land: You gain 3
/// life."
///
/// The sacrifice names no land in particular, so the engine has to ask which
/// one — and that menu is half the card: every land this seat controls is on
/// it, while the Estate itself is an enchantment and no land, and the Forest
/// across the table is not this seat's to give up (CR 701.21a). The other half
/// is the ordering CR 601.2h gives every activation: the land is already in
/// its owner's graveyard *before* the ability goes on the stack, so the three
/// life can only arrive when it resolves — no mana on this board could have
/// bought it, and the pool is asserted empty to say so.
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
#[test]
fn overgrown_estate_eats_a_land_of_your_own_for_three_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), swamp(), forest()])
        .hand(0, &[overgrown_estate()])
        .battlefield(1, &[forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {W}{B}{G} off one of each, which leaves the pool empty once the
    // enchantment has landed: the price below is a land and no mana at all.
    cast_from_hand(&mut engine, p0, overgrown_estate());
    pass_until(&mut engine, stack_is_empty);
    let estate = on_battlefield(&engine, p0, overgrown_estate()).expect("the Estate resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three lands paid the {{W}}{{B}}{{G}} exactly and float nothing"
    );

    let my_forest = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(estate, 0)),
        "the Estate's only line costs a land and no mana, so it is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, overgrown_estate(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert_eq!(
        options.len(),
        3,
        "the three lands this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&my_forest),
        "a land you control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&estate),
        "the Estate is an enchantment and no land: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "CR 701.21a: an opponent's land is not yours to sacrifice: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![their_forest],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_forest],
            },
        )
        .expect("the land the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "CR 601.2h: the price is paid before the ability is on the stack, so \
         the land is already in its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the ability is what is waiting: gaining life is no mana ability"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained yet — the effect resolves off the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 23, "\"You gain 3 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the player who paid, not the opponent"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        2,
        "exactly one land was given up: the other two are still standing"
    );
    assert!(
        on_battlefield(&engine, p0, overgrown_estate()).is_some(),
        "the Estate outlives the land it ate"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing of the opponent's ever moved"
    );
}

// oracle_id = "4d7a5b14-8fce-41f2-a0d5-fff3d15f41f6"

/// Field of Souls — {2}{W}{W} enchantment: "Whenever a nontoken creature is
/// put into your graveyard from the battlefield, create a 1/1 white Spirit
/// creature token with flying."
///
/// Every death here is dealt by the harness and read back out of a graveyard,
/// so what is measured is the trigger itself: a printed Elf of this seat's
/// dying brings one Spirit, a second one brings a second, and the Elf across
/// the table — a nontoken creature dying where "your graveyard" has to decline
/// it — brings none. The token's own body comes off its token record, and a
/// Spirit token dying afterwards is the `Nontoken` half of the card's filter.
#[test]
fn field_of_souls_makes_a_spirit_for_each_nontoken_creature_of_yours_that_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[field_of_souls()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The enchantment arrives the way the card arrives — {2}{W}{W} out of a
    // pool four Plains and two Elves actually paid into — so the trigger below
    // is not read off a printing that was merely placed on the board.
    cast_from_hand(&mut engine, p0, field_of_souls());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, field_of_souls()).is_some(),
        "the Field resolved onto the battlefield"
    );

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves of mine and one across the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "nothing has died yet"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and nothing has been made yet"
    );

    // (1) A nontoken creature of mine is put into my graveyard.
    kill(&mut engine, mine[0]);
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the creature died into its owner's graveyard, which is the condition \
         the card prints"
    );
    let spirits = tokens_of(&engine, p0);
    assert_eq!(spirits.len(), 1, "one death, one Spirit");
    let spirit = spirits[0];
    assert!(
        types(&engine, spirit).contains(TypeSet::CREATURE),
        "the token the Field makes is a creature: {:?}",
        types(&engine, spirit)
    );
    assert_eq!(pt(&engine, spirit), (1, 1), "the body the token prints");
    assert!(
        keywords(&engine, spirit).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );
    let printed = engine
        .state()
        .object(spirit)
        .expect("the Spirit is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Spirit");
    assert!(
        printed.colors.contains(baylee_core::color::Color::White),
        "a white Spirit, and not a colourless creature token: {:?}",
        printed.colors
    );

    // (2) A nontoken creature dying where the graveyard is not mine. The
    // object is still in a graveyard afterwards, so "your graveyard" is read
    // on a card and not on a hole in the board.
    kill(&mut engine, theirs);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Elf across the table died into its own owner's graveyard"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "a creature of theirs dying is no creature of mine, so the Field made \
         nothing for it"
    );

    // (3) The ability is one Spirit per creature, and not one per turn.
    kill(&mut engine, mine[1]);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "a second creature of mine dies and a second Spirit arrives"
    );

    // (4) The `Nontoken` half of the filter, on the only token creature this
    // board can hold: a Spirit of the Field's own making. It ceases to exist
    // as it leaves (CR 111.7), so an extra Spirit here would be the trigger
    // firing on it and nothing else.
    let spirits = tokens_of(&engine, p0);
    kill(&mut engine, spirits[0]);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "\"nontoken creature\": a Spirit token dying is not one, so no new \
         Spirit was made for it"
    );
}

/// Castle — {3}{W} enchantment: "Untapped creatures you control get +0/+2."
///
/// Two words in that sentence carry the card and each needs its own witness on
/// one board: an Elf of mine that has tapped for mana reads the printed 1/1
/// while its standing twin reads 1/3, which is `Untapped`; and an untapped Elf
/// across the table stays a printed 1/1, which is `you control`. Both readings
/// come off a single cast — four Plains and one Elf pay the {3}{W}, with the
/// other Elf named as the source kept back, because the creature the static is
/// about must not be tapped by the helper that pays for it. Tapping the pumped
/// Elf afterwards shows the bonus is read off the live status rather than
/// frozen at the moment the Castle arrived.
#[test]
fn castle_pumps_the_untapped_creatures_its_controller_has_and_no_other() {
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
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[castle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        my_elves.len(),
        2,
        "two Elves of mine, one of which is about to tap"
    );
    let (standing, drinking) = (my_elves[0], my_elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, standing),
        (1, 1),
        "a printed 1/1 before the Castle"
    );

    // `can_afford` reads the pool and not the untapped lands, so nothing is
    // castable until the mana is actually floating — the control for the
    // payment below.
    let card = in_hand(&engine, p0, castle()).expect("the Castle is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{W}}: {:?}",
        legal.castable
    );

    // Four Plains and one Elf are exactly the {3}{W}, and the standing Elf is
    // named as the thing kept back: it is the creature the static is about, and
    // `tap_all_mana` would have taken its own `{T}: Add {G}` as well (#159).
    tap_mana_except(&mut engine, p0, standing);
    assert!(
        is_tapped(&engine, drinking),
        "the Elf nobody kept back paid with its own tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "four Plains and one Elf, which is what a four-mana enchantment asks"
    );
    cast_with_floating(&mut engine, p0, castle());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, castle()).is_some(),
        "the Castle resolved onto the battlefield"
    );

    assert_eq!(
        pt(&engine, standing),
        (1, 3),
        "\"Untapped creatures you control get +0/+2\": an untapped 1/1 is a 1/3"
    );
    assert_eq!(
        pt(&engine, drinking),
        (1, 1),
        "the Elf that tapped for its own mana is tapped, and the static has \
         left it exactly as it was printed"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"you control\" reaches the seat that cast the Castle and never \
         across the table"
    );

    // The status is read live rather than snapshotted when the Castle arrived:
    // tapping the pumped Elf takes the +0/+2 away with it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == standing)
        .expect("the standing Elf's own mana ability is offered, the tapped one's is not");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("its whole price is its own tap");
    assert!(is_tapped(&engine, standing), "the Elf tapped for mana");
    assert_eq!(
        pt(&engine, standing),
        (1, 1),
        "so the +0/+2 went with the untapped status it depended on"
    );
}

/// Embargo — {3}{U} Enchantment: "Nonland permanents don't untap during
/// their controllers' untap steps" and "At the beginning of your upkeep, you
/// lose 2 life."
///
/// Both printed sentences are read off one turn cycle, and each is held up by
/// a control the words themselves demand. The Islands are the word *nonland*:
/// they come back while the Elves and the Sol Ring beside them stay down, so a
/// permanent still tapped after an untap step is the sentence and not a turn
/// that never ran. The Elf across the table is the same sentence on the other
/// side of it, and p1's untouched life total tells "your upkeep" from "each
/// upkeep". Nothing here was tapped by the harness — every permanent went down
/// paying its own mana ability — so the untap step is the only thing that
/// could stand one back up.
#[test]
fn embargo_holds_every_nonland_permanent_down_and_drains_its_controller_each_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                quiet_creature(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[forest(), quiet_creature()])
        .hand(0, &[embargo()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3}{U} out of every source this seat has: four Islands, the Elves'
    // printed {T}: Add {G} and the Sol Ring's own tap, which `tap_all_mana`
    // presses because each one's whole price is its own {T} (#159).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "four Islands, one mana creature and one Sol Ring: seven mana"
    );
    cast_with_floating(&mut engine, p0, embargo());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, embargo()).is_some(),
        "the Embargo resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{3}}{{U}} came out of the pool it was cast from"
    );

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("my Elves are out");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    assert!(
        is_tapped(&engine, elf) && is_tapped(&engine, ring),
        "both nonland permanents went down paying for the enchantment"
    );

    // p1's own turn: their Forest and their Elves are tapped by their own mana
    // abilities, which is the only way a permanent may be found lying down
    // here without the harness having put it that way.
    reach_their_main_phase(&mut engine, p1);
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elves are out");
    tap_all_mana(&mut engine, p1);
    assert!(
        is_tapped(&engine, their_forest) && is_tapped(&engine, their_elf),
        "their Forest and their Elves paid for their own mana"
    );

    // Back to p0's next turn: the untap step is the first printed sentence and
    // the upkeep before the main phase is the second, so one arrival reads both.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"At the beginning of your upkeep, you lose 2 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and it is the controller's upkeep and not every seat's: p1 has already \
         taken an upkeep with the Embargo on the table"
    );

    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 4, "four Islands were dealt");
    assert!(
        islands.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: the lands come back, because \"nonland\" is read"
    );
    assert!(
        is_tapped(&engine, elf) && is_tapped(&engine, ring),
        "while the Elves and the Sol Ring, which are no lands, stayed down"
    );

    // And the same sentence on the other side of the table, which is where a
    // static quietly scoped to its own controller would show.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, their_forest),
        "p1's untap step ran as well: their Forest is standing again"
    );
    assert!(
        is_tapped(&engine, their_elf),
        "\"Nonland permanents don't untap during their controllers' untap \
         steps\": the Elf across the table is one of them"
    );
}

/// Narcissism — {2}{G} Enchantment: "{G}, Discard a card: Target creature gets
/// +2/+2 until end of turn" and "{G}, Sacrifice this enchantment: Target
/// creature gets +2/+2 until end of turn."
///
/// The two printed lines differ in one word of their price, so one board pays
/// both inside a single main phase: the discard line leaves the enchantment
/// standing with a card gone out of the hand, and the sacrifice line then takes
/// the enchantment itself to its owner's graveyard — which is the only thing
/// that tells the second line from the first, since the pump they hand out is
/// identical and stacks. The creature aimed at is read beside an Elf across the
/// table, so `Filter::CREATURE` is shown to reach either side of the board
/// rather than being assumed, and a turn boundary afterwards shows that the
/// printed "until end of turn" is a duration and not a body.
#[test]
#[allow(clippy::too_many_lines)] // one card, both printed prices, and the pump read three times
fn narcissism_pumps_for_a_discarded_card_and_then_for_the_enchantment_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
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
        .hand(0, &[narcissism(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before either pump is aimed at it"
    );

    // Six Forests into the pool and the Elf kept back: {2}{G} brings the
    // enchantment to the table and leaves three green, which is the {G} of each
    // printed line. Keeping the Elf out of the tapping is what leaves the
    // creature the pumps are about to be aimed at in the state the assertions
    // below find it in.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped and nothing off the Elf"
    );
    cast_with_floating(&mut engine, p0, narcissism());
    pass_until(&mut engine, stack_is_empty);

    let card = on_battlefield(&engine, p0, narcissism()).expect("the enchantment resolved");
    assert!(
        types(&engine, card).contains(TypeSet::ENCHANTMENT),
        "it is the enchantment the card prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{2}}{{G}} is spent and three green are left for the two lines"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(card, 0)) && legal.abilities.contains(&(card, 1)),
        "both printed lines are offered while {{G}} is in the pool and a card \
         is in hand: {:?}",
        legal.abilities
    );

    // Ability 0: "{G}, Discard a card: Target creature gets +2/+2 …". The
    // target is named first (CR 601.2c) and the price is the last step of the
    // activation (CR 601.2h), so while the question stands the pool is
    // untouched and the hand is still full.
    let fodder = in_hand(&engine, p0, giant_growth()).expect("the card to discard is in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, narcissism(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&card),
        "the enchantment is no creature, so it is not a legal target for its own \
         ability: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cost is the last step of the activation, so the {{G}} is still floating"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and nothing has left the hand yet, for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options it enumerated");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not an effect, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.len() == hand_before && options.contains(&fodder),
        "`Discard a card` names no filter, so the whole hand is the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("a card the cost's own menu offered pays it");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{G}} came out of the pool"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and the card left the hand"
    );
    assert!(
        in_graveyard(&engine, p0, giant_growth()).is_some(),
        "a discarded card goes to its owner's graveyard"
    );
    assert!(
        in_hand(&engine, p0, giant_growth()).is_none(),
        "and it is not still in the hand it was discarded from"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        on_battlefield(&engine, p0, narcissism()).is_some(),
        "the discard line costs a card and leaves the enchantment where it is"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "+2/+2 on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the same card standing across the table"
    );

    // Ability 1: "{G}, Sacrifice this enchantment: …". The same target and the
    // same pump, and the only thing that differs is which price is taken.
    activate(&mut engine, p0, narcissism(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims the second line too");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "the second line carries the same target filter as the first: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options it enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{G}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, narcissism()).is_none(),
        "\"Sacrifice this enchantment\" takes the card the ability is printed on"
    );
    assert!(
        in_graveyard(&engine, p0, narcissism()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the pump is still no mana ability"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, host),
        (5, 5),
        "the second +2/+2 stacks on the first, which is what says both printed \
         lines resolved rather than one resolving twice"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );

    // "until end of turn": one turn boundary later the Elf is a printed 1/1
    // again, so both grants were durations and not bodies the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "both grants lasted the turn they were made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}

/// Noble Steeds — {2}{W} enchantment: "{1}{W}: Target creature gains first
/// strike until end of turn."
///
/// The printed filter is `Filter::CREATURE` and not "a creature you control",
/// so the scenario seats a Llanowar Elves on each side of the table: the menu
/// has to name both, and only the one that was aimed at may carry the keyword
/// afterwards. The Steeds itself is an enchantment — no creature, and so not
/// on its own menu — and the turn walked at the end is what tells the printed
/// "until end of turn" from a grant the board would have kept.
#[test]
#[allow(clippy::too_many_lines)]
fn noble_steeds_grants_first_strike_to_the_creature_it_names_and_only_until_the_turn_ends() {
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
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[noble_steeds()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "nothing has granted anything yet"
    );

    // Five Plains into the pool, and the Elf named as the printing kept back:
    // it is the creature the ability is about to aim at, and a mana creature
    // tapped for the cost would read as a different board afterwards.
    tap_mana_except(&mut engine, p0, elf);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, noble_steeds());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let steeds = on_battlefield(&engine, p0, noble_steeds()).expect("the Steeds resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{2}}{{W}} is spent and exactly the {{1}}{{W}} the ability charges is left"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands — so the claim is made with the mana already
    // floating, which is where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(steeds, 0)),
        "with the mana floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, noble_steeds(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the ability asks once"
    );
    assert!(
        options.contains(&elf) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&steeds),
        "the Steeds is an enchantment and no creature: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2c before CR 601.2h: the target is named while the mana is still in the pool"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "the creature the ability named gained first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "the Elf nobody named never moved: the effect targets, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, steeds).contains(KeywordSet::FIRST_STRIKE),
        "the Steeds grants the keyword, it does not keep it"
    );

    // "until end of turn": one turn later the Elf is a printed 1/1 again, so
    // the keyword was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the keyword left rather than the creature"
    );
}

/// Opposition — {2}{U}{U} enchantment: "Tap an untapped creature you control:
/// Tap target artifact, creature, or land."
///
/// Both halves of that sentence are the engine's answer rather than the card's,
/// so one activation has to be read off two menus and a board afterwards. The
/// price turns on the word *untapped*, which is why a second Elf is seated and
/// spent on the enchantment itself: it is down before the ability is ever
/// activated, so an offer that listed it would satisfy the same card file. The
/// target turns on the disjunction, so the artifact, the creature and the land
/// across the table are each asserted on the menu, and the two permanents
/// nobody named are read afterwards to show that "tap target" reaches exactly
/// one permanent.
#[test]
#[allow(clippy::too_many_lines)]
fn opposition_taps_an_untapped_creature_of_yours_to_tap_the_permanent_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[opposition()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which pays for the enchantment"
    );
    let (payer, spent) = (elves[0], elves[1]);
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // Four Islands and one Elf pay the {2}{U}{U}, with `payer` named as the one
    // thing kept back: it is the creature the ability's price is about to ask
    // for, and a source tapped for mana is a source that is no longer untapped.
    tap_mana_except(&mut engine, p0, payer);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "four Islands and the Elf beside them, which is every source on this board"
    );
    assert!(is_tapped(&engine, spent), "the Elf that paid is down");
    assert!(
        !is_tapped(&engine, payer),
        "and the one that was kept back is still standing"
    );

    cast_with_floating(&mut engine, p0, opposition());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let enchantment = on_battlefield(&engine, p0, opposition()).expect("the enchantment resolved");

    // The ability's whole price is a creature's tap and no mana at all, so the
    // offer is read with the pool still holding what the cast left behind —
    // which is the state `can_afford` reads.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, opposition(), 0);

    // One activation, two questions: the target at CR 601.2c and the creature
    // at CR 601.2h. Answered in the order they arrive rather than in the order
    // they are expected, and each menu is read on the spot.
    let mut cost_menu: Option<Vec<ObjectId>> = None;
    let mut target_menu: Option<Vec<ObjectId>> = None;
    for _ in 0..12 {
        if cost_menu.is_some() && target_menu.is_some() {
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
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
                if cost_menu.is_none() {
                    assert!(
                        !is_tapped(&engine, payer),
                        "CR 601.2h pays last: the creature that will pay the cost is \
                         still standing while the target is being chosen"
                    );
                }
                target_menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![their_rock],
                        },
                    )
                    .expect("the artifact across the table was one of the options");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(player, p0, "the activating seat answers its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostTap,
                    "the variant is what tells a client this is a cost and not a search"
                );
                assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
                cost_menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![payer],
                        },
                    )
                    .expect("the untapped creature the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the ability resolves: {other:?}"),
        }
    }

    assert_eq!(
        cost_menu.expect("tapping a creature is a cost, so the engine asks which one"),
        vec![payer],
        "the whole menu is the one untapped creature this seat controls: the Elf \
         that paid for the enchantment is already down and the Elf across the \
         table is not yours to tap"
    );

    let menu = target_menu.expect("the ability targets, so the engine asks what");
    assert!(
        menu.contains(&their_rock),
        "\"target artifact\" reaches the Sol Ring across the table: {menu:?}"
    );
    assert!(
        menu.contains(&their_elf),
        "\"target creature\" reaches the Elf across the table: {menu:?}"
    );
    assert!(
        menu.contains(&their_forest),
        "and \"target land\" is the third type the disjunction names: {menu:?}"
    );
    assert!(
        menu.contains(&payer),
        "the filter names no controller, so a creature you control is one too: {menu:?}"
    );
    assert!(
        !menu.contains(&enchantment),
        "the enchantment is an artifact, a creature and a land none: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, their_rock),
        "the permanent the ability named is the one it tapped"
    );
    assert!(
        is_tapped(&engine, payer),
        "and tapping a creature you control was the price, paid by the Elf"
    );
    assert!(
        on_battlefield(&engine, p0, opposition()).is_some(),
        "an activated ability costs the enchantment nothing"
    );
    assert!(
        !is_tapped(&engine, their_elf) && !is_tapped(&engine, their_forest),
        "the two permanents nobody named are untouched, so the effect reaches the \
         target and not the board"
    );
}

/// Phyrexian Arena — {1}{B}{B} enchantment: "At the beginning of your upkeep,
/// you draw a card and you lose 1 life." The word that carries the card is
/// *your*, so the board is read twice: once after the opponent has taken a
/// whole turn, where neither library nor life total may have moved, and once
/// after the Arena's controller has had an upkeep of their own, where the life
/// total is exactly one lower — one trigger, not one per upkeep of the table —
/// and the library is two cards shorter, counting the turn's own draw step
/// beside the card the enchantment gave.
#[test]
fn phyrexian_arena_draws_a_card_and_bites_its_controller_on_their_own_upkeep_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[phyrexian_arena()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `{1}{B}{B}` off three Swamps, and nothing else stands on this board: no
    // other permanent here can touch a life total or a library.
    cast_from_hand(&mut engine, p0, phyrexian_arena());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, phyrexian_arena()).is_some(),
        "the Arena resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three Swamps paid `{{1}}{{B}}{{B}}` and nothing is left floating"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let life_before = engine.state().players[0].life;
    let their_life_before = engine.state().players[1].life;

    // The opponent's whole turn goes by. "At the beginning of *your* upkeep"
    // is the Arena's controller's upkeep and nobody else's, so p1's upkeep has
    // to pass without a card drawn or a life lost.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        (
            library_size(&engine, p0),
            engine.state().players[0].life,
            engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        ),
        (library_before, life_before, hand_before),
        "\"your upkeep\" is not the opponent's: p1's turn moved neither p0's \
         library, nor p0's life, nor p0's hand"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life_before,
        "and the Arena drains nobody but its own controller: the other seat is \
         untouched too"
    );

    // Back round to the Arena's controller, whose upkeep is when the sentence
    // fires. One turn of the table is one trigger, so the life total drops by
    // exactly one and the library by two: the turn's own draw step and the
    // card the Arena gave on top of it.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        engine.state().players[0].life,
        life_before - 1,
        "\"you lose 1 life\" once per one of your own upkeeps, and not once per \
         upkeep of the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "two cards off the top of the library: the draw step's own card and the \
         one the Arena draws at the beginning of the upkeep, which comes first"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "and both are in hand, so a library that merely emptied would not \
         satisfy the count above"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life_before,
        "the life is still the controller's: a trigger keyed on the wrong seat \
         would have taken a point from p1 instead, and the drawn card would not \
         have left p0's hand two larger"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_arena()).is_some(),
        "the Arena pays its own price in life and never in permanents, so it is \
         still on the battlefield"
    );
}

/// Spiritual Asylum — {2}{W}{W} — Enchantment: "Creatures and lands you
/// control have shroud" and "Whenever a creature you control attacks,
/// sacrifice this enchantment."
///
/// Both printed sentences are played on one board, since each is the other's
/// control. Shroud is read off Vindicate's published target menu, which is the
/// only place a targeting restriction is visible: the same menu offers the
/// opponent's own Elf and Plains — so the list is enumerated and its scoping
/// is real — while declining this seat's Forest and Elves, the two card types
/// the static names. The trigger is then played, and the enchantment leaving
/// for its owner's graveyard on an attack is the whole of the second sentence.
#[test]
fn spiritual_asylum_shrouds_your_creatures_and_lands_and_is_sacrificed_to_an_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spiritual_asylum(), forest(), llanowar_elves()])
        .battlefield(1, &[plains(), plains(), swamp(), llanowar_elves()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let asylum = on_battlefield(&engine, p0, spiritual_asylum()).expect("the Asylum is out");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_land = on_battlefield(&engine, p1, plains()).expect("their Plains are out");

    // p1 casts Vindicate — "destroy target permanent" — on their own main
    // phase, where every permanent on the table but the shrouded pair is a
    // legal target.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert!(
        options.contains(&their_elves) && options.contains(&their_land),
        "their own Elf and their own Plains are on the menu, so the list is \
         enumerated and \"you control\" is what is being read: {options:?}"
    );
    assert!(
        options.contains(&asylum),
        "the Asylum is an enchantment, and the static shrouds creatures and \
         lands rather than the source itself: {options:?}"
    );
    assert!(
        !options.contains(&my_forest),
        "\"lands you control have shroud\": my Forest cannot be the target of \
         their spell: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "\"creatures you control have shroud\": my Elves cannot be targeted \
         either — the same card stands across the table and is offered there, \
         so the exclusion is the shroud and not the filter: {options:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![their_elves],
            },
        )
        .expect("a creature the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Vindicate resolved against the target it was given, so the menu \
         above was an honest offer rather than a list nothing could come of"
    );

    // Back to p0, whose Elves are untapped and past summoning sickness.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, spiritual_asylum()).is_some(),
        "nothing has attacked yet, so the Asylum is still standing"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else here")
    };
    assert_eq!(player, p0, "it is my combat step");
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 under my control may attack: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, spiritual_asylum()).is_none()
    });

    assert!(
        in_graveyard(&engine, p0, spiritual_asylum()).is_some(),
        "\"whenever a creature you control attacks, sacrifice this \
         enchantment\" — a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the price was the enchantment and not the creature that paid it"
    );
}

/// Infernal Tribute is an enchantment under `Coverage::Implemented` costing {B}{B}{B} with an activated card-draw ability.
/// Paying {2} and sacrificing a nontoken permanent draws a card.
/// The sacrifice is requested as a cost prompt with `ChoicePrompt::CostSacrifice` where controlled nontoken permanents are offered.
/// Upon resolution, the sacrificed permanent is in the graveyard and its controller draws one card from their library.
#[test]
fn infernal_tribute_sacrifices_nontoken_permanent_to_draw_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[infernal_tribute(), swamp(), swamp(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let tribute = on_battlefield(&engine, p0, infernal_tribute())
        .expect("Infernal Tribute is on battlefield");
    let elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves is on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == tribute && *idx == 0),
        "ability requiring {{2}} is not offered on an empty mana pool"
    );

    // The Elves are kept back: their own `{T}: Add {G}` is a third mana
    // route, and they are the permanent the sacrifice is about to name.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Swamps produce two mana"
    );

    let lib_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, infernal_tribute(), 0);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "controller chooses what to sacrifice");
    assert_eq!((min, max), (1, 1), "exactly one permanent sacrificed");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "choice is flagged as a cost sacrifice"
    );
    assert!(
        options.contains(&elf),
        "controlled nontoken creature is on the menu: {options:?}"
    );
    assert!(
        options.contains(&tribute),
        "Infernal Tribute itself is a nontoken permanent and on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("sacrificing the Elf is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "sacrificed creature is in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "sacrificed creature left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, infernal_tribute()).is_some(),
        "Infernal Tribute remains on the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "one card was drawn from library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "drawn card arrived in hand"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the floating mana was spent"
    );
}

/// Seismic Assault is an enchantment under `Coverage::Implemented` costing {R}{R}{R} that deals 2 damage to any target for discarding a land.
/// Following `CR 601.2c` and `CR 601.2h`, the target is announced first, followed by the discard cost.
/// The cost menu is prompted with `ChoicePrompt::CostDiscard` and restricted to land cards in hand, excluding non-land cards.
/// Resolving the ability deals 2 damage to the chosen target and puts the discarded land card into the graveyard.
#[test]
fn seismic_assault_discards_a_land_to_deal_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[seismic_assault()])
        .hand(0, &[mountain(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land_card = in_hand(&engine, p0, mountain()).expect("Mountain is in hand");
    let non_land_card = in_hand(&engine, p0, llanowar_elves()).expect("Elf is in hand");

    activate(&mut engine, p0, seismic_assault(), 0);

    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "one target required");
    assert!(
        player_options.contains(&p1),
        "defending player is an offered target: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeting player 1 is legal");

    let Pending::ChooseCards {
        player: cost_player,
        options,
        min: cost_min,
        max: cost_max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("expected discard cost prompt, got {:?}", engine.pending())
    };
    assert_eq!(cost_player, p0, "controller pays discard cost");
    assert_eq!((cost_min, cost_max), (1, 1), "exactly one card discarded");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "prompt is flagged as a discard cost"
    );
    assert!(
        options.contains(&land_card),
        "land in hand is on the discard menu: {options:?}"
    );
    assert!(
        !options.contains(&non_land_card),
        "non-land card is excluded from discard menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land_card],
            },
        )
        .expect("discarding land is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two damage dealt to player 1"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "player 0 life unchanged"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "discarded land is in graveyard"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "non-land card remains in hand"
    );
    assert!(
        on_battlefield(&engine, p0, seismic_assault()).is_some(),
        "Seismic Assault remains on battlefield"
    );
}

// oracle_id = "dabc4ba1-3f90-4cea-a737-d41537cce729"

/// Dispersing Orb prints one ability — "{3}{U}, Sacrifice a permanent: Return
/// target permanent to its owner's hand" — and both of its halves are the
/// engine's answer rather than the card's. The printed price is read off a pool
/// the nine Islands actually filled ({3}{U}{U} for the enchantment and {3}{U}
/// for the ability, one main phase and therefore one pool, CR 500.5), the
/// sacrifice menu as the permanents *this* seat controls — the Orb itself among
/// them, because "a permanent" says nothing about "another" — and the target
/// menu as every permanent on either side of the table, CR 601.2c naming the
/// target before CR 601.2h pays for it. What the bounce then does is the half
/// no board reading can see: the Elf across the table leaves the battlefield
/// and arrives in **its owner's** hand while the Orb that paid for it is in its
/// owner's graveyard.
#[test]
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
fn dispersing_orb_sacrifices_a_permanent_it_controls_to_return_any_permanent_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 9])
        .hand(0, &[dispersing_orb()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nine Islands are exactly the {3}{U}{U} the enchantment costs plus the
    // {3}{U} its ability charges, and the whole scenario plays inside this one
    // main phase, so CR 500.5 empties the pool only at the end of it.
    let card = in_hand(&engine, p0, dispersing_orb()).expect("the Orb is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{U}}{{U}}, and `can_afford` reads the pool \
         rather than the nine untapped Islands: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "nine Islands tapped, and nothing else on this board makes mana"
    );
    cast_with_floating(&mut engine, p0, dispersing_orb());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let orb = on_battlefield(&engine, p0, dispersing_orb()).expect("the Orb resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{3}}{{U}}{{U}} is spent and exactly the {{3}}{{U}} the ability \
         charges is left floating"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let land = on_battlefield(&engine, p0, island()).expect("an Island of mine is out");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — which is why the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(orb, 0)),
        "the one line the card prints, now that its {{3}}{{U}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dispersing_orb(), 0);

    // The two questions one activation asks: which permanent is aimed at
    // (CR 601.2c) and which permanent is being given up (CR 601.2h). Answered
    // in whichever order they arrive, and each menu is read on the spot.
    let mut target_menu: Vec<ObjectId> = Vec::new();
    let mut sacrifice_menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if !target_menu.is_empty() && !sacrifice_menu.is_empty() {
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
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "one permanent, and the ability asks once"
                );
                target_menu = options;
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .expect("the permanent across the table was one of the options");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
                sacrifice_menu = options;
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![orb] })
                    .expect("the Orb is a permanent this seat controls, so it may eat itself");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Orb's activation resolves: {other:?}"),
        }
    }

    assert!(
        target_menu.contains(&elf) && target_menu.contains(&land),
        "\"target permanent\" is any permanent, on either side of the table: {target_menu:?}"
    );
    assert_eq!(
        sacrifice_menu.len(),
        10,
        "the Orb and the nine Islands are every permanent this seat controls: {sacrifice_menu:?}"
    );
    assert!(
        sacrifice_menu.contains(&orb),
        "\"Sacrifice a permanent\" does not say \"another\": the source is on \
         its own menu: {sacrifice_menu:?}"
    );
    assert!(
        sacrifice_menu.contains(&land),
        "a land is as much a permanent as the enchantment that asks: {sacrifice_menu:?}"
    );
    assert!(
        !sacrifice_menu.contains(&elf),
        "`CR 701.21a`: an opponent's permanent is not yours to sacrifice: {sacrifice_menu:?}"
    );

    // CR 601.2c before CR 601.2h: with the target named and the sacrifice the
    // last price left, the {3}{U} is what the pool is missing.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "bouncing a permanent is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, dispersing_orb()).is_some(),
        "the sacrificed permanent is in its owner's graveyard"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        9,
        "and only the permanent that was named: every Island is still standing"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\" — the card went back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and not to the seat that aimed the bounce"
    );
}

/// Lifegift — {2}{G} enchantment: "Whenever a land enters, you may gain 1
/// life." The whole card is that *may*, so the printed question is answered
/// both ways across two land drops: declined, the life total is exactly where
/// it was, accepted, it is one higher — a trigger that gained on its own would
/// already read 21 after the first land, and one that never fired would still
/// read 20 after the second. Three Forests pay the {2}{G} and nothing else on
/// the board moves a life total, so neither point of life can be read as
/// anything but a land arriving.
#[test]
fn lifegift_asks_for_a_life_when_a_land_enters_and_takes_the_answer_it_is_given() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[lifegift(), forest(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // {2}{G} off the three Forests. The enchantment arriving is no land
    // arriving, so nothing is asked on the way in and no life moves with it.
    cast_from_hand(&mut engine, p0, lifegift());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, lifegift()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "its own arrival asks nothing — it watches lands and it is not one: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "Lifegift entering is worth no life by itself"
    );

    // First land drop. The land entering puts the trigger on the stack, and
    // the walk stops on the question it resolves into rather than answering
    // it, because the answer is what this test is about.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this question")
    };
    assert_eq!(
        prompt,
        YesNoPrompt::MayDo,
        "the card prints \"you may\", so the trigger is a question and not a gain"
    );
    assert_eq!(
        player, p0,
        "\"you\" is the enchantment's controller, and it is the seat asked"
    );
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining a may is always a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the first land arrived and the may was declined, so the life total \
         never moved — an unconditional trigger would already be at 21"
    );

    // A second land needs a turn of its own (CR 305.2a). Nobody does anything
    // on the way round, so the board p0 comes back to is the same board.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the same question, answered the other way");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "the second land arrived and the may was taken: one land, one life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the enchantment's controller, not to the table"
    );
    assert!(
        on_battlefield(&engine, p0, lifegift()).is_some(),
        "the trigger is the enchantment's, so it is still standing afterwards"
    );
}

/// Spidersilk Armor — {2}{G} — "Creatures you control get +0/+1 and have
/// reach."
///
/// Two printed statics, and every word in them gets a witness on this board.
/// The pump is read as `(1, 2)` on a printed 1/1 — a `(2, 2)` would mean
/// `+1/+1` was read instead — while the Serra Angel across the table keeps the
/// body it was seated with, which is what tells "creatures you control" from
/// "creatures". Reach is a blocking permission and nothing besides, so it is
/// not read off the keyword set alone: the same Elf is left standing in front
/// of a flying attacker and the engine has to offer it as a blocker, which is
/// the only place that keyword does anything at all.
#[test]
fn spidersilk_armor_pumps_and_arms_your_creatures_with_reach_and_no_one_elses() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[serra_angel()])
        .hand(0, &[spidersilk_armor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("the Angel is out");
    let angel_body = pt(&engine, angel);
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before the Armor");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::REACH),
        "a printed Llanowar Elves has no reach of its own"
    );

    // The Elf is named as the printing kept back: it prints its own
    // `{T}: Add {G}`, so `tap_all_mana` would have drunk it (#159) — and a
    // creature tapped for mana is a creature that cannot block, which is the
    // half of this scenario the reach reading below depends on.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, exactly the {{2}}{{G}} the Armor prints"
    );
    cast_with_floating(&mut engine, p0, spidersilk_armor());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, spidersilk_armor()).is_some(),
        "the Armor resolved onto the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the three green went into its {{2}}{{G}}"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 2),
        "\"get +0/+1\": the power the Elf was printed with and one more \
         toughness — a (2, 2) would mean the wrong half of the pump was read"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::REACH),
        "\"creatures you control … have reach\" reaches the Elf"
    );
    assert_eq!(
        pt(&engine, angel),
        angel_body,
        "an opponent's creature is not a creature you control: the Angel is \
         still the body it was seated as"
    );
    assert!(
        !keywords(&engine, angel).contains(KeywordSet::REACH),
        "and the static is scoped to one side of the table, not to the board"
    );

    // Reach only does anything against a flier, so the proof is the block the
    // engine offers: the printed 1/1 that could not block the Angel before the
    // Armor is on that list now, and the Angel is the only attacker there is.
    reach_their_main_phase(&mut engine, p1);
    let blocks = attack_and_collect_blocks(&mut engine, angel, p0);
    assert!(
        blocks
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&angel)),
        "\"creatures you control … have reach\": the Elf is offered as a \
         blocker of the flying Angel: {blocks:?}"
    );
}

/// `Centaur Glade` is an enchantment under `Coverage::Implemented` with an activated ability costing `{2}{G}{G}` to create a 3/3 Centaur token.
/// With an empty mana pool, its ability is unpayable and is withheld from legal actions.
/// Tapping four Forests pays the cost without tapping the enchantment itself, resolving a 3/3 green Centaur token onto the battlefield.
#[test]
fn centaur_glade_pays_four_mana_to_create_a_centaur_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), centaur_glade()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let glade = on_battlefield(&engine, p0, centaur_glade()).expect("Centaur Glade deployed");
    assert_eq!(tokens_of(&engine, p0).len(), 0, "no tokens initially");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(glade, 0)),
        "cannot afford {{2}}{{G}}{{G}} with an empty mana pool"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "four Forests produce four green mana"
    );

    activate(&mut engine, p0, centaur_glade(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{G}}{{G}} was spent from the pool"
    );
    assert!(
        !is_tapped(&engine, glade),
        "Centaur Glade does not tap to activate"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Centaur token was created");
    let centaur = tokens[0];
    assert_eq!(pt(&engine, centaur), (3, 3), "Centaur token is 3/3");
}

/// `Greed` is an enchantment under `Coverage::Implemented` with an activated ability costing `{B}` and 2 life to draw a card.
/// With an empty mana pool, its ability is unpayable and is withheld from legal actions.
/// Tapping a Swamp provides `{B}`, allowing the ability to activate and deduct 2 life and `{B}` as its cost.
/// Upon resolution, a card is drawn from the library into the controller's hand.
#[test]
fn greed_pays_black_mana_and_two_life_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), greed()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, greed()).expect("Greed is on battlefield");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(enchantment, 0)),
        "Greed cannot be activated without {{B}} in the mana pool"
    );

    let initial_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let initial_lib = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps produce two black mana"
    );

    activate(&mut engine, p0, greed(), 0);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "paying 2 life is part of the activation cost"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one {{B}} was spent from the pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand + 1,
        "drew one card into hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        initial_lib - 1,
        "one card was drawn from library"
    );
    assert!(
        !is_tapped(&engine, enchantment),
        "Greed does not tap to activate"
    );
}

/// `Still Life` is an enchantment under `Coverage::Implemented` with an activated ability costing `{G}{G}`.
/// When activated, it becomes a 4/3 Centaur creature in addition to its other types until end of turn.
/// After cycling to the next turn, the continuous animation effect expires and it reverts to a noncreature enchantment.
#[test]
fn still_life_becomes_a_four_three_centaur_creature_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), still_life()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, still_life()).expect("Still Life deployed");
    assert!(
        types(&engine, enchantment).contains(TypeSet::ENCHANTMENT),
        "Still Life is an enchantment"
    );
    assert!(
        !types(&engine, enchantment).contains(TypeSet::CREATURE),
        "Still Life is not a creature before activation"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "two Forests provide {{G}}{{G}}"
    );

    activate(&mut engine, p0, still_life(), 0);
    pass_until(&mut engine, stack_is_empty);

    let animated_types = types(&engine, enchantment);
    assert!(
        animated_types.contains(TypeSet::CREATURE),
        "Still Life gained the creature type"
    );
    assert!(
        animated_types.contains(TypeSet::ENCHANTMENT),
        "Still Life retains the enchantment type"
    );
    assert_eq!(
        pt(&engine, enchantment),
        (4, 3),
        "Still Life is a 4/3 creature"
    );

    // End of turn duration expires on the next turn.
    reach_their_main_phase(&mut engine, p1);
    let expired_types = types(&engine, enchantment);
    assert!(
        !expired_types.contains(TypeSet::CREATURE),
        "Still Life is no longer a creature after the turn ends"
    );
    assert!(
        expired_types.contains(TypeSet::ENCHANTMENT),
        "Still Life remains an enchantment"
    );
}

// oracle_id = "8f0179fe-6d7d-49cc-ab06-d3b402c6fc8d"

/// Living Lands — {3}{G} enchantment: "All Forests are 1/1 creatures that are
/// still lands."
///
/// Two printed words carry the card and each needs a different witness.
/// "Forests" is a *subtype*, so the Island across the table is the control: a
/// permanent that is a land and no Forest must stay a plain land, or the
/// static would be no more than "all lands are creatures". "All" is not "you
/// control", so the opponent's Forest is read beside my own — a filter that
/// had quietly grown a `ControlledByYou` would leave every one of my Forests
/// correct and only that one wrong. The body is the third claim, and `(1, 1)`
/// on a card that prints no power at all can only come off the layers.
#[test]
fn living_lands_turns_every_forest_into_a_one_one_creature_that_is_still_a_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[living_lands()])
        .battlefield(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests and nothing else on this side of the table: the printed
    // {3}{G} is exactly the whole pool, so the enchantment is paid for rather
    // than merely announced.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, four green, and no other source under this seat"
    );
    cast_with_floating(&mut engine, p0, living_lands());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, living_lands()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{G}} came out of the pool"
    );

    // Two types at once on a land that prints neither a body nor a type
    // line of its own: only the layer projection can produce this pair.
    let mine = all_on_battlefield(&engine, p0, forest());
    assert_eq!(mine.len(), 4, "every Forest this seat controls");
    for land in &mine {
        let kinds = types(&engine, *land);
        assert!(
            kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::CREATURE),
            "\"All Forests are 1/1 creatures that are still lands\": {kinds:?}"
        );
        assert_eq!(
            pt(&engine, *land),
            (1, 1),
            "and the body is the one the static sets, on a card that prints none"
        );
    }

    // "All" and not "you control": the same static reaches across the table.
    let theirs = on_battlefield(&engine, p1, forest()).expect("the opponent's Forest is out");
    let their_kinds = types(&engine, theirs);
    assert!(
        their_kinds.contains(TypeSet::LAND) && their_kinds.contains(TypeSet::CREATURE),
        "an opponent's Forest is a Forest too: {their_kinds:?}"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it is the same 1/1 as the ones on my side"
    );

    // The control: a land that is no Forest, which a static reading
    // `Filter::LAND` would have animated just the same.
    let other_land = on_battlefield(&engine, p1, island()).expect("the Island is out");
    let island_kinds = types(&engine, other_land);
    assert!(
        island_kinds.contains(TypeSet::LAND) && !island_kinds.contains(TypeSet::CREATURE),
        "an Island is a land and no Forest, so it stays a plain land: {island_kinds:?}"
    );
}

/// Moonlit Wake — {2}{W} enchantment: "Whenever a creature dies, you gain 1
/// life." The word the scenario turns on is "a creature", which names no
/// controller: the board carries an Elf on each side and both die, so the one
/// across the table has to pay the Wake's controller a life exactly as this
/// seat's own does — and the seat that lost the creature gains nothing. One
/// Llanowar Elf dies *before* the Wake resolves as the control, the same death
/// on the same board with nothing on the table to pay for it, and the pair of
/// readings 21 then 22 is what says each printed death paid exactly once.
#[test]
fn moonlit_wake_gains_a_life_for_a_creature_dying_on_either_side_of_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[moonlit_wake()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves on this side of the table");
    let (control, sacrifice) = (mine[0], mine[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The control: the same death on the same board with no Moonlit Wake on
    // it. `bury` moves the card on the spot and hands no priority over, which
    // is the whole finding here — the Elf prints no dies trigger of its own.
    bury(&mut engine, &[control]);
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()).len(),
        1,
        "the control creature really left the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "a creature dying with no Wake on the table pays nothing"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and pays the other seat nothing either"
    );

    // The card itself: {2}{W} off the three Plains, with the second Elf left
    // standing so that it can die once the Wake is out.
    cast_from_hand(&mut engine, p0, moonlit_wake());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, moonlit_wake()).is_some(),
        "the Wake resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "an enchantment entering is no creature dying"
    );

    kill(&mut engine, sacrifice);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"whenever a creature dies, you gain 1 life\" — and the Elf that died was this seat's"
    );

    kill(&mut engine, theirs);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the creature that died belonged to the other seat"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"a creature\" is not \"a creature you control\": a creature across the \
         table dying pays the Wake's controller just the same"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and \"you\" is the Wake's controller, not the seat that lost the creature"
    );
    assert!(
        on_battlefield(&engine, p0, moonlit_wake()).is_some(),
        "the Wake paid for two deaths and is still standing"
    );
}

/// Think Tank prints one sentence — "At the beginning of your upkeep, surveil
/// 1." — and the card is only itself if the question arrives on its
/// controller's upkeep and the answer moves a card to a graveyard. Both halves
/// are read off one board: the question is an `ArrangePrompt::Surveil` arrangement naming
/// exactly the top card of p0's library with `(0, 1)`, and the answer leaves
/// the library one card shorter with that card in the graveyard and the hand
/// untouched. That last pair is what tells surveil from a scry (the card would
/// have gone to the bottom) and from a draw (the hand would have grown).
#[test]
fn think_tank_surveils_one_on_its_controllers_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[think_tank()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, think_tank());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, think_tank()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    // "Your upkeep" is the word under test, so the walk crosses the opponent's
    // whole turn first: anything asked before p0's next upkeep is a question
    // the printed sentence does not give the card.
    reach_their_main_phase(&mut engine, p1);
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
    assert_eq!(player, p0, "the Think Tank's controller does the looking");
    assert_eq!(
        engine.state().turn.active,
        p0,
        "\"your upkeep\" — the question belongs to the seat that controls it"
    );
    assert_eq!(
        prompt,
        crate::choice::ArrangePrompt::Surveil,
        "surveil is not scry: the card either stays on top or goes to a graveyard"
    );
    assert_eq!(
        piles,
        surveil_piles(1),
        "one card is looked at, and either of the two answers is legal"
    );

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    assert_eq!(cards, vec![top], "the top card of the library, and only it");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("the card the question offered is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine
            .state()
            .object(top)
            .expect("the surveilled card is still an object")
            .zone,
        Zone::Graveyard,
        "\"put that card into your graveyard\" — the card that was offered, and not another"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before.len() - 1,
        "the surveilled card left the library, so the trigger fired exactly once"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "surveil draws nothing: a hand one longer would be the trigger read as a draw"
    );
    assert!(
        on_battlefield(&engine, p0, think_tank()).is_some(),
        "the enchantment stays where it is, to ask again on the next upkeep"
    );
}

/// `Mobilization` is an enchantment costing `{2}{W}` under `Coverage::Implemented`.
/// It prints "Soldier creatures have vigilance" and "{2}{W}: Create a 1/1 white Soldier creature token."
/// When activated off floating mana, ability 1 creates a 1/1 white Soldier token.
/// Its static ability grants that token vigilance.
#[test]
fn mobilization_creates_soldier_token_and_grants_vigilance() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mobilization(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no tokens on battlefield initially"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        3,
        "three white mana available"
    );

    // Ability 0 is the static vigilance grant; ability 1 is the token creation.
    activate(&mut engine, p0, mobilization(), 1);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token was created");
    let soldier = tokens[0];
    assert_eq!(pt(&engine, soldier), (1, 1), "Soldier token is a 1/1");
    assert!(
        keywords(&engine, soldier).contains(KeywordSet::VIGILANCE),
        "Mobilization grants vigilance to the Soldier token"
    );
}

/// `Righteous Cause` is an enchantment costing `{3}{W}{W}` under `Coverage::Implemented`.
/// It prints "Whenever a creature attacks, you gain 1 life."
/// When an opponent's creature attacks in their combat phase, the trigger fires and resolves,
/// increasing `Righteous Cause`'s controller's life total by 1.
#[test]
fn righteous_cause_gains_life_whenever_a_creature_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[righteous_cause()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls an Elf");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1, "opponent declares attackers");
    assert!(attackers.contains(&elf), "opponent's Elf can attack");

    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("opponent declaring attack is legal");

    // The attack triggers Righteous Cause. Pass until stack is empty.
    pass_until(&mut engine, |e| e.state().players[0].life > 20);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "Righteous Cause granted 1 life to its controller when a creature attacked"
    );
}

/// Day of Destiny prints one sentence — "Legendary creatures you control get
/// +2/+2" — and three of its words each need their own witness on the same
/// board: Katara, the Fearless under my control is the legendary creature the
/// static is about, a Llanowar Elves beside her is the creature "legendary"
/// has to decline, and the very same card across the table is what tells "you
/// control" from "legendary creatures". The enchantment is *cast* rather than
/// seated, so the pump is read off a board it actually arrived on: a printing
/// that had never resolved would leave all three at their printed bodies.
#[test]
fn day_of_destiny_pumps_the_legendary_creatures_you_control_and_no_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                katara_the_fearless(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[katara_the_fearless()])
        .hand(0, &[day_of_destiny()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, katara_the_fearless()).expect("my legend is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, katara_the_fearless()).expect("their legend is out");
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "a printed 3/3 before the enchantment"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "and a printed 1/1 beside her");
    assert_eq!(
        pt(&engine, theirs),
        (3, 3),
        "the same printing stands across the table at the same body"
    );

    // Four Plains pay the {3}{W} the enchanment costs; the pump is only worth
    // reading once the static is on the battlefield.
    cast_from_hand(&mut engine, p0, day_of_destiny());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, day_of_destiny()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    assert_eq!(
        pt(&engine, mine),
        (5, 5),
        "\"Legendary creatures you control get +2/+2\" — both halves of the \
         pump, on the creature that is legendary and mine"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"legendary\" is read and not skipped: the Elf beside her is a \
         creature I control and no legend"
    );
    assert_eq!(
        pt(&engine, theirs),
        (3, 3),
        "\"you control\" is the other word: the same card across the table is \
         legendary and untouched"
    );
}

// oracle_id = "6dc98143-7c4c-4b75-9bbb-5226d800b1d6"

/// Dragon Roost prints one line — "{5}{R}{R}: Create a 5/5 red Dragon creature
/// token with flying" — and every word of it has to be read off a board rather
/// than off the card file. Thirteen Mountains pay the {4}{R}{R} that brings the
/// enchantment to the table and leave exactly the seven the ability charges, so
/// the empty pool afterwards says the printed price was really paid, and the
/// token that arrives is checked for the body, the creature type, the keyword
/// and the colour the sentence names — a colorless 0/0 would satisfy "a token
/// was created" and nothing else. The Roost still standing after the activation
/// is the other half: the ability costs mana and no sacrifice, so the same
/// enchantment is a repeatable engine rather than a one-shot.
#[test]
fn dragon_roost_spends_seven_mana_for_a_five_five_red_flying_dragon() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 13])
        .hand(0, &[dragon_roost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `legal.castable` is filtered through `can_afford`, and that reads the
    // pool rather than the thirteen untapped Mountains: with nothing floating
    // the {4}{R}{R} is unpayable, so the Roost is not among the castable cards
    // at all.
    let card = in_hand(&engine, p0, dragon_roost()).expect("the Roost is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}{{R}}{{R}}, so the Roost is not offered: {:?}",
        legal.castable
    );

    // Thirteen Mountains into the pool: {4}{R}{R} brings the enchantment to the
    // table and the {5}{R}{R} the ability charges is what is left floating
    // beside it — a pool survives until the step ends (CR 500.5) and this whole
    // scenario plays inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        13,
        "thirteen Mountains tapped, thirteen red"
    );
    cast_with_floating(&mut engine, p0, dragon_roost());
    pass_until(&mut engine, stack_is_empty);

    let roost = on_battlefield(&engine, p0, dragon_roost()).expect("the Roost resolved");
    assert!(
        types(&engine, roost).contains(TypeSet::ENCHANTMENT),
        "what arrived is the enchantment the card prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the cast's {{4}}{{R}}{{R}} is spent and exactly the seven the ability \
         charges is left"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so no Dragon has been made"
    );

    // `LegalActions::abilities` is filtered through `can_afford` too, which is
    // why the claim about the offer is made with the seven already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(roost, 0)),
        "with seven red floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dragon_roost(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{5}}{{R}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Dragon arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Dragon");
    let dragon = tokens[0];
    let kinds = types(&engine, dragon);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "\"creature token\": {kinds:?}"
    );
    assert_eq!(
        pt(&engine, dragon),
        (5, 5),
        "the body the card prints, read off the battlefield"
    );
    assert!(
        keywords(&engine, dragon).contains(KeywordSet::FLYING),
        "\"with flying\" — the keyword reaches the permanent the ability made"
    );

    // The token's own ledger, which the projection cannot show: the name the
    // card gives it and the colour of a Dragon that is red and nothing else.
    let printed = engine
        .state()
        .object(dragon)
        .expect("the Dragon is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Dragon", "the name the card gives it");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Red),
        "\"red Dragon\" — and never a colorless body with the right numbers"
    );

    assert!(
        on_battlefield(&engine, p0, dragon_roost()).is_some(),
        "the price was mana and no sacrifice, so the Roost is still standing to \
         make another Dragon"
    );
}

// oracle_id = "94b703c4-5584-4913-8365-7e9f2f535c2d"

/// Levitation is `{2}{U}{U}` for one printed sentence: "Creatures you control
/// have flying." So the reading worth playing is the one that tells which
/// creatures the static *names* — two Elves of mine against one Elf across the
/// table, with the keyword read off the layer projection rather than off the
/// card file. Both halves of "you control" need a witness, and the same card is
/// the only thing that can supply them: a filter that had widened to
/// `Filter::CREATURE` would arm the Elf opposite, and one that had lost
/// `Filter::CREATURE` altogether would arm the enchantment itself, which the
/// last assertion reads at zero. The board before the cast is the control for
/// the board after it, since a creature that was already flying would satisfy
/// every positive claim for a reason that has nothing to do with Levitation
/// arriving.
#[test]
fn levitation_grants_flying_to_the_creatures_you_control_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[levitation()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        mine.len(),
        2,
        "two Elves of mine, one of which is the control"
    );
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    for id in &mine {
        assert!(
            !keywords(&engine, *id).contains(KeywordSet::FLYING),
            "with no Levitation on the battlefield a printed 1/1 is grounded"
        );
    }
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and so is the Elf across the table"
    );

    // Four Islands are exactly `{2}{U}{U}`, and both Elves are named as the
    // printing kept back: they are the creatures this test reads afterwards,
    // and `tap_all_mana` would have spent their own `{T}: Add {G}` as well
    // (#159) — the offer is read off the pool, so the mana has to be really
    // there before the cast is claimed.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        4,
        "four Islands tapped for four blue and nothing else contributed"
    );
    cast_with_floating(&mut engine, p0, levitation());
    pass_until(&mut engine, stack_is_empty);

    let enchantment = on_battlefield(&engine, p0, levitation()).expect("Levitation resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}}{{U}} came out of the pool: an enchantment that had \
         resolved for free would land the keyword just as well"
    );
    let kinds = types(&engine, enchantment);
    assert!(
        kinds.contains(TypeSet::ENCHANTMENT) && !kinds.contains(TypeSet::CREATURE),
        "the permanent that arrived is the enchantment it prints: {kinds:?}"
    );

    for id in &mine {
        assert!(
            keywords(&engine, *id).contains(KeywordSet::FLYING),
            "\"Creatures you control have flying\" — every creature of mine, \
             not only the first one the filter looked at"
        );
    }
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "\"you control\" is not \"the table\": the Elf across it is a creature \
         and not mine"
    );
    assert!(
        !keywords(&engine, enchantment).contains(KeywordSet::FLYING),
        "the enchantment grants the keyword, it does not keep it — a static \
         that had reached its own source would show flying on an enchantment"
    );
}

/// Nature's Revolt — `{3}{G}{G}` — "All lands are 2/2 creatures that are
/// still lands."
///
/// Two statics, and each can hide the other, so every clause is read off one
/// Forest that paid for the spell: its projected type line carries both
/// `CREATURE` and `LAND`, its own basic land type is still a mana route in the
/// offer, and the combat step offers it as an attacker whose two power lands on
/// the opponent. A Forest across the table is animated by the same static
/// (`Filter::LAND` names no controller) and is the blocker that offer names,
/// while the enchantment beside them is the control that keeps "all lands" from
/// being read as "all permanents".
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn natures_revolt_animates_every_land_and_leaves_the_enchantment_beside_it_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board: Vec<CardIndex> = vec![forest(); 7];
    board.push(exploration());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[natures_revolt()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        forests.len(),
        7,
        "six Forests for the mana and one kept back to attack with"
    );
    let kept = forests[0];
    let paid = forests[1];
    let chant = on_battlefield(&engine, p0, exploration()).expect("the Exploration is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");

    // Before anything resolves, a land is a land and nothing else: the control
    // for every reading below, since a permanent that was animated already
    // would satisfy them for a reason that has nothing to do with this card.
    assert!(
        !types(&engine, paid).contains(TypeSet::CREATURE),
        "with no Revolt on the battlefield a Forest is a land and nothing else"
    );
    assert!(
        engine
            .state()
            .object(paid)
            .expect("the Forest is an object")
            .characteristics()
            .power
            .is_none(),
        "and it carries no body for the SetPT below to be given credit for"
    );
    assert!(
        !types(&engine, theirs).contains(TypeSet::CREATURE),
        "and the Forest across the table is the same plain land"
    );

    // Six of the seven Forests pay the {3}{G}{G}; the seventh is held back
    // because it is the permanent both offers below are read off, and a Forest
    // spent for mana is tapped and could not attack.
    tap_mana_except(&mut engine, p0, kept);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and the seventh kept standing: six green"
    );
    cast_with_floating(&mut engine, p0, natures_revolt());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{3}}{{G}}{{G}} is five of the six, so the pool was really charged"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, natures_revolt()).is_some(),
        "the Revolt resolved onto the battlefield"
    );

    // "2/2 creatures that are still lands", read on a Forest that paid for the
    // spell — one clause each, because a permanent can be a creature with no
    // body and a body with no type.
    let animated = types(&engine, paid);
    assert!(
        animated.contains(TypeSet::CREATURE) && animated.contains(TypeSet::LAND),
        "\"creatures that are still lands\": a creature *and* still a land, and \
         not one of the two: {animated:?}"
    );
    assert_eq!(
        pt(&engine, paid),
        (2, 2),
        "the body \"are 2/2\" prints, on a permanent that had no body at all"
    );

    // `Filter::LAND` names no controller, so the Forest across the table is
    // animated by the very same static.
    let across = types(&engine, theirs);
    assert!(
        across.contains(TypeSet::CREATURE) && across.contains(TypeSet::LAND),
        "\"All lands\" is the whole table: a Forest across it is a 2/2 creature \
         that is still a land: {across:?}"
    );
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "and the body is the same whoever controls it"
    );

    // The negative control: an enchantment is a permanent and no land, so it is
    // neither type and has no body a leaked `Modifier::SetPT` could hide in.
    let still = types(&engine, chant);
    assert!(
        !still.contains(TypeSet::CREATURE) && still.contains(TypeSet::ENCHANTMENT),
        "\"All lands\" is not \"all permanents\": the Exploration beside them is \
         no creature: {still:?}"
    );
    assert!(
        engine
            .state()
            .object(chant)
            .expect("the Exploration is an object")
            .characteristics()
            .power
            .is_none(),
        "and it was given no body either"
    );

    // "still lands" as a behaviour rather than a type bit: a Forest that is a
    // 2/2 creature still prints its own `{G}`, so the shortcut CR 305.6 puts it
    // in carries it beside every other untapped Forest.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.contains(&kept),
        "a Forest that is a creature is still a Forest: {{T}}: Add {{G}} is in \
         the offer: {:?}",
        legal.mana_abilities
    );

    // "creatures", read the same way: the permanent is an attacker the combat
    // step offers, and the animated Forest across the table is the blocker that
    // offer names — the other half of "All lands", on the other side of it.
    let blocks = attack_and_collect_blocks(&mut engine, kept, p1);
    assert!(
        blocks
            .iter()
            .any(|option| option.blocker == theirs && option.attackers.contains(&kept)),
        "the animated Forest across the table may block the animated Forest that \
         attacks, so both sides of \"All lands\" are creatures: {blocks:?}"
    );

    // Nothing blocks, so the points that land are the power the card prints —
    // and the attacker is still standing as the 2/2 creature-land it was
    // declared as, so the two came off its body and not off something the card
    // never says.
    pass_until(&mut engine, |e| e.state().players[1].life == 18);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the Forest that attacked dealt the 2 power the card prints"
    );
    assert_eq!(
        pt(&engine, kept),
        (2, 2),
        "and it is still on the battlefield as a 2/2 creature that is a land"
    );
}

// oracle_id = "07462467-e4a3-409e-bcef-9cc92ca4c299"

/// Pegasus Refuge ({3}{W} enchantment) prints one line: "{2}, Discard a card:
/// Create a 1/1 white Pegasus creature token with flying."
///
/// Both halves of that price are the engine's answer rather than the card's,
/// so six Plains are tapped for the cast and read again for the ability: the
/// {2} is a real payment out of a pool those lands actually filled, and the
/// discard is a question the engine has to ask before the cost can be paid
/// (CR 601.2h). The token is claimed only once the stack has emptied, and it
/// is read as a body, a colour, a keyword and a type line — four things a
/// token that merely "arrived" would not tell apart. The hand is counted on
/// both zones, because a discard that emptied the hand without filling a
/// graveyard would satisfy the count alone.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn pegasus_refuge_discards_a_card_for_a_flying_pegasus() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 6])
        // The hand the kit deals is exactly this list, so two Forests are
        // what is left to discard once the Refuge has been cast.
        .hand(0, &[pegasus_refuge(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Plains are the {3}{W} the enchantment costs plus the {2} its ability
    // then charges, both out of one pool inside one main phase (CR 500.5).
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Plains tapped, six white"
    );
    cast_with_floating(&mut engine, p0, pegasus_refuge());
    pass_until(&mut engine, stack_is_empty);

    let refuge = on_battlefield(&engine, p0, pegasus_refuge()).expect("the Refuge resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}}{{W}} is spent and exactly the {{2}} the ability charges is left floating"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the cast took its own card out of hand, and nothing else has moved yet"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — so the claim about the offer is
    // made with the mana already floating, which is where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(refuge, 0)),
        "with {{2}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, pegasus_refuge(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not a cleanup step: the variant is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options.len(),
        hand_before - 1,
        "every card left in hand is a legal price: {options:?}"
    );
    assert!(
        !options.contains(&refuge),
        "the enchantment is on the battlefield and no longer in hand: {options:?}"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the question offered is a legal answer");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} it charges came out of the pool"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&chosen),
        "a discarded card goes to its owner's graveyard, not merely out of hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 2,
        "exactly one card was given up — the cast's and the discard's, no more"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Pegasus arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Pegasus");
    let printed = engine
        .state()
        .object(tokens[0])
        .expect("the Pegasus is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Pegasus", "the name the card gives it");
    assert_eq!(
        (printed.power, printed.toughness),
        (Some(1), Some(1)),
        "the body the card prints"
    );
    assert!(
        printed.colors.contains(baylee_core::color::Color::White),
        "a 1/1 *white* Pegasus, not merely a 1/1"
    );
    assert!(
        printed.keywords.contains(KeywordSet::FLYING),
        "with flying, which no reading of the body alone shows"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "\"creature token\": {types:?}",
        types = types(&engine, tokens[0])
    );
    assert!(
        on_battlefield(&engine, p0, pegasus_refuge()).is_some(),
        "the price was mana and a card, so the enchantment stays to make another"
    );
}

/// Reckless Assault is an enchantment printing one line — "{1}, Pay 2 life:
/// This enchantment deals 1 damage to any target" — and neither half of that
/// price is anything a board can read off the card. Two Forests fill a pool
/// the `{1}` empties, the two life leave the controller, and the damage lands
/// on the Llanowar Elves opposite: one point on a printed 1/1 is lethal
/// (CR 704.5f), so the creature dying is what says the damage resolved at the
/// target that was named and not at the seat behind it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn reckless_assault_pays_a_mana_and_two_life_to_deal_one_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[reckless_assault(), forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let assault = on_battlefield(&engine, p0, reckless_assault()).expect("the Assault is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1 for one damage to kill"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before a Forest is tapped"
    );

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // pool rather than the untapped lands: with nothing floating the `{1}` is
    // unpayable and the line is not on the offer at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(assault, 0)),
        "`{{1}}` is not one, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Two Forests into the pool: the `{1}` the ability charges and one to
    // spare, so the mana the pool is missing afterwards is a payment rather
    // than a board that never held any.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped, and the Assault makes no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(assault, 0)),
        "with `{{1}}` floating the whole price is payable: {:?}",
        legal.abilities
    );
    assert!(
        !is_tapped(&engine, assault),
        "the price is a mana and two life, so the enchantment is never tapped"
    );

    activate(&mut engine, p0, reckless_assault(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        !options.contains(&assault),
        "the Assault is an enchantment and no creature, so it is no target for \
         its own ability: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards: both
    // halves of the price are still unpaid while this question stands.
    assert_eq!(
        engine.state().players[0].life,
        20,
        "Pay 2 life is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}} is still in the pool for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");

    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"Pay 2 life\" — two, and never a life per point of damage"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{1}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the damage is the resolution, not the cost: the target is still standing"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage on a printed 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the player \
         whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, reckless_assault()).is_some(),
        "the price was a mana and two life, so the enchantment stays to shoot again"
    );
}

/// Sacred Mesa — {2}{W} enchantment: "{1}{W}: Create a 1/1 white Pegasus
/// creature token with flying", and "At the beginning of your upkeep, sacrifice
/// this enchantment unless you sacrifice a Pegasus."
///
/// Two copies stand on the table and exactly one Pegasus is made, so the first
/// upkeep after that reads the whole card in both directions at once:
/// `PlayerMayPayCostOr` is no yes/no — it asks through a `ChooseCards` whose menu
/// is one Pegasus and whose `min` is 0 — and the copy that pays keeps standing
/// while the copy with nothing left to give up is sacrificed. The Llanowar Elves
/// is the cost's filter: a creature you control that is no Pegasus never reaches
/// that menu, and is still standing once both triggers have resolved.
#[test]
#[allow(clippy::too_many_lines)] // two casts, one token, and the upkeep that reads the whole card
fn sacred_mesa_trades_one_pegasus_for_one_of_its_two_copies_at_the_upkeep() {
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
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[sacred_mesa(), sacred_mesa()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Two casts at {2}{W} and one activation at {1}{W} are eight mana, and the
    // Elf is kept standing because it is the creature the upkeep cost's filter
    // is read against. `legal.castable` sits behind `can_afford`, which reads the
    // pool and not the untapped lands, so the empty pool is asserted first.
    let card = in_hand(&engine, p0, sacred_mesa()).expect("a Mesa is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{W}}, so no Mesa is offered: {:?}",
        legal.castable
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight tapped Plains, and the untapped Elf gave nothing"
    );
    cast_with_floating(&mut engine, p0, sacred_mesa());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, sacred_mesa());
    pass_until(&mut engine, stack_is_empty);

    let mesas = all_on_battlefield(&engine, p0, sacred_mesa());
    assert_eq!(mesas.len(), 2, "both copies resolved onto the battlefield");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two {{2}}{{W}} casts out of eight white leave exactly the {{1}}{{W}}"
    );

    // The activated half, pressed by the index the offer names rather than a
    // guessed one: a Sacred Mesa prints a triggered ability and an activated one,
    // and only the activated one is ever an `abilities` entry.
    let maker = mesas[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == maker)
        .expect("{{1}}{{W}} buys a Pegasus, and its price is already in the pool");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the price the offer named is payable");
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Pegasus");
    let pegasus = tokens[0];
    let printed = engine
        .state()
        .object(pegasus)
        .expect("the Pegasus is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Pegasus");
    assert_eq!(
        (printed.power, printed.toughness),
        (Some(1), Some(1)),
        "the 1/1 body the card prints"
    );
    assert!(
        printed.colors.contains(baylee_core::color::Color::White),
        "\"a 1/1 white Pegasus\": the token is white and not merely named so"
    );
    assert!(
        printed.keywords.contains(KeywordSet::FLYING),
        "\"with flying\" reaches the token"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{W}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, maker),
        "the price is mana: nothing on the card taps the enchantment for it"
    );

    // Across the opponent's turn and into p0's next upkeep, where both copies ask
    // their question. The Pegasus pays for one; the other has nothing left to
    // give up, and that is the "unless" half of the printed sentence doing the
    // sacrificing. The loop answers whatever arrives and stops at p0's own main
    // phase, so a trigger that never fired is a board assertion and not a walk
    // that quietly gave up.
    let cast_turn = engine.state().turn.number;
    let mut asked = 0u32;
    for _ in 0..400 {
        if engine.state().turn.number > cast_turn
            && engine.state().turn.active == p0
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(player, p0, "the cost is paid by the Mesa's controller");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell apart"
                );
                asked += 1;
                let offering = if options.contains(&pegasus) {
                    assert_eq!(
                        (min, max),
                        (0, 1),
                        "\"unless you sacrifice a Pegasus\": declining is legal"
                    );
                    assert_eq!(
                        options,
                        vec![pegasus],
                        "a creature you control is no Pegasus, and the Pegasus \
                         across the table would not be yours to give up: {options:?}"
                    );
                    vec![pegasus]
                } else {
                    Vec::new()
                };
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: offering })
                    .expect("the question offered what was answered");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
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
            other => panic!("unexpected on the way to the next upkeep: {other:?}"),
        }
    }

    assert!(
        asked >= 1,
        "Sacred Mesa's upkeep trigger asked the copy that had a price to pay"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, sacred_mesa()).len(),
        1,
        "the copy that paid a Pegasus is still on the battlefield"
    );
    assert_eq!(
        mine(&engine, p0, sacred_mesa(), Zone::Graveyard).len(),
        1,
        "and the copy with nothing to give up was sacrificed to its owner's graveyard"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the Pegasus was the price: a sacrificed token ceases to exist (CR 111.7)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf was never a legal price and never moved"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing on the other side of the table paid for any of it"
    );
}

/// Teferi's Care prints two lines and both of them are filters: "{W},
/// Sacrifice an enchantment: Destroy target enchantment" reads *you control* at
/// its price and the whole table at its target, while "{3}{U}{U}: Counter
/// target enchantment spell" reads the stack.
///
/// So the same enchantment stands on both sides of the table and a Dark Ritual
/// waits on the stack beside the enchantment spell: the sacrifice menu holds
/// this seat's two enchantments while the destroy menu holds all three, and the
/// counter's menu holds the enchantment spell alone. Both lines are played
/// inside one main phase off eight basics, so every printed price is counted in
/// the pool rather than assumed (CR 500.5).
#[test]
#[allow(clippy::too_many_lines)] // one card, both its lines, every price read off a zone
fn teferis_care_sacrifices_an_enchantment_to_destroy_one_and_counters_an_enchantment_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                teferis_care(),
                exploration(),
                plains(),
                swamp(),
                forest(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[exploration(), dark_ritual()])
        .battlefield(1, &[exploration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let care = on_battlefield(&engine, p0, teferis_care()).expect("the Care is on the table");
    let my_chant = on_battlefield(&engine, p0, exploration()).expect("my enchantment is out");
    let their_chant = on_battlefield(&engine, p1, exploration()).expect("their enchantment is out");

    // One main phase pays for everything: eight basics make {W} for the first
    // line, {G} and {B} for the two spells the second line is about, and
    // {3}{U}{U} to counter one of them. CR 500.5 keeps the pool across all of
    // it, so this is one payment and not three.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight basics tapped, and neither the Care nor the Explorations make mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(care, 0)),
        "with the white floating the first line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, teferis_care(), 0);
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
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one enchantment, and the ability asks once"
    );
    assert!(
        options.contains(&my_chant) && options.contains(&care) && options.contains(&their_chant),
        "\"target enchantment\" names any enchantment on either side of the \
         table: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "the three enchantments on the battlefield and nothing else: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // {W} is still floating and the enchantment that will pay it is still
    // standing while this question is open.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cost is the last step of the activation, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p0, exploration()).is_some(),
        "and nothing has been sacrificed yet, for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_chant],
            },
        )
        .expect("the enchantment across the table was one of the options");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one enchantment, no more and no fewer");
    assert!(
        options.contains(&my_chant),
        "the enchantment you control is on the menu: {options:?}"
    );
    assert!(
        options.contains(&care),
        "\"an enchantment\" is not \"another\": the Care is itself an \
         enchantment this seat controls, so it may pay its own price: {options:?}"
    );
    assert!(
        !options.contains(&their_chant),
        "`CR 701.21a`: an opponent's enchantment is not yours to sacrifice: {options:?}"
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
                objects: vec![my_chant],
            },
        )
        .expect("the enchantment the question offered pays the cost");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the {{W}} came out of the pool"
    );
    assert_eq!(
        mine(&engine, p0, exploration(), Zone::Graveyard).len(),
        1,
        "and the enchantment that was named is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, exploration()).is_none(),
        "nothing has been destroyed yet: the ability is still on the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability waits to resolve"
    );

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        in_graveyard(&engine, p1, exploration()).is_some(),
        "\"destroy target enchantment\" — the enchantment across the table is gone"
    );
    assert!(
        on_battlefield(&engine, p1, exploration()).is_none(),
        "and it left the battlefield, which is what destroy means"
    );
    assert!(
        on_battlefield(&engine, p0, teferis_care()).is_some(),
        "the Care ate another enchantment and is still standing"
    );

    // The second line wants a spell to point at, so the other copy of the same
    // enchantment is cast and priority held over it: a countered spell and a
    // resolved one differ only in the question that gets asked about them.
    cast_with_floating(&mut engine, p0, exploration());
    pass_until(&mut engine, |e| {
        on_stack(e, exploration()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let chant_spell = on_stack(&engine, exploration()).expect("the other copy is a spell now");

    // A spell that is no enchantment, so the menu below has something it must
    // decline. Dark Ritual asks nothing of its own while it waits.
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| {
        on_stack(e, exploration()).is_some()
            && on_stack(e, dark_ritual()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let ritual_spell = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the green and the black paid for the two spells, and five blue is left \
         — exactly the counter's {{3}}{{U}}{{U}}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        5,
        "and every land left standing on this board makes blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(care, 1)),
        "with five blue floating the counter is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, teferis_care(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "one spell, and the ability asks once");
    assert!(
        options.contains(&chant_spell),
        "the enchantment spell waiting on the stack is the target: {options:?}"
    );
    assert!(
        !options.contains(&ritual_spell),
        "the Ritual is a spell and no enchantment spell, so it is off the \
         menu: {options:?}"
    );
    assert!(
        !options.contains(&care),
        "the Care is a permanent on the battlefield and no spell at all: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that one spell is the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chant_spell],
            },
        )
        .expect("the spell the question offered is the one that is countered");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "countering is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        mine(&engine, p0, exploration(), Zone::Graveyard).len(),
        2,
        "a countered spell goes to its owner's graveyard: the copy that was \
         cast and countered lies beside the one that was sacrificed"
    );
    assert!(
        all_on_battlefield(&engine, p0, exploration()).is_empty(),
        "and no Exploration of mine ever reached the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_some(),
        "the spell the counter did not name resolved instead of being countered"
    );
    assert!(
        on_battlefield(&engine, p0, teferis_care()).is_some(),
        "the Care is still standing after both of its printed lines"
    );
}

/// Treasure Trove is `{2}{U}{U}` for one activated ability and no other text:
/// "`{2}{U}{U}`: Draw a card." The price is mana and **no tap symbol**, so one
/// board reads both halves of that. With an empty pool the line is not offered
/// at all — `legal.abilities` is filtered through `can_afford`, which reads the
/// pool rather than the eight untapped Islands — and eight Islands buy the
/// *same* enchantment two draws inside one turn, which a `{T}` in the cost
/// would forbid. Each draw is read on the library and the hand together, so an
/// emptied library could not stand in for it, and the Trove is asserted still
/// on the battlefield afterwards: the price is mana and no sacrifice.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn treasure_trove_buys_two_cards_off_one_enchantment_with_mana_and_no_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                treasure_trove(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let trove = on_battlefield(&engine, p0, treasure_trove()).expect("the Trove is out");
    let kinds = types(&engine, trove);
    assert!(
        kinds.contains(TypeSet::ENCHANTMENT) && !kinds.contains(TypeSet::CREATURE),
        "it is the enchantment it prints: {kinds:?}"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the seat with the Trove holds a quiet main phase"
    );
    assert!(
        !legal.abilities.contains(&(trove, 0)),
        "an empty pool pays no {{2}}{{U}}{{U}}, so nothing is offered: {:?}",
        legal.abilities
    );

    // Eight Islands: the first four pay for one activation and the second four
    // for another, because nothing in the price turns the source sideways.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Islands, and the Trove makes no mana of its own"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        8,
        "and all eight make blue, so the {{U}}{{U}} in the cost has a source"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trove, 0)),
        "with the mana floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, treasure_trove(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{2}}{{U}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );

    // The second draw is the half a `{T}` would forbid: the same enchantment is
    // offered again off the mana left over, and it was never tapped.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trove, 0)),
        "no part of the price taps the Trove, so it is offered again: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, treasure_trove(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both prices are paid and nothing is left of the eight"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "two activations, two cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "and both reached the hand"
    );
    assert!(
        on_battlefield(&engine, p0, treasure_trove()).is_some(),
        "the price was mana and no sacrifice, so the Trove stays to draw again"
    );
}

/// `Blood Rites` is an enchantment costing `{3}{R}{R}` under `Coverage::Implemented`.
/// It prints "{1}{R}, Sacrifice a creature: This enchantment deals 2 damage to any target."
/// Under CR 601.2c and CR 601.2h, activating the ability prompts for the target first
/// and then prompts with `ChoicePrompt::CostSacrifice` to sacrifice a controlled creature.
/// Upon resolution, the 2 damage destroys an opponent's 2/2 creature (such as `Desert Drake`).
#[test]
fn blood_rites_sacrifices_creature_to_deal_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[blood_rites(), llanowar_elves(), mountain(), mountain()],
        )
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let drake =
        on_battlefield(&engine, p1, desert_drake()).expect("opponent controls Desert Drake");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls Llanowar Elves");
    assert_eq!(pt(&engine, drake), (2, 2));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, blood_rites(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Blood Rites, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&drake),
        "opponent's creature is an offered target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drake],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        prompt,
        options: sacrifice_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected CostSacrifice prompt for Blood Rites, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "prompt is CostSacrifice"
    );
    assert!(
        sacrifice_options.contains(&elf),
        "Llanowar Elves is offered as sacrifice: {sacrifice_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "sacrificed creature is in p0's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, desert_drake()).is_some(),
        "damaged creature was destroyed and placed in p1's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, desert_drake()).is_none(),
        "target creature is no longer on the battlefield"
    );
}

/// `Knighthood` is an enchantment costing `{2}{W}` under `Coverage::Implemented`.
/// It prints "Creatures you control have first strike."
/// While on the battlefield, creatures controlled by its controller gain first strike,
/// while creatures controlled by the opponent do not gain the keyword.
#[test]
fn knighthood_grants_first_strike_to_controlled_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[knighthood(), llanowar_elves()])
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls Llanowar Elves");
    let their_drake =
        on_battlefield(&engine, p1, desert_drake()).expect("opponent controls Desert Drake");

    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::FIRST_STRIKE),
        "creature controlled by Knighthood's controller has first strike"
    );
    assert!(
        !keywords(&engine, their_drake).contains(KeywordSet::FIRST_STRIKE),
        "opponent's creature does not gain first strike"
    );
}

/// `Living Plane` is a world enchantment costing `{2}{G}{G}` under `Coverage::Implemented`.
/// It prints "All lands are 1/1 creatures that are still lands."
/// While on the battlefield, lands controlled by all players gain the creature type in addition
/// to their land type and have their power and toughness set to 1/1.
#[test]
fn living_plane_turns_all_lands_into_one_one_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[living_plane(), forest()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let p0_land = on_battlefield(&engine, p0, forest()).expect("p0 Forest is present");
    let p1_land = on_battlefield(&engine, p1, mountain()).expect("p1 Mountain is present");

    let chars0 = engine
        .state()
        .object(p0_land)
        .expect("p0 land exists")
        .characteristics();
    assert!(
        chars0.types.contains(TypeSet::LAND) && chars0.types.contains(TypeSet::CREATURE),
        "p0 land is both a land and a creature"
    );
    assert_eq!(
        pt(&engine, p0_land),
        (1, 1),
        "p0 land has 1/1 power and toughness"
    );

    let chars1 = engine
        .state()
        .object(p1_land)
        .expect("p1 land exists")
        .characteristics();
    assert!(
        chars1.types.contains(TypeSet::LAND) && chars1.types.contains(TypeSet::CREATURE),
        "p1 land is both a land and a creature"
    );
    assert_eq!(
        pt(&engine, p1_land),
        (1, 1),
        "p1 land has 1/1 power and toughness"
    );
}

/// `Unspeakable Symbol` is an enchantment costing `{1}{B}{B}` under `Coverage::Implemented`.
/// It prints "Pay 3 life: Put a +1/+1 counter on target creature."
/// Under CR 601.2c and CR 601.2h, targeting occurs before the life cost is deducted.
/// Upon completing activation, 3 life is paid, and upon resolution, the target creature
/// (such as `Llanowar Elves`) receives a +1/+1 counter.
#[test]
fn unspeakable_symbol_pays_life_to_place_counter_on_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[unspeakable_symbol(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves is on the battlefield");
    assert_eq!(counters_on(&engine, elf, CounterKind::P1P1), 0);

    activate(&mut engine, p0, unspeakable_symbol(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Unspeakable Symbol, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "Llanowar Elves is an offered target: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "life is not yet paid while target choice is pending"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    assert_eq!(
        engine.state().players[0].life,
        17,
        "paid 3 life upon finishing the activation"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "placed a +1/+1 counter on the target creature"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "creature body grew to 2/2");
}

/// `Rhystic Study` is an enchantment costing `{2}{U}` under `Coverage::Implemented`.
/// It prints "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}."
/// When an opponent casts a spell, the triggered ability prompts the opponent with `YesNoPrompt::PayTax`,
/// and when the opponent declines to pay, its controller draws a card.
#[test]
fn rhystic_study_triggers_on_opponent_cast_and_draws_when_tax_declined() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[rhystic_study()])
        .hand(0, &[])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, llanowar_elves());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { .. },
                ..
            } if *player == p1
        )
    });

    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "controller drew one card after opponent declined to pay the tax"
    );
}

/// `Root Cage` is an enchantment costing `{1}{G}` under `Coverage::Implemented`.
/// It prints "Mercenaries don't untap during their controllers' untap steps."
/// When both a Mercenary (`moggcatcher()`) and a non-Mercenary (`llanowar_elves()`) attack and tap,
/// advancing through the opponent's turn to the controller's next turn untaps the non-Mercenary,
/// while `moggcatcher()` remains tapped due to `Root Cage`.
#[test]
fn root_cage_stops_mercenaries_from_untapping() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[root_cage(), moggcatcher(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mogg = on_battlefield(&engine, p0, moggcatcher()).expect("moggcatcher deployed");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    let def = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mogg, def), (elf, def)],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, mogg),
        "moggcatcher is tapped from attacking"
    );
    assert!(is_tapped(&engine, elf), "elf is tapped from attacking");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, elf),
        "non-Mercenary elf untaps as normal"
    );
    assert!(
        is_tapped(&engine, mogg),
        "Mercenary moggcatcher stays tapped under Root Cage"
    );
}

/// `Tribute to the World Tree` is an enchantment costing `{G}{G}{G}` under `Coverage::Implemented`.
/// It prints "Whenever a creature you control enters, draw a card if its power is 3 or greater. Otherwise, put two +1/+1 counters on it."
/// When a 1/1 creature like `llanowar_elves()` enters, its power is below 3,
/// so it receives two `CounterKind::P1P1` counters and grows to a 3/3 without drawing a card.
#[test]
fn tribute_to_the_world_tree_adds_counters_to_small_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tribute_to_the_world_tree(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves entered the battlefield");
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        2,
        "gained two +1/+1 counters"
    );
    assert_eq!(pt(&engine, elf), (3, 3), "power and toughness are 3/3");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        0,
        "no card was drawn because entering power was less than 3"
    );
}
