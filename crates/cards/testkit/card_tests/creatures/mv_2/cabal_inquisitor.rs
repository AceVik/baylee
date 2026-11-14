//! `cards/creatures/mv_2/cabal_inquisitor.rs`, played.
//!
//! Oracle:
//! - "Threshold — {1}{B}, {T}, Exile two cards from your graveyard: Target
//!   player discards a card. Activate only as a sorcery and only if there are
//!   seven or more cards in your graveyard."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cabal_inquisitor() -> CardIndex {
    card_index("5bebf38e-bff6-470b-a2b8-cb0a87a7372f")
}

#[test]
fn cabal_inquisitor_threshold_and_sorcery_timing_gates_and_discard() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), cabal_inquisitor()])
        .hand(0, &[forest()])
        .hand(1, &[forest(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let inq = on_battlefield(&engine, P0, cabal_inquisitor()).unwrap();
    assert_eq!(pt(&engine, inq), (1, 1));
    seed_graveyard(&mut engine, P0, 6);
    seed_graveyard(&mut engine, P1, 2);
    tap_all_mana(&mut engine, P0);

    // 6 cards in graveyard: Threshold not met.
    assert!(!ability_offered(&engine, inq, 0));

    // 7th card in graveyard: Threshold active during own main phase.
    hand_to_graveyard(&mut engine, P0, forest());
    engine.refresh_offer();
    assert!(ability_offered(&engine, inq, 0));

    // Moving to combat violates sorcery-speed restriction.
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Combat);
    assert!(!ability_offered(&engine, inq, 0));

    // Next own main phase: activate, target opponent, exile two own graveyard cards, force discard.
    assert!(walk_to_own_main(&mut engine, P0));
    tap_all_mana(&mut engine, P0);
    assert!(ability_offered(&engine, inq, 0));
    activate(&mut engine, P0, cabal_inquisitor(), 0);

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&P0) && player_options.contains(&P1));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();

    while let Pending::ChooseCards {
        options,
        min,
        prompt: ChoicePrompt::CostExile,
        ..
    } = engine.pending().clone()
    {
        for id in &options {
            assert_eq!(
                engine.state().object(*id).unwrap().owner,
                P0,
                "must only exile from your own graveyard"
            );
        }
        let take = usize::from(min);
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: options[..take].to_vec(),
                },
            )
            .unwrap();
    }

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        5
    );
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    // Resolves even though graveyard dropped from 7 to 5 when paying the cost.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P1),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("stopped on P1 discard");
    };
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, inq));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_before - 1
    );

    // Graveyard now holds 5 < 7 cards: next turn Threshold is off again.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    tap_all_mana(&mut engine, P0);
    assert!(!ability_offered(&engine, inq, 0));
}
