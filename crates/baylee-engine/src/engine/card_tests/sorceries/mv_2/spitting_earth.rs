//! `cards/sorceries/mv_2/spitting_earth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::event::DamageTarget;
use baylee_core::generated::index;

fn spitting_earth() -> CardIndex {
    card_index("0c3bf4e1-d91e-4dd3-a800-e40971222c71")
}

#[test]
fn spitting_earth_counts_only_its_controllers_mountains() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain()],
        &[mountain(), index::GRAY_OGRE],
        &[spitting_earth()],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    let target = lk::obj(&e, lk::P1, index::GRAY_OGRE);
    lk::cast_at(&mut e, lk::P0, spitting_earth(), &[target], &[]);
    assert!(in_graveyard(&e, lk::P1, index::GRAY_OGRE).is_some());
    assert!(e.state().journal.entries().iter().any(|entry| {
        matches!(entry.event, GameEvent::DamageDealt {
            target: DamageTarget::Object(object),
            amount: 2,
            ..
        } if object == target)
    }));
}
