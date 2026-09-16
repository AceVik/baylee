//! Cards whose front face is a creature, the door `cards/creatures/`
//! puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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
    // `mana_abilities` and the Sol Rings do not — an intrinsic land tap
    // and a printed mana ability are two different offers — so both halves
    // are pressed, and only the three artifacts are what Aang and Katara
    // counts.
    tap_all_mana(&mut engine, p0);
    for _ in 0..3 {
        activate(&mut engine, p0, quiet_artifact(), 0);
    }
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

/// CR 608.2g: an effect that needs information about an object no longer in
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
/// (CR 603.6d, CR 400.7), so a trigger that read "target artifact card" would
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
