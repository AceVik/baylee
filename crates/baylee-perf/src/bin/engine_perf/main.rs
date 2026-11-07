//! A fixed set of seeded games to measure the engine by, and to prove that a
//! change played them the same (`docs/perf-baseline.md`).
//!
//! ```text
//! cargo build --profile selfplay -p baylee-perf --features simulation
//! valgrind --tool=callgrind --toggle-collect='engine_perf::play_all' \
//!     target/selfplay/engine_perf house
//! target/selfplay/engine_perf house --verify > before.txt
//! ```
//!
//! Four shapes, each over the same tables:
//!
//! - `house`: recorded games between house agents, the self-play runner's
//!   path, which hashes the game after every answer.
//! - `frames`: one chair answers over a socket and the house plays the other,
//!   the way a live table runs: every answer builds and serializes that
//!   seat's frames. A profile for a shipped binary trains on this one.
//! - `decide`: both chairs driven from outside the way `eval4` drives a net's
//!   chair. Every seat observes the table, the asked seat is given its
//!   question and view, the game is hashed, and a house answer stands in for
//!   the net's.
//! - `search` (feature `simulation`): one chair driven, and at each of its
//!   main-phase priorities a few worlds forked and rolled out, the shape of
//!   `netplay4::search`.
//!
//! This crate is a bin and not an example because a release builds it beside
//! the binaries it ships, in the same `cargo` run: an example would bring its
//! crate's dev-dependencies in, and with them `simulation`, which must not
//! reach a hosting build.
//!
//! Count instructions, not seconds, unless the machine is quiet. `--verify`
//! prints a line of digests per game: every action with its seat, every view
//! and question as JSON, every state hash. It costs far more than the game, so
//! it is a separate run and is not what is counted. Two builds that print the
//! same lines played the same games and showed their seats the same things;
//! `hashes` alone may move, and only where a commit says it moved them.
//!
//! `--game <id>` plays one game of the set, as one engine process does, and
//! `--rss` (`frames` only) writes the resident set to stderr as it goes, for
//! comparing allocators (Linux).
//!
//! `--verify --patches` rebuilds every seat's view from patches as the game
//! goes, on any shape, and stops at the first view that does not come out byte
//! for byte the whole view (`rebuild.rs`).
//!
//! Three more runs are not for counting instructions but for what goes down a
//! socket (`wire.rs`): `wire` is `frames` as a live engine drives it, pumping
//! `LIVE_SLICE` house steps at a time with the clock running; `table` is the
//! same with every chair on a socket, and `--four` seats four at each table;
//! `record <file>…` plays recorded games again and reads what each seat of
//! them is sent. All three print bytes and frames per seat and message kind.

mod fuzz;
mod rebuild;
mod wire;

use std::collections::BTreeMap;
use std::hint::black_box;
use std::ops::ControlFlow;
use std::path::Path;

use anyhow::{Context as _, bail};
use baylee_cards::decks::{LoadedDeck, house_deck, preset_for_all};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
#[cfg(feature = "simulation")]
use baylee_engine::engine::DeckKnowledge;
#[cfg(feature = "simulation")]
use baylee_gamehost::SimulationSession;
use baylee_gamehost::record::Line;
use baylee_gamehost::replay::{Perspective, Verify, project};
use baylee_gamehost::{PlayerView, Session};
use baylee_protocol::v1::Envelope;
use baylee_protocol::v1::envelope::Msg;
use baylee_view::ViewReader;
use prost::Message as _;

use rebuild::Rebuild;
use wire::Wire;

/// The decks at each table and the profiles playing them. Mirrors of nothing:
/// every house deck sits down, so an artifact board, a token board, a
/// commander and a combo-less midrange deck are all in the count.
const TABLES: [(&str, &str, &str, &str); 8] = [
    ("allytifact", "victory", "steady", "sharp"),
    ("breya", "kess", "sharp", "steady"),
    ("kenrith", "tayam", "expert", "sharp"),
    ("maik", "schwarzrand", "steady", "expert"),
    ("weltenbaum", "victory", "sharp", "casual"),
    ("kess", "allytifact", "casual", "steady"),
    ("tayam", "maik", "sharp", "sharp"),
    ("schwarzrand", "breya", "expert", "steady"),
];

