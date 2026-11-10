//! Oracle: This creature gets +1/+2 as long as you control a Forest.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;

fn subject() -> CardIndex {
    card_index("18961e25-3feb-4dba-a77f-240a4d664ecb")
}

// Checklist: no own Forest/opponent Forest; one and multiple own Forests;
// exact +1/+2 only once; a real bounce of last Forest removes bonus.
#[test]
fn forest_condition_tracks_controller_and_last_forest() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for existing in [false, true] {
        let mut board = vec![subject(), island(), island()];
        if existing {
            board.push(forest());
        }
        let mut e = Duel::new(SEED, island())
            .battlefield(0, &board)
            .battlefield(1, &[forest()])
            .hand(0, &[forest(), boomerang()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let lion = on_battlefield(&e, p0, subject()).unwrap();
        assert_eq!(pt(&e, lion), if existing { (2, 3) } else { (1, 1) });
        let land = in_hand(&e, p0, forest()).unwrap();
        e.apply(p0, PlayerAction::PlayLand { card: land }).unwrap();
        assert_eq!(pt(&e, lion), (2, 3));
        cast_from_hand(&mut e, p0, boomerang());
        aim_at(&mut e, p0, land);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(pt(&e, lion), if existing { (2, 3) } else { (1, 1) });
        assert!(on_battlefield(&e, p1, forest()).is_some());
    }
}
