//! `cargo tools pgo compare`: a plain client against a profiled one, in turn.
//!
//! What ships has no harness, so both binaries are built with `dev-control`
//! (`docs/perf-client.md` §"A client built from a profile"). They run one after
//! the other, each round the other first, so that what the machine does
//! meanwhile falls on both.
//!
//! Each client stands at a frame limit (120 frames a second on the machine it was
//! first run on), so a frame's length says little. What does is the main
//! schedule's time, which no limit touches, and the CPU a frame takes, which
//! holds the render thread and the driver as well.

use std::{
    fs,
    path::Path,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use baylee_client_e2e::prelude::*;
use serde_json::{Value, json};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::train::{LONGEST_GAME, SEED, board, play};

/// How long one reading of a screen lasts.
const READING: Duration = Duration::from_secs(10);
/// How long a screen stands before it is read, for what it loads to settle.
const SETTLE: Duration = Duration::from_secs(4);
/// The games played per binary: one takes minutes, and the frames take seconds.
const GAMES: usize = 3;

/// One reading of one screen.
struct Reading {
    screen: &'static str,
    /// The main schedule's median, in milliseconds.
    update_p50: f64,
    update_p95: f64,
    frame_p50: f64,
    frame_p99: f64,
    frame_max: f64,
    /// All threads' CPU time per frame, in milliseconds.
    cpu_per_frame: f64,
}

/// One whole game.
struct Game {
    cpu_seconds: f64,
    wall_seconds: f64,
    turn: u64,
}

/// Plays `rounds` of each binary in turn and prints the medians and their spans.
pub fn run(root: &Path, plain: &Path, profiled: &Path, rounds: usize) -> Result<()> {
    let work = root.join("target/pgo-profiles/client/compare");
    let binaries = [("plain", plain), ("profiled", profiled)];
    let mut readings: [Vec<Reading>; 2] = [Vec::new(), Vec::new()];
    let mut games: [Vec<Game>; 2] = [Vec::new(), Vec::new()];
    let mut failed = 0;
    for round in 0..rounds {
        for turn in 0..2 {
            let which = (round + turn) % 2;
            let (name, binary) = binaries[which];
            let dir = work.join(format!("{round}-{name}"));
            println!("round {round}: {name}, the screens");
            // A run that fails is said and left out: it takes the others' numbers
            // with it otherwise, and a game can stand still at a question.
            match screens(root, binary, &dir) {
                Ok(read) => {
                    for reading in &read {
                        println!(
                            "  {}: main {:.2} ms, CPU {:.2} ms a frame, frame {:.2} ms",
                            reading.screen,
                            reading.update_p50,
                            reading.cpu_per_frame,
                            reading.frame_p50
                        );
                    }
                    readings[which].extend(read);
                }
                Err(why) => {
                    failed += 1;
                    println!("  failed, left out: {why:#}");
                }
            }
        }
    }
    for round in 0..GAMES {
        for turn in 0..2 {
            let which = (round + turn) % 2;
            let (name, binary) = binaries[which];
            let dir = work.join(format!("game-{round}-{name}"));
            println!("round {round}: {name}, a whole game");
            match whole_game(binary, &dir) {
                Ok(game) => {
                    println!(
                        "  turn {}, {:.1} s of CPU over {:.1} s",
                        game.turn, game.cpu_seconds, game.wall_seconds
                    );
                    games[which].push(game);
                }
                Err(why) => {
                    failed += 1;
                    println!("  failed, left out: {why:#}");
                }
            }
        }
    }
    report(&readings, &games);
    ensure!(failed == 0, "{failed} runs failed and are left out");
    Ok(())
}

/// A directory of its own for one client, emptied first.
fn fresh(dir: &Path) -> Result<()> {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir.join("config/baylee")).with_context(|| dir.display().to_string())
}

/// The lobby, a small duel and a full board, each read at rest.
fn screens(root: &Path, binary: &Path, dir: &Path) -> Result<Vec<Reading>> {
    fresh(dir)?;
    let launch = Launch::new().tours(false).env("BAYLEE_DEV_SEED", SEED);
    let client = Client::start(binary, &launch, dir)?;
    client.play_offline()?;
    let lobby = read(&client, "Lobby")?;
    client.sit_down_with_the_house()?;
    let duel = read(&client, "Duel")?;
    client.close()?;

    fresh(dir)?;
    let launch = Launch::new()
        .tours(false)
        .env("BAYLEE_DEV_SEED", SEED)
        .env("BAYLEE_DEV_SEAT_BOARD", &board(root)?);
    let client = Client::start(binary, &launch, dir)?;
    client.play_offline()?;
    client.sit_down_with_the_house()?;
    client.press("K")?;
    let full_board = read(&client, "Full board")?;
    client.close()?;
    Ok(vec![lobby, duel, full_board])
}

