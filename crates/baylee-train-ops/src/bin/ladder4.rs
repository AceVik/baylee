//! A small ladder: the net in `seat4` against a language model in `baylee-seat`,
//! on a private gateway. For numbers only: these games are never training data,
//! and the root says so in `NOT-FOR-TRAINING`.
//!
//! The net hosts every room and sits in chair 0, the language model takes
//! chair 1 as a guest. A pair of decks is played twice with the seats swapped.
//! Keys are read from the environment by `baylee-seat` alone; this program
//! never touches them and starts each seat with only what it needs.
// Counts of games, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
use anyhow::{Context, Result, bail, ensure};
use baylee_train_ops::evaluation::{atomic_json, read_json};
use clap::Parser;
use serde_json::{Value, json};
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "Plays seat4 against language model seats, for numbers only")]
struct Args {
    /// Where results, transcripts and logs go.
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    gateway: String,
    #[arg(long)]
    seat4: PathBuf,
    #[arg(long)]
    seat: PathBuf,
    #[arg(long)]
    model: PathBuf,
    /// The seat settings file of the profiles.
    #[arg(long)]
    profiles: PathBuf,
    /// The spend book of the run, which holds the file's caps.
    #[arg(long)]
    ledger: PathBuf,
    /// A deck manifest: `{"decks":[{"key","path"}]}`.
    #[arg(long)]
    decks: PathBuf,
    /// `PROFILE:MODE:GAMES`, MODE being `raw`, `tactical` or `tactical=GROUPS`.
    #[arg(long, required = true)]
    arm: Vec<String>,
    /// Games at once, over all arms.
    #[arg(long, default_value_t = 6)]
    tables: usize,
    #[arg(long, default_value_t = 2)]
    thinking_steps: u8,
    #[arg(long, default_value_t = 120)]
    think_secs: u64,
    /// A game not over by then is stopped and counted as unfinished.
    #[arg(long, default_value_t = 3600)]
    game_timeout_secs: u64,
    /// Offset into the deck manifest, so that a later run plays other decks.
    #[arg(long, default_value_t = 0)]
    first_deck: usize,
    /// Seconds between two games' starts, over all tables.
    #[arg(long, default_value_t = 80)]
    start_gap_secs: u64,
    /// No game starts once the book will likely have spent this much, in the
    /// profile's unit (dollars for a priced model, else tokens). The book's cap
    /// has to hold the reservations of the open games on top of this.
    #[arg(long)]
    stop_spent: Option<f64>,
    /// Passed to `seat4`: the model is older than this build's catalog.
    #[arg(long)]
    catalog_drift: bool,
}

#[derive(Clone, Debug)]
struct Game {
    index: usize,
    profile: String,
    mode: String,
    net_deck: PathBuf,
    llm_deck: PathBuf,
    net_key: String,
    llm_key: String,
}

fn schedule(args: &Args) -> Result<Vec<Game>> {
    let manifest = read_json(&args.decks)?;
    let decks = manifest["decks"]
        .as_array()
        .context("manifest has no decks")?;
    ensure!(decks.len() >= 2, "A ladder needs two decks");
    let mut games = Vec::new();
    for arm in &args.arm {
        // Every arm plays the same decks in the same order, so that two arms
        // differ in what they play, not in what they were dealt to play with.
        let mut at = args.first_deck;
        // From the right: `house:sharp` is a profile with a colon of its own.
        let parts: Vec<&str> = arm.rsplitn(3, ':').collect();
        let [count, mode, profile] = parts[..] else {
            bail!("an arm is PROFILE:MODE:GAMES, not {arm}");
        };
        ensure!(
            matches!(mode, "raw" | "tactical") || mode.starts_with("tactical="),
            "unknown mode {mode}"
        );
        for game in 0..count.parse::<usize>().context("games")? {
            // Two games of a deal share the decks, with the seats swapped.
            let (first, second) = (
                decks[at % decks.len()].clone(),
                decks[(at + 1) % decks.len()].clone(),
            );
            let (net, llm) = if game % 2 == 0 {
                (&first, &second)
            } else {
                (&second, &first)
            };
            if game % 2 == 1 {
                at += 2;
            }
            games.push(Game {
                index: games.len(),
                profile: profile.to_owned(),
                mode: mode.to_owned(),
                net_deck: PathBuf::from(net["path"].as_str().context("deck path")?),
                llm_deck: PathBuf::from(llm["path"].as_str().context("deck path")?),
                net_key: net["key"].as_str().unwrap_or("").to_owned(),
                llm_key: llm["key"].as_str().unwrap_or("").to_owned(),
            });
        }
    }
    Ok(games)
}

