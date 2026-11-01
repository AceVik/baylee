//! Reproduce Wartime Protestors + Doubling Season with a large Ally arrival.
//! `cargo run -p baylee-gamehost --example rally_stress --profile ci-release -- 500`
//! An optional second argument saves `(GameStatic, PlayerView)` for client profiling.
//! Load it in a dev-control client with `BAYLEE_DEV_SNAPSHOT=/path/to/snapshot.json`.
#![allow(missing_docs)]

use baylee_core::ids::{CardIndex, PlayerId, PrintRef};
use baylee_core::mana::ManaColor;
use baylee_core::preset::{DeckEntry, FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
use baylee_core::types::TypeSet;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::object::{ObjectKind, Status};
use baylee_engine::turn::Phase;
use baylee_engine::zone::ZoneLocation;
use baylee_gamehost::{RegistryLookup, SeatContext};
use std::time::Instant;

fn card(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).expect("fixture card")
}

fn preset() -> GamePreset {
    let cards = [
        card("Forest"),
        card("Wartime Protestors"),
        card("Doubling Season"),
        card("Aang and Katara"),
    ];
    let entry = |i: usize| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: cards[i],
        print: PrintRef::new(i as u16),
    };
    GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 37,
        house_rules: HouseRules {
            starting_player: Some(0),
            ..Default::default()
        },
        modifiers: vec![],
        prints: cards
            .iter()
            .copied()
            .map(baylee_cards::decks::reference_print)
            .collect(),
        seats: (0..2)
            .map(|seat| SeatSpec {
                controller: SeatController::Open,
                capabilities: baylee_core::preset::SeatCapabilities {
                    dev_commands: true,
                    ..Default::default()
                },
                deck: vec![entry(0); 60],
                sideboard: vec![],
                commanders: vec![],
                starting_life: None,
                starting_hand: Some(if seat == 0 { vec![entry(3)] } else { vec![] }),
                starting_battlefield: if seat == 0 {
                    vec![entry(1), entry(2)]
                } else {
                    vec![]
                },
                emblems: vec![],
                team: None,
            })
            .collect(),
    }
}

fn pass(engine: &mut Engine<RegistryLookup>) {
    let (player, action) = match engine.pending() {
        Pending::Mulligan { player, .. } => (*player, PlayerAction::MulliganKeep),
        Pending::Priority { player, .. } => (*player, PlayerAction::PassPriority),
        other => panic!("unexpected question: {other:?}"),
    };
    engine.apply(player, action).expect("legal fixture action");
}

fn seed_artifacts(engine: &mut Engine<RegistryLookup>, player: PlayerId, count: u32) {
    let state = engine.dev_state_mut(player).unwrap();
    let name = state.names.intern("Tapped artifact fixture");
    for _ in 0..count {
        let id = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let object = state.object_mut(id).unwrap();
        object.base_mut().types = TypeSet::ARTIFACT;
        object.status.insert(Status::TAPPED);
    }
    for (color, amount) in [
        (ManaColor::Colorless, 3),
        (ManaColor::Green, 1),
        (ManaColor::White, 1),
        (ManaColor::Blue, 1),
    ] {
        state.players[0].mana_pool.add(color, amount);
    }
}

fn statics() -> baylee_gamehost::GameStatic {
    let preset = preset();
    baylee_gamehost::game_static(
        "rally-stress".into(),
        PlayerId::new(0),
        (0..2)
            .map(|seat| baylee_gamehost::SeatIdentity {
                player: PlayerId::new(seat),
                display_name: format!("Profiling seat {seat}"),
                is_ai: false,
                away: false,
                team: None,
            })
            .collect(),
        &preset.prints,
        &[true; 4],
        &preset.house_rules,
    )
}

fn run(count: u32, dump: Option<&str>) {
    let player = PlayerId::new(0);
    let mut engine = Engine::new(&preset(), RegistryLookup).unwrap();
    for _ in 0..32 {
        if matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player)
            && engine.state().turn.phase == Phase::FirstMain
        {
            break;
        }
        pass(&mut engine);
    }
    seed_artifacts(&mut engine, player, count);
    engine.refresh_offer();
    let aang = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == card("Aang and Katara"))
        })
        .unwrap();
    let started = Instant::now();
    engine
        .apply(player, PlayerAction::CastSpell { card: aang })
        .unwrap();
    let mut peak = false;
    let mut max_step = std::time::Duration::ZERO;
    for _ in 0..(count * 8 + 100) {
        let state = engine.state();
        let tokens = state
            .battlefield_view()
            .iter()
            .filter(|id| state.object(**id).unwrap().token.is_some())
            .count();
        if tokens == count as usize * 2 && !peak {
            peak = true;
            let at = Instant::now();
            let view = baylee_gamehost::player_view(
                state,
                player,
                1,
                Some(engine.pending()),
                &SeatContext::default(),
                &[],
            );
            eprintln!(
                "allies={tokens} stack={} build_view_ms={:.3}",
                view.stack.len(),
                at.elapsed().as_secs_f64() * 1000.0
            );
            if let Some(path) = dump {
                std::fs::write(path, serde_json::to_vec(&(statics(), view)).unwrap()).unwrap();
            }
        }
        if peak && state.zones.list(ZoneLocation::Stack).is_empty() {
            break;
        }
        let step = Instant::now();
        pass(&mut engine);
        max_step = max_step.max(step.elapsed());
    }
    assert!(peak);
    assert!(engine.state().zones.list(ZoneLocation::Stack).is_empty());
    let state = engine.state();
    let allies: Vec<_> = state
        .battlefield_view()
        .into_iter()
        .filter(|id| state.object(*id).unwrap().token.is_some())
        .collect();
    assert_eq!(allies.len(), count as usize * 2);
    for id in allies {
        let object = state.object(id).unwrap();
        assert_eq!(object.counters.get(baylee_cards_dsl::CounterKind::P1P1), 2);
        assert!(
            object
                .characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::HASTE)
        );
    }
    eprintln!(
        "rally_ms={:.3} max_apply_ms={:.3} hash={:016x}",
        started.elapsed().as_secs_f64() * 1000.0,
        max_step.as_secs_f64() * 1000.0,
        state.snapshot_hash()
    );
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let count = args
        .get(1)
        .map_or(128, |s| s.parse().expect("artifact count"));
    run(count, args.get(2).map(String::as_str));
}

#[test]
fn every_ally_gets_exactly_two_counters_and_haste() {
    run(64, None);
}
