//! The copy that does not last: Cursed Mirror's, which is a `Layer::Copy` effect with `Duration::UntilEndOfTurn`, and Glasspool Mimic's, which ends when the object does at a zone change (CR 400.7). What such a copy may do while it lasts, what has to be handed back when it stops — the rules text, which is not layer-projected and so was written onto the object, and the effect-table entries registered from it — what a copy *of* one comes down as, and which values it took in the first place, since a +1/+1 counter is not copiable (CR 707.2). The ordering question lives here too, because only these paths raise it: whether what a copy's arrival registers is in place before the trigger that same arrival caused is collected. An ability already on the stack is its own object and outlives all of it (CR 113.7a); a copy that simply stands there carrying what it copied is `copied_abilities`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Three cards, and the question the first two only half asked: a Glasspool
/// Mimic copying a Snapcaster Mage is a *Wizard*, which is what Riptide
/// Laboratory returns — and what comes back to the hand has to be a Mimic
/// again.
///
/// A copy lasts exactly as long as the object does (CR 707.2a); the object
/// ends at the zone change (CR 400.7). Left as a copy, the card in hand
/// would be a Snapcaster Mage that costs `{2}{U}`, and recasting it would
/// find no enters-as-a-copy ability at all — a 0/0 that dies on arrival.
/// That is the cost of storing copied abilities on the object, so this is
/// the test that says the storing is paid for.
#[test]
fn a_mimic_that_copied_a_wizard_comes_home_a_mimic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(74, forest())
        .battlefield(
            0,
            &[
                riptide_laboratory(),
                island(),
                island(),
                island(),
                island(),
                island(),
                snapcaster_mage(),
            ],
        )
        .hand(0, &[glasspool_mimic()])
        .battlefield(1, &[snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, riptide_laboratory()).expect("the laboratory is out");
    let mine = on_battlefield(&engine, p0, snapcaster_mage()).expect("my wizard");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their wizard");
    tap_mana_except(&mut engine, p0, lab);

    let mimic = in_hand(&engine, p0, glasspool_mimic()).expect("mimic in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mimic })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = target_options(&engine);
    assert!(
        !options.contains(&theirs),
        "the Mimic copies a creature *you* control: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| pt(e, mimic) == (2, 1));

    let index = offered_ability(&engine, lab).expect("a copy of a Wizard is a Wizard");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: lab,
                ability_index: index,
            },
        )
        .unwrap();
    let options = target_options(&engine);
    assert!(
        options.contains(&mimic),
        "the copy is a Wizard the Laboratory can save: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" still does not reach across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mimic],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| in_hand(e, p0, glasspool_mimic()).is_some());

    let back = in_hand(&engine, p0, glasspool_mimic()).expect("the copy came home");
    assert_eq!(back, mimic, "the same handle, one zone later");
    let obj = engine.state().object(back).expect("it is in the hand");
    assert_eq!(
        (obj.base.power.unwrap_or(0), obj.base.toughness.unwrap_or(0)),
        (0, 0),
        "Glasspool Mimic is printed 0/0; the Snapcaster it copied was 2/1"
    );
    assert!(
        obj.original_base.is_none(),
        "the pre-copy base was spent, not kept for a second zone change"
    );
    assert!(
        matches!(
            obj.abilities(&RegistryLookup).first(),
            Some(AbilityDef::CopyOnEnter { .. })
        ),
        "cast again it must still be able to enter as a copy"
    );
}