/// Seeds each table is dealt with.
const SEEDS: [u64; 2] = [11, 12];

/// Answers a game may take before it is stopped, every seat's together.
const MAX_ANSWERS: u64 = 3000;

/// Answers between two `--rss` samples.
const RSS_EVERY: u64 = 100;

/// House steps a live engine runs before it looks at its sockets again: its
/// `ATTACHED_PUMP_SLICE`, whose crate this one does not link.
const LIVE_SLICE: usize = 128;

/// How long a seat takes over an answer in `wire` and `table`, so that the
/// clocks in their views count as they do at a table.
const THINKING_MS: u64 = 1500;

/// Worlds forked at a searched decision.
#[cfg(feature = "simulation")]
const WORLDS: u64 = 4;

/// Actions a world is rolled out for.
#[cfg(feature = "simulation")]
const HORIZON: u64 = 24;

/// Which of the searching seat's main-phase priorities are searched: the
/// first of each `SEARCH_EVERY`.
#[cfg(feature = "simulation")]
const SEARCH_EVERY: u64 = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    House,
    Frames,
    Decide,
    #[cfg(feature = "simulation")]
    Search,
    /// One chair on a socket, as a live engine drives it.
    Wire,
    /// Every chair on a socket, as a live engine drives it.
    Table,
}

/// What a run checks as it goes.
#[derive(Clone, Copy)]
struct Checks {
    /// Digest every action, view and hash.
    verify: bool,
    /// Rebuild every seat's view from patches.
    patches: bool,
}

/// One game of the fixed set.
struct Game {
    id: String,
    preset: GamePreset,
}

/// FNV-1a over everything fed to it: cross-build equality wants a function
/// that is the same everywhere and owes nothing to the code under test.
struct Fnv(u64);