/// Spawns `seat4` as the room's host and reads its room from its first line.
fn host(
    args: &Args,
    game: &Game,
    work: &Path,
) -> Result<(Child, String, BufReader<std::process::ChildStdout>)> {
    let mut command = Command::new(&args.seat4);
    command
        .arg("--gateway")
        .arg(&args.gateway)
        .arg("--model")
        .arg(&args.model)
        .arg("--thinking-steps")
        .arg(args.thinking_steps.to_string())
        .arg("--deck")
        .arg(&game.net_deck)
        .arg("--room-name")
        .arg(format!("Ladder-{}", game.index))
        .arg("--games")
        .arg("1")
        .arg("--name")
        .arg(format!("b{}", game.index))
        .arg("--reports")
        .arg(work.join("reports"))
        .arg("--journal")
        .arg(work.join("journal.jsonl.gz"));
    if args.catalog_drift {
        command.arg("--catalog-drift");
    }
    if game.mode == "raw" {
        command.arg("--raw-policy");
    } else if let Some(groups) = game.mode.strip_prefix("tactical=") {
        command.arg("--tactics-rules").arg(groups);
    }
    command
        .env_clear()
        .env("BAYLEE_EP", "cpu")
        .env("OMP_NUM_THREADS", "1");
    for name in ["PATH", "HOME", "USER", "LANG", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(work.join("seat4.stderr.log"))?)
        .spawn()
        .context("seat4")?;
    let mut lines = BufReader::new(child.stdout.take().context("seat4 stdout")?);
    let mut line = String::new();
    loop {
        line.clear();
        if lines.read_line(&mut line)? == 0 {
            let _ = child.wait();
            bail!("seat4 ended before it opened a room");
        }
        if let Ok(value) = serde_json::from_str::<Value>(&line)
            && value["phase"] == "waiting-for-player"
        {
            let room = value["game"].as_str().context("room id")?.to_owned();
            return Ok((child, room, lines));
        }
    }
}

/// Spawns the language model's seat as a guest in chair 1. Its environment is
/// cleared to what it needs and its profile's key variable, passed by name.
fn guest(args: &Args, game: &Game, room: &str, work: &Path) -> Result<Child> {
    let mut command = Command::new(&args.seat);
    command
        .arg("join")
        .arg(room)
        .arg("--gateway")
        .arg(&args.gateway)
        .arg("--chair")
        .arg("1")
        .arg("--tethered");
    // `house:<level>` seats the house, which needs no key: the same path with
    // nothing to pay, and a rung of its own.
    if let Some(level) = game.profile.strip_prefix("house:") {
        command.arg("--mind").arg("house").arg("--level").arg(level);
    } else {
        command
            .arg("--profile")
            .arg(&game.profile)
            .arg("--config")
            .arg(&args.profiles)
            .arg("--ledger")
            .arg(&args.ledger)
            .arg("--level")
            .arg("sharp");
    }
    command
        .arg("--deck")
        .arg(&game.llm_deck)
        .arg("--think-secs")
        .arg(args.think_secs.to_string())
        .arg("--transcripts")
        .arg(work.join("transcripts"))
        .arg("--name")
        .arg(format!("l{}", game.index))
        .arg("--no-send-reports");
    command.env_clear();
    for name in [
        "PATH",
        "HOME",
        "USER",
        "LANG",
        "ANTHROPIC_API_KEY",
        "DEEPSEEK_API_KEY",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .current_dir(work)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(work.join("seat.stderr.log"))?)
        .spawn()
        .context("baylee-seat")
}

/// Stops a child this program started, if it still runs.
fn reap(child: &mut Child) {
    if matches!(child.try_wait(), Ok(None)) {
        let _ = child.kill();
    }
    let _ = child.wait();
}

