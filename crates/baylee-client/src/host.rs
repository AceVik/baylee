//! Where a duel's state comes from.
//!
//! The renderer never talks to a socket. It talks to a [`DuelHost`], which is
//! either a real connection or an engine running in this process. That is the
//! seam the open-world client will reuse: an embedded duel and a networked one
//! differ only in which host is installed, and every system above this line is
//! written once.

#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
use baylee_cards::decks::DevZone;
use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, LogTail, PlayerView};

/// Something a host tells the client.
#[derive(Clone, Debug)]
pub enum HostMessage {
    /// The once-per-game payload: seats and the print table.
    Static(Box<GameStatic>),
    /// A fresh snapshot of the game, and the game log's lines that came
    /// with it (#262), when there were any.
    ///
    /// One message and not two because the book reads the lines against the
    /// view of their own frame, and a seat with many lines waiting is sent
    /// several frames repeating one view and `seq`, each with the next part
    /// of the log. Every frame's lines count, whatever becomes of its view.
    View(Box<PlayerView>, Option<LogTail>),
    /// A choice addressed to this seat.
    Choice(Box<Pending>),
    /// Something went wrong; the string is safe to show a player.
    Failed(String),
    /// The table is open: every human seat has prepared it and the shared
    /// entrance has finished. Nothing this seat sends is read before it, which is
    /// why the client holds its outbox until then. It never comes down again
    /// in the same game.
    Curtain,
    /// What an AI seat's mind said beside an answer: a debug engine sends it
    /// to every other seat, a release engine to none (`docs/protocol.md`
    /// §"An AI seat's reasoning").
    AiLog(baylee_protocol::v1::AiLog),
    /// Human preparation progress and departure relative to receipt, clock corrected.
    Preparing {
        /// Prepared human seats.
        ready: u32,
        /// Expected human seats.
        total: u32,
        /// Seconds until the common portal departure; none also cancels it.
        starts_in: Option<f64>,
    },
}

/// Whether a host still has the connection it plays through.
///
/// A state rather than a message, because the banner that draws it is a
/// function of *now*: a message would have to be cleared by something, and
/// the only thing a client does on a dead socket is wait. It was routed
/// through `Duel::last_error` first, which clears in `submit` — a call a
/// disconnected player cannot make, so the words stayed on the screen after
/// the table came back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinkState {
    /// There is no socket to lose: an engine running in this process.
    Local,
    /// Connected, or opening for the first time.
    Up,
    /// The socket went away and this host is dialling again.
    Connecting,
    /// The socket went away and nothing is being done about it.
    Down,
    /// The table refused this client's protocol (#271). Final: a table
    /// that said no is never dialled again, and the player is told which
    /// side is behind.
    Refused {
        /// The protocol the table speaks.
        table: u32,
    },
}

/// A source of duel state.
///
/// Implementations must be non-blocking: [`DuelHost::poll`] is called once per
/// frame and may not wait on I/O. A networked implementation drains a channel
/// its transport task fills.
pub trait DuelHost: Send + Sync + 'static {
    /// Everything that arrived since the last call.
    fn poll(&mut self) -> Vec<HostMessage>;

    /// Sends the local seat's answer.
    fn submit(&mut self, action: PlayerAction);

    /// Says this seat has drawn its table (#256), so the engine can open it
    /// once every seat has. Called after the table
    /// and its required assets have been prepared by the renderer.
    ///
    /// No default: a host that forgot it would hold the whole table behind
    /// the curtain for the engine's full wait, and a missing method is the
    /// only way the compiler can say so.
    fn ready(&mut self);

    /// The seat this client plays.
    fn seat(&self) -> PlayerId;

    /// This seat's own decklist, one entry per card, where the host knows it.
    ///
    /// Empty by default: a networked seat is handed its deck by the gateway
    /// and never sees the list, only the cards as they are shown to it.
    fn own_deck(&self) -> Vec<baylee_core::ids::CardIndex> {
        Vec::new()
    }

    /// Whether the connection this host plays through is still there.
    ///
    /// Defaults to [`LinkState::Local`], which is the truthful answer for a host
    /// with no socket: an in-process engine cannot be disconnected from, so
    /// the retry schedule never starts.
    fn link(&self) -> LinkState {
        LinkState::Local
    }

    /// Opens the connection again, resuming the seat where it left off.
    ///
    /// Only ever called on a [`LinkState::Down`] host, and deliberately not called
    /// by the host itself: one that redialled on its own would hammer a
    /// gateway that is down, and only the application knows whether a player
    /// is still sitting there. Defaults to doing nothing, for a host that has
    /// nothing to reopen.
    ///
    /// # Errors
    /// When the connection cannot be started at all — a malformed URL, or a
    /// transport that could not be created. Not when the dial *fails*: that
    /// answer arrives later, through [`DuelHost::link`].
    fn reconnect(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// The seat token this host plays with, for the report form's leak
    /// check (`report::keyring`) and for nothing else.
    ///
    /// Defaults to `None`, the truthful answer for a host with no gateway
    /// behind it: an in-process table has no token to leak.
    fn seat_token(&self) -> Option<&str> {
        None
    }

    /// The record of the game this client hosts itself, as written so far
    /// (`baylee_gamehost::record`), for a report about it to offer.
    ///
    /// Defaults to `None`, the truthful answer for a networked host: its
    /// game's record is the gateway's, which attaches it itself.
    fn local_record(&self) -> Option<&[u8]> {
        None
    }

    /// Where a restart picks this game up again, for the game this client
    /// hosts itself ([`ResumePoint`]); `None` for a networked host, whose
    /// chair the gateway hands back.
    fn resume_point(&self) -> Option<ResumePoint> {
        None
    }
}

