#[allow(clippy::wildcard_imports)]
use super::*;

fn sanctum_guardian() -> CardIndex {
    card_index("eb7fe094-2237-40c8-b831-93498c93b768")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn sanctum_guardian_partial_body_can_be_cast_and_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[], &[], &[sanctum_guardian()], &[]);
    lk::float(&mut e, p0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut e, p0, sanctum_guardian());
    lk::settle(&mut e);
    let creature = lk::obj(&e, p0, sanctum_guardian());
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::attack(&mut e, &[creature]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 20 - 1);
}
