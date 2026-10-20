//! `cards/enchantments/auras/mv_1/essence_flare.rs`, played.
//!
//! Oracle:
//! - "Enchant creature"
//! - "Enchanted creature gets +2/+0."
//! - "At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn essence_flare() -> CardIndex {
    card_index("9bdc79c9-c8b0-4db1-89d9-0ca7920b6576")
}

#[test]
fn essence_flare_grants_plus_two_plus_zero_and_adds_m0m1_counter_at_upkeep() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[essence_flare()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    lk::cast_at(&mut engine, P0, essence_flare(), &[bears], &[]);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .toughness,
        Some(2)
    );

    lk::to_step(&mut engine, P1, Step::Upkeep);
    lk::settle(&mut engine);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .toughness,
        Some(1)
    );
}
