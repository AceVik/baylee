//! Oracle: {2}{U}: Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;

fn subject() -> CardIndex {
    card_index("6f5e6799-cb0f-4403-9c2a-69abeac08b79")
}

fn watchful_automaton() -> CardIndex {
    subject()
}

// Checklist: {2}{U} affordability; no tap/summoning restriction; scry exactly
// one, keep and bottom alternatives, repeat for another three mana; no draw.
#[test]
fn scry_keeps_or_bottoms_one_and_can_repeat_without_tapping() {
    let p0 = PlayerId::new(0);
    for bottom in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                ],
            )
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        cast_from_hand(&mut e, p0, subject());
        pass_until(&mut e, stack_is_empty);
        let source = on_battlefield(&e, p0, subject()).unwrap();
        let hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
        for _ in 0..2 {
            let before = e.state().zones.list(ZoneLocation::Library(p0)).clone();
            activate(&mut e, p0, subject(), 0);
            pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
            let Pending::Arrange {
                player,
                cards,
                prompt,
                ..
            } = e.pending().clone()
            else {
                unreachable!()
            };
            assert_eq!(player, p0);
            assert_eq!(prompt, ArrangePrompt::Scry);
            assert_eq!(cards.len(), 1);
            let top = cards[0];
            let away = if bottom { cards.clone() } else { vec![] };
            e.apply(p0, look_answer(&cards, &away)).unwrap();
            pass_until(&mut e, stack_is_empty);
            let after = e.state().zones.list(ZoneLocation::Library(p0));
            assert_eq!(after.len(), before.len());
            let position_before = before.iter().position(|id| *id == top).unwrap();
            let position_after = after.iter().position(|id| *id == top).unwrap();
            if bottom {
                assert_eq!(position_before, before.len() - 1);
                assert_eq!(position_after, 0);
                let mut expected = before.clone();
                expected.pop();
                expected.insert(0, top);
                assert_eq!(after, &expected);
            } else {
                assert_eq!(after, &before);
            }
            assert!(!is_tapped(&e, source));
            assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), hand);
        }
        assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Blue), 0);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
    }
}

// Colored cost boundary: three generic mana and only two blue mana both fail.
#[test]
fn scry_needs_three_mana_including_blue() {
    let p0 = PlayerId::new(0);
    for lands in [vec![mountain(); 3], vec![island(); 2]] {
        let mut board = vec![subject()];
        board.extend(lands);
        let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        tap_all_mana(&mut e, p0);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
    }
}

/// Watchful Automaton — "{2}{U}: Scry 1."
#[test]
fn watchful_automaton_scries_one_for_two_and_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[watchful_automaton(), island(), forest(), forest()]);
    let automaton = on_battlefield(&engine, p0, watchful_automaton()).expect("seated");
    assert_eq!(pt(&engine, automaton), (2, 2));
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, watchful_automaton(), 0);
    let before = answer_scry(&mut engine, 1, true);

    assert_scried(&engine, &before, 1, true);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
