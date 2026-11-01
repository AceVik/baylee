//! Reviewed jackal pup behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

use baylee_core::generated::index;
fn jackal_pup() -> CardIndex {
    card_index("993e41e6-0643-4a1f-8ada-b7a4aae486d9")
}

#[test]
fn jackal_pup_repeats_the_actual_damage_even_when_lethal() {
    for (spell, n) in [(index::LIGHTNING_BOLT, 3), (index::SHOCK, 2)] {
        let mut e = lk::game(SEED, &[jackal_pup()], &[], &[spell], &[]);
        let pup = lk::obj(&e, lk::P0, jackal_pup());
        lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
        lk::cast_at(&mut e, lk::P0, spell, &[pup], &[]);
        assert!(in_graveyard(&e, lk::P0, jackal_pup()).is_some());
        assert_eq!(lk::life(&e, lk::P0), 20 - n);
        assert_eq!(lk::life(&e, lk::P1), 20);
    }
}

#[test]
fn jackal_pup_does_not_repeat_damage_to_another_creature() {
    let mut e = lk::game(
        SEED,
        &[jackal_pup(), index::GRIZZLY_BEARS],
        &[],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&e, lk::P0, index::GRIZZLY_BEARS);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::LIGHTNING_BOLT, &[bears], &[]);
    assert_eq!(lk::life(&e, lk::P0), 20);
}