impl Fnv {
    const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// `value` as JSON with every entry at its default left out: what two builds
/// that disagree on writing a default still agree on.
fn canonical<T: serde::Serialize + ?Sized>(value: &T) -> Vec<u8> {
    fn strip(value: &mut serde_json::Value) {
        use serde_json::Value;
        match value {
            Value::Object(map) => {
                map.values_mut().for_each(strip);
                map.retain(|_, value| match value {
                    Value::Null | Value::Bool(false) => false,
                    Value::Number(number) => number.as_u64() != Some(0),
                    Value::Array(list) => !list.is_empty(),
                    Value::Object(map) => !map.is_empty(),
                    _ => true,
                });
            }
            Value::Array(list) => list.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut json = serde_json::to_value(value).expect("a value serializes");
    strip(&mut json);
    serde_json::to_vec(&json).expect("a value writes")
}

/// What a `--verify` run compares between builds.
struct Digests {
    on: bool,
    actions: Fnv,
    views: Fnv,
    hashes: Fnv,
    /// What each seat's socket has been left with by its frames.
    readers: BTreeMap<PlayerId, ViewReader>,
    /// `StateDelta`s read.
    frames_read: u64,
    /// Seats' views as a host was handed them, rebuilt from patches.
    rebuilt: Option<Rebuild>,
}

impl Digests {
    fn new(checks: Checks) -> Self {
        Self {
            on: checks.verify,
            actions: Fnv::new(),
            views: Fnv::new(),
            hashes: Fnv::new(),
            readers: BTreeMap::new(),
            frames_read: 0,
            rebuilt: checks.patches.then(Rebuild::default),
        }
    }

    fn view(&mut self, view: &PlayerView) {
        if self.on {
            self.views.feed(&canonical(view));
        } else {
            black_box(view);
        }
        if let Some(handed) = &mut self.rebuilt {
            handed.view(view);
        }
    }

    /// Views rebuilt from patches and found equal, so far.
    fn patched(&self) -> u64 {
        self.rebuilt
            .as_ref()
            .map_or(0, |handed| handed.views + self.frames_read)
    }

    /// As JSON and not `Debug`: this is what a seat is asked, and a field a
    /// question gains without saying anything new prints in `Debug` and stays
    /// off the wire.
    fn question(&mut self, pending: &Pending) {
        if self.on {
            self.views.feed(&canonical(pending));
        }
    }

    fn action(&mut self, seat: PlayerId, action: &PlayerAction) {
        if self.on {
            self.actions.feed(&[seat.get()]);
            self.actions.feed(format!("{action:?}").as_bytes());
        }
    }

    fn hash(&mut self, hash: u64) {
        if self.on {
            self.hashes.feed(&hash.to_le_bytes());
        } else {
            black_box(hash);
        }
    }

    /// What a socket is sent: every frame, and of a `StateDelta` the view a
    /// reader is left with, as JSON. Whatever goes down the wire, these bytes
    /// are the view, so two builds that encode it differently still compare.
    fn frames(&mut self, frames: &[(PlayerId, Envelope)]) {
        if !self.on {
            black_box(frames);
            return;
        }
        for (seat, frame) in frames {
            self.views.feed(&[seat.get()]);
            match &frame.msg {
                Some(Msg::GameStatic(statics)) => {
                    // Not its version: that moves when the wire does.
                    let mut statics: baylee_view::GameStatic =
                        baylee_core::wire::decode(&statics.statics).expect("a payload reads");
                    statics.view_version = 0;
                    self.views.feed(&canonical(&statics));
                }
                Some(Msg::StateDelta(delta)) => {
                    let view = self
                        .readers
                        .entry(*seat)
                        .or_default()
                        .read(&delta.view_frame)
                        .expect("a seat's frame places")
                        .expect("a seat holds a view");
                    self.views.feed(&delta.seq.to_le_bytes());
                    self.views.feed(&canonical(view));
                    if !delta.log.is_empty() {
                        let log: baylee_view::LogTail =
                            baylee_core::wire::decode(&delta.log).expect("a log reads");
                        self.views.feed(&canonical(&log));
                    }
                    self.frames_read += 1;
                }
                Some(Msg::ChoiceRequest(choice)) => {
                    let pending: Pending =
                        baylee_core::wire::decode(&choice.pending).expect("a question reads");
                    self.views.feed(&choice.seq.to_le_bytes());
                    self.views.feed(&canonical(&pending));
                }
                _ => self.views.feed(&frame.encode_to_vec()),
            }
        }
    }

    /// A record's inputs and hashes, line by line, less the header's build.
    fn record(&mut self, record: &[u8]) {
        for line in record.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            match serde_json::from_slice::<Line>(line).expect("a record line parses") {
                Line::Header { hash, seen, .. } => {
                    self.hashes.feed(hash.as_bytes());
                    self.hashes.feed(seen.unwrap_or_default().as_bytes());
                }
                Line::Input {
                    n,
                    seat,
                    by,
                    action,
                    hash,
                    seen,
                    ..
                } => {
                    self.actions
                        .feed(format!("{n} {seat} {by:?} {action:?}").as_bytes());
                    self.hashes.feed(hash.as_bytes());
                    self.hashes.feed(seen.unwrap_or_default().as_bytes());
                }
                other => self.actions.feed(format!("{other:?}").as_bytes()),
            }
        }
    }
}

/// What one game came to.
struct Played {
    answers: u64,
    turn: u32,
    ending: String,
    forks: u64,
    rollout_steps: u64,
    digests: Digests,
    /// What went down its sockets, for `wire` and `table`.
    wire: Option<Wire>,
}

fn played(session: &Session, digests: Digests, forks: u64, rollout_steps: u64) -> Played {
    let ending = match session.pending() {
        Pending::GameOver(result) => format!(
            "over:{:?}:{:?}",
            session.winning_seats(*result),
            result.reason
        ),
        _ => "cap".to_owned(),
    };
    Played {
        answers: session.decision_seq(),
        turn: session.state().turn.number,
        ending,
        forks,
        rollout_steps,
        digests,
        wire: None,
    }
}

fn seats_of(game: &Game) -> Vec<PlayerId> {
    (0..game.preset.seats.len())
        .map(|seat| PlayerId::new(u8::try_from(seat).expect("a table has few seats")))
        .collect()
}

fn describe(session: &mut Session, game: &Game) {
    let chairs = game.preset.seats.len();
    session.describe(
        game.id.clone(),
        (0..chairs).map(|s| s.to_string()).collect(),
    );
}

fn running(session: &Session) -> bool {
    !matches!(session.pending(), Pending::GameOver(_)) && session.decision_seq() < MAX_ANSWERS
}

fn house(game: &Game, checks: Checks) -> Played {
    let mut session = Session::new_recorded(&game.preset, "perf").expect("the preset builds");
    describe(&mut session, game);
    let seats = seats_of(game);
    let mut digests = Digests::new(checks);
    while running(&session) {
        session.pump_at_most(1);
        if checks.verify {
            for &seat in &seats {
                digests.view(&session.observe_view_with_announcement_context(seat));
            }
            digests.question(session.pending());
        }
    }
    if checks.verify {
        digests.record(&session.take_record());
    }
    played(&session, digests, 0, 0)
}

/// Answers `seat`'s standing question at `session`, as `eval4` does: the
/// house, then the answer that does nothing, then a pass. `false` when none
/// was taken, which ends the game there.
fn answer(session: &mut Session, seat: PlayerId, pending: &Pending, digests: &mut Digests) -> bool {
    let house = session.house_action(seat);
    let fallback = timeout_answer(pending);
    let tries = [house, fallback, Some(PlayerAction::PassPriority)];
    tries.into_iter().flatten().any(|action| {
        digests.action(seat, &action);
        session.act_slice(seat, action, 0).is_ok_and(|frames| {
            digests.frames(&frames);
            true
        })
    })
}

/// Writes this process's resident set and its peak so far to stderr, in KiB, off
/// `/proc/self/status`: stderr, so that `--verify` output stays what it is.
fn report_rss(game: &str, answers: u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let kib = |field: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(field))
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or("0")
            .to_owned()
    };
    eprintln!(
        "rss {game} answers={answers} kib={} peak={}",
        kib("VmRSS:"),
        kib("VmHWM:")
    );
}

