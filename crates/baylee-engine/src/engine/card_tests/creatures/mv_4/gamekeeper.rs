//! `cards/creatures/mv_4/gamekeeper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gamekeeper() -> CardIndex {
    card_index("c6921cfe-48f9-4378-b145-d9b430c2aaf6")
}

#[test]
fn gamekeeper_does_not_use_the_wrong_reveal_destination() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[gamekeeper()], &[], &[], &[]);
    let keeper = lk::obj(&engine, lk::P0, gamekeeper());
    let library = engine
        .state()
        .zones
        .list(ZoneLocation::Library(lk::P0))
        .clone();
    kill(&mut engine, keeper);
    assert!(in_graveyard(&engine, lk::P0, gamekeeper()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(lk::P0)),
        &library
    );
}

#[test]
fn creature_body_remains_playable() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[gamekeeper()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Green, 6)]);
    cast_with_floating(&mut engine, lk::P0, gamekeeper());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, gamekeeper()).is_some());
}
