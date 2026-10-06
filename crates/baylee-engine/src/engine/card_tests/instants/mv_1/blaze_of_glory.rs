//! `cards/instants/mv_1/blaze_of_glory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Cast this spell only before blockers are declared" (CR 506.7b, a
/// spell's own printed timing restriction): refused in the first main
/// phase with mana already floating, accepted (and resolved) at the
/// beginning of combat, still on offer once attackers are declared and no
/// blocks have been made yet, and refused again — mana floating once
/// more — once the declare blockers step is reached. The last of those
/// is the one that isolates the printed restriction itself; see its own
/// comment below.
#[allow(clippy::too_many_lines)] // four checkpoints walked across one turn
#[test]
fn blaze_of_glory_is_castable_only_before_blockers_are_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), fire_sprites()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[blaze_of_glory(), blaze_of_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sprite = on_battlefield(&engine, p0, fire_sprites()).expect("seated");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    // In the first main phase, mana is already floating, so the refusal
    // below is not "not enough mana" — but two gates coincide here, not
    // one: outside combat there is also no legal target for "target
    // creature defending player controls" (`Filter::ControlledByDefendingPlayer`
    // is false outside the combat phase), so this checkpoint alone would
    // refuse the cast even without the card's own timing condition. The
    // declare-blockers-step refusal further down, where a legal target is
    // standing and mana is floating, is what isolates the printed
    // restriction itself.
    float_one_white(&mut engine, p0);
    let first = in_hand(&engine, p0, blaze_of_glory()).expect("the first copy is in hand");
    refused(
        engine.apply(p0, PlayerAction::CastSpell { card: first }),
        "not among the options",
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatBegin
    });
    float_one_white(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: first })
        .expect("castable at the beginning of combat");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature defending player controls\" asks for one, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bear) && !options.contains(&sprite),
        "p1's creature is offered and p0's own creature is not: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, blaze_of_glory()).is_some(),
        "the first copy resolved"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sprite, Defender::Player(p1))],
            },
        )
        .expect("the Sprites attack");
    float_one_white(&mut engine, p0);
    let second = in_hand(&engine, p0, blaze_of_glory()).expect("the second copy is still in hand");
    assert!(
        priority_offer(&engine).castable.contains(&second),
        "still castable once attackers are declared and no blocks are made yet"
    );

    // The Bear cannot block a flier, so `pass_until`'s own default answer
    // of no blocks at all is legal here, and crosses this question safely.
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_white(&mut engine, p0);
    refused(
        engine.apply(p0, PlayerAction::CastSpell { card: second }),
        "not among the options",
    );
}

