//! A copy is handed the original's rules text (CR 707.2), and this is every place that text is then read: the offer, the trigger scan, the statics and the replacement rules `sync_static_effects` registers, and the look-back that lets a copy keep the dies trigger it copied (CR 603.10a). Both producers are here, the token a Rite of Replication creates and the permanent a Glasspool Mimic or a Sakashima enters as, because they reach the text through different code and were each wrong on their own — and with them the permanent that becomes a copy and goes on carrying its *printed* static, plus the pool-wide walk that says no other card reaches that unnoticed. What a bystander across the table is for here is the original: a grant scoped to "you control" reads the same on a board where the copied creature is still standing beside its copy. A copy that ends belongs in `temporary_copies`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Glasspool Mimic ("enter as a copy of a creature **you control**") plus
/// Earth King's Lieutenant ("When this creature enters, put a +1/+1 counter
/// on each other Ally creature you control", and "Whenever another Ally you
/// control enters, put a +1/+1 counter on this creature").
///
/// A copy is not a still picture: the Mimic that arrives as a Lieutenant
/// *has* the Lieutenant's enters-trigger and fires it, and the original then
/// sees an Ally enter and grows itself. Three cards' text has to agree for
/// the numbers below to come out, which is the reason to run it rather than
/// reason about it.
#[test]
fn a_mimic_copying_an_ally_fires_what_it_copied_and_cannot_copy_across_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(73, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                earth_king_s_lieutenant(),
                ondu_cleric(),
            ],
        )
        .hand(0, &[glasspool_mimic()])
        .battlefield(1, &[snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lieutenant =
        on_battlefield(&engine, p0, earth_king_s_lieutenant()).expect("lieutenant deployed");
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric deployed");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their mage");
    assert_eq!(pt(&engine, lieutenant), (1, 1));
    assert_eq!(pt(&engine, cleric), (1, 1));

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

    let options = target_options(&engine);
    assert!(
        options.contains(&lieutenant) && options.contains(&cleric),
        "either of my creatures may be copied: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"a creature you control\" does not reach across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();

    // The copy's own enters-trigger counters every *other* Ally, and the
    // original Lieutenant's rally trigger then counters itself: 1/1 → 3/3
    // for the original, 1/1 → 2/2 for the cleric.
    pass_until(&mut engine, |e| pt(e, lieutenant) == (3, 3));
    assert_eq!(
        pt(&engine, cleric),
        (2, 2),
        "the copy's enters-trigger reached the cleric too"
    );
}

/// The copy Rite of Replication makes is a copy of the creature's **rules
/// text** as well as its characteristics (CR 707.2).
///
/// The proof is a colour the rest of the board cannot make: seat 0 has four
/// Islands and nothing green, so a `{G}` in its pool came out of the copy of
/// their Llanowar Elves or out of nowhere. The bystanders are those Islands,
/// which were offering their own mana all along, and the Elf across the
/// table, which is still theirs and still untapped afterwards.
#[test]
fn a_token_copy_carries_the_rules_text_of_the_creature_it_copies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(93, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[rite_of_replication()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let victim = aim_at_their_elf(&mut engine, rite_of_replication(), false);
    let copy = the_copy_on(&engine, p0);

    // CR 302.6: it arrived this turn, so its `{T}` is not offered yet. The
    // seat is named because a list belonging to the other one would be
    // silent about the copy for a reason that has nothing to do with the
    // claim.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the resolved spell hands priority back");
    };
    assert_eq!(player, p0, "to the seat that cast it");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == copy),
        "a creature that entered this turn cannot tap"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase grants priority");
    };
    let index = legal
        .abilities
        .iter()
        .find_map(|(id, index)| (*id == copy).then_some(*index))
        .expect("the copy offers the Elf's mana ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: copy,
                ability_index: index,
            },
        )
        .expect("an offered ability is activatable");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(baylee_core::mana::ManaColor::Green),
        1,
        "four Islands make no green — the copy did"
    );
    assert!(
        engine
            .state()
            .object(copy)
            .is_some_and(|o| o.card.is_none()),
        "and it is a token while it does it (CR 707.10)"
    );
    let original = engine.state().object(victim).expect("their Elf");
    assert_eq!(
        original.controller, p1,
        "the creature copied is still theirs"
    );
    assert!(
        !original.status.contains(crate::object::Status::TAPPED),
        "and tapping the copy did not tap it"
    );
}

