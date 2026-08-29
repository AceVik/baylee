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
                deck,
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
        dev_mode: msg.dev_mode,
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
