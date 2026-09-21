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
            let commanders = s
                .commanders
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
                commanders,
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
            commanders: vec![],
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

    fn msg(seats: Vec<v1::SeatSpec>) -> v1::GamePresetMsg {
        v1::GamePresetMsg {
            format: 2,
            seed: 1,
            house_rules: None,
            modifiers: vec![],
            prints: vec![],
            seats,
        }
    }

    /// The four formats the wire may name, and the refusal for anything
    /// else. A fallback arm here would start a Commander game because a
    /// client sent a number from a newer build.
    #[test]
    fn an_unknown_format_is_refused_rather_than_defaulted() {
        for (id, expected) in [
            (0, FormatId::Commander),
            (1, FormatId::Highlander),
            (2, FormatId::Freeform),
            (3, FormatId::Custom),
        ] {
            let preset = from_proto(&v1::GamePresetMsg {
                format: id,
                ..msg(vec![seat_msg(), seat_msg()])
            })
            .expect("a known format");
            assert_eq!(preset.format, expected);
        }
        assert!(
            from_proto(&v1::GamePresetMsg {
                format: 99,
                ..msg(vec![seat_msg(), seat_msg()])
            })
            .is_err()
        );
    }

    /// A game is played between sides, so one seat is not a game. The check
    /// is here as well as in `GamePreset::validate` because this is the
    /// conversion a socket reaches first.
    #[test]
    fn a_table_of_fewer_than_two_is_refused() {
        assert!(from_proto(&msg(vec![])).is_err());
        assert!(from_proto(&msg(vec![seat_msg()])).is_err());
        assert!(from_proto(&msg(vec![seat_msg(), seat_msg()])).is_ok());
    }

    /// Naming a user on the wire does not seat them. The conversion knows
    /// two controllers — an AI profile and an open chair — and everything
    /// else, including a missing one, is the open chair a lobby fills.
    #[test]
    fn a_wire_seat_is_an_ai_profile_or_an_open_chair_and_nothing_else() {
        let human = v1::SeatSpec {
            controller: Some(v1::SeatController {
                kind: Some(v1::seat_controller::Kind::HumanUserId(17)),
            }),
            ..seat_msg()
        };
        let headless = v1::SeatSpec {
            controller: None,
            ..seat_msg()
        };
        let ai = v1::SeatSpec {
            controller: Some(v1::SeatController {
                kind: Some(v1::seat_controller::Kind::Ai(v1::AiProfile {
                    lookahead: 2,
                    temperature_milli: 250,
                    mulligan_skill: 3,
                    politics: 2,
                    hold_up: 1,
                })),
            }),
            ..seat_msg()
        };

        let preset = from_proto(&msg(vec![human, headless, ai])).expect("three seats");
        assert!(matches!(preset.seats[0].controller, SeatController::Open));
        assert!(matches!(preset.seats[1].controller, SeatController::Open));
        let SeatController::Ai(profile) = preset.seats[2].controller else {
            panic!("the AI seat lost its profile");
        };
        assert_eq!(profile.lookahead, 2);
        assert_eq!(profile.temperature_milli, 250);
        assert_eq!(profile.mulligan_skill, 3);
        assert_eq!(profile.politics, Politics::Archenemy);
        assert_eq!(profile.hold_up, HoldUp::Basic);
    }

    /// An empty starting hand is **not** an empty hand: `None` is "deal the
    /// opening hand the rules say", and `Some(vec![])` is a seat that
    /// starts with no cards. A conversion collapsing the two would make
    /// every ordinary lobby game a game of seven-card mulligans into
    /// nothing, or the reverse.
    #[test]
    fn an_unstated_opening_hand_is_not_a_stated_empty_one() {
        let entry = v1::DeckEntry {
            card_index: 5,
            print_ref: 0,
        };
        let stacked = v1::SeatSpec {
            starting_hand: vec![entry],
            starting_battlefield: vec![entry],
            ..seat_msg()
        };
        let preset = from_proto(&msg(vec![seat_msg(), stacked])).expect("two seats");

        assert_eq!(preset.seats[0].starting_hand, None, "deal it normally");
        assert_eq!(
            preset.seats[1].starting_hand.as_deref().map(<[_]>::len),
            Some(1)
        );
        assert_eq!(preset.seats[0].starting_battlefield.len(), 0);
        assert_eq!(preset.seats[1].starting_battlefield.len(), 1);
        assert_eq!(preset.seats[1].starting_battlefield[0].card.get(), 5);
    }

    /// The parts of a printing a client may get wrong without stopping a
    /// game: an unreadable Scryfall id becomes the nil uuid, because a
    /// printing is how a card is *drawn* and the rules do not read it.
    /// Beside it, the enum fallbacks that are deliberate — an unknown
    /// finish is an ordinary one.
    #[test]
    fn an_unreadable_printing_does_not_stop_the_game() {
        let preset = from_proto(&v1::GamePresetMsg {
            prints: vec![
                v1::PrintInfo {
                    scryfall_id: "not a uuid".into(),
                    lang: "DE".into(),
                    finish: 2,
                },
                v1::PrintInfo {
                    scryfall_id: String::new(),
                    lang: "EN".into(),
                    finish: 47,
                },
            ],
            ..msg(vec![seat_msg(), seat_msg()])
        })
        .expect("two seats");

        assert_eq!(preset.prints[0].scryfall_id, uuid::Uuid::nil());
        assert_eq!(preset.prints[0].lang, "DE", "the language is kept");
        assert_eq!(preset.prints[0].finish, Finish::Foil);
        assert_eq!(
            preset.prints[1].finish,
            Finish::Normal,
            "an unknown finish is an ordinary one"
        );
    }

    /// House rules the message did not send are the defaults, and the one
    /// policy with two named values is read as written. Every other field
    /// rides through unchanged, which is what a game's clocks depend on.
    #[test]
    fn unsent_house_rules_are_the_defaults() {
        let sent = from_proto(&v1::GamePresetMsg {
            house_rules: Some(v1::HouseRules {
                mulligan_free_first: true,
                loop_policy: 1,
                decision_timeout_secs: 45,
                reconnect_window_secs: 90,
                timing_normalization: true,
                takebacks: true,
                time_extension_votes: true,
            }),
            ..msg(vec![seat_msg(), seat_msg()])
        })
        .expect("two seats");
        assert_eq!(sent.house_rules.loop_policy, LoopPolicy::CompRulesDraw);
        assert_eq!(sent.house_rules.decision_timeout_secs, 45);
        assert_eq!(sent.house_rules.reconnect_window_secs, 90);
        assert!(sent.house_rules.takebacks);

        let unsent = from_proto(&msg(vec![seat_msg(), seat_msg()])).expect("two seats");
        assert_eq!(unsent.house_rules, HouseRules::default());
    }
}
