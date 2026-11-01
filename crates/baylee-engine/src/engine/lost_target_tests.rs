//! What "this" names once a target is lost (CR 608.2b): "Illegal targets,
//! if any, won't be affected by parts of a resolving spell's effect for
//! which they're illegal."
//!
//! An instruction spelled without a target of its own (`Effect::AddCounter`,
//! `Filter::This`) is about the ability's object target when it has one and
//! about its source when it targets nothing. A target that is only a player
//! leaves it about the source (Jinxed Choker's "target opponent gains
//! control of this artifact and puts a charge counter on it"). An object
//! target that became illegal does not: the counter it would have had goes
//! nowhere, and not onto the source, even while a player chosen beside it
//! is still a legal target.

use super::synthetic::{SyntheticLookup, creature, keep_mulligans, land, preset_both, walk_past};
use super::*;
use baylee_cards_dsl::prelude::*;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);
const MARKER: u32 = 995_200;
const SOLDIER: u32 = 995_201;

static MARK: &[AbilityDef] = &[
    // "Put a +1/+1 counter on each of up to two targets": any target, so a
    // creature and a player may be chosen together.
    activated!(
        Cost::FREE,
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1)
        }],
        targets = Some(TargetReq::up_to(TargetSpec::AnyTarget, 2))
    ),
    // "Target opponent …, and put a +1/+1 counter on this land."
    activated!(
        Cost::FREE,
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1)
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    ),
];

/// Seat 0's Marker and seat 1's Soldier, at seat 0's priority in its first
/// main phase.
fn seated(seed: u64) -> (Engine<SyntheticLookup>, ObjectId, ObjectId) {
    let lookup = SyntheticLookup::new(vec![
        land(MARKER, "Marker", MARK),
        creature(SOLDIER, "Soldier", 1, 1, &[]),
    ]);
    let mut engine =
        Engine::new(&preset_both(seed, &[MARKER], &[SOLDIER]), lookup).expect("the game starts");
    keep_mulligans(&mut engine);
    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == ME
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == ME)
        {
            let of = |seat: PlayerId| {
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .copied()
                    .find(|id| {
                        engine
                            .state()
                            .object(*id)
                            .is_some_and(|o| o.controller == seat)
                    })
                    .expect("one permanent a side")
            };
            let (marker, soldier) = (of(ME), of(THEM));
            return (engine, marker, soldier);
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    panic!("never reached seat 0's main phase");
}

/// Activates the Marker's `ability`, aiming it at `objects` and `players`.
fn aim(
    engine: &mut Engine<SyntheticLookup>,
    marker: ObjectId,
    ability: u32,
    objects: Vec<ObjectId>,
    players: Vec<PlayerId>,
) {
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: marker,
                ability_index: ability,
            },
        )
        .expect("the ability activates");
    engine
        .apply(ME, PlayerAction::ChooseTargets { objects, players })
        .expect("the targets are legal");
}

fn resolve(engine: &mut Engine<SyntheticLookup>) {
    while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
}

fn counters(engine: &Engine<SyntheticLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(CounterKind::P1P1))
}

/// The Soldier and seat 1 are chosen; the Soldier leaves before the ability
/// resolves. Seat 1 is still a legal target, so the ability resolves, and
/// does nothing: the Soldier's counter does not land on the Marker.
#[test]
fn an_object_target_that_became_illegal_is_not_replaced_by_the_source() {
    let (mut engine, marker, soldier) = seated(5);
    aim(&mut engine, marker, 0, vec![soldier], vec![THEM]);
    engine
        .dev_state_mut(ME)
        .expect("the harness may set boards up")
        .move_object(
            soldier,
            ZoneLocation::Graveyard(THEM),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("the Soldier leaves");
    engine.refresh_offer();
    resolve(&mut engine);
    assert_eq!(
        counters(&engine, marker),
        0,
        "the counter went to the source in the illegal target's place"
    );
}

/// The two halves the rule sits between: with both targets legal the
/// counter goes on the Soldier and not the Marker, and with a player-only
/// target ("target opponent") it goes on the Marker, the source.
#[test]
fn a_counter_goes_on_the_object_target_or_with_none_on_the_source() {
    let (mut engine, marker, soldier) = seated(6);
    aim(&mut engine, marker, 0, vec![soldier], vec![THEM]);
    resolve(&mut engine);
    assert_eq!(counters(&engine, soldier), 1);
    assert_eq!(counters(&engine, marker), 0);

    let (mut engine, marker, soldier) = seated(7);
    aim(&mut engine, marker, 1, vec![], vec![THEM]);
    resolve(&mut engine);
    assert_eq!(
        counters(&engine, marker),
        1,
        "a player target leaves the source"
    );
    assert_eq!(counters(&engine, soldier), 0);
}