/// A corpus deck's main deck as the seats read one: its count lines under
/// `Deck`. A game has no sideboard, so nothing from `[sideboard]` on is played.
fn main_deck(text: &str) -> String {
    let mut plain = String::from("Deck\n");
    for line in text.lines() {
        if line.trim().to_ascii_lowercase().starts_with("[sideboard") {
            break;
        }
        if line.starts_with(|first: char| first.is_ascii_digit()) {
            plain.push_str(line);
            plain.push('\n');
        }
    }
    plain
}

fn plain_deck(source: &Path, target: PathBuf) -> Result<PathBuf> {
    let text =
        std::fs::read_to_string(source).with_context(|| format!("reading {}", source.display()))?;
    std::fs::write(&target, main_deck(&text))?;
    Ok(target)
}

fn play(args: &Args, game: &Game) -> Result<Value> {
    let work = args.root.join(format!("game-{:04}", game.index));
    std::fs::create_dir_all(work.join("transcripts"))?;
    let started = Instant::now();
    let game = &Game {
        net_deck: plain_deck(&game.net_deck, work.join("net-deck.txt"))?,
        llm_deck: plain_deck(&game.llm_deck, work.join("llm-deck.txt"))?,
        ..game.clone()
    };
    let (mut net, room, mut lines) = host(args, game, &work)?;
    let mut model = guest(args, game, &room, &work)?;
    let reader = std::thread::spawn(move || -> Option<Value> {
        let mut last = None;
        let mut line = String::new();
        while lines.read_line(&mut line).is_ok_and(|read| read > 0) {
            if let Ok(value) = serde_json::from_str::<Value>(&line)
                && matches!(
                    value["phase"].as_str(),
                    Some("game-complete" | "game-failed")
                )
            {
                last = Some(value);
            }
            line.clear();
        }
        last
    });
    let limit = Duration::from_secs(args.game_timeout_secs);
    let mut timed_out = false;
    while matches!(net.try_wait(), Ok(None)) {
        if started.elapsed() > limit {
            timed_out = true;
            break;
        }
        // A seat that never sat down leaves the host waiting for the whole
        // limit, on a table with no game (the gateway's sign-in limit).
        if let Ok(Some(status)) = model.try_wait()
            && !status.success()
        {
            reap(&mut net);
            bail!("the seat ended first: {status}");
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    reap(&mut net);
    // The tether: its stdin closing ends the seat.
    drop(model.stdin.take());
    let waited = Instant::now();
    while matches!(model.try_wait(), Ok(None)) && waited.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(200));
    }
    reap(&mut model);
    let report = reader.join().ok().flatten();
    let inner = report
        .as_ref()
        .map_or(&Value::Null, |value| &value["report"]);
    Ok(json!({
        "game": game.index, "room": room, "profile": game.profile, "mode": game.mode,
        "net_deck": game.net_key, "llm_deck": game.llm_key,
        "seconds": started.elapsed().as_secs_f64(), "timed_out": timed_out,
        "phase": report.as_ref().map(|value| value["phase"].clone()),
        "won": inner.get("won"), "drawn": inner.get("drawn"),
        "answers": inner.get("answers"), "neural_answers": inner.get("neural_answers"),
        "fallback_answers": inner.get("fallback_answers"), "failure": inner.get("failure"),
    }))
}

/// The unit a profile is booked in and the budget of one of its games. A house
/// seat has no book.
fn game_budget(settings: &Value, profile: &str) -> Option<(&'static str, f64)> {
    let entry = settings["profiles"].get(profile)?;
    entry["game_usd"].as_f64().map_or_else(
        || Some(("tokens", entry["game_tokens"].as_f64()?)),
        |usd| Some(("usd", usd)),
    )
}

/// The games of the book as `(open, amount in unit)`: an open game at its
/// reservation, a settled one at what it spent.
fn booked(book: &Value, unit: &str) -> Vec<(bool, f64)> {
    book["games"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|game| {
            let open = game["settled"].is_null();
            let state = if open { "reserved" } else { "spent" };
            let amount = game[format!("{state}_{unit}")].as_f64().unwrap_or(0.0);
            (open, amount)
        })
        .collect()
}

