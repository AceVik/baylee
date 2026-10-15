#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("0844f4e6-2b98-4d09-a5c5-92f0a3b6a517")
}

// Scenario: entry scries exactly one and can send it to bottom; then the tap
// mana ability adds one selected color without using the stack.
#[test]
fn mana_geode_scries_on_entry_and_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    for (n, color) in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ]
    .into_iter()
    .enumerate()
    {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[forest(), forest(), forest()])
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let before = library_size(&e, p0);
        cast_from_hand(&mut e, p0, subject());
        pass_until(&mut e, |game| {
            matches!(game.pending(), Pending::Arrange { .. })
        });
        let Pending::Arrange {
            player,
            cards,
            piles,
            ..
        } = e.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!(player, p0);
        assert_eq!(cards.len(), 1);
        assert_eq!(piles, scry_piles(1));
        let inspected = cards[0];
        let bottom = n % 2 == 0;
        e.apply(p0, look_answer(&cards, if bottom { &cards } else { &[] }))
            .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, p0), before);
        let library = e.state().zones.list(ZoneLocation::Library(p0));
        assert_eq!(
            if bottom {
                library.first()
            } else {
                library.last()
            },
            Some(&inspected)
        );
        let geode = on_battlefield(&e, p0, subject()).unwrap();
        pass_until(
            &mut e,
            |game| matches!(game.pending(), Pending::Priority { player, .. } if *player == p0),
        );
        // Entry scry is the first printed ability; tap-for-mana is the second.
        activate(&mut e, p0, subject(), 1);
        let Pending::ChooseColor { options, .. } = e.pending().clone() else {
            panic!("expected color choice")
        };
        assert_eq!(options.len(), 5);
        assert!(options.contains(&color));
        e.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        assert!(stack_is_empty(&e));
        assert!(is_tapped(&e, geode));
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
        assert_eq!(e.state().players[0].mana_pool.total(), 1);
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: geode,
                    ability_index: 1
                }
            )
            .is_err()
        );
    }
}
