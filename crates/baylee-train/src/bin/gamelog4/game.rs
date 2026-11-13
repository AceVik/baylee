//! One recorded game of an eval4 run, replayed input by input.

use std::fs::File;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::Path;

use anyhow::{Context as _, anyhow, bail};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_gamehost::record::Line;
use baylee_protocol::v1::Envelope;
use baylee_seat::memory::TableMemory;
use baylee_seat::reports::game_log;
use baylee_view::PlayerView;
use flate2::read::GzDecoder;
use prost::Message as _;
use serde_json::Value;

/// What a replay meets, in order.
pub(super) enum Event<'a> {
    /// A new turn began; one view per seat, each seeing its own hand.
    Turn { views: &'a [PlayerView], turn: u32 },
    /// The evaluated seat answers a question.
    Decision(Decision<'a>),
    /// Another seat answers one; one view per seat before its answer.
    Other { views: &'a [PlayerView] },
    /// A line of the game's log, as the evaluated seat was told it.
    Log { turn: u32, text: &'a str },
    /// The game is over.
    End {
        winners: &'a [u8],
        reason: &'a str,
        turn: Option<u32>,
    },
}

/// One answer of the evaluated seat.
pub(super) struct Decision<'a> {
    /// One view per seat before the answer, each seeing its own hand.
    pub views: &'a [PlayerView],
    /// The evaluated seat's view of the question, with its casting and
    /// targeting context.
    pub asked: Option<&'a PlayerView>,
    /// The question.
    pub pending: &'a Pending,
    /// The answer.
    pub action: &'a PlayerAction,
    /// Who answered for the seat: `Net`, `Search`, `House` or `Forced`, as
    /// the run's rollout sidecar says.
    pub source: &'a str,
}

/// A game of an eval4 `--record-out` directory.
pub(super) struct Game {
    /// Its line in `games.jsonl`.
    pub row: Value,
    /// The evaluated seat.
    pub seat: PlayerId,
    sources: Vec<String>,
    lines: Vec<Line>,
}

impl Game {
    /// Reads deal `game` of the record directory `run`.
    pub(super) fn load(run: &Path, game: u64) -> anyhow::Result<Self> {
        Self::load_as(run, game, None)
    }

    /// [`Self::load`], read from `seat`'s chair when given. A `selfplay`
    /// run, the house against the house, names no focal seat and keeps no
    /// rollout sidecar: every answer in it is the house's, and which chair
    /// to follow is the reader's choice.
    pub(super) fn load_as(run: &Path, game: u64, seat: Option<u8>) -> anyhow::Result<Self> {
        let row = row(run, game)?;
        let seat = PlayerId::new(match seat {
            Some(seat) => seat,
            None => u8::try_from(
                row["focal_seat"]
                    .as_u64()
                    .context("the game has no focal seat (name one with --seat)")?,
            )?,
        });
        let sources = match row["rollout_sidecar"].as_array() {
            Some(steps) => steps
                .iter()
                .map(|step| {
                    step["provenance"]["source"]
                        .as_str()
                        .unwrap_or("?")
                        .to_owned()
                })
                .collect(),
            None if row["focal_seat"].is_null() => Vec::new(),
            None => bail!("the game has no rollout sidecar"),
        };
        let lines = record(run, &row)?;
        Ok(Self {
            row,
            seat,
            sources,
            lines,
        })
    }

    /// The seats on the evaluated seat's side: itself and its teammates.
    pub(super) fn side(&self) -> Vec<PlayerId> {
        let Some(Line::Header { preset, .. }) = self.lines.first() else {
            return vec![self.seat];
        };
        let team = preset
            .seats
            .get(usize::from(self.seat.get()))
            .and_then(|seat| seat.team);
        (0..preset.seats.len())
            .filter_map(|index| u8::try_from(index).ok())
            .map(PlayerId::new)
            .filter(|player| {
                *player == self.seat
                    || team.is_some_and(|team| {
                        preset.seats[usize::from(player.get())].team == Some(team)
                    })
            })
            .collect()
    }

    /// Replays the game and shows `visit` what happens; with `log`, the
    /// game's log lines too (which costs time).
    pub(super) fn replay(
        &self,
        log: bool,
        mut visit: impl FnMut(Event<'_>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let Some(Line::Header { preset, .. }) = self.lines.first() else {
            bail!("the record does not begin with its header");
        };
        let seats = u8::try_from(preset.seats.len())?;
        let mut session = Session::new(preset).context("the record's table")?;
        for seat in 0..seats {
            session.take_over(PlayerId::new(seat));
        }
        let mut memory = TableMemory::default();
        if log {
            memory.hear(&session.game_static_envelope(self.seat).encode_to_vec());
            hear(&mut memory, self.seat, session.pump());
        }
        let mut printed = 0;
        let mut turn = None;
        let mut sources = self.sources.iter();
        for line in &self.lines[1..] {
            match line {
                Line::Input {
                    n,
                    seat: actor,
                    action,
                    ..
                } => {
                    let views: Vec<PlayerView> = (0..seats)
                        .map(|s| session.observe_view(PlayerId::new(s)))
                        .collect();
                    let now = views[usize::from(self.seat.get())].turn;
                    if turn != Some(now) {
                        turn = Some(now);
                        visit(Event::Turn {
                            views: &views,
                            turn: now,
                        })?;
                    }
                    if *actor == self.seat.get() {
                        let asked = session.view_for_with_context(self.seat);
                        visit(Event::Decision(Decision {
                            views: &views,
                            asked: asked.as_ref().map(|(_, view)| view),
                            pending: session.pending(),
                            action,
                            source: sources.next().map_or(
                                if self.sources.is_empty() {
                                    "house"
                                } else {
                                    "?"
                                },
                                String::as_str,
                            ),
                        }))?;
                    } else {
                        visit(Event::Other { views: &views })?;
                    }
                    let routed = session
                        .act(PlayerId::new(*actor), action.clone())
                        .map_err(|error| anyhow!("input {n} refused on replay: {error}"))?;
                    if log {
                        hear(&mut memory, self.seat, routed);
                        if let Some(book) = game_log(&memory) {
                            for row in book.lines.iter().skip(printed) {
                                visit(Event::Log {
                                    turn: row.turn,
                                    text: &row.text,
                                })?;
                            }
                            printed = book.lines.len();
                        }
                    }
                }
                Line::End {
                    winners,
                    reason,
                    turn,
                    ..
                } => visit(Event::End {
                    winners,
                    reason,
                    turn: *turn,
                })?,
                Line::Header { .. } => bail!("a second header inside the record"),
                Line::Chair { .. } | Line::DeclaredMind { .. } => {}
            }
        }
        if sources.next().is_some() {
            bail!("the rollout sidecar names more decisions than the seat made");
        }
        Ok(())
    }
}

/// The game's line in the run's `games.jsonl`.
fn row(run: &Path, game: u64) -> anyhow::Result<Value> {
    let path = run.join("games.jsonl");
    let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
    for line in BufReader::new(file).lines() {
        let row: Value = serde_json::from_str(&line?)?;
        if row["i"].as_u64() == Some(game) {
            return Ok(row);
        }
    }
    bail!("{} has no game {game}", path.display())
}

/// The record's lines: one gzip member of a shard.
fn record(run: &Path, row: &Value) -> anyhow::Result<Vec<Line>> {
    let shard = run
        .join("records")
        .join(row["shard"].as_str().context("the game names no shard")?);
    let offset = row["offset"].as_u64().context("the game has no offset")?;
    let len = row["len"].as_u64().context("the game has no length")?;
    let mut file = File::open(&shard).with_context(|| format!("opening {}", shard.display()))?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    GzDecoder::new(file.take(len)).read_to_end(&mut bytes)?;
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).map_err(Into::into))
        .collect()
}

/// Gives the evaluated seat's memory the frames routed to it.
fn hear(memory: &mut TableMemory, seat: PlayerId, routed: Vec<(PlayerId, Envelope)>) {
    for (to, envelope) in routed {
        if to == seat {
            memory.hear(&envelope.encode_to_vec());
        }
    }
}
