//! `cards/creatures/mv_3/ophiomancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn ophiomancer_makes_a_snake_at_each_upkeep_only_when_you_control_none() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::OPHIOMANCER, swamp(), swamp(), swamp()])
        .hand(0, &[hero_s_downfall()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let first = only_snake(&e);
    assert_snake(&e, first);
    cast_from_hand(&mut e, P0, hero_s_downfall());
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![first],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state().object(first).is_none(),
        "a destroyed token ceases to exist"
    );
    assert!(tokens_of(&e, P0).is_empty());

    // The opponent's upkeep also triggers Ophiomancer; the token still belongs
    // to Ophiomancer's controller, not to the active player.
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    let second = only_snake(&e);
    assert_ne!(first, second);
    assert_snake(&e, second);

    // The next own upkeep sees the existing Snake and creates no second one.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(only_snake(&e), second);
}

fn only_snake(e: &Engine<RegistryLookup>) -> ObjectId {
    let tokens = tokens_of(e, P0);
    assert_eq!(tokens.len(), 1, "exactly one Snake token");
    tokens[0]
}

fn assert_snake(e: &Engine<RegistryLookup>, id: ObjectId) {
    let object = e.state().object(id).unwrap();
    assert_eq!(object.controller, P0);
    let characteristics = object.characteristics();
    assert_eq!(pt(e, id), (1, 1));
    assert_eq!(
        characteristics.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black])
    );
    assert!(characteristics.types.contains(TypeSet::CREATURE));
    assert!(characteristics.subtypes.contains(subtypes::creature::SNAKE));
    assert!(characteristics.keywords.contains(KeywordSet::DEATHTOUCH));
}
