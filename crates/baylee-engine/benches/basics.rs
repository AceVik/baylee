//! Baseline performance benchmarks (CI regression budgets derive from
//! these). Run: `cargo bench -p baylee-engine`.
#![allow(missing_docs)] // bench target: criterion's generated fns are self-describing

use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::{CardLookup, GameState};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

struct RegistryLookup;

impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

/// A mid-game-ish 2-player preset: mixed lands and creatures.
fn preset(seed: u64) -> GamePreset {
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let plains = card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99");
    let cleric = card_index("f4232466-dd6a-49bf-be6c-95905c3ded17");
    let druid = card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1");
    let pool = [forest, plains, cleric, druid];
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| DeckEntry {
            card: pool[i % pool.len()],
            print: PrintRef::new(0),
        })
        .collect();
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: (0..2)
            .map(|_| SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                deck: deck.clone(),
                sideboard: vec![],
                starting_life: None,
                starting_hand: None,
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            })
            .collect(),
    }
}

fn engine_started() -> Engine<RegistryLookup> {
    let mut engine = Engine::new(&preset(42), RegistryLookup).unwrap();
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("mulligan expected");
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

fn bench_setup(c: &mut Criterion) {
    let p = preset(42);
    c.bench_function("setup/from_preset", |b| {
        b.iter(|| GameState::from_preset(&p, &RegistryLookup).unwrap());
    });
}

fn bench_clone(c: &mut Criterion) {
    let state = GameState::from_preset(&preset(42), &RegistryLookup).unwrap();
    c.bench_function("state/clone", |b| b.iter(|| state.clone()));
}

fn bench_snapshot_hash(c: &mut Criterion) {
    let state = GameState::from_preset(&preset(42), &RegistryLookup).unwrap();
    c.bench_function("state/snapshot_hash", |b| {
        b.iter(|| state.snapshot_hash());
    });
}

fn bench_priority_pass(c: &mut Criterion) {
    c.bench_function("engine/priority_pass_x4", |b| {
        b.iter_batched(
            engine_started,
            |mut engine| {
                for _ in 0..4 {
                    let Pending::Priority { player, .. } = engine.pending().clone() else {
                        break;
                    };
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                engine
            },
            BatchSize::SmallInput,
        );
    });
}

/// A wide board under many anthems: the shape the layer system is actually
/// expensive on.
///
/// The old projection sorted every effect from scratch for every object it
/// touched (`objects · layers · effects²`); the plan is now built once per
/// refresh (`layers · effects² + objects · effects`). Nothing else in this
/// file grows with the number of continuous effects, so without this bench
/// the difference is invisible.
fn wide_board(anthems: usize) -> GameState {
    use baylee_cards_dsl::static_ability::{Duration, Layer, Modifier};
    use baylee_core::ids::EffectId;
    use baylee_engine::effects::{ContinuousEffect, EffectFilter};
    use baylee_engine::zone::{ZoneLocation, ZonePosition};

    static CREATURES: baylee_cards_dsl::Filter =
        baylee_cards_dsl::Filter::HasType(baylee_core::types::TypeSet::CREATURE);

    let mut state = GameState::from_preset(&preset(42), &RegistryLookup).unwrap();
    // Put every library card of seat 0 onto the battlefield: a board far
    // wider than a real one, so the per-object cost dominates the noise.
    let seat = baylee_core::ids::PlayerId::new(0);
    let library = state.zones.list(ZoneLocation::Library(seat)).clone();
    for id in library {
        let _ = state.move_object(
            id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            baylee_engine::event::Cause::Setup,
        );
    }
    for i in 0..anthems {
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: seat,
            layer: Layer::PtModify,
            timestamp: i as u64,
            duration: Duration::Indefinitely,
            filter: EffectFilter::Dsl(&CREATURES),
            modifier: Modifier::ModifyPT(1, 1),
        });
    }
    state
}

fn bench_layers(c: &mut Criterion) {
    for anthems in [1usize, 8, 32] {
        let state = wide_board(anthems);
        c.bench_function(&format!("layers/refresh_x{anthems}"), |b| {
            b.iter_batched(
                || state.clone(),
                |mut s| {
                    // Force a full recompute rather than a cache hit.
                    s.effects.generation += 1;
                    s.refresh_characteristics();
                    s
                },
                BatchSize::SmallInput,
            );
        });
    }
}

// The baseline benchmark group (CI regression budgets derive from these).
criterion_group!(
    basics,
    bench_setup,
    bench_clone,
    bench_snapshot_hash,
    bench_priority_pass,
    bench_layers
);
criterion_main!(basics);
