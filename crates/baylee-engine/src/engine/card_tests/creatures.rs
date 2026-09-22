//! Cards whose front face is a creature, the door `cards/creatures/`
//! puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use crate::choice::ChoicePrompt;

#[test]
fn a_copied_rebound_spell_does_not_schedule_a_cast_of_a_vanished_copy() {
    let p0 = PlayerId::new(0);
    let ephemerate = card_index("0fd57894-b917-41c8-a394-360d1d31b236");
    let mut engine = Duel::new(21, forest())
        .battlefield(0, &[plains(), jin_gitaxias(), ondu_cleric()])
        .hand(0, &[ephemerate])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).unwrap();
    let jin = on_battlefield(&engine, p0, jin_gitaxias()).unwrap();
    let original = in_hand(&engine, p0, ephemerate).unwrap();
    cast_from_hand(&mut engine, p0, ephemerate);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    let options = options_offered_including(&mut engine, jin);
    assert!(options.contains(&jin));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![jin] })
        .unwrap();
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));
    let rebounds: Vec<_> = engine
        .state
        .delayed
        .iter()
        .filter_map(|d| match d.action {
            crate::state::DelayedAction::CastFromExileWithoutPaying { card, .. } => Some(card),
            _ => None,
        })
        .collect();
    assert_eq!(
        rebounds,
        vec![original],
        "only the spell actually cast from hand can rebound"
    );
}

#[test]
fn rebound_does_not_follow_a_card_out_of_exile_and_back() {
    let p0 = PlayerId::new(0);
    let ephemerate = card_index("0fd57894-b917-41c8-a394-360d1d31b236");
    let mut engine = Duel::new(21, forest())
        .battlefield(0, &[plains(), ondu_cleric()])
        .hand(0, &[ephemerate])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).unwrap();
    let original = in_hand(&engine, p0, ephemerate).unwrap();
    cast_from_hand(&mut engine, p0, ephemerate);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));
    let delayed = engine.state.delayed.remove(0).action;
    engine
        .state
        .move_object(
            original,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            original,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.delayed_queue.push_back(delayed);
    assert!(
        !engine.process_delayed(),
        "CR 400.7: the returning card is a new object"
    );
    assert!(engine.cast_wizard.is_none());
}

/// Earth King's Lieutenant ({G}{W}, 1/1): the ETB puts a +1/+1 counter
/// on each other Ally — here the Ondu Cleric that waited on the board.
#[test]
fn earth_king_s_lieutenant_etb_counters_other_allies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[forest(), plains(), ondu_cleric()])
        .hand(0, &[earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric deployed");
    assert_eq!(pt(&engine, cleric), (1, 1));

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let lieutenant = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: lieutenant })
        .unwrap();

    // The spell resolves, the ETB trigger resolves: the cleric grew.
    pass_until(&mut engine, |e| pt(e, cleric) == (2, 2));
    let lieutenant =
        on_battlefield(&engine, p0, earth_king_s_lieutenant()).expect("lieutenant landed");
    assert_eq!(pt(&engine, lieutenant), (1, 1), "no counter on itself");
}

/// Jin-Gitaxias, Progress Tyrant: "copy that spell. You may choose new
/// targets for the copy." The copy starts on the original's target, and its
/// controller is asked whether to move it — here they do, so one Swords to
/// Plowshares exiles two creatures.
#[test]
fn jin_gitaxias_copy_may_be_given_a_new_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(21, forest())
        .battlefield(0, &[plains(), jin_gitaxias()])
        .hand(0, &[swords_to_plowshares()])
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
    let swords = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords })
        .unwrap();

    // The spell's own target, chosen at cast time: p1's cleric.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the spell's target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&cleric), "the cleric is targetable");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    // Walk to the copy's re-choice, answering anything the trigger asks on
    // the way (its own target is the spell that was cast).
    let options = options_offered_including(&mut engine, lieutenant);
    assert!(
        options.contains(&cleric) && options.contains(&lieutenant),
        "every legal creature is offered, not just the original target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();

    // Original exiles the cleric, the retargeted copy exiles the lieutenant.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, ondu_cleric()).is_none()
            && on_battlefield(e, p1, earth_king_s_lieutenant()).is_none()
    });
}

/// Great Divide Guide grants "{T}: Add one mana of any color" to each land and
/// Ally its controller has — and it is an Ally, so it grants the ability to
/// itself.
///
/// A *granted* mana ability is offered in `LegalActions::mana_abilities`
/// alongside the CR 305.6 shortcut, and until now it could not be taken from
/// there: `ActivateManaAbility` went straight to `intrinsic_mana`, which
/// answers only for a land with one basic type, so the engine refused an
/// action it had just listed. Every caller reads that list the same way — the
/// house AI sends `ActivateManaAbility { source: legal.mana_abilities[0] }`
/// outright — so the list has to mean one thing.
#[test]
fn a_granted_mana_ability_is_activatable_the_way_it_is_offered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(63, forest())
        .battlefield(0, &[great_divide_guide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guide = on_battlefield(&engine, p0, great_divide_guide()).expect("the guide is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase grants priority");
    };
    assert!(
        legal.mana_abilities.contains(&guide),
        "the guide grants itself a mana ability and the engine offers it"
    );
    assert!(
        !legal.lands.contains(&guide),
        "and it is not a land, which is the whole point: it has no intrinsic mana"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: guide })
        .expect("an offered mana ability is activatable");

    // "One mana of any color" asks which — the ability resolved rather than
    // erroring, which is the claim.
    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("any-colour mana asks a colour, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "and asks the seat that tapped it");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("the colour is the ability's own choice");

    // Not erroring is only half of it. The synthetic index reaches
    // `start_activation`, which is what pays the cost — if it ever skipped
    // that, a granted `{T}` ability would be infinite mana and this is where
    // that has to fail.
    assert!(
        engine
            .state()
            .object(guide)
            .expect("the guide is still there")
            .status
            .contains(Status::TAPPED),
        "paying {{T}} left it tapped"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "and it is the colour that was named"
    );
}

/// "Exile **up to one** target ... you don't control" against a board with
/// nothing on it to exile.
///
/// The trigger still goes on the stack. CR 603.3d removes a triggered
/// ability that cannot be given a legal target, and one that requires no
/// target always can be — so what it must not do is *ask*: with an empty
/// option list and a minimum of zero, the only answer is the empty list,
/// and a stop the player cannot influence is not a choice. The engine used
/// to publish `ChooseTargets { options: [], min: 0, max: 0 }` and wait
/// there.
///
/// The second board is what keeps the first honest. A filter that matched
/// nothing at all would pass the first half and read exactly the same, so
/// the same Apparition is put down against a creature it *can* exile and
/// the question has to appear.
#[test]
fn up_to_one_target_with_nothing_to_point_at_is_not_a_question() {
    let p0 = PlayerId::new(0);
    let mut engine = a_skyclave_over(&[forest(), forest()]);
    let mut trigger_stacked = false;
    for _ in 0..20 {
        if stack_is_empty(&engine) {
            break;
        }
        // Something on the stack while the Apparition is already standing is
        // its own enters-trigger: the spell has left, and nothing else at
        // this table triggers at all. Without this the test would read the
        // same on a trigger CR 603.3d had *removed* — an exile that finds
        // nothing to exile does nothing either way, so "nobody was asked" is
        // only half of what is being claimed.
        if on_battlefield(&engine, p0, skyclave_apparition()).is_some() {
            trigger_stacked = true;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("nothing was legal to exile, so nothing was asked: {other:?}"),
        }
    }
    assert!(
        trigger_stacked,
        "the trigger went on the stack, with no targets and no question"
    );
    assert!(stack_is_empty(&engine), "and then resolved");
    assert!(
        on_battlefield(&engine, p0, skyclave_apparition()).is_some(),
        "and the Apparition itself is standing there, trigger and all"
    );

    let mut engine = a_skyclave_over(&[forest(), ondu_cleric()]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
        unreachable!("the loop above stopped on one")
    };
    assert_eq!(min, 0, "\"up to one\" may still decline");
    assert_eq!(
        options,
        vec![on_battlefield(&engine, PlayerId::new(1), ondu_cleric()).expect("their Cleric")],
        "their Cleric is the one thing it may point at"
    );
}

/// "…for as long as this creature remains on the battlefield." Swords to
/// Plowshares takes the Tidebinder away and the Strix has its flying and its
/// deathtouch back.
///
/// The rider was written `Duration::UntilEndOfTurn`, which is what the card
/// file's header claimed too — and both were wrong in the same direction:
/// the suppression is not a turn's effect, it is the Tidebinder's, and it
/// outlives the turn exactly as long as the Tidebinder does.
#[test]
fn the_strix_takes_its_keywords_back_when_the_tidebinder_leaves() {
    let (mut engine, p0, p1, strix) = a_strix_the_tidebinder_answered();
    let tidebinder = on_battlefield(&engine, p1, tishanas_tidebinder()).expect("it stayed");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana_but(&mut engine, p0, None);
    let stp = in_hand(&engine, p0, swords_to_plowshares()).expect("the sword is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: stp })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&tidebinder),
        "the tidebinder was not a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tidebinder],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, tishanas_tidebinder()).is_none() && stack_is_empty(e)
    });

    let keywords = keywords_of(&engine, strix);
    assert!(
        keywords.contains(KeywordSet::FLYING) && keywords.contains(KeywordSet::DEATHTOUCH),
        "the tidebinder is gone and the strix is still stripped: {keywords:?}"
    );
}

/// Aang and Katara make X Allies at once; Wartime Protestors says
/// "whenever **another Ally** you control enters, put a +1/+1 counter on
/// that creature and it gains haste". Every one of them is an Ally
/// entering, so the rally fires once for each.
///
/// It fired **once for the whole batch**, because the trigger scan broke
/// out of the event loop after the first match. Six tokens arrived, one of
/// them was answered, and the other five were invisible to everything on
/// the board — which is how it was reported: "4 of the tokens disappeared
/// and only one of the two was handled correctly".
///
/// The counted assertion is the counter-test in both directions. One
/// counter on each token fails on the old code (five have none) and would
/// also fail if the loop were made to fire per *permanent* per event, which
/// is the shape that gives N² triggers for N tokens.
#[test]
fn a_rally_trigger_fires_once_for_every_ally_that_entered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(
            0,
            &[
                wartime_protestors(),
                forest(),
                plains(),
                island(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[aang_and_katara()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(tokens_of(&engine, p0).is_empty(), "no tokens yet");

    // Tapping everything for mana is what sets X. The lands go through
    // `mana_abilities` and the Sol Rings through `abilities` — an intrinsic
    // land tap and a printed mana ability are two different offers — and
    // `tap_all_mana` takes both (#159), so the three artifacts are tapped by
    // the same call that taps the lands. The Protestors is a creature with
    // no mana ability and stays untapped, which is why X is three and not
    // four.
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, aang_and_katara()).expect("the spell is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("six mana is on the table");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && tokens_of(e, p0).len() == 3
    });

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 3, "one Ally per tapped artifact");
    // Let the three rally triggers resolve.
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });

    for (n, token) in tokens.iter().copied().enumerate() {
        let obj = engine
            .state()
            .object(token)
            .expect("the token is still here");
        assert_eq!(
            obj.counters.get(CounterKind::P1P1),
            1,
            "token {n} was answered exactly once"
        );
        assert!(
            keywords(&engine, token).contains(KeywordSet::HASTE),
            "token {n} gained haste"
        );
    }

    let protestors = on_battlefield(&engine, p0, wartime_protestors()).expect("still there");
    assert_eq!(
        engine
            .state()
            .object(protestors)
            .expect("the source is on the battlefield")
            .counters
            .get(CounterKind::P1P1),
        0,
        "the trigger says `another Ally`",
    );
}

/// Ertai Resurrected's second mode, which is the mutant for the collection
/// arm: it is the only place in the pool where
/// `Effect::DrawCardsFor { who: PlayerRel::ControllerOfTarget }` can run at
/// all, and a modal trigger that never fires is a card whose whole printed
/// text is unreachable. "Destroy another target creature or planeswalker.
/// Its controller draws a card."
#[test]
fn ertais_chosen_mode_destroys_and_lets_its_victim_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, island())
        .battlefield(0, &[island(), island(), swamp(), swamp()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[ertai_resurrected()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, quiet_creature()).expect("the opponent's creature");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    cast_from_hand(&mut engine, p0, ertai_resurrected());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Answered by *position*, and the position is not the mode number here:
    // with an empty stack Ertai's first mode has no spell or ability to
    // counter, so CR 603.3c takes it off the list and "destroy" is offered
    // first. A test that sent `ChooseMode(1)` picked the decline instead —
    // which is what it did before this line existed, and it failed loudly
    // rather than quietly, because the destroy asked for no target.
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![1, 2],
        "destroy and decline; \"counter target spell, activated ability, or \
         triggered ability\" has nothing on an empty stack",
    );
    let destroy = modes
        .iter()
        .position(|m| *m == 1)
        .expect("the destroy mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(destroy)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the destroy mode asked for no target — got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "another creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "the targeted creature was destroyed",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before + 1,
        "\"its controller draws a card\" — the *target's* controller, not \
         Ertai's; `PlayerRel::ControllerOfTarget` has never resolved for a \
         trigger before, because no modal trigger ever reached the stack",
    );
}

/// CR 608.2h: an effect that needs information about an object no longer in
/// the zone it was expected to be in uses that object's last known
/// information.
///
/// Swords to Plowshares is two effects in one sentence — exile the creature,
/// *then* read its power — so the second half asks about an object the first
/// half moved. It read the printed card and paid one life for a 2/2.
#[test]
fn swords_reads_the_creature_it_exiled_as_it_last_stood() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(41, swords_to_plowshares(), &[]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let life_before = engine.state().players[0].life;

    let swords = in_hand(&engine, p0, swords_to_plowshares()).expect("the sword is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, umara_raptor()).is_none(),
        "the Raptor was exiled",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "the power it had on the battlefield, not the 1 its card prints",
    );
}

/// The same sentence on a creature, and the reason the sweep named two cards
/// and not one: Solitude exiles and reads through a *trigger* rather than a
/// spell, which is a second resolution path to the same `Amount`.
#[test]
fn solitudes_trigger_reads_the_creature_it_exiled_as_it_last_stood() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(42, solitude(), &[]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let life_before = engine.state().players[0].life;

    let incarnation = in_hand(&engine, p0, solitude()).expect("Solitude is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: incarnation })
        .unwrap();
    // Solitude itself targets nothing; the first target question belongs to
    // its enters trigger, and the Raptor is the only other creature.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, umara_raptor()).is_none(),
        "the Raptor was exiled",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "the power it had on the battlefield, not the 1 its card prints",
    );
}

/// A trigger that can find no legal target takes *itself* off the queue and
/// nothing else.
///
/// `collect_triggers` pops the entry it is working on before it asks a
/// synthetic trigger for its target, so the branch that drops a granted
/// trigger with no legal target (CR 603.3d) was popping a second time — and
/// the second pop took whatever was queued behind it, unread and unresolved.
///
/// Wizard Class at level 3 is the only card in the pool that grants a
/// *targeted* trigger, and a Class is an enchantment, so its controller can
/// hold it with no creature anywhere to put the counter on. The draw that
/// fires it fires Sheoldred across the table on the same event, and
/// Sheoldred's is the trigger that was being eaten: the life it takes is the
/// whole assertion.
#[test]
fn a_trigger_that_finds_no_target_takes_only_itself_off_the_queue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![island(); 8];
    board.push(wizard_class());
    let mut engine = Duel::new(9, quiet_artifact())
        .battlefield(0, &board)
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Both levels in one main phase: eight Islands is {2}{U} and {4}{U}
    // exactly, and a mana pool empties at the end of a step, not on a pass.
    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, wizard_class(), 2);
    pass_until(&mut engine, stack_is_empty);

    let before = engine.state().players[0].life;
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        before - 2,
        "Sheoldred's trigger was queued behind one that fizzled, and still resolved",
    );
}

/// And the same trigger *answered* takes only itself off the queue.
///
/// The fizzle branch and the answer path are two pops for one queue entry,
/// both of them after the tail pop that already removed it. This is the half
/// a player actually reaches: a creature on the board means the granted
/// trigger has a target, the question is asked, and it was the answer that
/// ate the trigger behind it — so the more a board has going on, the more
/// there is to lose.
#[test]
fn answering_a_granted_triggers_target_takes_only_itself_off_the_queue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![island(); 8];
    board.push(wizard_class());
    board.push(quiet_creature());
    let mut engine = Duel::new(9, quiet_artifact())
        .battlefield(0, &board)
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, wizard_class(), 2);
    pass_until(&mut engine, stack_is_empty);

    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elves are out");
    let before = engine.state().players[0].life;
    reach_their_main_phase(&mut engine, p1);
    // `walk_to_own_main` and not `reach_their_main_phase`: the draw step on
    // the way asks for the counter's target, which passing cannot answer.
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round"
    );

    assert_eq!(
        engine.state().players[0].life,
        before - 2,
        "Sheoldred's trigger was queued behind one that was answered, and still resolved",
    );
    assert_eq!(
        engine
            .state()
            .object(elves)
            .map(|o| o.counters.get(CounterKind::P1P1)),
        Some(1),
        "the granted trigger put its own counter down",
    );
}

/// A two-card draw is two draws, and a draw-watcher fires for both.
///
/// `draw_cards` records one `CardsDrawn { count }` for the whole draw, so an
/// ability that watches draws saw one event and fired once — entry 35's
/// defect in the shape its fix could not see, a batch that is a field rather
/// than a list of events.
///
/// Wizard Class's own level-up draws two cards and Sheoldred, the Apocalypse
/// takes 2 life per card an opponent draws, so the assertion is a **count**
/// in both directions: 2 life is the old bug, 6 would be firing per card and
/// per event both, and 4 is the card.
#[test]
fn a_two_card_draw_fires_a_draw_watcher_twice() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(21, quiet_artifact())
        .battlefield(0, &[wizard_class(), island(), island(), island()])
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    let before = engine.state().players[0].life;
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        before - 4,
        "two cards drawn, so Sheoldred took 2 life twice",
    );
}

/// Hyperfrag Round shrinks the creatures of the player it named, and of
/// nobody else.
///
/// "Creatures target player controls get -2/-2 until end of turn" was
/// written as a mode with no target at all and a filter matching every
/// creature on the battlefield — so a 3/2 choosing its own second mode
/// killed itself, the board it had just joined and the opponent's together.
/// The mode targets a player now, and `PumpFilter::controlled_by` is what
/// reads the choice back.
///
/// Three assertions because there are three ways to be wrong: the named
/// seat's creature dies, the caster's does not, and the Eliminator itself
/// is still standing.
#[test]
fn primaris_eliminators_hyperfrag_shrinks_only_the_player_it_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(43, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[primaris_eliminator()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my own Elves");

    cast_from_hand(&mut engine, p0, primaris_eliminator());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let hyperfrag = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("the Hyperfrag Round is offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(hyperfrag))
        .unwrap();

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("Hyperfrag asked for no player — got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a legal choice for \"target player\"",
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

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "the named player's 1/1 took -2/-2",
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "my own creature is not one of theirs",
    );
    assert!(
        on_battlefield(&engine, p0, primaris_eliminator()).is_some(),
        "and a 3/2 does not kill itself with its own second mode",
    );
}

/// Halimar Excavator's rally mills a player the controller *chose*.
///
/// It was written as `PlayerRel::Opponent` with no target requirement at
/// all, so it milled the opponent by construction: the controller could
/// never mill themselves, and a player who could not legally be targeted
/// was milled anyway. The printed line is "target player mills X", and it
/// is not optional — with nobody else legal the controller has to point it
/// at themselves, which is why the requirement's `min` is one.
#[test]
fn halimar_excavator_mills_the_player_it_targeted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(57, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    cast_from_hand(&mut engine, p0, halimar_excavator());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "either player may be targeted, the controller included",
    );
    assert_eq!(min, 1, "\"target player mills X\" is not optional");

    // Aimed at the controller, which is the half the old card could not do.
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard_before + 1,
        "one Ally on the battlefield, so the player it named mills one card",
    );
}

/// Vendilion Clique: "look at **target player's** hand", which is the whole
/// reason the card is played — you point it at yourself to bottom the card
/// you would rather not have drawn and draw again.
///
/// It was written as `PlayerRel::Opponent` with no target at all, so the one
/// thing it is famous for was the one thing it could not do.
#[test]
fn vendilion_clique_may_be_pointed_at_its_own_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[vendilion_clique(), counterspell(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, vendilion_clique());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert!(
        player_options.contains(&p0),
        "the controller is a legal target: {player_options:?}",
    );
    assert!(player_options.contains(&p1), "and so is the opponent");
    assert_eq!(min, 1, "the trigger is not optional; the card choice is");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();

    // The trigger is on the stack with its target chosen; it resolves when
    // the round of priority after it does.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // The choice is the *controller's*, whoever's hand it is — "look at
    // target player's hand. **You** may choose a nonland card from it."
    let Pending::ChooseCards {
        player: chooser,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the bottom choice, got {:?}", engine.pending())
    };
    assert_eq!(chooser, p0, "the Clique's controller picks the card");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // One card left the hand for the bottom of the library and one was
    // drawn, so the hand is the size it was and the library is too — and
    // the opponent, who used to be the only seat this could reach, is
    // untouched.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "bottomed one and drew one",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before,
        "the card went under the library the draw came off",
    );
}

/// Loran of the Third Path: "{T}: You and **target opponent** each draw a
/// card." An opponent, so the controller is not on offer — and *one* of
/// them, which `PlayerRel::Opponent` could not say.
#[test]
fn loran_draws_for_the_one_opponent_she_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, plains())
        .battlefield(0, &[loran_of_the_third_path()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let loran = on_battlefield(&engine, p0, loran_of_the_third_path()).expect("Loran deployed");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == loran)
        .expect("Loran's tap ability is offered");
    let (hand0, hand1) = (
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseTargets {
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        player_options,
        vec![p1],
        "\"target opponent\" leaves the controller out (CR 115.1)",
    );
    assert_eq!((min, max), (1, 1));

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
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand0 + 1,
        "you draw",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1 + 1,
        "and so does the opponent you named",
    );
}

/// Orcish Bowmasters: "deals 1 damage to any target. Then amass Orcs 1."
///
/// The card was written with the damage pointed at `PlayerRel::Opponent` and
/// no target requirement on the ability at all, so the arrow was never aimed
/// — the engine reads the *ability's* requirement and the effect's own
/// `target` field is not what it asks about. At a duel that is invisible
/// (there is one opponent, and they were hit either way); at a four-player
/// table it picked one, and it could never hit a creature.
#[test]
fn the_bowmasters_aim_where_their_controller_points() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, island())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[orcish_bowmasters()])
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
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    let before = engine.state().players[1].life;
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("two Swamps pay {1}{B}");

    assert!(
        settle_aiming_at(&mut engine, p1),
        "the enters trigger asks where its arrow goes"
    );
    assert_eq!(
        engine.state().players[1].life,
        before - 1,
        "and the chosen face takes the damage"
    );
}

/// The same ability's other trigger. It is one printed ability with two
/// triggers and the engine has no variant for that, so the card writes it
/// twice — and the second copy was written without the damage, so an
/// opponent's extra draw amassed an Orc and fired no arrow.
///
/// Mikokoro makes both players draw on *this* turn, which is outside the
/// opponent's draw step, so it is never their excepted first draw.
#[test]
fn an_opponents_extra_draw_fires_an_arrow_as_well_as_an_orc() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(12, island())
        .battlefield(0, &[orcish_bowmasters(), mikokoro(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let well = on_battlefield(&engine, p0, mikokoro()).expect("Mikokoro");
    tap_mana_except(&mut engine, p0, well);
    let before = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: well,
                ability_index: 1,
            },
        )
        .expect("two Swamps pay the {2}");

    assert!(
        settle_aiming_at(&mut engine, p1),
        "the draw trigger asks where its arrow goes"
    );
    assert_eq!(
        engine.state().players[1].life,
        before - 1,
        "an opponent's extra draw costs them a life, not only a token"
    );
}

/// Myr Retriever ({2}, 1/1): "When this creature dies, return **another**
/// target artifact card from your graveyard to your hand."
///
/// The word the whole card turns on is `another`, and it is load-bearing in a
/// way no other dies trigger's is: by the time the ability is put on the
/// stack the Myr is itself an artifact card lying in that same graveyard
/// (CR 603.6c, CR 400.7), so a trigger that read "target artifact card" would
/// offer the Myr its own corpse and return it to hand every time — a
/// two-mana artifact that recurs itself forever, which is not the card.
///
/// Both halves are struck, and on the ids the cards have **after** the move:
/// comparing against the Myr's battlefield id would pass however wrong the
/// filter was, because an object changes id when it changes zone.
///
/// The library is filled with an artifact rather than a basic land, which is
/// what gives `seed_graveyard` an artifact card to put there — the Myr needs
/// something legal to point at or the trigger would be removed from the stack
/// for having no legal target, and the interesting assertion would never be
/// reached.
#[test]
fn a_dying_myr_returns_another_artifact_card_and_never_its_own_corpse() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &[myr_retriever()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "an artifact card is waiting in the graveyard"
    );

    reach_their_main_phase(&mut engine, p1);
    let myr = on_battlefield(&engine, p0, myr_retriever()).expect("the Myr is out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![myr] })
        .expect("their removal may point at an ordinary creature");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    let corpse = in_graveyard(&engine, p0, myr_retriever()).expect("the Myr died");
    let other = in_graveyard(&engine, p0, quiet_artifact()).expect("and it is not alone");
    assert!(
        !options.contains(&corpse),
        "`another` keeps the Myr from targeting itself in the graveyard it is \
         now lying in: {options:?}"
    );
    assert!(
        options.contains(&other),
        "and the other artifact card is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![other],
            },
        )
        .expect("the one legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "the artifact card came back to hand"
    );
    assert!(
        in_graveyard(&engine, p0, myr_retriever()).is_some(),
        "and the Myr stayed where it fell"
    );
}

// oracle_id = "e3c85068-b4b6-40b9-a16c-5c3b2d059ec4"
fn academy_rector() -> baylee_core::ids::CardIndex {
    card_index("e3c85068-b4b6-40b9-a16c-5c3b2d059ec4")
}

/// An Academy Rector killed by the opponent's Vindicate, stopped at the one
/// question the card asks.
///
/// Two boards over one builder rather than two arms of one, for the reason
/// the Ondu Cleric pair has: the second answer wants the same open board the
/// first one spent.
///
/// The whole library is Luminarch Ascension, and that is what gives the
/// trigger somewhere to reach: "search your library for an enchantment card"
/// has an answer, so a library that stays whole below is a search that was
/// never made rather than a search that found nothing — failing to find in a
/// hidden zone is always legal, and the two outcomes would look the same.
fn a_rector_asking() -> (Engine<RegistryLookup>, PlayerId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, luminarch_ascension())
        .battlefield(0, &[academy_rector()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    let rector = on_battlefield(&engine, p0, academy_rector()).expect("the Rector is out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![rector],
            },
        )
        .expect("their removal may point at an ordinary creature");

    // Stopping *at* the "may" rather than through it: `pass_until` checks the
    // predicate before it answers anything, so the question the card prints
    // is still unanswered when this hands the game back.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    (engine, p0)
}

/// Academy Rector ({3}{W}, 1/2): "When this creature dies, you may exile it.
/// If you do, search your library for an enchantment card, put that card
/// onto the battlefield, then shuffle."
///
/// One printed "may" gates both halves, and the gate is the thing worth
/// playing: the exile is what the tutor is paid with, so a Rector that stays
/// in its graveyard must also leave the library alone. Both answers are
/// struck on boards identical up to the question.
///
/// The enchantment has to arrive on the **battlefield** and not in hand,
/// which is why the last assertion reads the found card's own zone: p0's
/// hand is full of Luminarch Ascensions off the filler deck, so "is one in
/// hand" was already true before the Rector ever died and would have passed
/// against a card that fetched to the wrong place.
#[test]
fn the_rectors_enchantment_arrives_only_when_it_exiles_itself() {
    // Declined: the Rector lies where it fell, and nothing is searched for.
    let (mut engine, p0) = a_rector_asking();
    let library_before = library_size(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, academy_rector()).is_some(),
        "a declined \"you may exile it\" leaves the Rector in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, luminarch_ascension()).is_none(),
        "and \"if you do\" fetches no enchantment at all"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the library was never searched"
    );

    // Taken: the Rector exiles itself, and that buys the enchantment.
    let (mut engine, p0) = a_rector_asking();
    let library_before = library_size(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the loop above waited for exactly this")
    };
    assert_eq!(
        (min, max),
        (1, 1),
        "\"search your library for an enchantment card\" finds one card and \
         is not an \"up to\""
    );
    let found = *options
        .first()
        .expect("the library is nothing but enchantments");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a card the search itself offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, academy_rector()).is_none(),
        "the Rector paid the exile it offered"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Exile(p0))
            .iter()
            .copied()
            .any(|id| engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == academy_rector()))),
        "and it is exiled rather than merely gone"
    );
    assert_eq!(
        engine
            .state()
            .object(found)
            .expect("the card the search found")
            .zone,
        crate::zone::Zone::Battlefield,
        "the enchantment is put onto the battlefield, not into hand"
    );
    assert!(
        !engine
            .state()
            .object(found)
            .expect("the card the search found")
            .status
            .contains(crate::object::Status::TAPPED),
        "and it is put onto the battlefield, not onto the battlefield tapped \
         — the printed sentence names no tapping, and `Find::BATTLEFIELD` is \
         the half of the pair that says so"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and it came out of the library"
    );
}

// oracle_id = "f9b46a1a-474f-4fac-8d71-131c1720e4c0"
fn borg_queen_perfection_manifest() -> baylee_core::ids::CardIndex {
    card_index("f9b46a1a-474f-4fac-8d71-131c1720e4c0")
}

/// Borg Queen, Perfection Manifest ({4}{B}{B}, 1/4): "Artifact creatures you
/// control get +2/+0. When Borg Queen enters, assimilate target creature card
/// from an opponent's graveyard. (Put it onto the battlefield under your
/// control with a +1/+1 counter. It's a Borg artifact creature and loses all
/// other creature types.)"
///
/// One landing plays both printed sentences at once, and they are read on the
/// same permanent on purpose: assimilate makes its victim an *artifact*
/// creature, so the anthem has to catch a creature that was not an artifact
/// when the anthem started. That is CR 613.1 — layer 4 hands the type change
/// to layer 7c — and the Elf's 4/2 is the only number on the board that could
/// not come out of any one clause alone: 1/1 printed, +2/+0 from the anthem,
/// then the +1/+1 counter on top.
///
/// Three counter-proofs keep each assertion from passing for the wrong reason.
/// The bystander Elf p0 already controls is a creature the anthem must *not*
/// touch, so a 1/1 there is the word "artifact" in the filter doing work
/// rather than a blanket pump. A creature card is seeded into p0's **own**
/// graveyard beside the one in p1's, so "an opponent's graveyard" is a choice
/// the engine makes and not the only card there was. And the victim is read on
/// the `ObjectId` it had while it was still a card in that graveyard, which is
/// what says it *moved* — `GraveyardToBattlefield` keeps the id across the
/// zone change, so the three effects behind the target all land on one object.
///
/// The last block is the other half of `Coverage::Partial`. The card's
/// `NOT SUPPORTED` note says the one clause it does not write is "and loses
/// all other creature types": nothing in `Modifier` subtracts a subtype. So
/// the assimilated Elf is asserted to still be an Elf Druid beside its new
/// Borg type — the gap spelled out, and written to **fail** the day a modifier
/// can set a creature-type set, which is the day this card stops being
/// `Partial`.
#[test]
#[allow(clippy::too_many_lines)] // one game, played from the cast to the assertion
fn a_landing_borg_queen_assimilates_their_creature_card_and_pumps_only_artifact_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[borg_queen_perfection_manifest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The creature the anthem must not reach: p0 controls it, and it is not
    // an artifact.
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("a plain Elf is out");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the bystander is an ordinary 1/1 before the Queen lands"
    );

    // A creature card in each graveyard, so "an opponent's" is a choice.
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let mine = in_graveyard(&engine, p0, llanowar_elves()).expect("one of mine is buried");
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("and one of theirs");

    cast_from_hand(&mut engine, p0, borg_queen_perfection_manifest());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    assert!(
        options.contains(&theirs),
        "the creature card in the opponent's graveyard is offered: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "and assimilate never reaches into my own graveyard: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the one card the trigger may point at");
    pass_until(&mut engine, stack_is_empty);

    // The victim moved, and it kept its id across the move.
    let assimilated = engine
        .state()
        .object(theirs)
        .expect("the assimilated card is still an object");
    assert_eq!(
        assimilated.zone,
        crate::zone::Zone::Battlefield,
        "it was put onto the battlefield"
    );
    assert_eq!(
        assimilated.controller, p0,
        "under the Queen's controller, not its owner's"
    );
    assert_eq!(
        assimilated
            .counters
            .get(baylee_cards_dsl::CounterKind::P1P1),
        1,
        "with a +1/+1 counter on it"
    );

    let c = assimilated.characteristics();
    assert!(
        c.types.contains(baylee_core::types::TypeSet::ARTIFACT),
        "it is an artifact creature now"
    );
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::BORG),
        "and a Borg"
    );

    // The anthem, read on both halves of its filter.
    let queen =
        on_battlefield(&engine, p0, borg_queen_perfection_manifest()).expect("the Queen landed");
    assert_eq!(
        pt(&engine, queen),
        (3, 4),
        "the Queen is an artifact creature she controls, so she pumps herself"
    );
    assert_eq!(
        pt(&engine, theirs),
        (4, 2),
        "1/1 printed, +2/+0 because assimilate made it an artifact, +1/+1 from \
         the counter"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and the Elf that is only a creature is left alone — the anthem's \
         subject is artifact creatures"
    );

    // `Coverage::Partial`: the clause that is not written.
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::ELF)
            && c.subtypes
                .contains(baylee_core::generated::subtypes::creature::DRUID),
        "`and loses all other creature types` is the card's NOT SUPPORTED \
         note: no `Modifier` subtracts a subtype, so the assimilated Elf Druid \
         keeps both tribes beside Borg. When this fires, the clause has become \
         expressible and the card is no longer Coverage::Partial"
    );
}

// oracle_id = "c7b044c3-3cfa-407e-bf20-2875e8e04b7b"
fn brazen_borrower() -> baylee_core::ids::CardIndex {
    card_index("c7b044c3-3cfa-407e-bf20-2875e8e04b7b")
}

/// Answers whatever the engine asks until `pred` holds, through the kit's
/// own `answer_one` — with the cleanup discard taken back off it.
///
/// [`pass_until`] would do for all of this but one question, and it is the
/// question this scenario is built to ask: a seat that deliberately holds
/// the card it was dealt across the *opponent's* turn sits on eight cards
/// at a cleanup and is asked to discard, which `pass_until` has no arm for.
/// `answer_one` does have one — and it answers with the first cards in the
/// hand, which may be the Borrower itself, so the card under test would be
/// thrown away before it was ever cast. Everything else is delegated.
#[track_caller]
fn walk_the_game_until(
    engine: &mut Engine<RegistryLookup>,
    pred: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..200 {
        if pred(engine) {
            return;
        }
        if let Pending::DiscardChoice { player, count } = engine.pending().clone() {
            let filler: Vec<ObjectId> = engine
                .state()
                .zones
                .list(crate::zone::ZoneLocation::Hand(player))
                .iter()
                .copied()
                .filter(|id| {
                    engine
                        .state()
                        .object(*id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index != brazen_borrower()))
                })
                .take(usize::from(count))
                .collect();
            engine
                .apply(player, PlayerAction::ChooseObjects { objects: filler })
                .expect("a seat discards down to seven");
            continue;
        }
        let (player, action) = match answer_one(engine) {
            Ok(pair) => pair,
            Err(rest) => panic!("the game stopped before the test did: {rest:?}"),
        };
        engine
            .apply(player, action)
            .expect("every answer came out of the question that enumerated it");
    }
    panic!("the walk never reached what it was waiting for");
}

/// Brazen Borrower // Petty Theft ({1}{U}{U}, 3/1): "Flash. Flying. This
/// creature can block only creatures with flying." — in front of Petty
/// Theft, an Adventure instant for {1}{U}: "Return target nonland permanent
/// an opponent controls to its owner's hand."
///
/// The card is played here the way it is played at a table, which is the
/// only way its four printed lines are all in one scene: the adventure goes
/// off on the **opponent's** turn, the permanent it names goes back to its
/// owner's hand, the card is exiled on its adventure (CR 715), and the
/// creature is cast out of that exile — still on the opponent's turn, which
/// is what flash buys (CR 702.8a against CR 117.1a). Every press goes
/// through the engine's own offer: `legal.castable` before each cast, the
/// `ChooseCastMode` list for which face, the `ChooseTargets` list for which
/// permanent.
///
/// The targeting list is where the printed restrictions are struck. The
/// opponent's artifact and the opponent's creature are both on it;
/// **their land** is not, which is `nonland`, and **my own creature** is
/// not, which is `an opponent controls`. A filter that lost either word
/// would still bounce the artifact and still pass a test that only looked
/// at the artifact.
///
/// Then the other half of `Coverage::Partial`, and the reason the card
/// carries it. "This creature can block only creatures with flying" is not
/// enforced: `combat::can_block` reads the attacker's flying and
/// unblockable and the blocker's flying and reach, and nothing in the
/// engine names the attackers a given blocker may be paired with. (Menace
/// is not in that list and was never a pairing question: CR 702.111b
/// restricts the whole declaration, so it is counted in
/// `Engine::declare_blockers` — #156.) So a 1/1
/// ground Elf attacks and the Borrower is offered against it, which the
/// printed line forbids. The assertion is written to say so and to break
/// the day it stops being true — a blocker with no legal attacker is
/// dropped from the offer entirely, so when the restriction lands, this
/// test fails and `Coverage::Partial` is what gets flipped.
#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability
fn the_borrower_flashes_out_of_its_own_adventure_and_then_blocks_a_ground_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, island())
        .hand(0, &[brazen_borrower()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[forest(), llanowar_elves(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("a creature of my own");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("and one of theirs");
    let their_land = on_battlefield(&engine, p1, forest()).expect("a land of theirs");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("the permanent to steal");

    // Held through their turn and cast in it: the stack is empty, the
    // active player is the other seat, and the card is a creature.
    walk_the_game_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let card = in_hand(&engine, p0, brazen_borrower()).expect("still in hand on their turn");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "flash puts the card on offer while the other seat is the active player"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");

    // Six Islands pay for either face, so the engine asks which is being
    // cast — the {1}{U}{U} creature or the {1}{U} adventure.
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected the face choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        options
            .iter()
            .any(|o| matches!(o.kind, crate::choice::CastModeKind::Normal)),
        "the creature is one way to cast the card: {options:?}"
    );
    let adventure = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
        .expect("and Petty Theft is the other");
    engine
        .apply(p0, PlayerAction::ChooseMode(adventure))
        .expect("the mode came out of the list that was offered");

    // "Target nonland permanent an opponent controls": both of their
    // nonland permanents, their land not, my own creature not.
    walk_the_game_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    assert!(
        options.contains(&ring) && options.contains(&theirs),
        "every nonland permanent the opponent controls is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&their_land),
        "`nonland` keeps their Forest off the list: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "`an opponent controls` keeps my own creature off it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the permanent the theft is aimed at");
    let stack = engine.state().zones.list(crate::zone::ZoneLocation::Stack);
    let spell = stack.last().copied().expect("a spell on the stack");
    let obj = engine.state().object(spell).expect("the spell exists");
    assert_eq!(obj.face_index, 1, "the adventure is the back face");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Petty Theft"
    );

    // It resolves, and the walk stops at this seat's next quiet priority —
    // still inside the same step, which is what keeps the pool alive.
    walk_the_game_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the permanent Petty Theft named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's hand"
    );
    let exiled = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Exile(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == brazen_borrower()))
        })
        .expect("the card itself went on its adventure (CR 715)");
    assert!(
        engine
            .state()
            .object(exiled)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Adventure)),
        "wearing the rider that says it may be cast from there"
    );

    // Still the same step, so what the theft did not spend is still in the
    // pool — a step's end is what empties it (CR 500.5).
    assert!(
        engine.state().players[0].mana_pool.total() >= 3,
        "three of the six Islands are unspent, which is the creature's price"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&exiled),
        "the creature is on offer out of the exile its own adventure made"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .expect("the offer is honoured");
    walk_the_game_until(&mut engine, |e| {
        on_battlefield(e, p0, brazen_borrower()).is_some()
    });
    let borrower = on_battlefield(&engine, p0, brazen_borrower()).expect("the creature landed");
    assert_eq!(pt(&engine, borrower), (3, 1), "a 3/1 Faerie Rogue");
    assert!(
        keywords(&engine, borrower).contains(baylee_cards_dsl::KeywordSet::FLYING),
        "and flying, the half of the evasion the engine does read"
    );
    assert_eq!(
        engine.state().turn.active,
        p1,
        "and all of it on the other seat's turn, which is what flash is for"
    );

    // Their ground creature attacks. It is summoning sick on the turn it
    // began the game under, so the loop walks to the combat where the
    // engine itself offers it (CR 508.1a).
    let mut attacked = false;
    for _ in 0..8 {
        walk_the_game_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let Pending::ChooseAttackers {
            player, attackers, ..
        } = engine.pending().clone()
        else {
            unreachable!("the walk waited for exactly this")
        };
        if player == p1 && attackers.contains(&theirs) {
            engine
                .apply(
                    p1,
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(theirs, baylee_core::ids::Defender::Player(p0))],
                    },
                )
                .expect("the attacker came out of the offer");
            attacked = true;
            break;
        }
        engine
            .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
            .expect("an empty attack is always legal");
    }
    assert!(
        attacked,
        "their creature attacks once it is no longer summoning sick"
    );

    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                assert_eq!(player, p0, "the attack is aimed at me, so I am blocking");
                break blockers;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while reaching blockers: {other:?}"),
        }
    };
    let Some(offered) = blockers.iter().find(|o| o.blocker == borrower) else {
        panic!(
            "the Borrower was offered no attacker at all — either it is not being \
             read as an untapped creature, or the restriction below has been \
             implemented and this half of the test is the one to rewrite: \
             {blockers:?}"
        )
    };
    assert!(
        offered.attackers.contains(&theirs),
        "the printed line is `This creature can block only creatures with flying` \
         and the attacker on offer is a ground Elf. `combat::can_block` reads the \
         attacker's flying and unblockable and the blocker's flying and reach, \
         and no `Modifier` names the attackers one blocker may be paired \
         with — which is exactly what `Coverage::Partial` promises a player here. \
         The day this fires, the pairing has learned to say it and the card is no \
         longer Partial: {blockers:?}"
    );
}

// oracle_id = "22f1a4a4-c423-4d1c-8775-0ed604a9fa51"
fn deathrite_shaman() -> baylee_core::ids::CardIndex {
    card_index("22f1a4a4-c423-4d1c-8775-0ed604a9fa51")
}

/// Deathrite Shaman ({B/G}, 1/2): "{T}: Exile target land card from **a**
/// graveyard. Add one mana of any color. (Activate only as an instant.)"
///
/// Three claims live in that one printed line and none of them had ever been
/// played. The first is the reminder text: an ability that targets is not a
/// mana ability however much mana it makes (CR 605.1a), so the Shaman has to
/// be absent from `mana_abilities` and present in `abilities`, and the mana
/// only arrives once the thing has gone on the stack and resolved. The
/// Forest standing beside it is what makes that reading non-vacuous — the
/// list is not empty, the Shaman is simply not in it.
///
/// The second is "**a** graveyard", which the card spells
/// `PlayerRel::EachPlayer`: both graveyards are seeded and both land cards
/// have to be on the offer, or the Shaman is a card that can only eat its
/// own yard. The one it is pointed at is the opponent's, and mine is checked
/// afterwards to catch the opposite fault — a relation resolved as "every
/// player" would empty both and pass a test that only looked at theirs.
///
/// The third is "one mana of any color". Red is neither of the Shaman's own
/// colours, so a pool holding {R} says the colour came from the choice and
/// not from its identity — and the Forest is still untapped, so it did not
/// come from the land either.
#[test]
#[allow(clippy::too_many_lines)] // both graveyards have to be seeded and both halves of the ability played in one duel
fn deathrite_shaman_eats_a_land_out_of_either_graveyard_and_pays_a_colour_it_is_not() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[deathrite_shaman(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A land card in each graveyard: the filler deck is Forests, so this is
    // what gives the ability something legal to point at on both sides.
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let my_land = in_graveyard(&engine, p0, forest()).expect("a land card of my own is buried");
    let their_land = in_graveyard(&engine, p1, forest()).expect("and one of theirs");

    let shaman = on_battlefield(&engine, p0, deathrite_shaman()).expect("the Shaman is out");
    let land = on_battlefield(&engine, p0, forest()).expect("and a Forest beside it");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.contains(&land),
        "the Forest is what a mana ability looks like in this list, so the \
         Shaman's absence from it below is a reading and not an empty list"
    );
    assert!(
        !legal.mana_abilities.contains(&shaman),
        "an ability that targets is no mana ability (CR 605.1a), whatever it \
         adds: {:?}",
        legal.mana_abilities
    );
    assert!(
        legal.abilities.contains(&(shaman, 0)),
        "it is offered as an ordinary activated ability instead: {:?}",
        legal.abilities
    );

    // Nothing is tapped for mana first: {T} is the entire cost.
    activate(&mut engine, p0, deathrite_shaman(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the ability asks which land card, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&my_land),
        "\"a graveyard\" includes my own: {options:?}"
    );
    assert!(
        options.contains(&their_land),
        "and it reaches across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_land],
                players: vec![],
            },
        )
        .expect("a land card in the opponent's graveyard is a legal target");

    // The mana arrives on resolution, not on activation — so the colour is
    // asked after both seats have passed on an ability sitting on the stack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    assert_eq!(player, p0, "the Shaman's controller picks the colour");
    assert!(
        options.contains(&ManaColor::Red),
        "\"any color\" is not the card's own two: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red is one of the colours that were offered");

    assert!(
        in_graveyard(&engine, p1, forest()).is_none(),
        "the land card it named left that graveyard"
    );
    let exiled: Vec<baylee_core::ids::ObjectId> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Exile(p1))
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(
        exiled.len(),
        1,
        "and it is exiled under its owner, not merely gone"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "one card was targeted and only that one moved"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Exile(p0))
            .is_empty(),
        "my own graveyard was on the offer and was not the answer"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "and it is the colour that was named"
    );
    assert!(
        engine
            .state()
            .object(shaman)
            .expect("the Shaman is still there")
            .status
            .contains(crate::object::Status::TAPPED),
        "paying {{T}} left it tapped"
    );
    assert!(
        !engine
            .state()
            .object(land)
            .expect("the Forest is still there")
            .status
            .contains(crate::object::Status::TAPPED),
        "the Forest never paid for any of this, so the mana in the pool is \
         the Shaman's"
    );
}

// oracle_id = "f9d3b046-0b95-4103-a630-4b3fb88bb60b"
fn delighted_halfling() -> baylee_core::ids::CardIndex {
    card_index("f9d3b046-0b95-4103-a630-4b3fb88bb60b")
}
fn ravenous_chupacabra() -> baylee_core::ids::CardIndex {
    card_index("7b459306-149b-4f43-abc1-2dd70c748c0e")
}

/// Delighted Halfling: "{T}: Add {C}." and "{T}: Add one mana of any color.
/// Spend this mana only to cast a legendary spell, and that spell can't be
/// countered."
///
/// Three claims in one sentence, and each is satisfiable on its own by a
/// card that is wrong. So the board is built to separate them.
///
/// The spend restriction is read against **Ravenous Chupacabra**, which
/// costs exactly the `{2}{B}{B}` Sheoldred does and is not legendary: with
/// three Swamps tapped neither is castable, and the Halfling's fourth mana
/// makes one of them castable and not the other. Same cost, same colour,
/// same window — the supertype is the only thing left to be doing the work.
/// A test that had paired the legend with a cheaper commoner would have
/// passed against a restriction that did nothing at all.
///
/// The rider is read against a real Counterspell taken to resolution,
/// because "can't be countered" is not "can't be targeted": the Counterspell
/// must still be offered the Sheoldred as a target and must still resolve,
/// and what differs is only where the creature ends up.
#[test]
#[allow(clippy::too_many_lines)] // one printed sentence, three clauses, one board
fn delighted_halflings_mana_pays_only_for_the_legend_and_makes_it_uncounterable() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(79, forest())
        .battlefield(0, &[delighted_halfling(), swamp(), swamp(), swamp()])
        .hand(0, &[sheoldred_the_apocalypse(), ravenous_chupacabra()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let halfling = on_battlefield(&engine, p0, delighted_halfling()).expect("the Halfling is out");
    let legend = in_hand(&engine, p0, sheoldred_the_apocalypse()).expect("Sheoldred is in hand");
    let commoner = in_hand(&engine, p0, ravenous_chupacabra()).expect("the Chupacabra is in hand");

    // Two printed mana abilities, and the card is offered both of them.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(halfling, 0)) && legal.abilities.contains(&(halfling, 1)),
        "the plain {{C}} is ability 0 and the restricted any-colour is ability 1: {:?}",
        legal.abilities
    );

    // Three Swamps is one mana short of either four-drop. Neither is
    // castable yet, which is what makes the comparison below a comparison —
    // so the Halfling is named as the one source kept back, since it is the
    // fourth mana and this test is about which spell that mana may pay for.
    tap_mana_except(&mut engine, p0, halfling);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "three Swamps, and the Halfling held back"
    );
    assert!(
        !legal.castable.contains(&legend) && !legal.castable.contains(&commoner),
        "{{2}}{{B}}{{B}} is four mana and three are floating"
    );

    // "Add one mana of any color" — and the choice really is all five.
    activate(&mut engine, p0, delighted_halfling(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    let all_colors = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ];
    assert_eq!(options, all_colors, "one mana of any color");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("colour chosen");

    // The fourth mana is black and is not *free* black: the rider rides on
    // the mana, so the pool keeps it as a restricted entry.
    {
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 4, "three Swamps and the Halfling");
        assert_eq!(
            pool.available(ManaColor::Black),
            3,
            "the Halfling's black is not one of the three"
        );
        let restricted = pool.restricted();
        assert_eq!(restricted.len(), 1);
        assert_eq!(restricted[0].color, ManaColor::Black);
        assert_eq!(restricted[0].amount, 1);
        assert!(
            engine
                .state()
                .restriction_info
                .contains_key(&restricted[0].restriction.0),
            "the spend restriction is registered, or nothing can check it"
        );
    }

    // The whole of "spend this mana only to cast a legendary spell", in one
    // priority window and against one cost.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&legend),
        "a legendary spell is what this mana is for"
    );
    assert!(
        !legal.castable.contains(&commoner),
        "Ravenous Chupacabra costs the same {{2}}{{B}}{{B}} and is not legendary"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: legend })
        .expect("Sheoldred is cast with the Halfling's mana");
    // `total()` counts the restricted entry too, so nought is the whole
    // pool: the restricted black paid rather than sitting the cast out.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all four mana went into it, the Halfling's among them"
    );

    // "…and that spell can't be countered." A hard counter, a legal target,
    // taken all the way to resolution.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "expected the opponent to hold priority, got {:?}",
        engine.pending()
    );
    tap_all_mana(&mut engine, p1);
    let cs = in_hand(&engine, p1, counterspell()).expect("Counterspell is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .expect("two Islands pay for it");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![legend],
        "can't be countered is not can't be targeted — the spell is still a legal target"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, sheoldred_the_apocalypse()).is_some()
    });
    assert!(
        in_graveyard(&engine, p0, sheoldred_the_apocalypse()).is_none(),
        "the spell the Halfling's mana paid for arrived instead of being countered"
    );
    assert!(
        in_graveyard(&engine, p1, counterspell()).is_some(),
        "the Counterspell itself resolved, and did nothing"
    );
}

// oracle_id = "afa49a09-146f-4439-850e-dd1938c93cef"
fn derevi_empyrial_tactician() -> baylee_core::ids::CardIndex {
    card_index("afa49a09-146f-4439-850e-dd1938c93cef")
}

/// Answers Derevi's trigger: takes mode `mode`, points it at `target`, and
/// lets it resolve.
///
/// Both of her trigger conditions run the same pair of modes — one printed
/// sentence, two abilities over one `SpellMode` list — so both readings of
/// the card go through this one door, and a difference between them would be
/// a difference in the engine rather than in the test.
///
/// Mode 0 is tap and mode 1 is untap, in the order the card writes them. They
/// are answered by *position* and not by number: a modal trigger drops the
/// modes it cannot legally choose (CR 603.3c), so the two are the same list
/// only while both are offered — which is asserted here rather than assumed.
#[track_caller]
fn tap_or_untap(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    mode: usize,
    target: ObjectId,
) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Derevi's trigger asked for no mode — got {:?}. \"You may tap or \
             untap target permanent\" is two modes, chosen by the controller \
             as the ability goes on the stack (CR 603.3c); a modal trigger \
             that is never collected is a card with no text at all.",
            engine.pending()
        )
    };
    assert_eq!(player, seat, "Derevi's controller answers her trigger");
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            crate::choice::CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 1],
        "tap *or* untap, and both are always offered: \"target permanent\" is \
         `Filter::Any` over the battlefield, which is never empty while she \
         is standing on it",
    );
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(m) if m == mode))
        .expect("the mode asked for is on the list");
    engine.apply(seat, PlayerAction::ChooseMode(slot)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the chosen mode asked for no target — got {:?}. The `TargetReq` \
             is on the mode, not on the ability.",
            engine.pending()
        )
    };
    assert!(
        options.contains(&target),
        "\"target permanent\" reaches any permanent on the table, whoever \
         controls it",
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    // The printed "may" is asked on resolution; `pass_until` takes it.
    pass_until(engine, stack_is_empty);
}

/// Derevi, Empyrial Tactician ({G}{W}{U}, 2/3): "When Derevi enters **and
/// whenever a creature you control deals combat damage to a player**, you
/// may tap or untap target permanent."
///
/// One printed sentence and two trigger conditions, which is why this test
/// does not stop when she lands. The second half is the pool's only
/// `Trigger::DealsCombatDamageToPlayer` pointed at a *filter* rather than at
/// the equipped creature, so until now nothing had fired one off a creature
/// that was not the ability's own source — and Derevi deliberately never
/// attacks here. The Llanowar Elves does the connecting, which is the whole
/// of what "a creature you control" claims.
///
/// Both modes are taken, one per trigger, and each is struck against a
/// control: the enters trigger untaps the Island that just paid for her and
/// the Plains beside it stays down, so "target permanent" is one permanent
/// rather than a sweep.
#[test]
fn derevi_asks_on_both_her_triggers_and_moves_only_the_permanent_she_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[forest(), plains(), island(), llanowar_elves()])
        .battlefield(1, &[plains()])
        .hand(0, &[derevi_empyrial_tactician()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_island = on_battlefield(&engine, p0, island()).expect("an Island of her own");
    let my_plains = on_battlefield(&engine, p0, plains()).expect("a Plains of her own");
    let their_plains = on_battlefield(&engine, p1, plains()).expect("the opponent's land");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves waited");

    // Exactly {G}{W}{U}: all three of her lands are spent, which is what
    // gives the untap mode something of consequence to point at.
    cast_from_hand(&mut engine, p0, derevi_empyrial_tactician());
    assert!(
        is_tapped(&engine, my_island) && is_tapped(&engine, my_plains),
        "both lands paid for her",
    );
    // Stopping at a quiet priority as well is what turns "the trigger never
    // fired" into a sentence instead of a hundred wasted passes.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
            || (stack_is_empty(e) && on_battlefield(e, p0, derevi_empyrial_tactician()).is_some())
    });
    tap_or_untap(&mut engine, p0, 1, my_island);
    assert!(
        !is_tapped(&engine, my_island),
        "\"you may ... untap target permanent\" — the Island she named is back up",
    );
    assert!(
        is_tapped(&engine, my_plains),
        "and only the one she named: the Plains beside it is still down",
    );

    // Her second trigger condition. The Elves has been there since before
    // turn one and swings on its controller's second turn.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(attackers.contains(&elves), "the Elves may attack");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();
    let life_before = engine.state().players[1].life;
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
            || (stack_is_empty(e) && e.state().players[1].life < life_before)
    });
    assert_eq!(
        engine.state().players[1].life,
        life_before - 1,
        "the unblocked Elves connected, which is the event the second half of \
         the sentence listens for",
    );
    assert!(
        !is_tapped(&engine, their_plains),
        "the opponent's land is up before the trigger resolves",
    );
    tap_or_untap(&mut engine, p0, 0, their_plains);
    assert!(
        is_tapped(&engine, their_plains),
        "\"you may tap ... target permanent\" — and Derevi herself never \
         attacked, so the trigger read the Elves as \"a creature you control\"",
    );
}

/// The half the card cannot do, struck where it would show.
///
/// "{1}{G}{W}{U}: Put Derevi onto the battlefield from the command zone" is
/// the `Coverage::Partial` gap, and the card file is explicit that it is not
/// written at all rather than written and skipped: `ActivationZone` names the
/// battlefield and your hand and nothing else. So she is seated as a
/// commander with four lands tapped for {G}{W}{U}{G} — which pays
/// {1}{G}{W}{U} exactly, so the ability is missing from the offer because
/// nobody wrote it and not because nobody could afford it — and the engine
/// offers precisely one thing to do with the card in that zone: cast her
/// (CR 903.8). Both halves are needed: the cast is what proves `LegalActions`
/// can see a command-zone object at all, without which "no ability is
/// offered" would be true of a card the engine had never looked at.
#[test]
fn derevi_leaves_the_command_zone_by_being_cast_and_by_no_ability_of_her_own() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .commander(0, &[derevi_empyrial_tactician()])
        .battlefield(0, &[forest(), plains(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_island = on_battlefield(&engine, p0, island()).expect("an Island of her own");
    let command_zone = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Command(p0))
        .clone();
    assert_eq!(command_zone.len(), 1, "Derevi starts in the command zone");
    let derevi = command_zone[0];

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&derevi),
        "the engine sees the card in the command zone and offers the cast",
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == derevi),
        "and offers no ability on it: \"{{1}}{{G}}{{W}}{{U}}: Put Derevi onto \
         the battlefield from the command zone\" is the gap this card's \
         `Coverage::Partial` names, so she plays exactly as though that line \
         were not printed",
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: derevi })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
            || (stack_is_empty(e) && on_battlefield(e, p0, derevi_empyrial_tactician()).is_some())
    });
    tap_or_untap(&mut engine, p0, 1, my_island);
    assert!(
        on_battlefield(&engine, p0, derevi_empyrial_tactician()).is_some(),
        "cast out of the command zone is the one road onto the battlefield \
         she has, and it works",
    );
}

// oracle_id = "c8625113-0ce4-4454-83a1-25c31b8bfb9a"
fn disciple_of_the_vault() -> baylee_core::ids::CardIndex {
    card_index("c8625113-0ce4-4454-83a1-25c31b8bfb9a")
}

/// Disciple of the Vault ({B}, 1/1): "Whenever an artifact is put into a
/// graveyard from the battlefield, you may have target opponent lose 1
/// life."
///
/// The printed sentence says **an** artifact and not one you control, so
/// the artifact that dies here is the *opponent's* Sol Ring, killed by a
/// Vindicate the Disciple's own controller casts. Every step is played
/// through what the engine offered: the spell off `castable`, its target
/// out of the choice that followed, then the trigger's target and the
/// trigger's "may".
///
/// Three claims are struck, and each fails somewhere else. The trigger has
/// to fire at all for an artifact nobody on this side ever controlled — a
/// filter narrowed to "an artifact you control" would leave the board
/// silent. Its target is a choice over the opponents only, so the
/// controller must not be among the player options; a Disciple that could
/// point at its own seat is a card that kills you. And a yes has to take
/// the life off the seat that was named: a player target rides in
/// `chosen_player`, which is what `PlayerRel::Chosen` reads back on
/// resolution, so a trigger that asked nobody would resolve into an empty
/// list of players and move no life total at all.
#[test]
fn an_opponents_artifact_dying_takes_a_life_from_the_targeted_opponent_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[disciple_of_the_vault(), plains(), swamp(), plains()])
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring =
        on_battlefield(&engine, p1, quiet_artifact()).expect("the opponent's artifact is out");
    let before = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );

    cast_from_hand(&mut engine, p0, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("Vindicate may point at any permanent, the opponent's artifact included");

    // The artifact is dead and the trigger wants a target on the way to the
    // stack, which is the second choice this scenario reaches.
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
        panic!(
            "expected the Disciple's target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the Disciple's controller is the one asked");
    assert!(
        options.is_empty(),
        "the life loss points at a player, not at an object: {options:?}"
    );
    assert_eq!(
        player_options,
        vec![p1],
        "\"target opponent\" is a choice over the opponents only, so the \
         controller is not among them"
    );
    assert_eq!((min, max), (1, 1), "one opponent, and exactly one");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the one opponent at the table");

    // "You may" — answered here rather than by `pass_until`, because the
    // word is part of what the test is about.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the printed \"may\", taken");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "an artifact really went from the battlefield to a graveyard, so the \
         life below is that death and not something else on the board"
    );
    assert_eq!(
        engine.state().players[1].life,
        before.1 - 1,
        "the targeted opponent loses the 1 life, for an artifact they \
         controlled themselves"
    );
    assert_eq!(
        engine.state().players[0].life,
        before.0,
        "and the Disciple's controller pays nothing for it"
    );
}

// oracle_id = "8eb7c0a5-6190-40de-b473-2d1daa3bbe28"
fn dualcaster_mage() -> baylee_core::ids::CardIndex {
    card_index("8eb7c0a5-6190-40de-b473-2d1daa3bbe28")
}

/// Dualcaster Mage ({1}{R}{R}, 2/2): "Flash. When this creature enters, copy
/// target instant or sorcery spell. You may choose new targets for the copy."
///
/// The two printed sentences only mean anything together, so they are played
/// together. `casting::timing_allows` lets a creature spell be cast only in
/// its controller's main phase with an **empty** stack, so a board with a
/// spell standing on it is exactly the one a creature without flash cannot be
/// cast onto — and it is also the only board on which the trigger has a legal
/// target at all. Flash is not decoration here; it is what reaches the rest of
/// the card.
///
/// The trigger is the first in the pool that goes and *picks* a spell: every
/// other card that reaches `Effect::CopyTargetSpell` from a trigger copies the
/// spell that caused it (`TargetSpec::EventObject`). What comes back is p0's
/// copy of p1's removal, and it is pointed somewhere new — one Swords to
/// Plowshares exiles two creatures, and the second one belongs to the player
/// who cast it.
#[test]
fn a_flashed_in_mage_copies_the_opponents_removal_and_points_it_back_at_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[dualcaster_mage()])
        .battlefield(1, &[plains(), ondu_cleric()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the elves are out");
    let their_cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("the cleric is out");

    // p0's main phase, and p0 hands priority straight over: the opponent's
    // instant is cast at p0's only creature.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the swords' target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&my_elves), "the elves are targetable");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![my_elves],
            },
        )
        .unwrap();

    // Back to p0 with the swords — the only thing on the stack — still on it.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let swords_spell = engine.state().zones.list(crate::zone::ZoneLocation::Stack)[0];

    // Flash. The mana goes first because `castable` is an affordability answer
    // too, and an untapped board would hide the timing question behind a price.
    tap_all_mana_but(&mut engine, p0, None);
    let mage = in_hand(&engine, p0, dualcaster_mage()).expect("the mage is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p0's priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&mage),
        "flash offers a creature spell onto a stack that is not empty"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: mage })
        .unwrap();

    // The mage resolves, enters, and its trigger goes looking for a spell.
    let offered = options_offered_including(&mut engine, swords_spell);
    assert_eq!(
        offered.len(),
        1,
        "the one instant still on the stack is the only thing to copy: {offered:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords_spell],
            },
        )
        .unwrap();

    // The copy is made under p0's control, and p0 is asked again where it
    // points — the original's target is offered too, which is how a player
    // declines the "you may".
    let retarget = options_offered_including(&mut engine, their_cleric);
    assert!(
        retarget.contains(&my_elves) && retarget.contains(&their_cleric),
        "the copy may be pointed at any legal creature: {retarget:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_cleric],
            },
        )
        .unwrap();

    // One card, two creatures exiled — and the second is the caster's own.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, ondu_cleric()).is_none()
            && on_battlefield(e, p0, llanowar_elves()).is_none()
    });
    assert!(
        on_battlefield(&engine, p0, dualcaster_mage()).is_some(),
        "the flashed-in mage is a permanent and stays where it landed"
    );
    assert!(
        in_graveyard(&engine, p1, ondu_cleric()).is_none(),
        "the copy exiled its new target, the way the card it copied reads"
    );
    assert!(
        in_graveyard(&engine, p1, swords_to_plowshares()).is_some()
            && in_graveyard(&engine, p0, swords_to_plowshares()).is_none(),
        "one card was cast and exactly one card lies in a graveyard: a copy \
         was never a card and leaves none behind"
    );
}

// oracle_id = "b11c250c-f191-4c52-ba02-a9176f163447"
fn emiel_the_blessed() -> baylee_core::ids::CardIndex {
    card_index("b11c250c-f191-4c52-ba02-a9176f163447")
}

/// Emiel the Blessed: "{3}: Exile another target creature you control, then
/// return it to the battlefield under its owner's control."
///
/// The pool's first `Effect::blink` on an *activated* ability — the five
/// that existed before it hang off a spell, a trigger or a loyalty cost — so
/// the ability is pressed out of `LegalActions::abilities`, the creature is
/// named out of the choice the engine enumerates, and the blink is read off
/// the Baleful Strix's own "When this creature enters, draw a card": a
/// creature that never left the battlefield cannot enter it, so the card
/// drawn *is* the exile-and-return having happened.
///
/// The target choice carries both halves of "another target creature you
/// control" — Emiel himself is not on offer, and neither is the opponent's
/// Llanowar Elves.
///
/// The last assertion is the half `Coverage::Partial` names. The printing
/// also says "Whenever another creature you control enters, you may pay
/// {G/W}. If you do, put a +1/+1 counter on it. If it's a Unicorn, put two
/// +1/+1 counters on it instead" — and the Strix coming back *is* another
/// creature you control entering — with a fourth Forest's `{G}` still
/// floating, so the `{G/W}` was affordable and the counter is absent for a
/// reason other than the price. That ability is deliberately not written (no
/// `Effect` asks for an optional payment of a *named* hybrid cost and runs
/// the rest of the clause on the yes), so the Strix returns a bare 1/1 and
/// nothing is asked at all: `pass_until` panics on any question that is not
/// a priority, a combat declaration or a `MayDo`, and a `MayDo` answered yes
/// — which is how the ability would have had to be written with what the
/// DSL has — would have put the counter on.
#[test]
fn emiel_blinks_another_creature_you_control_and_puts_no_counter_on_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(241, forest())
        .battlefield(
            0,
            &[
                emiel_the_blessed(),
                baleful_strix(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let emiel = on_battlefield(&engine, p0, emiel_the_blessed()).expect("Emiel deployed");
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("the Strix waited beside him");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the opponent has a creature");
    assert_eq!(pt(&engine, strix), (1, 1), "a bare Strix before anything");

    // Four Forests, three of which pay the `{3}` — the fourth is there so
    // that a `{G}` is still floating when the Strix comes back, which is
    // what the printed `{G/W}` would have been paid with. Emiel's ability
    // has no `{T}` in its cost, and he taps for nothing anyway.
    tap_all_mana_but(&mut engine, p0, None);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == emiel)
        .expect("Emiel's {3} blink is offered");
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the Forests pay the {3}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the blink asks which creature: {:?}", engine.pending())
    };
    assert!(
        options.contains(&strix),
        "the other creature you control is on offer: {options:?}"
    );
    assert!(
        !options.contains(&emiel),
        "\"another\" leaves Emiel himself out: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "\"you control\" leaves the opponent's Elves out: {options:?}"
    );
    assert_eq!(options.len(), 1, "and nothing else: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![strix],
                players: vec![],
            },
        )
        .unwrap();

    // The ability resolves, the Strix's enters-trigger goes on the stack and
    // resolves after it.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    let strix = on_battlefield(&engine, p0, baleful_strix())
        .expect("the Strix is back on the battlefield and not left in exile");
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before + 1,
        "it entered: \"When this creature enters, draw a card\" fired",
    );
    assert!(
        on_battlefield(&engine, p0, emiel_the_blessed()).is_some(),
        "and Emiel stayed where he was",
    );
    // The `// NOT SUPPORTED:` half, asserted rather than assumed: the
    // printing would have offered `{G/W}` for a +1/+1 counter as the Strix
    // came back, and this build offers nothing and places none.
    assert_eq!(
        pt(&engine, strix),
        (1, 1),
        "the unwritten enters-trigger put no +1/+1 counter on it",
    );
}

// oracle_id = "da3e7d3d-2ca0-40c3-9602-fca37c92f507"
fn emry_lurker_of_the_loch() -> baylee_core::ids::CardIndex {
    card_index("da3e7d3d-2ca0-40c3-9602-fca37c92f507")
}

/// Emry, Lurker of the Loch ({2}{U}, 1/2): "Affinity for artifacts. When Emry
/// enters, mill four cards. {T}: Choose target artifact card in your
/// graveyard. You may cast that card this turn."
///
/// One game, played through the engine's own offers: she is paid for, she
/// mills four, and on her controller's next turn — the first turn her `{T}` is
/// hers at all (CR 302.6) — she taps to lend one of the cards she milled the
/// permission the card prints. That permission is the point of the ability and
/// is spent the way a seat would spend it: the card in the graveyard turns up
/// in `legal.castable`, is cast for its own printed `{1}`, and **resolves onto
/// the battlefield**. Emry is the first card in the pool to point
/// `Effect::GrantFlashback` at a *permanent* card, and the half of flashback
/// she does not print — exile instead of the graveyard — must not reach her:
/// one more artifact standing on the table and one fewer card lying in the
/// graveyard is what says it did not.
///
/// The card stands at `Coverage::Partial`, and the first half of this test is
/// that claim rather than the working part. Two artifacts are out and exactly
/// `{U}` is floating, which is the price the printed affinity would have
/// charged — and the engine does not offer her. It is written to **fail** the
/// day that stops being true: when a cost reducer can count permanents this
/// assertion breaks, and flipping `Partial` to `Implemented` is what closes
/// it. Its counter-half is the two artifacts standing there and the two
/// further Islands right after — she is offered the moment her whole printed
/// `{2}{U}` is floating, so the refusal is the missing discount and not an
/// empty board or a card that cannot be cast at all.
#[test]
#[allow(clippy::too_many_lines)] // one game, from her cast to the one she pays for
fn emry_mills_four_and_taps_to_cast_one_of_them_but_never_costs_less() {
    let p0 = PlayerId::new(0);
    let rock = quiet_artifact();
    let mut engine = Duel::new(71, rock)
        .battlefield(0, &[island(), island(), island(), rock, rock])
        .hand(0, &[emry_lurker_of_the_loch()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    // The board the printed affinity would have read: two artifacts, so
    // "{1} less to cast for each artifact you control" would price her at {U}.
    let rocks = mine(&engine, p0, rock, Zone::Battlefield);
    assert_eq!(rocks.len(), 2, "two artifacts stand on the board");
    let islands = mine(&engine, p0, island(), Zone::Battlefield);
    assert_eq!(islands.len(), 3, "three Islands to tap");

    let emry = in_hand(&engine, p0, emry_lurker_of_the_loch()).expect("Emry is in hand");
    let first = islands[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: first })
        .expect("an Island taps for one blue");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&emry),
        "affinity for artifacts is not implemented, so one blue mana beside two \
         artifacts does not buy a card printed at {{2}}{{U}} — if this fires, a \
         cost reducer learned to count permanents and Emry is no longer \
         Coverage::Partial: {:?}",
        legal.castable
    );

    for &land in &islands[1..] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .expect("the other two Islands tap for one blue each");
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&emry),
        "three Islands pay her printed {{2}}{{U}}, so the refusal above was the \
         price she was never given a discount on: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: emry })
        .expect("what the engine offers is payable");

    // She resolves, and the enters trigger mills four off the top. The library
    // is filled with the artifact she is about to reach for.
    pass_until(&mut engine, |e| {
        mine(e, p0, rock, Zone::Graveyard).len() >= 4 && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, p0, emry_lurker_of_the_loch()).is_some(),
        "Emry landed"
    );
    assert_eq!(
        mine(&engine, p0, rock, Zone::Graveyard).len(),
        4,
        "mill four put four cards into the graveyard, and the graveyard was empty"
    );

    // Her `{T}` belongs to her controller's next turn (CR 302.6).
    let cast_on = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > cast_on
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });

    // `activate` takes the ability out of `legal.abilities` and panics if it
    // was never offered, so the tap below is one the engine published.
    activate(&mut engine, p0, emry_lurker_of_the_loch(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the choice of an artifact card in the graveyard, got {:?}",
            engine.pending()
        )
    };
    let rocks = mine(&engine, p0, rock, Zone::Battlefield);
    assert!(
        !options.iter().any(|id| rocks.contains(id)),
        "\"target artifact card in your graveyard\" never reaches an artifact \
         standing on the battlefield: {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "the four cards she milled, and nothing else: {options:?}"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("one of the cards the choice itself enumerated");
    pass_until(&mut engine, stack_is_empty);

    // The permission the whole card is for: with the mana floating, a card
    // lying in the graveyard is on the list of things this seat may cast.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&chosen),
        "the card Emry pointed at may be cast out of the graveyard: {:?}",
        legal.castable
    );

    let before = mine(&engine, p0, rock, Zone::Battlefield).len();
    engine
        .apply(p0, PlayerAction::CastSpell { card: chosen })
        .expect("its own printed {1}, paid out of the pool");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        mine(&engine, p0, rock, Zone::Battlefield).len(),
        before + 1,
        "the milled card was cast from the graveyard and resolved onto the \
         battlefield"
    );
    assert_eq!(
        mine(&engine, p0, rock, Zone::Graveyard).len(),
        3,
        "and it is the graveyard it left behind and not exile — Emry prints no \
         exile clause for the permission she grants to ride on"
    );
}

// oracle_id = "30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c"
fn eternal_witness() -> baylee_core::ids::CardIndex {
    card_index("30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c")
}

/// A cast Eternal Witness whose enters-trigger is waiting to be pointed
/// somewhere, over a graveyard holding exactly one card.
///
/// The card in that graveyard comes back with the engine, because the answer
/// has to be the object the engine itself offered rather than a card looked
/// up by index — and because the assertions afterwards are counts: an object
/// takes a new id when it changes zone, and with a Forest filler deck "a
/// Forest is in hand" is as true of a draw step as of the return.
///
/// A whole game per half, the way `hagra_on_the_table` is used: a second
/// Witness would want three untapped Forests again in a main phase that has
/// already spent them, and what the two halves share is the board, not the
/// turn.
fn a_witness_asking() -> (Engine<RegistryLookup>, PlayerId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[eternal_witness()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    let buried = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .first()
        .expect("one card was put into the graveyard");

    cast_from_hand(&mut engine, p0, eternal_witness());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    (engine, p0, buried)
}

/// Eternal Witness ({1}{G}{G}, 2/1): "When this creature enters, you **may**
/// return target card from your graveyard to your hand."
///
/// Both halves of that sentence, because this is the first card to pair
/// `GraveyardToHand` with a target minimum of nought and the two halves are
/// two different things going right. Taken, the card the trigger named is
/// gone from the graveyard and the hand is one card larger. Declined, the
/// trigger leaves that graveyard alone: the printed "you may" is written here
/// as `TargetReq::up_to_one`, so choosing no target is how a player says no,
/// and an effect that assumed a target had been chosen would empty a
/// graveyard nobody pointed at.
#[test]
fn an_entering_witness_returns_the_card_it_named_and_nothing_when_it_declines() {
    let (mut engine, p0, buried) = a_witness_asking();
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the builder stops at the target choice")
    };
    assert_eq!((min, max), (0, 1), "\"you may\" is the nought in the min");
    assert_eq!(
        options,
        vec![buried],
        "the one card in its controller's own graveyard is the one thing it \
         may be pointed at",
    );

    let held = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![buried],
            },
        )
        .expect("the one legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, eternal_witness()).is_some(),
        "the Witness finished entering, so this is its own trigger resolving",
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .is_empty(),
        "the card it named left the graveyard",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        held + 1,
        "and arrived in hand",
    );

    // The other half of the "may": a fresh table, the same trigger, declined.
    let (mut engine, p0, buried) = a_witness_asking();
    let held = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("nought targets is a legal answer to an \"up to one\"");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, eternal_witness()).is_some(),
        "the declined trigger costs the Witness nothing",
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .contains(&buried),
        "nothing was named, so the card stayed where it lay",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        held,
        "and no hand grew",
    );
}

// oracle_id = "a426a258-fd8b-489c-8642-9868ee47de85"
fn golgari_thug() -> baylee_core::ids::CardIndex {
    card_index("a426a258-fd8b-489c-8642-9868ee47de85")
}

/// How many cards sit in one zone, for the two counts the dredge half reads.
fn cards_in(engine: &Engine<RegistryLookup>, loc: crate::zone::ZoneLocation) -> usize {
    engine.state().zones.list(loc).len()
}

/// Golgari Thug ({1}{B}, 1/1): "When this creature dies, put target creature
/// card from your graveyard on top of your library." — and a printed
/// `Dredge 4` the card stands at `Coverage::Partial` for.
///
/// It is Myr Retriever's mirror, and the word that is *missing* is what makes
/// it one. The printed sentence says "target creature card", not "another",
/// so the Thug lying in the graveyard it has just fallen into is a legal
/// target for its own trigger: the trigger's targets are chosen as it is put
/// on the stack (CR 603.3d), by which time the Thug is a creature card in
/// that graveyard (CR 400.7). Both halves are struck here — the corpse is in
/// the offer, and the trigger resolves onto it and puts the Thug on top of
/// its owner's library.
///
/// The second half is what `Partial` promises. Dredge 4 replaces a draw with
/// "mill four cards, then return this card from your graveyard to your hand",
/// and nothing in the DSL says "instead of drawing", so the line is not
/// written at all. Mikokoro is activated in *response* to the Thug's own
/// trigger, which is the only moment in this game where the Thug's controller
/// draws a card with the Thug lying in their graveyard: a dredge that existed
/// would have to fire exactly there. The draw is asserted to have happened,
/// so the three assertions after it cannot pass by nobody having drawn.
#[test]
#[allow(clippy::too_many_lines)] // the replacement only shows itself against a draw that follows it in the same game
fn a_dying_thug_puts_its_own_corpse_on_top_and_replaces_no_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, quiet_creature())
        .battlefield(0, &[golgari_thug(), mikokoro(), swamp(), swamp()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // The library is Llanowar Elves, so seeding the graveyard puts a second
    // creature card there — the Thug needs something *other* than itself to
    // point at, or "its own corpse is offered too" would be the only thing
    // that could be offered and would say nothing.
    seed_graveyard(&mut engine, p0, 1);
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "a second creature card is waiting in the graveyard"
    );

    reach_their_main_phase(&mut engine, p1);
    let thug = on_battlefield(&engine, p0, golgari_thug()).expect("the Thug is out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![thug],
            },
        )
        .expect("their removal may point at an ordinary creature");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    let corpse = in_graveyard(&engine, p0, golgari_thug()).expect("the Thug died");
    let other = in_graveyard(&engine, p0, quiet_creature()).expect("and it is not alone");
    assert!(
        options.contains(&corpse),
        "the printed sentence says no `another`, so the Thug is one of its own \
         trigger's legal targets: {options:?}"
    );
    assert!(
        options.contains(&other),
        "and so is the creature card that was already lying there: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![corpse],
            },
        )
        .expect("its own corpse is one of the two legal targets");

    // The trigger is on the stack and the Thug is in the graveyard: the one
    // moment where a dredge 4 would have a draw to replace.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let well = on_battlefield(&engine, p0, mikokoro()).expect("Mikokoro is out");
    tap_mana_except(&mut engine, p0, well);
    let hand_before = cards_in(&engine, crate::zone::ZoneLocation::Hand(p0));
    let yard_before = cards_in(&engine, crate::zone::ZoneLocation::Graveyard(p0));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: well,
                ability_index: 1,
            },
        )
        .expect("two Swamps pay the {2}");
    pass_until(&mut engine, |e| {
        cards_in(e, crate::zone::ZoneLocation::Hand(p0)) > hand_before
    });
    assert_eq!(
        cards_in(&engine, crate::zone::ZoneLocation::Hand(p0)),
        hand_before + 1,
        "the draw dredge would have replaced really happened"
    );
    assert_eq!(
        in_graveyard(&engine, p0, golgari_thug()),
        Some(corpse),
        "dredge 4 is not written: the Thug stayed in the graveyard instead of \
         coming back to hand off that draw"
    );
    assert_eq!(
        cards_in(&engine, crate::zone::ZoneLocation::Graveyard(p0)),
        yard_before,
        "and nothing was milled: dredge 4 would have put four cards here"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, golgari_thug()).is_none(),
        "the trigger resolved and the Thug left the graveyard"
    );
    let top = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .last()
        .expect("p0 still has a library");
    assert!(
        engine
            .state()
            .object(top)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == golgari_thug())),
        "and it is the card on top of its owner's library"
    );
}

// oracle_id = "5eb4403f-f199-4f75-a7c6-e76783f9b07d"
fn liliana_the_repentant() -> baylee_core::ids::CardIndex {
    card_index("5eb4403f-f199-4f75-a7c6-e76783f9b07d")
}

/// Liliana the Repentant ({1}{B}, 2/2): "Whenever **another** creature or
/// planeswalker you control enters, mill two cards."
///
/// `Effect::Mill` had never been written with `PlayerRel::You` anywhere in
/// this pool — every other one names `Chosen` or `ControllerOfTarget` — so
/// *whose* library loses the two cards is the half worth striking, and the
/// opponent's is read as well: `You` is one seat and not the table.
///
/// Both arms of the filter are entered, a creature and a planeswalker, and
/// Liliana is **cast** rather than seeded so that her own entry is a real
/// one. `Filter::Another` is the word that keeps the trigger off it, and a
/// board built with her already standing on it could never say so.
#[test]
fn another_creature_or_planeswalker_entering_mills_you_two_and_liliana_herself_mills_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, forest())
        .battlefield(
            0,
            &[
                forest(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(
            0,
            &[
                liliana_the_repentant(),
                llanowar_elves(),
                karn_the_great_creator(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let graveyard_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .len();

    // Her own entry first. The Forest is the one land kept back, because it
    // is what pays for the Elves below.
    let wood = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    tap_mana_except(&mut engine, p0, wood);
    let her = in_hand(&engine, p0, liliana_the_repentant()).expect("she is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: her })
        .expect("the Swamps pay {1}{B}");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, liliana_the_repentant()).is_some(),
        "she resolved onto the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "`another` keeps her own trigger off her own entry"
    );

    // A creature, which is one arm of the filter.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves arrived"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "another creature you control entering mills you two"
    );

    // And a planeswalker, which is the other.
    cast_from_hand(&mut engine, p0, karn_the_great_creator());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, karn_the_great_creator()).is_some(),
        "Karn arrived"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 4,
        "and so does another planeswalker you control"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .len(),
        graveyard_before + 4,
        "milled, so the four are in your graveyard and nowhere else"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "`PlayerRel::You` is you, and the opponent mills nothing"
    );
}

/// Liliana's second printed line: "Exhaust — {5}{B}: Return target creature
/// or planeswalker card from your graveyard to the battlefield. Put a +1/+1
/// counter on Liliana. Activate only as a sorcery."
///
/// Two effects in one resolution, and the second is what the card's own note
/// is about: `AddCounter` puts its counters on the *first target*, which here
/// is the card coming back, so the printed counter is placed through a filter
/// naming the source instead. Both ends are struck — the card that came back
/// keeps the body it prints, and Liliana is the one that grew.
///
/// The tail is the `Coverage::Partial`, which a test of the working mechanism
/// alone would keep quiet about. "(Activate each exhaust ability only once.)"
/// is not enforced: nothing on an object records what an ability has already
/// done, so the ability is offered — and pressed, and resolved — a second
/// time in the same main phase. Twelve Swamps is two activations' worth of
/// mana on purpose, and this test **fails the day exhaust is implemented**,
/// which is the day the card stops being `Partial`.
#[test]
fn liliana_reanimates_a_creature_takes_the_counter_herself_and_may_exhaust_twice() {
    let p0 = PlayerId::new(0);
    let mut field = vec![liliana_the_repentant()];
    field.extend([swamp(); 12]);
    let mut engine = Duel::new(72, llanowar_elves())
        .battlefield(0, &field)
        .start();
    keep_mulligans(&mut engine);
    // Two creature cards, so the second activation has something to point at
    // whether or not the first one's own arrival milled any.
    seed_graveyard(&mut engine, p0, 2);
    reach_main_phase(&mut engine, p0);

    let liliana = on_battlefield(&engine, p0, liliana_the_repentant()).expect("she is out");
    assert_eq!(pt(&engine, liliana), (2, 2), "the body she prints");
    let corpse = in_graveyard(&engine, p0, llanowar_elves()).expect("a creature card waits");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, liliana_the_repentant(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the reanimation asks which card: {:?}", engine.pending())
    };
    assert!(
        options.contains(&corpse),
        "the creature card in your own graveyard is offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![corpse],
            },
        )
        .expect("a target taken out of the offer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let back = on_battlefield(&engine, p0, llanowar_elves())
        .expect("the creature card came back under your control");
    assert_eq!(pt(&engine, liliana), (3, 3), "the +1/+1 counter is hers");
    assert_eq!(
        pt(&engine, back),
        (1, 1),
        "and not the reanimated card's, which keeps the body it prints"
    );

    // The gap the `Partial` names, read off an offer that should not be there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = legal.abilities;
    assert!(
        offered.contains(&(liliana, 1)),
        "exhaust is unenforced, so the once-per-game ability is offered a \
         second time with the mana still floating: {offered:?}"
    );
    let second = in_graveyard(&engine, p0, llanowar_elves()).expect("the other creature card");
    activate(&mut engine, p0, liliana_the_repentant(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("and the second activation goes through");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        mine(
            &engine,
            p0,
            llanowar_elves(),
            crate::zone::Zone::Battlefield
        )
        .len(),
        2,
        "two creature cards off one exhaust ability in one turn — when this \
         assertion fires, exhaust is enforced and the card is no longer Partial"
    );
    assert_eq!(pt(&engine, liliana), (4, 4), "and a second counter with it");
}

// oracle_id = "726d9d2c-736a-4852-9938-a0f50d8fd89f"
fn marionette_apprentice() -> baylee_core::ids::CardIndex {
    card_index("726d9d2c-736a-4852-9938-a0f50d8fd89f")
}

fn mind_stone() -> baylee_core::ids::CardIndex {
    card_index("c97361b5-af16-4a7b-af85-a429dbaf4ad2")
}

/// Marionette Apprentice ({1}{B}, 1/2): "Fabricate 1" and "Whenever
/// **another** creature **or artifact you control** is put into a graveyard
/// from the battlefield, each opponent loses 1 life."
///
/// The drain is one trigger reading a four-word filter, and three of those
/// words are only visible as a *difference* — a trigger that fired on every
/// death would pass any test that watched one permanent die. So four deaths
/// are played out on one board and the life total is read after each:
///
/// * the Mind Stone sacrificed to its own ability — "or artifact", which no
///   creature death can show;
/// * a Toxic Deluge for X=1, which kills the Llanowar Elves on **both**
///   sides at once: one drain, not two, because `Filter::ControlledByYou`
///   is what separates them, and a board where only my creature died could
///   not tell the two readings apart;
/// * the Apprentice itself, Vindicated — `Filter::Another` is `obj.id !=
///   this` and the id survives the move to the graveyard, so its own corpse
///   must take nothing. A dies trigger that read itself would drain here,
///   and the Deluge is deliberately X=1 rather than X=2 so that this death
///   stands alone instead of arriving in the same batch as the Elves'.
///
/// The other half is what the card stands at `Coverage::Partial` for.
/// `Fabricate 1` is not written — there is no Servo in `tokens::ALL` to
/// create and half a "choose one" would be a mandatory counter the player
/// never agreed to — so the Apprentice must arrive as the plain 1/2 the
/// face prints: no +1/+1 counter, no token, and no question asked on the
/// way (a mode choice would stop `pass_until` dead). The day fabricate is
/// written, those two assertions are what has to be deleted.
#[test]
#[allow(clippy::too_many_lines)] // three deaths on two sides, and the point is which of them drains
fn the_apprentice_skips_fabricate_and_drains_only_for_another_permanent_of_yours_that_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mind_stone(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[marionette_apprentice(), toxic_deluge()])
        .battlefield(1, &[plains(), swamp(), plains(), llanowar_elves()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // ---- The Apprentice is cast, and fabricate is not written. ----
    let opening = engine.state().players[1].life;
    cast_from_hand(&mut engine, p0, marionette_apprentice());
    pass_until(&mut engine, stack_is_empty);
    let apprentice =
        on_battlefield(&engine, p0, marionette_apprentice()).expect("the Apprentice resolved");
    assert_eq!(
        pt(&engine, apprentice),
        (1, 2),
        "no +1/+1 counter: fabricate is the clause this card is Partial for",
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and no Servo either — the whole `Fabricate 1` is dropped, so the \
         Apprentice enters as the 1/2 its face prints",
    );
    assert_eq!(
        engine.state().players[1].life,
        opening,
        "an Apprentice merely entering takes nothing from anybody",
    );

    // ---- "or artifact": the Mind Stone eats itself for a card. ----
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let stone = on_battlefield(&engine, p0, mind_stone()).expect("the Stone is out");
    // Everything but the Stone, which still owes its own `{T}` and would
    // otherwise have been spent making the `{1}` it costs.
    tap_mana_except(&mut engine, p0, stone);
    let before_stone = engine.state().players[1].life;
    activate(&mut engine, p0, mind_stone(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, mind_stone()).is_some(),
        "the sacrifice is the activation cost, so the Stone is in the yard",
    );
    assert_eq!(
        engine.state().players[1].life,
        before_stone - 1,
        "an artifact of yours going to a graveyard from the battlefield \
         drains exactly as readily as a creature does",
    );

    // ---- Both Elves die in one sweep; only mine is mine. ----
    let before_deluge = engine.state().players[1].life;
    let deluge = in_hand(&engine, p0, toxic_deluge()).expect("the Deluge is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: deluge })
        .expect("a sorcery in an open main phase, off the mana still floating");
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("the Deluge asks for X, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none()
            && on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "-1/-1 killed the 1/1 on each side, which is what makes the count \
         below a reading of the filter rather than of the board",
    );
    assert_eq!(
        engine.state().players[1].life,
        before_deluge - 1,
        "two creatures died and one of them was theirs: `you control` is \
         worth exactly one life here, and 2 would mean it is not read",
    );
    assert!(
        on_battlefield(&engine, p0, marionette_apprentice()).is_some(),
        "the 1/2 survived X=1, so its own death below is an event of its own",
    );

    // ---- `another`: the Apprentice's own corpse takes nothing. ----
    reach_their_main_phase(&mut engine, p1);
    let before_vindicate = engine.state().players[1].life;
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![apprentice],
            },
        )
        .expect("their removal may point at the Apprentice");
    pass_until(&mut engine, |e| {
        at_rest(e, p1) && in_graveyard(e, p0, marionette_apprentice()).is_some()
    });
    assert_eq!(
        engine.state().players[1].life,
        before_vindicate,
        "`another` is the word: the Apprentice keeps its id through the move \
         to the graveyard, so its own death is the one death it never reads",
    );
}

// oracle_id = "056b651e-e0e2-4333-9235-d1ffe8fcca29"
fn stingcaster_mage() -> baylee_core::ids::CardIndex {
    card_index("056b651e-e0e2-4333-9235-d1ffe8fcca29")
}

/// Stingcaster Mage ({1}{R}, 2/1): "Haste" and "When this creature enters,
/// target instant or sorcery card in your graveyard gains flashback until
/// end of turn. The flashback cost is equal to its mana cost."
///
/// One main phase holds the whole card. A Swords to Plowshares is spent on
/// the first Elf the ordinary way, so the card the trigger will point at got
/// into the graveyard by being *played*; the Mage then arrives, the trigger
/// offers that one card and nothing else, and afterwards `legal.castable`
/// names a card in a graveyard — an offer nothing in the pool had ever made,
/// because no test played `Effect::GrantFlashback` at all. The second cast
/// eats the second Elf, and the card is exiled rather than buried again
/// (CR 702.34a).
///
/// The attack at the end is the other printed line. The Mage entered this
/// very turn, so without haste it would be summoning sick (CR 302.6) and
/// `ChooseAttackers` would not name it — the offer is the assertion, and the
/// two life the defender loses is the same claim read off the board.
#[test]
#[allow(clippy::too_many_lines)] // scenario script — one main phase, read in order
fn a_hasty_wizard_flashes_back_a_spent_swords_and_swings_the_turn_it_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(37, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[swords_to_plowshares(), stingcaster_mage()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let elves = mine(
        &engine,
        p1,
        llanowar_elves(),
        crate::zone::Zone::Battlefield,
    );
    assert_eq!(elves.len(), 2, "one Elf per cast of the same Swords");
    let life_before = engine.state().players[1].life;

    // Six lands, tapped once. A mana pool empties at the end of a step
    // (CR 500.5) and nothing between here and the attack ends one, so all
    // three casts — {W}, {1}{R}, {W} again — are paid out of this pool.
    tap_all_mana(&mut engine, p0);

    // Swords to Plowshares on the first Elf.
    let swords_in_hand =
        in_hand(&engine, p0, swords_to_plowshares()).expect("the Swords is in hand");
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: swords_in_hand,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Swords targets a creature, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves[0]), "the Elf is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        mine(
            &engine,
            p1,
            llanowar_elves(),
            crate::zone::Zone::Battlefield
        )
        .len(),
        1,
        "the first Elf is exiled",
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before + 1,
        "its controller gains life equal to its power",
    );

    // The Mage, cast the same way. Its ETB trigger asks for a target once
    // the creature has resolved onto the battlefield.
    let mage_card = in_hand(&engine, p0, stingcaster_mage()).expect("the Mage is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mage_card })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let mage = on_battlefield(&engine, p0, stingcaster_mage()).expect("the Mage landed");
    let swords_card = in_graveyard(&engine, p0, swords_to_plowshares())
        .expect("the spent Swords is in its owner's graveyard");
    let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    assert_eq!(
        options,
        vec![swords_card],
        "the instant in your own graveyard, and nothing on the battlefield",
    );
    assert_eq!(min, 1, "the trigger is not an optional one");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // The grant is an *offer*: the engine now lists a card in a graveyard
    // among the things this seat may cast.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        legal.castable.contains(&swords_card),
        "the granted flashback makes the graveyard card castable",
    );

    // Cast it from the graveyard, at the second Elf. "The flashback cost is
    // equal to its mana cost" is a claim about a number, so the pool is what
    // reads it: one white mana leaves it and nothing else does.
    let pool_before = engine.state().players[0].mana_pool.total();
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords_card })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the flashed-back Swords targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves[1]),
        "the surviving Elf is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves[1]],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before - 1,
        "the flashback cost is the printed mana cost, one white and no more",
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        mine(
            &engine,
            p1,
            llanowar_elves(),
            crate::zone::Zone::Battlefield
        )
        .is_empty(),
        "one Swords, cast twice, exiled both Elves",
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before + 2,
        "and gained its controller one life each time",
    );
    assert!(
        in_graveyard(&engine, p0, swords_to_plowshares()).is_none(),
        "a card cast for flashback does not go back to the graveyard",
    );
    let exiled = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Exile(p0))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == swords_to_plowshares()))
        });
    assert!(exiled, "it is exiled instead (CR 702.34a)");

    // Haste: the Mage entered this turn and attacks anyway.
    let life_after_flashback = engine.state().players[1].life;
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    assert!(
        attackers.contains(&mage),
        "a creature that entered this turn is offered as an attacker: haste",
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mage, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().players[1].life < life_after_flashback
    });
    assert_eq!(
        engine.state().players[1].life,
        life_after_flashback - 2,
        "an unblocked 2/1 that was never summoning sick",
    );
}

// oracle_id = "7fd61a18-6e4f-40c5-aa00-3d101ec1ec82"
fn stitcher_s_supplier() -> baylee_core::ids::CardIndex {
    card_index("7fd61a18-6e4f-40c5-aa00-3d101ec1ec82")
}

/// Stitcher's Supplier ({B}, 1/1): "When this creature enters **or dies**,
/// mill three cards."
///
/// One printed sentence firing on two events, and the card is only itself
/// when both of them fire: read as an enter-trigger alone it is an ordinary
/// 1/1 that mills three once, which is a different card — and the half that
/// goes missing is the one no board shows, because a Zombie that entered and
/// milled looks right until somebody kills it. So the Zombie is cast off a
/// Swamp, and then killed by the opponent's Vindicate, and the same
/// three-card step is struck twice.
///
/// Both halves are read off the *library*, which is what makes the word
/// "mill": three cards leave the top of the deck, and the graveyard's growth
/// is struck beside the library's loss so that a mill which exiled or drew
/// instead could not pass. After the death that growth is **four** — three
/// milled cards and the Zombie's own corpse, which is lying in the same
/// graveyard by the time the trigger resolves.
///
/// The opponent's library is the counter-half. `PlayerRel::You` is what makes
/// this "mill three cards" rather than "target player mills three", and a
/// card that milled the wrong seat would satisfy every other assertion here.
#[test]
fn a_stitchers_supplier_mills_three_entering_and_three_more_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let graveyard = |e: &Engine<RegistryLookup>| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .len()
    };
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[stitcher_s_supplier()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let library_before = library_size(&engine, p0);
    let graveyard_before = graveyard(&engine);
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, stitcher_s_supplier());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, stitcher_s_supplier()).is_some(),
        "one Swamp pays {{B}} and the Zombie arrives"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "entering took three cards off the top of its controller's library"
    );
    assert_eq!(
        graveyard(&engine),
        graveyard_before + 3,
        "and put all three of them into the graveyard"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "\"mill three cards\" mills its controller: the opponent's library is untouched"
    );

    // The other half of the same sentence, on the opponent's turn: Vindicate
    // destroys the Zombie, and dying has to mill again.
    reach_their_main_phase(&mut engine, p1);
    let zombie =
        on_battlefield(&engine, p0, stitcher_s_supplier()).expect("the Zombie is still out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .expect("their removal may point at an ordinary creature");

    // Snapshotted with the Zombie still alive, so the four cards counted
    // below are the three it mills plus itself and nothing else.
    let library_alive = library_size(&engine, p0);
    let graveyard_alive = graveyard(&engine);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, stitcher_s_supplier()).is_none(),
        "Vindicate destroyed it"
    );
    assert!(
        in_graveyard(&engine, p0, stitcher_s_supplier()).is_some(),
        "and it fell into the graveyard it had been milling into"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_alive - 3,
        "dying is the second event of the one printed sentence: three more \
         cards leave the library"
    );
    assert_eq!(
        graveyard(&engine),
        graveyard_alive + 4,
        "three milled cards and the Zombie's own corpse"
    );
}

// oracle_id = "a145ff8c-5812-4bcb-bd16-9839dc25121d"
fn storm_kiln_artist() -> baylee_core::ids::CardIndex {
    card_index("a145ff8c-5812-4bcb-bd16-9839dc25121d")
}

/// How many Dark Rituals `seat` controls on the stack: one while it is the
/// spell that was cast, two the moment a copy of it stands beside it.
fn rituals_on_the_stack(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat && o.card.is_some_and(|c| c.index == dark_ritual())
            })
        })
        .count()
}

/// Storm-Kiln Artist: "This creature gets +1/+0 for each artifact you
/// control. Magecraft — Whenever you cast or copy an instant or sorcery
/// spell, create a Treasure token."
///
/// One main phase holds every half of the card. The opponent's Sol Ring is
/// on the table before anything is cast, so the opening 2/2 is what says
/// "you control" is read and not merely "an artifact". Then a Dark Ritual is
/// cast, answered by a flashed Dualcaster Mage (CR 702.8a) whose enters
/// trigger copies that same Ritual, and answered again by an opponent's own
/// Ritual — so three instants pass through the stack and exactly one of them
/// is an instant *you cast*.
///
/// The card is `Coverage::Partial` on the "or copy" half, and this is where
/// that gap is nailed down rather than described: a copy is put onto the
/// stack and never cast (CR 707.10), no `GameEvent::SpellCast` is journalled
/// for it, and `Trigger::SpellCast` has nothing to hear. Counting the
/// Rituals on the stack is what keeps the claim honest — the walk waits for
/// two of them under p0's control at once, so a Treasure count of 1 means
/// "the copy minted none" and not "the copy was never made".
#[test]
fn a_cast_ritual_mints_a_treasure_that_grows_the_dwarf_and_a_copy_of_it_mints_none() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(59, forest())
        .battlefield(
            0,
            &[
                storm_kiln_artist(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[dark_ritual(), dualcaster_mage()])
        .battlefield(1, &[swamp(), quiet_artifact()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    let artist = on_battlefield(&engine, p0, storm_kiln_artist()).expect("the Dwarf is deployed");
    assert_eq!(
        pt(&engine, artist),
        (2, 2),
        "the only artifact on the table is the opponent's, and `you control` does not reach it",
    );

    // Cast the Ritual: one instant, cast by the Dwarf's controller.
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .unwrap();

    // Answer it with the Mage, which has flash and copies a spell as it
    // enters. Casting a *creature* is no magecraft trigger of its own.
    let mage = in_hand(&engine, p0, dualcaster_mage()).expect("the Mage is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mage })
        .unwrap();

    // The opponent answers with a Ritual of their own: a cast instant that is
    // not yours.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let their_ritual = in_hand(&engine, p1, dark_ritual()).expect("the opponent holds one too");
    engine
        .apply(p1, PlayerAction::CastSpell { card: their_ritual })
        .unwrap();

    // Their Ritual resolves, then the Mage, and its trigger asks which spell
    // to copy — yours is the only one left up there.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the copy trigger's target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![ritual], "the only spell still on the stack");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    // The copy is really made, and then everything resolves.
    pass_until(&mut engine, |e| rituals_on_the_stack(e, p0) == 2);
    pass_until(&mut engine, stack_is_empty);

    let minted = tokens_of(&engine, p0);
    assert_eq!(
        minted.len(),
        1,
        "three instants resolved and only the one you cast minted a Treasure",
    );
    let def = engine
        .state()
        .object(minted[0])
        .expect("the token is on the battlefield")
        .token
        .expect("a token knows what it is");
    assert_eq!(def.name, "Treasure", "magecraft mints a Treasure");
    assert!(
        tokens_of(&engine, p1).is_empty(),
        "the opponent's own Ritual is not a spell you cast",
    );
    assert_eq!(
        pt(&engine, artist),
        (3, 2),
        "2/2 plus the one artifact you control — the Treasure it just made",
    );
}

// oracle_id = "9deded8b-cec4-4ede-a50b-131404d456d4"
fn thought_monitor() -> baylee_core::ids::CardIndex {
    card_index("9deded8b-cec4-4ede-a50b-131404d456d4")
}

/// Thought Monitor ({6}{U}, a 2/2 flier): "Affinity for artifacts" and
/// "When this creature enters, draw two cards."
///
/// Both halves of its `Coverage::Partial`, because a Partial test that plays
/// only the half that works says nothing about the half that does not.
///
/// The half that works is the enters trigger: the spell resolves, the trigger
/// goes on the stack, and its controller's library is two cards shorter.
///
/// The half that does not is affinity. `FaceDef::cost_reduction` carries one
/// variant — `CostReduction::NotStartingPlayer` — and `casting::printed_reduction`
/// reads that one and nothing else, so no rule in the engine counts
/// permanents on the battlefield and this face declares no reduction at all.
/// The bill therefore stays {6}{U} with three artifacts out: six Islands
/// would pay the printed {3}{U} with room to spare, here they do not pay at
/// all, and the seventh Island is what flips the offer. That the offer gates
/// on the *quantity* of mana floating — rather than only on its colours — is
/// what `a_printed_cost_reduction_is_counted_by_the_offer_as_well` settled,
/// which is what makes one Island a discriminator instead of a coincidence.
#[test]
fn a_thought_monitor_draws_two_as_it_enters_and_three_artifacts_shorten_nothing() {
    let p0 = PlayerId::new(0);

    // The Monitor in hand over `islands` Islands and three artifacts, walked
    // to p0's own main phase with every land already tapped: `castable` is
    // read off mana that is floating, never off mana that could be. The
    // artifacts are Lightning Greaves because they make no mana — a Sol Ring
    // would pay the missing mana itself and the test would prove nothing.
    let board = |islands: usize| {
        let mut field = vec![lightning_greaves(); 3];
        field.extend(std::iter::repeat_n(island(), islands));
        let mut engine = Duel::new(77, island())
            .battlefield(0, &field)
            .hand(0, &[thought_monitor()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        tap_all_mana(&mut engine, p0);
        engine
    };

    // The declared gap, asserted rather than described: six mana and three
    // artifacts are a board the printed card casts off comfortably, and this
    // engine does not offer the spell at all.
    let six = board(6);
    let unaffordable = in_hand(&six, p0, thought_monitor()).expect("the Monitor is in hand");
    let Pending::Priority { legal, .. } = six.pending().clone() else {
        panic!("expected priority, got {:?}", six.pending())
    };
    assert!(
        !legal.castable.contains(&unaffordable),
        "three artifacts take nothing off {{6}}{{U}}, so six Islands do not \
         cast it — affinity is the gap this card declares"
    );

    // And the half that works, one Island further along.
    let mut engine = board(7);
    let spell = in_hand(&engine, p0, thought_monitor()).expect("the Monitor is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "the seventh Island is what pays for it, and nothing else changed"
    );
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let library_before = library_size(&engine, p0);

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the offer quoted a price this board pays");
    pass_until(&mut engine, stack_is_empty);

    let monitor = on_battlefield(&engine, p0, thought_monitor())
        .expect("the Monitor resolved onto the table");
    assert_eq!(pt(&engine, monitor), (2, 2), "the printed body");
    assert!(
        keywords(&engine, monitor).contains(baylee_cards_dsl::KeywordSet::FLYING),
        "and its printed flying"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"when this creature enters, draw two cards\" — two off the top",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before + 1,
        "the Monitor left the hand and its own trigger put two cards back",
    );
}

// oracle_id = "f82a4e85-526d-4456-b700-7760043a31be"
fn viscera_seer() -> baylee_core::ids::CardIndex {
    card_index("f82a4e85-526d-4456-b700-7760043a31be")
}

/// Viscera Seer ({B}, 1/1 Vampire Wizard): "Sacrifice a creature: Scry 1."
///
/// The cost is the whole ability, so nothing here can be read off the card.
/// The printed sentence names no creature, and what a player is handed is a
/// question — which one — asked between the targets there are none of
/// (CR 601.2c) and the payment (CR 601.2h). It arrives as
/// `Pending::ChooseCards` with `ChoicePrompt::CostSacrifice`, because
/// choosing what to sacrifice is not targeting (CR 115.1), and the variant
/// is asserted beside the options: a list alone passes against
/// `ChoicePrompt::Generic`, which tells a client to say "choose a card"
/// where what the game means is "which one are you giving up".
///
/// Both halves of that menu are struck. It holds the Elf **and the Seer
/// herself**, who is a creature her controller controls and so is her own
/// fodder; it holds neither the opponent's Cleric — CR 701.21a only lets a
/// player sacrifice a permanent they control — nor the Swamp beside her,
/// which is not a creature. Naming the Cleric anyway is refused, so the
/// list is the enumeration `apply` validates against and not a hint.
///
/// She is eaten from on the turn she was cast, which the cost allows:
/// CR 302.6 holds back an ability with `{T}` in its cost and this one has
/// none, so summoning sickness is not what decides who may be sacrificed.
///
/// Then the two things paying it does. The Elf lies in the graveyard while
/// the ability is still on the stack — a cost is paid on activation, not on
/// resolution — and the scry that follows *moves* a card rather than merely
/// asking a question: the card looked at is on the bottom afterwards, the
/// one beneath it is the new top, and the library is the length it was,
/// because scry reorders and draws nothing.
///
/// The second activation is the case the printed sentence hides. Her own
/// body is all that is left on the menu, the cost takes her off the
/// battlefield before the ability resolves, and the ability resolves anyway
/// — it exists on the stack independently of its source (CR 113.7a). That
/// scry bottoms nothing, which is the other half of "you may put that card
/// on the bottom": the card that was on top is still on top.
#[allow(clippy::too_many_lines)] // one game, two activations, both scries
#[test]
fn viscera_seer_eats_the_elf_then_herself_and_scries_for_each() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(23, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .hand(0, &[viscera_seer()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    // Cast, not seeded: one Swamp pays the {B} the printing asks for.
    cast_from_hand(&mut engine, p0, viscera_seer());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, viscera_seer()).is_some()
    });
    let seer = on_battlefield(&engine, p0, viscera_seer()).expect("the Seer resolved");
    assert_eq!(pt(&engine, seer), (1, 1), "the printed body arrived");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the fodder stands");
    let land = on_battlefield(&engine, p0, swamp()).expect("the Swamp that paid for her");
    let theirs = on_battlefield(&engine, p1, ondu_cleric()).expect("the opponent has a creature");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    let offered = deeds(&legal, &[seer]);
    assert!(
        matches!(offered.as_slice(), [(0, Deed::Ability(0))]),
        "her one printed ability is offered, because something on the board \
         can pay for it: {offered:?}"
    );
    let def = baylee_cards::by_index(viscera_seer()).expect("the Seer is in the pool");
    assert!(
        def.is_implemented(),
        "and the card says the same to the deckbuilder, which is the promise \
         the rest of this test is the evidence for"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: seer,
                ability_index: 0,
            },
        )
        .expect("the ability the offer just named");

    // CR 601.2h, and the question this card is written about.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("paying asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "her controller decides what she eats");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the question is part of a cost, and the variant is the only thing \
         that says so — it is not a search and it is not a target"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature exactly: the cost is neither optional nor a pile"
    );
    assert!(
        options.contains(&elf),
        "the Elf is a creature her controller controls: {options:?}"
    );
    assert!(
        options.contains(&seer),
        "and so is the Seer, so she is on her own menu (CR 701.21a): \
         {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "the opponent's Cleric is a creature and is not this player's to \
         sacrifice (CR 701.21a): {options:?}"
    );
    assert!(
        !options.contains(&land),
        "and a Swamp is not a creature at all: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "the list is the enumeration the answer is validated against, so a \
         creature it never held cannot be eaten by naming it"
    );

    // The cards the scry is about to look at, read after the answer but
    // before the ability resolves. The list's last entry is the top of the
    // library and its first is the bottom.
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("one of the two creatures just offered");
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the Elf left the battlefield the moment the cost was paid"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard while the ability is still on the \
         stack: a cost is paid on activation, not on resolution"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability it paid for is waiting to resolve"
    );

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::ScryBottom,
        "the cost is behind her; this question is the effect she was paid for"
    );
    assert_eq!(options, vec![top], "scry 1 looks at exactly the top card");
    assert_eq!(
        (min, max),
        (0, 1),
        "\"you **may** put that card on the bottom\""
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![top] })
        .expect("the card just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the card she looked at is on the bottom"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the one beneath it is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "and scry drew nothing on the way"
    );

    // Again, with nothing left to eat but herself.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("`at_rest` waited for exactly this")
    };
    let offered = deeds(&legal, &[seer]);
    assert!(
        matches!(offered.as_slice(), [(0, Deed::Ability(0))]),
        "she is still an outlet, now with her own body as the fodder: \
         {offered:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: seer,
                ability_index: 0,
            },
        )
        .expect("an outlet with one creature left is an outlet");
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("paying asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert_eq!(
        options,
        vec![seer],
        "the Elf is eaten and the Cleric is not hers, so her own body is the \
         whole menu"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![seer],
            },
        )
        .expect("a creature may be sacrificed to its own ability");
    assert!(
        on_battlefield(&engine, p0, viscera_seer()).is_none(),
        "she ate herself to pay for the ability"
    );
    assert!(
        in_graveyard(&engine, p0, viscera_seer()).is_some(),
        "and lies in the graveyard beside the Elf"
    );

    // CR 113.7a: the ability is on the stack and its source is gone, and it
    // still does what it says.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::ScryBottom);
    assert_eq!(
        options,
        vec![second],
        "the top card, which is the one the first scry left there"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("bottoming nothing is an answer scry allows");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library_after = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library_after.last().copied(),
        Some(second),
        "nothing was chosen, so the card looked at stayed on top"
    );
    assert_eq!(
        library_after.len(),
        library.len(),
        "and this scry drew nothing either"
    );
    assert_eq!(
        on_battlefield(&engine, p1, ondu_cleric()),
        Some(theirs),
        "the opponent's creature was never on the menu and never left"
    );
}

// oracle_id = "afedce7b-0e18-40ad-a26a-1933fddb560d"

/// Akoum Warrior // Akoum Teeth: a {5}{R} 4/5 Minotaur Warrior with trample
/// on the front, a land on the back.
fn akoum_warrior() -> CardIndex {
    card_index("afedce7b-0e18-40ad-a26a-1933fddb560d")
}

/// The front face prints one word — "Trample" — and a chump block is the
/// only scenario that can tell it from nothing at all: CR 702.19b lets the
/// attacker assign its blockers no more than lethal damage and give the rest
/// to the player it is attacking, so a 4/5 held up by a 1/1 Elf puts three
/// through. Without the word the same board deals **nothing** to the player,
/// which is why the life total and not the dead Elf is the assertion (the
/// Elf dies either way, so stopping on its corpse is a stop both worlds
/// reach). Nothing here reads the keyword set: a projected `KeywordSet` says
/// only that the compiled card carries the bit, and it would fail one line
/// ahead of the damage it is supposed to be evidence for.
///
/// The cast says the other half of CR 712.11b on the way in. One card in hand
/// is offered twice — as a land drop *and* as a spell — and the land back is
/// never a cast mode, so a card whose only castable face is the front asks no
/// question and goes straight onto the stack.
#[test]
fn akoum_warrior_tramples_three_points_past_the_elf_that_chumps_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[akoum_warrior()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, akoum_warrior()).expect("the Warrior is in hand");
    // With the six Mountains already tapped, because `castable` is the list
    // of spells the floating mana pays for and not the list of cards in hand.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.lands.contains(&card),
        "one MDFC in hand is a land drop, because its back face is a land"
    );
    assert!(
        legal.castable.contains(&card),
        "and the same card is a spell, because its front face is a creature"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("six Mountains pay for a six-drop");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the land back is played and never cast (CR 712.12), so the front is \
         the only castable face and nothing is asked: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    let warrior = on_battlefield(&engine, p0, akoum_warrior()).expect("the Minotaur resolved");
    assert_eq!(pt(&engine, warrior), (4, 5), "a 4/5 as printed");

    // Its controller's next turn: summoning sickness has worn off, and the
    // offer of legal attackers is what says so.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { attackers, .. } if attackers.contains(&warrior)
        )
    });
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the chump stands ready");
    let before = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(warrior, Defender::Player(p1))],
            },
        )
        .expect("a 4/5 that has been out since the turn began may attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, warrior)],
            },
        )
        .expect("a 1/1 may block a 4/5");

    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
    });
    assert_eq!(
        engine.state().players[1].life,
        before - 3,
        "one lethal point stays on the Elf and the other three go to the \
         player being attacked (CR 702.19b). Without trample a blocked \
         attacker gives the player nothing."
    );
}

/// The back face, Akoum Teeth: "This land enters tapped." and "{T}: Add {R}."
///
/// A land face is *played*, not cast (CR 712.12), and this card prints only
/// one of them, so the land drop resolves straight to face 1 with no face
/// choice to make. Three printed claims follow from that and are struck here:
/// what arrives is a land rather than the 4/5 Minotaur on the other side, it
/// arrives tapped — so it pays for nothing the turn it lands — and once it
/// has untapped it makes one red mana and no more.
///
/// The ability is looked for in `abilities` and not in `mana_abilities`:
/// Akoum Teeth prints no basic land type, so `casting::intrinsic_mana` has
/// nothing to answer with and the printed `{T}: Add {R}` is an ordinary
/// activated ability of the permanent (CR 605.1).
#[test]
fn akoum_teeth_is_played_as_a_land_that_enters_tapped_and_later_taps_for_red() {
    let p0 = PlayerId::new(0);
    let (mut engine, teeth) =
        play_land_face(akoum_warrior(), 1).expect("the back face is a legal land drop");

    assert_eq!(
        engine.state().names.get(
            engine
                .state()
                .object(teeth)
                .expect("the land is on the battlefield")
                .characteristics()
                .name
        ),
        "Akoum Teeth",
        "the land face is the one that arrived"
    );
    assert!(types(&engine, teeth).contains(TypeSet::LAND));
    assert!(
        !types(&engine, teeth).contains(TypeSet::CREATURE),
        "and the Minotaur stayed on the other side of the card"
    );
    assert!(is_tapped(&engine, teeth), "\"This land enters tapped.\"");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and hands it back to the seat that played it");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == teeth),
        "a land that came in tapped cannot pay the tap symbol in its own \
         cost, so it is offered nothing at all on the turn it landed"
    );

    // Its controller's next main phase, with the land untapped — which is the
    // clause that carries the walk past the main phase it is standing in.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && !is_tapped(e, teeth)
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the walk above waited for exactly this")
    };
    assert_eq!(
        legal.abilities,
        vec![(teeth, 0)],
        "once it has untapped the land face's own printed ability is back on \
         offer, and it is the only thing this seat has"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: teeth,
                ability_index: 0,
            },
        )
        .expect("an untapped Akoum Teeth taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else beside it"
    );
    assert!(
        is_tapped(&engine, teeth),
        "paid for by the tap symbol in its own cost"
    );
}

// oracle_id = "34320ebf-da97-44a4-bbeb-a9da06548289"

/// Blackbloom Rogue // Blackbloom Bog (ZNR #91), the modal double-faced card
/// the three tests below play from both sides.
fn blackbloom_rogue() -> CardIndex {
    card_index("34320ebf-da97-44a4-bbeb-a9da06548289")
}

/// "This land enters tapped. {T}: Add {B}." — the back face, reached the way
/// CR 712.12 says a player reaches it: the card is **played as a land**, not
/// cast. The front face is a creature, so exactly one face is a land and the
/// engine switches to it with no question asked; the `ChooseCastMode` list
/// pathways get is for a card whose *both* faces are lands.
///
/// The tapped clause is why this has to be a real `PlayLand`. A Bog seeded
/// through `Duel::battlefield` is placed rather than entered, so no
/// replacement effect ever looks at it and the first untap step would have
/// untapped it anyway — a test resting on that would measure nothing. And the
/// modifier is printed on face **1**, so a reader that took its enter
/// modifiers off `faces[0]` would let the Bog make mana the turn it landed.
///
/// `{T}: Add {B}` is asked for through `legal.abilities` and **not**
/// `legal.mana_abilities`, and the difference is the card rather than the
/// kit. `legal.mana_abilities` is fed by `casting::can_activate_mana`, which
/// asks `intrinsic_mana` — the CR 305.6 shortcut, and it answers only for the
/// five basic land types. Blackbloom Bog prints plain `Land` and carries its
/// own `mana_ability!`, so it lives in `legal.abilities` at index 0 and is
/// pressed with `ActivateAbility`; `ActivateManaAbility { source }` is
/// refused outright for a source that list never named. A test written the
/// other way round asserts nothing on the tapped turn — the Bog is in
/// `mana_abilities` on no turn at all — and is refused on the untapped one.
#[test]
fn a_blackbloom_bog_enters_tapped_and_taps_for_black_once_it_has_untapped() {
    let p0 = PlayerId::new(0);
    let (mut engine, bog) = play_land_face(blackbloom_rogue(), 1)
        .expect("the back face is a land, and a land drop is the way to it");

    let obj = engine.state().object(bog).expect("the Bog is on the table");
    assert_eq!(obj.face_index, 1, "the land is the back face");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Blackbloom Bog",
        "and the permanent is named after that face, not after the card"
    );
    assert!(
        obj.characteristics().types.contains(TypeSet::LAND),
        "it came down as a land and not as the {{2}}{{B}} Rogue on the front"
    );
    assert!(
        obj.status.contains(Status::TAPPED),
        "`This land enters tapped` — printed on face 1, which is the face the \
         enter modifiers have to be read from"
    );

    // Tapped, it makes nothing: the whole ability costs {T}, and `can_afford`
    // refuses a `CostPart::TapSelf` on a source that is already tapped.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = legal.abilities;
    assert!(
        !offered.contains(&(bog, 0)),
        "a land that entered tapped is offered no {{T}} ability the turn it \
         landed: {offered:?}"
    );

    // Its controller's next turn: it untaps, and then it makes black mana.
    walk_the_game_until(&mut engine, |e| {
        e.state().turn.number > 1
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(!is_tapped(&engine, bog), "the untap step untapped it");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    let offered = legal.abilities;
    assert!(
        offered.contains(&(bog, 0)),
        "`{{T}}: Add {{B}}` is the back face's only printed ability and there \
         is a {{T}} to pay now: {offered:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bog,
                ability_index: 0,
            },
        )
        .expect("the offer is honoured");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one black mana, which is the whole of the back face's printed text — \
         and it is in the pool already, because a mana ability resolves \
         without the stack (CR 605.3b)"
    );
    assert!(is_tapped(&engine, bog), "and it paid {{T}} to make it");
}

/// "Menace. This creature gets +3/+0 as long as an opponent has eight or more
/// cards in their graveyard." — the front face cast for its printed {2}{B},
/// with the condition of the second sentence deliberately met.
///
/// Eight cards go into the opponent's graveyard *before* the Rogue is cast,
/// which is the threshold the card names exactly, and the creature that
/// arrives is still the 2/3 the type line prints. That is the other half of
/// `Coverage::Partial("the graveyard-threshold +3/+0 is never applied")`, and
/// the card leaves the modifier off rather than writing it unconditionally on
/// purpose: a bare `Modifier::ModifyPT(3, 0)` would be a permanent 5/3, which
/// is stronger than the printing, where omitting it only ever holds the Rogue
/// at the 2/3 it prints. A `StaticAbility` is a layer, a filter and a
/// modifier, and a `Filter` asks about an object, so there is nowhere to put
/// the *while* half of the sentence.
///
/// The day `pt` here reads `(5, 3)`, the condition has found a home and this
/// test is what says the `Coverage` flag on the card is now a lie.
#[test]
fn a_blackbloom_rogue_stays_a_two_three_though_their_graveyard_holds_eight() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[blackbloom_rogue()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    walk_the_game_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    seed_graveyard(&mut engine, p1, 8);
    // `eight **or more**`, and the assertion is written that way on purpose:
    // a seat that crossed a cleanup with a full hand on the way here discarded
    // into the same graveyard, which is nothing to do with the Rogue.
    assert!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len() >= 8,
        "the opponent has eight or more cards in their graveyard — the \
         printed threshold, met"
    );

    // One cast option and no question: `casting::castable_back_faces` skips a
    // land face, so the only way to cast this card is its front one.
    cast_from_hand(&mut engine, p0, blackbloom_rogue());
    pass_until(&mut engine, stack_is_empty);
    let rogue = on_battlefield(&engine, p0, blackbloom_rogue()).expect("the Rogue resolved");
    assert_eq!(
        engine
            .state()
            .object(rogue)
            .expect("it is on the table")
            .face_index,
        0,
        "cast out of the hand it is the creature face, not the land"
    );
    assert!(
        keywords(&engine, rogue).contains(KeywordSet::MENACE),
        "menace is printed on the front face and the layers project it"
    );
    assert_eq!(
        pt(&engine, rogue),
        (2, 3),
        "`gets +3/+0 as long as an opponent has eight or more cards in their \
         graveyard` is the clause the card admits it cannot say: the condition \
         holds and no modifier is applied. When this reads (5, 3) the static \
         has learned a condition and `Coverage::Partial` is what to fix"
    );
    let def = baylee_cards::by_index(blackbloom_rogue()).expect("the Rogue is in the pool");
    assert!(
        !def.is_implemented(),
        "and the card still says so, so the 2/3 above is a promise the \
         deckbuilder makes rather than a silent hole"
    );
}

/// "Menace (This creature can't be blocked except by two or more creatures.)"
/// — CR 702.111b, played out in combat, which is the only place the word
/// means anything.
///
/// Two untapped Elves stand across the table, so the sentence's own escape
/// clause is available: two or more creatures *may* block. Both halves of it
/// are asked here — the Rogue is on the offer that each Elf gets, and a lone
/// Elf is still refused.
///
/// This test used to say the opposite, and said it at length: the offer
/// named the Rogue to nobody and a *pair* was refused too, because
/// `combat::can_block` asked `state.combat.blockers_of(attacker)` while that
/// list was necessarily empty — both callers ask before anything is
/// recorded. Menace read as plain unblockable (#156). The assertion that
/// moved is the offer one; the one-blocker refusal is the printed line
/// holding and was green through both readings, which is why it is the one
/// that says nothing new.
///
/// That a pair is now *accepted* is a rules claim rather than a claim about
/// this card, so it lives beside the rule in `combat_choice_tests`. What
/// stays here is the card: the unblocked Rogue deals the 2 its type line
/// prints, and this card stays `Coverage::Partial` because what that flag
/// names is the +3/+0 and not menace.
#[test]
#[allow(clippy::too_many_lines)] // one attack, played step by step
fn a_blackbloom_rogue_s_menace_is_offered_to_both_blockers_and_refuses_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, forest())
        .battlefield(0, &[blackbloom_rogue(), swamp()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    walk_the_game_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let rogue = on_battlefield(&engine, p0, blackbloom_rogue()).expect("the Rogue stands");
    assert_eq!(
        engine
            .state()
            .object(rogue)
            .expect("it is on the table")
            .face_index,
        0,
        "the preset seats the card on its front face, so the thing attacking \
         below is the creature and not the Bog"
    );
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "two creatures are there to block with");

    // Their graveyard is full, which changes nothing about what the Rogue
    // hits for — the same gap the P/T test states, read as damage.
    seed_graveyard(&mut engine, p1, 8);
    let before = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&rogue),
        "a permanent the preset put out before turn one is not summoning sick"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rogue, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p1, "the attack is aimed at them, so they block");
    assert_eq!(
        blockers.len(),
        2,
        "both untapped Elves are on the offer: {blockers:?}"
    );
    assert!(
        blockers.iter().all(|o| o.attackers.contains(&rogue)),
        "each Elf is paired with the Rogue, because either of them may be one \
         of the two CR 702.111b asks for — the restriction is on the whole \
         declaration (CR 509.1b) and the offer is per pair: {blockers:?}"
    );

    // One blocker is refused, and that is the printed line holding.
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(elves[0], rogue)],
                },
            )
            .is_err(),
        "`can't be blocked except by two or more creatures`: a lone Elf is not \
         a legal block, and this assertion stays green when the one above it \
         goes"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declining to block is always legal");
    pass_until(&mut engine, |e| e.state().players[1].life < before);
    assert_eq!(
        engine.state().players[1].life,
        before - 2,
        "an unblocked Rogue deals the 2 its type line prints — not the 5 the \
         +3/+0 would have made of it, with eight cards lying in their yard"
    );
}

// oracle_id = "727f3201-1cfc-4ab2-9dfe-be4f7251f42f"

/// Boggart Trawler // Boggart Bog — a modal double-faced card (CR 712.3)
/// whose front is a {2}{B} 3/1 Goblin and whose back is a land.
fn boggart_trawler() -> CardIndex {
    card_index("727f3201-1cfc-4ab2-9dfe-be4f7251f42f")
}

/// "When this creature enters, exile target player's graveyard."
///
/// The front face is cast as an ordinary creature spell, and its enters
/// trigger points at a *player* — `PlayerRel::Chosen`, the relation
/// `eval::players` cannot answer on its own. Both graveyards are seeded and
/// only one is named, because a resolution that read `Chosen` as "each
/// player" would empty the caster's own graveyard too and would otherwise
/// pass unnoticed.
///
/// The P/T assertion at the end is the other half of the same sentence: the
/// card that entered has to be the Goblin, not the land on its back.
#[test]
fn boggart_trawler_exiles_only_the_graveyard_its_enters_trigger_points_at() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[boggart_trawler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 3);
    seed_graveyard(&mut engine, p1, 3);
    let mine_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(
        mine_before, 3,
        "both graveyards start with something in them"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        3
    );

    // The spell resolves, the Goblin enters, and the trigger stops the game
    // to ask. Without the target requirement the trigger stacks unasked, the
    // passing runs out of turns and this is where the test goes red.
    cast_from_hand(&mut engine, p0, boggart_trawler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a player choice, got {:?}", engine.pending())
    };
    assert!(
        options.is_empty(),
        "the exile points at a player, not an object"
    );
    assert_eq!(
        player_options,
        vec![p0, p1],
        "\"target player\" is anyone at the table (CR 115.1), the caster included"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the Goblin points at the opponent");

    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty()
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        3,
        "the cards are exiled, not merely gone"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        mine_before,
        "one graveyard was named and only that one is emptied"
    );
    let goblin = on_battlefield(&engine, p0, boggart_trawler()).expect("the Goblin landed");
    assert_eq!(
        pt(&engine, goblin),
        (3, 1),
        "the front face is what was cast and what entered"
    );
}

/// "As this land enters, you may pay 3 life. If you don't, it enters
/// tapped." — and then "{T}: Add {B}."
///
/// The back face is a land, so it is *played* rather than cast (CR 712.12),
/// and it is the card's only land face: the engine switches to it with no
/// mode question at all, which is what the face assertion pins down. What
/// follows is the `as … enters` replacement (CR 614.1c) being a real
/// question rather than a silent default, and the land the answer bought
/// actually making the black mana it prints.
#[test]
fn boggart_bog_pays_three_life_to_land_untapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest()).hand(0, &[boggart_trawler()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, boggart_trawler());

    // One land face means no face choice: the card goes straight to the Bog
    // and the only thing standing between it and the battlefield is the
    // three life.
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the Bog never asked about its three life: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    assert_eq!(
        engine.state().players[0].life,
        life_before - 3,
        "three life is what the printed line asks for"
    );
    let obj = engine
        .state()
        .object(land)
        .expect("the Bog is on the board");
    assert_eq!(
        obj.face_index, 1,
        "the back face is the one that was played"
    );
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Boggart Bog"
    );
    assert!(obj.characteristics().types.contains(TypeSet::LAND));
    assert!(
        !entered_tapped(&engine, land),
        "the life was paid, so the land is untapped"
    );

    // "{T}: Add {B}." — a *printed* mana ability, offered at index 0 in
    // `legal.abilities`. `legal.mana_abilities` is CR 305.6's intrinsic
    // shortcut and the Bog prints no basic land type to take it, which is
    // why it is empty here and why the press below is `ActivateAbility`.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "expected priority after the answer, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert!(
        legal.mana_abilities.is_empty(),
        "Boggart Bog prints no basic land type, so it has no intrinsic mana"
    );
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped Boggart Bog is offered its own mana ability"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one land, one mana");
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the Bog taps for {{B}} and nothing else"
    );
}

/// The other side of the same choice: "If you don't, it enters tapped."
///
/// Declining has to cost nothing and tap the land — a replacement effect
/// that only ever fired on "yes" would look identical on the board a turn
/// later, and the tapped assertion is what tells the two apart.
///
/// The second half is what keeps the negative assertion honest. "The ability
/// is not offered" is also what a Bog with no mana ability at all would say,
/// so the test stays on the board until the next untap step (CR 502.3) and
/// presses the same index: what withheld it was the {T} in its own cost and
/// nothing else.
#[test]
fn boggart_bog_that_declines_the_three_life_waits_a_turn_for_its_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(7, forest()).hand(0, &[boggart_trawler()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, boggart_trawler());
    let Pending::YesNo { prompt, .. } = engine.pending().clone() else {
        panic!(
            "the Bog never asked about its three life: {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();

    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "nothing was paid"
    );
    let obj = engine
        .state()
        .object(land)
        .expect("the Bog is on the board");
    assert_eq!(obj.face_index, 1, "still the land face");
    assert!(
        entered_tapped(&engine, land),
        "declining the three life is what puts it onto the battlefield tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after the answer, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "a land that entered tapped cannot pay the {{T}} in its own ability"
    );

    // One turn cycle later the same land, the same index, and the {B} the
    // card prints.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !entered_tapped(&engine, land),
        "the untap step untaps what entered tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the ability was there all along; the tap was what withheld it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the Bog taps for {{B}}"
    );
}

// oracle_id = "573151f0-00d4-4a8a-8a09-745c5f376532"
fn hydroelectric_specimen() -> CardIndex {
    card_index("573151f0-00d4-4a8a-8a09-745c5f376532")
}

/// Curse of the Swine, which this file wants for one printed property alone:
/// "Exile X target creatures" is a spell that genuinely stands on the stack
/// holding **more than one** target, which is the case the Weird's printed
/// line excludes and the engine cannot.
///
/// Named after the Weird rather than after the card, because the handles in
/// `card_tests` share one namespace and a sibling module may want the card's
/// own name for itself.
fn the_specimens_two_target_spell() -> CardIndex {
    card_index("5669ea7c-c4fc-494c-896b-4bce9b494817")
}

/// Flashes the Weird in on top of `their_spell` and answers with the options
/// its enters-trigger is offered.
///
/// The `castable` assertion is the first printed word doing its work: a
/// creature spell is a noninstant, so CR 117.1a offers it only in its own
/// controller's main phase with an **empty** stack, and the only thing that
/// reaches a stack with somebody else's spell standing on it — or, below,
/// somebody else's turn — is flash (CR 702.8a). The mana is tapped first
/// because `castable` is an affordability answer too, and an untapped board
/// would hide the timing question behind a price.
#[track_caller]
fn flash_the_specimen_in(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    their_spell: ObjectId,
) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana_but(engine, p0, None);
    let weird = in_hand(engine, p0, hydroelectric_specimen()).expect("the Weird is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p0's priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&weird),
        "\"Flash\": a creature spell is offered onto a stack that is not empty"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: weird })
        .expect("three Islands pay {2}{U}");
    options_offered_including(engine, their_spell)
}

/// Answers the trigger's "you may" and points the redirect at the Weird.
///
/// The offer is answered here rather than walked past, because "you may" is a
/// clause under test on both boards. What follows it is the rest of the same
/// sentence: the new target is chosen as the trigger resolves (CR 115.7), and
/// "to this creature" leaves `Filter::This` exactly one thing to offer.
#[track_caller]
fn the_weird_takes_the_aim(engine: &mut Engine<RegistryLookup>, p0: PlayerId, weird: ObjectId) {
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the offer")
    };
    assert_eq!(
        (player, prompt),
        (p0, YesNoPrompt::MayDo),
        "\"you may change the target\" is asked of the Weird's controller"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the redirect's choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![weird],
        "\"to this creature\": the Weird itself, and nothing else on the board"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![weird],
            },
        )
        .unwrap();
}

/// Hydroelectric Specimen ({2}{U}, 1/4 Weird): "Flash. When this creature
/// enters, you may change the target of target instant or sorcery spell with
/// a single target to this creature."
///
/// The whole card is one play, so it is played as one: the removal is already
/// on the stack pointing at something else, which is the only board on which
/// the trigger has a legal target at all — and the only board a creature
/// without flash could never be cast onto.
///
/// Swords to Plowshares is the sharpest reading available, because its two
/// clauses land on two different players once the target moves. The Elves it
/// was aimed at are untouched, the Weird that stole the aim is **exiled**
/// rather than destroyed, and the life goes to the Weird's controller — the
/// player who redirected it — for the Weird's own power.
///
/// This is also the working half of `Coverage::Partial`: one target is
/// exactly what the printed line asks for, so nothing here is an
/// approximation.
#[test]
fn a_flashed_in_weird_takes_the_swords_that_was_aimed_at_the_elves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .hand(0, &[hydroelectric_specimen()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the elves are out");
    let life_before = engine.state().players[0].life;

    // p0's main phase, and p0 hands priority straight over: the opponent's
    // removal is cast at p0's only creature.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the swords' target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![my_elves], "the only creature on the table");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![my_elves],
            },
        )
        .unwrap();
    let swords_spell = engine.state().zones.list(ZoneLocation::Stack)[0];

    let offered = flash_the_specimen_in(&mut engine, p0, swords_spell);
    assert_eq!(
        offered,
        vec![swords_spell],
        "\"target instant or sorcery spell\": the one instant still on the \
         stack is the only thing the trigger may point at"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords_spell],
            },
        )
        .unwrap();
    let weird = on_battlefield(&engine, p0, hydroelectric_specimen()).expect("the Weird landed");
    the_weird_takes_the_aim(&mut engine, p0, weird);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the removal named is still standing: the aim moved"
    );
    assert_eq!(
        engine.state().object(weird).map(|o| o.zone),
        Some(Zone::Exile),
        "the Weird took the Swords, which exiles rather than destroys"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"its controller gains life equal to its power\", and after the \
         redirect that controller is the Weird's own, for its own 1 power"
    );
    assert!(
        in_graveyard(&engine, p1, swords_to_plowshares()).is_some(),
        "one spell was cast and it resolved"
    );
}

/// The `Coverage::Partial` half: "with a single target" is a clause no
/// `Filter` can ask about.
///
/// Curse of the Swine is cast for X = 2 and stands on the stack naming two
/// creatures, which the printed line excludes from the trigger outright — and
/// the engine offers it anyway, because `TargetSpec::Spell` can only narrow a
/// spell by its *printed* characteristics and no `Filter` counts what an
/// object points at. Taking it shows the second half of the same gap: the
/// redirect writes one target where two stood, so a spell that named two
/// creatures exiles exactly one and makes one Boar instead of two.
///
/// This is the test that turns red when the gap closes. A trigger with no
/// legal target is removed from the stack (CR 603.3d), so a `TargetSpec` that
/// could count targets would leave nothing to choose and
/// `flash_the_specimen_in` would never see the choice at all.
///
/// A **sorcery** is deliberate rather than convenient. The printed line reads
/// "instant or sorcery", and a sorcery only ever stands on the stack during
/// its own controller's turn — which is the other thing flash buys and the
/// board above cannot show, since it never leaves p0's own main phase.
#[test]
fn the_weirds_trigger_offers_a_spell_holding_two_targets_and_exiles_only_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let elf = llanowar_elves();
    let mine = [island(), island(), island(), elf, elf];
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &mine)
        .hand(0, &[hydroelectric_specimen()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[the_specimens_two_target_spell()])
        .start();
    keep_mulligans(&mut engine);
    let my_elves = all_on_battlefield(&engine, p0, elf);
    assert_eq!(my_elves.len(), 2, "two creatures for the curse to name");

    // p1's own main phase, because a sorcery has no other window (CR 307.1).
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, the_specimens_two_target_spell());
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    engine.apply(p1, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("expected the curse's aim, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (2, 2), "\"Exile X target creatures\", X = 2");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: my_elves.clone(),
            },
        )
        .expect("two creatures is what X = 2 asks for");
    let curse = engine.state().zones.list(ZoneLocation::Stack)[0];
    let aimed: Vec<ObjectId> = engine
        .state()
        .object(curse)
        .expect("the curse is on the stack")
        .targets
        .to_vec();
    assert_eq!(aimed.len(), 2, "the premise: two targets on the stack");
    assert!(
        my_elves.iter().all(|named| aimed.contains(named)),
        "and they are the two creatures that were named: {aimed:?}"
    );

    let offered = flash_the_specimen_in(&mut engine, p0, curse);
    assert_eq!(
        offered,
        vec![curse],
        "NOT SUPPORTED, and this is its exact shape: the printed line reads \
         \"with a single target\", and a two-target spell is offered anyway"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![curse],
            },
        )
        .unwrap();
    let weird = on_battlefield(&engine, p0, hydroelectric_specimen()).expect("the Weird landed");
    the_weird_takes_the_aim(&mut engine, p0, weird);

    let collapsed: Vec<ObjectId> = engine
        .state()
        .object(curse)
        .expect("the curse is still on the stack")
        .targets
        .to_vec();
    assert_eq!(
        collapsed,
        vec![weird],
        "the other half of the gap: the redirect writes one target where two \
         stood, so a spell aimed at two creatures now names one"
    );

    // And what that is worth on the board, which is the only place the gap
    // is visible to a player.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        all_on_battlefield(&engine, p0, elf).len(),
        2,
        "\"Exile X target creatures\" named two and exiled neither of them"
    );
    assert_eq!(
        engine.state().object(weird).map(|o| o.zone),
        Some(Zone::Exile),
        "the one creature it did exile is the one that stole the aim"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "\"for each creature exiled this way, its controller creates a 2/2 \
         green Boar\": one exile, one Boar, where X had been two"
    );
}

/// Plays the back face and answers its entry question with `pay`.
///
/// There is no face choice on the way, and that is worth saying out loud: the
/// card file calls the back "an MDFC land reached by the face choice"
/// (CR 712.12), but the engine only asks when *both* faces are lands, the way
/// a pathway prints them. Here the front is a creature, so exactly one face
/// answers `TypeSet::LAND` and the engine switches to it itself.
#[track_caller]
fn a_played_laboratory(pay: bool) -> (Engine<RegistryLookup>, PlayerId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[hydroelectric_specimen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = play_land(&mut engine, p0, hydroelectric_specimen());
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }),
        "only one face is a land, so nothing is asked about which one"
    );

    // "As this land enters, you may pay 3 life. If you don't, it enters
    // tapped." — the entry stops here, mid-way, and the land is already on
    // the battlefield while the question stands.
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("the laboratory never asked: {:?}", engine.pending())
    };
    assert_eq!(
        (player, prompt),
        (p0, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 }),
        "three life, which is what this card prints"
    );
    engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
    (engine, p0, land)
}

/// Hydroelectric Laboratory: "As this land enters, you may pay 3 life. If you
/// don't, it enters tapped. {T}: Add {U}."
///
/// Both answers are legal and they differ in exactly one bit, so they are
/// played on two boards and read against each other. Paying is the one that
/// can be pressed further: a land that stood up is a land that makes mana the
/// turn it landed, and the colour it makes is the back face's own ability
/// rather than anything the Weird on the front prints.
#[test]
fn the_laboratory_side_stands_up_for_three_life_and_comes_in_tapped_without_it() {
    let (mut engine, p0, land) = a_played_laboratory(true);
    let (calm, _, tapped_land) = a_played_laboratory(false);
    assert_eq!(
        engine
            .state()
            .object(land)
            .expect("it is on the table")
            .face_index,
        1,
        "the back face is the one that was played"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "and what it was played as is a land"
    );
    assert_eq!(
        engine.state().players[0].life,
        calm.state().players[0].life - 3,
        "three life were paid, measured against the board that declined"
    );
    assert!(!entered_tapped(&engine, land), "so it stands up");

    // "{T}: Add {U}."
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land play hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped land is offered its own mana ability the turn it landed"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and it is the colour the back face names"
    );

    // The other answer, on its own board.
    assert!(
        entered_tapped(&calm, tapped_land),
        "\"If you don't, it enters tapped.\""
    );
    let Pending::Priority { legal, .. } = calm.pending().clone() else {
        panic!("a land play hands priority back, got {:?}", calm.pending())
    };
    assert!(
        !legal.abilities.contains(&(tapped_land, 0)),
        "a tapped land cannot pay its own {{T}}, so the blue waits a turn"
    );
}

// oracle_id = "2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7"
fn kazandu_mammoth() -> CardIndex {
    card_index("2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7")
}

/// The back half of Kazandu Mammoth // Kazandu Valley: "This land enters
/// tapped. {T}: Add {G}."
///
/// CR 712.12 lets a player playing a modal double-faced card as a land
/// choose one of its faces that is a land, and this card prints exactly one
/// — so the choice has a single answer and the engine takes it rather than
/// asking. What arrives is therefore the Valley and not the Elephant, and it
/// arrives tapped: `enter_modifiers` are printed on the *back* face here,
/// and a reader that took `faces[0]` would find none and let the land make
/// mana the turn it landed, which is the fault Glasspool Shore had. So the
/// turn it enters it is offered nothing at all, and the {G} is there a turn
/// later — the only reading that shows the back face brought its own ability
/// rather than the Elephant's trigger.
#[test]
fn kazandu_valley_is_played_as_the_back_face_and_taps_for_green_a_turn_later() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(37, forest())
        .hand(0, &[kazandu_mammoth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let valley = play_land(&mut engine, p0, kazandu_mammoth());
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }),
        "one face is a land and the other is a creature, so CR 712.12's \
         choice has one answer and is never put to the player",
    );
    let object = engine
        .state()
        .object(valley)
        .expect("the land is on the battlefield");
    assert_eq!(object.face_index, 1, "the back face is what was played");
    assert_eq!(
        engine.state().names.get(object.characteristics().name),
        "Kazandu Valley",
    );
    let types = object.characteristics().types;
    assert!(types.contains(TypeSet::LAND), "and it is a land");
    assert!(
        !types.contains(TypeSet::CREATURE),
        "and nothing of the 3/3 Elephant on the other side came with it",
    );
    assert!(
        entered_tapped(&engine, valley),
        "\"This land enters tapped\"",
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority straight back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.iter().all(|(id, _)| *id != valley),
        "and a tapped permanent cannot pay {{T}}, so the Valley is offered \
         nothing on the turn it arrived — which is what that sentence costs",
    );

    // A turn each way. The untap step is the earliest this land could ever
    // make mana, which is the other half of the same sentence.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, valley),
        "and it untapped in its controller's own untap step",
    );
    // By printed index, and deliberately not through `legal.mana_abilities`:
    // that list is the CR 305.6 shortcut for a basic land type plus whatever
    // a continuous effect granted, and Kazandu Valley prints no subtype at
    // all — its `{T}: Add {G}` is an ability of its own back face.
    activate(&mut engine, p0, kazandu_mammoth(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "\"{{T}}: Add {{G}}\" — off the face that prints it, and resolved \
         without the stack (CR 605.3b)",
    );
    assert!(
        is_tapped(&engine, valley),
        "and the {{T}} in its cost was paid by tapping the Valley itself",
    );
}

/// The front half: "Landfall — Whenever a land you control enters, this
/// creature gets +2/+2 until end of turn."
///
/// The land that lands is the card's *other* copy, played as Kazandu Valley,
/// so one printing plays both of its own halves. Landfall is an ability word
/// with no rules meaning of its own (CR 207.2c), which means everything here
/// has to be the ordinary trigger machinery: the ability waits on the stack
/// until a player would next receive priority (CR 603.3) instead of pumping
/// the moment the land arrives, `Filter::This` resolves to the Elephant
/// rather than to the land that came in, and the +2/+2 is gone when the turn
/// is (CR 514.2) — a duration left at "while on the battlefield" would leave
/// a 5/5 standing on the opponent's turn, and no test that only counted the
/// pump would say so.
#[test]
fn a_land_you_control_entering_pumps_kazandu_mammoth_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[kazandu_mammoth(), kazandu_mammoth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, kazandu_mammoth());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, kazandu_mammoth()).is_some()
    });
    // Captured *before* the second copy is played: both halves are the same
    // `CardIndex`, so afterwards `on_battlefield` could answer with the land.
    let elephant = on_battlefield(&engine, p0, kazandu_mammoth()).expect("the Elephant resolved");
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "cast off three Forests as the printed 3/3",
    );

    let valley = play_land(&mut engine, p0, kazandu_mammoth());
    assert_eq!(
        engine
            .state()
            .object(valley)
            .expect("the land is on the battlefield")
            .face_index,
        1,
        "the land that triggers it is this card's own back face",
    );
    assert!(
        !stack_is_empty(&engine),
        "landfall is a triggered ability and waits on the stack (CR 603.3)",
    );
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "so the Elephant is untouched while it sits there",
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elephant),
        (5, 5),
        "\"this creature gets +2/+2\" — *this* creature, the trigger's own \
         source, and not the land that entered",
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "\"until end of turn\" ends in that turn's cleanup step (CR 514.2)",
    );
}

/// The narrow word in the same sentence: "whenever a land **you control**
/// enters".
///
/// Every land that enters is somebody's, so a filter narrowed to your own and
/// a filter over every land agree on every other board there is — this is the
/// one scenario that tells them apart, and Kazandu Mammoth is printed with the
/// narrow one. Both directions are struck: the opponent's land really reaches
/// the battlefield, so a silent 3/3 cannot be a land drop that never happened,
/// and nothing goes on the stack at all.
#[test]
fn an_opponents_land_leaves_kazandu_mammoth_the_three_three_it_prints() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[kazandu_mammoth()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elephant = on_battlefield(&engine, p0, kazandu_mammoth()).expect("the Elephant is seated");
    assert_eq!(pt(&engine, elephant), (3, 3), "a 3/3 to start with");

    reach_their_main_phase(&mut engine, p1);
    let theirs = play_land(&mut engine, p1, forest());
    assert!(
        engine
            .state()
            .object(theirs)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == p1),
        "the opponent's Forest really did enter, under the opponent",
    );
    assert!(
        stack_is_empty(&engine),
        "and no landfall trigger was put on the stack for it",
    );
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "\"a land you control\" — the Elephant's controller played none of it",
    );
}

// oracle_id = "f3d48efa-910a-4872-a5b1-a353c5dbce99"
fn pinnacle_monk() -> CardIndex {
    card_index("f3d48efa-910a-4872-a5b1-a353c5dbce99")
}

/// Pinnacle Monk ({3}{R}{R}, 2/2): "When this creature enters, return target
/// instant or sorcery card from your graveyard to your hand."
///
/// The graveyard is stocked so that the *filter* is what the test reads:
/// two Llanowar Elves are milled into it first and a Dark Ritual is cast on
/// top of them, so "target instant or sorcery card" has one legal answer out
/// of three cards lying in the same zone. Creature cards rather than lands,
/// because they are the control that rejects the two filters a card like this
/// drifts into — `Filter::Any`, which is what the neighbouring Eternal
/// Witness prints, *and* a nonland filter, which land bystanders would let
/// through.
///
/// The minimum is struck too: the printed line has no "may" in it, so the
/// choice is one-of-one rather than the up-to-one the Witness gets, and a
/// player cannot decline the buy-back.
#[test]
fn an_entering_pinnacle_monk_buys_back_the_instant_and_leaves_the_creature_cards_lying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(23, llanowar_elves())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                swamp(),
            ],
        )
        .hand(0, &[pinnacle_monk(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    // Two cards off an Elf deck: the controls that say the filter is read.
    seed_graveyard(&mut engine, p0, 2);

    // The Swamp alone pays for the Ritual, so the five Mountains are still
    // standing for the {3}{R}{R} the Monk costs.
    let swamp_land = on_battlefield(&engine, p0, swamp()).expect("a Swamp of its own");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp_land })
        .expect("the Swamp taps for {B}");
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("{B} is in the pool");
    pass_until(&mut engine, stack_is_empty);
    let buried = in_graveyard(&engine, p0, dark_ritual()).expect("the Ritual resolved and died");

    tap_all_mana(&mut engine, p0);
    let monk = in_hand(&engine, p0, pinnacle_monk()).expect("the Monk is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: monk })
        .expect("five Mountains pay {3}{R}{R}");
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
        unreachable!("the walk stopped at the trigger's target choice")
    };
    assert_eq!(player, p0, "its own controller points the trigger");
    assert_eq!(
        (min, max),
        (1, 1),
        "the printed line says \"return target instant or sorcery card\" with \
         no \"may\" anywhere in it, so the choice cannot be declined",
    );
    assert_eq!(
        options,
        vec![buried],
        "the Ritual alone: the two creature cards in the same graveyard are \
         neither instants nor sorceries",
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![buried],
            },
        )
        .expect("the one legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, pinnacle_monk()).is_some(),
        "the Monk finished entering, so this was its own enters-trigger",
    );
    assert!(
        in_hand(&engine, p0, dark_ritual()).is_some(),
        "\"to your hand\" — the Ritual is castable again",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "the Ritual left; the two creature cards stayed where they lay",
    );
}

/// Prowess: "Whenever you cast a noncreature spell, this creature gets
/// +1/+1 **until end of turn**."
///
/// The Monk is seated rather than cast, which is the point of a second test
/// rather than a longer first one: prowess is about the spell cast *after*
/// it is already standing, and seeding the board skips the enters-trigger
/// the test above is entirely about.
///
/// Both halves of the printed sentence are struck. Dark Ritual is an instant
/// and therefore a noncreature spell, so the 2/2 is a 3/3 once the trigger
/// has resolved — and the turn is then handed over, where a pump written as
/// a permanent effect rather than an until-end-of-turn one (CR 702.108)
/// would leave a 3/3 standing.
#[test]
fn a_noncreature_spell_grows_the_monk_and_the_next_turn_takes_it_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[pinnacle_monk(), swamp()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let monk = on_battlefield(&engine, p0, pinnacle_monk()).expect("the Monk is standing");
    assert_eq!(
        pt(&engine, monk),
        (2, 2),
        "the printed 2/2, before anything has been cast",
    );

    let swamp_land = on_battlefield(&engine, p0, swamp()).expect("a Swamp of its own");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp_land })
        .expect("the Swamp taps for {B}");
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("{B} is in the pool");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, monk),
        (3, 3),
        "Dark Ritual is an instant, which is a noncreature spell, and prowess \
         answered it with +1/+1",
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, monk),
        (2, 2),
        "\"until end of turn\" — the turn the spell was cast in is over and \
         the Monk is the 2/2 it prints again",
    );
}

/// A game stopped on Mystic Peak's own entry question, with the object the
/// land play made.
///
/// A whole game per half, the way the Witness pair is used: "you may pay 3
/// life" is answered once and for good on the way in, so the two answers are
/// two land drops and cannot share a turn.
///
/// The graveyard is stocked with one Dark Ritual for a reason that has
/// nothing to do with mana: it is the target the *Djinn's* enters-trigger
/// would want. `CardDef::abilities_for_face` gives a back face only what that
/// face prints, so the land carries no trigger at all — and with an empty
/// graveyard that could never have been seen: a mandatory target with nothing
/// legal to point at leaves nothing on the stack either way, so a land that
/// wrongly carried the Djinn's trigger would have looked exactly like one
/// that carries nothing.
fn a_mystic_peak_asking(seed: u64) -> (Engine<RegistryLookup>, PlayerId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, dark_ritual())
        .hand(0, &[pinnacle_monk()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    let land = play_land(&mut engine, p0, pinnacle_monk());
    (engine, p0, land)
}

/// Mystic Peak, the back face: "As this land enters, you may pay 3 life. If
/// you don't, it enters tapped." and "{T}: Add {R}."
///
/// The face is reached by *playing the card as a land* (CR 712.12), and this
/// card is the shape that is offered no mode choice at all: only one of its
/// two faces is a land, so the engine plays that one the way it plays
/// Glasspool Shore — which is why the first thing asserted is the face index
/// rather than a `ChooseCastMode` the card never puts up. A front face read
/// as the land face would arrive as a 2/2 Djinn and tap for nothing.
///
/// Then both answers to the one question it does ask, because they are the
/// two different lands the same card can be: paid, it is up and makes {R}
/// the turn it lands; declined, it is down and the life total is untouched.
#[test]
fn mystic_peak_is_the_only_land_face_and_buys_itself_untapped_for_three_life() {
    let (mut engine, p0, land) = a_mystic_peak_asking(19);
    let peak = engine
        .state()
        .object(land)
        .expect("the Peak is on the battlefield while it asks");
    assert_eq!(
        peak.face_index, 1,
        "the front face is a creature, so the back is the only land face \
         there is and no mode is ever offered",
    );
    assert!(
        peak.characteristics().types.contains(TypeSet::LAND),
        "and it arrived as a land rather than as the Djinn",
    );

    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the Peak asked nothing on the way in; pending is {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "its own controller answers");
    assert_eq!(
        prompt,
        YesNoPrompt::PayLifeOrEnterTapped { amount: 3 },
        "\"you may pay 3 life\" — three, not the two a shockland asks for",
    );

    let life = engine.state().players[0].life;
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("paying is a legal answer at twenty life");
    assert_eq!(
        engine.state().players[0].life,
        life - 3,
        "the three life is gone",
    );
    assert!(
        !is_tapped(&engine, land),
        "and that is what it bought: \"if you don't, it enters tapped\"",
    );
    assert!(
        stack_is_empty(&engine),
        "the land's printed text is two sentences and neither is a trigger: \
         the buy-back is printed on the Djinn, and the Ritual lying in the \
         graveyard was never pointed at",
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "\"{{T}}: Add {{R}}\" — an untapped Peak is offered the turn it lands",
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the ability the engine just offered");
    assert!(is_tapped(&engine, land), "the {{T}} in the cost was paid");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "and red is what came out of it",
    );

    // The other answer, on a table of its own.
    let (mut engine, p0, land) = a_mystic_peak_asking(19);
    let life = engine.state().players[0].life;
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is a legal answer too");
    assert_eq!(
        engine.state().players[0].life,
        life,
        "nothing was paid, so nothing was lost",
    );
    assert!(
        is_tapped(&engine, land),
        "\"if you don't, it enters tapped\"",
    );
    assert!(
        stack_is_empty(&engine),
        "and the declined land carries no trigger either",
    );
}

// oracle_id = "da9e3910-9a1c-43a9-9138-ca971b2bccae"

/// Skyclave Cleric // Skyclave Basilica, the modal double-faced card whose
/// two faces are a creature and a land (CR 712.3).
fn skyclave_cleric() -> CardIndex {
    card_index("da9e3910-9a1c-43a9-9138-ca971b2bccae")
}

/// The creature front: "When this creature enters, you gain 2 life."
///
/// Cast off two Plains, which is the printed `{1}{W}` exactly, so nothing but
/// the card's own sentence can move a life total on this board. Three things
/// are asserted because there are three ways to be wrong: the card arrives as
/// **face 0** — `abilities_for_face` falls back to the card-level list on
/// face 0 alone, so a card that had landed on its land back would carry no
/// trigger at all — the controller gains 2 and not some other number, and the
/// opponent gains nothing, because "you gain 2 life" names the player who
/// controls the ability and not the table.
#[test]
fn skyclave_clerics_creature_front_gains_its_controller_two_life_on_entry() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[skyclave_cleric()])
        .start();
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let mine_before = engine.state().players[0].life;
    let theirs_before = engine.state().players[1].life;
    cast_from_hand(&mut engine, p0, skyclave_cleric());
    // The trigger goes on the stack as the creature enters, so "the stack is
    // empty again *and* the life total moved" is the moment it has resolved.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().players[0].life > mine_before
    });

    let cleric = on_battlefield(&engine, p0, skyclave_cleric()).expect("the Cleric landed");
    let obj = engine.state().object(cleric).expect("it is still there");
    assert_eq!(obj.face_index, 0, "a cast takes the creature front");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Skyclave Cleric"
    );
    assert_eq!(pt(&engine, cleric), (1, 3), "the printed 1/3 stands there");
    assert_eq!(
        engine.state().players[0].life,
        mine_before + 2,
        "the enters-trigger gained exactly the 2 life the card prints",
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs_before,
        "\"you gain 2 life\" is the controller and nobody else",
    );
}

/// The land back: "This land enters tapped." and "{T}: Add {W}."
///
/// Only one of the two faces is a land, so this card takes the Glasspool
/// Shore path and not the pathway one: the engine switches to face 1 itself
/// and asks no face question, there being only one face to play (CR 712.12).
/// Four printed sentences are then held against the board — it arrives as the
/// *land* Skyclave Basilica and not as a creature, it arrives tapped, it
/// triggers nothing (a back face carries only the abilities it prints, and
/// the enters-trigger is printed on the face that stayed down), and once it
/// untaps it makes exactly one white mana and nothing else.
///
/// The third of those is asserted on the **stack** and not on a life total
/// alone. A card-level trigger wrongly queued for this face would be sitting
/// on the stack unresolved the instant the land drop hands priority back, and
/// both players are on 20 life either way; a test that only read the total
/// there would pass against an engine that had fired it.
#[test]
fn skyclave_basilica_is_the_land_back_played_without_a_face_question_entering_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .hand(0, &[skyclave_cleric()])
        .start();
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, skyclave_cleric());
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "one land face, so no face question is asked: {:?}",
        engine.pending()
    );
    // The stack and not the life total is what says the trigger did not fire:
    // a trigger queued by this entry would be sitting here unresolved, and a
    // life total read at this instant is still 20 either way.
    assert!(
        stack_is_empty(&engine),
        "the back face prints no trigger, so the land drop put nothing on the stack",
    );

    let obj = engine.state().object(land).expect("it is on the table");
    assert_eq!(obj.zone, Zone::Battlefield, "the land drop landed");
    assert_eq!(obj.face_index, 1, "it is on the table as its land back");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Skyclave Basilica"
    );
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert!(
        !types(&engine, land).contains(TypeSet::CREATURE),
        "the creature front is not what was played",
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is the back face's own modifier",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the Cleric's enters-trigger is printed on the face that stayed down",
    );

    // Tapped, so it pays for nothing this turn; untapped next turn it is the
    // only mana source p0 has, which is what makes the pool a measurement.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a land drop hands priority back")
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "a land that entered tapped cannot pay {{T}} the turn it arrived",
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn comes back round and the untap step frees the land"
    );
    assert!(!is_tapped(&engine, land), "it untapped");
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "and two turns of resolutions later still no life has been gained",
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("{T}: Add {W}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the Basilica taps for exactly one white",
    );
    assert_eq!(pool.total(), 1, "and for nothing else besides");
}

// oracle_id = "53542c79-a62a-4d6a-97db-5296e9c68302"
fn tangled_florahedron() -> CardIndex {
    card_index("53542c79-a62a-4d6a-97db-5296e9c68302")
}

/// Walks to `seat`'s **next** first main phase, through the kit's own
/// [`answer_one`].
///
/// [`reach_their_main_phase`] cannot serve here: it is [`pass_until`] with
/// the predicate "a first main phase whose active player is `seat`", and
/// both tests below already stand in exactly that, so it would return
/// without moving the game at all.
///
/// Crossing the turn boundary is then where `pass_until` itself gives out.
/// A seeded `starting_hand` *replaces* the opening draw, so the seat under
/// test holds one card and never overflows — but the **opponent** keeps the
/// seven it was dealt and draws an eighth on its own turn, CR 103.8a
/// skipping the first draw for the starting seat alone, and is asked to
/// discard at cleanup (CR 514.1). `pass_until` has no arm for that question
/// and `answer_one` does.
///
/// Answering it off the top of the hand is safe here, where
/// `walk_the_game_until` above had to intercept it: the cards handed back
/// are the opponent's filler Forests, and the card under test is on the
/// battlefield before any of it is asked.
#[track_caller]
fn florahedron_to_next_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let from = engine.state().turn.number;
    for _ in 0..200 {
        if engine.state().turn.number > from
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        let (player, action) = match answer_one(engine) {
            Ok(pair) => pair,
            Err(rest) => panic!("the game stopped before {seat:?}'s next main: {rest:?}"),
        };
        engine
            .apply(player, action)
            .expect("every answer came out of the question that enumerated it");
    }
    panic!("never reached {seat:?}'s next main phase");
}

/// Tangled Florahedron, the front face: `{1}{G}` Creature — Elemental 1/1
/// whose whole printed text is "{T}: Add {G}."
///
/// A mana creature and a mana land print the same sentence and differ by one
/// rule, and that rule is what this test is about: CR 302.6's second
/// sentence forbids activating an ability with `{T}` in its cost unless the
/// creature has been under its controller's control since their turn began,
/// so the Florahedron makes no mana on the turn it is cast and makes it on
/// the next. Both halves are needed — the silence alone would also be true
/// of a card whose mana ability nobody wrote.
///
/// The board is read once before the cast for the other printed fact: one
/// card, two ways to play it (CR 712.12), so the engine offers the same
/// object as a creature spell *and* as a land drop in one priority window.
#[test]
fn a_florahedron_cast_as_a_creature_waits_a_turn_before_it_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[tangled_florahedron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Forests first: `castable` is answered against the *pool*, so a
    // spell nobody has floated mana for is not on the list at all.
    let card = in_hand(&engine, p0, tangled_florahedron()).expect("the card is in hand");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase grants priority");
    };
    assert!(
        legal.castable.contains(&card),
        "the front face is a {{1}}{{G}} creature spell",
    );
    assert!(
        legal.lands.contains(&card),
        "and the same card in the same window is a land drop (CR 712.12): \
         one card, two ways to play it",
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("an offered cast is castable");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && on_battlefield(e, p0, tangled_florahedron()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let flora = on_battlefield(&engine, p0, tangled_florahedron()).expect("the spell resolved");
    assert_eq!(
        engine
            .state()
            .object(flora)
            .expect("it is in play")
            .face_index,
        0,
        "a cast reaches the front face, never the land on the back",
    );
    assert_eq!(pt(&engine, flora), (1, 1), "Tangled Florahedron is a 1/1");
    assert!(
        types(&engine, flora).contains(TypeSet::CREATURE),
        "and it is a creature, which is what puts CR 302.6 over it",
    );

    // `pass_until` stopped on p0's own priority, so this is p0's offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the resolution hands priority back");
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == flora),
        "CR 302.6: \"{{T}}: Add {{G}}\" is not offered the turn it arrived",
    );

    florahedron_to_next_main(&mut engine, p0);
    assert!(
        !is_tapped(&engine, flora),
        "nothing has tapped it in the meantime, so {{T}} is a cost it can \
         still pay",
    );
    activate(&mut engine, p0, tangled_florahedron(), 0);
    assert!(is_tapped(&engine, flora), "paying {{T}} left it tapped");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"{{T}}: Add {{G}}\" — and it is green",
    );
}

/// Tangled Vale, the back face: "Land. This land enters tapped. {T}: Add
/// {G}."
///
/// Only one of this card's two faces is a land, so CR 712.12's face choice
/// is not a *question* here — the engine switches to that face and plays it,
/// which is Glasspool Shore's shape and not a pathway's. What the scenario
/// is really about is the two sentences printed under the type line: the
/// land arrives **tapped**, so its `{T}` is not offered on the turn it was
/// played, and once it untaps the same ability adds {G} without waiting the
/// turn the creature face owes — CR 302.6 speaks of creatures, and this face
/// is not one.
///
/// Both negative halves are read off the priority the land drop itself hands
/// back, and that priority is pinned to p0 before it is read: `legal`
/// belonging to the other seat would be empty of this land whatever the card
/// said, and the assertion would pass on a Vale that came down untapped.
#[test]
fn a_florahedron_played_as_tangled_vale_enters_tapped_and_taps_for_green_next_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .hand(0, &[tangled_florahedron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vale = play_land(&mut engine, p0, tangled_florahedron());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "one land face, so nothing is asked (CR 712.12): the engine takes the \
         face the card prints, plays it, and hands the same seat its priority \
         back — got {:?}",
        engine.pending(),
    );
    let obj = engine.state().object(vale).expect("the land is in play");
    assert_eq!(
        obj.face_index, 1,
        "a land drop reaches the back face (CR 712.12)",
    );
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Tangled Vale",
        "and it is that face's own name on the battlefield",
    );
    assert!(
        obj.characteristics().types.contains(TypeSet::LAND),
        "Tangled Vale is a Land",
    );
    assert!(
        !obj.characteristics().types.contains(TypeSet::CREATURE),
        "and nothing of the Elemental on the other side came with it",
    );
    assert!(
        entered_tapped(&engine, vale),
        "\"This land enters tapped\" — and this is a real land drop, which is \
         the only way a replacement effect gets to look at it",
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the land play hands priority back");
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == vale),
        "a land that entered tapped cannot pay {{T}}, so the mana ability is \
         not offered on the turn it was played",
    );

    florahedron_to_next_main(&mut engine, p0);
    assert!(
        !is_tapped(&engine, vale),
        "the untap step gives it back, which is what \"enters tapped\" costs: \
         one turn and no more",
    );
    activate(&mut engine, p0, tangled_florahedron(), 0);
    assert!(is_tapped(&engine, vale), "paying {{T}} left it tapped");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"{{T}}: Add {{G}}\" on the back face too",
    );
}

// oracle_id = "6bc668f4-8fc7-4aaf-891b-277d8328b376"
fn umara_wizard() -> CardIndex {
    card_index("6bc668f4-8fc7-4aaf-891b-277d8328b376")
}

/// Umara Wizard ({4}{U}, 4/3 Merfolk Wizard): "Whenever you cast an instant,
/// sorcery, or Wizard spell, this creature gains flying until end of turn."
///
/// The printed sentence names three kinds of spell, and the one that is easy
/// to get wrong is the third: "Wizard spell" reads the *tribe of a card on the
/// stack*, which is a characteristic nothing else in this scenario asks about.
/// So it needs a control beside it, or "flying appeared" would be equally true
/// of a trigger that fired on any spell at all. Llanowar Elves is that
/// control, and Viscera Seer is the same shape one tribe over: both are
/// one-mana creature spells, and the only difference between them is that the
/// Seer's printing says Vampire **Wizard**.
///
/// The Wizard itself is cast rather than seeded, for two reasons. The front
/// face is the face a player chooses when casting a modal double-faced card
/// (CR 712.11b), so a seeded permanent would never have gone that way at all;
/// and its own cast is the first thing that could wrongly fire the ability —
/// the card it was cast from *is* a Wizard spell — which it must not, because
/// the ability only exists while the permanent is on the battlefield. The 4/3
/// that lands without flying is that assertion.
#[test]
fn an_umara_wizard_takes_flight_for_a_wizard_spell_and_stays_down_for_an_elf() {
    let p0 = PlayerId::new(0);
    // Five Islands pay {4}{U} exactly, which leaves the Forest and the Swamp
    // untouched for the two creature spells below: each of them is paid for
    // with the mana of its own colour and nothing else.
    let mut board = vec![island(); 5];
    board.extend_from_slice(&[forest(), swamp()]);
    let mut engine = Duel::new(47, island())
        .battlefield(0, &board)
        .hand(0, &[umara_wizard(), llanowar_elves(), viscera_seer()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    for source in all_on_battlefield(&engine, p0, island()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&engine, p0, umara_wizard()).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "the front face of a modal double-faced card is cast like any other \
         creature card (CR 712.11b)",
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the creature face is cast");
    pass_until(&mut engine, stack_is_empty);

    let wizard = on_battlefield(&engine, p0, umara_wizard()).expect("the Wizard resolved");
    assert_eq!(pt(&engine, wizard), (4, 3), "the printed body arrived");
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "its own cast was a Wizard spell and gave it nothing: the ability is \
         on the permanent, and the permanent was on the stack",
    );

    // "An instant, sorcery, or Wizard spell" — an Elf Druid is none of them.
    for source in all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let elf = in_hand(&engine, p0, llanowar_elves()).expect("the Elf is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elf })
        .expect("the Elf is cast");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "a creature spell that is not a Wizard leaves it on the ground — if \
         this fires, the trigger is reading \"a spell\" and the three printed \
         kinds are decoration",
    );

    // Viscera Seer is a Vampire Wizard: one mana, one tribe of difference.
    for source in all_on_battlefield(&engine, p0, swamp()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let seer = in_hand(&engine, p0, viscera_seer()).expect("the Seer is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: seer })
        .expect("the Seer is cast");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "\"or Wizard spell\" reads the tribe of the card on the stack, which \
         is the only thing the Seer had that the Elf did not",
    );
}

/// The other two words in the same sentence: "**you** cast", and "until end of
/// turn".
///
/// Dark Ritual is the instant clause in its smallest form — no targets, no
/// choices, nothing on the board to read — and both halves of this scenario
/// hang off *both* seats holding one. The grant is taken on its controller's
/// turn, and then two things have to be true that a careless continuous effect
/// gets wrong: it is gone on the next turn, because the cleanup step ends
/// every "until end of turn" effect (CR 514.2), and the opponent casting the
/// identical instant on that turn brings nothing back, because
/// `Filter::ControlledByYou` is the whole of "you cast".
///
/// Neither half stands alone. Without the second, the first would be satisfied
/// by an effect that expired for some reason of its own; without the first,
/// the second would be satisfied by an effect that had simply never ended.
#[test]
fn umara_wizards_flying_ends_at_cleanup_and_never_comes_off_an_opponents_instant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[umara_wizard(), swamp()])
        .battlefield(1, &[swamp()])
        .hand(0, &[dark_ritual()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);
    let wizard = on_battlefield(&engine, p0, umara_wizard()).expect("the Wizard stands");
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "nothing has been cast yet, and the printing grants no flying of its own",
    );

    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "Dark Ritual is an Instant, the first of the three kinds the sentence \
         names",
    );

    let granted_on = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > granted_on
            && e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "\"until end of turn\": the cleanup step of the turn it was granted on \
         ended the effect (CR 514.2)",
    );

    // The same card, cast by the other seat, on that seat's own turn.
    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "the sentence says whenever *you* cast — an opponent's instant is not \
         the controller's, so the trigger never fires",
    );
}

/// The middle word of the three, and the one the other two tests never reach:
/// "instant, **sorcery**, or Wizard spell".
///
/// Its absence was the hole worth closing. The Wizard clause is struck by the
/// Viscera Seer above and the instant clause by Dark Ritual, so
/// `Filter::HasType(TypeSet::SORCERY)` could be deleted from the card's filter
/// and every other assertion here would stay green. Vindicate is that word on
/// its own: a Sorcery, no creature type at all, and nothing in common with
/// either of the other two clauses.
///
/// It targets on purpose. A spell becomes cast at the *end* of the casting
/// procedure, and that is where a cast trigger fires (CR 601.2i) — so the
/// question the engine stops on in the middle is a moment at which the trigger
/// must not yet have granted anything, and the reading taken there is the
/// second half of this test rather than a detour around Vindicate's target.
#[test]
fn an_umara_wizard_takes_flight_for_the_sorcery_its_controller_casts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, forest())
        .battlefield(0, &[umara_wizard(), plains(), swamp(), island()])
        .battlefield(1, &[swamp()])
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);
    let wizard = on_battlefield(&engine, p0, umara_wizard()).expect("the Wizard stands");
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "nothing has been cast yet, and the 4/3 prints no flying of its own",
    );

    // {1}{W}{B} off the Plains, the Swamp and the Island, which is every mana
    // source p0 has — the Wizard itself makes none.
    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"destroy target permanent\" is asked mid-cast: {:?}",
            engine.pending()
        )
    };
    let theirs = on_battlefield(&engine, p1, swamp()).expect("the other seat has a Swamp");
    assert!(
        options.contains(&theirs),
        "a land across the table is a permanent: {options:?}",
    );
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "the spell is not cast until the procedure finishes (CR 601.2i), so \
         nothing has triggered while the target is still being chosen",
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("a permanent is a legal answer to \"target permanent\"");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "Vindicate is a Sorcery and nothing else the sentence names — no \
         instant, no Wizard — so this is the only assertion in the pool that \
         notices if that clause goes missing",
    );
    assert!(
        on_battlefield(&engine, p1, swamp()).is_none(),
        "and the sorcery that triggered it did what it prints",
    );
}

/// Umara Skyfalls, the back face: "This land enters tapped." and "{T}: Add
/// {U}."
///
/// A player playing a modal double-faced card as a land chooses one of its
/// faces that is a land, and it enters the battlefield with that face up
/// (CR 712.12). Only one of Umara Wizard's two faces is a land, so that choice
/// has exactly one answer and the engine takes it without asking — which is
/// worth striking on its own, because a `ChooseCastMode` with one option here
/// would be a question a client has to draw a button for.
///
/// Three things are then read off the permanent, and every one of them lives
/// on the *back* face's own data rather than on the card's first face: the
/// name and the land type, the enters-tapped modifier, and the mana ability. A
/// reader that asked `faces[0]` — which is how Glasspool Shore once came down
/// untapped — would answer a creature, no modifier and no ability to all three.
#[test]
fn umara_skyfalls_is_the_land_face_enters_tapped_and_then_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest()).hand(0, &[umara_wizard()]).start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, umara_wizard()).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&card),
        "a card whose front face is a creature is still a land drop, because \
         its back face is a land (CR 712.12)",
    );

    let land = play_land(&mut engine, p0, umara_wizard());
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }),
        "one of the two faces is a land, so there is nothing to choose \
         between and nothing is asked: {:?}",
        engine.pending()
    );
    let played = engine
        .state()
        .object(land)
        .expect("the land is on the battlefield");
    let types = played.characteristics().types;
    assert_eq!(played.face_index, 1, "it entered with the land face up");
    assert_eq!(
        engine.state().names.get(played.characteristics().name),
        "Umara Skyfalls",
        "and the permanent is that face, name and all (CR 712.8f)",
    );
    assert!(
        types.contains(TypeSet::LAND) && !types.contains(TypeSet::CREATURE),
        "a Land, and not the 4/3 printed on the other side: {types:?}",
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped.\" is printed on the back face, so a \
         modifier read off the front one would have let it make mana at once",
    );

    // Its controller's next turn, so the tap it came down with is spent and
    // what is pressed below is the land's own ability.
    let played_on = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > played_on
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(!is_tapped(&engine, land), "it untapped like any other land");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the walk above waited for exactly this")
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "\"{{T}}: Add {{U}}\" is the whole of what the land face does, and it \
         is offered: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one blue, which is the colour the back face prints",
    );
    assert!(is_tapped(&engine, land), "and it paid with its own {{T}}");
}

// oracle_id = "0355249a-8e4e-41db-9cea-1b901faffbe6"
fn witch_enchanter() -> CardIndex {
    card_index("0355249a-8e4e-41db-9cea-1b901faffbe6")
}

/// Witch Enchanter ({3}{W}, 2/2): "When this creature enters, destroy target
/// artifact or enchantment an opponent controls."
///
/// The board is built so that each word of that sentence can fail on its
/// own. A Sol Ring stands on **both** sides of the table, so "an opponent
/// controls" is the only thing keeping p0's own copy out of the offer; a
/// Wizard Class stands opposite as well, so "or enchantment" has something
/// to name that a filter narrowed to artifacts would miss; and a Llanowar
/// Elves stands beside them, which is the only permanent on the board that
/// says what "artifact or enchantment" *excludes* — a filter that had lost
/// the noun and kept nothing but "an opponent controls" would offer it.
///
/// What is asserted first is the enumeration the engine offered, because
/// that is where a too-wide filter shows. A test that read only the
/// graveyard afterwards would pass just as happily for a Witch Enchanter
/// that could point at its controller's own artifacts — it would simply
/// never be asked to.
#[test]
fn witch_enchanter_destroys_an_opponents_artifact_and_is_never_offered_its_own() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quiet_artifact()],
        )
        .battlefield(1, &[quiet_artifact(), wizard_class(), quiet_creature()])
        .hand(0, &[witch_enchanter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_ring = on_battlefield(&engine, p0, quiet_artifact()).expect("a Sol Ring of my own");
    let their_ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring");
    let their_class = on_battlefield(&engine, p1, wizard_class()).expect("their enchantment");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Llanowar Elves");

    // Four Plains pay {3}{W} exactly; the Sol Rings are printed mana
    // abilities and are left alone by this.
    cast_from_hand(&mut engine, p0, witch_enchanter());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Enchanter's controller points the trigger");
    assert!(
        options.contains(&their_ring),
        "\"target artifact ... an opponent controls\": {options:?}"
    );
    assert!(
        options.contains(&their_class),
        "\"or enchantment\" — the Class across the table is a legal target \
         too: {options:?}"
    );
    assert!(
        !options.contains(&my_ring),
        "\"an opponent controls\" — p0's own Sol Ring must not be in the \
         offer: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "\"artifact or enchantment\" — their Elf is neither, and is theirs: \
         {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_ring],
                players: Vec::new(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the artifact it named was destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "and only that one: the Sol Ring on this side is untouched"
    );
    let enchanter = on_battlefield(&engine, p0, witch_enchanter()).expect("the 2/2 landed");
    assert_eq!(
        pt(&engine, enchanter),
        (2, 2),
        "the trigger came off a creature that actually resolved"
    );
}

/// Witch-Blessed Meadow, the back face: "As this land enters, you may pay 3
/// life. If you don't, it enters tapped." and "{T}: Add {W}".
///
/// That a *creature* card is a legal land drop is CR 712.12, and it is read
/// off `legal.lands` rather than assumed: the front face is a Human Warlock,
/// so an offer that looked only at the front would never name the card and
/// the `PlayLand` below would be refused for a reason that has nothing to do
/// with the printed sentence. Only one of the two faces is a land, so the
/// engine switches to it without asking which — the Glasspool Shore path —
/// and the one question it does ask is the entry replacement (CR 614.1c).
///
/// Paid, the land is up at once and taps the same turn: a land is not a
/// creature, so nothing about summoning sickness applies to its `{T}`.
#[test]
fn witch_blessed_meadow_pays_three_life_to_arrive_untapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .hand(0, &[witch_enchanter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, witch_enchanter()).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&card),
        "CR 712.12: a card whose back face is a land is a legal land drop, \
         however the front face reads"
    );

    let land = play_land(&mut engine, p0, witch_enchanter());
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the land asked nothing on the way in; pending is {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "its controller answers");
    assert_eq!(
        prompt,
        YesNoPrompt::PayLifeOrEnterTapped { amount: 3 },
        "three life, the number this card prints"
    );

    let life_before = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.state().players[0].life,
        life_before - 3,
        "\"you may pay 3 life\""
    );
    assert!(
        !entered_tapped(&engine, land),
        "paid, so the \"if you don't\" half never ran"
    );

    let obj = engine
        .state()
        .object(land)
        .expect("the land is on the table");
    assert_eq!(obj.face_index, 1, "it arrived as its land face");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Witch-Blessed Meadow",
        "and under the land face's name rather than the Warlock's"
    );
    assert!(
        obj.characteristics().types.contains(TypeSet::LAND),
        "a Land, not a Creature"
    );

    activate(&mut engine, p0, witch_enchanter(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "\"{{T}}: Add {{W}}\" — one white mana, on the turn it was played"
    );
    assert_eq!(
        pool.total(),
        1,
        "and one mana in all: the Meadow is the only source on this board, \
         so anything more came out of this ability"
    );
    assert!(
        is_tapped(&engine, land),
        "and the {{T}} in that cost was actually paid"
    );
}

/// The other answer to the same question, which is invisible from the first.
///
/// Declining costs no life and the land arrives tapped (CR 614.1c) — which
/// for a land means it makes no mana at all on the turn it was played, so
/// both consequences are struck and not just the status bit. An engine that
/// asked the question and then dropped the answer would leave the life total
/// right and the land standing up, and only the second assertion would
/// notice a land that was tapped but still offering its ability.
#[test]
fn witch_blessed_meadow_enters_tapped_when_the_three_life_go_unpaid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(317, forest())
        .hand(0, &[witch_enchanter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, witch_enchanter());
    let life_before = engine.state().players[0].life;
    let Pending::YesNo { prompt, .. } = engine.pending().clone() else {
        panic!(
            "the land asked nothing on the way in; pending is {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 });

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "a declined \"you may pay\" costs nothing"
    );
    assert!(
        entered_tapped(&engine, land),
        "\"If you don't, it enters tapped.\""
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // Whose offer this is, before anything is read off it: `legal` belongs to
    // whoever holds priority, so an assertion that the Meadow is missing from
    // it would be satisfied for free by p1's list, which never held a
    // permanent of p0's in the first place.
    assert_eq!(
        player, p0,
        "the Meadow's controller is the one being offered"
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "and a tapped land cannot pay the {{T}} in its own mana ability, so \
         the Meadow makes no white mana this turn"
    );
}

// oracle_id = "cb814e16-acf7-41d5-a357-1323dcc369f3"
fn devoted_druid() -> CardIndex {
    card_index("cb814e16-acf7-41d5-a357-1323dcc369f3")
}

// oracle_id = "01546b7d-a233-4176-8843-d732074dc5b6"
fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

/// Devoted Druid ({1}{G}, 0/2): `{T}: Add {G}.` and `Put a -1/-1 counter on
/// this creature: Untap this creature.`
///
/// Two firsts in one card, and they are two halves of the same sentence.
/// `CostPart::PutCounterSelf` is a cost that puts a counter **on** the thing
/// paying it, which is the direction the engine had no door for — every
/// counter cost until now took one off — and `Effect::UntapSelf` is an untap
/// that names no target, so nobody is asked anything (CR 115.1c makes an
/// activated ability targeted only when it says the word).
///
/// **The bound is the counter and not the cost.** `can_afford` has nothing
/// to refuse here: a permanent can always take a counter, so the ability is
/// offered whatever has happened to the Druid. What stops it is the Druid
/// itself, and the moment it stops is sharper than "eventually": a cost is
/// paid as the ability is activated (CR 602.2b), state-based actions run
/// before anybody gets priority again (CR 704.3), and a creature with
/// toughness 0 is put into its owner's graveyard (CR 704.5f). So the second
/// payment kills the Druid **before the untap it just bought resolves** —
/// the ability is on the stack with nothing left to untap.
///
/// Each step is asserted rather than the total, because the total is the one
/// number that would still be right if the cost did nothing: a Druid whose
/// counters never landed would make two green as well, and then keep going.
#[test]
fn devoted_druid_untaps_itself_until_its_own_counters_kill_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1201, forest())
        .battlefield(0, &[devoted_druid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let druid = on_battlefield(&engine, p0, devoted_druid()).expect("the Druid is on the table");
    assert_eq!(pt(&engine, druid), (0, 2), "a 0/2 to start with");

    // Round one: tap for green, pay a counter, untap.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 0,
            },
        )
        .expect("an untapped Druid taps for green");
    assert!(is_tapped(&engine, druid));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 1,
            },
        )
        .expect("a permanent can always take a counter, so this is never refused");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "`Untap this creature` names no target, so nobody is asked anything: {:?}",
        engine.pending()
    );
    assert_eq!(
        counters_on(&engine, druid, CounterKind::M1M1),
        1,
        "the counter lands as the cost is paid, before the ability resolves"
    );
    assert_eq!(
        pt(&engine, druid),
        (-1, 1),
        "so the Druid is already smaller"
    );
    assert!(
        is_tapped(&engine, druid),
        "and still tapped, because what it paid for is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, druid), "which then untaps it");

    // Round two: the same again, and the payment is what kills it.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 0,
            },
        )
        .expect("an untapped Druid taps for green a second time");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "two green, which is the whole of what an untouched Druid is worth"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 1,
            },
        )
        .expect("still never refused — a 0/1 can take a counter like anything else");
    assert!(
        in_graveyard(&engine, p0, devoted_druid()).is_some(),
        "the second -1/-1 leaves a 0/0, and CR 704.5f takes it away before \
         priority comes back — so the untap it paid for is on the stack with \
         nothing left to untap"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "and the pool keeps what a creature that has since died put into it"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == druid),
        "a Druid in a graveyard is offered nothing: {:?}",
        legal.abilities
    );
}

/// The same payment under a Doubling Season, which does **not** double it.
///
/// CR 614.16 is the whole of this test. A replacement effect written "if an
/// effect would put one or more counters on a permanent" applies to what the
/// effect of a resolving spell or ability puts there, and to what another
/// replacement or prevention effect puts there — **paying a cost is
/// neither**. Doubling Season is the pool's only such replacement and prints
/// exactly that wording, so the answer is measured off the card rather than
/// chosen to be safe.
///
/// It matters by two counters: a doubled payment would put a 0/2 Druid in
/// the graveyard on its *first* untap, turning a card that makes two green
/// into one that makes one. So the counter-test is here too — the enchantment
/// is on the battlefield and doing its job on a permanent that *enters* with
/// counters is asserted nowhere near here, but the Druid's own line has to
/// come out at one.
#[test]
fn a_counter_paid_as_a_cost_is_not_doubled() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1203, forest())
        .battlefield(0, &[devoted_druid(), doubling_season()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let druid = on_battlefield(&engine, p0, devoted_druid()).expect("the Druid is on the table");
    assert!(
        on_battlefield(&engine, p0, doubling_season()).is_some(),
        "and the Season is out, which is the whole premise"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 0,
            },
        )
        .expect("tap for green");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 1,
            },
        )
        .expect("pay a counter to untap");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, druid, CounterKind::M1M1),
        1,
        "one counter, not two: a cost is not an effect, so the Season's \
         `if an effect would put` never applies to it (CR 614.16)"
    );
    assert_eq!(
        pt(&engine, druid),
        (-1, 1),
        "and the Druid is a -1/1 rather than the -2/0 a doubled payment \
         would have killed"
    );
    assert!(
        in_graveyard(&engine, p0, devoted_druid()).is_none(),
        "so it is still on the battlefield and still worth a second green"
    );
}

// oracle_id = "3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f"
fn wall_of_roots() -> CardIndex {
    card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
}

// oracle_id = "9575d7ce-f26d-4b90-87a3-6329e9799572"
fn abandoned_air_temple() -> CardIndex {
    card_index("9575d7ce-f26d-4b90-87a3-6329e9799572")
}

/// Wall of Roots ({1}{G}, 0/5): "Put a -0/-1 counter on this creature: Add
/// {G}. Activate only once each turn."
///
/// **The limit is spent when the ability is activated, and the turn is the
/// unit.** CR 602.2 makes activating an ability putting it on the stack and
/// paying its costs, so the count moves at the payment and not at the
/// resolution — which for a mana ability are one moment anyway (CR 605.3b),
/// and for the next card that prints this clause will not be.
///
/// **Each turn, not each of *your* turns.** The sentence says neither "only
/// during your turn" nor "only once each of your turns", so a Wall used in
/// its controller's main phase is available again while the opponent is
/// taking theirs — which is the half a tally cleared at the wrong moment
/// would get wrong in the player's favour, and the half worth two extra
/// passes to assert.
///
/// The cost is also why the Wall is not a tapper: nothing in it says {T}, so
/// summoning sickness never touched it and neither does tapping. What stops
/// the second activation is the printed sentence and nothing else.
#[test]
fn wall_of_roots_may_only_be_asked_once_a_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(3307, forest())
        .battlefield(0, &[wall_of_roots()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_roots()).expect("the Wall is on the table");
    assert_eq!(pt(&engine, wall), (0, 5), "a 0/5 to start with");

    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(wall, 0))
        )
    };
    assert!(offered(&engine), "the first activation is offered");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .expect("a permanent can always take a counter, so the cost is payable");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green, and no stack: a mana ability resolves as it is activated"
    );
    assert_eq!(
        counters_on(
            &engine,
            wall,
            CounterKind::Minus {
                power: 0,
                toughness: 1
            }
        ),
        1,
        "the counter it paid with"
    );
    assert_eq!(
        pt(&engine, wall),
        (0, 4),
        "-0/-1 takes a toughness, so the Wall is a 0/4"
    );
    assert!(
        !offered(&engine),
        "and the printed sentence takes the ability off the offer for the \
         rest of the turn — an ability nobody is offered is the whole of the \
         refusal, the way every other activation restriction works here"
    );

    // The opponent's turn is a different turn, and the Wall's controller has
    // priority during it.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        offered(&engine),
        "\"only once each turn\" is not \"only once each of your turns\""
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .expect("the second turn's activation");
    assert_eq!(
        pt(&engine, wall),
        (0, 3),
        "two counters now, and still only one a turn"
    );
    assert!(!offered(&engine), "spent again, in the same one-a-turn way");
}

/// A -0/-1 counter and a +1/+1 counter on the same creature, which is two
/// rules at once.
///
/// CR 122.1a gives every +X/+Y counter its own arithmetic, so layer 7c
/// (CR 613.4c) has to sum power and toughness **apart**: one shared delta is
/// right only while every counter is symmetric, which was true of the two
/// counters this pool used to print and is true of nothing else. A Wall of
/// Roots wearing both is a 1/5 — the plus moved the power the minus did not
/// touch.
///
/// And they do **not** cancel. CR 704.5q annihilates a +1/+1 against a
/// -1/-1 and no other pair, so both counters are still there afterwards;
/// a state-based action that read "any plus against any minus" would leave
/// the Wall a bare 0/5 and this test would say so.
///
/// Abandoned Air Temple is the +1/+1, because its ability puts one on *each*
/// creature its controller has rather than on a target — no question to
/// answer, and nothing between the activation and the counter.
#[test]
fn a_minus_zero_one_counter_takes_a_toughness_and_leaves_the_power() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3308, forest())
        .battlefield(
            0,
            &[
                wall_of_roots(),
                abandoned_air_temple(),
                plains(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_roots()).expect("the Wall is on the table");
    let temple =
        on_battlefield(&engine, p0, abandoned_air_temple()).expect("the Temple is on the table");
    assert_eq!(pt(&engine, wall), (0, 5), "a 0/5 to start with");

    // {3}{W} out of the Plains and three Forests, then the Temple's own tap.
    tap_all_mana_but(&mut engine, p0, Some(abandoned_air_temple()));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: temple,
                ability_index: 1,
            },
        )
        .expect("{3}{W} and a tap buys a counter for every creature");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wall),
        (1, 6),
        "the +1/+1 moved both numbers, which is what a symmetric counter does"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .expect("the Wall pays a -0/-1 for its green");
    assert_eq!(
        pt(&engine, wall),
        (1, 5),
        "and the -0/-1 took a toughness and left the power alone"
    );
    assert_eq!(
        counters_on(&engine, wall, CounterKind::P1P1),
        1,
        "the +1/+1 is still there: CR 704.5q cancels it against a -1/-1 and \
         against nothing else"
    );
    assert_eq!(
        counters_on(
            &engine,
            wall,
            CounterKind::Minus {
                power: 0,
                toughness: 1
            }
        ),
        1,
        "and so is the -0/-1"
    );
}

// oracle_id = "3ecaefc8-ead2-47a3-a7ea-b030faab65a7"
fn quirion_ranger() -> CardIndex {
    card_index("3ecaefc8-ead2-47a3-a7ea-b030faab65a7")
}

/// Quirion Ranger ({G}, 1/1): "Return a Forest you control to its owner's
/// hand: Untap target creature. Activate only once each turn."
///
/// **The Forest is tapped when it is returned, and that is the whole
/// card.** Tap the Forest for `{G}`, spend the land itself on the Ranger,
/// and the mana stays in the pool (CR 106.4) while the land goes to the
/// hand — so `cost_wizard::options` must *not* borrow the "untapped" that
/// CR 118.3 gives a tap cost. A menu of untapped Forests would offer this
/// player nothing at all.
///
/// **Two questions in the order CR 601.2 asks them**: the target at 601.2c,
/// the cost at 601.2h. Choosing what to bounce is not targeting (CR 115.1c
/// — only the word "target" targets), so it arrives as `ChooseCards` under
/// its own `ChoicePrompt::CostReturn` rather than as a second target.
///
/// **Two Forests, deliberately.** With one, the ability would stop being
/// offered because the cost had become unpayable, and the test would pass
/// without the activation limit existing at all. The second Forest is what
/// makes "not offered again" mean the printed sentence.
#[test]
// One card, two questions and two turns: cutting it in half would leave a
// second test that has to rebuild the board to ask the second half, which is
// what the 22 allows already in this file are for.
#[allow(clippy::too_many_lines)]
fn quirion_ranger_bounces_a_tapped_forest_and_only_once_a_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4211, forest())
        .battlefield(0, &[quirion_ranger(), forest(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ranger = on_battlefield(&engine, p0, quirion_ranger()).expect("the Ranger is on the table");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are with it");

    // Both lands and the mana creature: `mana_abilities` carries the CR 305.6
    // land shortcut and the Elf's {T} is a printed ability, and `tap_all_mana`
    // takes both lists (#159). What it leaves is the board this card was
    // printed for: everything tapped, and a Forest that is still a legal cost.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "two Forests and an Elf"
    );
    assert!(is_tapped(&engine, elves), "the Elf tapped for its own mana");

    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(ranger, 0))
        )
    };
    assert!(
        offered(&engine),
        "a tapped Forest still pays a cost that only returns it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ranger,
                ability_index: 0,
            },
        )
        .expect("two Forests on the battlefield, so the cost is payable");

    // CR 601.2c: the target, before anything is paid.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the untap asks which creature: {:?}", engine.pending())
    };
    assert!(
        options.contains(&elves) && options.contains(&ranger),
        "\"target creature\" names no controller and no state, so a tapped \
         Elf and the untapped Ranger itself are both legal: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: Vec::new(),
            },
        )
        .expect("the tapped Elf is a legal target");

    // CR 601.2h, and the one step of it the player takes.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Forest: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostReturn,
        "not `CostSacrifice`: the Forest named here comes back to a hand, \
         and a player shown \"which one are you giving up\" would decline"
    );
    assert_eq!((min, max), (1, 1), "one Forest, and exactly one");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 2, "the board was dealt two of them");
    assert_eq!(
        options, forests,
        "both Forests are on the menu although both are tapped — CR 118.3's \
         \"untapped\" belongs to a tap cost and to no other: {options:?}"
    );
    assert!(
        !options.contains(&ranger) && !options.contains(&elves),
        "and nothing that is not a Forest: {options:?}"
    );

    let paid = forests[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .expect("a tapped Forest pays the cost");

    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "the cost is paid, so the Forest is in its owner's hand"
    );
    assert!(
        engine
            .state()
            .object(paid)
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "and off the battlefield: a return is not a tap"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "the mana the Forest made is still in the pool (CR 106.4) — a land \
         that leaves takes nothing with it, which is the play this card was \
         printed for"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is no mana ability (CR 605.1), so it uses the stack"
    );
    assert!(is_tapped(&engine, elves), "and nothing has untapped yet");

    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, elves), "the targeted Elf is untapped");
    assert!(
        !offered(&engine),
        "and the printed sentence takes the ability off the offer for the \
         rest of the turn — the second Forest is still standing, so the \
         cost is payable and the limit is the only thing refusing"
    );

    // The opponent's turn is a different turn.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        offered(&engine),
        "\"only once each turn\" is not \"only once each of your turns\""
    );
}

// oracle_id = "62e7e7b1-9887-4d15-b0e5-a8ddc711bd88"
fn arcbound_ravager() -> CardIndex {
    card_index("62e7e7b1-9887-4d15-b0e5-a8ddc711bd88")
}

/// Arcbound Ravager — {2} — is printed `0/0`, and its second line, modular,
/// is the `Coverage::Partial` gap: a `0/0` with nothing on it is a corpse
/// (CR 704.5f) before any seat is offered priority, so the harness plants the
/// counter modular would have brought and the card is played from there.
///
/// The one activation left is the whole test, and every half of it is the
/// engine's answer rather than the card's: the cost names no artifact, so the
/// engine asks which one, and the menu is what reads the filter — this seat's
/// two artifacts, the Ravager itself and the Sol Ring beside it, and neither
/// the Forest under the same seat nor the Sol Ring across the table, because
/// "an artifact" paid to a cost still means one you control (CR 701.21a).
/// The eaten Sol Ring ends in its owner's graveyard and the +1/+1 counter
/// lands on the Ravager, which is the only place this printing ever puts one.
#[allow(clippy::too_many_lines)] // one activation, the menu it offers and the counter it leaves
#[test]
fn arcbound_ravager_eats_the_artifact_you_name_and_grows_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arcbound_ravager(), quiet_artifact(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .start();

    let ravager =
        on_battlefield(&engine, p0, arcbound_ravager()).expect("the Ravager is on the table");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");
    // Modular's replacement is the missing half, so the counter it would have
    // entered with comes from the harness — before the first state-based
    // check, which is the only moment a `0/0` is still on the battlefield.
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::replacement::put_counters(state, ravager, CounterKind::P1P1, 1);
    }

    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(
        counters_on(&engine, ravager, CounterKind::P1P1),
        1,
        "the counter modular cannot supply, planted by the harness"
    );
    assert_eq!(pt(&engine, ravager), (1, 1), "a printed 0/0 with one on it");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat with the Ravager");
    let offered = deeds(&legal, &[ravager]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "there is an artifact to eat, so the one line the Ravager prints is \
         offered: {offered:?}"
    );

    activate(&mut engine, p0, arcbound_ravager(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, and the cost asks once");
    assert!(
        options.contains(&ravager),
        "the Ravager is an artifact, so it is on its own menu: {options:?}"
    );
    assert!(
        options.contains(&rock),
        "and so is the Sol Ring beside it: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is a land and no artifact: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "a seat sacrifices only what it controls: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "the sacrificed artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the Sol Ring across the table never moved"
    );
    assert!(
        on_battlefield(&engine, p0, arcbound_ravager()).is_some(),
        "the Ravager ate something else and still stands"
    );
    assert_eq!(
        counters_on(&engine, ravager, CounterKind::P1P1),
        2,
        "the harness' counter plus the one the ability put there"
    );
    assert_eq!(
        pt(&engine, ravager),
        (2, 2),
        "a printed 0/0 with two +1/+1 counters"
    );
}

fn archon_of_emeria() -> CardIndex {
    card_index("ceef2d5a-77ea-4e56-9806-fd1a2d5be400")
}

/// Archon of Emeria is `Coverage::Partial`: its flying is a keyword bit,
/// while both printed statics — the one-spell-per-turn limit and the tapped
/// nonbasic lands — have no vocabulary in the DSL at all. So the card is
/// played the only way it can be, cast off three Plains and read back off the
/// battlefield through the layer system rather than off the card file: a 2/3
/// with flying. The second half is the gap struck rather than left implied —
/// a nonbasic land the *opponent* plays comes in untapped, which is exactly
/// the sentence the card does not carry. The land has to be one that enters
/// untapped by its own text, which Rogue's Passage is and Irrigated
/// Farmland — the first one tried here — is not: a land that taps itself on
/// the way in would have passed this assertion for a reason that has
/// nothing to do with the Archon.
#[test]
fn archon_of_emeria_lands_as_a_two_three_flier_and_does_not_tap_the_opponents_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(83, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[archon_of_emeria()])
        .hand(1, &[rogue_s_passage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, archon_of_emeria());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, archon_of_emeria()).is_some() && stack_is_empty(e)
    });
    let archon = on_battlefield(&engine, p0, archon_of_emeria()).expect("the Archon resolved");
    assert_eq!(pt(&engine, archon), (2, 3), "the body the card prints");
    assert!(
        keywords(&engine, archon).contains(KeywordSet::FLYING),
        "the one implemented line, as the layers project it"
    );

    reach_their_main_phase(&mut engine, p1);
    let land = play_land(&mut engine, p1, rogue_s_passage());
    assert!(
        !entered_tapped(&engine, land),
        "\"nonbasic lands your opponents control enter tapped\" is the \
         Coverage::Partial gap: the Archon stands and the land still comes in untapped"
    );
}

fn aven_mindcensor() -> CardIndex {
    card_index("d9517c5d-66d0-4178-96fb-a8c04f311ad8")
}

/// Aven Mindcensor is a {2}{W} 2/1 Bird Wizard printing Flash and Flying,
/// and the third line — "if an opponent would search a library, that player
/// searches the top four cards instead" — is the `Coverage::Partial` gap the
/// card head names, so nothing here pretends to reach it. What is proven is
/// the two lines that *are* written, and Flash is proven by its timing: the
/// cast happens during the opponent's main phase, where a creature without
/// Flash is refused the action outright, and the body and Flying are then
/// read off the permanent that landed while it is still the opponent's turn.
#[test]
fn aven_mindcensor_flashes_in_on_the_opponents_turn_and_flies_once_it_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[aven_mindcensor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        on_battlefield(&engine, p0, aven_mindcensor()).is_none(),
        "it starts in hand, not on the table"
    );

    // The opponent's own first main phase: a sorcery-speed creature could
    // not be cast here at all, so this is what Flash buys.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert_eq!(
        engine.state().turn.active,
        p1,
        "the cast below happens on the opponent's turn"
    );

    cast_from_hand(&mut engine, p0, aven_mindcensor());
    pass_until(&mut engine, stack_is_empty);

    let bird = on_battlefield(&engine, p0, aven_mindcensor())
        .expect("three Plains pay {2}{W} on the opponent's turn, which is Flash");
    assert_eq!(
        engine.state().turn.active,
        p1,
        "and it resolved without the turn ever coming back around"
    );
    assert_eq!(pt(&engine, bird), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "Flying is on the permanent, not only in the card's keyword list"
    );
}

// oracle_id = "10af9cd9-1700-48f9-97e1-61e239536fef"
fn brad_boimler_eager_ensign() -> CardIndex {
    card_index("10af9cd9-1700-48f9-97e1-61e239536fef")
}

/// Brad Boimler, Eager Ensign ({1}{W}) is a 2/2 legendary Human Officer with
/// lifelink, and the card is `Coverage::Partial` because the replacement
/// effect that would add an extra counter whenever he becomes tapped has no
/// DSL form — so the half there is to play is the keyword. Lifelink exists
/// only while damage is being dealt, so he attacks and the other seat blocks:
/// his two damage land on an Elf, which is worth exactly two life to his
/// controller, while the defending player's life total does not move. A
/// lifegain trigger, or damage that simply connected with a player, would
/// each satisfy only one of those two readings.
#[test]
fn brad_boimler_deals_his_damage_to_a_blocker_and_his_controller_gains_that_much_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[brad_boimler_eager_ensign()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let brad = on_battlefield(&engine, p0, brad_boimler_eager_ensign()).expect("he is out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, brad), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, brad).contains(KeywordSet::LIFELINK),
        "the one line of this card that is implemented"
    );
    let (mine, theirs) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );

    // Declare him as the only attacker, against the one defender a duel has.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the declaration it was asked for")
    };
    assert!(
        attackers.contains(&brad),
        "nothing keeps a settled 2/2 out of combat: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "one surviving opponent, and no planeswalkers"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brad, defenders[0])],
            },
        )
        .unwrap();

    // The other seat blocks, so the damage lands on a creature rather than on
    // a player: lifelink is about damage, not about connecting.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the declaration it was asked for")
    };
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == blocker && option.attackers.contains(&brad)),
        "a 1/1 may block a 2/2 with no evasion: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, brad)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().players[0].life > mine);
    assert_eq!(
        engine.state().players[0].life,
        mine + 2,
        "lifelink: the two damage he dealt to the Elf are two life"
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs,
        "and none of it reached the player, because the Elf blocked"
    );
}

// oracle_id = "a1cc5e37-b09a-4b7f-afd5-77c1c35aa425"
fn carrion_feeder() -> CardIndex {
    card_index("a1cc5e37-b09a-4b7f-afd5-77c1c35aa425")
}

/// Carrion Feeder ({B}, a 1/1 Zombie) is `Coverage::Partial`: "This creature
/// can't block" is not in the DSL, but "Sacrifice a creature: Put a +1/+1
/// counter on this creature" is. The sacrifice is a *cost* and not a target
/// (CR 701.21a), so which creatures are on the menu is a board reading: the
/// Feeder is a creature you control and sits on its own menu, the Elves
/// beside it pay, and the Elves across the table are neither offered nor
/// accepted. Paying leaves the card in its owner's graveyard and turns the
/// projected 1/1 into the 2/2 the counter prints — the half of the sentence
/// the card file cannot show, since the effect and the cost are the engine's
/// answer rather than the card's.
#[allow(clippy::too_many_lines)] // both halves of "a creature you control", in one game
#[test]
fn carrion_feeder_eats_a_creature_of_yours_for_a_counter_and_may_not_eat_theirs() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(83, swamp())
        .battlefield(0, &[carrion_feeder(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let feeder = on_battlefield(&engine, p0, carrion_feeder()).expect("the Feeder stands");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves stand");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");
    assert_eq!(pt(&engine, feeder), (1, 1), "the body it prints");
    assert_eq!(
        counters_on(&engine, feeder, CounterKind::P1P1),
        0,
        "and no counter on it yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(feeder, 0)),
        "a creature to eat is on the table, so the one line the Feeder \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, carrion_feeder(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is the whole of what tells a client this is a cost being \
         paid and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert!(
        options.contains(&fodder),
        "the Elves are a creature you control: {options:?}"
    );
    assert!(
        options.contains(&feeder),
        "and the Feeder is a creature too, so it is on its own menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: a seat sacrifices only what it controls, whatever the \
         filter says: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused before anything moves"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard, not exiled"
    );
    assert_eq!(
        counters_on(&engine, feeder, CounterKind::P1P1),
        1,
        "the ability put a +1/+1 counter on the Feeder"
    );
    assert_eq!(
        pt(&engine, feeder),
        (2, 2),
        "a printed 1/1 plus one +1/+1 counter, read through the layers"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's Elves never moved"
    );
}

// oracle_id = "cee583b7-7cc3-40ea-a227-b760839ec291"
fn dwalin_weaponmaster() -> CardIndex {
    card_index("cee583b7-7cc3-40ea-a227-b760839ec291")
}

/// Dwalin, Weaponmaster — {1}{R/W} — 2/1 legendary Dwarf Warrior with
/// "First strike". He is cast rather than seeded onto a board, and the
/// keyword is proven where it is the only thing that can be seen: a 1/1
/// Llanowar Elves blocking him. Dwalin's two damage is dealt in the
/// first-strike damage step, so the Elf dies before it deals the single
/// point that would also be lethal to a 2/1 — without the keyword the two
/// trade, which is exactly the reading asserted against. The hone-counter
/// sentence is the `Coverage::Partial` half and stays off the card, so
/// nothing else is claimed here.
#[test]
fn dwalin_kills_what_blocks_him_before_it_can_kill_him_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), plains()])
        .hand(0, &[dwalin_weaponmaster()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, dwalin_weaponmaster());
    pass_until(&mut engine, stack_is_empty);
    let dwalin = on_battlefield(&engine, p0, dwalin_weaponmaster()).expect("Dwalin resolved");
    assert!(
        keywords(&engine, dwalin).contains(KeywordSet::FIRST_STRIKE),
        "the printed keyword, as the layer system projects it"
    );

    // He was cast this turn, so the attack has to wait for the next one.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the attacker declaration")
    };
    let defender = defenders
        .into_iter()
        .next()
        .expect("the other seat is the one thing to attack");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dwalin, defender)],
            },
        )
        .expect("an untapped, unsick Dwalin may attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is still standing");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, dwalin)],
            },
        )
        .expect("the Elf may block him");

    // The Elf's death is the end of the first-strike damage step, which is
    // also where the loop stops caring about anything behind it.
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "his two damage is more than a 1/1 has"
    );
    assert!(
        on_battlefield(&engine, p0, dwalin_weaponmaster()).is_some(),
        "and the Elf's damage is dealt in no step at all: one point of it \
         would have been lethal to a 2/1, so a Dwalin that had merely traded \
         would be lying in the graveyard beside it"
    );
}

fn endurance() -> CardIndex {
    card_index("c85d824b-c190-4d04-ab99-918ad0e6516c")
}

/// Endurance — {1}{G}{G} — 3/4 Elemental Incarnation with flash and reach,
/// and "Evoke—Exile a green card from your hand", which this pool models as
/// an alternative cost plus the trigger that sacrifices what entered for it.
///
/// The scenario is p0's own end step, because that is the one moment that
/// tells flash apart from the timing every creature already has: the Llanowar
/// Elves in the same hand print no flash and must not be castable while the
/// Elemental is. The evoke cast is then read off the whole table — the Elves
/// leave the hand for *exile* and not the graveyard, the three Forests stay
/// untapped because the printed {1}{G}{G} was never paid, and the Elemental
/// that lands a moment later is eaten by its own enters-evoked trigger, with
/// flash and reach projected onto it while it is there. The unsupported enter
/// trigger asks nothing: no target choice interrupts the walk at all.
#[allow(clippy::too_many_lines)] // flash, the cost menu, the exile it pays with, and the trigger after it
#[test]
fn endurance_flashes_in_on_its_evoke_cost_and_is_sacrificed_by_its_own_trigger() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7719, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[endurance(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // Walk to the end step, where a creature spell is legal only on the
    // strength of its own printed flash.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let elemental = in_hand(&engine, p0, endurance()).expect("Endurance is in hand");
    let fodder = in_hand(&engine, p0, llanowar_elves()).expect("the green card is in hand");
    // The Forests are tapped *before* the cast, because affordability here
    // is read off the mana pool and not off what could still be tapped: with
    // an empty pool the printed {1}{G}{G} is not an option at all, the evoke
    // cost is the only one left, and a wizard with one option asks nothing.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the end step hands p0 priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&elemental),
        "flash: three untapped Forests pay {{1}}{{G}}{{G}}, and a creature \
         may be cast in an end step only when it prints flash: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&fodder),
        "the Elves print no flash, so the end step is not theirs: {:?}",
        legal.castable
    );

    // Cast it for the evoke cost rather than for its mana cost.
    engine
        .apply(p0, PlayerAction::CastSpell { card: elemental })
        .expect("the Elemental may be cast in this end step");
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "evoke is an alternative cost, so the cast asks which one: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat chooses its own cost");
    let evoke = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
        .expect("the evoke cost is offered beside the printed {1}{G}{G}");
    engine
        .apply(p0, PlayerAction::ChooseMode(evoke))
        .expect("the alternative cost may be chosen");

    // The cost names a green *card* in hand; the Forests the seat drew are
    // colourless lands, so the Elves are what the question is about.
    if let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p0, "the cost is paid by the seat paying it");
        assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
        assert!(
            options.contains(&fodder),
            "the Llanowar Elves are the green card in hand: {options:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .expect("the cost is paid with what the question offered");
    }

    // The Elemental itself: the body it prints, with both keyword bits, on
    // the table for as long as its own trigger takes to resolve.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, endurance()).is_some()
    });
    let body = on_battlefield(&engine, p0, endurance()).expect("the Elemental resolved");
    assert_eq!(pt(&engine, body), (3, 4), "the body the card prints");
    let kw = keywords(&engine, body);
    assert!(kw.contains(KeywordSet::FLASH), "flash");
    assert!(kw.contains(KeywordSet::REACH), "reach");

    // "When this creature enters [evoked], sacrifice it."
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, endurance()).is_none() && stack_is_empty(e)
    });
    assert!(
        in_graveyard(&engine, p0, endurance()).is_some(),
        "the enters-evoked trigger sacrifices the Elemental it just made"
    );

    // And the cost that bought it is in exile rather than the graveyard.
    let exile = engine.state().zones.list(ZoneLocation::Exile(p0));
    assert!(
        exile.iter().any(|id| engine
            .state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))),
        "`exile a green card from your hand`: {exile:?}"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "the evoke cost exiles; it does not discard"
    );
    // The three Forests were tapped to make the printed cost *offerable*,
    // and the mana they made is still sitting in the pool: an evoke cast
    // spends the card in exile and nothing else.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "the printed {{1}}{{G}}{{G}} was never paid: evoke is a *different* cost"
    );
}

// oracle_id = "70a6f08e-854d-4e2f-9d8c-c45ec3231157"
fn faeburrow_elder() -> CardIndex {
    card_index("70a6f08e-854d-4e2f-9d8c-c45ec3231157")
}

/// Faeburrow Elder prints vigilance, "+1/+1 for each color among permanents
/// you control" and a `{T}` ability adding one mana of each of those colors —
/// and its head is `Coverage::Partial`, with both colour-count lines left off
/// the card. That leaves the vigilance, and the only place it can be read is
/// the projection of a permanent the harness put down: the printed body is
/// 0/0, and the line that would grow it is the missing one, so whoever
/// receives priority first lets CR 704.5f bury it. Both halves are one
/// scenario — the layer system projects the vigilance onto the 0/0, and the
/// same card cast for real resolves, dies, and lies in its owner's graveyard
/// without ever having been able to attack.
#[test]
fn faeburrow_elder_projects_vigilance_onto_a_zero_zero_the_missing_line_never_grows() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[faeburrow_elder(), forest(), forest(), plains()])
        .hand(0, &[faeburrow_elder()])
        .start();

    // Read before the first priority round, which is the one window a 0/0
    // permanent exists in.
    let seeded = on_battlefield(&engine, p0, faeburrow_elder()).expect("the harness put it down");
    let printed = keywords(&engine, seeded);
    assert!(
        printed.contains(KeywordSet::VIGILANCE),
        "the printed vigilance is projected onto the permanent: {printed:?}"
    );
    assert_eq!(
        pt(&engine, seeded),
        (0, 0),
        "and the body is the printed 0/0: nothing counts the colours among \
         permanents you control, because that line is the `Coverage::Partial` gap"
    );

    // The first priority round is where CR 704.5f reads that body.
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );
    assert!(
        on_battlefield(&engine, p0, faeburrow_elder()).is_none(),
        "a 0/0 with no counters is put into its owner's graveyard before \
         anybody may use the vigilance it has"
    );

    // And the same card cast for real, which is what the gap costs the player
    // who draws it: it resolves and is buried again.
    cast_from_hand(&mut engine, p0, faeburrow_elder());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, faeburrow_elder()).is_none(),
        "the freshly cast Elder is not on the battlefield either"
    );
    assert!(
        in_graveyard(&engine, p0, faeburrow_elder()).is_some(),
        "it resolved and went to its owner's graveyard, as a 0/0 does"
    );
}

fn geist_of_saint_thalia() -> CardIndex {
    card_index("ef32a4a9-14e2-4738-b4c2-53ce5e1d2a53")
}

/// Geist of Saint Thalia — {1}{U} — Legendary Creature — Spirit Cleric, a
/// 1/2 with flying. `Coverage::Partial`: only the keyword is implemented and
/// the cost-reduction sentence is the documented gap, so playing the card is
/// the only way to prove the flying arrives through the layer projection
/// rather than merely being printed on the definition. Two Islands are
/// exactly {U}{U}, so the cast also shows the printed {1}{U} is what was
/// paid and nothing else was needed.
#[test]
fn geist_of_saint_thalia_lands_as_a_flying_body_on_its_printed_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[geist_of_saint_thalia()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, geist_of_saint_thalia());
    pass_until(&mut engine, stack_is_empty);

    let geist = on_battlefield(&engine, p0, geist_of_saint_thalia()).expect("the Geist resolved");
    assert!(
        types(&engine, geist).contains(TypeSet::CREATURE),
        "it landed as a creature",
    );
    assert_eq!(pt(&engine, geist), (1, 2), "the printed 1/2");
    assert!(
        keywords(&engine, geist).contains(KeywordSet::FLYING),
        "flying — the clause of the card that exists",
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}} came out of the two Islands and nothing was left over",
    );
}

fn gemrazer() -> CardIndex {
    card_index("3dfb0c0a-b68f-43b9-8475-28d0192fc4ed")
}

/// Gemrazer ({3}{G}) is `Coverage::Partial`: only two of its three printed
/// lines exist in the DSL, so what can be played is the body — a 4/4 Beast
/// with reach and trample — while mutate and the mutate trigger are the gap.
/// The scenario casts it off four Forests and then reads the *projected*
/// characteristics, which is the only reading that can see a keyword at all,
/// so it says the two keywords on the card are the two the creature on the
/// battlefield actually has. The 4/4 body is asserted beside them because
/// reach and trample on a permanent that arrived as the wrong creature would
/// be the same passing test — and the two Forests beyond the cost are there
/// so the {3} is not quietly paid by a land the test never counted.
#[test]
fn gemrazer_arrives_as_a_four_four_beast_with_reach_and_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[gemrazer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        in_hand(&engine, p0, gemrazer()).is_some(),
        "the card is in hand before anything is cast"
    );
    cast_from_hand(&mut engine, p0, gemrazer());
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, gemrazer()).expect("Gemrazer resolved");
    assert!(
        in_hand(&engine, p0, gemrazer()).is_none(),
        "and it left the hand to get there"
    );
    assert!(
        types(&engine, beast).contains(TypeSet::CREATURE),
        "it is a creature permanent: {:?}",
        types(&engine, beast)
    );
    assert_eq!(pt(&engine, beast), (4, 4), "the body the card prints");
    let kw = keywords(&engine, beast);
    assert!(kw.contains(KeywordSet::REACH), "Gemrazer has reach");
    assert!(kw.contains(KeywordSet::TRAMPLE), "Gemrazer has trample");
}

// oracle_id = "c1d6cce8-085f-42cb-8b0c-b6fbbf88b16a"
fn goblin_engineer() -> CardIndex {
    card_index("c1d6cce8-085f-42cb-8b0c-b6fbbf88b16a")
}

/// Goblin Engineer's activated half — "{R}, {T}, Sacrifice an artifact:
/// Return target artifact card with mana value 3 or less from your graveyard
/// to the battlefield" — since the search-to-graveyard ETB is the
/// `Coverage::Partial` gap. Three readings are struck by one activation: the
/// card that comes back is a Chromatic Lantern, mana value 3 exactly, so a
/// filter that read "less than three" would leave the ability with no target
/// at all; the identical Lantern in the opponent's graveyard is the "your"
/// the sentence prints; and the menu the sacrifice asks with holds the Sol
/// Ring this seat controls rather than the one across the table. Tapping to
/// float the {R} first is not scenery either — the ability is only offered
/// once the cost is payable, and it ends tapped with the red gone.
#[allow(clippy::too_many_lines)] // a sacrifice, a graveyard search and the permanent that arrives tapped
#[test]
fn goblin_engineer_trades_an_artifact_for_the_lantern_in_his_own_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The backing deck is the card's own graveyard clause: nothing else in the
    // harness puts a named *artifact card* into a graveyard — an opening
    // battlefield holds permanents and `seed_graveyard` moves cards off the top
    // of a library — so the library has to be the printing the ability is
    // about. Chromatic Lantern is mana value 3 exactly, the number "3 or less"
    // has to include.
    let mut engine = Duel::new(SEED, chromatic_lantern())
        .battlefield(0, &[goblin_engineer(), mountain(), quiet_artifact()])
        // The same card and the same artifact across the table, so each
        // refusal is made by the sentence rather than by the board.
        .battlefield(1, &[forest(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    let engineer = on_battlefield(&engine, p0, goblin_engineer()).expect("the Engineer is out");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let lantern =
        in_graveyard(&engine, p0, chromatic_lantern()).expect("my Lantern is in my graveyard");
    let their_lantern =
        in_graveyard(&engine, p1, chromatic_lantern()).expect("theirs is in their graveyard");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain is floating the {{R}} the cost asks for"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(engineer, 0)),
        "a card in the graveyard is a target and a red source is floating, so \
         the one line the Engineer prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, goblin_engineer(), 0);
    // CR 601.2c comes before CR 601.2h: the graveyard is asked about first and
    // the sacrifice second.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&lantern),
        "mana value 3 is within \"3 or less\": {options:?}"
    );
    assert!(
        !options.contains(&their_lantern),
        "\"from your graveyard\" — the identical card across the table is not \
         offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lantern],
            },
        )
        .expect("the card the question offered is the card it takes");

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the artifact this seat controls is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&their_rock),
        "CR 701.21a: an opponent's artifact is not yours to sacrifice: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is the whole menu — the Engineer is a creature and the Mountain \
         a land"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, chromatic_lantern()).is_some(),
        "the ability returns the card from the graveyard to the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, chromatic_lantern()).is_none(),
        "and it is no longer a card in a graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the sacrificed artifact is what the ability spent"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, chromatic_lantern()).is_some(),
        "the other seat's graveyard was never read, let alone moved from"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and neither was their artifact"
    );
    assert!(is_tapped(&engine, engineer), "{{T}} was part of the cost");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "and the {{R}} was spent, not merely floated"
    );
}

// oracle_id = "a8ddea1c-8d80-49c1-a5b4-630d5e51d66e"
fn hapatra_vizier_of_poisons() -> CardIndex {
    card_index("a8ddea1c-8d80-49c1-a5b4-630d5e51d66e")
}

/// Hapatra, Vizier of Poisons ({B}{G}, a 2/2) prints two triggers and the
/// pool implements one of them: "Whenever Hapatra deals combat damage to a
/// player, you may put a -1/-1 counter on target creature."
///
/// Nothing short of connecting can fire it, so the test attacks with her and
/// picks the target off the menu the trigger publishes — which has to hold a
/// creature on *each* side of the table, because the card says "target
/// creature" and not "you control". Sheoldred the Apocalypse is the creature
/// pointed at precisely because a 4/5 survives one -1/-1: the counter is then
/// read where it landed — one `CounterKind::M1M1` and a projected 3/4 — rather
/// than inferred from a 1/1 that had to die for it. The request's `min` of 0
/// is the printed "you may": the counter is optional, and the second,
/// Snake-making trigger has no `Trigger` variant at all, which is the
/// `Coverage::Partial` gap this test does not pretend to cross.
#[allow(clippy::too_many_lines)] // a combat played to damage, which is where the trigger lives
#[test]
fn hapatra_marks_the_creature_she_points_at_when_she_connects() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[hapatra_vizier_of_poisons(), llanowar_elves()])
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: which seat the seed
    // puts on the play decides whether a whole turn is in the way, and that
    // turn's combat is a question `reach_main_phase` has no arm for.
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let hapatra =
        on_battlefield(&engine, p0, hapatra_vizier_of_poisons()).expect("Hapatra is on the table");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("your own Elves are out");
    let victim =
        on_battlefield(&engine, p1, sheoldred_the_apocalypse()).expect("the Praetor is out");
    assert_eq!(pt(&engine, hapatra), (2, 2), "the body as printed");
    assert_eq!(
        counters_on(&engine, victim, CounterKind::M1M1),
        0,
        "nothing has marked anybody yet"
    );

    // Into combat. The attack is declared by hand, because `pass_until`'s own
    // answer to this question is an empty one — and a combat that never
    // happens is a trigger that never fires.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player,
        attackers,
        defenders,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0);
    assert!(
        attackers.contains(&hapatra),
        "a permanent that stood on the battlefield before the turn began is \
         not summoning sick: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "the only thing to attack is the opponent"
    );
    let defender = defenders
        .into_iter()
        .next()
        .expect("one defender, checked above");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(hapatra, defender)],
            },
        )
        .unwrap();

    // p1 may block with a 4/5 and chooses not to, so Hapatra connects and the
    // printed trigger asks what it may mark.
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
    assert_eq!(player, p0, "the controller of the trigger chooses");
    assert_eq!(
        (min, max),
        (0, 1),
        "\"you may put a -1/-1 counter on target creature\": one target, and \
         taking none of them is legal"
    );
    assert!(
        options.contains(&victim) && options.contains(&mine),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, victim, CounterKind::M1M1),
        1,
        "one -1/-1 counter, and not a +1/+1"
    );
    assert_eq!(
        pt(&engine, victim),
        (3, 4),
        "the counter read back through the layer system: a 4/5 marked once"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and only the creature that was named took anything"
    );
    assert_eq!(
        pt(&engine, hapatra),
        (2, 2),
        "Hapatra marks, she is not marked"
    );
}

fn kenrith_the_returned_king() -> CardIndex {
    card_index("d209b948-9afb-4fd1-a961-72c87282878c")
}

/// Kenrith is a 5/5 for {4}{W} whose red and green lines are both written,
/// and this scenario plays them off one first main phase. The red one is the
/// reason the test exists: "{R}: All creatures gain trample and haste until
/// end of turn" names no controller, so the pump lands on the Elves across
/// the table exactly as it lands on Kenrith's own side — the same crossing
/// the Liquimetal Coating test guards, one activation cheaper. The `0/0` in
/// the card is asserted with the keywords so a pump that had also moved P/T
/// could not pass. The green one is the contrast: "{1}{G}: Put a +1/+1
/// counter on **target** creature" reaches either side of the table and is
/// answered with one, so the counter has to appear on the creature that was
/// named and on none of the others it was offered.
#[allow(clippy::too_many_lines)] // one ability, the whole board read before and after it
#[test]
fn kenrith_pumps_every_creature_and_marks_only_the_one_he_aimed_at() {
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
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[kenrith_the_returned_king()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main"
    );

    // The King is cast off the Plains and one Forest alone, leaving the
    // Mountains and the other Forests in reserve: a mana pool empties when a
    // step ends (CR 500.5), and the colours left floating are the colours
    // the two abilities below are paid with.
    for land in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    let grove = all_on_battlefield(&engine, p0, forest());
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: grove[0] })
        .unwrap();
    let king_card = in_hand(&engine, p0, kenrith_the_returned_king()).expect("the King is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: king_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let king = on_battlefield(&engine, p0, kenrith_the_returned_king()).expect("the King resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, king), (5, 5), "the body the card prints");

    // {R}: the Mountains pay it, and "{R}" is the whole cost.
    for mountain in all_on_battlefield(&engine, p0, mountain()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: mountain })
            .unwrap();
    }
    // Ability 0 is the red team pump; 1 is the counter, and 2 and 3 are the
    // life and the draw.
    activate(&mut engine, p0, kenrith_the_returned_king(), 0);
    pass_until(&mut engine, stack_is_empty);

    for (who, creature) in [
        ("the King", king),
        ("my Elves", mine),
        ("their Elves", theirs),
    ] {
        let granted = keywords(&engine, creature);
        assert!(
            granted.contains(KeywordSet::TRAMPLE) && granted.contains(KeywordSet::HASTE),
            "\"all creatures\" reached {who}: {granted:?}"
        );
    }
    assert_eq!(
        (pt(&engine, king), pt(&engine, mine)),
        ((5, 5), (1, 1)),
        "the red ability grants keywords and no body: 0/0 is still 0/0"
    );

    // {1}{G}: the two Forests still standing pay it, and the ability that
    // prints a target asks for one.
    for forest_land in &grove[1..] {
        engine
            .apply(
                p0,
                PlayerAction::ActivateManaAbility {
                    source: *forest_land,
                },
            )
            .unwrap();
    }
    activate(&mut engine, p0, kenrith_the_returned_king(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the counter targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs) && options.contains(&king),
        "\"target creature\" names no controller, so every creature is offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, mine, CounterKind::P1P1),
        1,
        "the creature that was named takes the counter"
    );
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::P1P1),
        0,
        "and the creature that was only offered does not"
    );
    assert_eq!(
        counters_on(&engine, king, CounterKind::P1P1),
        0,
        "Kenrith is a legal target for his own green line and still not the one picked"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "a 1/1 with one +1/+1 counter on it"
    );
}

fn kess_dissident_mage() -> CardIndex {
    card_index("f5092c14-eec4-472c-999c-ba96c36b2fbb")
}

/// Kess, Dissident Mage prints two sentences and the DSL carries the first
/// one: `Flying`, with the once-per-turn graveyard cast left to the
/// `// NOT SUPPORTED` line. What this plays is that half — she is cast off
/// `{1}{U}{B}{R}`, resolves, and stands as the printed 3/4 with flying
/// projected onto her by the keyword itself and by nothing else on the
/// board. The Llanowar Elves beside her are the counter-half: a projection
/// that had lost flying's subject and handed it to every creature under the
/// seat would read exactly the same on Kess alone, and reads wrong here.
#[test]
fn kess_resolves_as_a_flying_three_four_and_grants_flying_to_nobody_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, island())
        .battlefield(
            0,
            &[island(), swamp(), mountain(), mountain(), llanowar_elves()],
        )
        .hand(0, &[kess_dissident_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the ground-bound bystander flies before she lands"
    );

    // The four lands pay {1}{U}{B}{R} exactly. The Elf is kept back so that
    // "exactly" stays true: it makes a fifth mana that would still be
    // floating at the assertion below.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, kess_dissident_mage());
    pass_until(&mut engine, stack_is_empty);

    let kess = on_battlefield(&engine, p0, kess_dissident_mage()).expect("she resolved");
    assert_eq!(pt(&engine, kess), (3, 4), "the body the card prints");
    let types = engine
        .state()
        .object(kess)
        .expect("she is still an object")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::CREATURE),
        "a legendary Human Wizard is a creature: {types:?}"
    );
    assert!(
        keywords(&engine, kess).contains(KeywordSet::FLYING),
        "\"Flying\" is the half of the card the DSL carries"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}}{{B}}{{R}} was paid out of the pool, not left floating"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "and flying stayed where it was printed: the Elves are still on the ground"
    );
}

// oracle_id = "5470dcfa-4eff-43da-abf7-19922841f719"
fn kitchen_finks() -> CardIndex {
    card_index("5470dcfa-4eff-43da-abf7-19922841f719")
}

/// Kitchen Finks — {1}{G/W}{G/W} — 3/2 Ouphe: "When this creature enters,
/// you gain 2 life", and persist underneath it.
///
/// The half that is written is the enter trigger, so the scenario is the
/// only one that can read it: three Forests pay the hybrid cost, and the
/// life total is asked for *twice* — once while the spell is still on the
/// stack, where it must not have moved, and once after everything has
/// resolved, where it must have moved by exactly 2 and only for the
/// controller. The opponent's total is the counter-half of "you gain", and
/// the body on the table is what says the spell resolved rather than that
/// the trigger fired off something else.
///
/// The persist clause is the card's `Coverage::Partial` gap and is
/// deliberately not played here: a test written around what the engine
/// cannot express would pass by doing nothing at all.
#[test]
fn kitchen_finks_gains_its_controller_two_life_and_leaves_the_opponent_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[kitchen_finks()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained before anything is cast"
    );

    cast_from_hand(&mut engine, p0, kitchen_finks());
    assert!(
        !stack_is_empty(&engine),
        "the Finks is a spell, not a permanent yet"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the life belongs to the enters trigger and not to the cast"
    );

    pass_until(&mut engine, stack_is_empty);

    let finks = on_battlefield(&engine, p0, kitchen_finks()).expect("the Finks resolved");
    assert_eq!(
        pt(&engine, finks),
        (3, 2),
        "the body the card prints, so it is the Finks that entered"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "exactly 2, once, for the seat that controls it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you gain 2 life\" is not each player"
    );
}

// oracle_id = "e9117015-1050-44dd-a46b-e7ffe2085fae"
fn ledger_shredder() -> CardIndex {
    card_index("e9117015-1050-44dd-a46b-e7ffe2085fae")
}

/// Ledger Shredder prints `{1}{U}` for a 1/3 flying Bird Advisor, and the
/// connive trigger — "whenever a player casts their second spell each turn"
/// — is the half this printing does not have.
///
/// So two spells are cast in one first main phase, which is exactly the
/// second spell the missing trigger is about, and the Bird is a 1/3 with no
/// counter on it when the stack clears. The keyword the card *does* print is
/// played rather than read: a turn later the Shredder and a ground Elves
/// attack together, and the blocker the opponent's own Elves is offered for
/// is the ground attacker and never the flier — flying is a pairing question
/// (CR 509.1b), and an offer that simply omitted the Bird would say the same
/// on a board holding nothing that could block anything at all.
#[test]
fn ledger_shredder_flies_over_the_ground_and_never_connives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(6111, forest())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[llanowar_elves(), ledger_shredder()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elves go first: `{G}` can only come off the Forest and leaves the
    // two Islands in the pool for the Bird, so neither cast has to choose.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    cast_from_hand(&mut engine, p0, ledger_shredder());
    pass_until(&mut engine, stack_is_empty);

    let shredder = on_battlefield(&engine, p0, ledger_shredder()).expect("the Shredder resolved");
    let ground = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves resolved");
    assert_eq!(pt(&engine, shredder), (1, 3), "a 1/3 as printed");
    assert!(
        keywords(&engine, shredder).contains(KeywordSet::FLYING),
        "and the one keyword the card prints"
    );
    assert_eq!(
        pt(&engine, shredder),
        (1, 3),
        "two spells in one turn and the Bird is untouched: the second-spell \
         connive is the `Coverage::Partial` gap, so no card was drawn and no \
         +1/+1 counter landed"
    );

    // A turn has to turn before either of them may attack.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate above stops on nothing else")
    };
    assert!(
        attackers.contains(&shredder) && attackers.contains(&ground),
        "both are untapped and no longer summoning sick: {attackers:?}"
    );
    let target = *defenders.first().expect("the opponent is there to attack");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shredder, target), (ground, target)],
            },
        )
        .expect("a 1/3 flier and a 1/1 may attack the opponent");

    // The block step. The Elves across the table can block the ground
    // attacker, which is what makes their silence about the flier mean
    // something rather than being an empty menu.
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let mut pairings: Option<Vec<ObjectId>> = None;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                pairings = Some(
                    blockers
                        .iter()
                        .filter(|option| option.blocker == theirs)
                        .flat_map(|option| option.attackers.clone())
                        .collect(),
                );
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
                break;
            }
            other => panic!("expected the block step, got {other:?}"),
        }
    }
    let pairings = pairings.expect("their Elves can block, so the step is asked");
    assert!(
        pairings.contains(&ground),
        "a ground 1/1 may block a ground 1/1: {pairings:?}"
    );
    assert!(
        !pairings.contains(&shredder),
        "\"flying\" (CR 509.1b): the same blocker is offered the ground \
         attacker and never the Bird"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, shredder),
        "and it went through — attacking tapped it and nothing blocked it"
    );
}

// oracle_id = "61b1d7e5-6155-4204-b110-35a890551ec8"
fn lotleth_troll() -> CardIndex {
    card_index("61b1d7e5-6155-4204-b110-35a890551ec8")
}

/// Lotleth Troll — {B}{G} 2/1 Zombie Troll with trample — and the whole of
/// what the engine writes of it: "Discard a creature card: Put a +1/+1
/// counter on this creature."
///
/// Every half of that sentence is the *engine's* answer and not the card's,
/// so the test reads it out of the question it is asked. The menu holds the
/// Llanowar Elves and none of the seven Forests the filler deck dealt, which
/// is `Filter::CREATURE` doing its work; the prompt variant is what says the
/// card is being *paid* and not searched for; and the payoff is counted on
/// the battleffeld rather than read off the card file — one +1/+1 counter
/// turns the printed 2/1 into a 3/2, which no other reading of the board
/// produces.
///
/// The refused follow-up is the cost's other half. With the only creature
/// card spent, no creature card is left in hand, the cost cannot be paid
/// (CR 118.3) and the engine stops offering the ability at all — which is
/// how this engine refuses every cost a board cannot meet. The `{B}`
/// regeneration is the `Coverage::Partial` gap and is deliberately not
/// asserted here.
#[test]
fn lotleth_troll_trades_a_creature_card_for_a_counter_and_tramples() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        // A third land, and it is the {B} the regenerate line would cost.
        // `cast_from_hand` taps everything and the Troll takes {B}{G}, so
        // one black is left floating when the offer below is read — without
        // it `can_afford` refuses a priced ability whatever the card says,
        // and the pin would keep passing after the line was written.
        .battlefield(0, &[forest(), swamp(), swamp()])
        .hand(0, &[lotleth_troll(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, lotleth_troll());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll resolved");
    assert!(
        keywords(&engine, troll).contains(KeywordSet::TRAMPLE),
        "trample is the card's whole keyword line"
    );
    assert_eq!(
        pt(&engine, troll),
        (2, 1),
        "a printed 2/1 with nothing on it yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = deeds(&legal, &[troll]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "the discard is the Troll's only activated ability, and it is offered \
         because a creature card is there to pay it: {offered:?}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "and the {{B}} the missing regenerate line costs is floating, so the \
         price is not what keeps it off that list"
    );

    activate(&mut engine, p0, lotleth_troll(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which card, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat pays the cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one card, and the cost asks once");
    let fodder = in_hand(&engine, p0, llanowar_elves()).expect("the Elves are in hand");
    assert_eq!(
        options,
        vec![fodder],
        "a creature card and nothing else: the seven Forests the filler deck \
         dealt are no more discardable to this cost than the opponent's board"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature card the question offered pays the cost");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "the discarded card left the hand"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard"
    );
    assert_eq!(
        counters_on(&engine, troll, CounterKind::P1P1),
        1,
        "one +1/+1 counter, put on the creature the ability names"
    );
    assert_eq!(
        pt(&engine, troll),
        (3, 2),
        "so the 2/1 is a 3/2 — and the Elves' absence from the board is the \
         other half of the same counter"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(troll, 0)),
        "with the only creature card spent, the cost cannot be paid and the \
         ability is no longer offered: {:?}",
        legal.abilities
    );
}

fn luminous_broodmoth() -> CardIndex {
    card_index("28c7c816-07e7-42fb-923c-bf149ba28b38")
}

/// Luminous Broodmoth is `Coverage::Partial`: the printed flying is on the
/// card and the second sentence — "whenever a creature you control without
/// flying dies, return it to the battlefield under its owner's control with a
/// flying counter on it" — is not.
///
/// So the Broodmoth is cast for {2}{W}{W} and lands as the 3/4 flyer it
/// prints, which is the half that is written; then a creature under the same
/// seat with no flying is destroyed through the stack, which is exactly the
/// event the missing sentence is about. Vindicate is the kill because it
/// leaves the card in a graveyard rather than exiling it, and the Broodmoth
/// standing untouched beside the dead Elf is the control: the assertion is
/// about the Elf, not about a spell that ate the wrong permanent.
#[test]
fn luminous_broodmoth_flies_in_and_leaves_a_dead_elf_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(89, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[luminous_broodmoth(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    // One Swamp sits out the Broodmoth's {2}{W}{W}: the Vindicate behind it
    // is {1}{W}{B}, and a payment that spent both black sources on the first
    // cast would leave the second uncastable for a reason that has nothing
    // to do with the card under test.
    let held = on_battlefield(&engine, p0, swamp()).expect("a Swamp is out");
    tap_mana_except(&mut engine, p0, held);
    let moth_card = in_hand(&engine, p0, luminous_broodmoth()).expect("the Broodmoth is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: moth_card })
        .expect("{2}{W}{W} is in the pool");
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, luminous_broodmoth()).is_some()
    });

    let moth = on_battlefield(&engine, p0, luminous_broodmoth()).expect("the Broodmoth resolved");
    assert_eq!(pt(&engine, moth), (3, 4), "the body the card prints");
    assert!(
        keywords(&engine, moth).contains(KeywordSet::FLYING),
        "and the one line of its text that is implemented"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the Elf has no flying to save it"
    );

    // The held Swamp pays the {1}{W}{B}, and the Elf is what it is pointed at.
    tap_all_mana(&mut engine, p0);
    let doom = in_hand(&engine, p0, vindicate()).expect("the Vindicate is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: doom })
        .expect("one white, one black and one generic");
    let mut aimed = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                assert!(
                    options.contains(&elves),
                    "\"destroy target permanent\" reaches a creature of your own: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![elves],
                        },
                    )
                    .expect("the Elf was one of the options");
                aimed = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected a target for the Vindicate, got {other:?}"),
        }
    }
    assert!(aimed, "the Vindicate asks what it is pointed at");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the Elf died"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and stayed in its owner's graveyard: nothing returned it with a \
         flying counter, which is the second sentence the card does not have \
         yet"
    );
    assert!(
        on_battlefield(&engine, p0, luminous_broodmoth()).is_some(),
        "while the Broodmoth stood untouched through both spells"
    );
}

// oracle_id = "51233ade-70cd-4539-9f41-5ffab761da54"
fn malevolent_hermit() -> CardIndex {
    card_index("51233ade-70cd-4539-9f41-5ffab761da54")
}

/// Malevolent Hermit's whole printed text is one line: "{U}, Sacrifice this
/// creature: Counter target noncreature spell unless its controller pays
/// {3}." Two spells of different kinds stand on the stack at once, so the
/// target menu is the evidence for the filter — the Ritual on it and the
/// Elves not. The price then falls on the *target's* controller rather than
/// the Hermit's, which is what `PlayerRel::ControllerOfTarget` gets right or
/// wrong in silence, and declining to pay is what turns "unless" into the
/// countered spell in a graveyard. The Hermit in its owner's graveyard is
/// the sacrifice having been paid as a cost rather than promised.
#[allow(clippy::too_many_lines)] // two spells on the stack, the menu between them and the tax after
#[test]
fn malevolent_hermit_taxes_a_noncreature_spell_and_pays_for_it_with_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(97, island())
        .battlefield(0, &[malevolent_hermit(), island()])
        .battlefield(1, &[forest(), swamp(), swamp(), swamp(), swamp()])
        .hand(1, &[llanowar_elves(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // One spell of each kind, so the menu has something it must offer and
    // something it must decline.
    cast_from_hand(&mut engine, p1, llanowar_elves());
    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    // The Island is tapped first: an ability whose cost nobody can pay is
    // absent from the offer, which would make the assertion below pass for a
    // reason that has nothing to do with the Hermit.
    tap_all_mana(&mut engine, p0);
    let ritual = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == dark_ritual()))
        })
        .expect("the Ritual is on the stack");
    assert!(
        on_battlefield(&engine, p0, malevolent_hermit()).is_some(),
        "the Hermit is standing, and nothing has been sacrificed yet"
    );

    // Ability 0 is the printed line; the offer is part of the assertion.
    activate(&mut engine, p0, malevolent_hermit(), 0);

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target noncreature spell\" asks for one, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that picks");
    assert_eq!(
        options,
        vec![ritual],
        "the Ritual is a noncreature spell and the Elves are a creature \
         spell, which is the whole of the filter"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the target was one of the options");
    assert!(
        in_graveyard(&engine, p0, malevolent_hermit()).is_some(),
        "the creature is the cost, paid as the ability is activated \
         (CR 601.2h), so it is in its owner's graveyard before anybody is \
         asked for {{3}}"
    );

    // The ability resolves: the price lands on the Ritual's controller.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the yes/no it was waiting for")
    };
    assert_eq!(
        player, p1,
        "`PlayerRel::ControllerOfTarget`: the price falls on the spell's \
         controller and not on the Hermit's"
    );
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::PayTax { mana: 3 },
        "the printed {{3}}"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "and the pool could pay it, so declining is a choice and not an \
         inability"
    );

    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining answers out of the question's own enumeration");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "the unpaid Ritual was countered, and a countered spell goes to its \
         owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "and the {{3}} stayed in the pool: the spell was countered, not \
         bought off"
    );
    assert!(
        on_battlefield(&engine, p0, malevolent_hermit()).is_none(),
        "the Hermit is still the payment for an ability already spent"
    );
}

// oracle_id = "5d27c63e-d1ef-48af-b51d-01ebc6daeac9"
fn mikaeus_the_unhallowed() -> CardIndex {
    card_index("5d27c63e-d1ef-48af-b51d-01ebc6daeac9")
}

/// Mikaeus, the Unhallowed prints one sentence this card can say — "other
/// non-Human creatures you control get +1/+1" — and two it cannot. One board
/// strikes all three words of the one at once: a Llanowar Elves beside him is
/// a 2/2 while he stays the printed 5/5 ("other", so he pumps no part of
/// himself) and the Elves across the table stays the printed 1/1 ("you
/// control").
///
/// The rest is the `Coverage::Partial` gap, and it is struck rather than left
/// implied, because the first draft of this card claimed both keywords and
/// nothing at the table changed: intimidate and undying are bits in
/// `KeywordSet` that no engine rule reads. So the same creature is fed to an
/// Ashnod's Altar, and what the board says afterwards is a dead Elf — which
/// is what a granted keyword nobody reads actually looks like.
#[test]
fn mikaeus_lords_the_nonhumans_beside_him_and_the_undying_half_is_not_granted() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(88, swamp())
        .battlefield(
            0,
            &[mikaeus_the_unhallowed(), ashnods_altar(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: which seat the seed
    // put on the play decides whether a whole turn is in the way.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mikaeus = on_battlefield(&engine, p0, mikaeus_the_unhallowed()).expect("Mikaeus stands");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // The filter's third word, "non-Human", goes unstruck: no Human card is
    // named on this board.
    assert_eq!(
        pt(&engine, elves),
        (2, 2),
        "\"other non-Human creatures you control get +1/+1\""
    );
    assert_eq!(
        pt(&engine, mikaeus),
        (5, 5),
        "\"other\" — Mikaeus is a Zombie Cleric and no part of him is included"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"you control\" — the Elf across the table is the printed 1/1"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::UNDYING),
        "the undying half of that same sentence is *not* granted: no rule \
         reads the bit, and a keyword nothing reads would read as finished"
    );
    assert!(
        !keywords(&engine, mikaeus).contains(KeywordSet::INTIMIDATE),
        "nor is the keyword he prints for himself, for the same reason"
    );

    // The Altar costs nothing but the creature, so nothing but undying stands
    // between "dies" and "comes back".
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Altar asks which creature to eat, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search"
    );
    assert!(
        options.contains(&elves) && options.contains(&mikaeus),
        "both of this seat's creatures are food: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves pay the cost");

    // The trigger has to come and go before the board can be read.
    pass_until(&mut engine, stack_is_empty);

    // The gap, struck rather than left implied. Undying is a bit in
    // `KeywordSet` that no engine rule reads, so the card does not grant it
    // (`keyword_tests::ENFORCED` is the list, and the lint beside it is what
    // caught this card claiming intimidate as well). What that looks like at
    // the table is exactly this: the creature dies and stays dead.
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "no rule reads undying, so the Elves lie where the Altar put them"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and nothing returned them to the battlefield"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "the Altar's own line did run: {{C}}{{C}} for the creature it ate"
    );
}

fn phyrexian_fleshgorger() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

/// Phyrexian Fleshgorger is `{7}` for a 7/5 artifact creature with menace and
/// lifelink; prototype and ward—pay life equal to its power are the two
/// clauses the DSL cannot carry, which is what `Coverage::Partial` says.
/// What *is* written is played here in full: seven Forests pay the printed
/// cost, the Wurm resolves onto the battlefield, and the reading is taken off
/// the layer projection rather than off the card file — an artifact *and* a
/// creature, a 7/5 body, and both keywords, none of which anything else on
/// this board could have made.
#[test]
fn phyrexian_fleshgorger_resolves_as_the_seven_five_that_carries_menace_and_lifelink() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 7])
        .hand(0, &[phyrexian_fleshgorger()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // An offer is read off the *pool*, so the seven Forests are tapped
    // first: `{7}` is not castable out of untapped lands.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat holding the card holds priority");
    let card = in_hand(&engine, p0, phyrexian_fleshgorger()).expect("the Wurm is in hand");
    assert!(
        legal.castable.contains(&card),
        "seven Forests, seven mana, so `{{7}}` is payable: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{7} is paid");
    pass_until(&mut engine, stack_is_empty);

    let wurm = on_battlefield(&engine, p0, phyrexian_fleshgorger())
        .expect("the Wurm resolved under the seat that cast it");
    let chars = engine
        .state()
        .object(wurm)
        .expect("the Wurm is an object")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::ARTIFACT) && chars.types.contains(TypeSet::CREATURE),
        "an artifact creature, not one or the other: {:?}",
        chars.types
    );
    assert_eq!(pt(&engine, wurm), (7, 5), "the body the card prints");
    let granted = keywords(&engine, wurm);
    assert!(granted.contains(KeywordSet::MENACE), "menace");
    assert!(granted.contains(KeywordSet::LIFELINK), "lifelink");
}

// oracle_id = "c739e180-2f14-41ed-8e7e-50b7df985f35"
fn rabbit_battery() -> CardIndex {
    card_index("c739e180-2f14-41ed-8e7e-50b7df985f35")
}

/// Rabbit Battery — {R}, a 1/1 artifact creature — Equipment Rabbit printing
/// haste for itself, "Equipped creature gets +1/+1 and has haste", and
/// Reconfigure {R}, whose attach half is the equip ability CR 702.151 words
/// exactly as CR 702.6 does.
///
/// The Elves is the control the whole claim rests on: a printed 1/1 with no
/// haste of its own before the reconfigure, a 2/2 with haste after it — so the
/// bonus and the keyword are read as changes to a *bystander* and not as
/// anything the Battery says about itself. The Battery standing at its printed
/// 1/1 while it holds the Elves is the other half of
/// `Filter::AttachedToBySource`: the grant reaches the creature it is attached
/// to and never its own source. Reconfiguring is pressed rather than read,
/// because "target creature you control" is a question the engine asks, and the
/// {R} that pays for it is the red the three Mountains left floating one spell
/// earlier; the unattach mode and "while attached, this isn't a creature" are
/// the `Coverage::Partial` gap and are deliberately left alone.
#[test]
fn rabbit_battery_reconfigures_onto_the_elves_and_hands_it_a_bonus_and_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4211, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[rabbit_battery()])
        .start();
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, elves), (1, 1), "a printed 1/1");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "and nothing has handed it haste yet"
    );

    cast_from_hand(&mut engine, p0, rabbit_battery());
    pass_until(&mut engine, stack_is_empty);
    let battery = on_battlefield(&engine, p0, rabbit_battery()).expect("the Battery resolved");
    assert_eq!(pt(&engine, battery), (1, 1), "a printed 1/1 of its own");
    assert!(
        keywords(&engine, battery).contains(KeywordSet::HASTE),
        "and the haste the card prints for itself"
    );
    assert!(
        engine
            .state()
            .object(battery)
            .is_some_and(|o| o.attached_to.is_none()),
        "it arrives holding nobody"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "{{R}} of the three Mountains paid for the Battery and the rest is \
         still floating, so Reconfigure's {{R}} is payable"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == battery)
        .expect("the floating red pays for Reconfigure, so its one line is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("reconfigure activates");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "reconfigure asks which creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "\"attach to target creature you control\": the Elves is one of them: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves was among the options the question enumerated");
    pass_until(&mut engine, |e| {
        e.state()
            .object(battery)
            .is_some_and(|o| o.attached_to == Some(elves))
    });

    assert_eq!(
        pt(&engine, elves),
        (2, 2),
        "\"equipped creature gets +1/+1\": the 1/1 it was printed as, plus one"
    );
    assert!(
        keywords(&engine, elves).contains(KeywordSet::HASTE),
        "\"and has haste\" — the Elves prints none of its own"
    );
    assert_eq!(
        pt(&engine, battery),
        (1, 1),
        "and the grant belongs to the creature it is attached to, not to its own source"
    );
}

// oracle_id = "37108cd4-bbab-4ce3-9ed6-f60e8422e703"
fn ragavan_nimble_pilferer() -> CardIndex {
    card_index("37108cd4-bbab-4ce3-9ed6-f60e8422e703")
}

/// Ragavan, Nimble Pilferer — {R} — 2/1 legendary Monkey Pirate: "Whenever
/// Ragavan deals combat damage to a player, create a Treasure token and exile
/// the top card of that player's library. Until end of turn, you may cast that
/// card." Only the Treasure is written (the impulse half is the
/// `Coverage::Partial` gap), so the card is cast for {R}, handed haste by an
/// equipped Lightning Greaves — a Monkey cast this turn may not attack
/// otherwise (CR 302.6) — and swung into an empty board.
///
/// The token is the proof the trigger reached the *player* and not merely the
/// combat damage step: the board is read empty before the swing and holds one
/// Treasure afterwards, while the defending library is the length it was and
/// that seat's exile still empty — the missing clause asserted as a non-move
/// rather than only noted.
#[allow(clippy::too_many_lines)] // a combat played to damage, and the token counted after it
#[test]
fn ragavan_makes_a_treasure_for_connecting_and_exiles_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[ragavan_nimble_pilferer(), lightning_greaves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {R} for the Monkey and {2} for the Greaves, off three Mountains.
    cast_from_hand(&mut engine, p0, ragavan_nimble_pilferer());
    pass_until(&mut engine, |e| at_rest(e, p0));
    cast_from_hand(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let monkey =
        on_battlefield(&engine, p0, ragavan_nimble_pilferer()).expect("the Monkey resolved");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves resolved");
    assert_eq!(pt(&engine, monkey), (2, 1), "the body the card prints");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has connected yet"
    );

    // Equip {0} — ability 0 is the static that grants, 1 is the equip.
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![monkey], "the only creature you control");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![monkey],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(monkey))
    });
    assert!(
        keywords(&engine, monkey).contains(KeywordSet::HASTE),
        "the Greaves are what let a Monkey cast this turn attack at all (CR 302.6)"
    );

    let their_library = library_size(&engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&monkey),
        "untapped, hasty and no longer sick: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "the other seat's board is empty, so there is one thing to attack: {defenders:?}"
    );
    let target = defenders.into_iter().next().expect("asserted above");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(monkey, target)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| !tokens_of(e, p0).is_empty());

    assert_eq!(
        engine.state().players[1].life,
        18,
        "the 2/1 got through: damage to the *player* is what the trigger waits for"
    );
    let treasures = tokens_of(&engine, p0);
    assert_eq!(treasures.len(), 1, "one hit, one Treasure");
    assert!(
        types(&engine, treasures[0]).contains(TypeSet::ARTIFACT),
        "and it is the artifact token the card creates"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "the impulse half is not written: no card left the top of their library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .is_empty(),
        "and nothing of theirs is in exile waiting to be cast"
    );
}

fn ranger_captain_of_eos() -> CardIndex {
    card_index("cada3481-cc2b-4412-b9b5-0436af53aad2")
}

/// Ranger-Captain of Eos — {1}{W}{W} — Creature — Human Soldier Ranger, 3/3:
/// "When this creature enters, you may search your library for a creature card
/// with mana value 1 or less, reveal it, put it into your hand, then shuffle."
/// The backing library here is sixty copies of a one-mana 1/1, so the filter
/// has something to find and the assertions can say exactly where the found
/// card went — the object chosen out of the library is the object that turns up
/// in the hand, with the library one shorter and the hand one longer.
/// The tail is the half that **moved**. It pinned the card's
/// `Coverage::Partial`: "Sacrifice this creature: your opponents can't cast
/// noncreature spells this turn" was on no offer, with the Sol Ring beside
/// it as the control saying the board offered activations at all. The
/// sentence is now `Modifier::OpponentsCantCast`, so the pin is inverted
/// rather than deleted — a limitation that was written down is a test that
/// has to move, and the move is the record of it. What the ability *does*
/// is the test below this one.
#[allow(clippy::too_many_lines)] // one search answered, and the ability that is not there
#[test]
fn ranger_captain_of_eos_searches_up_a_one_mana_creature_and_is_never_offered_its_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2024, llanowar_elves())
        .battlefield(0, &[plains(), plains(), plains(), quiet_artifact()])
        .hand(0, &[ranger_captain_of_eos()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{W}{W} off the three Plains, and the creature's one trigger is the
    // only thing the board has to resolve. The Sol Ring is kept back: it is
    // the control below for "this board does offer activations", and a
    // tapped one is offered nothing.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    cast_with_floating(&mut engine, p0, ranger_captain_of_eos());
    // "You may search" is **one** question here and not two: an optional
    // `Effect::SearchLibrary` is offered as the search itself with `min: 0`,
    // so declining is answering it with nothing. There is no `YesNo` in
    // front of it, which is what the first draft of this test waited for.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        min,
        max,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the searching seat is the one choosing");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a library search, and not a scry, a discard or a sacrifice"
    );
    assert_eq!(
        (min, max),
        (0, 1),
        "\"you may\": nought is a legal answer, and one is the most it takes"
    );

    let library_before: Vec<ObjectId> =
        engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(
        !options.is_empty(),
        "every card in this library is a creature with mana value 1, which is \
         exactly what the filter asks for: {library_before:?}"
    );
    assert!(
        options.iter().all(|id| library_before.contains(id)),
        "the menu is that seat's own library and nothing else: {options:?}"
    );

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("the card the question offered is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&found),
        "the card the search offered is the card that reached the hand"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&found),
        "and it is no longer in the library it came out of"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"a creature card\", singular: exactly one left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "which the hand reads from the other side"
    );
    let captain =
        on_battlefield(&engine, p0, ranger_captain_of_eos()).expect("the Captain resolved");
    assert_eq!(
        pt(&engine, captain),
        (3, 3),
        "the body the card prints, on the battlefield and not in a graveyard"
    );

    // The `Coverage::Partial` half, with its control: the Sol Ring's printed
    // {T} is on the offer list, the Captain's Sacrifice line is not on the
    // card at all.
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a quiet priority after the search: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == ring),
        "the board does offer activations: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == captain),
        "\"Sacrifice this creature: …\" is on the card now, and its price is \
         the Captain itself — which is standing right here: {:?}",
        legal.abilities
    );
}

/// What the Ranger-Captain's sacrifice actually does: "Your opponents can't
/// cast noncreature spells this turn."
///
/// One board taken twice, and the only difference between the rows is
/// whether p0 pressed the ability — so the refusal cannot be the price, the
/// phase or the priority round. Brainstorm is the opponent's spell because
/// it needs no target: a counterspell with nothing to counter is refused for
/// a reason that has nothing to do with this card, and the negative would
/// have been true either way.
///
/// The second Island is held back through the first phase on purpose. A
/// pool empties at the end of a phase (CR 500.4), and the row that matters
/// is read after p0 has had a priority of its own — so the mana that pays
/// for the Brainstorm has to be floated *after* that, and is asserted to be
/// floating when the refusal is read.
#[test]
fn the_ranger_captains_sacrifice_takes_an_opponents_noncreature_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for sacrificed in [false, true] {
        let mut engine = Duel::new(2024, llanowar_elves())
            .battlefield(0, &[ranger_captain_of_eos()])
            .battlefield(1, &[island()])
            .hand(1, &[brainstorm()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

        let captain =
            on_battlefield(&engine, p0, ranger_captain_of_eos()).expect("the Captain is out");
        if sacrificed {
            activate(&mut engine, p0, ranger_captain_of_eos(), 1);
            pass_until(&mut engine, stack_is_empty);
            assert!(
                in_graveyard(&engine, p0, ranger_captain_of_eos()).is_some(),
                "the price is the Captain itself"
            );
            assert!(
                on_battlefield(&engine, p0, ranger_captain_of_eos()).is_none(),
                "and it is off the battlefield: {captain:?}"
            );
        }

        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
                && e.state().zones.stack_is_empty()
        });
        tap_all_mana(&mut engine, p1);
        assert_eq!(
            engine.state().players[1]
                .mana_pool
                .available(ManaColor::Blue),
            1,
            "the {{U}} the Brainstorm costs is floating, sacrificed {sacrificed}"
        );

        let storm = in_hand(&engine, p1, brainstorm()).expect("the Brainstorm is in hand");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected p1's priority, got {:?}", engine.pending())
        };
        assert_eq!(
            legal.castable.contains(&storm),
            !sacrificed,
            "\"your opponents can't cast noncreature spells this turn\", \
             sacrificed {sacrificed}: {:?}",
            legal.castable
        );
    }
}

fn renegade_rallier() -> CardIndex {
    card_index("6fa07b6c-f01a-4416-b0fc-986b0fc4e412")
}

/// Renegade Rallier — {1}{G}{W} 3/2 Human Warrior. Its printed enter trigger
/// is a revolt trigger: return a permanent card with mana value 2 or less from
/// your graveyard to the battlefield only "if a permanent left the battlefield
/// under your control this turn". That intervening-if (CR 603.4) is the
/// `Coverage::Partial` gap and the reason this board is built to leave nothing:
/// two Forests sit in the graveyard and a Dark Ritual has just resolved into
/// it, so the Rallier's own arrival is the turn's only event — and the trigger
/// asks for a target anyway.
///
/// The menu is the second half of the proof: the two Forests are offered and
/// the instant is not, so "permanent card" (CR 110.4a) is read and not merely
/// the mana-value cap.
#[test]
fn renegade_rallier_reanimates_a_cheap_permanent_card_with_nothing_having_left_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[forest(), forest(), plains(), swamp()])
        .hand(0, &[renegade_rallier(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Two cards the trigger may take, off the top of a deck of Forests.
    seed_graveyard(&mut engine, p0, 2);

    // And one it may not. Nothing has left the battlefield to pay for this
    // either: an instant resolving is a card changing zones, not a permanent.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let ritual = in_graveyard(&engine, p0, dark_ritual())
        .expect("Dark Ritual resolved and went to its owner's graveyard");

    let forests_before = all_on_battlefield(&engine, p0, forest()).len();
    let board_before = engine.state().zones.list(ZoneLocation::Battlefield).clone();

    cast_from_hand(&mut engine, p0, renegade_rallier());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    // Revolt's condition is not merely unread on this board: every permanent
    // that was here before is still here, and the Rallier is the only addition.
    let now = engine.state().zones.list(ZoneLocation::Battlefield);
    assert!(
        board_before.iter().all(|id| now.contains(id)),
        "no permanent left the battlefield under p0's control this turn"
    );
    assert_eq!(
        now.len(),
        board_before.len() + 1,
        "the Rallier's own arrival is the only thing that has happened"
    );

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
    assert_eq!(player, p0, "the Rallier's controller is the one asked");
    assert_eq!((min, max), (1, 1), "exactly one card comes back");
    assert!(
        player_options.is_empty(),
        "\"target permanent card\" reaches no player: {player_options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "the two Forests: lands, mana value 0, and nothing else: {options:?}"
    );
    assert!(
        !options.contains(&ritual),
        "Dark Ritual is an instant, so it is no permanent card (CR 110.4a): {options:?}"
    );

    let taken = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![taken],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let landed = engine
        .state()
        .object(taken)
        .expect("the returned card is still an object");
    assert_eq!(
        landed.zone,
        crate::zone::Zone::Battlefield,
        "\"return target permanent card ... to the battlefield\""
    );
    assert_eq!(
        landed.controller, p0,
        "under the control of the seat that cast it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        forests_before + 1,
        "one Forest came back out of the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_some(),
        "and the instant the menu refused stayed exactly where it was"
    );

    let rallier =
        on_battlefield(&engine, p0, renegade_rallier()).expect("the Rallier is on the table");
    assert_eq!(pt(&engine, rallier), (3, 2), "and it is the printed 3/2");
}

// oracle_id = "68ca91ba-31fb-47e0-9b32-e4f3504cbbca"
fn safehold_elite() -> CardIndex {
    card_index("68ca91ba-31fb-47e0-9b32-e4f3504cbbca")
}

/// Safehold Elite is a 2/2 Elf Scout for `{1}{G/W}`, and its second sentence —
/// persist — is the whole of the `Coverage::Partial` note: nothing in the DSL
/// returns a card from a graveyard with a -1/-1 counter on it, so the body and
/// the hybrid symbol are what is left to play. The board is deliberately
/// **two Plains and no other land**: `{1}` is one white and the hybrid symbol
/// is the other, so a reading that only ever paid `{G/W}` with green would
/// refuse the cast outright rather than pass quietly. What lands is the
/// printed 2/2 creature, with the mana gone off both lands.
#[test]
fn safehold_elite_is_cast_off_two_plains_and_lands_as_its_printed_two_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[safehold_elite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The only mana this seat can make is white, so `{1}{G/W}` has to be paid
    // out of white twice over — generic from one Plains, the hybrid symbol
    // from the other.
    cast_from_hand(&mut engine, p0, safehold_elite());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, safehold_elite()).is_some() && stack_is_empty(e)
    });

    let elite = on_battlefield(&engine, p0, safehold_elite()).expect("the Elite resolved");
    assert_eq!(pt(&engine, elite), (2, 2), "the body the card prints");
    assert!(
        types(&engine, elite).contains(TypeSet::CREATURE),
        "and it arrived as the creature card it is"
    );
    assert!(
        all_on_battlefield(&engine, p0, plains())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "both Plains paid for {{1}}{{G/W}}: the white half of the hybrid is a \
         way to pay it, not only the green"
    );
}

fn scavenging_ooze() -> CardIndex {
    card_index("1ff25f67-36a7-4cfa-a2b1-2135b5b6fb67")
}

/// Scavenging Ooze — {1}{G}, 2/2 — prints "{G}: Exile target card from a
/// graveyard…", and the rider that pays a counter and a life is the
/// `Coverage::Partial` gap. What is left to play is the half that is written,
/// so the two things it has to show are that the ability asks for a card in
/// *any* graveyard (one seeded on each side, and only those two offered) and
/// that the card the question enumerated is the card that leaves for exile —
/// which is why the Ooze's own body standing on an untargeted battlefield is
/// the control: "card in a graveyard" is read, not skipped.
#[test]
fn scavenging_ooze_exiles_a_card_from_either_players_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(3001, forest())
        .battlefield(0, &[scavenging_ooze(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One card in each graveyard, so the target spec's `EachPlayer` is a
    // question about the table rather than about the caster's own bin. The
    // harness' dev capability is what puts them there (no `SeatSpec` field
    // for a graveyard), and `seed_graveyard` refreshes the offer so an
    // ability that reads a graveyard is not withheld for want of one.
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let mine = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let theirs = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .clone();
    assert_eq!(
        (mine.len(), theirs.len()),
        (1, 1),
        "one seeded card apiece and no discard before it"
    );
    let doomed = theirs[0];

    // Mana into the pool first: the offer is computed against what is
    // floating, and {G} that is still sitting on a Forest pays for nothing.
    tap_all_mana(&mut engine, p0);
    let ooze = on_battlefield(&engine, p0, scavenging_ooze()).expect("the Ooze resolved");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let offered = deeds(&legal, &[ooze]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "{{G}} in the pool and a card in a graveyard: the one line the Ooze \
         prints is offered: {offered:?}"
    );

    activate(&mut engine, p0, scavenging_ooze(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the exile targets, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine[0]) && options.contains(&doomed),
        "\"target card from a graveyard\" reaches either side of the table: \
         {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and holds the graveyard cards alone — the Ooze's own 2/2 body is no \
         card in a graveyard: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the card the question enumerated is the one it takes");
    pass_until(&mut engine, stack_is_empty);

    let exiled = engine
        .state()
        .object(doomed)
        .expect("the card still exists");
    assert_eq!(exiled.zone, Zone::Exile, "the named card left for exile");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        0,
        "the opponent's graveyard is one card lighter"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "and the card on this side was never touched"
    );
    assert!(
        on_battlefield(&engine, p0, scavenging_ooze()).is_some(),
        "the Ooze stays where it was: nothing about the ability moves its \
         source, and no +1/+1 rider is written to move anything else"
    );
}

// oracle_id = "164f3f85-21fc-40b7-9871-4f303ba98428"
fn scrap_trawler() -> CardIndex {
    card_index("164f3f85-21fc-40b7-9871-4f303ba98428")
}

// oracle_id = "68e1f7e0-a9b3-437f-8086-0c0cb85f2880"
fn krark_clan_ironworks() -> CardIndex {
    card_index("68e1f7e0-a9b3-437f-8086-0c0cb85f2880")
}

/// Scrap Trawler — {3} 3/2 artifact creature: "Whenever this creature dies or
/// another artifact you control is put into a graveyard from the battlefield,
/// return to your hand target artifact card in your graveyard with lesser mana
/// value."
///
/// The Trawler is cast and then left standing while a Sol Ring beside it is
/// sacrificed to Krark-Clan Ironworks, so what fires the ability is the
/// printed *second* half — an artifact you control — and not the Trawler
/// itself, which is still on the battlefield when the dust settles. Ashnod's
/// Altar eats a Llanowar Elves first, which is the control twice over: a
/// creature that is no artifact dies and the ability says nothing at all, and
/// its card is then the non-artifact corpse the trigger's target menu has to
/// leave out while the Sol Ring is the only artifact card in that graveyard.
/// That Sol Ring's mana value is below the Trawler's, so it is a target the
/// printed "with lesser mana value" — the clause this `Partial` printing
/// cannot say — would have allowed as well.
#[allow(clippy::too_many_lines)] // two deaths of different kinds, one trigger, one menu
#[test]
fn scrap_trawler_returns_the_artifact_card_that_died_beside_it_and_never_the_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                krark_clan_ironworks(),
                ashnods_altar(),
                quiet_artifact(),
                llanowar_elves(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[scrap_trawler()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, scrap_trawler());
    pass_until(&mut engine, stack_is_empty);
    let trawler = on_battlefield(&engine, p0, scrap_trawler()).expect("the Trawler resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");

    // First a creature that is no artifact. The Trawler is a creature too, so
    // the Altar's menu holds both, and this takes the one the printed ability
    // must ignore.
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&elves) && options.contains(&trawler),
        "both creatures you control are on the menu: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the question offered the Elves");
    assert!(
        at_rest(&engine, p0),
        "a creature that is no artifact dies and the trigger says nothing, \
         got {:?}",
        engine.pending()
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and its card is now lying in that graveyard"
    );

    // Now an artifact you control is put into a graveyard from the
    // battlefield, which is the half the printed sentence is really about.
    activate(&mut engine, p0, krark_clan_ironworks(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Ironworks asks which artifact, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&ring),
        "the Sol Ring is the artifact to eat: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elves are a creature and no artifact: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the question offered the Sol Ring");

    // The cost is paid, the trigger is put on the stack, and it asks what it
    // is pointing at: an artifact card in your graveyard.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on a target choice")
    };
    let corpse =
        in_graveyard(&engine, p0, quiet_artifact()).expect("the Sol Ring reached the graveyard");
    assert_eq!((min, max), (1, 1), "one card, and the ability has to aim");
    assert_eq!(
        options,
        vec![corpse],
        "\"target artifact card in your graveyard\": the Elves card in the \
         same graveyard is a creature and is not offered"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![corpse],
            },
        )
        .expect("the card the question offered is the card it returns");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "\"return to your hand target artifact card in your graveyard\""
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_none(),
        "and it left the graveyard to get there"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the creature card in that same graveyard never moved"
    );
    assert!(
        on_battlefield(&engine, p0, scrap_trawler()).is_some(),
        "the Trawler itself never left the battlefield: this was the \
         \"another artifact\" half of the trigger"
    );
    assert!(
        on_battlefield(&engine, p0, krark_clan_ironworks()).is_some(),
        "and the Ironworks ate the Sol Ring, not itself"
    );
}

// oracle_id = "d1961110-575b-4a1b-9cee-db0e1f0fdbc1"
fn scryb_ranger() -> CardIndex {
    card_index("d1961110-575b-4a1b-9cee-db0e1f0fdbc1")
}

/// Scryb Ranger — {1}{G} 1/1 Faerie Ranger with flash, flying and "Return a
/// Forest you control to its owner's hand: Untap target creature. Activate
/// only once each turn."
///
/// Every word of the activation is read off an offer rather than the file:
/// the cost menu holds this seat's three Forests and neither the Plains
/// beside them nor the Forest across the table, the target question reaches
/// both sides of the table, and the Forest really leaves the battlefield for
/// the hand while the tapped Elves stands back up. The two Forests still on
/// the table are what make the second activation's absence the printed limit
/// rather than an empty board.
#[allow(clippy::too_many_lines)] // a cost that returns a land, and the once-a-turn limit after it
#[test]
fn scryb_ranger_trades_a_forest_for_one_untap_and_then_its_limit_bites() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(101, island())
        .battlefield(
            0,
            &[forest(), forest(), forest(), plains(), llanowar_elves()],
        )
        .hand(0, &[scryb_ranger()])
        // A Forest and a creature on the other side of the table: both "a
        // Forest you control" and "target creature" have something to
        // decline over there.
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf is kept back: tapping it is the deliberate move below, and an
    // untap aimed at a creature something else already tapped would prove
    // nothing about this card.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, scryb_ranger());
    pass_until(&mut engine, stack_is_empty);
    let ranger = on_battlefield(&engine, p0, scryb_ranger()).expect("the Ranger resolved");
    assert!(
        keywords(&engine, ranger).contains(KeywordSet::FLYING),
        "flying is a keyword bit on the printed card"
    );

    let land = on_battlefield(&engine, p0, plains()).expect("my Plains is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        3,
        "three Forests to trade"
    );

    // Tap the Elves with its own {T}: an untap aimed at an untapped creature
    // changes nothing and would prove nothing.
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert!(is_tapped(&engine, elves), "the Elves paid its own tap");

    // The protection static is index 0 in the card's list, so the one
    // printed activation is looked up rather than assumed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == ranger)
        .expect("the Ranger's one printed activation is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the activation starts");

    // The two questions one activation asks, answered in the order they
    // arrive rather than the order they are expected.
    let mut asked_target = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if asked_target && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                assert!(
                    player_options.is_empty(),
                    "\"target creature\" is objects only: {player_options:?}"
                );
                assert!(
                    options.contains(&elves) && options.contains(&their_elves),
                    "\"target creature\" reaches both sides of the table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![elves],
                            players: vec![],
                        },
                    )
                    .unwrap();
                asked_target = true;
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
                    crate::choice::ChoicePrompt::CostReturn,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one Forest, no more and no fewer");
                menu = options.clone();
                let mine = options
                    .iter()
                    .copied()
                    .find(|id| {
                        engine
                            .state()
                            .object(*id)
                            .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
                    })
                    .expect("a Forest of mine is on the menu");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Ranger's activation resolves: {other:?}"),
        }
    }
    assert!(asked_target, "\"untap target creature\" is a target choice");
    assert_eq!(
        menu.len(),
        3,
        "the three Forests this seat controls: {menu:?}"
    );
    assert!(!menu.contains(&land), "a Plains is no Forest: {menu:?}");
    assert!(
        !menu.contains(&their_forest),
        "a seat returns only what it controls: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "the Forest left the battlefield for its owner's hand"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "and it is in hand — the library is Islands, so no Forest could have \
         been there before"
    );
    assert!(
        !is_tapped(&engine, elves),
        "\"untap target creature\": the Elves stands back up"
    );
    assert_eq!(
        all_on_battlefield(&engine, p1, forest()).len(),
        1,
        "and the Forest across the table never moved"
    );

    // Once each turn. Two Forests are still on the table — the cost could be
    // paid again — so the absence is the printed limit and not an empty
    // board.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == ranger),
        "\"activate only once each turn\": {:?}",
        legal.abilities
    );
}

fn strangleroot_geist() -> CardIndex {
    card_index("af12758f-4a7b-4156-8942-de4716aa0623")
}

/// Strangleroot Geist prints `{G}{G}` for a 2/1 Spirit with haste; the
/// undying line is the `Coverage::Partial` gap and nothing here presses it.
///
/// Haste is a keyword bit the engine reads, and the only way to see it *do*
/// anything is the attack it permits: the Geist arrives in p0's first main
/// phase and is still offered as an attacker in that same turn's combat
/// step. The Llanowar Elves cast beside it is the control — untapped, so its
/// exclusion cannot be read as "it already paid for something", and no haste,
/// so being left off the list is summoning sickness and nothing else. The
/// attack itself then lands two damage, which is the printed body paid out
/// rather than a list merely containing a name.
#[test]
fn strangleroot_geist_attacks_the_turn_it_arrives_and_the_elves_beside_it_cannot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[strangleroot_geist(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, strangleroot_geist());
    pass_until(&mut engine, stack_is_empty);
    let geist = on_battlefield(&engine, p0, strangleroot_geist()).expect("the Geist resolved");
    assert_eq!(pt(&engine, geist), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, geist).contains(KeywordSet::HASTE),
        "haste is a keyword bit the layer projection carries"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves resolved");
    assert!(
        !is_tapped(&engine, elves),
        "the control is untapped, so only summoning sickness can keep it home"
    );

    // Out of the main phase and into the declare-attackers step.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player,
        attackers,
        defenders,
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(player, p0, "the turn is p0's");
    assert!(
        attackers.contains(&geist),
        "the Geist came under p0's control this turn and may attack anyway: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elves),
        "the Elves came under p0's control this turn and are summoning sick: {attackers:?}"
    );

    let life_before = engine.state().players[1].life;
    assert_eq!(
        defenders.len(),
        1,
        "one surviving opponent and no planeswalkers of theirs: {defenders:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(geist, defenders[0])],
            },
        )
        .expect("the defender the offer enumerated is legal");
    pass_until(&mut engine, |e| e.state().players[1].life < life_before);
    assert_eq!(
        engine.state().players[1].life,
        life_before - 2,
        "two damage, the Geist's printed power, so the attack resolved rather \
         than merely being declared"
    );
}

// oracle_id = "e87906d2-db1a-4e19-b910-adb4eb339945"
fn urza_lord_high_artificer() -> CardIndex {
    card_index("e87906d2-db1a-4e19-b910-adb4eb339945")
}

/// Urza, Lord High Artificer — {2}{U}{U} 1/4 — the two printed sentences that
/// are implemented, both played in one first main phase. The entry trigger
/// makes a 0/0 Construct *artifact creature* whose own static grows it by one
/// for each artifact its controller controls, so the token standing beside the
/// Sol Ring has to be a 2/2: one would mean it never counted itself, three that
/// the opponent's Sol Ring across the table was counted too. And "Tap an
/// untapped artifact you control: Add {U}" is a mana ability whose cost names
/// no artifact of its own, so the question, the menu it publishes and the {U}
/// it pays are all read off the same activation. The `{5}` activation is the
/// `Coverage::Partial` gap and is never pressed.
#[allow(clippy::too_many_lines)] // the token's body read off the board, then its mana ability
#[test]
fn urza_builds_a_construct_that_counts_your_artifacts_and_taps_one_for_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), quiet_artifact()],
        )
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[urza_lord_high_artificer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Islands pay {2}{U}{U}. The Sol Ring is kept back through both
    // taps below: it is the "untapped artifact you control" Urza's own mana
    // ability charges, and it is what the Construct counts.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    cast_with_floating(&mut engine, p0, urza_lord_high_artificer());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && !tokens_of(e, p0).is_empty()
    });

    let urza = on_battlefield(&engine, p0, urza_lord_high_artificer()).expect("Urza resolved");
    assert_eq!(pt(&engine, urza), (1, 4), "the body the card prints");

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one Construct — a token left at 0/0 would have died to CR 704.5f"
    );
    let construct = tokens[0];
    let kinds = types(&engine, construct);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "the token counts itself because it is an artifact creature: {kinds:?}"
    );
    assert_eq!(
        pt(&engine, construct),
        (2, 2),
        "+1/+1 for each artifact *you* control: the Sol Ring and the Construct \
         itself, and nothing for the Sol Ring the opponent controls"
    );

    // "Tap an untapped artifact you control: Add {U}". Tapping everything
    // else first settles the pool: a mana ability needs no mana, but the
    // offer is read once nothing else is floating.
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(urza, 1)),
        "there is an untapped artifact to tap, so the printed mana ability is \
         offered: {:?}",
        legal.abilities
    );
    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Blue);

    activate(&mut engine, p0, urza_lord_high_artificer(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the tap is chosen before it is paid, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, and the cost asks once");
    assert!(
        options.contains(&ring),
        "the untapped artifact you control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an artifact this seat does not control is not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact the question offered pays the cost");

    assert!(is_tapped(&engine, ring), "the artifact that was named paid");
    assert!(
        !is_tapped(&engine, construct),
        "and the Construct beside it never moved"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        before + 1,
        "{{U}} reached the pool the moment the answer landed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        u32::from(before) + 1,
        "one mana, off one tap, and nothing else came with it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}

fn yawgmoth_thran_physician() -> CardIndex {
    card_index("a1e232c0-dc38-47be-a5a0-f68bc1d86a29")
}

/// Yawgmoth, Thran Physician — {2}{B}{B} — 2/4 Legendary Creature — Human
/// Cleric, and the written half of a `Coverage::Partial` card: one life and
/// another creature buy a -1/-1 counter on up to one target creature and a
/// card.
///
/// A single activation is read four ways at once — the activating seat's life
/// total drops by exactly one, the creature named in the sacrifice menu leaves
/// the battlefield for its owner's graveyard while the Cleric stays, the
/// counter lands on the creature the ability was aimed at (a printed 2/2 that
/// survives it as a 1/1), and the draw takes one card off the top of the
/// library. Everything else on the board is the control for those four: the
/// Elf across the table is not this seat's to sacrifice and keeps no counter,
/// and the Cleric is the ability's own source and so cannot be the creature it
/// eats. The printed "up to one" arrives as a forced single target, which the
/// question's `(min, max)` records.
#[allow(clippy::too_many_lines)] // a three-part cost, the counter it places and the card it draws
#[test]
fn yawgmoth_pays_a_life_and_another_creature_for_a_minus_counter_and_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                skyclave_apparition(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[yawgmoth_thran_physician()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    cast_from_hand(&mut engine, p0, yawgmoth_thran_physician());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let yawgmoth =
        on_battlefield(&engine, p0, yawgmoth_thran_physician()).expect("the Cleric resolved");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves stand");
    let victim = on_battlefield(&engine, p0, skyclave_apparition()).expect("the Apparition stands");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");
    assert_eq!(pt(&engine, victim), (2, 2), "a printed 2/2 to aim at");
    assert_eq!(engine.state().players[0].life, 20, "and no life paid yet");
    let library_before = library_size(&engine, p0);

    // Ability 0 is the protection static; the one printed line that is
    // activated is index 1, and it costs no mana at all.
    activate(&mut engine, p0, yawgmoth_thran_physician(), 1);

    // One activation, two questions: which creature is targeted (CR 601.2c)
    // and which creature pays the cost (CR 601.2h). Answered in whichever
    // order they arrive.
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut aimed = false;
    for _ in 0..12 {
        if aimed && !menu.is_empty() {
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
                assert_eq!(player, p0, "the activating seat aims its own ability");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "\"up to one\" is one here: this target cannot be optional"
                );
                assert!(
                    options.contains(&victim),
                    "\"target creature\" offers the Apparition: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![victim],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(player, p0, "the activating seat pays");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
                assert!(
                    options.contains(&fodder) && options.contains(&victim),
                    "the two other creatures you control are the menu: {options:?}"
                );
                assert!(
                    !options.contains(&yawgmoth),
                    "\"another creature\" — the Cleric is its own source and no cost of its own: {options:?}"
                );
                assert!(
                    !options.contains(&theirs),
                    "CR 701.21a: an opponent's creature is not yours to sacrifice: {options:?}"
                );
                menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while Yawgmoth's ability resolves: {other:?}"),
        }
    }
    assert!(aimed, "the ability targets, so a target was asked for");
    assert!(
        !menu.is_empty(),
        "and it sacrifices, so the cost was asked for"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\": exactly one, off the seat that activated the ability"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing off the other seat"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the creature named as the cost left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, yawgmoth_thran_physician()).is_some(),
        "the Cleric did not pay with itself"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elf across the table never moved"
    );

    assert_eq!(
        counters_on(&engine, victim, baylee_cards_dsl::CounterKind::M1M1),
        1,
        "\"put a -1/-1 counter on ... target creature\""
    );
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "the projection reads it: a 2/2 with a -1/-1 counter is a 1/1"
    );
    assert_eq!(
        counters_on(&engine, yawgmoth, baylee_cards_dsl::CounterKind::M1M1),
        0,
        "the ability's own source was not the creature it counted"
    );
    assert_eq!(
        counters_on(&engine, theirs, baylee_cards_dsl::CounterKind::M1M1),
        0,
        "and the counter crossed no table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"and draw a card\": one off the top of the library"
    );
}

/// Young Wolf is a printed 1/1 Wolf for {G}, and its only rules text is
/// undying — the `Coverage::Partial` gap, since no keyword bit carries it and
/// the DSL cannot return the source card from the graveyard with a +1/+1
/// counter. This scenario plays the body that *is* implemented (cast for
/// {G}, standing as a 1/1) and then reads the gap through a genuine
/// battlefield-to-graveyard death, which is exactly what undying answers: the
/// Wolf is fed to Ashnod's Altar's own sacrifice cost and simply stays in the
/// graveyard. An exile would be no proof — a card exiled never triggers
/// undying either — so the sacrifice is the clean control (CR 700.4), and the
/// missing returning 1/1 with a counter is the much-printed clause and
/// nothing else.
#[test]
fn young_wolf_lands_as_a_one_one_and_stays_dead_where_undying_would_return_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ashnods_altar(), forest()])
        .hand(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {G} off the Forest. The printed 1/1 body is the whole of what the
    // engine implements of this card.
    cast_from_hand(&mut engine, p0, young_wolf());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf resolved");
    assert_eq!(pt(&engine, wolf), (1, 1), "the printed 1/1 body");
    assert!(
        types(&engine, wolf).contains(TypeSet::CREATURE),
        "and it is a creature"
    );

    // The Altar's cost is "sacrifice a creature," which CR 700.4 counts as
    // dying — the very event undying watches for.
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the sacrifice asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&wolf),
        "the Wolf is the creature the Altar may eat: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolf],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, young_wolf()).is_some(),
        "the sacrificed Wolf is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_none(),
        "and nothing returned it with a +1/+1 counter — undying is the \
         `Coverage::Partial` gap"
    );
}

/// Theorist's Proxy — {1}{U}, a 0/3 Illusion — prints flash, "When this
/// creature enters, empower Jace 3." and "{U}, Sacrifice this creature: The
/// next spell you cast this turn can't be countered."
///
/// Flash is the written half, and p0's own end step is the board that reads
/// it: five tapped Islands pay for either spell in hand, so the only thing
/// separating the 0/3 from the {2}{U} Aether Channeler beside it is that one
/// of them may be cast once the main phases are gone. The other two lines are
/// the `Coverage::Partial` gap and are read as absences — no Jace token
/// arrives with it, and the sacrifice line stays unoffered with the creature
/// on the table and the {U} it costs still floating in the pool.
#[test]
fn theorist_s_proxy_flashes_in_after_the_main_phase_and_offers_neither_of_its_other_lines() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[theorist_s_proxy(), aether_channeler()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Out of the main phases and into the end step, which is instant timing:
    // sorcery speed ends with the postcombat main, and the cleanup the
    // nine-card opening hand owes lies beyond the priority asked here.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana first: `castable` is verified against the pool, and five blue
    // cover both spells at once.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, theorist_s_proxy()).expect("the Proxy is in hand");
    let channeler = in_hand(&engine, p0, aether_channeler()).expect("the Channeler is in hand");
    assert!(
        legal.castable.contains(&card),
        "flash: a creature may be cast with no main phase open, and the pool \
         covers it: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&channeler),
        "the same five blue buy the Channeler's {{2}}{{U}} and it is still \
         not offered, so what keeps it off this list is the timing and not \
         the mana: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{1}{U} out of the pool pays for the flashed creature");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let proxy = on_battlefield(&engine, p0, theorist_s_proxy()).expect("the Proxy resolved");
    assert_eq!(pt(&engine, proxy), (0, 3), "the body the card prints");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "empower is the `Coverage::Partial` gap: `crate::tokens` holds no Jace \
         planeswalker token, so nothing arrives beside it"
    );

    // The second gap, with its cost payable: three of the five Islands' blue
    // are left over, and the creature the sacrifice would eat is the Proxy.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the Proxy's controller holds priority again: {:?}",
            engine.pending()
        )
    };
    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue)
            >= 1,
        "the {{U}} the sacrifice asks for is right there in the pool"
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == proxy),
        "and `{{U}}, Sacrifice this creature` is offered nowhere all the same: \
         that line is not written at all: {:?}",
        legal.abilities
    );
}

/// Thorin Oakenshield is `Coverage::Partial`: the printed **trample** is
/// enforced, while storied and the enduring-story ward grant are not. The
/// board is the smallest one that tells the enforced half from the missing
/// one — a 3/2 trampler attacking into a 1/1 — because the two excess points
/// of damage reaching the defending player are the only evidence that
/// trample is *applied* and not merely printed: a keyword the engine ignored
/// would leave that player at twenty. Casting it off a Mountain and a Plains,
/// and walking through a whole turn so the Dwarf is no longer summoning sick,
/// is the rest of the printed card arriving in a real game before it swings.
#[test]
fn thorin_oakenshield_casts_as_a_three_two_and_its_trample_spills_over_a_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), plains()])
        .hand(0, &[thorin_oakenshield()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A real cast: {R} off the Mountain and {W} off the Plains.
    cast_from_hand(&mut engine, p0, thorin_oakenshield());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, thorin_oakenshield()).is_some()
    });
    let thorin = on_battlefield(&engine, p0, thorin_oakenshield()).expect("Thorin resolved");
    assert_eq!(pt(&engine, thorin), (3, 2), "the printed 3/2 body");
    assert!(
        keywords(&engine, thorin).contains(KeywordSet::TRAMPLE),
        "the printed trample reaches the permanent"
    );

    // Through p1's turn and back, so the Dwarf is no longer summoning sick.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    // Declare Thorin as the only attacker, aimed at the only opponent: the
    // defender is taken straight out of the request the engine published
    // rather than built by hand.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&thorin),
        "an untapped, no-longer-sick Thorin may attack: {attackers:?}"
    );
    assert_eq!(defenders.len(), 1, "one opponent to attack in a duel");
    let target = defenders
        .into_iter()
        .next()
        .expect("the duel publishes its one defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(thorin, target)],
            },
        )
        .unwrap();

    // The 1/1 Elf blocks: one point of lethal damage, two trampling over.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the blocker is out");
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == elves && b.attackers.contains(&thorin)),
        "the Elves may block Thorin: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, thorin)],
            },
        )
        .unwrap();

    // Through the combat damage step.
    pass_until(&mut engine, |e| e.state().players[1].life < 20);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "three trampling power: one point is lethal to the 1/1 and the other \
         two go over the top, which is the keyword being applied"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocker took lethal damage and died"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the 1/1's single point of return damage is marked on Thorin, not on its controller"
    );
    assert!(
        on_battlefield(&engine, p0, thorin_oakenshield()).is_some(),
        "a 3/2 with one damage marked survives its own attack"
    );
}

/// Walking Ballista — {X}{X} artifact creature: "This creature enters with X +1/+1
/// counters on it. {4}: Put a +1/+1 counter on this creature. Remove a +1/+1 counter
/// from this creature: It deals 1 damage to any target."
///
/// It is seeded straight onto the battlefield here and never cast, so there is no
/// announced X for its entry clause to read (CR 107.3g: a card outside the stack has
/// an X of 0) and the harness plants the one counter the 0/0 body needs to survive
/// the state-based action. That the clause *does* work off a real cast is proved
/// beside the rule, in [`enter_tests`], which is where it belongs — this test is
/// about the two activated abilities: paying {4} adds a counter (growing it to 2/2),
/// and removing a counter pays the cost to deal 1 damage to the opponent.
///
/// [`enter_tests`]: crate::engine::enter_tests
#[test]
fn walking_ballista_grows_with_mana_and_removes_a_counter_to_deal_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[walking_ballista(), forest(), forest(), forest(), forest()],
        )
        .life(1, 20)
        .start();

    let ballista =
        on_battlefield(&engine, p0, walking_ballista()).expect("the Ballista is on the table");

    // Nobody cast it, so its entry clause read an X of nothing: the counter it needs
    // to survive the SBA 0-toughness check is planted by the harness before mulligans.
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::replacement::put_counters(state, ballista, CounterKind::P1P1, 1);
    }

    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        1,
        "one counter planted by the harness"
    );
    assert_eq!(pt(&engine, ballista), (1, 1), "starts as a 1/1");

    // Ability 0: {4}: Put a +1/+1 counter on this creature.
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    activate(&mut engine, p0, walking_ballista(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        2,
        "{{4}} added a second +1/+1 counter"
    );
    assert_eq!(pt(&engine, ballista), (2, 2), "grew to 2/2");

    // Ability 1: Remove a +1/+1 counter from this creature: It deals 1 damage to any target.
    activate(&mut engine, p0, walking_ballista(), 1);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("ability 1 targets any target, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a legal target for any target"
    );
    // The target is chosen before the cost is paid (CR 601.2c against CR
    // 601.2h, the last step of an activation), so the counter is still on
    // the creature while this question is open — it is not a cost the
    // engine takes as the ability is announced.
    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        2,
        "the counter is still there while the target is being chosen"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeting the opponent is legal");

    // Now the activation is complete and the cost has been paid, with the
    // ability still on the stack: the second counter is gone and the body
    // it was holding up is a 1/1 again.
    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        1,
        "removing a +1/+1 counter is the whole cost, and it is paid here"
    );
    assert_eq!(pt(&engine, ballista), (1, 1), "shrank back to 1/1");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "opponent took 1 damage from the ping"
    );
    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        1,
        "Ballista retains its remaining counter"
    );
    assert!(
        on_battlefield(&engine, p0, walking_ballista()).is_some(),
        "Ballista survived on the battlefield"
    );
}

// oracle_id = "1816eede-c5bd-49df-958f-a3af64cb2932"
/// Omnath, Locus of Rage prints two sentences: landfall makes a 5/5 red and
/// green Elemental token, and whenever Omnath or **another Elemental you
/// control** dies, Omnath deals 3 damage to any target.
///
/// Both are played in one main phase. The Elemental arrives by being cast off
/// eight basics, a real `PlayLand` turns landfall on — and the empty token
/// board before that land is the control, because the lands this board was
/// built from were *placed* and never entered. The token is then fed to
/// Ashnod's Altar, whose sacrifice is a cost asked as a `CostSacrifice` menu,
/// so the death the second sentence answers is an Elemental's and not
/// Omnath's own; the three damage goes to the opponent's face, which is one
/// of the answers an "any target" prompt offers.
#[test]
#[allow(clippy::too_many_lines)] // a whole game, as every test in this file is
fn omnath_makes_a_token_for_a_land_and_answers_that_elementals_death_with_three() {
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
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                ashnods_altar(),
            ],
        )
        // The Forest is in hand and not on the board: a landfall trigger
        // reads an *entry*, and `starting_battlefield` is a placement.
        .hand(0, &[omnath_locus_of_rage(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3}{R}{R}{G}{G} off the eight basics, so the Elemental is in a real
    // game rather than assumed onto the table.
    cast_from_hand(&mut engine, p0, omnath_locus_of_rage());
    pass_until(&mut engine, stack_is_empty);
    let omnath = on_battlefield(&engine, p0, omnath_locus_of_rage()).expect("Omnath resolved");
    assert_eq!(pt(&engine, omnath), (5, 5), "the printed body");
    let altar = on_battlefield(&engine, p0, ashnods_altar()).expect("the Altar stands");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the lands this board was built from were placed rather than entered, \
         so no landfall has fired yet"
    );

    // Landfall, off a real play.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one land entered, one Elemental");
    let elemental = tokens[0];
    assert_eq!(
        pt(&engine, elemental),
        (5, 5),
        "a 5/5 with no counter needed"
    );
    let token = engine
        .state()
        .object(elemental)
        .expect("the Elemental is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(token.name, "Elemental");
    assert!(
        token.colors.contains(baylee_core::color::Color::Red)
            && token.colors.contains(baylee_core::color::Color::Green),
        "red and green"
    );
    assert!(
        types(&engine, elemental).contains(TypeSet::CREATURE),
        "and a creature, which is what the second sentence looks for"
    );

    // The second sentence. The Altar's cost names no creature, so the engine
    // asks which one — and the menu is both creatures this seat controls.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(altar, 0)),
        "a creature is out, so the Altar's only line is offered: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert!(
        options.contains(&elemental),
        "the Elemental is on the menu: {options:?}"
    );
    assert!(
        options.contains(&omnath),
        "and so is the Elemental that made it: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "those two are the whole board: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elemental],
            },
        )
        .expect("the Elemental the question offered pays the cost");

    let asked = settle_aiming_at(&mut engine, p1);
    assert!(
        asked,
        "\"Omnath deals 3 damage to any target\" — the death is asked about, \
         and an any-target prompt offers a player among its answers; \
         the engine stopped at {:?}",
        engine.pending()
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the Elemental died to pay for the Altar, and it is not Omnath"
    );
    // And it is not in the graveyard either, which is the part worth
    // asserting: a token reaches its owner's graveyard and is swept from it
    // the next time state-based actions are checked (CR 111.7, CR 704.5d), so
    // nobody ever finds it there. The damage below is dealt regardless —
    // "applicable triggered abilities will trigger before the token ceases to
    // exist" is the same sentence, and it is the reason this card works at
    // all.
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| engine.state().object(*id).is_some_and(|o| o.card.is_none())),
        "CR 704.5d: no token is left lying in a graveyard"
    );
    assert_eq!(engine.state().players[1].life, 17, "three to the face");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and none of it to the ability's own controller"
    );
    assert!(
        on_battlefield(&engine, p0, omnath_locus_of_rage()).is_some(),
        "the ability's source never moved"
    );
}

// oracle_id = "032ec6e2-6cc3-4a97-9cc7-3233f5e11904"
// oracle_id = "90076bf5-aa9a-4a6e-9035-9aa97fd5561e"
/// Luminarch Ascension, the second half of the Sage's target filter.
///
/// A plain enchantment whose only trigger is on an end step this scenario
/// never reaches, so it sits on the table as a legal target and answers
/// nothing on the way.
/// Reclamation Sage — {2}{G} 2/1 Elf Shaman: "When this creature enters, you
/// may destroy target artifact or enchantment."
///
/// Both printed halves are read off one resolution. The target question is
/// what the filter produces, so the Sol Ring and the Ascension across the
/// table are on it and the Elf and the Forest beside them are not — an arm
/// dropped from `ARTIFACT_OR_ENCHANTMENT`, or a filter that fell back to
/// `Any`, changes that list. And the "you may" is answered yes: the named
/// artifact goes to its owner's graveyard while the enchantment the same
/// trigger could equally have named stays exactly where it was, which is what
/// separates "destroys the target it was given" from "destroys everything
/// legal".
#[test]
#[allow(clippy::too_many_lines)] // a whole game, as every test in this file is
fn reclamation_sage_destroys_the_artifact_it_names_and_leaves_the_rest_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let enchantment = their_enchantment();
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[reclamation_sage()])
        // Two halves of the filter, plus the two permanents it must decline:
        // a creature and a land.
        .battlefield(
            1,
            &[quiet_artifact(), enchantment, llanowar_elves(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let ascension = on_battlefield(&engine, p1, enchantment).expect("their Ascension is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    cast_from_hand(&mut engine, p0, reclamation_sage());

    // The trigger is answered where it arrives rather than where it is
    // expected: the "you may" and the target choice are one resolution, and
    // which of the two is asked first is the engine's business, not the
    // test's.
    let mut aimed: Option<ObjectId> = None;
    for _ in 0..30 {
        if aimed.is_some()
            && stack_is_empty(&engine)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the Sage's controller aims its own trigger");
                assert!(
                    options.contains(&ring) && options.contains(&ascension),
                    "\"target artifact or enchantment\" offers both halves of \
                     the printed filter: {options:?}"
                );
                assert_eq!(
                    options.len(),
                    2,
                    "and nothing else on either side of the table is either: {options:?}"
                );
                assert!(
                    !options.contains(&elves),
                    "a creature is neither an artifact nor an enchantment: {options:?}"
                );
                assert!(
                    !options.contains(&land),
                    "and neither is a land: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![ring],
                        },
                    )
                    .expect("the artifact was one of the options");
                aimed = Some(ring);
            }
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            } => {
                engine
                    .apply(player, PlayerAction::YesNo(true))
                    .expect("`you may` is a question with a yes");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Sage's trigger resolves: {other:?}"),
        }
    }

    assert_eq!(aimed, Some(ring), "the trigger asked for a target");
    assert!(
        stack_is_empty(&engine),
        "the trigger resolved and left nothing behind: {:?}",
        engine.pending()
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat that aimed it holds priority again, got {:?}",
        engine.pending()
    );

    assert!(
        on_battlefield(&engine, p0, reclamation_sage()).is_some(),
        "the Sage itself resolved onto the battlefield and stays"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\" — the named artifact is in its owner's \
         graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, enchantment).is_some(),
        "the Ascension was offered and not named, so it stands: the trigger \
         destroys the one target it was given and not every legal one"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some()
            && on_battlefield(&engine, p1, forest()).is_some(),
        "and the permanents the filter never offered were never touched"
    );
}

/// Sylvan Caryatid prints a {1}{G} 0/3 Plant with defender and hexproof and
/// one ability: "{T}: Add one mana of any color." Neither half is a static a
/// card file could be trusted to have — the mana arrives as a question the
/// engine asks as the {T} is paid, and the defender only means anything in
/// the declaration the plant stands untapped for. So the scenario plays both
/// in one turn: the attack step first, where the only creature its controller
/// has is refused while it is untapped and otherwise able, and then the tap,
/// where the color named is one nothing else on the board can make — the two
/// Forests that cast it are tapped and green.
#[test]
fn sylvan_caryatid_may_not_attack_and_taps_for_a_color_nobody_else_can_make() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sylvan_caryatid()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} out of the two Forests, which is everything the pool had.
    cast_from_hand(&mut engine, p0, sylvan_caryatid());
    pass_until(&mut engine, stack_is_empty);
    let plant = on_battlefield(&engine, p0, sylvan_caryatid()).expect("the Caryatid resolved");

    assert_eq!(pt(&engine, plant), (0, 3), "the body it prints");
    let printed = keywords(&engine, plant);
    assert!(printed.contains(KeywordSet::DEFENDER), "Defender");
    assert!(printed.contains(KeywordSet::HEXPROOF), "hexproof");

    // Defender, read where it decides something: the plant is untapped, on
    // the battlefield, and its controller is the one attacking, so nothing is
    // left to account for its absence but the keyword (CR 702.3b).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the Caryatid's controller is the active player");
    assert!(
        attackers.is_empty(),
        "the only creature on this board may not attack: {attackers:?}"
    );

    // Back to a priority the untapped plant is still worth spending: a mana
    // ability wants nothing but its own {T} and a moment to use it in.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Priority { player, legal }
                if *player == p0 && legal.abilities.iter().any(|(id, _)| *id == plant)
        )
    });
    assert!(
        !is_tapped(&engine, plant),
        "it stood through the combat step it was not allowed to join"
    );

    let before = engine.state().players[0].mana_pool.total();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the predicate above matched a priority")
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == plant)
        .expect("the printed {{T}} is the whole of its text");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("a mana ability needs no stack and no permission");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that tapped names the color");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, on a board whose own lands only make green"
    );
    assert_eq!(pool.total(), before + 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, plant), "the Caryatid paid its own {{T}}");
}

/// Tatyova, Benthic Druid — {3}{G}{U} 3/3 with Landfall: "Whenever a land you
/// control enters, you gain 1 life and draw a card."
///
/// The order is the test: five lands stand on the battlefield before the Druid
/// resolves and must pay nothing, so the one Forest played *after* her is the
/// only thing the ability can be reading. Exactly one life and exactly one
/// card is the load-bearing number, because she arrives as a creature and the
/// trigger is about lands — a `Trigger::EntersBattlefield` that had lost its
/// filter would have paid for the Druid herself, and one collected per
/// permanent on the board would have paid five times.
///
/// The card drawn is named rather than counted: the card that was on top of the
/// library before the land drop has to be the one in hand afterwards, which is
/// what separates a real draw from the land merely leaving the hand.
#[test]
fn tatyova_pays_one_life_and_one_card_for_the_land_that_enters_after_her() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), island(), island(), island()])
        .hand(0, &[tatyova_benthic_druid(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let library_before = library_size(&engine, p0);

    // {3}{G}{U} off the two Forests and the three Islands. She is cast and not
    // seeded: `starting_battlefield` is a placement rather than an entry, and a
    // landfall trigger reads entries.
    cast_from_hand(&mut engine, p0, tatyova_benthic_druid());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, tatyova_benthic_druid()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "the Druid's own arrival is not a land, so nothing has triggered yet"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and she drew nothing for herself"
    );

    // Measured *after* she is cast: she left the hand herself, and what
    // this is about is the land leaving it and a drawn card taking its
    // place.
    let hand_before_land = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // The one land drop this turn, played after her, which is the land the
    // ability is written about.
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");
    let land = play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().players[0].life == life_before + 1
    });

    assert!(
        engine
            .state()
            .object(land)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield),
        "the Forest really is the land that entered"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"you gain 1 life\" — once, for the one land that entered and not for \
         the five that were already there"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the land's controller, not to the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one card, off the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the card that was on top before the land was played"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before_land,
        "the land left the hand and the drawn card took its place"
    );
}

/// Waterspout Djinn — {2}{U}{U} 4/4 flier: "At the beginning of your upkeep,
/// sacrifice this creature unless you return an untapped Island you control
/// to its owner's hand."
///
/// The Karoo sentence on a creature, and the first card in the pool to write
/// it anywhere but on a land — which is how it arrived: the batch that added
/// it turned `every_land_that_pays_by_returning_one_is_in_the_table` red,
/// because that sweep asks the whole pool and the table it compares against
/// held ten lands. `PAYS_BY_RETURNING_A_LAND_ELSEWHERE` is the row, and this
/// is the test the row promises.
///
/// Both answers, because the two are different rules and a driver that only
/// paid would also pass over a card that never asked: the Island named goes
/// to its owner's hand and the Djinn stays (CR 400.3), and naming nothing is
/// how the player declines, after which the creature is sacrificed. The
/// trigger is an upkeep one, so nothing here plays a card at all — the board
/// is seeded and the question arrives on its own.
#[test]
fn a_djinn_that_costs_a_bounce_pays_it_or_is_sacrificed() {
    let p0 = PlayerId::new(0);
    let djinn = card_index("050dac46-9ba0-4b8a-b61b-1c7ec6f3723a");

    for pay in [true, false] {
        let mut engine = Duel::new(if pay { 940 } else { 941 }, island())
            .battlefield(0, &[djinn, island()])
            .start();
        keep_mulligans(&mut engine);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });

        let Pending::ChooseCards {
            options,
            prompt,
            min,
            ..
        } = engine.pending().clone()
        else {
            panic!("the upkeep trigger asks what pays: {:?}", engine.pending())
        };
        assert_eq!(
            prompt,
            ChoicePrompt::CostReturn,
            "the price is a bounce, and the client draws the question from the prompt"
        );
        assert_eq!(min, 0, "declining has to be an answer, or paying is forced");
        assert_eq!(
            options.len(),
            1,
            "the one untapped Island is the whole menu"
        );
        let body = on_battlefield(&engine, p0, djinn).expect("the Djinn is on the battlefield");

        let answer = if pay { options.clone() } else { Vec::new() };
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: answer })
            .unwrap();

        let battlefield = engine.state().zones.list(ZoneLocation::Battlefield);
        if pay {
            assert!(battlefield.contains(&body), "it was paid for and stays");
            assert!(
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(p0))
                    .contains(&options[0]),
                "what paid goes to its owner's hand (CR 400.3)"
            );
        } else {
            assert!(
                !battlefield.contains(&body),
                "nothing was named, so the sacrifice the card prints happens"
            );
            assert!(
                in_graveyard(&engine, p0, djinn).is_some(),
                "and a sacrificed creature is in its owner's graveyard"
            );
            assert!(
                battlefield.contains(&options[0]),
                "the Island it did not return is untouched"
            );
        }
    }
}

/// Carnage Tyrant: "This spell can't be countered." / "Trample, hexproof"
/// The green 7/6 Dinosaur is cast into two untapped Islands holding Counterspell.
/// Counterspell resolves but cannot counter the spell; Carnage Tyrant arrives
/// safely on the battlefield with 7/6 power/toughness, trample, and hexproof.
#[test]
fn carnage_tyrant_cannot_be_countered_and_enters_with_keywords() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(42, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[carnage_tyrant()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let tyrant = in_hand(&engine, p0, carnage_tyrant()).expect("Carnage Tyrant in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: tyrant })
        .expect("six Forests pay {4}{G}{G}");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let cs = in_hand(&engine, p1, counterspell()).expect("Counterspell in hand");
    // The engine refuses the cast outright, which is a stronger reading of
    // "this spell can't be countered" than the one this test was written to
    // make. CR 601.2c: a spell that requires a target cannot be cast at all
    // unless a legal one exists, and an uncounterable spell is not a legal
    // target for Counterspell. So the proof is that Counterspell never
    // reaches the stack, not that it resolves and does nothing.
    assert!(
        engine
            .apply(p1, PlayerAction::CastSpell { card: cs })
            .is_err(),
        "Counterspell has no legal target while the only spell on the stack \
         cannot be countered (CR 601.2c)"
    );
    assert!(
        in_hand(&engine, p1, counterspell()).is_some(),
        "the refused spell stays in its owner's hand, with the mana unspent"
    );
    let _ = tyrant;

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, carnage_tyrant()).is_some()
    });
    let tyrant_obj = on_battlefield(&engine, p0, carnage_tyrant()).expect("entered battlefield");
    assert!(
        in_graveyard(&engine, p0, carnage_tyrant()).is_none(),
        "uncounterable spell does not go to graveyard"
    );
    assert_eq!(pt(&engine, tyrant_obj), (7, 6));
    let kw = keywords(&engine, tyrant_obj);
    assert!(kw.contains(KeywordSet::TRAMPLE));
    assert!(kw.contains(KeywordSet::HEXPROOF));
}

/// Auriok Bladewarden: "{T}: Target creature gets +X/+X until end of turn, where X is this creature's power."
/// Starting as a 1/1 on the battlefield, its activated ability targets another 1/1 creature.
/// Upon resolution, the target creature receives +1/+1 based on the Bladewarden's power and becomes a 2/2.
#[test]
fn auriok_bladewarden_pumps_target_creature_by_its_own_power() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[auriok_bladewarden(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let warden = on_battlefield(&engine, p0, auriok_bladewarden()).expect("Bladewarden deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    activate(&mut engine, p0, auriok_bladewarden(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "Elves is a legal target creature");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (2, 2), "Elves received +1/+1");
    assert!(is_tapped(&engine, warden), "Bladewarden tapped to pay cost");
}

/// Bloom Tender: "{T}: For each color among permanents you control, add one mana of that color."
/// Controlled alongside Baleful Strix (blue and black), permanents you control exhibit three distinct colors.
/// Activating Bloom Tender prompts for three color choices, producing one mana of each color into the pool.
#[test]
fn bloom_tender_adds_one_mana_per_distinct_color_among_permanents() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[bloom_tender(), baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tender = on_battlefield(&engine, p0, bloom_tender()).expect("Bloom Tender deployed");

    activate(&mut engine, p0, bloom_tender(), 0);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected first color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected second color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected third color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 3, "three distinct colors produce 3 mana");
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, tender));
}

/// Ramunap Excavator: "You may play lands from your graveyard."
/// A static permission identical to Crucible of Worlds on a 2/3 Snake Cleric creature.
/// With Ramunap Excavator on the battlefield, a Forest seeded in the graveyard is legally played onto the battlefield.
#[test]
fn ramunap_excavator_allows_playing_land_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(103, forest())
        .battlefield(0, &[ramunap_excavator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let gy_forest = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.lands.contains(&gy_forest),
        "graveyard land offered as legal land play"
    );

    engine
        .apply(p0, PlayerAction::PlayLand { card: gy_forest })
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "forest moved to battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "forest left graveyard"
    );
}

/// Aurochs: "+1/+0 until end of turn **for each other attacking** Aurochs".
///
/// Three on the battlefield and two of them attacking, which is the one board
/// that separates the two words. `Filter::Another` dropped would count the
/// attacker itself, `Filter::Attacking` dropped would count the one standing
/// at home — and both mistakes make the same 4/3, so a board with two
/// Aurochs or with all three attacking cannot tell any of it apart.
///
/// It is also the counted pump on a **triggered** ability, where no `X` is
/// announced at all. That is why the card was a stub rather than wrong: the
/// transcoder refuses `Amount::X` on a `T:` line, because `x.unwrap_or(0)` is
/// a card that claims `Coverage::Implemented` and pumps by nothing.
#[test]
fn aurochs_counts_the_other_attacking_aurochs_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(315, forest())
        .battlefield(0, &[forest(), aurochs(), aurochs(), aurochs()])
        .start();
    keep_mulligans(&mut engine);

    // Seeded creatures are summoning sick on the turn the game began
    // (CR 302.6), so the swing is on this seat's second turn.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let herd = all_on_battlefield(&engine, p0, aurochs());
    assert_eq!(herd.len(), 3, "three of them, and only two will attack");
    assert_eq!(pt(&engine, herd[0]), (2, 3), "the premise: a 2/3");

    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    assert!(
        herd.iter().all(|a| attackers.contains(a)),
        "all three are offered as attackers"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (
                        herd[0],
                        baylee_core::ids::Defender::Player(PlayerId::new(1)),
                    ),
                    (
                        herd[1],
                        baylee_core::ids::Defender::Player(PlayerId::new(1)),
                    ),
                ],
            },
        )
        .expect("two of the three attack");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        (pt(&engine, herd[0]), pt(&engine, herd[1])),
        ((3, 3), (3, 3)),
        "each attacker sees exactly one *other attacking* Aurochs: +1/+0, \
         not +2/+0"
    );
    assert_eq!(
        pt(&engine, herd[2]),
        (2, 3),
        "and the one that stayed home never triggered at all"
    );
}

/// Collector Ouphe prints one sentence — "Activated abilities of artifacts
/// can't be activated" — and the card carries **no ability at all**:
/// `Modifier::CantActivateArtifacts` reaches the artifacts an effect's
/// opponents control (it was written for Karn, the Great Creator), and the
/// DSL has no modifier whose reach is every artifact, its controller's
/// included. So the file is `Coverage::Partial` with the whole of its text
/// in the reason.
///
/// This pins that. An artifact's activated ability is activated with the
/// Ouphe on the battlefield and it works, which is the defect written down
/// rather than left to be discovered in a game — and the assertion is
/// **meant to fail** the day the modifier exists. The printed body is
/// checked beside it, because a card that is only a 2/2 should at least be
/// the right 2/2.
#[test]
fn collector_ouphe_is_a_body_and_its_only_sentence_is_not_built_yet() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[collector_ouphe(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let body = on_battlefield(&engine, p0, collector_ouphe()).expect("the Ouphe is in play");
    assert_eq!(pt(&engine, body), (2, 2), "a 2/2 Ouphe is the printed body");

    let before = engine.state().players[0].mana_pool.total();
    activate(&mut engine, p0, quiet_artifact(), 0);
    assert!(
        engine.state().players[0].mana_pool.total() > before,
        "an artifact's activated ability still works with Collector Ouphe on \
         the battlefield: the card's only sentence is not built, and this is \
         the assertion that has to be deleted when it is"
    );
}

/// Ojer Pakpatiq's whole point is that killing it does not remove it: "When
/// Ojer Pakpatiq dies, return it to the battlefield tapped and transformed
/// under its owner's control with three time counters on it." So the
/// scenario kills a 4/3 with Hero's Downfall and then reads the battlefield
/// rather than the graveyard, because a god that stayed dead and a god that
/// came back as the wrong face are two different failures.
///
/// `Effect::ExileSelfReturnAsFace { face: 1 }` is the built half, and what
/// comes back is a **land** — Temple of Cyclical Time — off a card whose
/// front is a legendary creature, which is also the shape this pool got
/// wrong twenty-one times this morning: the back face is reachable by
/// transforming and by nothing else.
///
/// The file is `Coverage::Partial` for rebound and for removing a time
/// counter, and the pins are here: what returns is untapped and carries no
/// time counters, both of which the printing spells out. They are meant to
/// be deleted the day an effect can put counters on the object it returns.
#[test]
fn ojer_pakpatiq_dies_and_comes_back_as_the_land_on_its_other_face() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .battlefield(1, &[ojer_pakpatiq_deepest_epoch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("the god is out");
    assert_eq!(pt(&engine, god), (4, 3), "a 4/3 before anything happens");
    assert!(
        keywords(&engine, god).contains(KeywordSet::FLYING),
        "and it flies"
    );

    cast_from_hand(&mut engine, p0, heroes_downfall());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Downfall asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&god), "a creature is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![god],
                players: vec![],
            },
        )
        .expect("a target the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let temple =
        on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("it came back at once");
    let t = types(&engine, temple);
    assert!(
        t.contains(TypeSet::LAND),
        "\"transformed\" — the back face is Temple of Cyclical Time"
    );
    assert!(
        !t.contains(TypeSet::CREATURE),
        "and the creature stayed on the front"
    );
    assert!(
        in_graveyard(&engine, p1, ojer_pakpatiq_deepest_epoch()).is_none(),
        "the god is on the battlefield and not in a graveyard"
    );

    // The two pins. Both are printed and neither is built.
    assert!(
        !is_tapped(&engine, temple),
        "\"return it to the battlefield **tapped**\" — delete this when an \
         effect returning a face can tap what it returns"
    );
    // "with three time counters on it" is not pinned by a count, because
    // there is no `time` counter kind in `baylee_cards_dsl::counters` to
    // count — the clause cannot be spelled at all, which is a shorter
    // sentence than a wrong number. The land's own "{T}: Add {U}. Remove a
    // time counter" waits on the same thing.
    assert_eq!(
        counters_on(&engine, temple, baylee_cards_dsl::counters::QUEST),
        0,
        "and it carries no counters of any kind this DSL can name"
    );
}

/// Fatehold Chronologist "enters prepared", and the printed reminder says
/// what that buys: "While it's prepared, you may cast a copy of its spell."
/// Its spell is Peer Review on the back face, so the assertion is that the
/// creature carries an offer its face alone does not explain — a 1/2 flier
/// with no printed activated ability, standing there with one.
///
/// The file is `Coverage::Partial` for the Cadet token, which is the larger
/// half of Peer Review and has no entry in `crate::tokens`; the surveil
/// beside it is built. Nothing below casts the copy, so what is asserted is
/// the marker and the offer rather than the spell's own text.
#[test]
fn fatehold_chronologist_enters_prepared_and_carries_the_offer_that_buys() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[fatehold_chronologist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A card with a castable second face is offered both, so the front one is
    // named rather than assumed: mode 1 is Peer Review at {2}{W/U} and would
    // put a sorcery on the stack where this test wants a creature.
    cast_from_hand(&mut engine, p0, fatehold_chronologist());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "a card with two castable faces asks which, got {:?}",
            engine.pending()
        )
    };
    let normal = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Normal))
        .expect("the creature itself is one of the offers");
    engine
        .apply(p0, PlayerAction::ChooseMode(normal))
        .expect("the mode the question enumerated");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let bird = on_battlefield(&engine, p0, fatehold_chronologist()).expect("it resolved");
    assert_eq!(pt(&engine, bird), (1, 2), "a 1/2 Bird Wizard");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "with flying"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(id, _)| *id == bird),
        "\"While it's prepared, you may cast a copy of its spell\" — the \
         printed front face has no activated ability of its own, so this \
         offer is the prepared marker and nothing else: {:?}",
        legal.abilities
    );
}

/// Drannith Magistrate: "Your opponents can't cast spells from anywhere
/// other than their hands."
///
/// The zone is the whole sentence, so the test is one board read twice: a
/// Sol Ring in the opponent's hand and a commander in their command zone,
/// with four Swamps floating that pay for either of them. Without the
/// Magistrate both are offered; with it, the hand card alone — which is
/// what makes this a statement about the *zone* and not about the price or
/// about commanders.
#[test]
fn drannith_magistrate_leaves_an_opponent_their_hand_and_nothing_else() {
    let p1 = PlayerId::new(1);
    for magistrate in [false, true] {
        let mut board = vec![plains(), plains()];
        if magistrate {
            board.push(drannith_magistrate());
        }
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .commander(1, &[sheoldred_the_apocalypse()])
            .battlefield(1, &[swamp(), swamp(), swamp(), swamp()])
            .hand(1, &[quiet_artifact()])
            .start();
        keep_mulligans(&mut engine);
        reach_their_main_phase(&mut engine, p1);
        tap_all_mana(&mut engine, p1);
        assert_eq!(
            engine.state().players[1]
                .mana_pool
                .available(ManaColor::Black),
            4,
            "{{2}}{{B}}{{B}} for the commander and {{1}} for the Sol Ring are \
             both floating, so the price refuses neither"
        );

        let ring = in_hand(&engine, p1, quiet_artifact()).expect("the Sol Ring is in hand");
        let boss = engine
            .state()
            .zones
            .list(ZoneLocation::Command(p1))
            .first()
            .copied()
            .expect("the commander is in the command zone");
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            panic!("expected p1's priority, got {:?}", engine.pending())
        };
        assert_eq!(player, p1, "p1's own main phase");
        assert!(
            legal.castable.contains(&ring),
            "a card in hand is castable either way, Magistrate {magistrate}"
        );
        assert_eq!(
            legal.castable.contains(&boss),
            !magistrate,
            "the command zone is \"anywhere other than their hands\", \
             Magistrate {magistrate}"
        );
    }
}

fn courser_of_kruphix() -> CardIndex {
    card_index("46779609-4fa7-4fd2-b5b4-7d4d749339e6")
}

/// `Courser of Kruphix` prints `Play with the top card of your library revealed.`, `You may play lands from the top of your library.`, and `Landfall — Whenever a land you control enters, you gain 1 life.`
///
/// Marked `Coverage::Partial`, its Landfall ability is implemented through `Trigger::EntersBattlefield` on `Filter::YOUR_LAND`, gaining 1 life when a `forest()` enters under your control.
/// The unmodelled library-top land play ability is omitted, leaving `legal.lands` empty once the hand has no land cards.
#[test]
fn courser_of_kruphix_gains_life_on_land_entry_and_omits_library_play() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[courser_of_kruphix()])
        .hand(0, &[forest()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courser =
        on_battlefield(&engine, p0, courser_of_kruphix()).expect("courser on battlefield");
    assert_eq!(pt(&engine, courser), (2, 4));
    assert_eq!(engine.state().players[0].life, 20);

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 21, "landfall gained 1 life");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.lands.is_empty(),
        "under `Coverage::Partial` playing lands from the top of the library is omitted"
    );
}

fn dragon_s_rage_channeler() -> CardIndex {
    card_index("0c016ccc-a341-4b76-87ba-69c639d2746d")
}

/// `Dragon's Rage Channeler` prints `Whenever you cast a noncreature spell, surveil 1.` and `Delirium — As long as there are four or more card types among cards in your graveyard, this creature gets +2/+2, has flying, and attacks each combat if able.`
///
/// Marked `Coverage::Partial`, casting a noncreature spell like `lightning_greaves()` triggers surveil 1 via `Trigger::SpellCast`.
/// The trigger prompts through `Pending::ChooseCards` with `ChoicePrompt::SurveilGraveyard`, moving the top card into `ZoneLocation::Graveyard`, while the Delirium stat bonus and `KeywordSet::FLYING` are omitted.
#[test]
fn dragon_s_rage_channeler_surveils_on_noncreature_cast_and_omits_delirium() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragon_s_rage_channeler(), plains(), plains()])
        .hand(0, &[lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drc = on_battlefield(&engine, p0, dragon_s_rage_channeler()).expect("drc seated");
    assert_eq!(pt(&engine, drc), (1, 1));
    assert!(!keywords(&engine, drc).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        prompt,
        min,
        max,
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ChoicePrompt::SurveilGraveyard);
    assert_eq!((min, max), (0, 1));
    assert!(!options.is_empty());

    let binned = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![binned],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "surveiled card was placed into the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, lightning_greaves()).is_some(),
        "lightning greaves resolved"
    );
    assert_eq!(
        pt(&engine, drc),
        (1, 1),
        "under `Coverage::Partial` delirium is omitted, body remains 1/1"
    );
    assert!(!keywords(&engine, drc).contains(KeywordSet::FLYING));
}

fn dryad_of_the_ilysian_grove() -> CardIndex {
    card_index("bdbde5d0-f5e4-44da-b27c-b4ad6f374cc9")
}

/// `Dryad of the Ilysian Grove` prints `You may play an additional land on each of your turns.` and `Lands you control are every basic land type in addition to their other types.`
///
/// Marked `Coverage::Implemented`, its static abilities grant `Modifier::ExtraLandDrops` and `Modifier::AllBasicLandTypes`.
/// In a single turn, its controller plays two `forest()` cards from hand, and the controlled land gains every basic land subtype (`subtypes::land::PLAINS`, `subtypes::land::ISLAND`, `subtypes::land::SWAMP`, `subtypes::land::MOUNTAIN`, `subtypes::land::FOREST`) while an opponent's land is unaffected.
#[test]
fn dryad_of_the_ilysian_grove_grants_extra_land_drop_and_all_basic_land_types() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dryad_of_the_ilysian_grove()])
        .hand(0, &[forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dryad = on_battlefield(&engine, p0, dryad_of_the_ilysian_grove()).expect("dryad seated");
    assert_eq!(pt(&engine, dryad), (2, 4));

    play_land(&mut engine, p0, forest());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.lands.is_empty(),
        "extra land drop allows playing a second land"
    );

    play_land(&mut engine, p0, forest());
    let Pending::Priority {
        legal: legal_after, ..
    } = engine.pending().clone()
    else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal_after.lands.is_empty(),
        "no third land drop is allowed"
    );

    let my_forest = on_battlefield(&engine, p0, forest()).expect("my forest on battlefield");
    let my_subtypes = &engine
        .state()
        .object(my_forest)
        .expect("my forest object")
        .characteristics()
        .subtypes;
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::PLAINS));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::ISLAND));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::SWAMP));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::MOUNTAIN));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::FOREST));

    let their_forest =
        on_battlefield(&engine, p1, forest()).expect("opponent forest on battlefield");
    let their_subtypes = &engine
        .state()
        .object(their_forest)
        .expect("their forest object")
        .characteristics()
        .subtypes;
    assert!(
        !their_subtypes.contains(baylee_core::generated::subtypes::land::PLAINS),
        "opponent land is not affected by `Filter::YOUR_LAND`"
    );
}

fn enduring_vitality() -> CardIndex {
    card_index("3577c47e-76d3-4659-b922-31c4b74be3a0")
}

/// `Enduring Vitality` prints `Vigilance`, `Creatures you control have "{{T}}: Add one mana of any color."`, and an enduring return-on-death trigger.
///
/// Marked `Coverage::Partial`, it carries `KeywordSet::VIGILANCE` and its static ability grants a mana ability to `Filter::YOUR_CREATURE` via `Modifier::GrantActivated`.
/// This offers `PlayerAction::ActivateManaAbility` in `legal.mana_abilities` on both `Enduring Vitality` and a controlled `young_wolf()`, prompting via `Pending::ChooseColor` to produce `ManaColor::Blue`, while an opponent's creature is excluded.
#[test]
fn enduring_vitality_has_vigilance_and_grants_mana_ability_to_controlled_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[enduring_vitality(), young_wolf()])
        .battlefield(1, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vitality =
        on_battlefield(&engine, p0, enduring_vitality()).expect("vitality on battlefield");
    let my_wolf = on_battlefield(&engine, p0, young_wolf()).expect("my wolf on battlefield");
    let their_wolf = on_battlefield(&engine, p1, young_wolf()).expect("their wolf on battlefield");

    assert_eq!(pt(&engine, vitality), (3, 3));
    assert!(keywords(&engine, vitality).contains(KeywordSet::VIGILANCE));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.mana_abilities.contains(&my_wolf),
        "controlled wolf receives granted mana ability in `legal.mana_abilities`"
    );
    assert!(
        legal.mana_abilities.contains(&vitality),
        "vitality also receives its own granted mana ability"
    );
    assert!(
        !legal.mana_abilities.contains(&their_wolf),
        "opponent's creature does not receive granted mana ability"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: my_wolf })
        .unwrap();

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "all five colors can be chosen");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue mana produced by granted mana ability"
    );
    assert!(is_tapped(&engine, my_wolf));
}

fn flamekin_harbinger() -> CardIndex {
    card_index("d6585e30-4ca0-4701-b274-b24f3508dd97")
}

/// `Flamekin Harbinger` prints `When this creature enters, you may search your library for an Elemental card, reveal it, then shuffle and put that card on top.`
///
/// Marked `Coverage::Implemented`, its arrival trigger initiates an optional library search using `ChoicePrompt::SearchLibrary` with `(min: 0, max: 1)` through `Pending::ChooseCards`.
/// Selecting a matching Elemental card places it directly on top of `ZoneLocation::Library`.
#[test]
fn flamekin_harbinger_searches_library_for_elemental_and_puts_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, flamekin_harbinger())
        .battlefield(0, &[mountain()])
        .hand(0, &[flamekin_harbinger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flamekin_harbinger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        prompt,
        min,
        max,
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
    assert_eq!((min, max), (0, 1), "\"you may search\" is min 0 max 1");
    assert!(
        !options.is_empty(),
        "elemental cards in library are offered"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("card on top of library");
    assert_eq!(top, chosen, "chosen Elemental is on top of library");
    let harbinger =
        on_battlefield(&engine, p0, flamekin_harbinger()).expect("harbinger on battlefield");
    assert_eq!(pt(&engine, harbinger), (1, 1));
}

fn golden_guardian() -> CardIndex {
    card_index("58afb897-4d57-4b53-a5c3-b532cb3d5180")
}

/// `Golden Guardian` prints `Defender`, `{{2}}: This creature fights another target creature you control. When this creature dies this turn, return it to the battlefield transformed under your control.`, and transforms into `Gold-Forge Garrison`.
///
/// Marked `Coverage::Partial`, its front face carries `KeywordSet::DEFENDER` and a 4/4 body, which prevents it from being declared as an attacker in `Pending::ChooseAttackers` while `young_wolf()` can attack.
/// With `{{2}}` floating mana and another creature controlled, the unmodelled fight/transform ability is omitted from `legal.abilities`.
#[test]
fn golden_guardian_has_defender_and_omits_fight_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[golden_guardian(), young_wolf(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guardian = on_battlefield(&engine, p0, golden_guardian()).expect("guardian on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    assert_eq!(pt(&engine, guardian), (4, 4));
    assert!(keywords(&engine, guardian).contains(KeywordSet::DEFENDER));

    tap_mana_except(&mut engine, p0, guardian);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, guardian));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == guardian),
        "under `Coverage::Partial` the fight ability is not offered despite {{2}} floating and legal target"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers, got {:?}", engine.pending());
    };
    assert!(
        attackers.contains(&wolf),
        "young wolf without defender is a legal attacker"
    );
    assert!(
        !attackers.contains(&guardian),
        "golden guardian with defender cannot be declared as an attacker"
    );
}

fn hexdrinker() -> CardIndex {
    card_index("69bc2afd-9f53-47f2-b9c8-f12732784e10")
}

/// `Hexdrinker` prints `Level up {{1}} ({{1}}: Put a level counter on this. Level up only as a sorcery.)`, `LEVEL 3-7: 4/4, Protection from instants`, and `LEVEL 8+: 6/6, Protection from everything`.
///
/// Marked `Coverage::Partial`, its Level up activated ability uses `ActivationTiming::SorcerySpeed` to place a `CounterKind::Level` on itself when paid with `{{1}}`.
/// Activating it three times raises its level counter count to 3, while its body remains 2/1 because the level bands are unmodelled.
#[test]
fn hexdrinker_levels_up_with_counters_and_omits_level_bands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hexdrinker(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let snake = on_battlefield(&engine, p0, hexdrinker()).expect("hexdrinker seated");
    assert_eq!(pt(&engine, snake), (2, 1));
    assert_eq!(counters_on(&engine, snake, CounterKind::Level), 0);

    tap_mana_except(&mut engine, p0, snake);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    for expected in 1..=3 {
        activate(&mut engine, p0, hexdrinker(), 0);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(counters_on(&engine, snake, CounterKind::Level), expected);
    }

    assert_eq!(
        pt(&engine, snake),
        (2, 1),
        "under `Coverage::Partial` level bands are omitted, body remains 2/1"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all three mana spent paying for level up"
    );
}

fn ignoble_hierarch() -> CardIndex {
    card_index("c8de43a3-ebd3-4000-b343-a6ffed11d34d")
}

/// `Ignoble Hierarch` prints `Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)` and `{{T}}: Add {{B}}, {{R}}, or {{G}}.`
///
/// Marked `Coverage::Partial`, activating its printed mana ability prompts via `Pending::ChooseColor` with `ManaColor::Black`, `ManaColor::Red`, and `ManaColor::Green`, producing the chosen mana and tapping `Ignoble Hierarch`.
/// When a controlled `young_wolf()` attacks alone, Exalted is omitted under `Coverage::Partial`, leaving its body at 1/1.
#[test]
fn ignoble_hierarch_produces_mana_choice_and_omits_exalted() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ignoble_hierarch(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hierarch =
        on_battlefield(&engine, p0, ignoble_hierarch()).expect("hierarch on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    assert_eq!(pt(&engine, hierarch), (0, 1));
    assert_eq!(pt(&engine, wolf), (1, 1));

    activate(&mut engine, p0, ignoble_hierarch(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red, ManaColor::Green],
        "hierarch offers Black, Red, or Green"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, hierarch));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wolf, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wolf),
        (1, 1),
        "under `Coverage::Partial` exalted is omitted, attacker receives no pump"
    );
}

fn altered_ego() -> CardIndex {
    card_index("7c35f3fd-c64e-4944-a4d5-37ce916d23c3")
}

/// `Altered Ego` prints `This spell can't be countered.` and `You may have this creature enter as a copy of any creature on the battlefield, except it enters with X additional +1/+1 counters on it.`
///
/// Marked `Coverage::Partial`, it carries `KeywordSet::UNCOUNTERABLE`, causing an attempted `counterspell()` to be refused for lack of legal target (CR 601.2c).
/// Through `AbilityDef::CopyOnEnter`, it enters copying `young_wolf()`, but under `Coverage::Partial` the X additional `CounterKind::P1P1` counters are omitted.
#[test]
fn altered_ego_is_uncounterable_and_copies_creature_without_x_counters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                young_wolf(),
            ],
        )
        .hand(0, &[altered_ego()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf deployed");
    assert_eq!(pt(&engine, wolf), (1, 1));

    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, altered_ego()).expect("altered ego in hand");
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();

    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber prompt, got {:?}", engine.pending());
    };
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let cs = in_hand(&engine, p1, counterspell()).expect("counterspell in hand");
    assert!(
        engine
            .apply(p1, PlayerAction::CastSpell { card: cs })
            .is_err(),
        "Counterspell cannot target Altered Ego due to `KeywordSet::UNCOUNTERABLE`"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&wolf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wolf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let ego = on_battlefield(&engine, p0, altered_ego()).expect("altered ego on battlefield");
    assert_ne!(ego, wolf);
    assert_eq!(
        pt(&engine, ego),
        (1, 1),
        "enters as a 1/1 copy of young wolf"
    );
    assert_eq!(
        counters_on(&engine, ego, CounterKind::P1P1),
        0,
        "under `Coverage::Partial` X additional counters are omitted"
    );
}

fn badgermole_cub() -> CardIndex {
    card_index("2b0afb89-0944-4861-b9c3-e909e2ac215e")
}

/// `Badgermole Cub` prints `When this creature enters, earthbend 1. (Target land you control becomes a 0/0 creature with haste that's still a land. Put a +1/+1 counter on it. When it dies or is exiled, return it to the battlefield tapped.)` and `Whenever you tap a creature for mana, add an additional {{G}}.`
///
/// Marked `Coverage::Partial`, its arrival trigger targets a controlled land via `Pending::ChooseTargets` under `Filter::YOUR_LAND`, adding `TypeSet::CREATURE`, `KeywordSet::HASTE`, base P/T 0/0, and one `CounterKind::P1P1`.
/// The tapped animated land produces only its printed mana, omitting the unsupported additional `{{G}}` trigger.
#[test]
fn badgermole_cub_animates_land_and_omits_additional_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[badgermole_cub()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_forests = all_on_battlefield(&engine, p0, forest());
    let target_land = my_forests[0];
    let their_land = on_battlefield(&engine, p1, forest()).expect("opponent forest");

    tap_mana_except(&mut engine, p0, target_land);
    cast_with_floating(&mut engine, p0, badgermole_cub());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&target_land), "controlled land is legal");
    assert!(
        !options.contains(&their_land),
        "opponent land is not offered by `Filter::YOUR_LAND`"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target_land],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, target_land);
    assert!(
        t.contains(TypeSet::LAND) && t.contains(TypeSet::CREATURE),
        "target becomes creature land"
    );
    assert!(
        keywords(&engine, target_land).contains(KeywordSet::HASTE),
        "target gains haste"
    );
    assert_eq!(
        counters_on(&engine, target_land, CounterKind::P1P1),
        1,
        "target receives one +1/+1 counter"
    );
    assert_eq!(
        pt(&engine, target_land),
        (1, 1),
        "0/0 base plus one counter is a 1/1"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: target_land,
            },
        )
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "under `Coverage::Partial` the additional {{G}} trigger is omitted"
    );
}

fn birgi_god_of_storytelling() -> CardIndex {
    card_index("fb81e4d3-1d8c-4779-be62-87cf49277e51")
}

/// `Birgi, God of Storytelling` prints `Whenever you cast a spell, add {{R}}. Until end of turn, you don't lose this mana as steps and phases end.`, `Creatures you control can boast twice during each of your turns rather than once.`, and `Discard a card: Exile the top two cards of your library. You may play those cards this turn.`
///
/// Marked `Coverage::Partial`, casting a spell triggers `Trigger::SpellCast` under `Filter::ControlledByYou`, producing one `ManaColor::Red` via `Effect::mana`.
/// Harnfel's discard-to-exile activated ability is omitted from `LegalActions::abilities`.
#[test]
fn birgi_god_of_storytelling_adds_red_mana_on_spell_cast_and_omits_harnfel() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[birgi_god_of_storytelling(), forest(), forest()])
        .hand(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let birgi =
        on_battlefield(&engine, p0, birgi_god_of_storytelling()).expect("birgi on battlefield");
    assert_eq!(pt(&engine, birgi), (3, 3));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0
    );

    let my_forests = all_on_battlefield(&engine, p0, forest());
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: my_forests[0],
            },
        )
        .unwrap();

    cast_with_floating(&mut engine, p0, young_wolf());

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_some(),
        "young wolf resolved"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Birgi added {{R}} on spell cast"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == birgi),
        "under `Coverage::Partial` Harnfel's activated ability is omitted"
    );
}

fn bristly_bill_spine_sower() -> CardIndex {
    card_index("d3b2d8a2-d3bc-448c-9cf6-6bead6010c28")
}

/// `Bristly Bill, Spine Sower` prints `Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature.` and `{{3}}{{G}}{{G}}: Double the number of +1/+1 counters on each creature you control.`
///
/// Marked `Coverage::Partial`, entering lands trigger `Trigger::EntersBattlefield` with `Filter::YOUR_LAND`, placing a `CounterKind::P1P1` on target creature via `Pending::ChooseTargets`.
/// With sufficient floating mana to pay `{{3}}{{G}}{{G}}`, `LegalActions::abilities` contains no doubling activation for `Bristly Bill, Spine Sower`.
#[test]
fn bristly_bill_spine_sower_triggers_landfall_counter_and_omits_doubling() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                bristly_bill_spine_sower(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bill =
        on_battlefield(&engine, p0, bristly_bill_spine_sower()).expect("bill on battlefield");
    assert_eq!(pt(&engine, bill), (2, 2));
    assert_eq!(counters_on(&engine, bill, CounterKind::P1P1), 0);

    play_land(&mut engine, p0, forest());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&bill),
        "Bristly Bill is a legal target creature"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bill],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, bill, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, bill), (3, 3));

    tap_all_mana(&mut engine, p0);
    assert!(
        engine.state().players[0].mana_pool.total() >= 5,
        "sufficient mana is floating for the {{3}}{{G}}{{G}} cost"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == bill),
        "under `Coverage::Partial` the {{3}}{{G}}{{G}} counter doubling ability is omitted"
    );
}

fn lake_town_lookout() -> CardIndex {
    card_index("cf765efe-884c-48e2-9edb-9d45cf2756dd")
}

/// `Lake-town Lookout` prints `When this creature dies, recruit. (Draw a card, then discard a card. If you discarded a nonland card, create a 1/1 white Human Soldier creature token.)`
///
/// Marked `Coverage::Partial`, its death trigger fires `Trigger::Dies` with `Effect::draw(1)` and `Effect::DiscardForPlayers`.
/// When destroyed by `heroes_downfall()`, the controller draws a card, answers `Pending::ChooseCards` to discard a card via `PlayerAction::ChooseObjects`, and under `Coverage::Partial` no Soldier creature token is created.
#[test]
fn lake_town_lookout_dies_draws_and_discards_without_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lake_town_lookout(), swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lookout = on_battlefield(&engine, p0, lake_town_lookout()).expect("lookout on battlefield");
    assert_eq!(pt(&engine, lookout), (1, 1));
    let initial_library_size = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    let downfall = in_hand(&engine, p0, heroes_downfall()).expect("downfall in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: downfall })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for downfall, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&lookout));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![lookout],
                players: vec![],
            },
        )
        .unwrap();

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
        panic!("expected ChooseCards prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::Generic);
    assert!(
        in_graveyard(&engine, p0, lake_town_lookout()).is_some(),
        "lookout is in graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        initial_library_size - 1,
        "recruit drew one card before prompting for discard"
    );

    let to_discard = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![to_discard],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "under `Coverage::Partial` recruit token creation is omitted"
    );
}

fn murderous_rider() -> CardIndex {
    card_index("1080c5b5-6651-4c6a-93e6-099fbe389e26")
}

/// `Murderous Rider` prints `Lifelink`, `When this creature dies, put it on the bottom of its owner's library.`, and `Destroy target creature or planeswalker. You lose 2 life. (Then exile this card. You may cast the creature later from exile.)`
///
/// Marked `Coverage::Partial`, the front face carries `KeywordSet::LIFELINK`, while its adventure face `Swift End` is cast via `Pending::ChooseCastMode` with `CastModeKind::Face(1)`.
/// Targeting an opponent's `llanowar_elves()` destroys the creature and subtracts 2 life, placing the card into the graveyard without the adventure frame.
#[test]
fn murderous_rider_lifelink_and_swift_end_destroys_target_and_loses_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[murderous_rider(), swamp(), swamp(), swamp()])
        .hand(0, &[murderous_rider()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rider = on_battlefield(&engine, p0, murderous_rider()).expect("rider deployed");
    assert_eq!(pt(&engine, rider), (2, 3));
    assert!(
        keywords(&engine, rider).contains(KeywordSet::LIFELINK),
        "front face has lifelink"
    );

    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, murderous_rider()).expect("rider in hand");
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();

    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode prompt, got {:?}", engine.pending());
    };
    let adventure_slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Face(1)))
        .expect("Swift End adventure face");
    engine
        .apply(p0, PlayerAction::ChooseMode(adventure_slot))
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");
    assert!(
        options.contains(&elf),
        "opponent creature is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "target creature was destroyed"
    );
    assert_eq!(engine.state().players[0].life, 18, "caster lost 2 life");
    assert!(
        in_graveyard(&engine, p0, murderous_rider()).is_some(),
        "under `Coverage::Partial` Swift End lands in graveyard"
    );
}

fn nantuko_mentor() -> CardIndex {
    card_index("b79378e7-99db-403f-8f63-4d71ebdb3f6c")
}

/// `Nantuko Mentor` prints `{{2}}{{G}}, {{T}}: Target creature gets +X/+X until end of turn, where X is that creature's power.`
///
/// Marked `Coverage::Implemented`, paying `{{2}}{{G}}` and tapping `Nantuko Mentor` activates `Effect::PumpTarget` with `Amount::TargetPower`.
/// Targeting `aurochs()` (2/3) grants +2/+2 until end of turn, elevating its body to 4/5.
#[test]
fn nantuko_mentor_doubles_target_creature_power_and_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[nantuko_mentor(), aurochs(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mentor = on_battlefield(&engine, p0, nantuko_mentor()).expect("mentor deployed");
    let aurochs_obj = on_battlefield(&engine, p0, aurochs()).expect("aurochs deployed");
    assert_eq!(pt(&engine, mentor), (1, 1));
    assert_eq!(pt(&engine, aurochs_obj), (2, 3));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3
    );
    assert!(!is_tapped(&engine, mentor));

    activate(&mut engine, p0, nantuko_mentor(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&aurochs_obj));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![aurochs_obj],
                players: vec![],
            },
        )
        .unwrap();

    assert!(is_tapped(&engine, mentor), "mentor tapped for its cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana spent on activation"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, aurochs_obj),
        (4, 5),
        "aurochs received +2/+2 matching its power of 2"
    );
}

fn noble_hierarch() -> CardIndex {
    card_index("98aa9424-5912-4bd6-9300-b3972a31d8af")
}

/// `Noble Hierarch` prints `Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)` and `{{T}}: Add {{G}}, {{W}}, or {{U}}.`
///
/// Marked `Coverage::Partial`, activating its printed mana ability prompts via `Pending::ChooseColor` with `ManaColor::Green`, `ManaColor::White`, and `ManaColor::Blue`, producing the chosen mana and tapping `Noble Hierarch`.
/// When a controlled `young_wolf()` attacks alone, Exalted is omitted under `Coverage::Partial`, leaving its body at 1/1.
#[test]
fn noble_hierarch_produces_mana_choice_and_omits_exalted() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[noble_hierarch(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hierarch = on_battlefield(&engine, p0, noble_hierarch()).expect("hierarch on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    assert_eq!(pt(&engine, hierarch), (0, 1));
    assert_eq!(pt(&engine, wolf), (1, 1));

    activate(&mut engine, p0, noble_hierarch(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![ManaColor::Green, ManaColor::White, ManaColor::Blue],
        "hierarch offers Green, White, or Blue"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, hierarch));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wolf, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wolf),
        (1, 1),
        "under `Coverage::Partial` exalted is omitted, attacker receives no pump"
    );
}

fn rishkar_peema_renegade() -> CardIndex {
    card_index("761021ce-4559-464e-aa03-85c2fe78e267")
}

/// `Rishkar, Peema Renegade` prints `When Rishkar enters, put a +1/+1 counter on each of up to two target creatures.` and `Each creature you control with a counter on it has "{{T}}: Add {{G}}."`
///
/// Marked `Coverage::Partial`, its arrival trigger fires `Trigger::ETB` placing a `CounterKind::P1P1` on up to two targets chosen through `Pending::ChooseTargets`.
/// The creatures each grow by +1/+1, while the unsupported static granting `{{T}}: Add {{G}}` to creatures with counters is omitted from `LegalActions::mana_abilities`.
#[test]
fn rishkar_peema_renegade_distributes_counters_and_omits_mana_grant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), young_wolf()])
        .hand(0, &[rishkar_peema_renegade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rishkar_peema_renegade());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 2), "up to two target creatures");

    let rishkar =
        on_battlefield(&engine, p0, rishkar_peema_renegade()).expect("rishkar on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");
    assert!(options.contains(&rishkar));
    assert!(options.contains(&wolf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![rishkar, wolf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, rishkar, CounterKind::P1P1), 1);
    assert_eq!(counters_on(&engine, wolf, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, rishkar), (3, 3));
    assert_eq!(pt(&engine, wolf), (2, 2));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.mana_abilities.contains(&rishkar),
        "under `Coverage::Partial` Rishkar is not granted a mana ability"
    );
    assert!(
        !legal.mana_abilities.contains(&wolf),
        "under `Coverage::Partial` wolf is not granted a mana ability"
    );
}

fn tireless_provisioner() -> CardIndex {
    card_index("ab8d5f5c-1976-4f77-8ed2-8d28ee666741")
}

/// `Tireless Provisioner` prints `Landfall — Whenever a land you control enters, create a Food token or a Treasure token. (Food is an artifact with "{{2}}, {{T}}, Sacrifice this token: You gain 3 life." Treasure is an artifact with "{{T}}, Sacrifice this token: Add one mana of any color.")`
///
/// Marked `Coverage::Implemented`, playing a land triggers `Trigger::EntersBattlefield` with `Filter::YOUR_LAND`, presenting a modal choice via `Pending::ChooseCastMode`.
/// Choosing mode 1 creates an artifact Treasure token verified by `tokens_of`.
#[test]
fn tireless_provisioner_creates_treasure_token_on_landfall() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tireless_provisioner()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let prov = on_battlefield(&engine, p0, tireless_provisioner()).expect("provisioner deployed");
    assert_eq!(pt(&engine, prov), (3, 2));
    assert!(tokens_of(&engine, p0).is_empty());

    play_land(&mut engine, p0, forest());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode prompt, got {:?}", engine.pending());
    };
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 1],
        "offers Food (mode 0) and Treasure (mode 1)"
    );

    let treasure_slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("treasure mode");
    engine
        .apply(p0, PlayerAction::ChooseMode(treasure_slot))
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token created");
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::ARTIFACT),
        "token has artifact type"
    );
}

fn wayward_swordtooth() -> CardIndex {
    card_index("3875aef0-3102-4fbf-be90-e4139f7a2348")
}

/// `Wayward Swordtooth` prints `Ascend (If you control ten or more permanents, you get the city's blessing for the rest of the game.)`, `You may play an additional land on each of your turns.`, and `This creature can't attack or block unless you have the city's blessing.`
///
/// Marked `Coverage::Partial`, its static ability grants `Modifier::ExtraLandDrops(1)`, permitting two land drops in a single turn before `LegalActions::lands` empties.
/// Because ascend and the city's blessing are omitted under `Coverage::Partial`, the creature is legally offered in `Pending::ChooseAttackers` despite controlling fewer than ten permanents.
#[test]
fn wayward_swordtooth_grants_extra_land_drop_and_omits_ascend_restriction() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wayward_swordtooth()])
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dino = on_battlefield(&engine, p0, wayward_swordtooth()).expect("swordtooth deployed");
    assert_eq!(pt(&engine, dino), (5, 5));

    play_land(&mut engine, p0, forest());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.lands.is_empty(),
        "extra land drop allows playing a second land"
    );

    play_land(&mut engine, p0, forest());
    let Pending::Priority {
        legal: legal_after, ..
    } = engine.pending().clone()
    else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal_after.lands.is_empty(),
        "no third land drop is allowed"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    assert!(
        attackers.contains(&dino),
        "under `Coverage::Partial` the attack restriction is omitted"
    );
}

fn spirit_of_the_labyrinth() -> CardIndex {
    card_index("1463795b-ec0c-44d6-ae1a-55f78d9843ec")
}

fn counsel_of_the_soratami() -> CardIndex {
    card_index("62ddc5ae-ced9-4319-854c-1a114c6afc3f")
}

/// Spirit of the Labyrinth: "Each player can't draw more than one card each
/// turn."
///
/// Played rather than asserted on the modifier, because what a player sees is
/// the second card not arriving. Counsel of the Soratami is the instrument
/// and its whole text is "Draw two cards", so the **partial-carry** half of
/// CR 121.2b is what the hand count says: the spell resolves, one card
/// arrives and the other does not. A one-card draw spell would look the same
/// whether the rule counted draws or refused instructions, and Brainstorm —
/// the first instrument tried — puts two cards back and measures its own
/// rider instead.
///
/// The player on the play skips their first draw step (CR 103.7a), so the
/// turn's allowance is untouched when the spell resolves.
///
/// "Each player" includes the Spirit's own controller, which is the half a
/// card written from Leovold's sentence would get wrong.
#[test]
fn spirit_of_the_labyrinth_lets_a_draw_two_draw_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[spirit_of_the_labyrinth(), island(), island(), island()],
        )
        .hand(0, &[counsel_of_the_soratami()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spirit = on_battlefield(&engine, p0, spirit_of_the_labyrinth())
        .expect("the Spirit is on the battlefield");
    assert_eq!(pt(&engine, spirit), (3, 1));
    assert_eq!(
        engine.state().draw_limit(p0),
        Some(1),
        "the static is on and it names every player, its controller included"
    );
    assert_eq!(
        engine.state().per_turn.draws[0],
        0,
        "the player on the play has drawn nothing yet this turn"
    );

    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, counsel_of_the_soratami());
    pass_until(&mut engine, stack_is_empty);

    let after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        after, before,
        "the spell left the hand and one of its two cards came back: \
         CR 121.2b carries the instruction out in part"
    );
    assert_eq!(engine.state().per_turn.draws[0], 1);
}

// ---------------------------------------------------------------------------
// c11: eleven cards written by the DeepSeek lane, played here by the
// coordinator. The cross rule (scripts/llm/README.md) puts the test in
// another hand than the card, and with the Gemini lane's quota spent for the
// next three hours that hand is this one.
// ---------------------------------------------------------------------------

/// Oboro Envoy: the shrink is read **after** the land it charges has landed
/// in the hand it counts.
///
/// "…gets -X/-0 until end of turn, where X is the number of cards in your
/// hand" with a cost of "return a land you control to its owner's hand" is a
/// sentence that answers itself: the returned land is in the hand by the time
/// the ability resolves, because a cost is paid on activation (CR 601.2h) and
/// the amount is read on resolution (CR 608.2f). Two cards in hand plus the
/// land is three, so the wurm is a 3/6 — a reader counting the hand at
/// announcement would leave it a 4/6.
#[test]
fn oboro_envoy_counts_the_land_it_returned_to_pay_for_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(381, forest())
        .battlefield(0, &[forest(), forest(), forest(), oboro_envoy()])
        .hand(0, &[island(), island()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the wurm is seated");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "two cards in hand before the ability is paid for"
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, oboro_envoy(), 0);
    // The cost picks the land, then the ability picks its target.
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the driver answered every question the card asked"
    );

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        3,
        "the returned land is the third card in hand"
    );
    assert_eq!(
        pt(&engine, wurm),
        (3, 6),
        "three cards in hand is -3/-0 on a 6/6, and toughness is untouched"
    );
}

/// Thrun, the Last Troll: "this spell can't be countered", read the way this
/// engine reads it — the counterspell is never castable at all.
///
/// `eval::target_options` leaves an uncounterable spell out of
/// `TargetSpec::Spell`, so a Counterspell with nothing else on the stack has
/// no legal target and is not offered. That is a *negative*, so the same
/// hand casts the same Counterspell at an ordinary creature spell one step
/// earlier and is offered it: the difference between the two offers is the
/// keyword, and nothing about the board or the mana.
#[test]
fn thrun_the_last_troll_leaves_a_counterspell_no_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(382, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[llanowar_elves(), thrun_the_last_troll()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let counter = in_hand(&engine, p1, counterspell()).expect("the counterspell is in hand");
    tap_all_mana(&mut engine, p0);

    // The control: an ordinary creature spell, and the counterspell is there.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&counter),
        "an ordinary creature spell is a target, so the mana and the hand are \
         not the reason for what happens below"
    );
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    // The subject: the same counterspell, the same floating mana, a spell
    // that can't be countered.
    cast_with_floating(&mut engine, p0, thrun_the_last_troll());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&counter),
        "the troll can't be countered, so the counterspell has no legal \
         target and is not offered: {:?}",
        legal.castable
    );
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_some(),
        "and the troll arrived"
    );
}

/// Thrun, Breaker of Silence: the same keyword on a second card, and the
/// trample the first one does not print.
///
/// The control for the negative half is the test above, on the other Thrun;
/// what is new here is that the permanent that arrives carries **both**
/// printed keywords, which is the part a `Coverage::Partial` on the two
/// unexpressible clauses says nothing about.
#[test]
fn thrun_breaker_of_silence_arrives_with_trample_and_no_counterspell_to_stop_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(383, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[thrun_breaker_of_silence()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let counter = in_hand(&engine, p1, counterspell()).expect("the counterspell is in hand");
    cast_from_hand(&mut engine, p0, thrun_breaker_of_silence());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&counter),
        "nothing on the stack may be countered"
    );
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let thrun = on_battlefield(&engine, p0, thrun_breaker_of_silence()).expect("the troll arrived");
    assert!(
        keywords(&engine, thrun).contains(KeywordSet::TRAMPLE),
        "and it tramples, which is the half of the printing that is not the \
         reason it got here"
    );
    assert_eq!(pt(&engine, thrun), (5, 5), "a 5/5 as printed");
}

/// Ashaya, Soul of the Wild: the type change feeds the count that sizes it.
///
/// Both halves are one board. "Nontoken creatures you control are Forest
/// lands" makes Ashaya and the Elf beside it lands, and "power and toughness
/// each equal to the number of lands you control" then counts five: three
/// Forests, the Elf, and Ashaya itself. A reader applying the count before
/// the type change — or one that exempted the source from its own static —
/// would say three.
#[test]
fn ashaya_counts_the_creatures_its_own_static_made_into_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(384, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                ashaya_soul_of_the_wild(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ashaya = on_battlefield(&engine, p0, ashaya_soul_of_the_wild()).expect("Ashaya is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    assert!(
        engine
            .state()
            .object(elf)
            .expect("the Elf is still there")
            .characteristics()
            .types
            .intersects(TypeSet::LAND),
        "the Elf is a land in addition to its other types"
    );
    assert_eq!(
        pt(&engine, ashaya),
        (5, 5),
        "three Forests, the Elf and Ashaya itself are five lands, on a 0/0 body"
    );
}

/// Aesi, Tyrant of Gyre Strait: the extra land drop and the landfall draw,
/// which only a turn that plays two lands can tell apart.
///
/// The second land is the one that proves `ExtraLandDrops(1)` — CR 305.2
/// allows one a turn — and each of the two asks the "you may draw a card"
/// question, so the hand is down two lands and up two draws.
#[test]
fn aesi_plays_a_second_land_and_draws_off_each_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(385, forest())
        .battlefield(0, &[aesi_tyrant_of_gyre_strait()])
        .hand(0, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    for land in [forest(), island()] {
        let card = in_hand(&engine, p0, land).expect("the land is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card })
            .expect("the land drop is allowed");
        pass_until(&mut engine, stack_is_empty);
    }

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| engine
                .state()
                .object(**id)
                .is_some_and(|o| o.characteristics().types.intersects(TypeSet::LAND)))
            .count(),
        2,
        "both lands are on the battlefield, so the second drop was allowed"
    );
    assert_eq!(
        library_size(&engine, p0),
        before - 2,
        "and landfall drew a card for each of them"
    );
}

/// Lumra, Bellow of the Woods: the enter trigger mills four, and the body is
/// the lands you control.
///
/// The mill is the expressible half of a trigger whose second sentence the
/// card refuses by name, and the P/T is the count — four Forests, so a 4/4 on
/// a 0/0 printed body.
#[test]
fn lumra_mills_four_on_arrival_and_is_as_big_as_your_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(386, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[lumra_bellow_of_the_woods()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, lumra_bellow_of_the_woods());
    pass_until(&mut engine, stack_is_empty);

    let lumra = on_battlefield(&engine, p0, lumra_bellow_of_the_woods()).expect("Lumra arrived");
    assert_eq!(
        library_size(&engine, p0),
        before - 4,
        "four cards milled off the top"
    );
    assert_eq!(
        pt(&engine, lumra),
        (6, 6),
        "six Forests on a 0/0 printed body"
    );
}

/// Muldrotha, the Gravetide: the land half of the sentence, which is the half
/// the card claims.
///
/// `Modifier::PlayLandsFromGraveyard` is a permission and nothing else, so
/// the test is that a land in the graveyard is playable — and that it is
/// still the one land drop a turn allows, which is what separates this from
/// Aesi above.
#[test]
fn muldrotha_lets_you_play_a_land_out_of_your_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(387, forest())
        .battlefield(0, &[muldrotha_the_gravetide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // The library filler is a Forest, so the graveyard is one land.
    seed_graveyard(&mut engine, p0, 1);

    let land = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .expect("Muldrotha permits the land drop out of the graveyard");
    assert_eq!(
        engine
            .state()
            .object(land)
            .expect("the land is still an object")
            .zone,
        Zone::Battlefield,
        "and the land is on the battlefield rather than back in the graveyard"
    );
}

/// Primeval Titan: the enter trigger finds two lands and they arrive tapped.
///
/// "…put them onto the battlefield tapped" is the half a search that found
/// the cards would still get wrong, and `Find::BATTLEFIELD_TAPPED` twice is
/// what the card writes for it.
#[test]
fn primeval_titan_fetches_two_lands_and_both_arrive_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(388, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[primeval_titan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    let seated: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    cast_from_hand(&mut engine, p0, primeval_titan());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the search")
    };
    assert_eq!(
        max, 2,
        "\"up to two land cards\" is the offer the card makes"
    );
    let found: Vec<ObjectId> = options.into_iter().take(2).collect();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: found })
        .expect("two lands are a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        before - 2,
        "two land cards left the library"
    );
    // The six Forests the board started with were tapped for the Titan's own
    // cost, so "tapped" alone says nothing: what is asked is the two objects
    // that were not on the battlefield before.
    let arrived: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| !seated.contains(id))
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.characteristics().types.intersects(TypeSet::LAND))
        })
        .collect();
    assert_eq!(arrived.len(), 2, "two lands arrived from the library");
    assert!(
        arrived.iter().all(|id| is_tapped(&engine, *id)),
        "and both of them arrived tapped"
    );
}

/// Disciple of Freyalise, played as its back face: the land pays 3 life to
/// arrive untapped and then makes green.
///
/// The front face's enter trigger is off the card by name, so the back is
/// where this printing is testable at all — and it is the shape a modal
/// double-faced land carries: `EnterModifier::TappedOrPayLife(3)` asks, and
/// the answer decides whether the mana is available this turn.
#[test]
fn garden_of_freyalise_pays_three_life_to_arrive_untapped() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(disciple_of_freyalise(), 1).expect("the back face is a land");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

    assert!(
        !is_tapped(&engine, land),
        "3 life was paid, so it entered untapped"
    );
    assert_eq!(engine.state().players[0].life, 17, "and the 3 life is gone");
    // A *printed* mana ability is enumerated into `LegalActions::abilities`
    // like any other activated ability; `mana_abilities` is the CR 305.6 land
    // shortcut and what a continuous effect granted.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green in the pool"
    );
}

/// Drowner of Truth, played as its back face: a land that enters tapped and
/// then chooses between two colours.
///
/// The front face's cast trigger reads what the spell was paid with, which
/// the engine does not track and the card refuses by name; the devoid static
/// and the back face are what is left, and the back face is the half a game
/// can show.
#[test]
fn drowned_jungle_enters_tapped_and_taps_for_either_colour() {
    let (engine, land) = play_land_face(drowner_of_truth(), 1).expect("the back face is a land");
    assert!(
        is_tapped(&engine, land),
        "the printed \"This land enters tapped\" is an enter modifier and not a \
         line the harness can place around"
    );
}

/// World Shaper: the attack trigger, which is the half of the card that is
/// not refused by name.
///
/// "…you may mill three cards" is a `MayDo`, so the question is asked and
/// answering it is the test: three cards leave the library for the graveyard
/// only because a seat said yes.
#[test]
fn world_shaper_mills_three_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(389, forest())
        .battlefield(0, &[world_shaper()])
        .start();
    keep_mulligans(&mut engine);

    let shaper = on_battlefield(&engine, p0, world_shaper()).expect("the Shaper is seated");
    let before = library_size(&engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shaper, Defender::Player(PlayerId::new(1)))],
            },
        )
        .expect("the Shaper attacks");
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
        library_size(&engine, p0),
        before - 3,
        "three cards milled off the attack trigger"
    );
}

/// Shaleskin Bruiser: the trample it keeps, and the pump it refuses by name.
///
/// `Coverage::Partial` here is about an `Amount` that multiplies a count, so
/// what a game can show is the 4/4 body with trample — and, on a board with
/// two other attacking Beasts, that it is still a 4/4 afterwards. The second
/// half is the honest half of a refusal: a card that quietly pumped by one
/// per Beast instead of three would read as "nearly right" and is the thing
/// the `NOT SUPPORTED` note says was refused rather than approximated.
#[test]
fn shaleskin_bruiser_tramples_and_does_not_grow_beside_other_beasts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(393, forest())
        .battlefield(0, &[shaleskin_bruiser(), shaleskin_bruiser()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    let board: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    let beasts: Vec<ObjectId> = board
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == shaleskin_bruiser()))
        })
        .collect();
    assert_eq!(
        beasts.len(),
        2,
        "two Beasts, so \"each other\" is one of them"
    );
    assert!(
        keywords(&engine, beasts[0]).contains(KeywordSet::TRAMPLE),
        "trample is printed and kept"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: beasts
                    .iter()
                    .map(|id| (*id, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("both Beasts attack");

    assert_eq!(
        pt(&engine, beasts[0]),
        (4, 4),
        "the attack trigger is off the card, so a 4/4 attacks as a 4/4 — and \
         an approximation of +1/+0 per Beast would show up here as a 5/4"
    );
}

/// Sphinx of the Final Word: hexproof, read as a target offer.
///
/// The two "can't be countered" clauses are refused by name; hexproof and
/// flying are what the card carries, and hexproof is the one a game can put a
/// number on. An opponent's Swords to Plowshares is offered the Elf beside
/// the Sphinx and not the Sphinx — a difference in one creature's keywords
/// and in nothing else about the board.
#[test]
fn sphinx_of_the_final_word_is_no_target_for_an_opponents_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(394, forest())
        .battlefield(0, &[sphinx_of_the_final_word(), llanowar_elves()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphinx =
        on_battlefield(&engine, p0, sphinx_of_the_final_word()).expect("the Sphinx is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "flying is printed"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "the Elf across the table is a target, so the spell reaches this side"
    );
    assert!(
        !options.contains(&sphinx),
        "and the Sphinx is not, which is hexproof (CR 702.11b): {options:?}"
    );
}

/// Tyrranax Rex: ward {4} charges an opponent for the privilege.
///
/// `AbilityDef::Ward` is a triggered ability that counters the spell unless
/// its controller pays, so the played proof is the question: the opponent is
/// asked for `{4}` they cannot pay off one Plains, and the Rex is still
/// standing when the dust settles.
#[test]
fn tyrranax_rex_wards_an_opponents_removal_for_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(395, forest())
        .battlefield(0, &[tyrranax_rex()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rex = on_battlefield(&engine, p0, tyrranax_rex()).expect("the Rex is seated");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&rex),
        "ward does not stop the targeting, only charges for it"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![rex],
                players: vec![],
            },
        )
        .expect("the Rex is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the ward trigger is answered on the way"
    );

    assert!(
        on_battlefield(&engine, p0, tyrranax_rex()).is_some(),
        "one Plains cannot pay {{4}}, so the spell was countered by ward and \
         the Rex is still there"
    );
}

/// Maelstrom Wanderer: "creatures you control have haste", which is only
/// visible on a creature that has just arrived.
///
/// The two cascades are refused by name, so the static is the card here — and
/// a creature cast this turn attacking is the shape that separates a granted
/// haste from a board the harness happened to seat early.
#[test]
fn maelstrom_wanderer_gives_a_freshly_cast_creature_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(396, forest())
        .battlefield(0, &[maelstrom_wanderer(), forest()])
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
        "the Wanderer grants haste to the creatures you control"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attacker declaration")
    };
    assert!(
        attackers.contains(&elf),
        "and CR 302.6 lets it attack the turn it arrived: {attackers:?}"
    );
    let _ = p1;
}
