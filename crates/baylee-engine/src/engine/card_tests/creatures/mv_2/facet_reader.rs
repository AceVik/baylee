#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::TWIDDLE;

fn subject() -> CardIndex {
    card_index("7aefe7a9-d696-4e2e-b1a5-d041d9e4e213")
}

// Scenario: one generic mana plus tap draws exactly one then discards one;
// the selected discard, not a different hand card, reaches graveyard.
#[test]
fn facet_reader_draws_then_discards_one_for_one_mana_and_tap() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, island())
        .battlefield(0, &[subject(), island()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let reader = on_battlefield(&e, p0, subject()).unwrap();
    let discard = in_hand(&e, p0, quiet_artifact()).unwrap();
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&e, p0);
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, subject(), 0);
    assert!(is_tapped(&e, reader));
    pass_until(&mut e, |game| {
        matches!(game.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&discard));
    assert_eq!(library_size(&e, p0), library_before - 1);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p0, quiet_artifact()).is_some());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn facet_reader_requires_both_mana_and_untapped_source() {
    let p0 = PlayerId::new(0);
    let mut no_mana = Duel::new(SEED, island())
        .battlefield(0, &[subject()])
        .start();
    keep_mulligans(&mut no_mana);
    assert!(walk_to_own_main(&mut no_mana, p0));
    let reader = on_battlefield(&no_mana, p0, subject()).unwrap();
    assert!(
        no_mana
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: reader,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert!(!is_tapped(&no_mana, reader));

    let mut tapped = Duel::new(SEED, island())
        .battlefield(0, &[subject(), island(), island()])
        .hand(0, &[TWIDDLE])
        .start();
    keep_mulligans(&mut tapped);
    assert!(walk_to_own_main(&mut tapped, p0));
    let reader = on_battlefield(&tapped, p0, subject()).unwrap();
    cast_from_hand(&mut tapped, p0, TWIDDLE);
    tapped
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![reader],
            },
        )
        .unwrap();
    pass_until(&mut tapped, |game| {
        matches!(game.pending(), Pending::YesNo { .. })
    });
    tapped.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut tapped, stack_is_empty);
    assert!(is_tapped(&tapped, reader));
    assert_eq!(tapped.state().players[0].mana_pool.total(), 1);
    assert!(
        tapped
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: reader,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(tapped.state().players[0].mana_pool.total(), 1);
}