fn frames(game: &Game, checks: Checks, rss: bool) -> Played {
    let mut session = Session::new_recorded(&game.preset, "perf").expect("the preset builds");
    describe(&mut session, game);
    let seat = PlayerId::new(0);
    session.take_over(seat);
    let mut digests = Digests::new(checks);
    let mut next_sample = 0;
    while running(&session) {
        if rss && session.decision_seq() >= next_sample {
            report_rss(&game.id, session.decision_seq());
            next_sample = session.decision_seq() + RSS_EVERY;
        }
        digests.frames(&session.pump_at_most(1));
        if session.needs_pump() {
            continue;
        }
        let Some((pending, view)) = session.view_for(seat) else {
            break;
        };
        digests.question(&pending);
        digests.view(&view);
        if !answer(&mut session, seat, &pending, &mut digests) {
            break;
        }
    }
    if rss {
        report_rss(&game.id, session.decision_seq());
    }
    if checks.verify {
        digests.record(&session.take_record());
    }
    played(&session, digests, 0, 0)
}

fn decide(game: &Game, checks: Checks) -> Played {
    let mut session = Session::new(&game.preset).expect("the preset builds");
    describe(&mut session, game);
    session.discard_frames();
    let seats = seats_of(game);
    for &seat in &seats {
        session.take_over(seat);
    }
    let mut digests = Digests::new(checks);
    'game: while running(&session) {
        for &seat in &seats {
            digests.view(&session.observe_view_with_announcement_context(seat));
        }
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        for &seat in &seats {
            let Some((pending, view)) = session.view_for_with_announcement_context(seat) else {
                continue;
            };
            digests.question(&pending);
            digests.view(&view);
            digests.hash(session.snapshot_hash());
            if !answer(&mut session, seat, &pending, &mut digests) {
                break 'game;
            }
            break;
        }
    }
    played(&session, digests, 0, 0)
}

