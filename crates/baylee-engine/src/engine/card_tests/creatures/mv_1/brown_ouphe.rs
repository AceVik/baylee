//! `cards/creatures/mv_1/brown_ouphe.rs`, played.
//!
//! Oracle:
//! - "{1}{G}, {T}: Counter target activated ability from an artifact source. (Mana abilities can't be targeted.)"

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5b42e91a-e3bd-4f69-9abf-ed6273715fff"
fn brown_ouphe() -> CardIndex {
    card_index("5b42e91a-e3bd-4f69-9abf-ed6273715fff")
}

#[test]
fn brown_ouphe_counters_artifact_activated_ability() {
    let mut engine = lk::game(
        SEED,
        &[brown_ouphe()],
        &[baylee_core::generated::index::MIND_STONE],
        &[],
        &[],
    );
    let before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P1, &[(ManaColor::Colorless, 1)]);
    activate(
        &mut engine,
        P1,
        baylee_core::generated::index::MIND_STONE,
        1,
    );
    let ability = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    activate(&mut engine, P0, brown_ouphe(), 0);
    aim_at(&mut engine, P0, ability);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        before,
        "artifact's draw activation was countered by Brown Ouphe"
    );
}
