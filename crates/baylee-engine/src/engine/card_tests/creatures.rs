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
/// enforced: `combat::can_block` reads the attacker's flying, menace and
/// unblockable and the blocker's flying and reach, and nothing in the
/// engine names the attackers a given blocker may be paired with. So a 1/1
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
         attacker's flying, menace and unblockable and the blocker's flying and \
         reach, and no `Modifier` names the attackers one blocker may be paired \
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
    // castable yet, which is what makes the comparison below a comparison.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "three Swamps; the Halfling is a creature and taps for nothing on its own"
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
/// clause is available: two or more creatures *may* block. The engine offers
/// neither of them, and refuses the pair when it is named anyway.
/// `combat::can_block` asks `state.combat.blockers_of(attacker)` and answers
/// `false` while that list is empty — and it is empty both when
/// `progress_step` builds the `ChooseBlockers` offer and when
/// `declare_blockers` validates every pair *before* recording any of them. So
/// the restriction's first half is enforced (a lone blocker is illegal) and
/// its second half is not (a pair is illegal too), which makes menace read as
/// plain unblockable.
///
/// The two `is_err()` assertions are therefore not the same claim. The
/// one-blocker one is the printed line holding and should stay green forever.
/// The two-blocker one is an **engine** gap and not this card's: when the
/// pairing learns to count declared blockers, that assertion is the one to
/// delete, together with the offer assertion above it — and this card stays
/// `Coverage::Partial`, because what that flag names is the +3/+0 and not
/// menace. The Brazen Borrower test in this same file holds the other end of
/// the same missing pairing rule.
#[test]
#[allow(clippy::too_many_lines)] // one attack, played step by step
fn a_blackbloom_rogue_s_menace_is_offered_to_no_blocker_at_all() {
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
    assert!(
        blockers.iter().all(|o| !o.attackers.contains(&rogue)),
        "neither untapped Elf is paired with the Rogue in the offer, because \
         `can_block` refuses a menace attacker that nothing is blocking yet — \
         and a blocker with no legal attacker is dropped from the offer \
         entirely. When this fires the offer has learned to say `two of these, \
         together` and the `is_err` below it is the next line to go: \
         {blockers:?}"
    );

    // Two blockers is what the card allows, and the engine says no.
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(elves[0], rogue), (elves[1], rogue)],
                },
            )
            .is_err(),
        "`except by two or more creatures` names exactly this block, and it is \
         refused: every pair is validated against `can_block` before any of \
         them is recorded, so the second Elf is never counted for the first"
    );
    // One blocker is refused too, and that half is the printed line holding.
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
