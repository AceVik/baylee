//! `cards/sorceries/mv_5/fumarole.rs`, played.
//!
//! Oracle:
//! - "As an additional cost to cast this spell, pay 3 life."
//! - "Destroy target creature and target land."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fumarole() -> CardIndex {
    card_index("a9e33138-e1c0-4b69-b868-1ce8493c073d")
}

#[test]
fn fumarole_pays_three_life_additional_cost_and_destroys_target_creature() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[fumarole()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    lk::cast_at(&mut engine, P0, fumarole(), &[bears], &[]);

    assert_eq!(lk::life(&engine, P0), 17);
    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
}