/// The same pair from the other side: an ability outlives the copy that
/// activated it.
///
/// Werefox Bodyguard's second ability sacrifices itself as part of its cost
/// (`{1}{W}, Sacrifice this creature: You gain 2 life`), so a Glasspool Mimic
/// copying it is in the graveyard *before* the ability resolves — and stops
/// being a copy on the way (CR 400.7). The ability does not care: it has been
/// its own object on the stack since it was activated (CR 113.7a). Read back
/// off the source at resolution time it would find Glasspool Mimic's own
/// one-entry list and index 1 in it, which is nothing at all.
#[test]
fn an_ability_the_copy_paid_for_with_its_life_still_resolves() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(75, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                plains(),
                plains(),
                werefox_bodyguard(),
            ],
        )
        .hand(0, &[glasspool_mimic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fox = on_battlefield(&engine, p0, werefox_bodyguard()).expect("the bodyguard is out");
    let life = engine.state().players[0].life;
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }

    let mimic = in_hand(&engine, p0, glasspool_mimic()).expect("mimic in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mimic })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![fox] })
        .unwrap();
    pass_until(&mut engine, |e| pt(e, mimic) == (2, 2));
    // The copied enters-trigger is "exile up to one **other** target non-Fox
    // creature", and both creatures at the table are the same Fox: it has
    // nothing to point at, so it goes on the stack with no targets and
    // nobody is asked. This used to answer an empty choice by hand —
    // `up_to_one_target_with_nothing_to_point_at_is_not_a_question` is why
    // the choice no longer arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    // No tap in the cost, so the copy may do this the turn it arrives.
    let index = offered_ability(&engine, mimic).expect("the copy carries what it copied");
    assert_eq!(index, 1, "the sacrifice ability, not the enters-trigger");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mimic,
                ability_index: index,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[0].life != life);

    assert_eq!(
        engine.state().players[0].life,
        life + 2,
        "the ability resolved from the graveyard side of its own cost"
    );
    let obj = engine.state().object(mimic).expect("the copy is in a zone");
    assert_eq!(obj.zone, crate::zone::Zone::Graveyard, "sacrificed");
    assert_eq!(
        (obj.base.power.unwrap_or(0), obj.base.toughness.unwrap_or(0)),
        (0, 0),
        "the card in the graveyard is Glasspool Mimic, not the 2/2 it copied"
    );
}

/// A copy takes the abilities with everything else (CR 707.2). The Mirror
/// that became a Llanowar Elf taps for `{G}`, and the `{T}: Add {R}` it was
/// printed with is not among the things it can do — one activation reads
/// both halves of that sentence at once, because the answer is a colour and
/// there are only two candidates.
#[test]
fn a_mirror_that_became_an_elf_taps_for_what_the_elf_taps_for() {
    let p0 = PlayerId::new(0);
    let (mut engine, mirror) = a_mirror_that_became_their_elf(100);
    let index = the_mirrors_ability(&engine, mirror);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mirror,
                ability_index: index,
            },
        )
        .expect("an offered ability is activatable");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        1,
        "the Elf's mana ability came with the rest of the Elf"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Red),
        0,
        "and the Mirror's own {{T}}: Add {{R}} is not one of the things a \
         copy of an Elf can do"
    );
}

/// The other end of the same sentence — "until end of turn" — and the half
/// that has to be undone by hand.
///
/// The body goes back on its own: it is a `Layer::Copy` effect with
/// `Duration::UntilEndOfTurn`, and the cleanup step drops it. The rules text
/// cannot, because abilities are not layer-projected — `Characteristics` has
/// no field for them, so the copy wrote them onto the object and something
/// has to take them back. A Mirror still tapping for `{G}` on the next turn
/// would be a copy the rules had ended and the engine had not.
///
/// Both halves are asked here, and in that order: an Elf's body would make
/// the second answer meaningless, because a permanent that never stopped
/// being a copy is *supposed* to tap for green.
#[test]
fn the_mirror_stops_being_an_elf_when_the_turn_does() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, mirror) = a_mirror_that_became_their_elf(101);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let types = engine
        .state()
        .object(mirror)
        .expect("the Mirror is still on the battlefield")
        .characteristics()
        .types;
    assert!(
        types.intersects(TypeSet::ARTIFACT) && !types.intersects(TypeSet::CREATURE),
        "the body reverted with the effect that carried it: {types:?}"
    );

    let index = the_mirrors_ability(&engine, mirror);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mirror,
                ability_index: index,
            },
        )
        .expect("an offered ability is activatable");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Red),
        1,
        "the Mirror has its own {{T}}: Add {{R}} back"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        0,
        "and not the Elf's, which it stopped having at the cleanup step"
    );
}

