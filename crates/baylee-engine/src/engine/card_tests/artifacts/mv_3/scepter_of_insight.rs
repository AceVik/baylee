//! Oracle: {3}{U}, {T}: Draw a card.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;

fn subject() -> CardIndex {
    card_index("e2725411-a600-4ff3-94fb-5d52f5eda414")
}

// Checklist: {3}{U}, not generic-only/three mana; tap cost prevents reuse;
// one card drawn from controller's library and opponent unchanged.
#[test]
fn one_card_costs_four_including_blue_and_a_tap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for lands in [
        vec![island(), island(), island()],
        vec![mountain(); 4],
        vec![island(), mountain(), mountain(), mountain()],
    ] {
        let mut board = vec![subject()];
        board.extend(lands);
        let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
        let library = library_size(&e, p0);
        let other = library_size(&e, p1);
        tap_all_mana(&mut e, p0);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        let offered = legal.abilities.iter().any(|(id, _)| *id == source);
        let affordable = board.len() == 5 && board.contains(&island());
        assert_eq!(offered, affordable);
        if affordable {
            activate(&mut e, p0, subject(), 0);
            assert!(is_tapped(&e, source));
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                e.state().zones.list(ZoneLocation::Hand(p0)).len(),
                before + 1
            );
            assert_eq!(library_size(&e, p0), library - 1);
            assert_eq!(library_size(&e, p1), other);
            let Pending::Priority { legal, .. } = e.pending() else {
                panic!("priority")
            };
            assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
        }
    }
}
