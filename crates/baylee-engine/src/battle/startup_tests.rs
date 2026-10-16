//! Preset initialization keeps its mulligan window and ordinary arrival journal.
use super::*;
use crate::choice::{Pending, PlayerAction};
use crate::engine::Engine;
use crate::event::GameEvent;
use crate::state::CardLookup;
use baylee_cards_dsl::{AbilityDef, CardDef, CounterKind, FaceDef, Filter, ReplacementRule};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{DeckEntry, FormatId, GamePreset, HouseRules, SeatController, SeatSpec};

const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
const C: PlayerId = PlayerId::new(2);
static BATTLE: CardDef = CardDef {
    index: CardIndex::new(0),
    faces: &[FaceDef {
        name: "Startup Siege",
        types: TypeSet::BATTLE,
        subtypes: &[baylee_core::generated::subtypes::battle::SIEGE],
        defense: Some(4),
        ..FaceDef::DEFAULT
    }],
    ..CardDef::DEFAULT
};
static DOUBLER: CardDef = CardDef {
    index: CardIndex::new(1),
    faces: &[FaceDef {
        name: "Startup counter doubler",
        types: TypeSet::ENCHANTMENT,
        abilities: &[AbilityDef::Replacement(
            ReplacementRule::DoubleCounterPlacement {
                object_filter: &Filter::Any,
            },
        )],
        ..FaceDef::DEFAULT
    }],
    ..CardDef::DEFAULT
};
static LAND: CardDef = CardDef {
    index: CardIndex::new(2),
    faces: &[FaceDef {
        name: "Startup land",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    ..CardDef::DEFAULT
};
struct Lookup;
impl CardLookup for Lookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        match index.get() {
            0 => Some(&BATTLE),
            1 => Some(&DOUBLER),
            2 => Some(&LAND),
            _ => None,
        }
    }
}
fn entry(index: u32) -> DeckEntry {
    DeckEntry {
        card: CardIndex::new(index),
        print: PrintRef::new(0),
        rule_print: None,
        choices: Default::default(),
    }
}
fn preset(seats: usize) -> GamePreset {
    GamePreset {
        rules: Default::default(),
        format: FormatId::Freeform,
        seed: 4,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![baylee_core::preset::PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: baylee_core::preset::Finish::Normal,
        }],
        seats: (0..seats)
            .map(|seat| SeatSpec {
                controller: SeatController::Open,
                capabilities: Default::default(),
                deck: vec![entry(2); 60],
                sideboard: vec![],
                commanders: vec![],
                starting_life: Some(20),
                starting_hand: None,
                starting_battlefield: if seat == 0 {
                    vec![entry(0), entry(1)]
                } else {
                    vec![]
                },
                emblems: vec![],
                team: None,
            })
            .collect(),
    }
}
fn siege(engine: &Engine<Lookup>) -> ObjectId {
    engine
        .state()
        .battlefield_seen()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .unwrap()
                .base
                .types
                .contains(TypeSet::BATTLE)
        })
        .unwrap()
}
fn keep_all(engine: &mut Engine<Lookup>, seats: usize) {
    for _ in 0..seats {
        let Pending::Mulligan { player, .. } = *engine.pending() else {
            panic!("mulligan must precede entry questions")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
}
#[test]
fn preset_battle_forced_setup_precedes_mulligans_without_reentering() {
    let mut engine = Engine::new(&preset(2), Lookup).unwrap();
    let id = siege(&engine);
    let reference = engine.state().source_identity(id).unwrap();
    assert_eq!(protector(engine.state(), id), Some(B));
    assert_eq!(
        engine
            .state()
            .object(id)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        8
    );
    assert!(matches!(engine.pending(), Pending::Mulligan { .. }));
    keep_all(&mut engine, 2);
    assert_eq!(engine.state().source_identity(id), Some(reference));
    assert_eq!(
        engine
            .state()
            .object(id)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        8
    );
    assert_eq!(engine.state().journal.entries().iter().filter(|event| matches!(event.event, GameEvent::ZoneChanged { object, to: Zone::Battlefield, .. } if object == id)).count(), 1);
}
#[test]
fn preset_battle_waits_until_all_mulligans_for_multiplayer_protector() {
    let mut engine = Engine::new(&preset(3), Lookup).unwrap();
    let id = siege(&engine);
    assert_eq!(designated_protector(engine.state(), id), None);
    assert_eq!(
        engine
            .state()
            .object(id)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        8
    );
    keep_all(&mut engine, 3);
    assert!(
        matches!(engine.pending(), Pending::ChoosePlayer { player: A, options } if options == &[B, C])
    );
    engine.apply(A, PlayerAction::ChoosePlayer(C)).unwrap();
    assert_eq!(protector(engine.state(), id), Some(C));
    assert_eq!(
        engine
            .state()
            .object(id)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        8
    );
}
