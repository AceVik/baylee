#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::{LIGHTNING_BOLT, RIPTIDE_LABORATORY, TWIDDLE};

fn subject() -> CardIndex {
    card_index("693d8203-4405-4488-9067-cc1cd8041cb4")
}

// Scenario: {G}, tap, and discard are paid; search offers basic lands in our
// library and puts the chosen one in hand, with no battlefield entry.
#[test]
fn greenseeker_discards_to_find_one_basic_land_for_hand() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), forest()])
        .hand(
            0,
            &[LIGHTNING_BOLT, LIGHTNING_BOLT, RIPTIDE_LABORATORY, island()],
        )
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let seeker = on_battlefield(&e, p0, subject()).unwrap();
    let nonland = hand_to_library_top(&mut e, p0, LIGHTNING_BOLT);
    let nonbasic = hand_to_library_top(&mut e, p0, RIPTIDE_LABORATORY);
    let basic = hand_to_library_top(&mut e, p0, island());
    let discard = in_hand(&e, p0, LIGHTNING_BOLT).unwrap();
    let before = library_size(&e, p0);
    tap_mana_except(&mut e, p0, seeker);
    activate(&mut e, p0, subject(), 0);
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!("expected discard cost")
    };
    assert_eq!(player, p0);
    assert!(options.contains(&discard));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard],
        },
    )
    .unwrap();
    assert!(is_tapped(&e, seeker));
    assert!(in_graveyard(&e, p0, LIGHTNING_BOLT).is_some());
    pass_until(&mut e, |game| {
        matches!(
            game.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert!(!options.is_empty());
    assert!(options.contains(&basic));
    assert!(!options.contains(&nonbasic));
    assert!(!options.contains(&nonland));
    assert!(options.iter().all(|id| {
        e.state()
            .object(*id)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::LAND)
    }));
    let found = basic;
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![found],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(found).unwrap().zone, Zone::Hand);
    assert_eq!(library_size(&e, p0), before - 1);
    assert!(on_battlefield(&e, p0, forest()).is_some());
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(e.state().journal.entries().iter().any(|entry| matches!(&entry.event, crate::event::GameEvent::Revealed { player, cards } if *player == p0 && cards == &vec![found])));
    assert!(e.state().journal.entries().iter().any(|entry| matches!(entry.event, crate::event::GameEvent::Shuffled { player, .. } if player == p0)));
}

#[test]
fn greenseeker_needs_discard_green_mana_and_untapped_source() {
    let p0 = PlayerId::new(0);
    let mut no_card = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), forest()])
        .start();
    keep_mulligans(&mut no_card);
    assert!(walk_to_own_main(&mut no_card, p0));
    let seeker = on_battlefield(&no_card, p0, subject()).unwrap();
    tap_mana_except(&mut no_card, p0, seeker);
    assert!(
        no_card
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: seeker,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert!(on_battlefield(&no_card, p0, subject()).is_some());

    let mut wrong_color = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), island()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut wrong_color);
    assert!(walk_to_own_main(&mut wrong_color, p0));
    let seeker = on_battlefield(&wrong_color, p0, subject()).unwrap();
    tap_mana_except(&mut wrong_color, p0, seeker);
    assert_eq!(wrong_color.state().players[0].mana_pool.total(), 1);
    assert!(
        wrong_color
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: seeker,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert!(in_hand(&wrong_color, p0, LIGHTNING_BOLT).is_some());

    let mut tapped = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), forest(), island()])
        .hand(0, &[LIGHTNING_BOLT, TWIDDLE])
        .start();
    keep_mulligans(&mut tapped);
    assert!(walk_to_own_main(&mut tapped, p0));
    let seeker = on_battlefield(&tapped, p0, subject()).unwrap();
    cast_from_hand(&mut tapped, p0, TWIDDLE);
    tapped
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![seeker],
            },
        )
        .unwrap();
    pass_until(&mut tapped, |game| {
        matches!(game.pending(), Pending::YesNo { .. })
    });
    tapped.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut tapped, stack_is_empty);
    assert!(is_tapped(&tapped, seeker));
    assert_eq!(
        tapped.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(
        tapped
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: seeker,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert!(in_hand(&tapped, p0, LIGHTNING_BOLT).is_some());
}
