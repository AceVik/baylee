#[allow(clippy::wildcard_imports)]
use super::*;

fn soul_sculptor() -> CardIndex {
    card_index("d407abd0-69a9-405d-ab69-75b379a45780")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn soul_sculptor_partial_body_can_be_cast_and_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[], &[], &[soul_sculptor()], &[]);
    lk::float(&mut e, p0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut e, p0, soul_sculptor());
    lk::settle(&mut e);
    let creature = lk::obj(&e, p0, soul_sculptor());
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::attack(&mut e, &[creature]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 20 - 1);
}
