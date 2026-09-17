//! Paired acceptance-deck matches. Each seed plays both deck orders and both
//! profile assignments, so neither the first seat nor a deck gets the credit.

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_cards::decks::{load_acceptance, preset_for};
use baylee_core::ids::PlayerId;
use baylee_engine::win::Victor;
use baylee_gamehost::RegistryLookup;
use baylee_gamehost::harness::{Halt, play_report};
use serde_json::{Value, json};

fn outcome(halt: &Halt, a_seat: u8) -> &'static str {
    match halt {
        Halt::Finished(result) => match result.winner {
            Some(Victor::Player(p)) if p == PlayerId::new(a_seat) => "a",
            Some(Victor::Player(_)) => "b",
            None => "draw",
            Some(Victor::Team(_)) => panic!("this scoreboard runs duels without teams"),
        },
        Halt::Repeated { .. } => "repeated",
        Halt::CapReached => "cap",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: ai_match PROFILE_A PROFILE_B SEED,SEED,... (JSON to stdout)".into());
    }
    let a = AIProfile::named(&args[0]).ok_or("unknown profile A")?;
    let b = AIProfile::named(&args[1]).ok_or("unknown profile B")?;
    let seeds: Vec<u64> = args[2]
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ))?;
    let decks = [
        load_acceptance(&text, "Allytifact")?,
        load_acceptance(&text, "Victory")?,
    ];
    let mut games: Vec<Value> = Vec::new();
    let mut wins = [0_u32; 2];
    let mut draws = 0_u32;
    let mut unfinished = 0_u32;
    for &seed in &seeds {
        for deck_order in 0..2 {
            for a_seat in 0..2_u8 {
                let preset = preset_for(seed, &decks[deck_order], &decks[1 - deck_order]);
                let profiles = if a_seat == 0 { [a, b] } else { [b, a] };
                let agents = profiles.map(HeuristicAgent::new);
                let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    play_report(RegistryLookup, &preset, &agents, 20_000)
                }));
                let Ok(report) = played else {
                    unfinished += 1;
                    games.push(json!({
                        "seed": seed, "deck_order": deck_order, "a_seat": a_seat,
                        "outcome": "panic",
                    }));
                    continue;
                };
                let result = outcome(&report.halt, a_seat);
                match result {
                    "a" => wins[0] += 1,
                    "b" => wins[1] += 1,
                    "draw" => draws += 1,
                    _ => unfinished += 1,
                }
                games.push(json!({
                    "seed": seed, "deck_order": deck_order, "a_seat": a_seat,
                    "outcome": result, "halt": format!("{:?}", report.halt),
                    "actions": report.actions, "turn": report.turn,
                    "refused": report.tally.iter().map(|t| t.refused_cost + t.refused_other).sum::<usize>(),
                    "trail": report.trail,
                }));
            }
        }
    }
    let decisive = wins[0] + wins[1];
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "a": args[0], "b": args[1], "profiles": [a, b], "seeds": seeds,
            "action_cap": 20_000, "wins": wins, "draws": draws, "unfinished": unfinished,
            "a_decisive_win_rate": (decisive > 0).then(|| f64::from(wins[0]) / f64::from(decisive)),
            "games": games,
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inject a real loss into the engine, then read it from both seats.
    /// A scorer that always credits A fails even if self-play looks balanced.
    #[test]
    fn the_scoreboard_observes_a_loss_and_a_cap() {
        let forest = baylee_cards::decks::by_name("Forest").unwrap();
        let mut preset = baylee_cards::decks::probe_preset(7, forest).unwrap();
        preset.seats[0].starting_life = Some(0);
        let agents = [
            HeuristicAgent::new(AIProfile::STEADY),
            HeuristicAgent::new(AIProfile::STEADY),
        ];
        let report = play_report(RegistryLookup, &preset, &agents, 100);
        assert_eq!(outcome(&report.halt, 0), "b");
        assert_eq!(outcome(&report.halt, 1), "a");
        preset.seats[0].starting_life = Some(20);
        let capped = play_report(RegistryLookup, &preset, &agents, 0);
        assert_eq!(outcome(&capped.halt, 0), "cap");
    }
}