/// The third thing a copy leaves behind, and the one that is not on the
/// object at all: an entry in the effect table.
///
/// `sync_static_effects` registers a permanent's statics with
/// `Duration::WhileSourceOnBattlefield` and drops them when the source
/// leaves the battlefield — and a copy ending is not a departure, so nothing
/// was dropping them. Taking the rules text back is therefore only half a
/// revert: what was registered *from* that text has to go with it, and the
/// next pass registers the printed abilities in its place.
///
/// Karmic Guide is the same instrument the Glasspool Mimic measurement used,
/// read from the other end: `Filter::This` protection from black, which is
/// the one copyable static in this pool that says something about the
/// permanent that has it. Vindicate ("Destroy target permanent") is white
/// **and** black, so protection from black would keep it off the Mirror
/// (CR 702.16b) — and by seat 1's main phase the Mirror is an artifact that
/// never had any.
#[test]
fn the_mirror_gives_back_the_protection_it_borrowed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[cursed_mirror()])
        .battlefield(1, &[plains(), swamp(), forest(), karmic_guide()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Their Angel, because the clause reaches across the table and a Guide
    // of my own would have to be cast: it has echo {3}{W}{W}, and a seated
    // one is asked for it at the first upkeep `reach_main_phase` walks
    // through. Seat 1 is asked at *their* upkeep, which is after the copy
    // has already been made and reverted.
    let guide = on_battlefield(&engine, p1, karmic_guide()).expect("their Angel");

    cast_from_hand(&mut engine, p0, cursed_mirror());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![guide],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let mirror = on_battlefield(&engine, p0, cursed_mirror()).expect("the Mirror arrived");

    reach_their_main_phase(&mut engine, p1);
    let types = engine
        .state()
        .object(mirror)
        .expect("the Mirror is still on the battlefield")
        .characteristics()
        .types;
    assert!(
        !types.intersects(TypeSet::CREATURE),
        "a Mirror that was still an Angel would be a legal target for the \
         wrong reason: {types:?}"
    );

    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = target_options(&engine);
    assert!(
        options.contains(&mirror),
        "the protection was the Angel's, and the Mirror stopped being one \
         at the cleanup step: {options:?}"
    );
}

/// The ordering question the other two copy paths do not raise: is a copy's
/// replacement rule registered before the trigger its own arrival caused is
/// collected?
///
/// A token copy is handed its rules text by `settle_copied_rules_text` at
/// step 0 of the pass, ahead of everything, which is what makes the token
/// answer ([`super::copied_abilities::the_copy_registers_the_replacement_rule_it_copied`]) say
/// nothing about ordering.
///
/// A **cast** copy no longer raises it either, and that is a change rather
/// than a fact about this test. `check_copy_on_enter` ran at 0b, one step
/// *after* the scan that registers what a permanent can do, and what saved
/// it was that it asks the controller a question: the machine returns on
/// `awaiting_answer`, and the pass that applies the answer starts again at
/// step 0, so the rule was registered before step 3 collected a trigger.
/// CR 614.12a moved the question in front of the arrival, so the Mirror
/// below is already a copy on the pass it enters on and the scan at 0a is
/// the first thing to see it. The saving grace is gone because what it was
/// saving is gone; what this test now holds is the *outcome* either
/// ordering owes, which is why it is worth keeping unchanged.
///
/// Cursed Mirror rather than a Glasspool Mimic, because a Mimic may copy
/// only a creature you control and would need a Katara of mine standing
/// there multiplying the same trigger — two multipliers and no way to say
/// which one did it. "Any creature on the battlefield" reaches theirs, so
/// the only multiplier on my side of the table is the one the Mirror has
/// just become.
#[test]
fn the_mirror_multiplies_the_trigger_its_own_arrival_caused() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(103, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                earth_king_s_lieutenant(),
            ],
        )
        .hand(0, &[cursed_mirror()])
        .battlefield(1, &[katara_the_fearless(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        plus_one_counters(&engine, p0, earth_king_s_lieutenant()),
        0,
        "nothing has entered yet"
    );
    let katara = on_battlefield(&engine, p1, katara_the_fearless()).expect("their Katara");

    cast_from_hand(&mut engine, p0, cursed_mirror());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![katara],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        plus_one_counters(&engine, p0, earth_king_s_lieutenant()),
        2,
        "my Lieutenant's rally fired for the Ally that entered, and once \
         more because by the time the trigger was collected that Ally was a \
         Katara"
    );
    assert_eq!(
        engine.state().object(katara).map(|o| o.controller),
        Some(p1),
        "and the Katara it copied is still theirs"
    );
}

