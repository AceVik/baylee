#[allow(clippy::wildcard_imports)]
use super::*;

fn umbilicus() -> CardIndex {
    card_index("c9132e72-d016-4cb7-8662-a16b7d932bef")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn umbilicus_each_active_player_may_pay_two_life_or_choose_their_own_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[umbilicus(), gray_ogre()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert_eq!(e.pending().asked(), Some(p0));
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, p0);
    assert_eq!(lk::life(&e, p0), 18);
    assert!(on_battlefield(&e, p0, gray_ogre()).is_some());
    let mine = lk::obj(&e, p0, gray_ogre());
    let theirs = lk::obj(&e, p1, llanowar_elves());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert_eq!(e.pending().asked(), Some(p1));
    lk::yes_no(&mut e, false);
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending()
    else {
        panic!("choose a permanent")
    };
    assert_eq!(*player, p1);
    assert!(options.contains(&theirs) && !options.contains(&mine));
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    )
    .unwrap();
    reach_main_phase(&mut e, p1);
    assert_eq!(lk::life(&e, p1), 20);
    assert!(in_hand(&e, p1, llanowar_elves()).is_some());
}
