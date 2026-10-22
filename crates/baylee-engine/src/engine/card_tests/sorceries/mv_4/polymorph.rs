//! `cards/sorceries/mv_4/polymorph.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "039115f2-6322-4674-bc64-f21883ed375a"
fn polymorph() -> CardIndex {
    card_index("039115f2-6322-4674-bc64-f21883ed375a")
}

#[test]
fn polymorph_destroys_target_creature_without_regeneration() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[polymorph()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, polymorph(), &[bears], &[]);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
}
