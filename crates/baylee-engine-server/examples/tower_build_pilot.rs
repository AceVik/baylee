//! Twelve bounded native duels: a reproducible content pilot, not a balance verdict.
#![forbid(unsafe_code)]

use baylee_ai::HeuristicAgent;
use baylee_cards::decks::{DeckCard, LoadedDeck};
use baylee_cards_pool as _;
use baylee_core::{
    ids::PlayerId,
    preset::{FormatId, SeatController},
};
use baylee_engine::choice::Pending;
use baylee_engine_server::variant::{self, HostedSession};
use baylee_tower::v2::{
    self,
    catalog::{Definition, Family, Rarity},
    generation, loadout,
};
use std::time::{Duration, Instant};

const BUILDS: [(&str, [Family; 3]); 3] = [
    (
        "life",
        [
            Family::SpringChalice,
            Family::Hearthstone,
            Family::BloodAmber,
        ],
    ),
    (
        "tokens",
        [Family::Grove, Family::Hearthstone, Family::LifeCoil],
    ),
    (
        "reference",
        [Family::Grove, Family::Heartwood, Family::Satchel],
    ),
];

fn main() {
    println!("Native Common-item pilot; identical 39+1 Emmara decks, 12 gold per build.");
    println!("Three fixed seeds, both build assignments; no shop policy or human-skill claim.");
    println!(
        "Limits: 2000 engine steps/80 turns per duel, 60 seconds total; no invented outcomes."
    );
    println!("build,seed,challenger_seat,result,turns,steps,life0,life1");
    let deadline = Instant::now() + Duration::from_secs(60);
    let deck = deck();
    for (name, challenger) in BUILDS.into_iter().take(2) {
        for seed in [101, 991, 2027] {
            for challenger_seat in [0, 1] {
                if Instant::now() >= deadline {
                    println!("overall-budget-exhausted; remaining duels were not run");
                    return;
                }
                play(name, challenger, seed, challenger_seat, &deck, deadline);
            }
        }
    }
}

fn play(
    name: &str,
    challenger: [Family; 3],
    seed: u64,
    challenger_seat: u8,
    deck: &LoadedDeck,
    deadline: Instant,
) {
    let profile = generation::contract(v2::GENERATOR)
        .expect("compiled generation")
        .expert();
    let mut preset = baylee_cards::decks::preset_for(seed, deck, deck);
    preset.format = FormatId::Custom;
    let mut builds = [BUILDS[2].1; 2];
    builds[usize::from(challenger_seat)] = challenger;
    let resources = [0, 1].map(|seat| {
        let active = builds[usize::from(seat)].map(|family| Definition {
            family,
            rarity: Rarity::Common,
        });
        loadout::resources(&active, [0, 0], seat).expect("three Common relics")
    });
    preset.house_rules = loadout::rules(&resources, seed).expect("native rules");
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Ai(profile);
    for (seat, resource) in preset.seats.iter_mut().zip(resources) {
        seat.starting_life = Some(resource.life);
        for entry in seat.deck.iter_mut().chain(&mut seat.commanders) {
            entry.rule_print = None;
        }
    }
    let mut game = HostedSession::recorded_for_execution(&preset, &variant::identity())
        .expect("compiled executor admission");
    // The manually played chair uses only its authorized contextual view. The
    // other chair runs the actual host policy; swapping builds exposes chair bias.
    let player = PlayerId::new(0);
    let agent = HeuristicAgent::new(profile).with_seed(seed);
    let mut steps = 0;
    while !matches!(game.pending(), Pending::GameOver(_))
        && steps < 2000
        && game.state().turn.number <= 80
        && Instant::now() < deadline
    {
        if game.needs_pump() {
            game.pump_slice(1);
        } else {
            let (pending, view) = game
                .view_for_with_context(player)
                .expect("manual chair asked");
            let action = agent.act(&view, &pending);
            game.act_slice(player, action, 0)
                .expect("legal pilot answer");
        }
        steps += 1;
    }
    let result = if let Pending::GameOver(result) = game.pending() {
        let winners = game.winning_seats(*result);
        if winners.is_empty() {
            "draw"
        } else if winners.contains(&PlayerId::new(challenger_seat)) {
            "win"
        } else {
            "loss"
        }
    } else {
        "bounded-unfinished"
    };
    println!(
        "{name},{seed},{challenger_seat},{result},{},{steps},{},{}",
        game.state().turn.number,
        game.state().players[0].life,
        game.state().players[1].life,
    );
}

fn deck() -> LoadedDeck {
    let content = baylee_tower_content::standard().expect("reviewed Commander pool");
    let starter = &content.starters[0];
    let main = starter
        .deck
        .iter()
        .flat_map(|row| std::iter::repeat_n(DeckCard::plain(row.card), usize::from(row.copies)))
        .collect();
    LoadedDeck {
        name: "Synthetic Emmara pilot".into(),
        main,
        sideboard: Vec::new(),
        commanders: vec![DeckCard::plain(
            starter.formation.as_ref().unwrap().commander,
        )],
    }
}
