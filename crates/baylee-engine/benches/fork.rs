//! `Engine::fork_for`, the training and evaluation lookahead fork, early and
//! late in a game. Feature `simulation` (`required-features`), so a plain
//! `cargo bench -p baylee-engine` skips it:
//! `cargo bench -p baylee-engine --features simulation --bench fork`.
//! No timing budget is asserted; compare by hand (`docs/perf-baseline.md`).
#![allow(missing_docs)] // bench target: criterion's generated fns are self-describing

use baylee_core::ids::{CardIndex, PlayerId, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, Engine};
use baylee_engine::state::CardLookup;
use baylee_engine::turn::Phase;
use baylee_engine::zone::ZoneLocation;
use criterion::{Criterion, criterion_group, criterion_main};

#[derive(Clone, Copy)]
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

/// The `basics` bench's preset: two sixty-card decks of lands and
/// creatures, opening hands drawn.
fn preset(seed: u64) -> GamePreset {
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let plains = card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99");
    let cleric = card_index("f4232466-dd6a-49bf-be6c-95905c3ded17");
    let druid = card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1");
    let pool = [forest, plains, cleric, druid];
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card: pool[i % pool.len()],
            print: PrintRef::new(0),
        })
        .collect();
    GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules {
            starting_player: Some(0),
            ..Default::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: (0..2)
            .map(|_| SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities::default(),
                deck: deck.clone(),
                sideboard: vec![],
                commanders: vec![],
                starting_life: None,
                starting_hand: None,
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            })
            .collect(),
    }
}

/// Answers whatever is asked by doing nothing: passes, no attackers or
/// blockers, the first cards of the hand for a discard.
fn step(engine: &mut Engine<RegistryLookup>) {
    let (player, action) = match engine.pending().clone() {
        Pending::Mulligan { player, .. } => (player, PlayerAction::MulliganKeep),
        Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
        Pending::ChooseAttackers { player, .. } => {
            (player, PlayerAction::DeclareAttackers { attackers: vec![] })
        }
        Pending::ChooseBlockers { player, .. } => {
            (player, PlayerAction::DeclareBlockers { blockers: vec![] })
        }
        Pending::DiscardChoice { player, count } => {
            let hand = engine.state().zones.list(ZoneLocation::Hand(player));
            (
                player,
                PlayerAction::ChooseObjects {
                    objects: hand.iter().copied().take(usize::from(count)).collect(),
                },
            )
        }
        other => panic!("the bench walk was asked {other:?}"),
    };
    let actor = engine.decision_actor().unwrap_or(player);
    engine.apply(actor, action).expect("doing nothing is legal");
}

/// Seat 0 holding priority in a first main phase on or after `turn`.
fn main_phase(turn: u32) -> Engine<RegistryLookup> {
    let mut engine = Engine::new(&preset(42), RegistryLookup).unwrap();
    while !(engine.state().turn.number >= turn
        && engine.state().turn.active == PlayerId::new(0)
        && engine.state().turn.phase == Phase::FirstMain
        && matches!(engine.pending(), Pending::Priority { player, .. } if player.get() == 0))
    {
        step(&mut engine);
    }
    engine
}

/// A fork at the first main phase — the engine copied, 113 hidden cards in
/// three pools dealt again, the journal rewritten, statics synced, the
/// projection and the offer rebuilt — and the same late in a game, where the
/// journal it rewrites (linear in its length) is long.
fn bench_fork(c: &mut Criterion) {
    for (name, turn) in [("fork/first_main", 1), ("fork/turn_41", 41)] {
        let engine = main_phase(turn);
        eprintln!(
            "{name}: journal {} entries, arena {} objects",
            engine.journal().len(),
            engine.state().arena.len()
        );
        c.bench_function(name, |b| {
            b.iter(|| {
                engine
                    .fork_for(PlayerId::new(0), DeckKnowledge::Known, 7)
                    .expect("a main phase forks")
            });
        });
    }
}

criterion_group!(fork, bench_fork);
criterion_main!(fork);