/// What a restart needs to rebuild a game this client hosts
/// (`baylee_client_core::resume::Game::Local`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumePoint {
    /// The record's file among the kept records.
    pub record: String,
    /// The local seat.
    pub seat: PlayerId,
    /// What each seat is called.
    pub names: Vec<String>,
    /// The engine's hash now, as the record writes it.
    pub hash: String,
}

/// Decodes one server envelope into the message a client acts on.
///
/// Every host shares this: the in-process one and the networked one differ in
/// where the bytes come from, never in what they mean. Envelopes this client
/// has no use for (heartbeats, the handshake) decode to nothing rather than to
/// an error — a server is allowed to say more than a renderer listens to.
pub(crate) fn host_message(envelope: Envelope) -> Option<HostMessage> {
    Some(match envelope.msg? {
        v1::envelope::Msg::GameStatic(msg) => {
            if msg.view_version == baylee_view::VIEW_VERSION {
                match serde_json::from_slice::<GameStatic>(&msg.static_json) {
                    Ok(statics) => HostMessage::Static(Box::new(statics)),
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
        v1::envelope::Msg::StateDelta(delta) => {
            match serde_json::from_slice::<PlayerView>(&delta.view_json) {
                Ok(view) => HostMessage::View(Box::new(view), log_tail(&delta.log_json)),
                Err(e) => HostMessage::Failed(format!("unreadable game state: {e}")),
            }
        }
        v1::envelope::Msg::ChoiceRequest(req) => {
            match serde_json::from_slice::<Pending>(&req.pending_json) {
                Ok(pending) => HostMessage::Choice(Box::new(pending)),
                Err(e) => HostMessage::Failed(format!("unreadable choice: {e}")),
            }
        }
        v1::envelope::Msg::Error(err) => HostMessage::Failed(err.message),
        v1::envelope::Msg::Curtain(_) => HostMessage::Curtain,
        v1::envelope::Msg::AiLog(ai_log) => HostMessage::AiLog(ai_log),
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
fn log_tail(json: &[u8]) -> Option<LogTail> {
    if json.is_empty() {
        return None;
    }
    serde_json::from_slice(json)
        .inspect_err(|e| bevy::log::warn!("unreadable game log: {e}"))
        .ok()
}

/// A duel hosted inside this process.
///
/// Used for solo play against the house AI, for the open world's embedded
/// duels, and for tests — a headless test can play a whole game through the
/// same code path a networked client uses, because the messages are identical.
pub struct LocalHost {
    session: Session,
    seat: PlayerId,
    /// The opening payload, still encoded — see [`LocalHost::absorb`].
    statics: Option<Envelope>,
    pending_out: Vec<HostMessage>,
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
}

impl LocalHost {
    /// Starts a game from a preset, with the local player on `seat`.
    ///
    /// Returns `None` when the preset does not produce a playable game — a
    /// malformed deck, or a seat count the engine refuses.
    #[must_use]
    pub fn new(preset: &GamePreset, seat: PlayerId, seat_names: &[&str]) -> Option<Self> {
        // Untimed: nothing here runs a decision clock — that is the
        // engine-server's attach loop — so a limit would only be drawn,
        // counted down to nought in plain sight now that the client shows
        // every clock in full, and then never enforced.
        let mut preset = preset.clone();
        preset.house_rules.decision_timeout_secs = 0;
        let mut session = Session::new_recorded(&preset, baylee_build::short())?;
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
            record: Vec::new(),
            live: crate::records::LiveRecord::new(&record_name),
            deck: own_deck(&preset, seat),
            record_name,
            names: seat_names.iter().map(|n| (*n).to_string()).collect(),
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
    pub fn resume(
        record: Vec<u8>,
        record_name: &str,
        seat: PlayerId,
        seat_names: &[String],
    ) -> Result<Self, String> {
        let mut session = Session::resume_recorded(&record).map_err(|why| format!("{why:?}"))?;
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
            live: crate::records::LiveRecord::continuing(record_name, record.len()),
            record,
            deck: own_deck(&preset, seat),
            record_name: record_name.to_string(),
            names: seat_names.to_vec(),
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
        .map(|spec| spec.deck.iter().map(|entry| entry.card).collect())
        .unwrap_or_default()
}

impl LocalHost {
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
            self.pending_out.extend(host_message(envelope));
        }
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

impl DuelHost for LocalHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        let mut out = Vec::new();
        let opening = self.statics.take();
        let opens = opening.is_some();
        if let Some(statics) = opening {
            out.extend(host_message(statics));
            self.session.tell_time(wall_ms());
            let routed = self.session.pump();
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
        match self.session.act(self.seat, action) {
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
                let again = self.session.reask(self.seat);
                self.pending_out
                    .extend(again.into_iter().filter_map(host_message));
            }
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
}

/// A game left before its end is kept as far as it went: the table closed,
/// the lobby took another, or the client is quitting. Its steps are on disk
/// already; this compacts them into one gzip member.
impl Drop for LocalHost {
    fn drop(&mut self) {
        self.take_the_record();
        self.live.finish(&self.record);
    }
}

/// Builds the demo duel (Allytifact vs the house AI) from the acceptance deck
/// file's contents.
///
/// Takes the text rather than a path so the parsing is testable against the
/// real data file without depending on a working directory — which is exactly
/// the thing that differs between `cargo run`, an installed binary, and a
/// browser.
#[must_use]
pub fn demo_duel(deck_file: &str, seed: u64) -> Option<GamePreset> {
    let player = baylee_cards::decks::load_acceptance(deck_file, "Allytifact").ok()?;
    let house = baylee_cards::decks::load_acceptance(deck_file, "Victory").ok()?;
    let mut preset = baylee_cards::decks::preset_for(seed, &player, &house);
    // Seat 0 is the person at the keyboard.
    preset.seats.first_mut()?.controller = baylee_core::preset::SeatController::Open;
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
    const EMBEDDED: &str = include_str!("../../../data/acceptance-decks.txt");
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
    let mut bytes = [0u8; 8];
    match getrandom::fill(&mut bytes) {
        Ok(()) => u64::from_le_bytes(bytes),
        Err(_) => 0x5eed_1234,
    }
}

/// A solo duel against the house AI, ready to install.
///
/// Everything the standalone binary needs for offline play, and the same thing
/// the lobby's "play the house" button installs — so the two cannot drift.
#[must_use]
pub fn house_duel() -> Option<LocalHost> {
    let preset = demo_duel(&acceptance_text(), fresh_seed())?;
    LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"])
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, HouseRules, PrintInfo, SeatController, SeatSpec,
    };

    fn island() -> CardIndex {
        baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
            .expect("Island is in the registry")
            .index
    }

    /// Two seats of sixty Islands, one open and one AI.
    ///
    /// `pub(crate)` so other modules' tests can start a real game rather than
    /// hand-building a `PlayerView`: a view assembled by a test is a view
    /// that agrees with whatever the test expected of it.
    pub(crate) fn duel_preset() -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: island(),
                print: PrintRef::new(0),
            })
            .collect();
        let seat = |ai: bool| SeatSpec {
            controller: if ai {
                SeatController::Ai(AIProfile::default())
            } else {
                SeatController::Open
            },
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 11,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![seat(false), seat(true)],
        }
    }

    /// The acceptance deck file that ships with the repository.
    const DECK_FILE: &str = include_str!("../../../data/acceptance-decks.txt");

    /// A table of one seat is open as soon as it is dealt, and says so once,
    /// last in its first batch (#256): the client holds its outbox until it
    /// hears it, so an offline game that never said it could not be played.
    #[test]
    fn a_table_of_one_opens_on_its_first_poll() {
        let preset = demo_duel(DECK_FILE, 7).expect("the demo duel");
        let mut host =
            LocalHost::new(&preset, PlayerId::new(0), &["You", "House"]).expect("a game");
        let first = host.poll();
        assert!(matches!(first.first(), Some(HostMessage::Static(_))));
        assert!(
            matches!(first.last(), Some(HostMessage::Curtain)),
            "the curtain is last in the opening batch"
        );
        assert_eq!(
            first
                .iter()
                .filter(|m| matches!(m, HostMessage::Curtain))
                .count(),
            1
        );
        host.submit(PlayerAction::MulliganKeep);
        assert!(
            !host
                .poll()
                .iter()
                .any(|m| matches!(m, HostMessage::Curtain)),
            "and only once"
        );
    }

    /// A dealt card and the same card in a decklist are **one** print entry.
    ///
    /// `deal_the_dev_board` appends its printing to the game's print table and
    /// deduplicates against what is already there by whole-struct equality, so
    /// the two constructions have to agree field for field. They did not — the
    /// harness wrote `"EN"` where `reference_print` writes `"en"` — and every
    /// dealt card the deck already carried got a second index. Card text is
    /// filed against the first index claiming an id, so the dealt copy, the
    /// one actually on the table, had no printed sentence at all.
    ///
    /// This is checked without the env var, on the decks the binary ships:
    /// what it really asserts is that the two print constructions are the same
    /// value, which is the whole of the dedup's precondition.
    #[test]
    fn a_dealt_card_reuses_the_printing_the_deck_already_named() {
        let preset = demo_duel(DECK_FILE, 7).expect("the shipped decks parse");
        let entry = *preset.seats[0].deck.first().expect("the deck is not empty");
        let card = baylee_cards::by_index(entry.card).expect("a deck names real cards");
        let want = baylee_cards::decks::reference_print(card.index);
        assert!(
            !want.scryfall_id.is_nil(),
            "a card in the acceptance decks has a printing to draw"
        );
        assert!(
            preset.prints.contains(&want),
            "the harness deals the entry the decklist already resolved to"
        );
    }

    #[test]
    fn the_demo_duel_that_the_binary_launches_actually_builds() {
        let preset = demo_duel(DECK_FILE, 42).expect("the shipped decks parse");
        assert_eq!(preset.seats.len(), 2);
        assert!(matches!(preset.seats[0].controller, SeatController::Open));
        assert!(
            preset.seats[0].deck.len() >= 60,
            "the human seat gets a real deck, not an empty chair"
        );

        // And it starts, with the human dealt in.
        let mut host = LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"])
            .expect("the demo duel starts");
        let messages = host.poll();
        let view = messages
            .iter()
            .find_map(|m| match m {
                HostMessage::View(v, _) => Some(v),
                _ => None,
            })
            .expect("a view");
        assert_eq!(view.hand.len(), 7);
    }

    #[test]
    fn a_local_host_announces_the_static_payload_before_any_state() {
        let mut host =
            LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "House AI"]).expect("host");
        let first = host.poll();
        assert!(matches!(first.first(), Some(HostMessage::Static(_))));

        let HostMessage::Static(statics) = &first[0] else {
            panic!("the first message is always the static payload");
        };
        assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
        assert_eq!(statics.seat_name(PlayerId::new(1)), "House AI");
        assert!(statics.seats[1].is_ai);
        assert!(!statics.seats[0].is_ai);
    }

    /// A frame's log rides beside its view (#262): read when it is there,
    /// nothing when the bytes are empty, and a log that does not decode costs
    /// its lines and not the view.
    #[test]
    fn a_frame_s_log_is_read_beside_its_view() {
        use baylee_view::{LogEntry, LogEvent};
        let frame = |log_json: Vec<u8>| Envelope {
            msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
                game_id: "g".to_string(),
                seq: 1,
                view_json: serde_json::to_vec(
                    &baylee_client_core::test_support::ViewBuilder::new(2).build(),
                )
                .expect("a view encodes"),
                log_json,
            })),
        };
        let told = LogTail {
            from: 3,
            entries: vec![LogEntry {
                at: 0,
                turn: 2,
                repeat: 1,
                event: LogEvent::TurnStarted {
                    active: PlayerId::new(1),
                },
            }],
        };
        let Some(HostMessage::View(_, Some(got))) =
            host_message(frame(serde_json::to_vec(&told).expect("a tail encodes")))
        else {
            panic!("the frame's lines were not read");
        };
        assert_eq!(got, told);
        assert!(
            matches!(
                host_message(frame(Vec::new())),
                Some(HostMessage::View(_, None))
            ),
            "a frame with nothing new in its log is not a view"
        );
        assert!(
            matches!(
                host_message(frame(b"{\"from\":".to_vec())),
                Some(HostMessage::View(_, None))
            ),
            "a log that does not decode took its view with it"
        );
    }

    /// The version rides outside the payload so this check can happen before
    /// the decode — the payload's shape is exactly what a bump changes.
    #[test]
    fn a_table_speaking_a_view_version_this_client_cannot_render_is_refused() {
        let envelope = Envelope {
            msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
                game_id: "g".to_string(),
                view_version: baylee_view::VIEW_VERSION + 1,
                static_json: b"{}".to_vec(),
            })),
        };
        let Some(HostMessage::Failed(reason)) = host_message(envelope) else {
            panic!("a version this client cannot render is refused, not rendered");
        };
        assert!(
            reason.contains(&(baylee_view::VIEW_VERSION + 1).to_string()),
            "{reason}"
        );
    }

    #[test]
    fn the_opening_poll_delivers_a_view_and_a_choice_for_the_local_seat() {
        let mut host =
            LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
        let messages = host.poll();
        assert!(
            messages.iter().any(|m| matches!(m, HostMessage::View(..))),
            "the client must be able to draw before it is asked anything"
        );
        assert!(messages.iter().any(|m| matches!(m, HostMessage::Choice(_))));
    }

    #[test]
    fn a_view_survives_the_wire_encoding_intact() {
        let mut host =
            LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
        let messages = host.poll();
        let view = messages
            .iter()
            .find_map(|m| match m {
                HostMessage::View(v, _) => Some(v),
                _ => None,
            })
            .expect("a view");

        assert_eq!(view.seat, PlayerId::new(0));
        assert_eq!(view.seats.len(), 2);
        // The opening hand reached the client, and the opponent's did not.
        // The house has already answered the AI chair's mulligan (#257). A
        // hand of seven Islands is never a keep for its policy, so it takes
        // until its limit and keeps five: still only a count.
        assert_eq!(view.hand.len(), 7);
        let them = &view.seats[1];
        assert_eq!(them.hand_count, 5);
        assert_eq!(them.hand_count + them.library_count, 60);
    }

    #[test]
    fn an_illegal_action_is_reported_rather_than_silently_dropped() {
        let mut host =
            LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
        host.poll();
        // The opening choice is a mulligan; passing priority is not an answer.
        host.submit(PlayerAction::PassPriority);
        let out = host.poll();
        assert!(
            out.iter().any(|m| matches!(m, HostMessage::Failed(_))),
            "a rejected action must surface, or the table just freezes"
        );
        // The half this test used to miss, and the reason it missed a real
        // freeze: surfacing the error is not enough. The client drops its
        // `Interaction` on submit, so a refusal that does not hand the
        // question back leaves the seat unable to answer anything ever again.
        assert!(
            out.iter().any(|m| matches!(m, HostMessage::Choice(_))),
            "a refusal must re-ask, not just say no"
        );
        // And the proof that it re-asked the *same* question, without having
        // advanced the game while saying so: the mulligan is still answerable.
        host.submit(PlayerAction::MulliganKeep);
        assert!(
            host.poll()
                .iter()
                .any(|m| matches!(m, HostMessage::View(..))),
            "the seat can still play after a refusal"
        );
    }

    #[test]
    fn keeping_the_opening_hand_advances_the_game() {
        let mut host =
            LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
        host.poll();
        host.submit(PlayerAction::MulliganKeep);
        let out = host.poll();
        assert!(out.iter().any(|m| matches!(m, HostMessage::View(..))));
        assert!(!out.iter().any(|m| matches!(m, HostMessage::Failed(_))));
    }

    /// "Restart now" in a game against the house: the game is played, its
    /// record goes to the kept records' folder as the client keeps it (one
    /// gzip member per step), the process "ends" (the host is dropped), and
    /// a new host rebuilt from that file is the same game — the same hash,
    /// the same question — and plays on in the same file. A record that
    /// lost a step on the way does not rebuild (red if a step is lost).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_house_game_restarted_in_process_resumes_to_the_same_hash() {
        let me = PlayerId::new(0);
        let folder = std::env::temp_dir().join(format!("baylee-resume-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&folder).expect("scratch");
        let name = baylee_client_core::bugreport::record_file_name(1_000, 7);
        let names = vec!["You".to_string(), "Steady 1".to_string()];
        let mut host = LocalHost::new(&duel_preset(), me, &["You", "Steady 1"]).expect("host");
        host.live = crate::records::LiveRecord::in_folder(Some(folder.clone()), &name);
        host.record_name.clone_from(&name);
        host.poll();
        for _ in 0..12 {
            let Some(action) = host.session.house_action(me) else {
                break;
            };
            host.submit(action);
            host.poll();
        }
        let point = host.resume_point().expect("a local game resumes");
        assert_eq!(point.record, name);
        assert_eq!(point.names, names);
        let inputs = host
            .local_record()
            .expect("kept")
            .split(|b| *b == b'\n')
            .count();
        assert!(inputs > 10, "the game was played: {inputs} lines");
        let asked = format!("{:?}", host.session.pending());
        drop(host); // the old client ends; its record is finished on disk

        let record = crate::records::read_kept_in(&folder, &name).expect("the kept record");
        let mut again = LocalHost::resume(record.clone(), &name, me, &names).expect("resumes");
        again.live =
            crate::records::LiveRecord::continuing_in(Some(folder.clone()), &name, record.len());
        assert_eq!(again.hash(), point.hash, "the same game");
        assert_eq!(format!("{:?}", again.session.pending()), asked);
        let opening = again.poll();
        assert!(matches!(opening.first(), Some(HostMessage::Static(_))));
        assert!(opening.iter().any(|m| matches!(m, HostMessage::View(..))));
        assert!(opening.iter().any(|m| matches!(m, HostMessage::Curtain)));

        // It plays on, in the same file, which is still one record.
        for _ in 0..3 {
            let Some(action) = again.session.house_action(me) else {
                break;
            };
            again.submit(action);
            again.poll();
        }
        let later = again.hash();
        drop(again);
        let whole = crate::records::read_kept_in(&folder, &name).expect("still kept");
        assert_eq!(
            LocalHost::resume(whole, &name, me, &names)
                .expect("one record")
                .hash(),
            later
        );

        // A step lost on the way is no resume.
        let lines: Vec<&[u8]> = record.split_inclusive(|b| *b == b'\n').collect();
        let torn: Vec<u8> = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != lines.len() / 2)
            .flat_map(|(_, l)| l.iter().copied())
            .collect();
        assert!(LocalHost::resume(torn, &name, me, &names).is_err());
        let _ = std::fs::remove_dir_all(folder);
    }

    /// A game hosted here keeps its record as a hosted game's engine
    /// writes it (#315): the header first, then each input as it is
    /// applied, the house's moves too; a networked host keeps none.
    #[test]
    fn a_local_game_keeps_its_record_as_it_goes() {
        let mut host =
            LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
        let header = host
            .local_record()
            .expect("a local host keeps one")
            .to_vec();
        assert!(header.starts_with(br#"{"kind":"header","record":1"#));
        assert_eq!(
            header.iter().position(|b| *b == b'\n'),
            Some(header.len() - 1),
            "the header and nothing else"
        );
        host.poll();
        host.submit(PlayerAction::MulliganKeep);
        host.poll();
        let record = std::str::from_utf8(host.local_record().expect("kept"))
            .expect("text")
            .to_string();
        assert!(record.starts_with(std::str::from_utf8(&header).expect("text")));
        let inputs: Vec<&str> = record.lines().skip(1).collect();
        assert!(!inputs.is_empty(), "the keep was recorded");
        assert!(
            inputs
                .iter()
                .all(|line| line.starts_with(r#"{"kind":"input""#)
                    || line.starts_with(r#"{"kind":"chair""#)),
            "{record}"
        );
        assert!(record.contains("MulliganKeep"), "{record}");
        assert!(
            baylee_client_core::bugreport::LocalRecord::pack(record.as_bytes())
                .is_some_and(|packed| !packed.complete)
        );
        assert!(
            Seatless.local_record().is_none(),
            "a host by default keeps none"
        );
    }

    struct Seatless;

    impl DuelHost for Seatless {
        fn poll(&mut self) -> Vec<HostMessage> {
            Vec::new()
        }
        fn submit(&mut self, _: PlayerAction) {}
        fn ready(&mut self) {}
        fn seat(&self) -> PlayerId {
            PlayerId::new(0)
        }
    }
}
