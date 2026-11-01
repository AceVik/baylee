#[allow(clippy::wildcard_imports)]
use super::*;

fn mishra_s_helix() -> CardIndex {
    card_index("6b1552f2-df64-485a-8509-b1916edf19cf")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn mishras_helix_uses_exactly_x_distinct_land_targets_including_zero() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for x in [0, 2] {
        let mut e = lk::game(
            SEED,
            &[mishra_s_helix(), forest()],
            &[island(), baylee_core::generated::index::GRIZZLY_BEARS],
            &[],
            &[],
        );
        let mine = lk::obj(&e, p0, forest());
        let theirs = lk::obj(&e, p1, island());
        let bear = lk::obj(&e, p1, baylee_core::generated::index::GRIZZLY_BEARS);
        lk::float(&mut e, p0, &[(ManaColor::Colorless, 2)]);
        activate(&mut e, p0, mishra_s_helix(), 0);
        e.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
        if x == 2 {
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![mine, bear],
                        players: vec![]
                    }
                )
                .is_err()
            );
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![mine],
                        players: vec![]
                    }
                )
                .is_err()
            );
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![mine, mine],
                        players: vec![]
                    }
                )
                .is_err()
            );
            lk::aim(&mut e, &[mine, theirs], &[]);
        } else if matches!(e.pending(), Pending::ChooseTargets { .. }) {
            lk::aim(&mut e, &[], &[]);
        }
        lk::settle(&mut e);
        assert_eq!(is_tapped(&e, mine), x != 0);
        assert_eq!(is_tapped(&e, theirs), x != 0);
    }
}
