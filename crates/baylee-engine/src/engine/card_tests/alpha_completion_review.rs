//! Independent behavioral review of the remaining Alpha clauses.

#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

mod artifacts;
mod creatures_batch_b;
mod enchantments_batch_b;
mod instants;
mod sorceries;
mod sorceries_batch_b;

fn fork() -> CardIndex {
    card_index("50c53ae0-51ba-4046-ac74-87c65e688032")
}

fn drain_life() -> CardIndex {
    card_index("e75ba79f-4cc2-4ede-8641-559ab94e7e36")
}

fn top(engine: &Engine<RegistryLookup>) -> ObjectId {
    *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("a spell")
}

fn aim(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("target choice expected: {:?}", engine.pending());
    };
    engine
        .apply(player, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

fn announce_drain(engine: &mut Engine<RegistryLookup>, caster: PlayerId, x: u32, target: ObjectId) {
    cast_from_hand(engine, caster, drain_life());
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(min <= x && x <= max);
                engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
            }
            Pending::ChooseTargets { .. } => aim(engine, vec![target], vec![]),
            Pending::Priority { .. } => return,
            pending => panic!("unexpected Drain Life announcement: {pending:?}"),
        }
    }
    panic!("announcement never finished");
}

fn power_sink() -> CardIndex {
    card_index("39412e6d-2837-4729-abf9-e64a5ba87e40")
}

fn sink(engine: &mut Engine<RegistryLookup>, caster: PlayerId, target: ObjectId, x: u32) {
    cast_from_hand(engine, caster, power_sink());
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseNumber { player, .. } => {
                engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
            }
            Pending::ChooseTargets { .. } => aim(engine, vec![target], vec![]),
            Pending::Priority { .. } => return,
            pending => panic!("unexpected Power Sink announcement: {pending:?}"),
        }
    }
    panic!("announcement never finished");
}

fn clockwork_beast() -> CardIndex {
    card_index("eb97c8db-ac6c-476c-b14d-87785e9c82f0")
}

fn beast_counters(engine: &Engine<RegistryLookup>, beast: ObjectId) -> u16 {
    engine
        .state()
        .object(beast)
        .unwrap()
        .counters
        .get(CounterKind::Plus {
            power: 1,
            toughness: 0,
        })
}

mod damage_order_review;

mod source_choice_review;
