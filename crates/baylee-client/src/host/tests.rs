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
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
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
        rules: baylee_core::card_metadata::RulesMetadata::default(),
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
const DECK_FILE: &str = include_str!("../../../../data/acceptance-decks.txt");

/// A table of one seat is open as soon as it is dealt, and says so once,
/// last in its first batch (#256): the client holds its outbox until it
/// hears it, so an offline game that never said it could not be played.
#[test]
fn a_table_of_one_opens_on_its_first_poll() {
    let preset = demo_duel(DECK_FILE, 7).expect("the demo duel");
    let mut host = LocalHost::new(&preset, PlayerId::new(0), &["You", "House"]).expect("a game");
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

/// The table a player is given ([`LocalHost::into_installed`]) has the
/// house search, as a hosted table's does; a table built for a test plays
/// the level as its heuristics do unless it asks.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_players_table_searches_and_a_tests_table_does_not() {
    let mut preset = demo_duel(DECK_FILE, 42).expect("the shipped decks parse");
    preset.seats[1].controller = SeatController::Ai(AIProfile::EXPERT);
    let names = ["You", "House AI"];
    let me = PlayerId::new(0);
    let played = |mut host: LocalHost| {
        for _ in 0..2_000 {
            let _ = host.poll();
            if host.session.needs_pump() {
                continue;
            }
            if host.session.state().turn.number > 6
                || matches!(host.session.pending(), Pending::GameOver(_))
            {
                break;
            }
            let Some(action) = host.session.house_action(me) else {
                break;
            };
            host.submit(action);
        }
        host.session.foresight_stats().clone()
    };
    let plain = played(LocalHost::new(&preset, me, &names).expect("it starts"));
    assert_eq!(plain.asked, 0, "a test's table searched");
    let searching = played(
        LocalHost::new(&preset, me, &names)
            .expect("it starts")
            .searching(),
    );
    assert!(searching.searched > 0, "the player's table never searched");
    assert!(searching.most_work <= AIProfile::EXPERT_WORK + 10_000);
}

#[test]
fn demo_duel_builds_and_deals_a_two_seat_acceptance_preset() {
    let preset = demo_duel(DECK_FILE, 42).expect("the shipped decks parse");
    assert_eq!(preset.seats.len(), 2);
    assert!(matches!(preset.seats[0].controller, SeatController::Open));
    assert!(
        preset.seats[0].deck.len() >= 60,
        "the human seat gets a real deck, not an empty chair"
    );

    // And it starts, with the human dealt in.
    let mut host = LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"])
        .expect("the test duel starts");
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
    let frame = |log: Vec<u8>| Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: "g".to_string(),
            seq: 1,
            view_frame: baylee_view::ViewFrame::snapshot_of(
                &baylee_client_core::test_support::ViewBuilder::new(2).build(),
            ),
            log,
        })),
    };
    let told = LogTail {
        from: 3,
        entries: vec![LogEntry {
            step: None,
            at: 0,
            turn: 2,
            repeat: 1,
            event: LogEvent::TurnStarted {
                active: PlayerId::new(1),
            },
            phase: None,
        }],
    };
    let mut views = SeatViews::default();
    let Some(HostMessage::View(_, Some(got))) =
        host_message(frame(baylee_core::wire::encode(&told)), &mut views)
    else {
        panic!("the frame's lines were not read");
    };
    assert_eq!(got, told);
    assert!(
        matches!(
            host_message(frame(Vec::new()), &mut views),
            Some(HostMessage::View(_, None))
        ),
        "a frame with nothing new in its log is not a view"
    );
    assert!(
        matches!(
            host_message(frame(b"{\"from\":".to_vec()), &mut views),
            Some(HostMessage::View(_, None))
        ),
        "a log that does not decode took its view with it"
    );
}

/// The frame of `view` at `seq`, as `sender` writes it.
fn told(sender: &mut baylee_view::ViewSender, view: &PlayerView) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: "g".to_string(),
            seq: view.seq,
            view_frame: sender.next(view),
            log: Vec::new(),
        })),
    }
}

#[test]
fn a_patch_reaches_the_table_as_the_whole_view_it_leaves() {
    let mut sender = baylee_view::ViewSender::default();
    let mut views = SeatViews::default();
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    host_message(told(&mut sender, &view), &mut views).expect("a view");
    view.seq += 1;
    view.turn += 4;
    let Some(HostMessage::View(got, None)) = host_message(told(&mut sender, &view), &mut views)
    else {
        panic!("a patch is a view to the table");
    };
    assert_eq!(*got, view);
    assert!(!views.take_lost());
}

