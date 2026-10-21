//! `cards/creatures/mv_1/brown_ouphe.rs`, played.
//!
//! Oracle:
//! - "{1}{G}, {T}: Counter target activated ability from an artifact source. (Mana abilities can't be targeted.)"

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5b42e91a-e3bd-4f69-9abf-ed6273715fff"
fn brown_ouphe() -> CardIndex {
    card_index("5b42e91a-e3bd-4f69-9abf-ed6273715fff")
}

fn stack(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Stack).clone()
}

/// Two activated abilities on the stack: P0's Prodigal Sorcerer (a
/// creature) pinging P1, and P1's Mind Stone (an artifact) drawing. Brown
/// Ouphe may aim only at the artifact's, and countering it leaves the other
/// to resolve.
#[test]
fn brown_ouphe_counters_an_artifact_ability_and_only_an_artifact_ability() {
    let mut engine = lk::game(
        SEED,
        &[brown_ouphe(), index::PRODIGAL_SORCERER],
        &[index::MIND_STONE],
        &[],
        &[],
    );
    let p1_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    activate(&mut engine, P0, index::PRODIGAL_SORCERER, 0);
    lk::aim(&mut engine, &[], &[P1]);
    let ping = stack(&engine)[0];
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P1, &[(ManaColor::Colorless, 1)]);
    activate(&mut engine, P1, index::MIND_STONE, 1);
    let draw = *stack(&engine)
        .iter()
        .find(|id| **id != ping)
        .expect("Mind Stone's draw is on the stack");
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    activate(&mut engine, P0, brown_ouphe(), 0);
    let options = aim_at(&mut engine, P0, draw);
    assert!(
        !options.contains(&ping),
        "an activated ability from a creature source is not a legal target"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand,
        "the artifact's draw ability was countered"
    );
    assert_eq!(
        lk::life(&engine, P1),
        19,
        "the creature's ability was left alone and resolved"
    );
}