/// Rolls `fork` on for at most [`HORIZON`] actions as `netplay4::search`
/// does; the world's steps.
#[cfg(feature = "simulation")]
fn rollout(fork: &mut SimulationSession, seat: PlayerId, digests: &mut Digests) -> u64 {
    for step in 0..HORIZON {
        if matches!(fork.pending(), Pending::GameOver(_)) {
            return step;
        }
        digests.view(&fork.observe_view_with_announcement_context(seat));
        if fork.needs_pump() {
            fork.pump_at_most(1);
            continue;
        }
        let Some((pending, view)) = fork.view_for_with_announcement_context(seat) else {
            return step;
        };
        digests.question(&pending);
        digests.view(&view);
        let Some(action) = fork.house_action(seat) else {
            return step;
        };
        digests.action(seat, &action);
        if fork.act_slice(seat, action, 0).is_err() {
            return step;
        }
    }
    HORIZON
}

#[cfg(feature = "simulation")]
fn search(game: &Game, checks: Checks) -> Played {
    let mut session = Session::new(&game.preset).expect("the preset builds");
    describe(&mut session, game);
    session.discard_frames();
    let seat = PlayerId::new(0);
    session.take_over(seat);
    let mut digests = Digests::new(checks);
    let (mut priorities, mut forks, mut rollout_steps) = (0_u64, 0_u64, 0_u64);
    while running(&session) {
        digests.view(&session.observe_view_with_announcement_context(seat));
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let Some((pending, view)) = session.view_for_with_announcement_context(seat) else {
            break;
        };
        digests.question(&pending);
        digests.view(&view);
        if matches!(pending, Pending::Priority { .. }) {
            if priorities.is_multiple_of(SEARCH_EVERY) {
                for world in 0..WORLDS {
                    let policy_seed = session.decision_seq().wrapping_mul(WORLDS) + world;
                    let Ok(mut fork) = session.fork_for(seat, DeckKnowledge::Known, policy_seed)
                    else {
                        continue;
                    };
                    forks += 1;
                    rollout_steps += rollout(&mut fork, seat, &mut digests);
                    digests.hash(fork.snapshot_hash());
                }
            }
            priorities += 1;
        }
        if !answer(&mut session, seat, &pending, &mut digests) {
            break;
        }
    }
    played(&session, digests, forks, rollout_steps)
}

/// Feeds what `seat` was sent to the tally and the digests.
fn told(wire: &mut Wire, digests: &mut Digests, frames: &[(PlayerId, Envelope)]) {
    for (seat, frame) in frames {
        wire.frame(*seat, frame);
    }
    digests.frames(frames);
}

/// `sockets` on sockets and the house at the rest of the table, driven as a
/// live engine drives one: `LIVE_SLICE` house steps at a time, each answer
/// through `act_slice`, the opening payload first and the clock running.
fn live(game: &Game, sockets: &[PlayerId], checks: Checks) -> Played {
    let mut session = Session::new(&game.preset).expect("the preset builds");
    describe(&mut session, game);
    for &seat in sockets {
        session.take_over(seat);
    }
    let mut digests = Digests::new(checks);
    let mut wire = Wire::default();
    let mut now = 0;
    for &seat in sockets {
        wire.frame(seat, &session.game_static_envelope(seat));
    }
    let opening = session.pump_slice(LIVE_SLICE);
    told(&mut wire, &mut digests, &opening);
    while running(&session) {
        now += THINKING_MS;
        session.tell_time(now);
        if session.needs_pump() {
            let frames = session.pump_slice(LIVE_SLICE);
            told(&mut wire, &mut digests, &frames);
            continue;
        }
        let asked = sockets
            .iter()
            .find_map(|&seat| session.view_for(seat).map(|(pending, _)| (seat, pending)));
        let Some((seat, pending)) = asked else {
            break;
        };
        let house = session.house_action(seat);
        let fallback = timeout_answer(&pending);
        let tries = [house, fallback, Some(PlayerAction::PassPriority)];
        let answered = tries.into_iter().flatten().any(|action| {
            digests.action(seat, &action);
            wire.corpus.action(&action);
            session
                .act_slice(seat, action, LIVE_SLICE)
                .is_ok_and(|frames| {
                    told(&mut wire, &mut digests, &frames);
                    true
                })
        });
        if !answered {
            break;
        }
    }
    let mut game_played = played(&session, digests, 0, 0);
    game_played.wire = Some(wire);
    game_played
}

