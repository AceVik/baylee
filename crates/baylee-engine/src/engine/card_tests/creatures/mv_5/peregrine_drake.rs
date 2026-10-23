#[allow(clippy::wildcard_imports)]
use super::*;

fn peregrine_drake() -> CardIndex {
    card_index("0bd67481-6bd9-48d6-92bd-8933b5ea1eae")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn peregrine_drake_untaps_chosen_lands_including_opponents_but_not_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for choose in [false, true] {
        let mut e = lk::game(
            SEED,
            &[island(), grizzly_bears()],
            &[forest()],
            &[peregrine_drake()],
            &[],
        );
        let mine = lk::obj(&e, p0, island());
        let theirs = lk::obj(&e, p1, forest());
        let bear = lk::obj(&e, p0, grizzly_bears());
        for id in [mine, theirs, bear] {
            e.dev_state_mut(p0).unwrap().set_tapped(id, true);
        }
        lk::float(&mut e, p0, &[(ManaColor::Blue, 5)]);
        cast_with_floating(&mut e, p0, peregrine_drake());
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending()
        else {
            unreachable!()
        };
        assert_eq!((*min, *max), (0, 5));
        assert!(options.contains(&mine) && options.contains(&theirs));
        assert!(!options.contains(&bear));
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: if choose { vec![mine, theirs] } else { vec![] },
            },
        )
        .unwrap();
        lk::settle(&mut e);
        assert_eq!(is_tapped(&e, mine), !choose);
        assert_eq!(is_tapped(&e, theirs), !choose);
        assert!(is_tapped(&e, bear));
    }
}
