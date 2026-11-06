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

use std::hint::black_box;
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
use baylee_gamehost::{PlayerView, Session};
use baylee_protocol::v1::Envelope;
use prost::Message as _;

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

/// What a `--verify` run compares between builds.
struct Digests {
    on: bool,
    actions: Fnv,
    views: Fnv,
    hashes: Fnv,
}

impl Digests {
    const fn new(on: bool) -> Self {
        Self {
            on,
            actions: Fnv::new(),
            views: Fnv::new(),
            hashes: Fnv::new(),
        }
    }

    fn view(&mut self, view: &PlayerView) {
        if self.on {
            self.views
                .feed(&serde_json::to_vec(view).expect("a view serializes"));
        } else {
            black_box(view);
        }
    }

    /// As JSON and not `Debug`: this is what a seat is asked, and a field a
    /// question gains without saying anything new prints in `Debug` and stays
    /// off the wire.
    fn question(&mut self, pending: &Pending) {
        if self.on {
            self.views
                .feed(&serde_json::to_vec(pending).expect("a question serializes"));
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

    /// What a socket is sent, as it goes down the wire.
    fn frames(&mut self, frames: &[(PlayerId, Envelope)]) {
        if self.on {
            for (seat, frame) in frames {
                self.views.feed(&[seat.get()]);
                self.views.feed(&frame.encode_to_vec());
            }
        } else {
            black_box(frames);
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

fn house(game: &Game, verify: bool) -> Played {
    let mut session = Session::new_recorded(&game.preset, "perf").expect("the preset builds");
    describe(&mut session, game);
    let seats = seats_of(game);
    let mut digests = Digests::new(verify);
    while running(&session) {
        session.pump_at_most(1);
        if verify {
            for &seat in &seats {
                digests.view(&session.observe_view_with_announcement_context(seat));
            }
            digests.question(session.pending());
        }
    }
    if verify {
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

fn frames(game: &Game, verify: bool) -> Played {
    let mut session = Session::new_recorded(&game.preset, "perf").expect("the preset builds");
    describe(&mut session, game);
    let seat = PlayerId::new(0);
    session.take_over(seat);
    let mut digests = Digests::new(verify);
    while running(&session) {
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
    if verify {
        digests.record(&session.take_record());
    }
    played(&session, digests, 0, 0)
}

fn decide(game: &Game, verify: bool) -> Played {
    let mut session = Session::new(&game.preset).expect("the preset builds");
    describe(&mut session, game);
    session.discard_frames();
    let seats = seats_of(game);
    for &seat in &seats {
        session.take_over(seat);
    }
    let mut digests = Digests::new(verify);
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
fn search(game: &Game, verify: bool) -> Played {
    let mut session = Session::new(&game.preset).expect("the preset builds");
    describe(&mut session, game);
    session.discard_frames();
    let seat = PlayerId::new(0);
    session.take_over(seat);
    let mut digests = Digests::new(verify);
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

/// Every game of the set, in order. Not inlined: callgrind counts from here
/// (`--toggle-collect`), which leaves the card registry and the deck files
/// out of the count.
#[inline(never)]
fn play_all(shape: Shape, games: &[Game], verify: bool) -> Vec<Played> {
    games
        .iter()
        .map(|game| match shape {
            Shape::House => house(game, verify),
            Shape::Frames => frames(game, verify),
            Shape::Decide => decide(game, verify),
            #[cfg(feature = "simulation")]
            Shape::Search => search(game, verify),
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

fn games() -> anyhow::Result<Vec<Game>> {
    let mut games = Vec::new();
    for (a, b, profile_a, profile_b) in TABLES {
        let decks = [deck(a)?, deck(b)?];
        let profiles = [profile_a, profile_b]
            .map(|name| AIProfile::named(name).with_context(|| format!("profile {name}")));
        let [profile_a, profile_b] = profiles;
        let profiles = [profile_a?, profile_b?];
        for seed in SEEDS {
            let mut preset = preset_for_all(seed, &[&decks[0], &decks[1]]);
            for (seat, profile) in preset.seats.iter_mut().zip(profiles) {
                seat.controller = SeatController::Ai(profile);
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

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verify = args.iter().any(|arg| arg == "--verify");
    let shape = match args.first().map(String::as_str) {
        Some("house") => Shape::House,
        Some("frames") => Shape::Frames,
        Some("decide") => Shape::Decide,
        #[cfg(feature = "simulation")]
        Some("search") => Shape::Search,
        _ => bail!("usage: engine_perf house|frames|decide|search [--verify]"),
    };
    let games = games()?;
    let played = play_all(shape, &games, verify);
    let (mut answers, mut forks, mut steps) = (0, 0, 0);
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
        if verify {
            print!(
                " actions={:016x} views={:016x} hashes={:016x}",
                digests.actions.0, digests.views.0, digests.hashes.0
            );
        }
        println!();
    }
    println!(
        "total games={} answers={answers} forks={forks} steps={steps}",
        games.len()
    );
    Ok(())
}