/// What each seat of `record` is sent as it is played again, read as `wire`
/// reads a live table, checked against the record where it can be.
fn recorded(path: &Path, checks: Checks) -> anyhow::Result<(Wire, u64, u64)> {
    let record = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let header = record.split(|byte| *byte == b'\n').next().unwrap_or(&[]);
    let Line::Header { preset, .. } =
        serde_json::from_slice(header).context("a record's header")?
    else {
        bail!("{} opens with no header", path.display());
    };
    let seats = preset.seats.len();
    let names: Vec<String> = (0..seats).map(|seat| seat.to_string()).collect();
    let mut wire = Wire::default();
    let mut digests = Digests::new(checks);
    let (mut inputs, mut exact) = (0, 0);
    for seat in 0..seats {
        let seat = PlayerId::new(u8::try_from(seat).context("a table has few seats")?);
        let projection = project(
            &record,
            names.clone(),
            Perspective::Seat(seat),
            Verify::Stop,
            |moment| {
                let frames: Vec<_> = moment.frames.into_iter().map(|f| (seat, f)).collect();
                told(&mut wire, &mut digests, &frames);
                ControlFlow::Continue(())
            },
        )
        .map_err(|why| anyhow::anyhow!("{}: {why:?}", path.display()))?;
        inputs = projection.inputs;
        exact = projection.exact_to.or(projection.seen_to).unwrap_or(0);
        eprintln!(
            "{} seat {}: {} inputs, {:?}, exact to {:?}, seen to {:?}",
            path.display(),
            seat.get(),
            projection.inputs,
            projection.stop,
            projection.exact_to,
            projection.seen_to,
        );
    }
    println!(
        "{} seats={seats} inputs={inputs} checked={exact} patched={}",
        path.display(),
        digests.patched()
    );
    Ok((wire, inputs, exact))
}

/// Every game of the set, in order. Not inlined: callgrind counts from here
/// (`--toggle-collect`), which leaves the card registry and the deck files
/// out of the count.
#[inline(never)]
fn play_all(shape: Shape, games: &[Game], checks: Checks, rss: bool) -> Vec<Played> {
    games
        .iter()
        .map(|game| match shape {
            Shape::House => house(game, checks),
            Shape::Frames => frames(game, checks, rss),
            Shape::Decide => decide(game, checks),
            #[cfg(feature = "simulation")]
            Shape::Search => search(game, checks),
            Shape::Wire => live(game, &[PlayerId::new(0)], checks),
            Shape::Table => live(game, &seats_of(game), checks),
        })
        .collect()
}

/// The house deck `key`, from the checkout this was built in.
fn deck(key: &str) -> anyhow::Result<LoadedDeck> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/decks")
        .join(format!("{key}.txt"));
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    house_deck(&text)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("loading {}", path.display()))
}

/// The fixed set; with `four`, each table's two decks twice around it.
fn games(four: bool) -> anyhow::Result<Vec<Game>> {
    let mut games = Vec::new();
    for (a, b, profile_a, profile_b) in TABLES {
        let decks = [deck(a)?, deck(b)?];
        let profiles = [profile_a, profile_b]
            .map(|name| AIProfile::named(name).with_context(|| format!("profile {name}")));
        let [profile_a, profile_b] = profiles;
        let profiles = [profile_a?, profile_b?];
        for seed in SEEDS {
            let chairs: &[&LoadedDeck] = if four {
                &[&decks[0], &decks[1], &decks[0], &decks[1]]
            } else {
                &[&decks[0], &decks[1]]
            };
            let mut preset = preset_for_all(seed, chairs);
            for (seat, profile) in preset.seats.iter_mut().zip(profiles.iter().cycle()) {
                seat.controller = SeatController::Ai(*profile);
                seat.team = None;
            }
            games.push(Game {
                id: format!("{a}-{b}-{seed}"),
                preset,
            });
        }
    }
    Ok(games)
}

