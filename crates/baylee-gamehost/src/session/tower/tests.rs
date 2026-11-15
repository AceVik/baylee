use super::*;
use baylee_core::preset::{DeckEntry, SeatSupplies};
use baylee_engine::{choice::PlayerAction, win::Victor};
use std::time::Duration;

mod recovery;

const ME: PlayerId = PlayerId::new(0);

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

fn fixture(spend: bool) -> (GamePreset, Vec<u8>) {
    let forest = baylee_cards::decks::by_name("Forest").unwrap();
    let mut preset = baylee_cards::decks::probe_preset(7, forest).unwrap();
    preset.format = FormatId::Custom;
    preset.house_rules.challenge_policy_seed = Some(31);
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.decision_timeout_secs = 0;
    preset.house_rules.reconnect_window_secs = 0;
    preset.house_rules.time_extension_votes = false;
    preset.house_rules.supplies = (0..2)
        .map(|seat| SeatSupplies {
            seat,
            healing: 1,
            mana: 1,
            seedling: false,
            concealed: false,
        })
        .collect();
    for (index, seat) in preset.seats.iter_mut().enumerate() {
        seat.controller = if index == 0 {
            SeatController::Open
        } else {
            SeatController::Ai(AIProfile::TOWER_EXPERT)
        };
        seat.starting_hand = None;
        seat.starting_battlefield.clear();
        seat.deck = vec![
            DeckEntry {
                card: forest,
                print: baylee_core::ids::PrintRef::new(0),
                rule_print: None,
                choices: baylee_core::card_metadata::DeckChoices::default()
            };
            40
        ];
    }
    let mut session = Session::new_recorded(&preset, record::this_build()).unwrap();
    for _ in 0..40 {
        session.pump_at_most(8);
        match session.engine.pending() {
            Pending::Mulligan { player, .. } if *player == ME => {
                session
                    .act_slice(ME, PlayerAction::MulliganKeep, 0)
                    .unwrap();
            }
            Pending::Priority { player, .. } if *player == ME => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    if spend {
        let sources = Sources::of(&session);
        for source in sources.0[0].iter().flatten() {
            session
                .act_slice(
                    ME,
                    PlayerAction::ActivateAbility {
                        source: source.id,
                        ability_index: 0,
                    },
                    0,
                )
                .unwrap();
        }
        session
            .act_slice(
                ME,
                PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue),
                0,
            )
            .unwrap();
    }
    session.act_slice(ME, PlayerAction::Concede, 0).unwrap();
    assert!(matches!(session.engine.pending(), Pending::GameOver(_)));
    (preset, session.take_record())
}

fn rewrite(bytes: &[u8], change: impl FnOnce(&mut Vec<Line>)) -> Vec<u8> {
    let mut lines: Vec<Line> = bytes[..bytes.len() - 1]
        .split(|b| *b == b'\n')
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    change(&mut lines);
    let mut bytes = Vec::new();
    for line in lines {
        serde_json::to_writer(&mut bytes, &line).unwrap();
        bytes.push(b'\n');
    }
    bytes
}

#[test]
fn tower_verification_recomputes_charge_payments_and_keeps_unused_items_on_elimination() {
    for spend in [false, true] {
        let (preset, bytes) = fixture(spend);
        let verified = verify(&preset, &bytes, deadline()).unwrap();
        assert_eq!(
            verified.result.winner,
            Some(Victor::Player(PlayerId::new(1)))
        );
        assert_eq!(verified.turn, 1);
        assert_eq!(verified.consumed, [[u8::from(spend); 2], [0; 2]]);
        assert_eq!(
            verified.record_sha256,
            <[u8; 32]>::from(Sha256::digest(&bytes))
        );
        assert_eq!(verify(&preset, &bytes, deadline()).unwrap(), verified);
    }
}

#[test]
fn tower_verification_rejects_wrong_package_sequence_hash_policy_and_terminal_claim() {
    let (preset, bytes) = fixture(false);
    let mutations: [fn(&mut Vec<Line>); 8] = [
        |lines| {
            if let Line::Header { preset, .. } = &mut lines[0] {
                preset.seed += 1;
            }
        },
        |lines| {
            if let Line::Header { build, .. } = &mut lines[0] {
                *build = "another-executor".into();
            }
        },
        |lines| {
            if let Line::Input { n, .. } = &mut lines[1] {
                *n += 1;
            }
        },
        |lines| {
            if let Line::Input { hash, .. } = &mut lines[1] {
                *hash = "0000000000000000".into();
            }
        },
        |lines| {
            if let Some(Line::Input { action, .. }) = lines.iter_mut().find(|line| {
                matches!(
                    line,
                    Line::Input {
                        by: record::Source::House,
                        ..
                    }
                )
            }) {
                *action = PlayerAction::Concede;
            }
        },
        |lines| {
            if let Some(Line::End { winners, .. }) = lines.last_mut() {
                *winners = vec![0];
            }
        },
        |lines| {
            lines.pop();
        },
        |lines| {
            lines.push(lines.last().unwrap().clone());
        },
    ];
    for (n, mutate) in mutations.into_iter().enumerate() {
        let forged = rewrite(&bytes, mutate);
        assert_ne!(forged, bytes, "mutation {n} must exercise its target");
        assert_eq!(
            verify(&preset, &forged, deadline()),
            Err(Refused::Invalid),
            "mutation {n}"
        );
    }
    assert_eq!(
        verify(&preset, &bytes[..bytes.len() - 1], deadline()),
        Err(Refused::Invalid)
    );
}

#[test]
fn tower_verification_bounds_input_before_parsing_or_replaying() {
    let (preset, bytes) = fixture(false);
    assert_eq!(
        verify(&preset, &bytes, Instant::now()),
        Err(Refused::Budget)
    );
    assert_eq!(
        verify(&preset, &vec![b'\n'; MAX_RECORD_BYTES + 1], deadline()),
        Err(Refused::Budget)
    );
    let mut oversized_line = vec![b'x'; MAX_LINE_BYTES + 1];
    oversized_line.push(b'\n');
    assert_eq!(
        verify(&preset, &oversized_line, deadline()),
        Err(Refused::Budget)
    );
    let oversized_count = b"{}\n".repeat(MAX_RECORD_LINES + 1);
    assert_eq!(
        verify(&preset, &oversized_count, deadline()),
        Err(Refused::Budget)
    );
}
