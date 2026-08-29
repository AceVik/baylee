//! One game session: an engine, a human seat, AI seats auto-driven by
//! baylee-ai. Socket-free so tests can drive it directly; the websocket
//! transport lives in `main.rs`.

use baylee_ai::HeuristicAgent;
use baylee_cards::dsl::CardDef;
use baylee_core::ids::CardIndex;
use baylee_core::preset::{GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::CardLookup;
use baylee_protocol::v1::{self, Envelope};

/// Registry lookup backed by the compiled card pool.
pub struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        baylee_cards::by_index(index)
    }
}

/// A live game: engine plus the seat the human connection controls.
pub struct Session {
    engine: Engine<RegistryLookup>,
    human_seat: baylee_core::ids::PlayerId,
    agents: Vec<Option<HeuristicAgent>>,
    seq: u64,
}

impl Session {
    /// Starts a game from a preset; the first non-AI seat belongs to the
    /// connecting human, all AI seats are auto-driven.
    #[must_use]
    pub fn new(preset: &GamePreset) -> Option<Self> {
        let human_seat = preset
            .seats
            .iter()
            .position(|s| !matches!(s.controller, SeatController::Ai(_)))
            .unwrap_or(0);
        let agents: Vec<Option<HeuristicAgent>> = preset
            .seats
            .iter()
            .map(|s| match &s.controller {
                SeatController::Ai(profile) => Some(HeuristicAgent::new(*profile)),
                _ => None,
            })
            .collect();
        let engine = Engine::new(preset, RegistryLookup).ok()?;
        Some(Self {
            engine,
            human_seat: baylee_core::ids::PlayerId::new(human_seat as u8),
            agents,
            seq: 0,
        })
    }

    /// Drains AI-controlled pendings, then returns the envelopes the
    /// human needs next (a choice request for their seat, or game over).
    pub fn pump(&mut self) -> Vec<Envelope> {
        let mut out = Vec::new();
        loop {
            let pending = self.engine.pending().clone();
            let Some(player) = pending_player(&pending) else {
                if let Pending::GameOver(result) = pending {
                    out.push(view_envelope(self.seq, &self.engine, self.human_seat));
                    out.push(choice_envelope(self.seq, &Pending::GameOver(result)));
                }
                return out;
            };
            if player == self.human_seat {
                out.push(view_envelope(self.seq, &self.engine, self.human_seat));
                out.push(choice_envelope(self.seq, &pending));
                return out;
            }
            let action = self.agents[player.get() as usize]
                .as_ref()
                .expect("non-human seat has an agent")
                .act(&self.engine, player);
            if self.engine.apply(player, action).is_err() {
                // AI mis-evaluation: pass when possible, else give up the game.
                if matches!(pending, Pending::Priority { .. }) {
                    let _ = self.engine.apply(player, PlayerAction::PassPriority);
                } else {
                    return out;
                }
            }
            self.seq += 1;
        }
    }

    /// Applies a human action, then pumps the AI until the human is
    /// needed again.
    pub fn act(&mut self, action: PlayerAction) -> Vec<Envelope> {
        let player = self.human_seat;
        if self.engine.apply(player, action).is_err() {
            return vec![error_envelope("illegal action for your seat")];
        }
        self.seq += 1;
        self.pump()
    }
}

/// The player who must answer a pending choice (mirrors baylee-ai's
/// helper; kept local so the server has no AI dependency for it).
fn pending_player(pending: &Pending) -> Option<baylee_core::ids::PlayerId> {
    match pending {
        Pending::Mulligan { player, .. }
        | Pending::MulliganBottom { player, .. }
        | Pending::Priority { player, .. }
        | Pending::ChooseAttackers { player }
        | Pending::ChooseBlockers { player, .. }
        | Pending::DiscardChoice { player, .. }
        | Pending::LegendChoice { player, .. }
        | Pending::ChooseCards { player, .. }
        | Pending::ChooseTargets { player, .. }
        | Pending::ChooseSubtype { player, .. }
        | Pending::ChooseColor { player, .. }
        | Pending::ChooseNumber { player, .. }
        | Pending::ChoosePlayer { player, .. }
        | Pending::ChooseCastMode { player, .. }
        | Pending::OrderObjects { player, .. }
        | Pending::YesNo { player, .. } => Some(*player),
        Pending::GameOver(_) => None,
    }
}

fn view_envelope(
    seq: u64,
    engine: &Engine<RegistryLookup>,
    seat: baylee_core::ids::PlayerId,
) -> Envelope {
    let view = crate::view::player_view(engine.state(), seat);
    Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: String::new(),
            seq,
            view_json: serde_json::to_vec(&view).unwrap_or_default(),
        })),
    }
}

fn choice_envelope(seq: u64, pending: &Pending) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
            game_id: String::new(),
            seq,
            pending_json: serde_json::to_vec(pending).unwrap_or_default(),
        })),
    }
}

fn error_envelope(message: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: message.to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, HouseRules, PrintInfo, SeatSpec,
    };

    fn island() -> CardIndex {
        baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
            .unwrap()
            .index
    }

    fn test_preset() -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: island(),
                print: PrintRef::new(0),
            })
            .collect();
        let mk = |ai: bool| SeatSpec {
            controller: if ai {
                SeatController::Ai(AIProfile::default())
            } else {
                SeatController::Open
            },
            deck: deck.clone(),
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 7,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![mk(false), mk(true)],
        }
    }

    /// M3: a session starts, the human answers mulligans, and the AI seat
    /// is driven automatically between human choices.
    #[test]
    fn session_pumps_ai_between_human_choices() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let mut human_choices = 0;
        for _ in 0..50 {
            let envelopes = session.pump();
            if envelopes.is_empty() {
                break;
            }
            for env in envelopes {
                let Some(v1::envelope::Msg::ChoiceRequest(req)) = env.msg else {
                    continue;
                };
                let pending: Pending = serde_json::from_slice(&req.pending_json).unwrap();
                human_choices += 1;
                let action = match pending {
                    Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                    Pending::ChooseAttackers { .. } => {
                        PlayerAction::DeclareAttackers { attackers: vec![] }
                    }
                    _ => PlayerAction::PassPriority,
                };
                session.act(action);
            }
        }
        assert!(human_choices > 0, "the human received choices");
    }
}
