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
use baylee_gamehost::{Session, view};
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, PlayerView, SeatIdentity};

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
}

/// A duel hosted inside this process.
///
/// Used for solo play against the house AI, for the open world's embedded
/// duels, and for tests — a headless test can play a whole game through the
/// same code path a networked client uses, because the messages are identical.
pub struct LocalHost {
    session: Session,
    seat: PlayerId,
    statics: Option<GameStatic>,
    pending_out: Vec<HostMessage>,
}

impl LocalHost {
    /// Starts a game from a preset, with the local player on `seat`.
    ///
    /// Returns `None` when the preset does not produce a playable game — a
    /// malformed deck, or a seat count the engine refuses.
    #[must_use]
    pub fn new(preset: &GamePreset, seat: PlayerId, seat_names: &[&str]) -> Option<Self> {
        let session = Session::new(preset)?;
        let seats = preset
            .seats
            .iter()
            .enumerate()
            .map(|(i, spec)| SeatIdentity {
                player: PlayerId::new(i as u8),
                display_name: seat_names
                    .get(i)
                    .map_or_else(|| format!("Seat {i}"), |n| (*n).to_string()),
                is_ai: matches!(spec.controller, baylee_core::preset::SeatController::Ai(_)),
                team: spec.team,
            })
            .collect();
        let statics = view::game_static("local".to_string(), seat, seats, &preset.prints);
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
            match envelope.msg {
                Some(v1::envelope::Msg::StateDelta(delta)) => {
                    match serde_json::from_slice::<PlayerView>(&delta.view_json) {
                        Ok(view) => self.pending_out.push(HostMessage::View(Box::new(view))),
                        Err(e) => self
                            .pending_out
                            .push(HostMessage::Failed(format!("unreadable game state: {e}"))),
                    }
                }
                Some(v1::envelope::Msg::ChoiceRequest(req)) => {
                    match serde_json::from_slice::<Pending>(&req.pending_json) {
                        Ok(pending) => self
                            .pending_out
                            .push(HostMessage::Choice(Box::new(pending))),
                        Err(e) => self
                            .pending_out
                            .push(HostMessage::Failed(format!("unreadable choice: {e}"))),
                    }
                }
                Some(v1::envelope::Msg::Error(err)) => {
                    self.pending_out.push(HostMessage::Failed(err.message));
                }
                _ => {}
            }
        }
    }
}

impl DuelHost for LocalHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        let mut out = Vec::new();
        if let Some(statics) = self.statics.take() {
            out.push(HostMessage::Static(Box::new(statics)));
            let routed = self.session.pump();
            self.absorb(routed);
        }
        out.append(&mut self.pending_out);
        out
    }

    fn submit(&mut self, action: PlayerAction) {
        match self.session.act(self.seat, action) {
            Ok(routed) => self.absorb(routed),
            Err(reason) => self.pending_out.push(HostMessage::Failed(reason)),
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

#[cfg(test)]
mod tests {
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

    fn duel_preset() -> GamePreset {
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
            dev_mode: false,
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