/// What a game of `profile` would leave of the book's monthly cap: below zero,
/// the book cannot pay for a whole game. A seat that sat down on less would run
/// out, the house would finish its game, and such a game counts for nothing.
fn margin(settings: &Value, book: &Value, profile: &str) -> Option<f64> {
    let (unit, budget) = game_budget(settings, profile)?;
    let cap = settings["caps"][format!("month_{unit}")].as_f64()?;
    let used: f64 = booked(book, unit).iter().map(|game| game.1).sum();
    Some(cap - used - budget)
}

/// What the book will likely have spent when the open games end: what the
/// settled games spent, and for each open one the mean of those played. Unlike
/// [`margin`] this does not count a reservation in full, so a run can stop at a
/// sum of money although each game holds far more than it will spend.
fn likely_spend(settings: &Value, book: &Value, profile: &str) -> Option<f64> {
    let (unit, budget) = game_budget(settings, profile)?;
    let games = booked(book, unit);
    let played: Vec<f64> = games
        .iter()
        .filter(|game| !game.0 && game.1 > 0.0)
        .map(|game| game.1)
        .collect();
    let spent: f64 = played.iter().sum();
    let mean = if played.is_empty() {
        budget
    } else {
        spent / played.len() as f64
    };
    let open = games.iter().filter(|game| game.0).count();
    Some(mean.mul_add(open as f64, spent))
}

/// Why no further game starts, if something says so: the book, `--stop-spent`,
/// or a `STOP` file in the root, which ends a run while the tables finish.
fn stop_reason(args: &Args, game: &Game) -> Result<Option<&'static str>> {
    if args.root.join("STOP").exists() {
        return Ok(Some("stopped"));
    }
    let book = if args.ledger.exists() {
        read_json(&args.ledger)?
    } else {
        Value::Null
    };
    let settings = read_json(&args.profiles)?;
    let short = margin(&settings, &book, &game.profile).is_some_and(|margin| margin < 0.0);
    let spent = args.stop_spent.is_some_and(|limit| {
        likely_spend(&settings, &book, &game.profile).is_some_and(|spend| spend >= limit)
    });
    Ok((short || spent).then_some("budget"))
}

/// Plays `game` when its turn to start comes, trying again when it never got
/// going: a game signs in twice, and the gateway takes 8 sign-ins from one
/// address in five minutes, however many tables are free.
fn play_paced(args: &Args, game: &Game, next_start: &Mutex<Instant>) -> Value {
    let gap = Duration::from_secs(args.start_gap_secs);
    let mut last = None;
    for _ in 0..4 {
        let start = {
            let mut next = next_start.lock().expect("start clock");
            let start = (*next).max(Instant::now());
            *next = start + gap;
            start
        };
        std::thread::sleep(start.saturating_duration_since(Instant::now()));
        match stop_reason(args, game) {
            Ok(None) => {}
            Ok(Some(reason)) => {
                return json!({"game": game.index, "profile": game.profile, "mode": game.mode,
                    "net_deck": game.net_key, "llm_deck": game.llm_key, "not_started": reason});
            }
            Err(error) => {
                last = Some(format!("{error:#}"));
                continue;
            }
        }
        match play(args, game) {
            Ok(result) => return result,
            Err(error) => last = Some(format!("{error:#}")),
        }
    }
    json!({"game": game.index, "profile": game.profile, "mode": game.mode, "error": last})
}

/// Wins, losses and the rest per arm.
fn summary(results: &[Value]) -> Value {
    let mut arms = std::collections::BTreeMap::<String, [u64; 4]>::new();
    for result in results {
        let key = format!(
            "{} / {}",
            result["profile"].as_str().unwrap_or(""),
            result["mode"].as_str().unwrap_or("")
        );
        let entry = arms.entry(key).or_default();
        entry[0] += 1;
        match (result["won"].as_bool(), result["drawn"].as_bool()) {
            (Some(true), _) => entry[1] += 1,
            (_, Some(true)) => entry[3] += 1,
            (Some(false), _) => entry[2] += 1,
            _ => {}
        }
    }
    Value::Object(
        arms.into_iter()
            .map(|(arm, [games, won, lost, drawn])| {
                (
                    arm,
                    json!({"games": games, "net_won": won, "net_lost": lost, "drawn": drawn,
                        "unfinished": games - won - lost - drawn}),
                )
            })
            .collect(),
    )
}