/// A whole offline game on the training's seed, and what it took.
fn whole_game(binary: &Path, dir: &Path) -> Result<Game> {
    fresh(dir)?;
    let launch = Launch::new().tours(false).env("BAYLEE_DEV_SEED", SEED);
    let client = Client::start(binary, &launch, dir)?;
    client.play_offline()?;
    client.sit_down_with_the_house()?;
    let cpu = cpu_seconds(&client)?;
    let started = Instant::now();
    play(&client, LONGEST_GAME, |_| Ok(()))?;
    let game = Game {
        cpu_seconds: cpu_seconds(&client)? - cpu,
        wall_seconds: started.elapsed().as_secs_f64(),
        turn: client.screen()?.at("/view/turn").as_u64().unwrap_or(0),
    };
    client.close()?;
    Ok(game)
}

/// One screen over [`READING`], after it stood for [`SETTLE`].
fn read(client: &Client, screen: &'static str) -> Result<Reading> {
    sleep(SETTLE);
    client.devctl().post("perf", &json!({"reset": true}))?;
    let before = cpu_seconds(client)?;
    sleep(READING);
    let perf = client.devctl().post("perf", &json!({}))?;
    let cpu = cpu_seconds(client)? - before;
    let number = |pointer: &str| perf.pointer(pointer).and_then(Value::as_f64).unwrap_or(0.0);
    Ok(Reading {
        screen,
        update_p50: number("/update_ms/p50"),
        update_p95: number("/update_ms/p95"),
        frame_p50: number("/frame_ms/p50"),
        frame_p99: number("/frame_ms/p99"),
        frame_max: number("/frame_ms/max"),
        cpu_per_frame: cpu * 1000.0 / number("/frames").max(1.0),
    })
}

/// What the client's threads have used so far.
fn cpu_seconds(client: &Client) -> Result<f64> {
    let pid = Pid::from_u32(client.pid().context("the client has ended")?);
    let mut system = System::new().context("reading the system")?;
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cpu(),
    );
    let process = system.process(pid).context("the client is not running")?;
    #[allow(clippy::cast_precision_loss)]
    Ok(process.accumulated_cpu_time() as f64 / 1000.0)
}

/// The middle value and the span, as `docs/perf-client.md` gives them.
fn summary(mut values: Vec<f64>) -> String {
    values.sort_by(f64::total_cmp);
    let Some((first, last)) = values.first().zip(values.last()) else {
        return "none".to_owned();
    };
    let middle = values.len() / 2;
    let median = if values.len().is_multiple_of(2) {
        f64::midpoint(values[middle - 1], values[middle])
    } else {
        values[middle]
    };
    format!("{median:.2} [{first:.2}-{last:.2}]")
}

fn report(readings: &[Vec<Reading>; 2], games: &[Vec<Game>; 2]) {
    type Field = fn(&Reading) -> f64;
    let fields: [(&str, Field); 6] = [
        ("Main schedule p50, ms", |r| r.update_p50),
        ("Main schedule p95, ms", |r| r.update_p95),
        ("CPU per frame, ms", |r| r.cpu_per_frame),
        ("Frame p50, ms", |r| r.frame_p50),
        ("Frame p99, ms", |r| r.frame_p99),
        ("Frame max, ms", |r| r.frame_max),
    ];
    for (title, field) in fields {
        println!("\n| {title} | Plain | Profiled |\n| --- | ---: | ---: |");
        for screen in ["Lobby", "Duel", "Full board"] {
            let of = |which: usize| {
                summary(
                    readings[which]
                        .iter()
                        .filter(|reading| reading.screen == screen)
                        .map(field)
                        .collect(),
                )
            };
            println!("| {screen} | {} | {} |", of(0), of(1));
        }
    }
    println!("\n| A whole game | Plain | Profiled |\n| --- | ---: | ---: |");
    let of =
        |which: usize, field: fn(&Game) -> f64| summary(games[which].iter().map(field).collect());
    println!(
        "| CPU, s | {} | {} |",
        of(0, |g| g.cpu_seconds),
        of(1, |g| g.cpu_seconds)
    );
    println!(
        "| Wall, s | {} | {} |",
        of(0, |g| g.wall_seconds),
        of(1, |g| g.wall_seconds)
    );
    #[allow(clippy::cast_precision_loss)]
    let turn = |g: &Game| g.turn as f64;
    println!("| Last turn | {} | {} |", of(0, turn), of(1, turn));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summary_is_the_median_and_the_span() {
        assert_eq!(summary(vec![3.0, 1.0, 2.0]), "2.00 [1.00-3.00]");
        assert_eq!(summary(vec![4.0, 1.0, 2.0, 3.0]), "2.50 [1.00-4.00]");
        assert_eq!(summary(Vec::new()), "none");
    }
}
