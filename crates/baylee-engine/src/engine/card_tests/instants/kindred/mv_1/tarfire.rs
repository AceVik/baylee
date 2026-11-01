//! `cards/instants/kindred/mv_1/tarfire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn tarfire() -> CardIndex {
    card_index("f07b821c-bbc1-4def-b009-6c03689478d0")
}

#[test]
fn tarfire_deals_two_damage_to_any_target() {
    let mut e = lk::game(SEED, &[], &[grizzly_bears()], &[tarfire()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    let target = lk::obj(&e, lk::P1, grizzly_bears());
    cast_with_floating(&mut e, lk::P0, tarfire());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("Tarfire targets: {:?}", e.pending());
    };
    assert!(options.contains(&target));
    assert!(player_options.contains(&lk::P0) && player_options.contains(&lk::P1));
    lk::aim(&mut e, &[target], &[]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P1, grizzly_bears()).is_some());
    assert!(in_graveyard(&e, lk::P0, tarfire()).is_some());
    assert_eq!(lk::life(&e, lk::P1), 20);
}