fn main() -> Result<()> {
    let args = Arc::new(Args::parse());
    std::fs::create_dir_all(&args.root)?;
    std::fs::write(
        args.root.join("NOT-FOR-TRAINING"),
        "Ladder games, for numbers only. They may never enter training, a league or a dataset.\n",
    )?;
    let games = schedule(&args)?;
    atomic_json(
        &args.root.join("schedule.json"),
        &json!(
            games
                .iter()
                .map(|game| json!({"game": game.index, "profile": game.profile,
            "mode": game.mode, "net_deck": game.net_key, "llm_deck": game.llm_key}))
                .collect::<Vec<_>>()
        ),
    )?;
    let queue = Arc::new(Mutex::new(games.into_iter()));
    let results = Arc::new(Mutex::new(Vec::new()));
    let next_start = Arc::new(Mutex::new(Instant::now()));
    let workers: Vec<_> = (0..args.tables)
        .map(|_| {
            let (args, queue, results) = (args.clone(), queue.clone(), results.clone());
            let next_start = next_start.clone();
            std::thread::spawn(move || {
                loop {
                    let Some(game) = queue.lock().expect("queue").next() else {
                        return;
                    };
                    let result = play_paced(&args, &game, &next_start);
                    let mut results = results.lock().expect("results");
                    results.push(result);
                    let so_far: &Vec<Value> = &results;
                    let report = json!({"summary": summary(so_far), "games": so_far});
                    if let Err(error) = atomic_json(&args.root.join("ladder.json"), &report) {
                        eprintln!("[ladder4] could not write the results: {error:#}");
                    }
                    println!("[ladder4] {} games done", so_far.len());
                }
            })
        })
        .collect();
    for worker in workers {
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("a table panicked"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_plays_the_main_deck_alone() {
        let text = "# source\n[deck:x]\n4 Island\n\n[sideboard]\n3 Pyroblast\n";
        assert_eq!(main_deck(text), "Deck\n4 Island\n");
    }

    #[test]
    fn a_book_that_cannot_pay_a_whole_game_says_so() {
        let settings = json!({"caps": {"month_usd": 1.0, "month_tokens": 100.0}, "profiles": {
            "paid": {"game_usd": 0.4}, "free": {"game_tokens": 60}}});
        let book = json!({"games": [
            {"reserved_usd": 0.4, "spent_usd": 0.1, "settled": 5, "spent_tokens": 30},
            {"reserved_usd": 0.4, "reserved_tokens": 30},
        ]});
        // 0.1 spent and 0.4 held leave 0.5: one game, with 0.1 over.
        let paid = margin(&settings, &book, "paid").unwrap();
        assert!((paid - 0.1).abs() < 1e-9, "{paid}");
        // 30 spent and 30 held leave 40 of 100, short of a game of 60.
        assert!(margin(&settings, &book, "free").unwrap() < 0.0);
        assert_eq!(margin(&settings, &book, "house:sharp"), None);
    }

    #[test]
    fn open_games_count_as_the_mean_of_those_played() {
        let settings = json!({"caps": {"month_usd": 9.0}, "profiles": {"paid": {"game_usd": 2.0}}});
        let book = json!({"games": [
            {"reserved_usd": 2.0, "spent_usd": 0.2, "settled": 5},
            {"reserved_usd": 2.0, "spent_usd": 0.4, "settled": 6},
            {"reserved_usd": 2.0, "spent_usd": 0.0, "settled": 7},
            {"reserved_usd": 2.0},
        ]});
        let spend = likely_spend(&settings, &book, "paid").unwrap();
        assert!((spend - 0.9).abs() < 1e-9, "{spend}");
        // Before any game was played an open one counts in full.
        let open = json!({"games": [{"reserved_usd": 2.0}]});
        let spend = likely_spend(&settings, &open, "paid").unwrap();
        assert!((spend - 2.0).abs() < 1e-9, "{spend}");
    }
}
