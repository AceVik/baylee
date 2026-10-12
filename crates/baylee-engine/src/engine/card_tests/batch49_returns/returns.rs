#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch49_paradise_returns_during_next_own_untap_even_if_untapped_early() {
    let mut e = prepared(&[], &[index::UNDISCOVERED_PARADISE], &[]);
    let land = on_battlefield(&e, P0, index::UNDISCOVERED_PARADISE).unwrap();
    activate(&mut e, P0, index::UNDISCOVERED_PARADISE, 0);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert!(stack_is_empty(&e));
    e.state.set_tapped(land, false);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    let turn = e.state.turn.number;
    // No priority is possible in untap; first upkeep priority already sees hand.
    pass_until(&mut e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == P0
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
    assert!(stack_is_empty(&e));
}

#[test]
fn batch49_paradise_does_not_follow_a_blinked_incarnation() {
    let mut e = prepared(&[], &[index::UNDISCOVERED_PARADISE], &[]);
    let land = on_battlefield(&e, P0, index::UNDISCOVERED_PARADISE).unwrap();
    activate(&mut e, P0, index::UNDISCOVERED_PARADISE, 0);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    blink(&mut e, land, P0);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
}

#[test]
fn batch49_glaciers_search_and_cleanup_return_are_separate_stack_resolutions() {
    let mut e = prepared(&[], &[index::THAWING_GLACIERS], &[]);
    let land = on_battlefield(&e, P0, index::THAWING_GLACIERS).unwrap();
    e.state.set_tapped(land, false);
    activate(&mut e, P0, index::THAWING_GLACIERS, 0);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let basic = options[0];
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![basic],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, basic));
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::Cleanup && !stack_is_empty(e)
    });
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
}

#[test]
fn batch49_maze_exiles_only_opponents_attacker_and_returns_it_tapped_next_end() {
    let mut e = prepared(&[], &[index::MYSTIFYING_MAZE], &[index::GRIZZLY_BEARS]);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    attack(&mut e, P1, bear, P0);
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, P0, index::MYSTIFYING_MAZE, 1);
    targets(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
    let life = e.state.players[0].life;
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(bear).unwrap().controller, P1);
    assert!(is_tapped(&e, bear));
    assert_eq!(e.state.players[0].life, life);
}

#[test]
fn batch49_glaciers_failed_search_still_schedules_return_and_does_not_follow_blink() {
    for blink_source in [false, true] {
        let mut e = prepared(&[], &[index::THAWING_GLACIERS], &[]);
        let land = on_battlefield(&e, P0, index::THAWING_GLACIERS).unwrap();
        e.state.set_tapped(land, false);
        activate(&mut e, P0, index::THAWING_GLACIERS, 0);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::SearchLibrary,
                    ..
                }
            )
        });
        e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
            .unwrap();
        pass_until(&mut e, stack_is_empty);
        if blink_source {
            blink(&mut e, land, P0);
        }
        let turn = e.state.turn.number;
        main_after(&mut e, P1, turn);
        assert_eq!(
            e.state.object(land).unwrap().zone,
            if blink_source {
                Zone::Battlefield
            } else {
                Zone::Hand
            }
        );
    }
}
