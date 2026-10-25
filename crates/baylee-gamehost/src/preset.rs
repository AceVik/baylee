//! `GamePresetMsg` (wire) → `GamePreset` (core) conversion.

use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HoldUp, HouseRules, LoopPolicy, Politics,
    PrintInfo, SeatController, SeatSpec,
};
use baylee_protocol::v1;

/// Bounds-check rules references and decode a physical copy's deck choices.
fn deck_entry(d: &v1::DeckEntry) -> Result<DeckEntry, String> {
    let print = u16::try_from(d.print_ref).map_err(|_| "printing reference is out of range")?;
    let rule_print = d
        .rule_print_ref
        .map(|id| u16::try_from(id).map(baylee_core::ids::RulePrintRef::new))
        .transpose()
        .map_err(|_| "rules printing reference is out of range")?;
    let circled_colors = if d.circled_colors == 0 {
        None
    } else {
        if d.circled_colors > 31 || d.circled_colors.count_ones() != 2 {
            return Err("circled colors must name exactly two different colors".into());
        }
        let mut colors = baylee_core::color::Color::ALL
            .into_iter()
            .filter(|color| d.circled_colors & u32::from(color.bit()) != 0);
        Some(baylee_core::color::ColorPair::new(
            colors.next().unwrap(),
            colors.next().unwrap(),
        ))
    };
    Ok(DeckEntry {
        card: CardIndex::new(d.card_index),
        print: PrintRef::new(print),
        rule_print,
        choices: baylee_core::card_metadata::DeckChoices { circled_colors },
    })
}

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
        .as_ref()
        .map(|h| HouseRules {
            mulligan_free_first: h.mulligan_free_first,
            free_mulligans: h.free_mulligans.map(|n| u8::try_from(n).unwrap_or(u8::MAX)),
            starting_counters: h
                .starting_counters
                .iter()
                .map(|p| baylee_core::preset::StartingCounters {
                    seat: p.seat as usize,
                    permanent: p.permanent as usize,
                    counters: p
                        .counters
                        .iter()
                        .map(|c| baylee_core::preset::StartingCounter {
                            kind: c.kind.clone(),
                            amount: u16::try_from(c.amount).unwrap_or(u16::MAX),
                        })
                        .collect(),
                })
                .collect(),
            loop_policy: match h.loop_policy {
                1 => LoopPolicy::CompRulesDraw,
                _ => LoopPolicy::RunOnceThenBreak,
            },
            starting_player: None,
            auto_discard_empty_optional_triggers: true,
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
                .map(deck_entry)
                .collect::<Result<Vec<_>, String>>()?;
            let sideboard = s
                .sideboard
                .iter()
                .map(deck_entry)
                .collect::<Result<Vec<_>, String>>()?;
            let commanders = s
                .commanders
                .iter()
                .map(deck_entry)
                .collect::<Result<Vec<_>, String>>()?;
            let starting_hand: Vec<DeckEntry> = s
                .starting_hand
                .iter()
                .map(deck_entry)
                .collect::<Result<Vec<_>, String>>()?;
            let starting_battlefield: Vec<DeckEntry> = s
                .starting_battlefield
                .iter()
                .map(deck_entry)
                .collect::<Result<Vec<_>, String>>()?;
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
    let rules = if msg.rules_metadata_json.is_empty() {
        baylee_core::card_metadata::RulesMetadata::default()
    } else {
        serde_json::from_slice(&msg.rules_metadata_json)
            .map_err(|e| format!("invalid rules metadata: {e}"))?
    };
    if !rules.is_valid() {
        return Err("invalid rules metadata".into());
    }
    Ok(GamePreset {
        rules,
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

    /// **A number this conversion does not know is refused in one field and
    /// defaulted in every other**, and the split is deliberate rather than
    /// an oversight.
    ///
    /// `format` decides what game is being played — a table whose format was
    /// silently read as Commander is not a degraded game, it is a different
    /// one — so an unknown id is an error and
    /// [`an_unknown_format_is_refused_rather_than_defaulted`] pins that.
    /// Everything below is a *preference* with a sane default, and refusing
    /// the whole preset over one would mean a newer gateway could never open
    /// a table on an older host at all.
    ///
    /// Pinned here so the asymmetry is a decision somebody can find rather
    /// than a shape three `match` arms happen to have. The day one of these
    /// starts mattering enough to refuse, this test is what has to move.
    #[test]
    fn an_unknown_preference_falls_back_where_an_unknown_format_refuses() {
        let with_rules = |loop_policy| {
            from_proto(&v1::GamePresetMsg {
                house_rules: Some(v1::HouseRules {
                    free_mulligans: None,
                    loop_policy,
                    ..v1::HouseRules::default()
                }),
                ..msg(vec![seat_msg(), seat_msg()])
            })
            .expect("two seats")
            .house_rules
            .loop_policy
        };
        assert_eq!(with_rules(1), LoopPolicy::CompRulesDraw, "the one it knows");
        assert_eq!(
            with_rules(99),
            LoopPolicy::RunOnceThenBreak,
            "and a policy from a newer host is the default, not a refusal"
        );
        assert_eq!(with_rules(0), LoopPolicy::RunOnceThenBreak);

        let ai = |politics, hold_up| {
            let seat = v1::SeatSpec {
                controller: Some(v1::SeatController {
                    kind: Some(v1::seat_controller::Kind::Ai(v1::AiProfile {
                        politics,
                        hold_up,
                        ..v1::AiProfile::default()
                    })),
                }),
                ..seat_msg()
            };
            match from_proto(&msg(vec![seat, seat_msg()]))
                .expect("two seats")
                .seats
                .swap_remove(0)
                .controller
            {
                SeatController::Ai(profile) => (profile.politics, profile.hold_up),
                other => panic!("expected an AI chair, got {other:?}"),
            }
        };
        assert_eq!(ai(1, 1), (Politics::AttackLeader, HoldUp::Basic));
        assert_eq!(ai(2, 2), (Politics::Archenemy, HoldUp::ThreatAware));
        assert_eq!(
            ai(99, 99),
            (Politics::Random, HoldUp::None),
            "an opponent that plays the house default beats a table that \
             will not open"
        );
    }

    /// Capabilities are granted by the host, never asked for. The message
    /// this conversion reads has no field for them — `dev_mode` used to be
    /// one, and it arrived from whoever opened the socket.
    #[test]
    fn a_wire_preset_cannot_grant_itself_any_capability() {
        let msg = v1::GamePresetMsg {
            rules_metadata_json: Vec::new(),
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
            rules_metadata_json: Vec::new(),
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
            rule_print_ref: None,
            circled_colors: 0,
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
                free_mulligans: Some(3),
                starting_counters: vec![],
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
        assert_eq!(sent.house_rules.free_mulligan_count(), 3);
        assert_eq!(sent.house_rules.loop_policy, LoopPolicy::CompRulesDraw);
        assert_eq!(sent.house_rules.decision_timeout_secs, 45);
        assert_eq!(sent.house_rules.reconnect_window_secs, 90);
        assert!(sent.house_rules.takebacks);

        let unsent = from_proto(&msg(vec![seat_msg(), seat_msg()])).expect("two seats");
        assert_eq!(unsent.house_rules, HouseRules::default());
    }
    #[test]
    fn initial_counter_placements_survive_the_wire() {
        let mut message = msg(vec![seat_msg(), seat_msg()]);
        message.seats[0].starting_battlefield = vec![v1::DeckEntry {
            card_index: 5,
            print_ref: 0,
            rule_print_ref: None,
            circled_colors: 0,
        }];
        message.prints.push(v1::PrintInfo::default());
        message.house_rules = Some(v1::HouseRules {
            starting_counters: vec![v1::StartingCounters {
                seat: 0,
                permanent: 0,
                counters: vec![v1::StartingCounter {
                    kind: "charge".into(),
                    amount: 7,
                }],
            }],
            ..v1::HouseRules::default()
        });
        let preset = from_proto(&message).unwrap();
        assert_eq!(
            preset.house_rules.starting_counters[0].counters[0].amount,
            7
        );
        message.house_rules.as_mut().unwrap().starting_counters[0].permanent = 999;
        assert!(from_proto(&message).unwrap().validate().is_err());
    }

    #[test]
    fn physical_rules_references_and_circles_survive_the_transport() {
        let mut rules = baylee_core::card_metadata::RulesMetadata::default();
        let rule_print = rules
            .insert(CardIndex::new(5), &[Some("simic"), None], true)
            .unwrap();
        let mut message = msg(vec![seat_msg(), seat_msg()]);
        message.rules_metadata_json = serde_json::to_vec(&rules).unwrap();
        message.seats[0].deck = vec![v1::DeckEntry {
            card_index: 5,
            print_ref: 0,
            rule_print_ref: Some(u32::from(rule_print.get())),
            circled_colors: 3,
        }];
        let preset = from_proto(&message).unwrap();
        assert_eq!(preset.rules, rules);
        let entry = preset.seats[0].deck[0];
        assert_eq!(entry.rule_print, Some(rule_print));
        assert_eq!(entry.choices.colors().bits(), 3);
    }

    #[test]
    fn physical_rules_transport_rejects_truncated_references_and_invalid_circles() {
        for bad in [
            v1::DeckEntry {
                print_ref: 65_536,
                ..v1::DeckEntry::default()
            },
            v1::DeckEntry {
                rule_print_ref: Some(65_536),
                ..v1::DeckEntry::default()
            },
            v1::DeckEntry {
                circled_colors: 1,
                ..v1::DeckEntry::default()
            },
            v1::DeckEntry {
                circled_colors: 32,
                ..v1::DeckEntry::default()
            },
        ] {
            assert!(deck_entry(&bad).is_err());
        }
    }
}
