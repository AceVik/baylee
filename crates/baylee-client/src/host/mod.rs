//! Where a duel's state comes from.
//!
//! The renderer never talks to a socket. It talks to a [`DuelHost`], which is
//! either a real connection or a local engine (an owned process on desktop). That is the
//! seam the open-world client will reuse: an embedded duel and a networked one
//! differ only in which host is installed, and every system above this line is
//! written once.

#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
use baylee_cards::decks::DevZone;
use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DetectLoops, LoopDetection};
use baylee_gamehost::Session;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, LogTail, PlayerView, StaticReader, ViewReader};

#[cfg(any(test, target_os = "android", target_os = "ios"))]
mod background;

mod variants;

#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub mod process;

#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub(crate) mod snapshot;

mod types;
pub use types::{Closure, DuelHost, HostMessage, LinkState, ResumePoint};

/// `text`, as one short line of what can be seen.
fn one_line(text: &str) -> String {
    baylee_protocol::refusal::clean_text(text).unwrap_or_default()
}

/// What one seat's frames have left it. A frame may be a patch on the last
/// view, so a host keeps this for as long as it reads the seat.
#[derive(Debug, Default)]
pub(crate) struct SeatViews {
    reader: ViewReader,
    statics: StaticReader,
    /// A frame could not be placed: the view held stands, and the table has
    /// to be asked for a whole one.
    lost: bool,
}

impl SeatViews {
    /// Whether the table has to be asked for a whole view, once.
    pub(crate) const fn take_lost(&mut self) -> bool {
        std::mem::replace(&mut self.lost, false)
    }
}

/// Decodes one server envelope into the message a client acts on.
///
/// Every host shares this: the in-process one and the networked one differ in
/// where the bytes come from, never in what they mean. Envelopes this client
/// has no use for (heartbeats, the handshake) decode to nothing rather than to
/// an error — a server is allowed to say more than a renderer listens to.
pub(crate) fn host_message(envelope: Envelope, views: &mut SeatViews) -> Option<HostMessage> {
    Some(match envelope.msg? {
        v1::envelope::Msg::GameStatic(msg) => {
            if msg.view_version == baylee_view::VIEW_VERSION {
                match views.statics.read(&msg.statics, &msg.more) {
                    Ok(Some(statics)) => HostMessage::Static(Box::new(statics.clone())),
                    // A whole payload is on its way with a whole view.
                    Ok(None) => return None,
                    Err(why) if !msg.more.is_empty() => {
                        bevy::log::warn!("asking the table for a whole view: {why}");
                        views.lost = true;
                        return None;
                    }
                    Err(e) => HostMessage::Failed(format!("unreadable game setup: {e}")),
                }
            } else {
                // Refused rather than rendered wrong: the payload's *shape* is
                // exactly what a version bump changes, which is why the version
                // rides outside it.
                HostMessage::Failed(format!(
                    "this client renders game views version {}, the table speaks {}",
                    baylee_view::VIEW_VERSION,
                    msg.view_version
                ))
            }
        }
        v1::envelope::Msg::StateDelta(delta) => match views.reader.read(&delta.view_frame) {
            Ok(Some(view)) => HostMessage::View(Box::new(view.clone()), log_tail(&delta.log)),
            // A whole view is on its way, with the whole log: nothing here is
            // news, and the table stays drawn as it is until it comes.
            Ok(None) => return None,
            Err(why) => {
                bevy::log::warn!("asking the table for a whole view: {why}");
                views.lost = true;
                return None;
            }
        },
        v1::envelope::Msg::ChoiceRequest(req) => {
            match baylee_core::wire::decode::<Pending>(&req.pending) {
                Ok(pending) => HostMessage::Choice(Box::new(pending)),
                Err(e) => HostMessage::Failed(format!("unreadable choice: {e}")),
            }
        }
        // A table's words reach the player as they are, so a hostile gateway's
        // are held to one short line of what can be seen.
        v1::envelope::Msg::Error(err) => HostMessage::Failed(one_line(&err.message)),
        v1::envelope::Msg::Curtain(_) => HostMessage::Curtain,
        v1::envelope::Msg::AiLog(ai_log) => HostMessage::AiLog(ai_log),
        v1::envelope::Msg::Spectators(s) => HostMessage::Spectators(s.count),
        v1::envelope::Msg::GameClosed(closed) => HostMessage::Closed(one_line(&closed.reason)),
        _ => return None,
    })
}

