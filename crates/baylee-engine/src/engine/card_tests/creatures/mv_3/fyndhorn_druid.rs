//! `cards/creatures/mv_3/fyndhorn_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fyndhorn_druid() -> CardIndex {
    card_index("98e51f1a-ed6e-4286-a41b-e4656d08c934")
}

fn bolt_the_druid(engine: &mut Engine<RegistryLookup>, druid: ObjectId) {
    lk::float(engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(engine, P0, lightning_bolt(), &[druid], &[]);
    assert!(in_graveyard(engine, P0, fyndhorn_druid()).is_some());
}

fn next_main(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(engine, |engine| {
        engine.state().turn.active == player && engine.state().turn.phase == Phase::FirstMain
    });
}

#[test]
fn fyndhorn_druid_gains_four_life_when_it_dies_after_being_blocked() {
    let mut engine = lk::game(SEED, &[fyndhorn_druid()], &[index::GRIZZLY_BEARS], &[], &[]);
    let druid = lk::obj(&engine, P0, fyndhorn_druid());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::attack(&mut engine, &[druid]);
    lk::block(&mut engine, &[(bears, druid)]);
    pass_until(&mut engine, |engine| {
        in_graveyard(engine, P0, fyndhorn_druid()).is_some()
    });
    assert!(
        !stack_is_empty(&engine),
        "the dies trigger goes on the stack: {:?}",
        engine.pending()
    );
    lk::through_combat(&mut engine);

    assert!(in_graveyard(&engine, P0, fyndhorn_druid()).is_some());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(lk::life(&engine, P0), 24);
    assert_eq!(lk::life(&engine, P1), 20);
}

#[test]
fn fyndhorn_druid_gains_nothing_when_it_dies_after_an_unblocked_attack() {
    let mut engine = lk::game(SEED, &[fyndhorn_druid()], &[], &[lightning_bolt()], &[]);
    let druid = lk::obj(&engine, P0, fyndhorn_druid());

    lk::attack(&mut engine, &[druid]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 18);

    bolt_the_druid(&mut engine, druid);
    assert_eq!(lk::life(&engine, P0), 20);
}

#[test]
fn fyndhorn_druid_gains_nothing_when_it_dies_after_blocking() {
    let mut engine = lk::game(SEED, &[fyndhorn_druid()], &[index::GRIZZLY_BEARS], &[], &[]);
    let druid = lk::obj(&engine, P0, fyndhorn_druid());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    next_main(&mut engine, P1);
    lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[(druid, bears)]);
    lk::through_combat(&mut engine);

    assert!(in_graveyard(&engine, P0, fyndhorn_druid()).is_some());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(lk::life(&engine, P0), 20);
}

#[test]
fn fyndhorn_druid_forgets_earlier_turns_blocks() {
    let mut engine = lk::game(
        SEED,
        &[fyndhorn_druid()],
        &[index::GIANT_TORTOISE],
        &[lightning_bolt()],
        &[],
    );
    let druid = lk::obj(&engine, P0, fyndhorn_druid());
    let tortoise = lk::obj(&engine, P1, index::GIANT_TORTOISE);

    lk::attack(&mut engine, &[druid]);
    lk::block(&mut engine, &[(tortoise, druid)]);
    lk::through_combat(&mut engine);
    assert!(on_battlefield(&engine, P0, fyndhorn_druid()).is_some());
    assert_eq!(lk::life(&engine, P0), 20);

    next_main(&mut engine, P0);
    bolt_the_druid(&mut engine, druid);
    assert_eq!(lk::life(&engine, P0), 20);
}
