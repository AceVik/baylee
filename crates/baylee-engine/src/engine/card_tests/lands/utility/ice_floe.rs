//! `cards/lands/utility/ice_floe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ice Floe prints "You may choose not to untap this land during your untap
/// step." and "{T}: Tap target creature without flying that's attacking you.
/// It doesn't untap during its controller's untap step for as long as this
/// land remains tapped."
///
/// Turn two: the opponent attacks with an Elf and a Serra Angel (flying) and
/// keeps a Gray Ogre home; I own a Gray Ogre too. The attackers are declared
/// and my Floe's ability is on the menu. Returns the engine with the Floe
/// activated at the Elf and resolved.
fn floe_on_the_elf() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ice_floe(), gray_ogre()])
        .battlefield(1, &[llanowar_elves(), serra_angel(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let floe = on_battlefield(&engine, p0, ice_floe()).expect("Ice Floe");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("Elf");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("Angel");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected choose attackers, got {:?}", engine.pending());
    };
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, defenders[0]), (angel, defenders[0])],
            },
        )
        .unwrap();
    if let Pending::Priority { player, .. } = engine.pending().clone()
        && player == p1
    {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    activate(&mut engine, p0, ice_floe(), 1);
    let options = aim_at(&mut engine, p0, elf);
    assert_eq!(
        options,
        vec![elf],
        "only the attacking non-flyer: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, floe), "{{T}} was paid");
    assert!(is_tapped(&engine, elf));
    (engine, floe, elf)
}

/// Walks until the untap step asks which permanents stay tapped, handing the
/// question back unanswered (`answer_one` would untap).
#[track_caller]
fn to_the_untap_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    for _ in 0..200 {
        if let Pending::ChooseCards {
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            return options;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way");
        engine.apply(player, action).expect("legal");
    }
    panic!("the untap step never asked");
}

/// While Floe stays tapped (I choose not to untap it) the Elf stays tapped
/// through its controller's next untap step.
#[test]
fn ice_floe_keeps_the_attacker_down_through_its_untap_step_while_it_stays_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, floe, elf) = floe_on_the_elf();
    let options = to_the_untap_question(&mut engine);
    assert!(options.contains(&floe), "the Floe may stay down");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![floe],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, floe), "stayed tapped into my main phase");
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, elf), "the lock held in its untap step");
}

/// Once the Floe untaps, the Elf untaps normally in its next untap step.
#[test]
fn ice_floe_releases_the_attacker_after_it_untaps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, floe, elf) = floe_on_the_elf();
    to_the_untap_question(&mut engine);
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, floe), "untapped");
    assert!(is_tapped(&engine, elf), "the Elf waits for its own step");
    reach_their_main_phase(&mut engine, p1);
    assert!(!is_tapped(&engine, elf), "released: untapped normally");
}

mod local_regressions {
    //! `cards/lands/utility/ice_floe.rs`, played.

    #[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
    use super::*;

    /// Ice Floe prints `You may choose not to untap this land during your untap step.` and `{{T}}: Tap target creature without flying that's attacking you. It doesn't untap during its controller's untap step for as long as this land remains tapped.`
    ///
    /// The untap choice keeps the source tapped and the targeted creature locked through its next untap.
    /// During an opponent's combat, when attacked by a non-flying creature (`llanowar_elves()`), ability 1 taps the attacker and taps Ice Floe.
    /// On the controller's subsequent untap step, the engine offers the `crate::choice::ChoicePrompt::LeaveTapped` question, and electing to keep Ice Floe tapped leaves it tapped into the main phase.
    #[test]
    fn ice_floe_taps_attacking_nonflyer_and_may_remain_tapped() {
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[ice_floe()])
            .battlefield(1, &[llanowar_elves()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let floe = on_battlefield(&engine, p0, ice_floe()).expect("ice floe on battlefield");
        let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf on battlefield");

        // Advance to p1's declare attackers step.
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
        );

        let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
            panic!("expected choose attackers, got {:?}", engine.pending());
        };
        let defender = defenders[0];
        engine
            .apply(
                p1,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(elf, defender)],
                },
            )
            .unwrap();

        // In declare attackers priority, pass p1's priority if held so p0 can respond.
        if let Pending::Priority { player, .. } = engine.pending().clone()
            && player == p1
        {
            engine.apply(p1, PlayerAction::PassPriority).unwrap();
        }

        activate(&mut engine, p0, ice_floe(), 1);

        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("expected target choice, got {:?}", engine.pending());
        };
        assert!(options.contains(&elf), "attacking elf is a legal target");

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
        assert!(is_tapped(&engine, floe), "ice floe tapped to activate");

        // Progress through the rest of the turn to p0's untap step.
        let mut asked_leave_tapped = false;
        for _ in 0..100 {
            if let Pending::ChooseCards {
                player,
                options,
                prompt: crate::choice::ChoicePrompt::LeaveTapped,
                ..
            } = engine.pending().clone()
            {
                assert_eq!(player, p0);
                assert!(
                    options.contains(&floe),
                    "ice floe is offered to leave tapped"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![floe],
                        },
                    )
                    .unwrap();
                asked_leave_tapped = true;
                break;
            }
            let (player, action) = answer_one(&engine).expect("walk to untap step");
            engine.apply(player, action).unwrap();
        }
        assert!(
            asked_leave_tapped,
            "untap step asked whether to leave Ice Floe tapped"
        );

        reach_main_phase(&mut engine, p0);
        assert!(
            is_tapped(&engine, floe),
            "ice floe remained tapped into main phase"
        );
        reach_their_main_phase(&mut engine, p1);
        assert!(
            is_tapped(&engine, elf),
            "the source remained tapped, so the target missed its untap"
        );
    }

    #[test]
    fn ice_floe_only_targets_nonflyers_attacking_its_controller_at_three_seats() {
        use baylee_core::ids::Defender;
        let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
        let mut engine = Duel::table(SEED, forest(), 3)
            .battlefield(0, &[ice_floe()])
            .battlefield(
                1,
                &[
                    baylee_core::generated::index::GRIZZLY_BEARS,
                    giant_spider(),
                    baylee_core::generated::index::AIR_ELEMENTAL,
                ],
            )
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let bear =
            on_battlefield(&engine, p1, baylee_core::generated::index::GRIZZLY_BEARS).unwrap();
        let spider = on_battlefield(&engine, p1, giant_spider()).unwrap();
        let flyer =
            on_battlefield(&engine, p1, baylee_core::generated::index::AIR_ELEMENTAL).unwrap();
        pass_until(
            &mut engine,
            |e| matches!(e.pending(),Pending::ChooseAttackers { player,.. } if *player==p1),
        );
        engine
            .apply(
                p1,
                PlayerAction::DeclareAttackers {
                    attackers: vec![
                        (bear, Defender::Player(p0)),
                        (spider, Defender::Player(p2)),
                        (flyer, Defender::Player(p0)),
                    ],
                },
            )
            .unwrap();
        pass_until(
            &mut engine,
            |e| matches!(e.pending(),Pending::Priority { player,.. } if *player==p0),
        );
        activate(&mut engine, p0, ice_floe(), 1);
        let Pending::ChooseTargets { options, .. } = engine.pending() else {
            panic!("target choice")
        };
        assert_eq!(options, &vec![bear]);
        aim_at(&mut engine, p0, bear);
        pass_until(&mut engine, stack_is_empty);
        assert!(is_tapped(&engine, bear));
    }
}