/// The log lines a frame carries, if it carries any.
///
/// Empty bytes are "nothing new" and not a tail to decode. Lines that do not
/// decode are dropped and the view kept: the view is the game and the lines
/// are its commentary, and a frame's view is not wrong because its lines
/// are. Nothing is lost for good, either: the book counts the gap the next
/// tail leaves, and the host tells the whole log again on the next snapshot
/// or reconnect.
pub(crate) fn log_tail(bytes: &[u8]) -> Option<LogTail> {
    if bytes.is_empty() {
        return None;
    }
    baylee_core::wire::decode(bytes)
        .inspect_err(|e| bevy::log::warn!("unreadable game log: {e}"))
        .ok()
}

/// How long one house search at a table of this program may take before it
/// is given up and the level's own answer stands: as long as an engine
/// server gives one by default (`baylee_engine_server::foresight::WALL_MS`).
#[cfg(not(target_arch = "wasm32"))]
const LOCAL_WALL: std::time::Duration = std::time::Duration::from_secs(3);

/// The wall guard's clock: the time since this program first asked.
#[cfg(not(target_arch = "wasm32"))]
fn monotonic() -> std::time::Duration {
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    ORIGIN.get_or_init(std::time::Instant::now).elapsed()
}

/// A duel hosted inside this process.
///
/// Used for solo play against the house AI, for the open world's embedded
/// duels, and for tests — a headless test can play a whole game through the
/// same code path a networked client uses, because the messages are identical.
pub struct LocalHost<D: LoopDetection = DetectLoops> {
    session: Session<D>,
    seat: PlayerId,
    /// The opening payload, still encoded — see [`LocalHost::absorb`].
    statics: Option<Envelope>,
    pending_out: Vec<HostMessage>,
    /// What this seat's frames left it.
    views: SeatViews,
    /// The game's record so far: the same JSON Lines a hosted game's engine
    /// writes (#315), taken from the session after every step.
    record: Vec<u8>,
    /// Its file among the kept records, written step by step
    /// (`crate::records`).
    live: crate::records::LiveRecord,
    /// The local seat's decklist, for the creature-type chooser's quick list.
    deck: Vec<baylee_core::ids::CardIndex>,
    /// The record's file name, and what each seat is called: what a restart
    /// needs to rebuild this game ([`DuelHost::resume_point`]).
    record_name: String,
    names: Vec<String>,
    /// Maximum house AI steps answered in one `poll` or `submit` slice before
    /// yielding intermediate `HostMessage::View` updates to the UI.
    pump_budget: usize,
}

impl<D: LoopDetection> LocalHost<D> {
    /// Installs the local rules engine behind the renderer's non-blocking seam.
    /// Mobile clients run it on one dedicated worker; browser builds retain
    /// the synchronous host until a worker transport is available there.
    /// Desktop callers use [`LocalHost::install`], whose process can be stopped.
    ///
    /// On the worker the house searches as at a hosted table
    /// ([`Self::searching`]). A browser build's house does not: its host
    /// runs on the frame's own thread, where a searched answer would hold
    /// the page for up to a second.
    #[must_use]
    #[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
    pub fn into_installed(self) -> crate::InstalledHost {
        #[cfg(not(target_arch = "wasm32"))]
        let host: Box<dyn DuelHost> = Box::new(background::BackgroundHost::new(self.searching()));
        #[cfg(target_arch = "wasm32")]
        let host: Box<dyn DuelHost> = Box::new(self);
        crate::InstalledHost(host)
    }

