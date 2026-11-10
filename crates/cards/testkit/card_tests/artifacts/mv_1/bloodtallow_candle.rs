#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::{HILL_GIANT_HERDGORGER, TWIDDLE};

fn subject() -> CardIndex {
    card_index("da9b3edc-bc2d-4ff5-b614-3626f504c4e9")
}

// Scenario: six mana, tap, sacrifice, and creature-only targeting pay for
// exact -5/-5. A 7/6 survives as 2/1 until cleanup; bystanders do not move.
#[test]
fn bloodtallow_candle_shrinks_one_creature_for_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                subject(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(
            1,
            &[HILL_GIANT_HERDGORGER, llanowar_elves(), quiet_artifact()],
        )
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let giant = on_battlefield(&e, p1, HILL_GIANT_HERDGORGER).unwrap();
    let elf = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    let artifact = on_battlefield(&e, p1, quiet_artifact()).unwrap();
    assert_eq!(pt(&e, giant), (7, 6));
    tap_all_mana_but(&mut e, p0, Some(subject()));
    assert_eq!(e.state().players[0].mana_pool.total(), 6);
    activate(&mut e, p0, subject(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected creature target")
    };
    assert!(options.contains(&giant) && options.contains(&elf));
    assert!(!options.contains(&artifact));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    assert!(in_graveyard(&e, p0, subject()).is_some());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, giant), (2, 1));
    assert_eq!(pt(&e, elf), (1, 1));
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_some());
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |game| {
        game.state().turn.active == p1 && matches!(game.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(pt(&e, giant), (7, 6));
}

#[test]
fn bloodtallow_candle_needs_six_mana_and_an_untapped_source() {
    let p0 = PlayerId::new(0);
    let mut short = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[subject(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[HILL_GIANT_HERDGORGER])
        .start();
    keep_mulligans(&mut short);
    assert!(walk_to_own_main(&mut short, p0));
    let candle = on_battlefield(&short, p0, subject()).unwrap();
    tap_all_mana_but(&mut short, p0, Some(subject()));
    let before = short.snapshot_hash();
    assert!(
        short
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: candle,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(short.snapshot_hash(), before);
    assert!(on_battlefield(&short, p0, subject()).is_some());
    assert_eq!(short.state().players[0].mana_pool.total(), 5);

    let mut tapped = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                subject(),
                island(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[HILL_GIANT_HERDGORGER])
        .hand(0, &[TWIDDLE])
        .start();
    keep_mulligans(&mut tapped);
    assert!(walk_to_own_main(&mut tapped, p0));
    let candle = on_battlefield(&tapped, p0, subject()).unwrap();
    cast_from_hand(&mut tapped, p0, TWIDDLE);
    tapped
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![candle],
            },
        )
        .unwrap();
    pass_until(&mut tapped, |game| {
        matches!(game.pending(), Pending::YesNo { .. })
    });
    tapped.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut tapped, stack_is_empty);
    assert!(is_tapped(&tapped, candle));
    assert_eq!(tapped.state().players[0].mana_pool.total(), 6);
    let before = tapped.snapshot_hash();
    assert!(
        tapped
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: candle,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(tapped.snapshot_hash(), before);
    assert!(on_battlefield(&tapped, p0, subject()).is_some());
}