/// With no attackers declared this combat, the declare blockers step is
/// skipped (CR 508.8, nothing to declare blocks against), so the spell's
/// "only before blockers are declared" window instead runs only through
/// the declare attackers step (CR 506.7e) — and it belongs to whoever
/// holds priority there, not only the active player: the defending player
/// casts it here too, aimed at their own creature, and not at the active
/// player's. Once that step itself ends, the window is gone: with mana
/// still floating, the defending player is refused a second copy at
/// `CombatEnd`.
#[allow(clippy::too_many_lines)] // one cast inside the window, one refusal once it has closed
#[test]
fn blaze_of_glory_is_castable_by_the_defending_player_with_no_attackers_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[plains(), plains(), grizzly_bears()])
        .hand(1, &[blaze_of_glory(), blaze_of_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `pass_until`'s own default answers `ChooseAttackers` with nobody,
    // which is a real choice here: the Bear on p0's board could attack.
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let p0_bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    float_one_white(&mut engine, p1);
    let first = in_hand(&engine, p1, blaze_of_glory()).expect("the first copy is in p1's hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: first })
        .expect("the defending player casts it in the declare attackers step");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature defending player controls\" asks for one, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&their_bear),
        "p1 is the defending player here, and may target their own creature"
    );
    assert!(
        !options.contains(&p0_bear),
        "p0 is the active player here, not the defending one, and is never offered"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![their_bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, blaze_of_glory()).is_some(),
        "it resolved: the cast was accepted"
    );

    // With no attackers, the declare blockers and combat damage steps are
    // skipped (CR 508.8), and this spell's own window closes with the
    // declare attackers step rather than running past it (CR 506.7e): by
    // CombatEnd — mana floating, the Bear still standing as a legal
    // target — the second copy is refused all the same.
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_white(&mut engine, p1);
    let second = in_hand(&engine, p1, blaze_of_glory()).expect("the second copy is still in hand");
    refused(
        engine.apply(p1, PlayerAction::CastSpell { card: second }),
        "not among the options",
    );
}

/// "Target creature defending player controls can block any number of
/// creatures this turn. It blocks each attacking creature this turn if
/// able.": the question names the Unicorn in `capacity` with `most: None`
/// and it blocks all three of p0's ground attackers at once — a
/// declaration blocking fewer is refused — while the flier it cannot block
/// is left out and the Minotaur, though never required, may still add
/// itself to a block the Unicorn alone already satisfies.
#[allow(clippy::too_many_lines)] // one cast, one target check, one capacity, two refusals, one accept
#[test]
fn blaze_of_glorys_target_must_block_every_attacker_it_can() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                grizzly_bears(),
                gray_ogre(),
                scathe_zombies(),
                fire_sprites(),
            ],
        )
        .battlefield(1, &[pearled_unicorn(), hurloon_minotaur()])
        .hand(0, &[blaze_of_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatBegin
    });

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let zombies = on_battlefield(&engine, p0, scathe_zombies()).expect("seated");
    let flier = on_battlefield(&engine, p0, fire_sprites()).expect("seated");
    let unicorn = on_battlefield(&engine, p1, pearled_unicorn()).expect("seated");
    let minotaur = on_battlefield(&engine, p1, hurloon_minotaur()).expect("seated");

    cast_from_hand(&mut engine, p0, blaze_of_glory());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature defending player controls\" asks for one, got {:?}",
            engine.pending()
        )
    };
    let mut options_sorted = options.clone();
    options_sorted.sort_unstable();
    let mut p1_creatures = vec![unicorn, minotaur];
    p1_creatures.sort_unstable();
    assert_eq!(
        options_sorted, p1_creatures,
        "p1's two creatures are offered and none of p0's four attackers are: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![unicorn],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bear, Defender::Player(p1)),
                    (ogre, Defender::Player(p1)),
                    (zombies, Defender::Player(p1)),
                    (flier, Defender::Player(p1)),
                ],
            },
        )
        .expect("all four attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        capacity, obeying, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on exactly this")
    };
    assert_eq!(
        capacity,
        vec![crate::choice::BlockCapacity {
            blocker: unicorn,
            most: None
        }],
        "only the targeted Unicorn may block more than one, and any number: {capacity:?}"
    );
    let mut obeying_sorted = obeying.clone();
    obeying_sorted.sort_unstable();
    let mut expected = vec![(unicorn, bear), (unicorn, ogre), (unicorn, zombies)];
    expected.sort_unstable();
    assert_eq!(
        obeying_sorted, expected,
        "the Unicorn must block every ground attacker; the flier is left out \
         and the Minotaur is named nowhere"
    );

    refused(
        engine.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(unicorn, bear), (unicorn, ogre)],
            },
        ),
        MUST_BLOCK,
    );
    refused(
        engine.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(unicorn, flier)],
            },
        ),
        "not among the options",
    );

    // The question's own declaration (the Unicorn alone) is legal — but so
    // is one that adds the Minotaur into it: the real counter-check that
    // the Minotaur is merely *allowed*, never required, is that this
    // still-legal declaration actually lands, not that a declaration which
    // never named it left it alone.
    let mut with_minotaur = obeying;
    with_minotaur.push((minotaur, bear));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: with_minotaur,
            },
        )
        .expect("the Minotaur may add itself to a block the Unicorn alone already satisfies");
    assert_eq!(
        engine.state().combat.blocked_by(minotaur),
        vec![bear],
        "the Minotaur's own free choice landed on the Bear"
    );
    let mut blockers_of_bear = engine.state().combat.blockers_of(bear);
    blockers_of_bear.sort_unstable();
    let mut expected_bear_blockers = vec![unicorn, minotaur];
    expected_bear_blockers.sort_unstable();
    assert_eq!(
        blockers_of_bear, expected_bear_blockers,
        "the Bear ends up double-blocked: the Unicorn under Blaze of Glory's \
         requirement, the Minotaur by its own choice alongside it"
    );
    assert_eq!(
        engine.state().combat.blockers_of(ogre),
        vec![unicorn],
        "the Ogre is blocked by the Unicorn alone"
    );
    assert_eq!(
        engine.state().combat.blockers_of(zombies),
        vec![unicorn],
        "the Zombies is blocked by the Unicorn alone"
    );
    assert!(
        !engine.state().combat.is_blocked(flier),
        "the flier, which the Unicorn cannot block and nothing else was aimed \
         at, went through unblocked"
    );
}