    /// This table with the house trying its options in copies of the game
    /// before it answers, within each level's own bound (`docs/house-ai.md`
    /// §"Foresight at a host"): no search past `expert`'s bound, one search
    /// at a time in this program, given up after [`LOCAL_WALL`] of wall time
    /// for the level's own answer, and one searched answer per slice so the
    /// table moves while the house thinks. A table built for a test plays
    /// without it unless it asks.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn searching(mut self) -> Self {
        static SEARCHES: std::sync::OnceLock<std::sync::Arc<baylee_gamehost::Searches>> =
            std::sync::OnceLock::new();
        let permits =
            std::sync::Arc::clone(SEARCHES.get_or_init(|| baylee_gamehost::Searches::new(1)));
        self.session.set_foresight(true);
        self.session
            .set_foresight_host(baylee_gamehost::HostForesight {
                percent: 100,
                ceiling: baylee_core::preset::AIProfile::EXPERT_WORK,
                wall: Some(baylee_gamehost::WallGuard {
                    clock: monotonic,
                    limit: LOCAL_WALL,
                }),
                permits: Some(permits),
                yields: true,
                needs_a_person: true,
            });
        self
    }

    /// Sets the maximum number of house AI steps answered per slice before
    /// streaming intermediate views.
    #[must_use]
    pub const fn with_pump_budget(mut self, budget: usize) -> Self {
        self.pump_budget = if budget == 0 { 1 } else { budget };
        self
    }

    /// Starts a game from a preset, with the local player on `seat`.
    ///
    /// Returns `None` when the preset does not produce a playable game — a
    /// malformed deck, or a seat count the engine refuses.
    #[must_use]
    pub fn with_detection(
        preset: &GamePreset,
        seat: PlayerId,
        seat_names: &[&str],
    ) -> Option<Self> {
        // Untimed: nothing here runs a decision clock — that is the
        // engine-server's attach loop — so a limit would only be drawn,
        // counted down to nought in plain sight now that the client shows
        // every clock in full, and then never enforced.
        let mut preset = preset.clone();
        preset.house_rules.decision_timeout_secs = 0;
        let mut session = Session::recorded_with_detection(&preset, baylee_build::short())?;
        session.describe(
            "local".to_string(),
            seat_names.iter().map(|n| (*n).to_string()).collect(),
        );
        let statics = session.game_static_envelope(seat);
        let record_name = baylee_client_core::bugreport::record_file_name(wall_ms(), preset.seed);
        let mut host = Self {
            session,
            seat,
            statics: Some(statics),
            pending_out: Vec::new(),
            views: SeatViews::default(),
            record: Vec::new(),
            live: crate::records::LiveRecord::new(&record_name),
            deck: own_deck(&preset, seat),
            record_name,
            names: seat_names.iter().map(|n| (*n).to_string()).collect(),
            pump_budget: 4096,
        };
        host.take_the_record();
        Some(host)
    }

    /// The game kept as `record` (the file `record_name` among the kept
    /// records) picked up where it stops, the local player on `seat`: what
    /// a restart into an update comes back to (`docs/client.md`
    /// §"Restarting into an update"). The record goes on in the same file.
    ///
    /// # Errors
    /// Why the record did not rebuild the game, in words for the log.
    pub fn resume_with_detection(
        record: Vec<u8>,
        record_name: &str,
        seat: PlayerId,
        seat_names: &[String],
    ) -> Result<Self, String> {
        let mut session =
            Session::resume_with_detection(&record).map_err(|why| format!("{why:?}"))?;
        session.describe("local".to_string(), seat_names.to_vec());
        let preset = record
            .split(|b| *b == b'\n')
            .next()
            .and_then(|header| serde_json::from_slice(header).ok())
            .and_then(|line| match line {
                baylee_gamehost::record::Line::Header { preset, .. } => Some(preset),
                _ => None,
            })
            .ok_or("the record has no header")?;
        if seat.get() as usize >= preset.seats.len() {
            return Err("no such seat".to_string());
        }
        let statics = session.game_static_envelope(seat);
        Ok(Self {
            session,
            seat,
            statics: Some(statics),
            pending_out: Vec::new(),
            views: SeatViews::default(),
            live: crate::records::LiveRecord::continuing(record_name, record.len()),
            record,
            deck: own_deck(&preset, seat),
            record_name: record_name.to_string(),
            names: seat_names.to_vec(),
            pump_budget: 4096,
        })
    }

    /// The engine's hash, as a record writes it.
    #[must_use]
    pub fn hash(&self) -> String {
        format!("{:016x}", self.session.snapshot_hash())
    }
}

/// The seat's decklist in `preset`.
fn own_deck(preset: &GamePreset, seat: PlayerId) -> Vec<baylee_core::ids::CardIndex> {
    preset
        .seats
        .get(seat.get() as usize)
        .map_or_default(|spec| spec.deck.iter().map(|entry| entry.card).collect())
}

