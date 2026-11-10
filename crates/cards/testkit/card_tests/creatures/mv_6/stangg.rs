//! `cards/creatures/mv_6/stangg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn stangg() -> CardIndex {
    card_index("739eac91-3029-4ce7-9885-0af3ddea472e")
}

fn murder() -> CardIndex {
    card_index("938b4e2c-88d9-4637-bc00-e228920c9a78")
}

/// Stangg in play with its Twin, ready to be murdered. Returns the engine,
/// Stangg and the Twin token.
fn stangg_with_twin(seed: u64) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[stangg(), murder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, stangg());
    pass_until(&mut engine, stack_is_empty);

    let stangg_id = on_battlefield(&engine, p0, stangg()).expect("Stangg on battlefield");
    let twin_id = engine
        .state()
        .battlefield_seen()
        .find(|&id| engine.state().object(id).is_some_and(|o| o.card.is_none()))
        .expect("Stangg Twin token created");
    (engine, stangg_id, twin_id)
}

/// Murder (instant) cast at `target` off floating black mana.
fn murder_at(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    let p0 = PlayerId::new(0);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 3);
    engine.refresh_offer();
    cast_with_floating(engine, p0, murder());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("Murder targets the creature");
    pass_until(engine, stack_is_empty);
}

/// Stangg (739eac91-3029-4ce7-9885-0af3ddea472e) — "When Stangg enters, create
/// Stangg Twin, a legendary 3/4 red and green Human Warrior creature token."
#[test]
fn stangg_enters_with_a_legendary_three_four_twin() {
    let (engine, stangg_id, twin_id) = stangg_with_twin(SEED);
    assert_eq!(pt(&engine, stangg_id), (3, 4));
    assert_eq!(pt(&engine, twin_id), (3, 4), "Stangg Twin is a 3/4");
    let twin = engine.state().object(twin_id).unwrap().characteristics();
    assert!(
        twin.supertypes.contains(SupertypeSet::LEGENDARY),
        "the token is legendary"
    );
    assert!(twin.types.contains(TypeSet::CREATURE));
}

/// Stangg — "Exile that token when Stangg leaves the battlefield."
#[test]
fn stangg_exiles_twin_when_stangg_leaves() {
    let (mut engine, stangg_id, twin_id) = stangg_with_twin(SEED);
    murder_at(&mut engine, stangg_id);

    assert!(
        in_graveyard(&engine, PlayerId::new(0), stangg()).is_some(),
        "Stangg died"
    );
    assert!(
        !engine.state().battlefield_seen().any(|id| id == twin_id),
        "Stangg Twin is exiled when Stangg leaves the battlefield"
    );
}

/// Stangg — "Sacrifice Stangg when that token leaves the battlefield."
#[test]
fn stangg_is_sacrificed_when_twin_leaves() {
    let (mut engine, _stangg_id, twin_id) = stangg_with_twin(SEED + 1);
    murder_at(&mut engine, twin_id);

    assert!(
        on_battlefield(&engine, PlayerId::new(0), stangg()).is_none(),
        "Stangg is sacrificed when Stangg Twin leaves the battlefield"
    );
    assert!(
        in_graveyard(&engine, PlayerId::new(0), stangg()).is_some(),
        "sacrificed Stangg is in the graveyard"
    );
}