/// Printings that cannot be placed cost a question too, and no failure in
/// the player's face.
#[test]
fn printings_that_will_not_place_ask_for_a_whole_view() {
    let frame = Envelope {
        msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
            game_id: "g".to_string(),
            view_version: baylee_view::VIEW_VERSION,
            statics: Vec::new(),
            more: baylee_core::wire::encode(&baylee_view::MorePrints {
                held: 9,
                prints: Vec::new(),
            }),
        })),
    };
    let mut views = SeatViews::default();
    assert!(host_message(frame, &mut views).is_none());
    assert!(views.take_lost(), "the table has to be asked");
}

/// A lost frame costs a question and nothing else: no failure in the
/// player's face, the table standing as it was, and one ask for however
/// many frames come before the whole view.
#[test]
fn a_frame_that_will_not_place_asks_for_a_whole_view_once() {
    let mut sender = baylee_view::ViewSender::default();
    let mut views = SeatViews::default();
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    host_message(told(&mut sender, &view), &mut views).expect("a view");
    for step in 0..3 {
        view.seq += 1;
        view.turn += 1;
        let _lost = (step == 0).then(|| told(&mut sender, &view));
    }
    view.seq += 1;
    let frame = told(&mut sender, &view);
    assert!(host_message(frame, &mut views).is_none());
    assert!(views.take_lost(), "the table has to be asked");
    view.seq += 1;
    let frame = told(&mut sender, &view);
    assert!(host_message(frame, &mut views).is_none());
    assert!(!views.take_lost(), "and only once");
    sender.forget();
    let Some(HostMessage::View(got, None)) = host_message(told(&mut sender, &view), &mut views)
    else {
        panic!("the whole view mends it");
    };
    assert_eq!(*got, view);
}

#[test]
fn split_log_frames_sharing_a_seq_fold_into_one_view_message() {
    use baylee_view::{LogEntry, LogEvent};
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.seq = 7;
    let view_frame = baylee_view::ViewFrame::snapshot_of(&view);
    let entry = |turn| LogEntry {
        step: None,
        at: 0,
        turn,
        repeat: 1,
        event: LogEvent::TurnStarted {
            active: PlayerId::new(0),
        },
        phase: None,
    };
    let first = Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: "g".to_string(),
            seq: 7,
            view_frame,
            log: baylee_core::wire::encode(&LogTail {
                from: 10,
                entries: vec![entry(1), entry(2)],
            }),
        })),
    };
    // A continuation chunk carries no view, only the next lines under the
    // same `seq`.
    let second = Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: "g".to_string(),
            seq: 7,
            view_frame: Vec::new(),
            log: baylee_core::wire::encode(&LogTail {
                from: 12,
                entries: vec![entry(3)],
            }),
        })),
    };
    let mut out = Vec::new();
    let mut views = SeatViews::default();
    push_host_message(&mut out, first, &mut views);
    push_host_message(&mut out, second, &mut views);
    assert_eq!(out.len(), 1);
    let HostMessage::View(got_view, Some(got_tail)) = &out[0] else {
        panic!("expected one merged View message");
    };
    assert_eq!(got_view.seq, 7);
    assert_eq!(got_tail.from, 10);
    assert_eq!(got_tail.entries, vec![entry(1), entry(2), entry(3)]);
}

/// The version rides outside the payload so this check can happen before
/// the decode — the payload's shape is exactly what a bump changes.
#[test]
fn a_table_speaking_a_view_version_this_client_cannot_render_is_refused() {
    let envelope = Envelope {
        msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
            game_id: "g".to_string(),
            view_version: baylee_view::VIEW_VERSION + 1,
            statics: b"{}".to_vec(),
            more: Vec::new(),
        })),
    };
    let Some(HostMessage::Failed(reason)) = host_message(envelope, &mut SeatViews::default())
    else {
        panic!("a version this client cannot render is refused, not rendered");
    };
    assert!(
        reason.contains(&(baylee_view::VIEW_VERSION + 1).to_string()),
        "{reason}"
    );
}

#[test]
fn the_opening_poll_delivers_a_view_and_a_choice_for_the_local_seat() {
    let mut host = LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
    let messages = host.poll();
    assert!(
        messages.iter().any(|m| matches!(m, HostMessage::View(..))),
        "the client must be able to draw before it is asked anything"
    );
    assert!(messages.iter().any(|m| matches!(m, HostMessage::Choice(_))));
}

#[test]
fn a_view_survives_the_wire_encoding_intact() {
    let mut host = LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
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
    let mut host = LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
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
    let mut host = LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
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
    let mut host = LocalHost::new(&duel_preset(), PlayerId::new(0), &["You", "AI"]).expect("host");
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