/// A copy of a copy, where the first copy is the temporary kind.
///
/// The Mimic's own ruling: "If the chosen creature is copying something
/// else, then Glasspool Mimic enters the battlefield as whatever the chosen
/// creature copied." A Mirror that has become a Llanowar Elf is a creature
/// you control, so its controller's Mimic is offered it — and what the
/// Mimic must come down as is an Elf, not an artifact named Cursed Mirror.
///
/// This is the question [`layers::copiable_values`] answers (CR 707.2), and
/// `apply_copy_choice` had no such thing: the permanent branch took the
/// target's `base`, which is layer 0 and knows nothing about a copy that
/// lives in the effect table, while the abilities beside it came through
/// [`GameObject::abilities`], which follows `own_abilities` and so does
/// know. Two halves of one object, read from two different places — and
/// they disagree only when the target is a *temporary* copy, which is why
/// nothing noticed until Cursed Mirror started carrying its rules text.
///
/// [`layers::copiable_values`]: crate::layers::copiable_values
/// [`GameObject::abilities`]: crate::object::GameObject::abilities
#[test]
fn a_mimic_copying_a_mirror_that_became_an_elf_comes_down_an_elf() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(104, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[cursed_mirror(), glasspool_mimic()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");

    // The Mirror's {2}{R} out of the Mountains and the Mimic's {2}{U} out
    // of the Islands, in one main phase, because the Mirror stops being an
    // Elf at this turn's cleanup and there is no later one to ask in.
    tap_only(&mut engine, p0, mountain());
    let mirror_card = in_hand(&engine, p0, cursed_mirror()).expect("the Mirror is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mirror_card })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let mirror = on_battlefield(&engine, p0, cursed_mirror()).expect("the Mirror arrived");

    tap_only(&mut engine, p0, island());
    let mimic_card = in_hand(&engine, p0, glasspool_mimic()).expect("the Mimic is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mimic_card })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert!(
        target_options(&engine).contains(&mirror),
        "the Mirror is a creature I control for as long as it is an Elf"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mirror],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let copy = on_battlefield(&engine, p0, glasspool_mimic()).expect("the Mimic arrived");
    let types = engine
        .state()
        .object(copy)
        .expect("a permanent on the battlefield is an object")
        .characteristics()
        .types;
    assert!(
        types.intersects(TypeSet::CREATURE),
        "what the chosen creature copied was a creature: {types:?}"
    );
    assert_eq!(
        pt(&engine, copy),
        (1, 1),
        "and the body it came down with is the Elf's"
    );

    // And it stays one after the Mirror stops being an Elf, which is the
    // other half of the same ruling: the Mimic's copy is the permanent
    // kind and was *written* to its base, not re-derived on demand from a
    // Mirror that has since gone back to being an artifact.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(copy)
            .expect("still standing")
            .characteristics()
            .types
            .intersects(TypeSet::CREATURE),
        "a copy lasts as long as the object does (CR 707.2a)"
    );
    assert_eq!(pt(&engine, copy), (1, 1), "and keeps the body with it");
}

/// A copy takes the printed body, not the one the counters made.
///
/// A +1/+1 counter is not a copiable value (CR 707.2): it is applied in
/// layer 7d, long after the copy effect in layer 1, and a copy of a
/// creature bearing one arrives without it. The Mirror's other half is the
/// one that could get this wrong — `Modifier::BecomeCopyOf` assigns the
/// target's `characteristics()`, which is every layer projected and not the
/// copiable values alone, so what it takes depends on what the projection
/// has already reached rather than on the rules.
///
/// The Ally is chosen for how quietly it can be given a counter: Earth
/// King's Lieutenant counts another Ally entering, and Harabaz Druid is the
/// one in the pool that enters without a rally trigger of its own to answer.
#[test]
fn a_mirror_copying_a_countered_creature_leaves_the_counter_behind() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(105, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[cursed_mirror()])
        .battlefield(1, &[earth_king_s_lieutenant(), forest(), forest()])
        .hand(1, &[harabaz_druid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, harabaz_druid());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, harabaz_druid()).is_some() && stack_is_empty(e)
    });
    let ekl = on_battlefield(&engine, p1, earth_king_s_lieutenant()).expect("their Lieutenant");
    assert_eq!(
        plus_one_counters(&engine, p1, earth_king_s_lieutenant()),
        1,
        "another Ally entered, so the Lieutenant rallied"
    );
    assert_eq!(pt(&engine, ekl), (2, 2), "and is a 2/2 while it holds it");

    reach_their_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, cursed_mirror());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![ekl] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let mirror = on_battlefield(&engine, p0, cursed_mirror()).expect("the Mirror arrived");
    assert_eq!(
        plus_one_counters(&engine, p0, cursed_mirror()),
        0,
        "a counter is not a copiable value, so none came across"
    );
    assert_eq!(
        pt(&engine, mirror),
        (1, 1),
        "and the body is the printed one, not the one the counter made"
    );
}
