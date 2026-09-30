//! Decision latency, including construction of the public tactical position.
#![allow(missing_docs)]

use baylee_ai::{AIProfile, HeuristicAgent, search};
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_engine::choice::{LegalActions, Pending};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn combat(c: &mut Criterion) {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, (0..6).map(|i| token(i, 0, "attacker", 3, 3)))
        .with_battlefield(1, (10..16).map(|i| token(i, 1, "blocker", 2, 2)))
        .build();
    let squad: Vec<_> = (0..6).map(|i| ObjectId::new(i, 0)).collect();
    let pending = Pending::ChooseAttackers {
        player: view.seat,
        attackers: squad.clone(),
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    for (name, profile) in AIProfile::NAMED {
        let agent = HeuristicAgent::new(profile);
        let report = search::attackers(&view, &squad, PlayerId::new(1), profile);
        eprintln!(
            "{name}: {} nodes, {} complete candidates",
            report.nodes, report.completed
        );
        c.bench_function(&format!("combat/6v6/{name}"), |b| {
            b.iter(|| black_box(agent.act(black_box(&view), black_box(&pending))));
        });
    }
    let crowded = ViewBuilder::new(2)
        .with_battlefield(0, (0..8).map(|i| token(i, 0, "attacker", 3, 3)))
        .with_battlefield(1, (10..18).map(|i| token(i, 1, "blocker", 2, 2)))
        .build();
    let squad: Vec<_> = (0..8).map(|i| ObjectId::new(i, 0)).collect();
    let pending = Pending::ChooseAttackers {
        player: crowded.seat,
        attackers: squad.clone(),
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    for (name, profile) in [("sharp", AIProfile::SHARP), ("expert", AIProfile::EXPERT)] {
        let agent = HeuristicAgent::new(profile);
        let report = search::attackers(&crowded, &squad, PlayerId::new(1), profile);
        eprintln!(
            "8v8/{name}: {} nodes, {} complete candidates",
            report.nodes, report.completed
        );
        c.bench_function(&format!("combat/8v8/{name}"), |b| {
            b.iter(|| black_box(agent.act(black_box(&crowded), black_box(&pending))));
        });
    }
}

/// Tokens on one side of a combat, the board self-play game r001 #431 grew
/// to tens of thousands of: every decision there used to look each creature
/// up by walking every zone of the view.
const ARMY: u32 = 1_000;

/// A planeswalker with `loyalty`, for the attack that must choose between it
/// and its controller.
fn walker(slot: u32, controller: u8, loyalty: u16) -> baylee_view::PublicObject {
    let mut w = token(slot, controller, "walker", 0, 0);
    w.types = baylee_core::types::TypeSet::PLANESWALKER;
    w.power = None;
    w.toughness = None;
    w.loyalty = Some(loyalty);
    w.counters = vec![baylee_view::CounterEntry {
        kind: baylee_view::CounterKind::Loyalty,
        count: loyalty,
    }];
    w
}

fn token_army(c: &mut Criterion) {
    let army = |seat: u8| (100..100 + ARMY).map(move |i| token(i, seat, "Ally", 1, 1));
    let few = |seat: u8| (10..14).map(move |i| token(i, seat, "few", 3, 3));
    // The army attacks four blockers and a walker.
    let view = ViewBuilder::new(2)
        .with_battlefield(0, army(0))
        .with_battlefield(1, few(1).chain([walker(20, 1, 3)]))
        .build();
    let attack = Pending::ChooseAttackers {
        player: view.seat,
        attackers: (100..100 + ARMY).map(|i| ObjectId::new(i, 0)).collect(),
        defenders: vec![
            Defender::Player(PlayerId::new(1)),
            Defender::Planeswalker(ObjectId::new(20, 0)),
        ],
        required: Vec::new(),
        limits: Vec::new(),
    };
    // Four attack into the army.
    let into = ViewBuilder::new(2)
        .with_battlefield(0, few(0))
        .with_battlefield(1, army(1))
        .build();
    let attack_into = Pending::ChooseAttackers {
        player: into.seat,
        attackers: (10..14).map(|i| ObjectId::new(i, 0)).collect(),
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    // The army blocks four, and four block the army.
    let blocking = |attackers: Vec<ObjectId>, blockers: Vec<ObjectId>, board| {
        let mut view = ViewBuilder::new(2)
            .with_combat(
                attackers
                    .iter()
                    .map(|&creature| baylee_view::AttackerView {
                        creature,
                        defending: Defender::Player(PlayerId::new(0)),
                        blocked: false,
                    })
                    .collect(),
                vec![],
            )
            .build();
        view.battlefield = board;
        view.active = PlayerId::new(1);
        view.step = baylee_view::Step::DeclareBlockers;
        let pending = Pending::ChooseBlockers {
            player: view.seat,
            attacker: PlayerId::new(1),
            blockers: blockers
                .into_iter()
                .map(|blocker| baylee_engine::choice::BlockOption {
                    blocker,
                    attackers: attackers.clone(),
                })
                .collect(),
            capacity: Vec::new(),
            obeying: Vec::new(),
        };
        (view, pending)
    };
    let ids = |range: std::ops::Range<u32>| range.map(|i| ObjectId::new(i, 0)).collect::<Vec<_>>();
    let (army_blocks, army_blocks_pending) = blocking(
        ids(10..14),
        ids(100..100 + ARMY),
        army(0).chain(few(1)).collect(),
    );
    let (few_block, few_block_pending) = blocking(
        ids(100..100 + ARMY),
        ids(10..14),
        few(0).chain(army(1)).collect(),
    );
    for (name, profile) in [
        ("novice", AIProfile::NOVICE),
        ("sharp", AIProfile::SHARP),
        ("expert", AIProfile::EXPERT),
    ] {
        let agent = HeuristicAgent::new(profile);
        for (shape, view, pending) in [
            ("attack", &view, &attack),
            ("attack-into", &into, &attack_into),
            ("block-with", &army_blocks, &army_blocks_pending),
            ("block-against", &few_block, &few_block_pending),
        ] {
            c.bench_function(&format!("combat/army-{ARMY}/{shape}/{name}"), |b| {
                b.iter(|| black_box(agent.act(black_box(view), black_box(pending))));
            });
        }
    }
}

fn priority(c: &mut Criterion) {
    let empty = ViewBuilder::new(2).build();
    let pending = Pending::Priority {
        player: empty.seat,
        legal: Box::new(LegalActions {
            can_pass: true,
            ..Default::default()
        }),
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    c.bench_function("priority/empty/expert", |b| {
        b.iter(|| black_box(agent.act(black_box(&empty), black_box(&pending))));
    });
    let mut casting = ViewBuilder::new(2).build();
    for i in 0..8 {
        let mut land = token(i, 0, "Island", 0, 0);
        land.types = baylee_core::types::TypeSet::LAND;
        land.subtypes
            .insert(baylee_core::generated::subtypes::land::ISLAND);
        casting.battlefield.push(land);
    }
    for (i, name) in ["Brainstorm", "Mana Drain", "Force of Will", "Sol Ring"]
        .into_iter()
        .enumerate()
    {
        let index = baylee_cards::decks::by_name(name).unwrap();
        let face = &baylee_cards::by_index(index).unwrap().faces[0];
        casting.hand.push(baylee_view::HandObject {
            id: ObjectId::new(20 + u32::try_from(i).unwrap(), 0),
            card: baylee_view::CardIdentity {
                index,
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            },
            name: name.into(),
            mana_value: face.mana_cost.cmc(),
            colors: face.mana_cost.colors(),
            types: face.types,
            commander: false,
        });
    }
    let pending = Pending::Priority {
        player: casting.seat,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: (0..8).map(|i| ObjectId::new(i, 0)).collect(),
            ..Default::default()
        }),
    };
    c.bench_function("priority/8lands-4spells/expert", |b| {
        b.iter(|| black_box(agent.act(black_box(&casting), black_box(&pending))));
    });
    scouted_priority(c, &casting, &pending, &agent);
    let pending = Pending::ChooseColor {
        player: casting.seat,
        options: vec![
            baylee_core::mana::ManaColor::White,
            baylee_core::mana::ManaColor::Blue,
            baylee_core::mana::ManaColor::Black,
        ],
    };
    c.bench_function("mana-choice/8lands-4spells/expert", |b| {
        b.iter(|| black_box(agent.act(black_box(&casting), black_box(&pending))));
    });
}

fn scouted_priority(
    c: &mut Criterion,
    view: &baylee_view::PlayerView,
    pending: &Pending,
    agent: &HeuristicAgent,
) {
    use baylee_ai::intelligence::{DeckIntel, ScoutedSeat, ScoutingReport};
    let cards: Vec<_> = view
        .hand
        .iter()
        .map(|c| c.card.index)
        .cycle()
        .take(100)
        .collect();
    let own = DeckIntel::new(cards.clone(), vec![]);
    let enemy = DeckIntel::new(cards.clone(), vec![]);
    let context = baylee_engine::engine::DecisionContext::default();
    // Includes report allocation/copies and the policy, but not host view
    // construction. Immutable deck analysis is benchmarked independently.
    c.bench_function("priority/scouted-8lands-4spells/expert", |b| {
        b.iter(|| {
            let report = ScoutingReport {
                seats: vec![
                    ScoutedSeat {
                        player: view.seat,
                        deck: &own,
                        hand: Some(cards[..4].to_vec()),
                        library: Some(cards[..3].to_vec()),
                        sideboard: None,
                    },
                    ScoutedSeat {
                        player: PlayerId::new(1),
                        deck: &enemy,
                        hand: Some(cards[..7].to_vec()),
                        library: Some(cards[..3].to_vec()),
                        sideboard: None,
                    },
                ],
            };
            black_box(agent.act_with_scouting(
                black_box(view),
                black_box(pending),
                &context,
                &report,
            ))
        });
    });
    c.bench_function("setup/deck-100-cards", |b| {
        b.iter(|| black_box(DeckIntel::new(cards.clone(), vec![])));
    });
}
criterion_group!(benches, combat, token_army, priority);
criterion_main!(benches);
