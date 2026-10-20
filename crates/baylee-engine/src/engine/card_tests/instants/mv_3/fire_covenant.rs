//! `cards/instants/mv_3/fire_covenant.rs`, played.
//!
//! Oracle:
//! - "As an additional cost to cast this spell, pay X life."
//! - "Fire Covenant deals X damage divided as you choose among any number of target creatures."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fire_covenant() -> CardIndex {
    card_index("025939a0-424a-41bf-8fc9-2ef9ea5485f7")
}

#[test]
fn fire_covenant_pays_life_cost_and_damages_target_creature() {
    let mut engine = lk::game(SEED, &[], &[index::LLANOWAR_ELVES], &[fire_covenant()], &[]);
    let elves = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    lk::cast_at(&mut engine, P0, fire_covenant(), &[elves], &[]);

    assert_eq!(lk::life(&engine, P0), 19);
    assert_eq!(lk::zone_of(&engine, elves), Zone::Graveyard);
}