/// Where the two halves meet: a copy is handed the original's rules text
/// *and* its arrival is an event, so the copy fires **its own** enters
/// trigger (CR 707.2, CR 603.6a).
///
/// Neither half alone reaches this. Rules text with no event is a card that
/// can be tapped for mana and never triggers; an event with no rules text is
/// a body the watchers see and that has nothing of its own to say. Baleful
/// Strix draws a card as it enters and asks for no target, so the whole
/// claim is one number.
///
/// The card is drawn by the seat that controls the **copy**, which is the
/// seat that cast the spell and not the one whose Strix is being copied —
/// so their library is the bystander, and it is the same size afterwards as
/// it was before.
#[test]
fn the_copy_fires_the_enters_trigger_it_copied() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(96, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[rite_of_replication()])
        .battlefield(1, &[baleful_strix(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (mine, theirs) = (library_of(&engine, p0), library_of(&engine, p1));

    let victim = aim_at_theirs(&mut engine, rite_of_replication(), baleful_strix(), false);
    let copy = the_copy_on(&engine, p0);

    assert_eq!(
        library_of(&engine, p0),
        mine - 1,
        "the copy entered and drew me a card, exactly once"
    );
    assert_eq!(
        library_of(&engine, p1),
        theirs,
        "the Strix that was copied did not enter again"
    );
    assert!(
        engine
            .state()
            .object(copy)
            .is_some_and(|o| o.card.is_none() && o.controller == p0),
        "and the thing that drew it is a token of mine (CR 707.10)"
    );
    assert!(
        engine.state().object(victim).is_some(),
        "their Strix is still on the battlefield"
    );
}

/// The third of the three places a permanent's rules text is read, after the
/// offer and the trigger scan: its **static** abilities, which are
/// continuous effects the machine registers rather than anything a player
/// takes (CR 611.3).
///
/// Great Divide Guide gives each land and Ally its controller has
/// "{T}: Add one mana of any color", so the copy gives them to *mine*. Sea
/// Gate Loremaster is the subject because it is an Ally with no mana of its
/// own — a land would be in `mana_abilities` either way, on the CR 305.6
/// shortcut, and would say the same thing before and after. The four Islands
/// are the bystanders twice over: they make no green, and they were offering
/// their own mana the whole time.
///
/// The Guide across the table is the other bystander. Its grant reads "you
/// control", so it never reached my board, and copying it did not tap it.
#[test]
fn the_copy_registers_the_static_ability_it_copied() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                sea_gate_loremaster(),
            ],
        )
        .hand(0, &[rite_of_replication()])
        .battlefield(1, &[great_divide_guide(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ally = on_battlefield(&engine, p0, sea_gate_loremaster()).expect("my Ally");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase grants priority");
    };
    assert!(
        !legal.mana_abilities.contains(&ally),
        "an Ally with no mana ability of its own is offered none"
    );

    let victim = aim_at_theirs(
        &mut engine,
        rite_of_replication(),
        great_divide_guide(),
        false,
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the resolved spell hands priority back");
    };
    assert_eq!(player, p0, "to the seat that cast it");
    assert!(
        legal.mana_abilities.contains(&ally),
        "the copy's static ability reached my Ally"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: ally })
        .expect("an offered mana ability is activatable");
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("any-colour mana asks a colour, got {:?}", engine.pending());
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Green),
        )
        .expect("the colour is the ability's own choice");

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        1,
        "four Islands make no green — the copied grant did"
    );
    assert!(
        engine
            .state()
            .object(ally)
            .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED)),
        "and the {{T}} in the granted cost was paid"
    );
    let original = engine.state().object(victim).expect("their Guide");
    assert_eq!(original.controller, p1, "the Guide copied is still theirs");
    assert!(
        !original.status.contains(crate::object::Status::TAPPED),
        "and copying it did not tap it"
    );
}

