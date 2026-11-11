//! A small ladder: the net in `seat4`, or the engine's own house, against a
//! language model in `baylee-seat`, on a private gateway. For numbers only:
//! these games are never training data, and the root says so in
//! `NOT-FOR-TRAINING`.
//!
//! A net arm: the net hosts every room and sits in chair 0, the language model
//! takes chair 1 as a guest. A house arm (`house=LEVEL`): the language model's
//! own guest account hosts the room and hands its chair 0 to the bridge
//! (`--host-chair`), and chair 1 is an AI chair of the room at LEVEL, played
//! by the house inside the game's engine as at any hosted table; a game's
//! `won` and `net_deck` are then the house's. A pair of decks is played twice
//! with the decks swapped. Keys are read from the environment by `baylee-seat`
//! alone; this program never touches them and starts each seat with only what
//! it needs.
// Counts of games, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
use anyhow::{Context, Result, bail, ensure};
use baylee_train_ops::evaluation::{atomic_json, read_json};
use clap::Parser;
use serde_json::{Value, json};
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "Plays seat4 or the house against language model seats, for numbers only")]
struct Args {
    /// Where results, transcripts and logs go.
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    gateway: String,
    /// The net's seat; every net arm needs it.
    #[arg(long)]
    seat4: Option<PathBuf>,
    #[arg(long)]
    seat: PathBuf,
    /// The net's model; every net arm needs it.
    #[arg(long)]
    model: Option<PathBuf>,
    /// The seat settings file of the profiles.
    #[arg(long)]
    profiles: PathBuf,
    /// The spend book of the run, which holds the file's caps.
    #[arg(long)]
    ledger: PathBuf,
    /// A deck manifest: `{"decks":[{"key","path"}]}`. A deck without a `key`
    /// goes by its `id`, as in `manifest.v4.json`.
    #[arg(long)]
    decks: PathBuf,
    /// Plays only the manifest's decks this file names, in the manifest's
    /// order: an `eval4` summary's `accepted_deck_keys`, so that the seat
    /// plays the decks an evaluation played.
    #[arg(long)]
    keep_decks: Option<PathBuf>,
    /// Where a deck's relative `path` is read from (a manifest's repository
    /// decks); without it, from the working directory.
    #[arg(long)]
    deck_root: Option<PathBuf>,
    /// `FIELD=VALUE`: plays only the manifest's decks whose FIELD says VALUE
    /// (`split=holdout2`, `format=constructed`); given again, every one holds.
    #[arg(long)]
    only: Vec<String>,
    /// `PROFILE:MODE:GAMES`, MODE being the net's `raw`, `tactical` (the layer
    /// with no rule group), `tactical=GROUPS` or `measured` (the groups the
    /// net is measured with, `seat4`'s default), or `house=LEVEL`: the house
    /// at a fair level a player can choose (`expert`), in the room's own AI
    /// chair.
    #[arg(long, required = true)]
    arm: Vec<String>,
    /// Games at once, over all arms.
    #[arg(long, default_value_t = 6)]
    tables: usize,
    #[arg(long, default_value_t = baylee_train_ops::measured::THINKING_STEPS)]
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
    /// The life a game starts at. A room left alone holds the gateway's default
    /// of 40 with a free mulligan, which the net never played against `sharp`.
    #[arg(long, default_value_t = 20)]
    starting_life: i32,
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

/// A deck's name in a manifest: its `key`, else its `id`.
fn deck_key(deck: &Value) -> &str {
    deck["key"]
        .as_str()
        .or_else(|| deck["id"].as_str())
        .unwrap_or("")
}

/// The `--only` conditions as `(field, value)`.
fn only_fields(only: &[String]) -> Result<Vec<(&str, &str)>> {
    only.iter()
        .map(|condition| {
            condition
                .split_once('=')
                .filter(|(field, value)| !field.is_empty() && !value.is_empty())
                .with_context(|| format!("--only is FIELD=VALUE, not {condition}"))
        })
        .collect()
}

/// The decks of `manifest` a ladder plays: those whose fields say what `only`
/// asks and that `keep` names (all without it), in the manifest's order, each
/// with its `key` and its `path` resolved against `root`.
fn ladder_decks(
    manifest: &Value,
    only: &[(&str, &str)],
    keep: Option<&Value>,
    root: Option<&Path>,
) -> Result<Vec<Value>> {
    let kept: Option<std::collections::BTreeSet<&str>> = keep
        .map(|keep| {
            keep["accepted_deck_keys"]
                .as_array()
                .context("the kept decks name no accepted_deck_keys")
                .map(|keys| keys.iter().filter_map(Value::as_str).collect())
        })
        .transpose()?;
    let mut decks = Vec::new();
    for deck in manifest["decks"]
        .as_array()
        .context("manifest has no decks")?
    {
        let key = deck_key(deck);
        if kept.as_ref().is_some_and(|kept| !kept.contains(key))
            || only
                .iter()
                .any(|(field, value)| deck[*field].as_str() != Some(*value))
        {
            continue;
        }
        let path = Path::new(deck["path"].as_str().context("deck path")?);
        let path = match root {
            Some(root) if path.is_relative() => root.join(path),
            _ => path.to_path_buf(),
        };
        decks.push(json!({"key": key, "path": path}));
    }
    if let Some(kept) = &kept {
        ensure!(
            decks.len() == kept.len(),
            "the manifest holds {} of the {} decks to keep",
            decks.len(),
            kept.len()
        );
    }
    Ok(decks)
}

fn schedule(args: &Args) -> Result<Vec<Game>> {
    let keep = args.keep_decks.as_deref().map(read_json).transpose()?;
    let decks = ladder_decks(
        &read_json(&args.decks)?,
        &only_fields(&args.only)?,
        keep.as_ref(),
        args.deck_root.as_deref(),
    )?;
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
        if let Host::Net(_) = host_of(mode)? {
            ensure!(
                args.seat4.is_some() && args.model.is_some(),
                "the net's arm {arm} needs --seat4 and --model"
            );
        }
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

/// Who plays the language model's opponent, by an arm's mode.
#[derive(Debug, PartialEq, Eq)]
enum Host<'a> {
    /// The net in `seat4`, told this.
    Net(Vec<String>),
    /// The house at this level, in the room's own AI chair.
    House(&'a str),
}

/// The opponent an arm's `mode` names. A house level is one a player can
/// choose and is fair: `seer` reads the model's hidden cards.
fn host_of(mode: &str) -> Result<Host<'_>> {
    let Some(level) = mode.strip_prefix("house=") else {
        return policy_args(mode).map(Host::Net);
    };
    let profile = baylee_core::preset::AIProfile::named(level)
        .with_context(|| format!("no house level is called {level}"))?;
    ensure!(
        profile.sight == baylee_core::preset::Sight::Fair,
        "the house at {level} reads hidden cards: no measurement"
    );
    Ok(Host::House(level))
}

/// What `seat4` is told to play for an arm's mode. Every mode says it in full,
/// so an arm means the same whatever `seat4` plays by default.
fn policy_args(mode: &str) -> Result<Vec<String>> {
    let groups = match mode {
        "raw" => return Ok(vec!["--raw-policy".to_owned()]),
        "tactical" => "",
        "measured" => baylee_train_ops::measured::RULE_GROUPS,
        other => other
            .strip_prefix("tactical=")
            .with_context(|| format!("unknown mode {other}"))?,
    };
    Ok(vec!["--tactics-rules".to_owned(), groups.to_owned()])
}

/// Spawns `seat4` as the room's host and reads its room from its first line.
fn host(
    args: &Args,
    game: &Game,
    policy: &[String],
    work: &Path,
) -> Result<(Child, String, BufReader<std::process::ChildStdout>)> {
    let mut command = Command::new(args.seat4.as_ref().context("a net arm needs --seat4")?);
    command
        .arg("--gateway")
        .arg(&args.gateway)
        .arg("--model")
        .arg(args.model.as_ref().context("a net arm needs --model")?)
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
        .arg(work.join("journal.jsonl.gz"))
        .arg("--starting-life")
        .arg(args.starting_life.to_string());
    if args.catalog_drift {
        command.arg("--catalog-drift");
    }
    command.args(policy);
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

/// Spawns the language model's seat: as a guest in chair 1 of the net's
/// room, or (`host_chair`) in chair 0 of a room its own account opened,
/// whose credentials then go to it as the first line on stdin. Its
/// environment is cleared to what it needs and its profile's key variable,
/// passed by name.
fn guest(args: &Args, game: &Game, room: &str, host_chair: bool, work: &Path) -> Result<Child> {
    let mut command = Command::new(&args.seat);
    command
        .arg("join")
        .arg(room)
        .arg("--gateway")
        .arg(&args.gateway)
        .arg("--chair")
        .arg(if host_chair { "0" } else { "1" })
        .arg("--tethered");
    if host_chair {
        command.arg("--host-chair");
    }
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
    // The host's chair is read for the line that says it is ready
    // ([`copy_until_ready`]); a guest's goes straight to its log.
    let stdout = if host_chair {
        Stdio::piped()
    } else {
        Stdio::from(std::fs::File::create(work.join("seat.stdout.log"))?)
    };
    command
        .current_dir(work)
        .stdin(Stdio::piped())
        .stdout(stdout)
        .stderr(std::fs::File::create(work.join("seat.stderr.log"))?)
        .spawn()
        .context("baylee-seat")
}

/// What the bridge prints once its mind answered its check and it said its
/// chair is ready (`baylee-seat`'s `ready_and_wait`). The room's listing
/// cannot say it: a host's own chair counts as ready there, since the host
/// consents by starting, so a room started before this line would be
/// playing when the bridge checks the chair it was handed, and it refuses.
const BRIDGE_READY: &str = "the chair is ready";

/// Copies the bridge's lines from `from` to `log`, raising `ready` at the
/// first that says [`BRIDGE_READY`], until the bridge closes its stdout.
fn copy_until_ready(
    from: impl std::io::Read,
    mut log: impl std::io::Write,
    ready: &std::sync::atomic::AtomicBool,
) {
    for line in BufReader::new(from).lines() {
        let Ok(line) = line else { break };
        if line.contains(BRIDGE_READY) {
            ready.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        let _ = writeln!(log, "{line}");
    }
    let _ = log.flush();
}

/// The private gateway, as the language model's own guest account sees it
/// when it hosts a house arm's room. Its session token goes only into a
/// request's `Authorization` and the bridge's stdin, never a log or an error.
struct Gateway {
    agent: ureq::Agent,
    base: String,
}

impl Gateway {
    fn new(base: &str) -> Self {
        // No redirect is followed: what the gateway is asked is asked of it.
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .new_agent();
        Self {
            agent,
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    /// `POST path` with `body` (`GET` without one): the answer's JSON, or why
    /// not, in the gateway's own words.
    fn call(&self, path: &str, token: Option<&str>, body: Option<&Value>) -> Result<Value> {
        let url = format!("{}{path}", self.base);
        let bearer = token.map(|token| format!("Bearer {token}"));
        let response = if let Some(body) = body {
            let mut request = self.agent.post(&url);
            if let Some(bearer) = &bearer {
                request = request.header("Authorization", bearer);
            }
            request.send_json(body)
        } else {
            let mut request = self.agent.get(&url);
            if let Some(bearer) = &bearer {
                request = request.header("Authorization", bearer);
            }
            request.call()
        };
        let mut response = response.with_context(|| format!("asking {path}"))?;
        let status = response.status();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let value = serde_json::from_str(&text).unwrap_or(Value::Null);
        ensure!(
            status.is_success(),
            "{path}: {status} {}",
            value["error"].as_str().unwrap_or("")
        );
        Ok(value)
    }

    /// Signs a guest in under `name`: its session token.
    fn guest(&self, name: &str) -> Result<String> {
        let body = json!({"display_name": name, "lang": "en"});
        let answer = self.call("/auth/guest", None, Some(&body))?;
        Ok(answer["token"]
            .as_str()
            .context("a guest without a token")?
            .to_owned())
    }

    /// Stores a main deck of `rows` under `token`: its id.
    fn upload(&self, token: &str, name: &str, rows: &[&str]) -> Result<String> {
        let body = json!({"name": name, "cards": rows, "sideboard": [], "commanders": []});
        let answer = self.call("/decks", Some(token), Some(&body))?;
        Ok(answer["deck_id"]
            .as_str()
            .context("a stored deck without an id")?
            .to_owned())
    }

    /// The room `game_id` as the listing shows it to `token`.
    fn room(&self, token: &str, game_id: &str) -> Result<Option<Value>> {
        let page = self.call(
            &format!("/lobby/games?q={game_id}&limit=50"),
            Some(token),
            None,
        )?;
        Ok(page["games"]
            .as_array()
            .and_then(|games| games.iter().find(|room| room["id"] == game_id))
            .cloned())
    }
}

/// The house arm's room before its game: opened by the model's own account
/// with the model's deck in chair 0, set to the ladder's rules, chair 1 the
/// house at `level` with the house's deck. The account's session token and
/// chair 0's seat token, which the bridge is handed.
struct HouseRoom {
    game_id: String,
    session_token: String,
    seat_token: String,
}

/// The ladder's rules for a room: `life` each, no free mulligan, two chairs.
fn ladder_rules(name: &str, life: i32) -> baylee_core::preset::RoomUpdate {
    baylee_core::preset::RoomUpdate {
        name: name.to_owned(),
        chairs: 2,
        password: None,
        setup: baylee_core::preset::RoomSetup {
            starting_life: life,
            free_mulligans: 0,
            ..baylee_core::preset::RoomSetup::default()
        },
    }
}

/// The name the model's account sits under: what plays it, then the game.
fn chair_name(profile: &str, index: usize) -> String {
    let prefix = if profile.starts_with("house:") {
        "HOUSE-"
    } else {
        "LLM-"
    };
    format!("{prefix}l{index}")
}

/// Opens a house arm's room ([`HouseRoom`]). The house's chair is arranged
/// before the bridge has the model's: arranging takes every chair's ready
/// back, so the room is startable only once the bridge has said ready.
fn open_house_room(gateway: &Gateway, args: &Args, game: &Game, level: &str) -> Result<HouseRoom> {
    let token = gateway.guest(&chair_name(&game.profile, game.index))?;
    let llm_text = std::fs::read_to_string(&game.llm_deck)?;
    let house_text = std::fs::read_to_string(&game.net_deck)?;
    let llm_deck = gateway.upload(&token, "llm-deck", &main_rows(&llm_text))?;
    let house_deck = gateway.upload(&token, "house-deck", &main_rows(&house_text))?;
    let name = format!("Ladder-{}", game.index);
    let opened = gateway.call(
        "/lobby/games",
        Some(&token),
        Some(&json!({"deck_id": llm_deck, "mode": "open", "seats": 2, "name": name})),
    )?;
    let game_id = opened["game_id"].as_str().context("room id")?.to_owned();
    ensure!(opened["seat"] == 0, "the host did not get chair 0");
    let seat_token = opened["seat_token"]
        .as_str()
        .context("no seat token")?
        .to_owned();
    let rules = serde_json::to_value(ladder_rules(&name, args.starting_life))?;
    gateway.call(
        &format!("/lobby/games/{game_id}/configure"),
        Some(&token),
        Some(&rules),
    )?;
    gateway.call(
        &format!("/lobby/games/{game_id}/seats/1"),
        Some(&token),
        Some(&json!({"kind": "ai", "ai": level, "deck_id": house_deck})),
    )?;
    Ok(HouseRoom {
        game_id,
        session_token: token,
        seat_token,
    })
}

/// The model's seat summary at the end of its transcript, from whichever chair.
fn model_summary(work: &Path, room: &str) -> Option<Value> {
    let entries = std::fs::read_dir(work.join("transcripts")).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let lines = path.extension().is_some_and(|ext| ext == "jsonl");
        if lines && name.starts_with(room) && !name.ends_with("-mind.jsonl") {
            let text = std::fs::read_to_string(&path).ok()?;
            let last: Value = serde_json::from_str(text.lines().next_back()?).ok()?;
            return Some(last["summary"].clone()).filter(|summary| !summary.is_null());
        }
    }
    None
}

/// A house arm's game: the model's own account hosts, the bridge takes its
/// chair 0, the house plays chair 1 inside the engine, and the room starts
/// once the bridge said ready. `won` is the house's.
fn play_house(args: &Args, game: &Game, level: &str, work: &Path) -> Result<Value> {
    let started = Instant::now();
    let gateway = Gateway::new(&args.gateway);
    let room = open_house_room(&gateway, args, game, level)?;
    let mut model = guest(args, game, &room.game_id, true, work)?;
    let handoff = json!({
        "game_id": room.game_id, "seat": 0,
        "seat_token": room.seat_token, "session_token": room.session_token,
    });
    let ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let copier = {
        let from = model.stdout.take().context("the bridge's stdout")?;
        let log = std::fs::File::create(work.join("seat.stdout.log"))?;
        let ready = ready.clone();
        std::thread::spawn(move || copy_until_ready(from, log, &ready))
    };
    let handed = model
        .stdin
        .as_mut()
        .context("the bridge's stdin")?
        .write_all(format!("{handoff}\n").as_bytes());
    let limit = Duration::from_secs(args.game_timeout_secs);
    let (mut begun, mut timed_out) = (false, false);
    let mut fault = handed
        .err()
        .map(|error| format!("handing the chair over: {error}"));
    while fault.is_none() && matches!(model.try_wait(), Ok(None)) {
        if started.elapsed() > limit {
            timed_out = true;
            break;
        }
        if !begun && ready.load(std::sync::atomic::Ordering::SeqCst) {
            match gateway.room(&room.session_token, &room.game_id) {
                Ok(Some(listed)) if listed["startable"] == true => {
                    match gateway.call(
                        &format!("/lobby/games/{}/start", room.game_id),
                        Some(&room.session_token),
                        Some(&json!({})),
                    ) {
                        Ok(_) => begun = true,
                        Err(error) => fault = Some(format!("{error:#}")),
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => fault = Some("the room is gone before its game".to_owned()),
                Err(error) => fault = Some(format!("{error:#}")),
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    // The bridge ends by itself after the game, its book settled; the tether
    // closing ends it where it stands otherwise.
    drop(model.stdin.take());
    let waited = Instant::now();
    while matches!(model.try_wait(), Ok(None)) && waited.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(200));
    }
    reap(&mut model);
    // The bridge is gone, so its stdout is closed and the copy ends.
    let _ = copier.join();
    if !begun {
        // Best effort: the only chair holder leaving closes the room.
        let _ = gateway.call(
            &format!("/lobby/games/{}/leave", room.game_id),
            Some(&room.session_token),
            Some(&json!({})),
        );
        bail!(
            "the game never began: {}",
            fault.as_deref().unwrap_or("the seat ended first")
        );
    }
    let summary = model_summary(work, &room.game_id);
    let outcome = summary
        .as_ref()
        .and_then(|summary| summary["outcome"].as_str())
        .map(str::to_owned);
    Ok(json!({
        "game": game.index, "room": room.game_id, "profile": game.profile, "mode": game.mode,
        "net_deck": game.net_key, "llm_deck": game.llm_key,
        "seconds": started.elapsed().as_secs_f64(), "timed_out": timed_out,
        "phase": outcome.as_ref().map(|_| "game-complete"),
        "won": outcome.as_deref().map(|outcome| outcome == "lost"),
        "drawn": outcome.as_deref().map(|outcome| outcome == "draw"),
        "llm_outcome": outcome, "failure": fault,
        "answers": summary.as_ref().map(|summary| summary["answered"]["mind"].clone()),
        "fallback_answers": summary.as_ref().map(|summary| summary["answered"]["house"].clone()),
    }))
}

/// Stops a child this program started, if it still runs.
fn reap(child: &mut Child) {
    if matches!(child.try_wait(), Ok(None)) {
        let _ = child.kill();
    }
    let _ = child.wait();
}

/// A corpus deck's main deck: its count lines. A game has no sideboard, so
/// nothing from `[sideboard]` on is played.
fn main_rows(text: &str) -> Vec<&str> {
    text.lines()
        .take_while(|line| !line.trim().to_ascii_lowercase().starts_with("[sideboard"))
        .filter(|line| line.starts_with(|first: char| first.is_ascii_digit()))
        .collect()
}

/// A corpus deck's main deck as the seats read one: its count lines under
/// `Deck`.
fn main_deck(text: &str) -> String {
    let mut plain = String::from("Deck\n");
    for line in main_rows(text) {
        plain.push_str(line);
        plain.push('\n');
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
    let policy = match host_of(&game.mode)? {
        Host::House(level) => return play_house(args, game, level, &work),
        Host::Net(policy) => policy,
    };
    let (mut net, room, mut lines) = host(args, game, &policy, &work)?;
    let mut model = guest(args, game, &room, false, &work)?;
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
        "rules_fired": inner.get("tactic_rules_fired"),
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

    /// The seat plays the decks an evaluation played: a `manifest.v4.json`'s
    /// decks by id, those an `eval4` summary accepted, repository paths read
    /// from the repository.
    #[test]
    fn a_ladder_keeps_the_decks_an_evaluation_accepted() {
        let manifest = json!({"decks": [
            {"id": "eval/a", "path": "data/decks/eval/a.txt", "split": "test"},
            {"id": "x", "path": "/decks/x.txt", "split": "train"},
            {"id": "y", "path": "/decks/y.txt", "split": "test"},
        ]});
        let keep = json!({"accepted_deck_keys": ["y", "eval/a"]});
        let decks = ladder_decks(&manifest, &[], Some(&keep), Some(Path::new("/repo"))).unwrap();
        assert_eq!(
            decks,
            [
                json!({"key": "eval/a", "path": "/repo/data/decks/eval/a.txt"}),
                json!({"key": "y", "path": "/decks/y.txt"}),
            ]
        );
        assert_eq!(ladder_decks(&manifest, &[], None, None).unwrap().len(), 3);
        let missing = json!({"accepted_deck_keys": ["y", "gone"]});
        assert!(ladder_decks(&manifest, &[], Some(&missing), None).is_err());
        let keyed = json!({"decks": [{"key": "k", "id": "i", "path": "/k.txt"}]});
        assert_eq!(
            ladder_decks(&keyed, &[], None, None).unwrap()[0]["key"],
            "k"
        );
    }

    /// `--only` plays the decks whose fields all say what it asks, in the
    /// manifest's order: one split's constructed decks of a mixed manifest.
    #[test]
    fn a_ladder_plays_only_the_decks_it_names_by_their_fields() {
        let manifest = json!({"decks": [
            {"id": "a", "path": "/a.txt", "split": "holdout2", "format": "constructed"},
            {"id": "b", "path": "/b.txt", "split": "holdout2", "format": "commander"},
            {"id": "c", "path": "/c.txt", "split": "train", "format": "constructed"},
            {"id": "d", "path": "/d.txt", "split": "holdout2", "format": "constructed"},
            {"id": "e", "path": "/e.txt"},
        ]});
        let only = ["split=holdout2".to_owned(), "format=constructed".to_owned()];
        let fields = only_fields(&only).unwrap();
        let keys: Vec<Value> = ladder_decks(&manifest, &fields, None, None)
            .unwrap()
            .into_iter()
            .map(|deck| deck["key"].clone())
            .collect();
        assert_eq!(keys, [json!("a"), json!("d")]);
        assert!(only_fields(&["split".to_owned()]).is_err());
        assert!(only_fields(&["=x".to_owned()]).is_err());
        assert!(only_fields(&["split=".to_owned()]).is_err());
    }

    /// `house=LEVEL` seats the house at a level a player can choose, a fair
    /// one; every other mode is the net's, told in full.
    #[test]
    fn a_house_arm_names_a_fair_level_a_player_can_choose() {
        assert_eq!(host_of("house=expert").unwrap(), Host::House("expert"));
        assert_eq!(host_of("house=sharp").unwrap(), Host::House("sharp"));
        assert!(host_of("house=seer").is_err(), "seer reads hidden cards");
        assert!(host_of("house=sharp-2026-11").is_err(), "no lobby level");
        assert!(host_of("house=").is_err());
        assert_eq!(
            host_of("raw").unwrap(),
            Host::Net(vec!["--raw-policy".to_owned()])
        );
    }

    /// A house arm needs no net, and a net arm refuses to start without one.
    #[test]
    fn only_a_net_arm_needs_the_net() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("decks.json");
        std::fs::write(
            &manifest,
            json!({"decks": [{"id": "a", "path": "/a.txt"}, {"id": "b", "path": "/b.txt"}]})
                .to_string(),
        )
        .unwrap();
        let parse = |arm: &str| {
            Args::try_parse_from([
                "ladder4",
                "--root",
                "r",
                "--gateway",
                "g",
                "--seat",
                "s",
                "--profiles",
                "p",
                "--ledger",
                "l",
                "--decks",
                manifest.to_str().unwrap(),
                "--arm",
                arm,
            ])
            .unwrap()
        };
        let house = schedule(&parse("deepseek-low:house=expert:4")).unwrap();
        assert_eq!(house.len(), 4);
        assert_eq!(
            (house[0].net_key.as_str(), house[1].net_key.as_str()),
            ("a", "b"),
            "the decks swap within a pair"
        );
        assert!(schedule(&parse("deepseek-low:measured:2")).is_err());
    }

    /// The room starts only after the bridge said its chair is ready, and
    /// every line it printed reaches the log whatever it says.
    #[test]
    fn the_bridge_is_ready_when_it_says_so() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ready = AtomicBool::new(false);
        let mut log = Vec::new();
        copy_until_ready(
            "«LLM-l0» sits in chair 0 of room r with llm-deck; waiting for the host to start\n"
                .as_bytes(),
            &mut log,
            &ready,
        );
        assert!(!ready.load(Ordering::SeqCst), "seated is not ready");
        let said = "sits\nthe mind answered its check: the chair is ready\nthe game is on\n";
        let mut log = Vec::new();
        copy_until_ready(said.as_bytes(), &mut log, &ready);
        assert!(ready.load(Ordering::SeqCst));
        assert_eq!(String::from_utf8(log).unwrap(), said);
    }

    /// The model's account says what plays it, in a name the gateway takes.
    #[test]
    fn the_model_account_is_named_for_its_mind() {
        assert_eq!(chair_name("deepseek-low", 7), "LLM-l7");
        assert_eq!(chair_name("house:sharp", 12), "HOUSE-l12");
    }

    /// The ladder's room: twenty life, no free mulligan, two chairs, the rest
    /// as a room is by default.
    #[test]
    fn a_house_room_plays_the_ladders_rules() {
        let rules = serde_json::to_value(ladder_rules("Ladder-3", 20)).unwrap();
        assert_eq!(rules["chairs"], 2);
        assert_eq!(rules["setup"]["starting_life"], 20);
        assert_eq!(rules["setup"]["free_mulligans"], 0);
        assert_eq!(rules["name"], "Ladder-3");
        let back: baylee_core::preset::RoomUpdate = serde_json::from_value(rules).unwrap();
        assert_eq!(
            back.setup.format,
            baylee_core::preset::RoomSetup::default().format
        );
    }

    /// The summary at the end of the model's transcript is found whichever
    /// chair it sat in, and the reasoning file beside it is not read.
    #[test]
    fn the_model_summary_is_read_from_its_own_chair() {
        let dir = tempfile::tempdir().unwrap();
        let transcripts = dir.path().join("transcripts");
        std::fs::create_dir_all(&transcripts).unwrap();
        std::fs::write(transcripts.join("r1-seat0-mind.jsonl"), "{\"summary\":1}\n").unwrap();
        std::fs::write(
            transcripts.join("r1-seat0.jsonl"),
            "{\"event\":\"over\"}\n{\"summary\":{\"outcome\":\"lost\"}}\n",
        )
        .unwrap();
        assert_eq!(model_summary(dir.path(), "r1").unwrap()["outcome"], "lost");
        assert!(model_summary(dir.path(), "r2").is_none());
    }

    /// `measured` passes what `seat4` plays by default, at the depth it plays
    /// by default; every other mode says its groups in full.
    #[test]
    fn an_arm_tells_the_seat_what_to_play_in_full() {
        use baylee_train_ops::measured::{RULE_GROUPS, THINKING_STEPS};
        assert_eq!(
            policy_args("measured").unwrap(),
            ["--tactics-rules", RULE_GROUPS]
        );
        assert_eq!(policy_args("tactical").unwrap(), ["--tactics-rules", ""]);
        assert_eq!(
            policy_args("tactical=mass,react").unwrap(),
            ["--tactics-rules", "mass,react"]
        );
        assert_eq!(policy_args("raw").unwrap(), ["--raw-policy"]);
        assert!(policy_args("bundle").is_err());
        let args = Args::try_parse_from([
            "ladder4",
            "--root",
            "r",
            "--gateway",
            "g",
            "--seat4",
            "s4",
            "--seat",
            "s",
            "--model",
            "m",
            "--profiles",
            "p",
            "--ledger",
            "l",
            "--decks",
            "d",
            "--arm",
            "x:measured:2",
        ])
        .unwrap();
        assert_eq!(args.thinking_steps, THINKING_STEPS);
    }

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
