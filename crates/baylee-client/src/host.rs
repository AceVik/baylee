//! Where a duel's state comes from.
//!
//! The renderer never talks to a socket. It talks to a [`DuelHost`], which is
//! either a real connection or an engine running in this process. That is the
//! seam the open-world client will reuse: an embedded duel and a networked one
//! differ only in which host is installed, and every system above this line is
//! written once.

use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, PlayerView};

/// Something a host tells the client.
#[derive(Clone, Debug)]
pub enum HostMessage {
    /// The once-per-game payload: seats and the print table.
    Static(Box<GameStatic>),
    /// A fresh snapshot of the game.
    View(Box<PlayerView>),
    /// A choice addressed to this seat.
    Choice(Box<Pending>),
    /// Something went wrong; the string is safe to show a player.
    Failed(String),
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

    /// The seat this client plays.
    fn seat(&self) -> PlayerId;

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
                Ok(view) => HostMessage::View(Box::new(view)),
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
        _ => return None,
    })
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
}

impl LocalHost {
    /// Starts a game from a preset, with the local player on `seat`.
    ///
    /// Returns `None` when the preset does not produce a playable game — a
    /// malformed deck, or a seat count the engine refuses.
    #[must_use]
    pub fn new(preset: &GamePreset, seat: PlayerId, seat_names: &[&str]) -> Option<Self> {
        let mut session = Session::new(preset)?;
        session.describe(
            "local".to_string(),
            seat_names.iter().map(|n| (*n).to_string()).collect(),
        );
        let statics = session.game_static_envelope(seat);
        Some(Self {
            session,
            seat,
            statics: Some(statics),
            pending_out: Vec::new(),
        })
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

impl DuelHost for LocalHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        let mut out = Vec::new();
        if let Some(statics) = self.statics.take() {
            out.extend(host_message(statics));
            let routed = self.session.pump();
            self.absorb(routed);
        }
        out.append(&mut self.pending_out);
        out
    }

    fn submit(&mut self, action: PlayerAction) {
        match self.session.act(self.seat, action) {
            Ok(routed) => self.absorb(routed),
            // A refusal must cost the seat the action and nothing else. The
            // client drops its `Interaction` the moment it submits, so a
            // `Failed` on its own leaves the player holding no question at
            // all — every later key and click then does nothing, which reads
            // exactly like a dead client. `snapshot` is read-only, so handing
            // the question back cannot advance the game.
            Err(reason) => {
                self.pending_out.push(HostMessage::Failed(reason));
                let again = self.session.snapshot(self.seat);
                self.pending_out
                    .extend(again.into_iter().filter_map(host_message));
            }
        }
    }

    fn seat(&self) -> PlayerId {
        self.seat
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
                HostMessage::View(v) => Some(v),
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
            messages.iter().any(|m| matches!(m, HostMessage::View(_))),
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
                HostMessage::View(v) => Some(v),
                _ => None,
            })
            .expect("a view");

        assert_eq!(view.seat, PlayerId::new(0));
        assert_eq!(view.seats.len(), 2);
        // The opening hand reached the client, and the opponent's did not.
        assert_eq!(view.hand.len(), 7);
        assert_eq!(view.seats[1].hand_count, 7);
        assert_eq!(view.seats[1].library_count, 53);
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
                .any(|m| matches!(m, HostMessage::View(_))),
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
        assert!(out.iter().any(|m| matches!(m, HostMessage::View(_))));
        assert!(!out.iter().any(|m| matches!(m, HostMessage::Failed(_))));
    }
}