/// The other half of that same scan: a **replacement** rule (CR 614), which
/// `sync_static_effects` registers in a second loop beside the statics and
/// which was reading the card behind the permanent for the same reason and
/// with the same result.
///
/// Katara, the Fearless ("If a triggered ability of an Ally you control
/// triggers, that ability triggers an additional time") over Earth King's
/// Lieutenant ("Whenever another Ally you control enters, put a +1/+1
/// counter on this creature"). One counter per firing is the readout
/// `panharmonicon_fires_your_enters_trigger_twice_and_theirs_once` already
/// established on this very pair of cards, so two counters is two firings
/// and the copy's rule was registered.
///
/// The copy is both what causes the trigger and what doubles it, which is
/// not a trick: a continuous effect is on as soon as the permanent is on
/// the battlefield, and the engine registers it earlier in the same pass
/// than the one that collects the trigger. The Lieutenant across the table
/// is the bystander for the *rally's* own "you control": the copy entered
/// under my control, so their rally never fired at all. Katara's filter
/// says "you control" too, and the `2` is what reads it from where the copy
/// stands — evaluated from seat 1's, my Lieutenant is not their Ally and
/// the number would be `1`.
#[test]
fn the_copy_registers_the_replacement_rule_it_copied() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(98, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                earth_king_s_lieutenant(),
            ],
        )
        .hand(0, &[rite_of_replication()])
        .battlefield(
            1,
            &[katara_the_fearless(), forest(), earth_king_s_lieutenant()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        plus_one_counters(&engine, p0, earth_king_s_lieutenant()),
        0,
        "no Ally has entered yet, on either side"
    );
    assert_eq!(plus_one_counters(&engine, p1, earth_king_s_lieutenant()), 0);

    let victim = aim_at_theirs(
        &mut engine,
        rite_of_replication(),
        katara_the_fearless(),
        false,
    );

    assert_eq!(
        plus_one_counters(&engine, p0, earth_king_s_lieutenant()),
        2,
        "my Lieutenant's rally fired twice: once for the Ally that entered, \
         and once more because that Ally was a Katara"
    );
    assert_eq!(
        plus_one_counters(&engine, p1, earth_king_s_lieutenant()),
        0,
        "nothing entered under their control, so their rally never fired"
    );
    assert_eq!(
        engine.state().object(victim).map(|o| o.controller),
        Some(p1),
        "and the Katara that was copied is still theirs"
    );
}

