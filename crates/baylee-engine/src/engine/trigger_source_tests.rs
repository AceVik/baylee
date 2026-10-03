//! Event-time source identity through trigger collection, stacking and damage.
use super::synthetic::{self, SyntheticLookup};
use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::prevention::{ChosenSource, Shield, ShieldKind, Shielded};
use baylee_cards_dsl::{
    AbilityDef, Amount, Duration, Effect, Filter, Layer, Modifier, PlayerRel, TargetSpec, Trigger,
};
use baylee_core::ids::{DamageSourceRef, EffectId};

const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
const SOURCE: u32 = 98_151;
const OTHER: u32 = 98_152;
static DAMAGE: &[Effect] = &[Effect::DealDamage {
    amount: Amount::Fixed(3),
    target: TargetSpec::Player(PlayerRel::EachOpponent),
}];
static DIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Dies(&Filter::CREATURE),
    DAMAGE
)];
static ENTERS: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::EntersBattlefield(&Filter::This),
    DAMAGE
)];

fn walk(engine: &mut Engine<SyntheticLookup>, done: impl Fn(&Engine<SyntheticLookup>) -> bool) {
    for _ in 0..200 {
        if done(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    panic!("condition not reached: {:?}", engine.pending());
}
fn game(abilities: &'static [AbilityDef]) -> Engine<SyntheticLookup> {
    let source = Box::leak(Box::new(baylee_cards_dsl::CardDef {
        abilities,
        ..*synthetic::creature(SOURCE, "event source probe", 3, 3, &[])
    }));
    let other = synthetic::creature(OTHER, "event victim probe", 2, 2, &[]);
    let mut engine = Engine::new(
        &synthetic::preset(452, &[SOURCE, OTHER]),
        SyntheticLookup::new(vec![source, other]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    walk(&mut engine, |e| {
        e.state.turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: A, .. })
    });
    engine
}
fn move_to(engine: &mut Engine<SyntheticLookup>, object: ObjectId, zone: ZoneLocation) {
    engine
        .state
        .move_object(object, zone, ZonePosition::Top, Cause::Effect)
        .unwrap();
}
fn shield(engine: &mut Engine<SyntheticLookup>, source: DamageSourceRef) {
    let chosen =
        ChosenSource::new(&engine.state, source, &Filter::Any, B, ObjectId::NO_SOURCE).unwrap();
    engine.state.shields.push(Shield {
        protects: Shielded::Player(B),
        controller: B,
        kind: ShieldKind::NextFrom {
            source: chosen,
            all_but: 0,
            gain_life: false,
            combat_only: false,
        },
    });
}
fn resolve(engine: &mut Engine<SyntheticLookup>) {
    engine.collect_triggers();
    assert!(!engine.state.zones.list(ZoneLocation::Stack).is_empty());
    walk(engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
}

#[test]
fn simultaneous_deaths_use_the_sources_own_departure_in_both_journal_orders() {
    for source_first in [false, true] {
        let mut engine = game(DIES);
        let source = synthetic::permanents(&engine, SOURCE)[0];
        let victim = synthetic::permanents(&engine, OTHER)[0];
        move_to(&mut engine, victim, ZoneLocation::Exile(A));
        move_to(&mut engine, victim, ZoneLocation::Battlefield);
        let was = engine.state.source_identity(source).unwrap();
        assert_ne!(was.version, engine.state.object(victim).unwrap().version);
        shield(&mut engine, was);
        let order = if source_first {
            [source, victim]
        } else {
            [victim, source]
        };
        let snapshots = order.map(|id| engine.state.departure_snapshot(id).unwrap());
        let from = engine.state.journal.last_seq();
        for (id, departure) in order.into_iter().zip(snapshots) {
            engine
                .state
                .move_object_with_departure(
                    id,
                    ZoneLocation::Graveyard(A),
                    ZonePosition::Top,
                    Cause::Effect,
                    Some(departure),
                )
                .unwrap();
        }
        let triggers = crate::trigger::collect(&engine.state, &engine.lookup, from);
        assert_eq!(triggers.len(), 2);
        assert!(
            triggers
                .iter()
                .all(|t| t.source == source && t.source_version == Some(was.version))
        );
        resolve(&mut engine);
        assert_eq!(
            engine.state.players[1].life, 17,
            "one shield prevents exactly one death trigger"
        );
        assert!(engine.state.shields.is_empty());
    }
}

#[test]
fn granted_dies_uses_the_exact_recipient_incarnation_not_the_grantor() {
    let mut engine = game(&[]);
    let recipient = synthetic::permanents(&engine, SOURCE)[0];
    let grantor = synthetic::permanents(&engine, OTHER)[0];
    move_to(&mut engine, grantor, ZoneLocation::Exile(A));
    move_to(&mut engine, grantor, ZoneLocation::Battlefield);
    let was = engine.state.source_identity(recipient).unwrap();
    engine.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(grantor),
        controller: A,
        origin: EffectOrigin::Resolution,
        layer: Layer::Ability,
        timestamp: engine.state.timestamp,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::ObjectIs(recipient, was.version),
        modifier: Modifier::GrantTriggered {
            trigger: Trigger::Dies(&Filter::This),
            effects: DAMAGE,
            target: None,
        },
    });
    shield(&mut engine, was);
    let from = engine.state.journal.last_seq();
    move_to(&mut engine, recipient, ZoneLocation::Graveyard(A));
    let triggers = crate::trigger::collect(&engine.state, &engine.lookup, from);
    assert_eq!(triggers.len(), 1);
    assert_eq!(
        (triggers[0].source, triggers[0].source_version),
        (recipient, Some(was.version))
    );
    assert_eq!(
        triggers[0].ability_index,
        baylee_core::ids::AbilityRef::SYNTHETIC
    );
    resolve(&mut engine);
    assert_eq!(engine.state.players[1].life, 20);
    assert!(engine.state.shields.is_empty());
}

#[test]
fn two_entries_in_one_batch_keep_both_entered_incarnations() {
    let mut engine = game(ENTERS);
    let source = synthetic::permanents(&engine, SOURCE)[0];
    let from = engine.state.journal.last_seq();
    move_to(&mut engine, source, ZoneLocation::Exile(A));
    move_to(&mut engine, source, ZoneLocation::Battlefield);
    let first = engine.state.source_identity(source).unwrap();
    shield(&mut engine, first);
    move_to(&mut engine, source, ZoneLocation::Exile(A));
    move_to(&mut engine, source, ZoneLocation::Battlefield);
    let second = engine.state.source_identity(source).unwrap();
    let triggers = crate::trigger::collect(&engine.state, &engine.lookup, from);
    assert_eq!(triggers.len(), 2);
    assert_eq!(
        triggers
            .iter()
            .map(|t| t.source_version)
            .collect::<Vec<_>>(),
        vec![Some(first.version), Some(second.version)]
    );
    resolve(&mut engine);
    assert_eq!(
        engine.state.players[1].life, 17,
        "only the first entered source was shielded"
    );
    assert!(engine.state.shields.is_empty());
}

#[test]
fn undying_references_the_new_graveyard_card_and_returns_it() {
    let mut engine = game(&[]);
    let source = synthetic::permanents(&engine, SOURCE)[0];
    engine.state.object_mut(source).unwrap().base_mut().keywords =
        baylee_cards_dsl::KeywordSet::UNDYING;
    engine.state.invalidate_projections();
    engine.state.refresh_characteristics();
    let departed = engine.state.source_identity(source).unwrap();
    move_to(&mut engine, source, ZoneLocation::Graveyard(A));
    let graveyard = engine.state.source_identity(source).unwrap();
    engine.collect_triggers();
    let options =
        crate::prevention::source_options(&mut engine.state, &Filter::Any, B, ObjectId::NO_SOURCE);
    assert!(options.contains(&departed));
    assert!(
        options.contains(&graveyard),
        "return it is a real reference to the resulting card"
    );
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    let object = engine.state.object(source).unwrap();
    assert_eq!(object.zone, crate::zone::Zone::Battlefield);
    assert_ne!(object.version, graveyard.version);
    assert_eq!(object.counters.get(baylee_cards_dsl::CounterKind::P1P1), 1);
}

#[test]
fn an_explicit_event_dealer_differs_from_its_own_departed_ability_source() {
    static EVENT_DAMAGE: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::EventObjectDealsDamageEqualToPower {
            target: TargetSpec::Player(PlayerRel::EachOpponent)
        }]
    )];
    for (abilities, life, consumed) in [(DIES, 20, true), (EVENT_DAMAGE, 17, false)] {
        let mut engine = game(abilities);
        let source = synthetic::permanents(&engine, SOURCE)[0];
        let old = engine.state.source_identity(source).unwrap();
        shield(&mut engine, old);
        move_to(&mut engine, source, ZoneLocation::Graveyard(A));
        resolve(&mut engine);
        assert_eq!(engine.state.players[1].life, life);
        assert_eq!(engine.state.shields.is_empty(), consumed);
    }
}
