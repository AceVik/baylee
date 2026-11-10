#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("ac2668a3-7a26-47b1-9ad4-56e23643de80")
}

// Scenario: {2}{U} buys scry one; both top and bottom are legal, no tap is
// paid, and a second activation is available while enough mana remains.
#[test]
fn tidepool_turtle_scry_one_can_bottom_and_activate_again() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                subject(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let turtle = on_battlefield(&e, p0, subject()).unwrap();
    let before = library_size(&e, p0);
    tap_all_mana(&mut e, p0);
    for bottom in [true, false] {
        activate(&mut e, p0, subject(), 0);
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
        e.apply(p0, look_answer(&cards, if bottom { &cards } else { &[] }))
            .unwrap();
        pass_until(&mut e, stack_is_empty);
        let library = e.state().zones.list(ZoneLocation::Library(p0));
        assert_eq!(
            if bottom {
                library.first()
            } else {
                library.last()
            },
            Some(&inspected)
        );
        assert!(!is_tapped(&e, turtle));
        assert_eq!(library_size(&e, p0), before);
    }
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn tidepool_turtle_needs_three_mana_including_blue() {
    let p0 = PlayerId::new(0);
    for lands in [vec![forest(), forest(), forest()], vec![island(), island()]] {
        let mut board = vec![subject()];
        board.extend(lands);
        let mut e = Duel::new(SEED, island()).battlefield(0, &board).start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let turtle = on_battlefield(&e, p0, subject()).unwrap();
        tap_all_mana(&mut e, p0);
        assert!(library_size(&e, p0) > 0);
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: turtle,
                    ability_index: 0
                }
            )
            .is_err()
        );
        assert!(!is_tapped(&e, turtle));
    }
}
