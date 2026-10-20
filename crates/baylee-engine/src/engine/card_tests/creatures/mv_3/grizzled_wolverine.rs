//! `cards/creatures/mv_3/grizzled_wolverine.rs`, played.
//!
//! Oracle:
//! - "{R}: This creature gets +2/+0 until end of turn. Activate only during the declare blockers step, only if at least one creature is blocking this creature, and only once each turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzled_wolverine() -> CardIndex {
    card_index("da4ed552-a85a-47e1-ab67-5c93a9a20048")
}

#[test]
fn grizzled_wolverine_pumps_once_during_declare_blockers_when_blocked() {
    let mut engine = lk::game(
        SEED,
        &[grizzled_wolverine()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let wolverine = lk::obj(&engine, P0, grizzled_wolverine());
    let blocker = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);

    assert!(
        !lk::offered(&engine, grizzled_wolverine(), P0, 0),
        "cannot activate Grizzled Wolverine during main phase"
    );

    let _ = lk::attack(&mut engine, &[wolverine]);
    lk::block(&mut engine, &[(blocker, wolverine)]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    assert!(lk::offered(&engine, grizzled_wolverine(), P0, 0));
    lk::use_ability(&mut engine, P0, grizzled_wolverine(), 0, &[], &[]);
    assert_eq!(power_of(&engine, wolverine), Some(4));
    assert!(
        !lk::offered(&engine, grizzled_wolverine(), P0, 0),
        "Grizzled Wolverine can only be activated once each turn"
    );
}
