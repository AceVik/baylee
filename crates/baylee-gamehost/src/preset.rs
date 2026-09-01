//! `GamePresetMsg` (wire) → `GamePreset` (core) conversion.

use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HoldUp, HouseRules, LoopPolicy, Politics,
    PrintInfo, SeatController, SeatSpec,
};
use baylee_protocol::v1;

/// Converts the wire preset to the engine preset.
///
/// # Errors
/// Returns a human-readable reason for malformed input.
#[allow(clippy::too_many_lines)]
pub fn from_proto(msg: &v1::GamePresetMsg) -> Result<GamePreset, String> {
    let format = match msg.format {
        0 => FormatId::Commander,
        1 => FormatId::Highlander,
        2 => FormatId::Freeform,
        3 => FormatId::Custom,
        other => return Err(format!("unknown format id {other}")),
    };
    let house_rules = msg
        .house_rules
        .map(|h| HouseRules {
            mulligan_free_first: h.mulligan_free_first,
            loop_policy: match h.loop_policy {
                1 => LoopPolicy::CompRulesDraw,
                _ => LoopPolicy::RunOnceThenBreak,
            },
            decision_timeout_secs: h.decision_timeout_secs,
            reconnect_window_secs: h.reconnect_window_secs,
            timing_normalization: h.timing_normalization,
            takebacks: h.takebacks,
            time_extension_votes: h.time_extension_votes,
        })
        .unwrap_or_default();
    let seats = msg
        .seats
        .iter()
        .map(|s| {
            let controller = match s.controller.as_ref().and_then(|c| c.kind.as_ref()) {
                Some(v1::seat_controller::Kind::Ai(ai)) => SeatController::Ai(AIProfile {
                    lookahead: ai.lookahead as u8,
                    temperature_milli: ai.temperature_milli,
                    mulligan_skill: ai.mulligan_skill as u8,
                    politics: match ai.politics {
                        1 => Politics::AttackLeader,
                        2 => Politics::Archenemy,
                        _ => Politics::Random,
                    },
                    hold_up: match ai.hold_up {
                        1 => HoldUp::Basic,
                        2 => HoldUp::ThreatAware,
                        _ => HoldUp::None,
                    },
                }),
                Some(
                    v1::seat_controller::Kind::HumanUserId(_) | v1::seat_controller::Kind::Open(_),
                )
                | None => SeatController::Open,
            };
            let deck = s
                .deck
                .iter()
                .map(|d| DeckEntry {
                    card: CardIndex::new(d.card_index),
                    print: PrintRef::new(d.print_ref as u16),
                })
                .collect();
            let sideboard = s
                .sideboard
                .iter()
                .map(|d| DeckEntry {
                    card: CardIndex::new(d.card_index),
                    print: PrintRef::new(d.print_ref as u16),
                })
                .collect();
            let starting_hand: Vec<DeckEntry> = s
                .starting_hand
                .iter()
                .map(|d| DeckEntry {
                    card: CardIndex::new(d.card_index),
                    print: PrintRef::new(d.print_ref as u16),
                })
                .collect();
            let starting_battlefield: Vec<DeckEntry> = s
                .starting_battlefield
                .iter()
                .map(|d| DeckEntry {
                    card: CardIndex::new(d.card_index),
                    print: PrintRef::new(d.print_ref as u16),
                })
                .collect();
            Ok(SeatSpec {
                controller,
                // Never from the request: a client does not get to ask for
                // capabilities. A lobby game hands out none at all.
                capabilities: baylee_core::preset::SeatCapabilities::default(),
                deck,
                sideboard,
                starting_life: s.starting_life,
                starting_hand: if starting_hand.is_empty() {
                    None
                } else {
                    Some(starting_hand)
                },
                starting_battlefield,
                emblems: s.emblems.clone(),
                team: s.team.map(|t| t as u8),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if seats.len() < 2 {
        return Err("a game needs at least two seats".to_string());
    }
    Ok(GamePreset {
        format,
        seed: msg.seed,
        house_rules,
        modifiers: vec![],
        prints: msg
            .prints
            .iter()
            .map(|p| PrintInfo {
                scryfall_id: p.scryfall_id.parse().unwrap_or_else(|_| uuid::Uuid::nil()),
                lang: p.lang.clone(),
                finish: match p.finish {
                    2 => Finish::Foil,
                    3 => Finish::Etched,
                    _ => Finish::Normal,
                },
            })
            .collect(),
        seats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat_msg() -> v1::SeatSpec {
        v1::SeatSpec {
            controller: Some(v1::SeatController {
                kind: Some(v1::seat_controller::Kind::Open(true)),
            }),
            deck: vec![],
            sideboard: vec![],
            starting_life: None,
            starting_hand: vec![],
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        }
    }

    /// Capabilities are granted by the host, never asked for. The message
    /// this conversion reads has no field for them — `dev_mode` used to be
    /// one, and it arrived from whoever opened the socket.
    #[test]
    fn a_wire_preset_cannot_grant_itself_any_capability() {
        let msg = v1::GamePresetMsg {
            format: 2,
            seed: 1,
            house_rules: None,
            modifiers: vec![],
            prints: vec![],
            seats: vec![seat_msg(), seat_msg()],
        };
        let preset = from_proto(&msg).expect("two AI seats convert");
        for seat in &preset.seats {
            assert_eq!(
                seat.capabilities,
                baylee_core::preset::SeatCapabilities::default(),
                "a seat built from a request came out with capabilities"
            );
            assert!(!seat.capabilities.dev_commands);
            assert!(!seat.capabilities.see_hidden);
        }
    }
}
