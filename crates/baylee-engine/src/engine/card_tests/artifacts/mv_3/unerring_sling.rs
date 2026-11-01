//! `cards/artifacts/mv_3/unerring_sling.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn unerring_sling() -> CardIndex {
    card_index("10a52a73-157f-475e-975a-eb00c80573d4")
}

fn hill_giant() -> CardIndex {
    baylee_core::generated::index::HILL_GIANT
}

fn wind_drake() -> CardIndex {
    baylee_core::generated::index::WIND_DRAKE
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

/// Unerring Sling (`{3}`, `Coverage::Implemented`): `{3}, {T}`, tap an untapped
/// creature you control: deals damage equal to the tapped creature's power to
/// target attacking or blocking creature with flying.
#[test]
fn unerring_sling_taps_creature_to_deal_its_power_to_attacking_flier() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[unerring_sling(), hill_giant(), forest(), forest(), forest()],
        )
        .battlefield(1, &[wind_drake(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sling = on_battlefield(&engine, p0, unerring_sling()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let drake = on_battlefield(&engine, p1, wind_drake()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // Advance to p1's combat and have both Wind Drake (flying) and Grizzly Bears (no flying) attack.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drake, Defender::Player(p0)), (bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sling,
                ability_index: 0,
            },
        )
        .unwrap();

    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                assert_eq!(
                    options,
                    vec![drake],
                    "only attacking creature with flying is a legal target"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![drake],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostTap,
                ..
            } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![giant],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, giant));
    assert!(
        on_battlefield(&engine, p1, wind_drake()).is_none(),
        "3/3 Hill Giant's power (3) destroys 2/2 Wind Drake"
    );
}
