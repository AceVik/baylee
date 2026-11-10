//! `cards/creatures/mv_3/whip_vine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::WIND_DRAKE;

#[test]
fn whip_vine_blocks_flyer_taps_and_locks_it_while_whip_vine_stays_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("4e90ddce-1765-44e4-9b3c-e576f3a1bcd4");
    let flyer_card = WIND_DRAKE;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, forest()])
        .battlefield(1, &[flyer_card])
        .start();
    keep_mulligans(&mut engine);

    let vine = on_battlefield(&engine, p0, card).unwrap();
    let flyer = on_battlefield(&engine, p1, flyer_card).unwrap();

    // Advance to p1's declare-attackers step; Wind Drake (2/2 flying) attacks p0.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(flyer, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Whip Vine (1/4 reach) blocks Wind Drake (2/2 flying).
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(vine, flyer)],
            },
        )
        .unwrap();

    // Activate Whip Vine's {T} ability targeting the blocked flyer.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut engine, p0, card, 1);
    aim_at(&mut engine, p0, flyer);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, vine));
    assert!(is_tapped(&engine, flyer));

    // On p0's next untap step, choose not to untap Whip Vine (ChoicePrompt::LeaveTapped).
    let mut asked = false;
    for _ in 0..100 {
        if let Pending::ChooseCards {
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            assert!(options.contains(&vine));
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![vine],
                    },
                )
                .unwrap();
            asked = true;
            break;
        }
        let (player, action) = answer_one(&engine).expect("walk to untap");
        engine.apply(player, action).unwrap();
    }
    assert!(asked, "untap step should ask LeaveTapped");
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, vine));

    // On p1's next turn, Wind Drake remains tapped because Whip Vine remained tapped.
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, flyer));
}

/// p1 attacks p0 with `attackers`; p0 blocks with the Whip Vine as `blocks`
/// says; the game stops with p0 holding priority in the declare blockers
/// step.
fn attack_and_block(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    blocks: &[(ObjectId, ObjectId)],
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(p0)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .unwrap();
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == PlayerId::new(0)),
    );
}

#[test]
fn whip_vine_cannot_lock_a_blocked_ground_creature_or_an_unblocked_flyer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("4e90ddce-1765-44e4-9b3c-e576f3a1bcd4");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, forest()])
        .battlefield(1, &[WIND_DRAKE, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    let vine = on_battlefield(&engine, p0, card).unwrap();
    let drake = on_battlefield(&engine, p1, WIND_DRAKE).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    // The Vine blocks the Bears (no flying); the Wind Drake (flying) is not
    // blocked by it. Neither is "a creature with flying blocked by this".
    attack_and_block(&mut engine, &[drake, bear], &[(vine, bear)]);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.abilities.contains(&(vine, 1)));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: vine,
                    ability_index: 1,
                },
            )
            .is_err(),
        "no creature with flying is blocked by the Vine"
    );
    assert!(!is_tapped(&engine, vine), "the {{T}} cost was not paid");
}

#[test]
fn whip_vine_releases_the_flyer_once_it_untaps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("4e90ddce-1765-44e4-9b3c-e576f3a1bcd4");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, forest()])
        .battlefield(1, &[WIND_DRAKE])
        .start();
    keep_mulligans(&mut engine);
    let vine = on_battlefield(&engine, p0, card).unwrap();
    let drake = on_battlefield(&engine, p1, WIND_DRAKE).unwrap();
    attack_and_block(&mut engine, &[drake], &[(vine, drake)]);
    activate(&mut engine, p0, card, 1);
    aim_at(&mut engine, p0, drake);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, vine));
    // p0 untaps normally (nothing is left tapped on purpose) ...
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, vine));
    // ... so the Wind Drake untaps in its controller's next untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, drake),
        "the lock lasts only as long as the Vine stays tapped"
    );
}
