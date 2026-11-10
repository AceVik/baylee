#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::TWIDDLE;

fn subject() -> CardIndex {
    card_index("46535f8e-1bcd-4588-ac6c-a4bc89c379c8")
}

// Scenario: first printed ability adds exactly one chosen W/B/G immediately.
// The second pays WBG, taps and sacrifices Banner to draw exactly one.
#[test]
fn abzan_banner_produces_each_printed_color() {
    let p0 = PlayerId::new(0);
    for color in [ManaColor::White, ManaColor::Black, ManaColor::Green] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[subject()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        activate(&mut e, p0, subject(), 0);
        let Pending::ChooseColor { options, .. } = e.pending().clone() else {
            panic!("expected color choice")
        };
        assert_eq!(options.len(), 3);
        assert!(options.contains(&color));
        e.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        assert!(stack_is_empty(&e));
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
        assert_eq!(e.state().players[0].mana_pool.total(), 1);
        let banner = on_battlefield(&e, p0, subject()).unwrap();
        assert!(is_tapped(&e, banner));
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: banner,
                    ability_index: 0
                }
            )
            .is_err()
        );
    }
}

#[test]
fn abzan_banner_pays_wbg_sacrifices_and_draws() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains(), swamp(), forest()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let banner = on_battlefield(&e, p0, subject()).unwrap();
    tap_mana_except(&mut e, p0, banner);
    assert_eq!(e.state().players[0].mana_pool.total(), 3);
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut e, p0, subject(), 1);
    assert!(in_graveyard(&e, p0, subject()).is_some());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn abzan_banner_draw_needs_exact_colors_and_an_untapped_source() {
    let p0 = PlayerId::new(0);
    let mut wrong_color = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut wrong_color);
    assert!(walk_to_own_main(&mut wrong_color, p0));
    let banner = on_battlefield(&wrong_color, p0, subject()).unwrap();
    tap_mana_except(&mut wrong_color, p0, banner);
    assert_eq!(wrong_color.state().players[0].mana_pool.total(), 3);
    let before = wrong_color.snapshot_hash();
    assert!(
        wrong_color
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: banner,
                    ability_index: 1
                }
            )
            .is_err()
    );
    assert_eq!(wrong_color.snapshot_hash(), before);
    assert!(on_battlefield(&wrong_color, p0, subject()).is_some());

    let mut tapped = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains(), swamp(), forest(), island()])
        .hand(0, &[TWIDDLE])
        .start();
    keep_mulligans(&mut tapped);
    assert!(walk_to_own_main(&mut tapped, p0));
    let banner = on_battlefield(&tapped, p0, subject()).unwrap();
    tap_mana_except(&mut tapped, p0, banner);
    cast_with_floating(&mut tapped, p0, TWIDDLE);
    tapped
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![banner],
            },
        )
        .unwrap();
    pass_until(&mut tapped, |game| {
        matches!(game.pending(), Pending::YesNo { .. })
    });
    tapped.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut tapped, stack_is_empty);
    assert!(is_tapped(&tapped, banner));
    assert_eq!(tapped.state().players[0].mana_pool.total(), 3);
    let before = tapped.snapshot_hash();
    assert!(
        tapped
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: banner,
                    ability_index: 1
                }
            )
            .is_err()
    );
    assert_eq!(tapped.snapshot_hash(), before);
    assert!(on_battlefield(&tapped, p0, subject()).is_some());
}