/// The **other** producer of a copied rules text, and the one that has a
/// card: [`Engine::check_copy_on_enter`] hands a Glasspool Mimic the list
/// of the creature it entered as (CR 707.2), and the scan that registers
/// statics was reading the Mimic's own printed face instead — which carries
/// none, so the thing it had become contributed nothing.
///
/// Karmic Guide is the one creature a Mimic can copy here whose static says
/// something about *itself*: `Filter::This`, protection from black. Being
/// self-scoped is what makes it visible at all. Every other copyable static
/// in this pool is a grant scoped to "you control", and a Mimic copies a
/// creature you control — so the original would still be standing there
/// supplying it, and two of them would read as the same board as one.
///
/// Vindicate ("Destroy target permanent") is white **and** black, so
/// protection from black keeps it off either Angel (CR 702.16b). The Island
/// in the same list is the bystander that says the spell was castable and
/// looking at my side of the table the whole time.
#[test]
fn a_mimic_copying_a_protected_creature_is_protected_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(99, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[karmic_guide(), glasspool_mimic()])
        .battlefield(1, &[plains(), swamp(), forest()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast rather than seated: Karmic Guide has echo {3}{W}{W}, and a
    // Guide that began the game on the battlefield was gone by the time
    // this line ran — the first upkeep is the one `reach_main_phase`
    // walks through, and answering for echo there is the only thing on
    // the card that could have taken it. Cast on my own turn, the echo is
    // asked at my *next* one, past the end of the measurement.
    cast_from_hand(&mut engine, p0, karmic_guide());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, karmic_guide()).is_some() && stack_is_empty(e)
    });
    let guide = on_battlefield(&engine, p0, karmic_guide()).expect("my Angel");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase grants priority");
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
    // The question is asked with the Mimic already on the battlefield —
    // `check_copy_on_enter` runs from `apply_enter_modifiers`, after the
    // permanent has arrived — so "any creature you control" reaches it, and
    // a Mimic offered as a copy of itself would be a choice that leaves it
    // a 0/0 shapeshifter and dies to the same state-based action that
    // brought it there. Asserted where the offer is made rather than in a
    // test of its own, because this is the one place the offer exists.
    assert!(
        !target_options(&engine).contains(
            &on_battlefield(&engine, p0, glasspool_mimic())
                .expect("the Mimic is standing there while it asks")
        ),
        "the permanent doing the copying is not among the things it may copy"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![guide],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, p0, glasspool_mimic()).expect("the Mimic arrived");

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = target_options(&engine);

    assert!(
        !options.contains(&copy),
        "the Mimic entered as an Angel with protection from black: {options:?}"
    );
    assert!(
        !options.contains(&guide),
        "and the Angel it copied has always had it: {options:?}"
    );
    assert!(
        options.contains(&on_battlefield(&engine, p0, island()).expect("my Island")),
        "while a land of mine was a legal target the whole time: {options:?}"
    );
}