/// Blaze of Glory's grant is "this turn": cast on the Wall of Stone (0/8,
/// so its own math never asks a division question either way — 0 power
/// gives nothing to divide, and it survives whatever the two attackers
/// deal), the Wall must block every attacker it can in the combat where
/// it resolves. By p0's next combat on the same board, with the same two
/// attackers and no fresh Blaze, the grant is gone: blocking both is
/// refused, blocking one is legal again, and the question requires
/// nothing of it at all.
#[allow(clippy::too_many_lines)] // one combat under the grant, one after it has lapsed
#[test]
fn blaze_of_glory_lasts_one_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), grizzly_bears(), gray_ogre()])
        .battlefield(1, &[wall_of_stone()])
        .hand(0, &[blaze_of_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatBegin
    });

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let wall = on_battlefield(&engine, p1, wall_of_stone()).expect("seated");

    cast_from_hand(&mut engine, p0, blaze_of_glory());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature defending player controls\" asks for one, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![wall],
        "the Wall is p1's only creature, and their only legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1)), (ogre, Defender::Player(p1))],
            },
        )
        .expect("both attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        capacity, obeying, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on exactly this")
    };
    assert_eq!(
        capacity,
        vec![crate::choice::BlockCapacity {
            blocker: wall,
            most: None
        }],
        "the Wall alone may block more than one, and any number: {capacity:?}"
    );
    let mut obeying_sorted = obeying.clone();
    obeying_sorted.sort_unstable();
    let mut expected = vec![(wall, bear), (wall, ogre)];
    expected.sort_unstable();
    assert_eq!(
        obeying_sorted, expected,
        "in the combat where Blaze resolved, the Wall must block every \
         attacker it can"
    );
    refused(
        engine.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bear)],
            },
        ),
        MUST_BLOCK,
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: obeying })
        .expect("blocking both, the question's own declaration, is legal");

    // Past this combat, the rest of this turn, p1's whole turn, and into
    // p0's next: Blaze of Glory's "this turn" is long over, and the Wall
    // — 0 power, 8 toughness, having taken 4 unprevented damage and
    // shrugged it off — is still standing to show it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1)), (ogre, Defender::Player(p1))],
            },
        )
        .expect("the same two attack again, a turn cycle later");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        capacity, obeying, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on exactly this")
    };
    assert!(
        capacity.is_empty(),
        "with the grant long since lapsed, the Wall is named nowhere in \
         `capacity`: back to an ordinary single block: {capacity:?}"
    );
    assert!(
        obeying.is_empty(),
        "and the question requires nothing of it: {obeying:?}"
    );
    match engine.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, bear), (wall, ogre)],
        },
    ) {
        Err(EngineError::IllegalAction(message)) => {
            assert_eq!(message, "creature cannot block that many attackers");
        }
        other => panic!("expected the capacity refusal, got {other:?}"),
    }
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bear)],
            },
        )
        .expect("blocking one, ordinary again, is legal");
}