/// `fuzz [--rounds <n>] [--seed <n>]`: the frames of a few tables of four, damaged
/// and put to every decoder (`fuzz.rs`).
fn fuzz_decoders(args: &[String]) -> anyhow::Result<()> {
    let number = |name: &str, otherwise: u64| {
        args.iter()
            .skip_while(|arg| *arg != name)
            .nth(1)
            .and_then(|value| value.parse().ok())
            .unwrap_or(otherwise)
    };
    let checks = Checks {
        verify: false,
        patches: false,
    };
    let mut corpus = fuzz::Corpus::default();
    for game in games(true)?.iter().take(4) {
        if let Some(wire) = live(game, &seats_of(game), checks).wire {
            corpus.merge(wire.corpus);
        }
    }
    fuzz::run(&corpus, number("--rounds", 1_000_000), number("--seed", 1));
    Ok(())
}

/// The tally of recorded games: `record <file>…`.
fn records(paths: &[String], checks: Checks) -> anyhow::Result<()> {
    let mut all = Wire::default();
    let (mut inputs, mut checked) = (0, 0);
    for path in paths {
        let (wire, game_inputs, game_checked) = recorded(Path::new(path), checks)?;
        wire.seat_lines(path);
        all.merge(&wire);
        inputs += game_inputs;
        checked += game_checked;
    }
    println!(
        "total records={} inputs={inputs} checked={checked}",
        paths.len()
    );
    all.report();
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let checks = Checks {
        verify: args.iter().any(|arg| arg == "--verify"),
        patches: args.iter().any(|arg| arg == "--patches"),
    };
    let rss = args.iter().any(|arg| arg == "--rss");
    let four = args.iter().any(|arg| arg == "--four");
    let shape = match args.first().map(String::as_str) {
        Some("house") => Shape::House,
        Some("frames") => Shape::Frames,
        Some("decide") => Shape::Decide,
        #[cfg(feature = "simulation")]
        Some("search") => Shape::Search,
        Some("wire") => Shape::Wire,
        Some("table") => Shape::Table,
        Some("record") => {
            let paths: Vec<_> = args[1..]
                .iter()
                .filter(|arg| !arg.starts_with("--"))
                .cloned()
                .collect();
            return records(&paths, checks);
        }
        Some("fuzz") => return fuzz_decoders(&args),
        _ => {
            bail!(
                "usage: engine_perf house|frames|decide|search|wire|table [--verify] [--patches] \
                 [--rss] [--four] [--game <id>] | record <file>… [--patches] | \
                 fuzz [--rounds <n>] [--seed <n>]"
            )
        }
    };
    let mut games = games(four)?;
    if let Some(only) = args.iter().skip_while(|arg| *arg != "--game").nth(1) {
        games.retain(|game| game.id == *only);
        if games.is_empty() {
            bail!("no game {only} in the set");
        }
    }
    let played = play_all(shape, &games, checks, rss);
    let (mut answers, mut forks, mut steps) = (0, 0, 0);
    let mut all = Wire::default();
    for (game, game_played) in games.iter().zip(&played) {
        answers += game_played.answers;
        forks += game_played.forks;
        steps += game_played.rollout_steps;
        let digests = &game_played.digests;
        print!(
            "{} answers={} turn={} forks={} steps={} {}",
            game.id,
            game_played.answers,
            game_played.turn,
            game_played.forks,
            game_played.rollout_steps,
            game_played.ending,
        );
        if checks.verify {
            print!(
                " actions={:016x} views={:016x} hashes={:016x}",
                digests.actions.0, digests.views.0, digests.hashes.0
            );
        }
        if checks.patches {
            print!(" patched={}", digests.patched());
        }
        println!();
        if let Some(wire) = &game_played.wire {
            wire.seat_lines(&game.id);
            all.merge(wire);
        }
    }
    println!(
        "total games={} answers={answers} forks={forks} steps={steps}",
        games.len()
    );
    if played.iter().any(|game| game.wire.is_some()) {
        all.report();
    }
    Ok(())
}