/// The permanent that becomes a copy and keeps its **own** printed static,
/// which `progress::apply_copy_choice` records as unreachable and is not.
///
/// The mechanism is written down there: `sync_static_effects` registers a
/// permanent's statics at step 0a and only a *departure* un-registers one,
/// while the copy is applied at 0b and merely rewrites `own_abilities`. So a
/// permanent that becomes a copy goes on carrying the continuous effect its
/// printed text asked for, and the note says no card in the pool can reach
/// that — the three it names print no static of their own.
///
/// Sakashima of a Thousand Faces is the tenth card with an enters-as-a-copy
/// ability and the one the note missed, because it prints exactly such a
/// static: "the legend rule doesn't apply to permanents you control". It
/// reaches the path on every cast.
///
/// And it is right by accident, which is why this is a test and not a fix.
/// The card says "…except it has Sakashima's other abilities", and the DSL
/// has no `CopyMod` that can say so — `mods: &[]` is the whole of what the
/// card carries. The clause that cannot be expressed and the effect that is
/// never un-registered cancel out exactly here: copy your own legend with a
/// Sakashima and you keep both, which is what the card does. A card wanting
/// the *opposite* — a printed static that should stop when the copy starts —
/// would be wrong, and the pool has none.
///
/// Padeem is the legend copied because she asks nothing on the way in: no
/// enters-trigger, no target, and her artifact hexproof reaches a board with
/// no artifact on it. What is being watched is the seat's own two
/// permanents, both legendary and both named Padeem after the copy lands —
/// a pair CR 704.5j would put one of into a graveyard.
#[test]
fn a_sakashima_copying_my_own_legend_keeps_the_legend_rule_off() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                padeem_consul_of_innovation(),
            ],
        )
        .hand(0, &[sakashima_of_a_thousand_faces()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let padeem =
        on_battlefield(&engine, p0, padeem_consul_of_innovation()).expect("my legend deployed");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase grants priority")
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("four Islands, four mana");
    }
    let sakashima =
        in_hand(&engine, p0, sakashima_of_a_thousand_faces()).expect("Sakashima in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: sakashima })
        .expect("{3}{U} off four Islands");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let options = target_options(&engine);
    assert!(
        options.contains(&padeem),
        "another creature I control may be copied: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![padeem],
            },
        )
        .expect("the copy choice is answered");

    // The pair exists: two permanents, one controller, the same name, both
    // legendary. Without that the assertion below would pass on a board the
    // legend rule was never asked about.
    let chars = |id| {
        engine
            .state()
            .object(id)
            .expect("on the battlefield")
            .characteristics()
    };
    assert_eq!(
        chars(sakashima).name,
        chars(padeem).name,
        "the copy took the legend's name"
    );
    assert!(
        chars(sakashima)
            .supertypes
            .contains(baylee_core::types::SupertypeSet::LEGENDARY),
        "and its legendary supertype with it"
    );

    // And the mechanism, stated rather than left to the outcome: the copy no
    // longer *has* the printed static — `own_abilities` is the legend's list
    // now — while the continuous effect it registered on the way in is still
    // in the table. That is the whole of the note in `apply_copy_choice`, and
    // the day a copy stops carrying its own printed statics this fails here
    // rather than at the assertion below, which is the more useful place: it
    // says the fix has to arrive with a `CopyMod` for "except it has its
    // other abilities", or Sakashima loses what the card prints.
    let copied_abilities = engine
        .state()
        .object(sakashima)
        .expect("the copy")
        .abilities(&RegistryLookup);
    assert!(
        !copied_abilities.iter().any(|a| matches!(
            a,
            AbilityDef::Static(sa) if matches!(sa.modifier, baylee_cards_dsl::Modifier::LegendRuleOff)
        )),
        "the copy took the legend's abilities, so its own printed static is \
         not among them"
    );
    assert!(
        engine.state().effects.iter().any(|fx| {
            fx.source == Some(sakashima)
                && matches!(fx.modifier, baylee_cards_dsl::Modifier::LegendRuleOff)
        }),
        "the static registered on the way in is still in the effect table"
    );

    assert!(
        !matches!(engine.pending(), Pending::LegendChoice { .. }),
        "the legend rule was applied to a seat Sakashima's static exempts: \
         {:?}",
        engine.pending()
    );
    for id in [sakashima, padeem] {
        assert_eq!(
            engine.state().object(id).map(|o| o.zone),
            Some(Zone::Battlefield),
            "both legends stay on the battlefield"
        );
    }
}