impl<D: LoopDetection> LocalHost<D> {
    /// Takes what the session recorded since the last step and puts it on
    /// disk at once, so a crash loses at most the step it happened in; the
    /// moment the game is over the file is finished.
    fn take_the_record(&mut self) {
        let fresh = self.session.take_record();
        if fresh.is_empty() {
            return;
        }
        let ended = fresh
            .split(|b| *b == b'\n')
            .any(|line| line.starts_with(br#"{"kind":"end""#));
        self.record.extend_from_slice(&fresh);
        if ended {
            self.live.finish(&self.record);
        } else {
            self.live.write(&self.record);
        }
    }

    /// Decodes the session's envelopes for this seat.
    ///
    /// The local host deliberately goes through the same protobuf envelopes a
    /// socket would carry rather than reaching into the engine: if the wire
    /// encoding loses something, solo play loses it too, and the bug is found
    /// at a desk instead of in a match.
    fn absorb(&mut self, routed: Vec<(PlayerId, Envelope)>) {
        for (player, envelope) in routed {
            if player != self.seat {
                continue;
            }
            push_host_message(&mut self.pending_out, envelope, &mut self.views);
        }
        // Nothing is lost between a session and this host, so a view that did
        // not place is a bug here; a seat in a game is still owed its table.
        if self.views.take_lost() {
            for envelope in self.session.resume(self.seat, 0) {
                push_host_message(&mut self.pending_out, envelope, &mut self.views);
            }
        }
    }
}

/// Folds a continuation `StateDelta` frame (same non-zero `seq`, contiguous
/// non-empty [`LogTail`]) into the preceding [`HostMessage::View`] without
/// rebuilding the board for it.
///
/// `Session::frames_of` splits a step with more than `LOG_TAIL_CAP` log lines
/// into several `StateDelta` frames carrying the same `seq`, the first with the
/// view and each with the next slice of `LogTail`. Merging contiguous chunks at
/// the host boundary keeps from rebuilding the board once per 200 log lines.
pub(crate) fn try_merge_log_chunk(out: &mut [HostMessage], envelope: &Envelope) -> bool {
    if let Some(v1::envelope::Msg::StateDelta(delta)) = &envelope.msg
        && delta.seq != 0
        && let Some(HostMessage::View(prev_view, Some(prev_tail))) = out.last_mut()
        && prev_view.seq == delta.seq
        && !prev_tail.entries.is_empty()
        && let Some(next_tail) = log_tail(&delta.log)
        && !next_tail.entries.is_empty()
        && prev_tail.from as usize + prev_tail.entries.len() == next_tail.from as usize
    {
        prev_tail.entries.extend(next_tail.entries);
        return true;
    }
    false
}

/// Decodes `envelope` and appends it to `out`, merging multi-chunk log tails
/// that repeat the same `seq` via [`try_merge_log_chunk`].
pub(crate) fn push_host_message(
    out: &mut Vec<HostMessage>,
    envelope: Envelope,
    views: &mut SeatViews,
) {
    if try_merge_log_chunk(out, &envelope) {
        return;
    }
    if let Some(msg) = host_message(envelope, views) {
        out.push(msg);
    }
}

/// The wall time the game log stamps its lines with (#300), through
/// `web_time`, because `std::time::SystemTime::now` panics in a browser.
fn wall_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

impl<D: LoopDetection> DuelHost for LocalHost<D> {
    fn poll(&mut self) -> Vec<HostMessage> {
        let mut out = Vec::new();
        let opening = self.statics.take();
        let opens = opening.is_some();
        if let Some(statics) = opening {
            out.extend(host_message(statics, &mut self.views));
        }
        if opens || (self.pending_out.is_empty() && self.session.needs_pump()) {
            self.session.tell_time(wall_ms());
            let routed = self.session.pump_slice(self.pump_budget);
            self.absorb(routed);
        }
        out.append(&mut self.pending_out);
        self.take_the_record();
        // A table of one seat is open as soon as it is dealt: nobody else is
        // loading, and no clock runs here. Said anyway, and last, because
        // every host that serves a seat does (#256).
        if opens {
            out.push(HostMessage::Curtain);
        }
        out
    }

    fn submit(&mut self, action: PlayerAction) {
        self.session.tell_time(wall_ms());
        match self.session.act_slice(self.seat, action, self.pump_budget) {
            Ok(routed) => self.absorb(routed),
            // A refusal must cost the seat the action and nothing else. The
            // client drops its `Interaction` the moment it submits, so a
            // `Failed` on its own leaves the player holding no question at
            // all — every later key and click then does nothing, which reads
            // exactly like a dead client. `reask` is read-only, so handing
            // the question back cannot advance the game. It carries no log:
            // a refusal loses no frame, and `snapshot` would tell the seat
            // every line of the game again for each one.
            Err(reason) => {
                self.pending_out.push(HostMessage::Failed(reason));
                for envelope in self.session.reask(self.seat) {
                    push_host_message(&mut self.pending_out, envelope, &mut self.views);
                }
            }
        }
        self.take_the_record();
    }

    fn seat_setting(&mut self, setting: baylee_view::SeatSetting) {
        self.session.tell_time(wall_ms());
        match self.session.seat_setting(self.seat, setting) {
            Ok(routed) => self.absorb(routed),
            Err(reason) => self.pending_out.push(HostMessage::Failed(reason)),
        }
        self.take_the_record();
    }

    /// Nothing to say: this table opened on the first poll.
    fn ready(&mut self) {}

    fn own_deck(&self) -> Vec<baylee_core::ids::CardIndex> {
        self.deck.clone()
    }

    fn seat(&self) -> PlayerId {
        self.seat
    }

    fn local_record(&self) -> Option<&[u8]> {
        Some(&self.record)
    }

    fn resume_point(&self) -> Option<ResumePoint> {
        Some(ResumePoint {
            record: self.record_name.clone(),
            seat: self.seat,
            names: self.names.clone(),
            hash: self.hash(),
        })
    }

    fn busy(&self) -> bool {
        self.session.needs_pump()
    }
}

/// A game left before its end is kept as far as it went: the table closed,
/// the lobby took another, or the client is quitting. Its steps are on disk
/// already; this compacts them into one gzip member.
impl<D: LoopDetection> Drop for LocalHost<D> {
    fn drop(&mut self) {
        self.take_the_record();
        self.live.finish(&self.record);
    }
}

/// Builds a standalone two-seat test duel (`Allytifact` vs `Victory`) from the
/// acceptance deck file's contents.
///
/// Takes the text rather than a path so test fixtures parse the real data file
/// without depending on a working directory.
#[cfg(test)]
#[must_use]
pub(crate) fn demo_duel(deck_file: &str, seed: u64) -> Option<GamePreset> {
    let player = baylee_cards::decks::load_acceptance(deck_file, "Allytifact").ok()?;
    let house = baylee_cards::decks::load_acceptance(deck_file, "Victory").ok()?;
    let mut preset = baylee_cards::decks::preset_for(seed, &player, &house);
    // Seat 0 is the person at the keyboard.
    preset.seats.first_mut()?.controller = baylee_core::preset::SeatController::Open;
    preset.house_rules.order_triggers = true;
    preset.house_rules.divide_trample = true;
    Some(preset)
}

/// A board — and a hand — dealt by hand, for proving something about a card.
///
/// `BAYLEE_DEV_SEAT_BOARD` is a **semicolon**-separated list of card names,
/// each optionally prefixed with a seat and a colon:
/// `Kazandu Blademaster; 1:Baleful Strix` puts the first on the player's side
/// and the second on the house's. The separator is a semicolon because a comma
/// is part of a card's name far too often — "Sokka, Tenacious Tactician" —
/// and a colon only counts as a seat prefix when what precedes it is a number,
/// for the same reason.
///
/// The cards arrive on the battlefield before turn one, through the same
/// `SeatSpec::starting_battlefield` the duel-flow tests use, so the engine
/// treats them exactly as it treats a boss board or a puzzle.
///
/// `BAYLEE_DEV_SEAT_HAND` is the same list one zone along, and **replaces**
/// the opening deal for the seats it names rather than adding to it — a
/// `starting_hand` is the whole hand. `BAYLEE_DEV_SEAT_HAND="Reveillark"`
/// with six `Plains` on the board is AZ's acceptance criterion, set up in two
/// variables instead of by playing a ninety-card singleton deck into
/// position.
///
/// `BAYLEE_DEV_SEAT_COMMANDER` is the third, and it names a card no amount of
/// playing can reach: a seat either started with a commander or it did not.
/// It does not make a commander *visible* — a deck names its own, and the
/// acceptance file's Allytifact sits down with General Tazri — it decides
/// **which** one, which is what a measurement usually needs: the slips were
/// photographed off Ragavan at `{R}`, castable on turn one from a dealt
/// board and cheap enough to copy twice, where Tazri costs six.
///
/// It exists because the alternative is playing a duel into position, and a
/// singleton in a ninety-card deck is not something a game reaches on request:
/// ten turns of the offline duel put four lands and no creature on the table,
/// which proves nothing about a rail of keyword marks that only a creature
/// wears. Anything about how a permanent is *drawn* needs that permanent, and
/// needs it in under a second.
///
/// Behind the `dev-control` feature with the rest of the harness, because a
/// shipped binary that seats cards from the environment is a cheat, and loud
/// on a name it cannot find — a typo that quietly dealt nothing would turn
/// "this mark does not animate" into a conclusion about the shader.
///
/// # Panics
///
/// On a card name no printing in the registry answers to, or a seat this game
/// does not have. Deliberately, and see above: the alternative is a harness
/// that reports a measurement of a board it never dealt.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub fn deal_the_dev_board(preset: &mut GamePreset) {
    deal_the_dev_zone(preset, "BAYLEE_DEV_SEAT_BOARD", DevZone::Battlefield);
    deal_the_dev_zone(preset, "BAYLEE_DEV_SEAT_HAND", DevZone::Hand);
    deal_the_dev_zone(preset, "BAYLEE_DEV_SEAT_COMMANDER", DevZone::Command);
}

/// The half of [`deal_the_dev_board`] that reads one variable into one zone.
///
/// `BAYLEE_DEV_SEAT_HAND` exists for the same reason the board variable does,
/// one zone along: anything about a card *in hand* — which is most of what a
/// click does — needs that card in hand, and a singleton in a ninety-card
/// deck is not something a game reaches on request. It is what makes AZ's own
/// acceptance criterion performable at all (`docs/client.md` §"Which way to
/// cast it is asked before anything is tapped"): Reveillark and six open
/// Plains are two variables and no duel played into position.
///
/// The dealing itself is `baylee_cards::decks::deal_named`, which takes the
/// spec as an argument and reads no environment — the gateway deals the same
/// list behind its own `dev-table` feature, and one parser is what keeps a
/// board dealt there from being a different board.
///
/// # Panics
///
/// As [`deal_the_dev_board`], and for its reasons.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
fn deal_the_dev_zone(preset: &mut GamePreset, variable: &str, zone: DevZone) {
    let Ok(spec) = std::env::var(variable) else {
        return;
    };
    if let Err(why) = baylee_cards::decks::deal_named(preset, &spec, zone) {
        panic!("{variable}: {why}");
    }
}

/// The acceptance deck file, wherever this build can find it.
///
/// Looked for beside the working directory first and then beside the source
/// tree, so `cargo run` from anywhere in the workspace works and so does a
/// binary run from the repository root. The final fallback is the copy
/// embedded at build time — a browser has no filesystem at all.
#[must_use]
pub fn acceptance_text() -> String {
    /// Embedded copy of the deck file (the only source in a browser build).
    const EMBEDDED: &str = include_str!("../../../../data/acceptance-decks.txt");
    #[cfg(not(target_arch = "wasm32"))]
    {
        const CANDIDATES: [&str; 3] = [
            "data/acceptance-decks.txt",
            "../data/acceptance-decks.txt",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/acceptance-decks.txt"
            ),
        ];
        if let Some(text) = CANDIDATES
            .iter()
            .find_map(|path| std::fs::read_to_string(path).ok())
        {
            return text;
        }
    }
    EMBEDDED.to_string()
}

/// A fresh shuffle per launch.
///
/// The engine is deterministic given a seed, which is what makes replays work;
/// that is a reason to *record* the seed, never a reason to reuse one. The
/// seed comes from the platform CSPRNG (Web Crypto in the browser) —
/// `std::time` panics on wasm32, so it is not an option here.
#[must_use]
pub fn fresh_seed() -> u64 {
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    if let Some(seed) = std::env::var("BAYLEE_DEV_SEED")
        .ok()
        .and_then(|seed| seed.parse().ok())
    {
        return seed;
    }
    let mut bytes = [0u8; 8];
    match getrandom::fill(&mut bytes) {
        Ok(()) => u64::from_le_bytes(bytes),
        Err(_) => 0x5eed_1234,
    }
}

/// A standalone two-seat local duel (`Allytifact` vs `Victory`) for tests.
///
/// Used by local-host and bug-report unit tests that need an installed
/// [`LocalHost`] without driving the offline lobby (`lobby::offline` builds the
/// game a player launches).
#[cfg(test)]
#[must_use]
pub(crate) fn house_duel() -> Option<LocalHost> {
    let preset = demo_duel(&acceptance_text(), fresh_seed())?;
    LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"])
}

#[cfg(test)]
pub(crate) mod tests;