/// No card becomes a copy carrying a printed static without somebody having
/// looked at it.
///
/// The note in `progress::apply_copy_choice` said the pool could not reach
/// this, and named the three permanents it had in mind. There are ten cards
/// with an enters-as-a-copy ability, and the count is exactly why a
/// pool-wide claim belongs in a test rather than in a comment: a sentence
/// that was true when it was written goes on reading as checked long after a
/// card batch has made it false.
///
/// Tokens too, through the door `tokens::ALL` keeps opening — nothing there
/// enters as a copy today, and the walk costs one loop.
///
/// What it keys on is `CopyOnEnter*` in a card's **own** ability list, and
/// that is complete only for as long as a card's own text is the one way a
/// permanent becomes a copy. `Modifier::BecomeCopyOf` takes an
/// `ObjectId`, which no card text can name — `apply_copy_choice` is its only
/// writer and `CopyOnEnter*` is what sends it there — so the permanent that
/// becomes a copy is always the one whose text said it would. (A
/// `CreateTokenCopyOf` is a different shape: the token is made as a copy and
/// never had a static of its own to keep.) The day the DSL can say "target
/// permanent becomes a copy of another", this walk stops seeing the card that
/// says it and needs a second arm reading that effect — and it would go on
/// passing, which is the failure a pool-wide guard has instead of a red test.
#[test]
fn no_card_becomes_a_copy_carrying_a_printed_static_unnoticed() {
    let becomes_a_copy = |abilities: &[AbilityDef]| {
        abilities.iter().any(|a| {
            matches!(
                a,
                AbilityDef::CopyOnEnter { .. } | AbilityDef::CopyOnEnterUntilEot { .. }
            )
        })
    };
    let printed_static =
        |abilities: &[AbilityDef]| abilities.iter().any(|a| matches!(a, AbilityDef::Static(_)));

    let mut offenders = Vec::new();
    let mut still_true = Vec::new();
    let mut check = |who: &str, abilities: &[AbilityDef]| {
        if !becomes_a_copy(abilities) || !printed_static(abilities) {
            return;
        }
        if COPIES_KEEPING_A_PRINTED_STATIC
            .iter()
            .any(|(name, _)| *name == who)
        {
            still_true.push(who.to_string());
        } else {
            offenders.push(who.to_string());
        }
    };
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            check(def.name(), def.abilities_for_face(face));
        }
    }
    for token in baylee_cards::tokens::ALL {
        check(token.name, token.abilities);
    }
    assert!(
        offenders.is_empty(),
        "a permanent prints a static ability and becomes a copy, so it keeps \
         that static registered after the copy takes its rules text away — \
         decide whether that is what the card says, and either fix \
         `apply_copy_choice` or write the reason into \
         COPIES_KEEPING_A_PRINTED_STATIC: {offenders:?}"
    );
    for (name, why) in COPIES_KEEPING_A_PRINTED_STATIC {
        assert!(
            still_true.iter().any(|n| n == name),
            "{name} is excused here for a shape it no longer has — {why}; \
             take the entry out of COPIES_KEEPING_A_PRINTED_STATIC"
        );
    }
}

/// CR 603.10a: a copy that dies keeps the dies trigger it copied.
///
/// The owner's Phyrexian Metamorph had entered as a copy of Solemn
/// Simulacrum, died in combat, and drew nobody a card. Three steps and the
/// ability was gone before anything could see it: `move_object` gives a
/// card-backed object its printed rules text back on the way off the
/// battlefield (CR 400.7 — the card in the graveyard is the printed card),
/// the look-back scan then reads the object *as it is now*, and what it finds
/// there is a Metamorph, whose one ability is "enter as a copy" and whose
/// index 1 does not exist at all.
///
/// The rule is explicit about which of those two things it wants: the game
/// looks back "using the existence of those abilities and the appearance of
/// objects immediately prior to the event". Not only the appearance.
///
/// The copy is of **their** Golem, so the ability under test is one this seat
/// only ever had as a copy: with the original on my own side of the table a
/// draw would prove nothing, because the original would still be standing
/// there with a dies trigger of its own.
///
/// Counter-test: put `obj.abilities(lookup)` back in `collect_for_objects`'s
/// look-back branch and the hand does not grow.
#[test]
fn a_copy_that_dies_keeps_the_trigger_it_copied() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(77, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[phyrexian_metamorph()])
        .battlefield(
            1,
            &[solemn_simulacrum(), plains(), plains(), swamp(), swamp()],
        )
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, phyrexian_metamorph());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let original = on_battlefield(&engine, p1, solemn_simulacrum()).expect("their Golem");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![original],
            },
        )
        .unwrap();
    // The copy brings the *enters* trigger with it as well, which is the same
    // rule read forwards and is asked here before anything else can happen.
    // Declined — the search is "you may", and a fetched Forest would only add
    // a card to the board the measurement below has to explain.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, p0, phyrexian_metamorph()).expect("the copy arrived");

    reach_their_main_phase(&mut engine, p1);
    let before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    assert!(
        !vindicated(&mut engine, p1, copy),
        "the copy is an ordinary creature and their removal destroys it"
    );
    let after = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();

    assert_eq!(
        after,
        before + 1,
        "the Golem it had become dies and draws its controller a card"
    );
}
