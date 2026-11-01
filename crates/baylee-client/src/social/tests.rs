//! The chat's client half, below the renderer: what a press is, the order
//! lines arrived in, when the chat holds the keys, how a body is cut into
//! pieces and a line read back as plain text, which tabs are drawn.
//!
//! The machine's own behaviour is tested in `baylee-client-core`; what is
//! asked here is only what this crate adds to it.

use std::collections::HashSet;

use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{
    Body, BodyLink, BodyMention, ChatLine, ChatPhrase, ChatPrefs, ChatTab, ConvId, LineKey,
    LineKind, LineState, Me, RosterSeat, Social, TableContext,
};
use baylee_protocol::social::{Conv, Person};
use bevy::prelude::*;

use super::rows::{self, Piece};
use super::window;
use super::{Arrivals, ChatUi, OsEntropy};

// Pointer release contracts run the real drawer handler and Bevy dispatch
// in `drawer::tests`; each other social surface tests its own handler too.

// ── Arrivals ────────────────────────────────────────────────────────────

fn key(conv: &ConvId, id: &str) -> LineKey {
    LineKey {
        conv: conv.clone(),
        id: id.into(),
    }
}

fn table_conv() -> ConvId {
    Conv::Table { game: "g1".into() }
}

#[test]
fn stamps_grow_in_the_order_things_arrived() {
    let mut a = Arrivals::default();
    a.log_grew(3);
    let first_chat = a.chat_line(&key(&table_conv(), "m1"));
    a.log_grew(5);
    let second_chat = a.chat_line(&key(&table_conv(), "m2"));
    assert!(a.log_stamp(2) < first_chat);
    assert!(first_chat < a.log_stamp(3));
    assert!(a.log_stamp(4) < second_chat);
}

#[test]
fn log_lines_that_arrived_together_share_a_stamp() {
    let mut a = Arrivals::default();
    a.log_grew(4);
    let stamp = a.log_stamp(0);
    assert_ne!(stamp, 0, "a stamped line has a stamp");
    for index in 1..4 {
        assert_eq!(a.log_stamp(index), stamp, "line {index}");
    }
}

#[test]
fn a_log_of_the_same_length_stamps_nothing_new() {
    let mut a = Arrivals::default();
    a.log_grew(2);
    a.log_grew(2);
    a.log_grew(2);
    assert_eq!(a.log.len(), 1);
    let next = a.chat_line(&key(&table_conv(), "m1"));
    assert_eq!(next, a.log_stamp(0) + 1, "nothing was spent in between");
}

#[test]
fn a_shorter_log_restarts_under_one_new_stamp() {
    let mut a = Arrivals::default();
    a.log_grew(5);
    a.log_grew(8);
    let before = a.log_stamp(7);
    // A reconnect replays the book from its start.
    a.log_grew(2);
    assert_eq!(a.log.len(), 1, "one mark for all of it");
    assert_eq!(a.log[0].0, 0);
    assert!(
        a.log_stamp(0) > before,
        "the replay is newer than anything seen"
    );
    assert_eq!(a.log_stamp(1), a.log_stamp(0));
    // And it grows on from there.
    a.log_grew(3);
    assert!(a.log_stamp(2) > a.log_stamp(1));
}

#[test]
fn an_emptied_log_has_no_stamps() {
    let mut a = Arrivals::default();
    a.log_grew(3);
    a.log_grew(0);
    assert!(a.log.is_empty());
    assert_eq!(a.log_stamp(0), 0);
}

#[test]
fn a_log_line_before_any_arrival_has_stamp_zero() {
    assert_eq!(Arrivals::default().log_stamp(0), 0);
    assert_eq!(Arrivals::default().log_stamp(9), 0);
}

#[test]
fn a_known_chat_line_keeps_its_stamp() {
    let mut a = Arrivals::default();
    let line = key(&table_conv(), "m1");
    let first = a.chat_line(&line);
    a.log_grew(3);
    let other = a.chat_line(&key(&table_conv(), "m2"));
    assert_eq!(a.chat_line(&line), first, "asked again");
    assert!(other > first);
    assert_eq!(a.chat.len(), 2, "no third entry");
    // The same id in another conversation is another line.
    let dm = Conv::dm("af03", "0012").expect("two tags");
    assert_ne!(a.chat_line(&key(&dm, "m1")), first);
}

// ── ChatUi ──────────────────────────────────────────────────────────────

#[test]
fn an_idle_chat_holds_no_keys() {
    assert!(!ChatUi::default().holds_keyboard());
    let open = ChatUi {
        open: true,
        drawer: true,
        ..ChatUi::default()
    };
    assert!(
        !open.holds_keyboard(),
        "an open window is not a caret in a box"
    );
}

#[test]
fn typing_holds_the_keys() {
    let ui = ChatUi {
        typing: true,
        ..ChatUi::default()
    };
    assert!(ui.holds_keyboard());
}

#[test]
fn the_frame_the_composer_let_go_still_holds_the_keys() {
    let ui = ChatUi {
        swallow: true,
        ..ChatUi::default()
    };
    assert!(ui.holds_keyboard());
}

// ── rows ────────────────────────────────────────────────────────────────

fn body(text: &str, links: &[[usize; 2]], mentions: &[[usize; 2]]) -> Body {
    Body {
        text: text.into(),
        links: links
            .iter()
            .enumerate()
            .map(|(i, at)| BodyLink {
                at: *at,
                card: 100 + u32::try_from(i).expect("few links"),
                face: 0,
                print: None,
            })
            .collect(),
        mentions: mentions
            .iter()
            .map(|at| BodyMention {
                at: *at,
                tag: "0012".into(),
            })
            .collect(),
        emote: false,
    }
}

fn words(text: &str) -> (String, Piece) {
    (text.into(), Piece::Words)
}

fn link(card: u32) -> Piece {
    Piece::Link(rows::ChatCardLink {
        card,
        face: 0,
        print: None,
    })
}

/// The pieces always read back as the whole text.
fn rejoined(pieces: &[(String, Piece)]) -> String {
    pieces.iter().map(|(t, _)| t.as_str()).collect()
}

#[test]
fn a_body_without_marks_is_one_run_of_words() {
    assert_eq!(
        rows::pieces(&body("good game", &[], &[])),
        [words("good game")]
    );
}

#[test]
fn an_empty_body_has_no_pieces() {
    assert!(rows::pieces(&body("", &[], &[])).is_empty());
}

#[test]
fn a_link_cuts_the_text_around_it() {
    let b = body("cast [Opt] now", &[[5, 10]], &[]);
    assert_eq!(
        rows::pieces(&b),
        [words("cast "), ("[Opt]".into(), link(100)), words(" now"),]
    );
}

#[test]
fn marks_are_cut_in_text_order_whatever_order_they_come_in() {
    // The link is listed first and stands last.
    let b = body("[@Bob#0012] sees [Opt]", &[[17, 22]], &[[0, 11]]);
    let p = rows::pieces(&b);
    assert_eq!(
        p,
        [
            ("[@Bob#0012]".into(), Piece::Mention),
            words(" sees "),
            ("[Opt]".into(), link(100)),
        ]
    );
    assert_eq!(rejoined(&p), b.text);
}

#[test]
fn spans_count_characters_not_bytes() {
    // "Grüße " is six characters and eight bytes; "[Lötus]" seven and eight.
    let b = body("Grüße [Lötus] 日本", &[[6, 13]], &[]);
    let p = rows::pieces(&b);
    assert_eq!(
        p,
        [
            words("Grüße "),
            ("[Lötus]".into(), link(100)),
            words(" 日本"),
        ]
    );
    assert_eq!(rejoined(&p), b.text);
}

#[test]
fn a_mark_in_the_last_characters_leaves_no_trailing_words() {
    let b = body("see 日本[X]", &[[6, 9]], &[]);
    let p = rows::pieces(&b);
    assert_eq!(p.last(), Some(&("[X]".into(), link(100))));
    assert_eq!(rejoined(&p), b.text);
}

#[test]
fn a_span_past_the_text_is_ignored() {
    let b = body("short", &[[2, 40]], &[]);
    assert_eq!(rows::pieces(&b), [words("short")]);
}

#[test]
fn an_empty_or_backwards_span_is_ignored() {
    let b = body("abcdef", &[[3, 3]], &[[4, 2]]);
    assert_eq!(rows::pieces(&b), [words("abcdef")]);
}

#[test]
fn of_two_overlapping_spans_the_earlier_wins() {
    let b = body("abcdefgh", &[[1, 5]], &[[3, 7]]);
    let p = rows::pieces(&b);
    assert_eq!(p, [words("a"), ("bcde".into(), link(100)), words("fgh"),]);
    assert_eq!(rejoined(&p), b.text, "nothing drawn twice, nothing lost");
}

#[test]
fn neighbouring_spans_leave_no_empty_run_between_them() {
    let b = body("abcdef", &[[0, 3]], &[[3, 6]]);
    assert_eq!(
        rows::pieces(&b),
        [("abc".into(), link(100)), ("def".into(), Piece::Mention),]
    );
}

#[test]
fn the_clock_reads_hours_and_minutes() {
    let text = rows::clock(1_800_000_000_000);
    let (h, m) = text.split_once(':').expect("HH:MM");
    assert_eq!((h.len(), m.len()), (2, 2), "{text}");
    assert!(h.parse::<u32>().expect("hours") < 24, "{text}");
    assert!(m.parse::<u32>().expect("minutes") < 60, "{text}");
}

#[test]
fn the_clock_moves_a_minute_per_minute_and_not_per_second() {
    // On a minute boundary; compared by minutes alone, which a whole-hour
    // zone cannot change, and which a local offset that the platform
    // refuses to name mid-test (it reads as UTC) cannot either.
    let base = 1_800_000_000_000_u64;
    let minute = |ms| {
        rows::clock(ms)
            .split_once(':')
            .and_then(|(_, m)| m.parse::<u64>().ok())
            .expect("minutes")
    };
    assert_eq!(minute(base), minute(base + 59_999), "same minute");
    assert_eq!(minute(base + 60_000), (minute(base) + 1) % 60);
}

// ── plain_text and prefix ───────────────────────────────────────────────

const ME: &str = "af03";
const BOB: &str = "0012";

fn bob() -> Person {
    Person {
        tag: BOB.into(),
        handle: "Bob#0012".into(),
    }
}

/// A machine for Alice, seated at table `g1` with Bob (and her own team's
/// chair), so that a whisper partner has a name.
fn machine(guest: bool) -> Social {
    let me = Me {
        tag: ME.into(),
        handle: "Alice#af03".into(),
        guest,
        gateway: "http://127.0.0.1:1".into(),
    };
    let (mut social, _) =
        Social::start(me, None, None, ChatPrefs::default(), 1_000, &mut OsEntropy);
    social.on_table(Some(TableContext {
        game: "g1".into(),
        playing: true,
        roster: vec![
            RosterSeat {
                seat: 0,
                handle: "Alice#af03".into(),
                team: Some(1),
            },
            RosterSeat {
                seat: 1,
                handle: "Bob#0012".into(),
                team: Some(1),
            },
        ],
        me_seat: Some(0),
        my_team: Some(1),
        watching: false,
    }));
    social
}

fn line(conv: &ConvId, id: &str, at: u64, kind: LineKind) -> ChatLine {
    ChatLine {
        key: key(conv, id),
        at,
        kind,
        state: LineState::Final,
        hidden: false,
        mentions_me: false,
        failure: None,
    }
}

fn said(text: &str, mine: bool) -> LineKind {
    LineKind::Message {
        from: if mine {
            Person {
                tag: ME.into(),
                handle: "Alice#af03".into(),
            }
        } else {
            bob()
        },
        mine,
        body: Body {
            text: text.into(),
            ..Body::default()
        },
    }
}

const AT: u64 = 1_800_000_000_000;

#[test]
fn a_table_line_reads_time_channel_name_and_text() {
    let social = machine(false);
    let conv = table_conv();
    let l = line(&conv, "m1", AT, said("good luck", false));
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        format!("{} [Table] Bob#0012: good luck", rows::clock(AT))
    );
}

#[test]
fn a_team_line_carries_the_team_prefix() {
    let social = machine(false);
    let conv = Conv::Team {
        game: "g1".into(),
        team: 1,
    };
    let l = line(&conv, "m1", AT, said("hold the blocker", false));
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        format!("{} [Team] Bob#0012: hold the blocker", rows::clock(AT))
    );
}

#[test]
fn a_received_whisper_names_its_sender_and_no_colon() {
    let social = machine(false);
    let conv = Conv::dm(ME, BOB).expect("two tags");
    let l = line(&conv, "m1", AT, said("psst", false));
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        format!("{} From Bob: psst", rows::clock(AT))
    );
}

#[test]
fn a_sent_whisper_names_its_addressee() {
    let social = machine(false);
    let conv = Conv::dm(ME, BOB).expect("two tags");
    let l = line(&conv, "m1", AT, said("psst", true));
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        format!("{} To Bob: psst", rows::clock(AT))
    );
}

#[test]
fn an_emote_is_drawn_as_a_star_and_the_name() {
    let social = machine(false);
    let conv = table_conv();
    let mut l = line(&conv, "m1", AT, said("waves", false));
    if let LineKind::Message { body, .. } = &mut l.kind {
        body.emote = true;
    }
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        format!("{} [Table] * Bob#0012 waves", rows::clock(AT))
    );
}

#[test]
fn a_notice_is_the_time_and_its_sentence() {
    let social = machine(false);
    let conv = table_conv();
    let l = line(
        &conv,
        "n1",
        AT,
        LineKind::Notice {
            phrase: ChatPhrase::NoTable,
            args: Vec::new(),
        },
    );
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        format!("{} {}", rows::clock(AT), ChatPhrase::NoTable.text(Lang::En))
    );
}

#[test]
fn a_divider_is_only_its_words() {
    let social = machine(false);
    let conv = table_conv();
    let l = line(&conv, "divider", 0, LineKind::Divider);
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        ChatPhrase::NewTable.text(Lang::En)
    );
}

#[test]
fn the_all_tab_prefixes_a_table_and_a_team_line() {
    let social = machine(false);
    let table = table_conv();
    let team = Conv::Team {
        game: "g1".into(),
        team: 1,
    };
    let l = line(&table, "m1", AT, said("x", false));
    assert_eq!(
        rows::prefix(&social, &table, &l, Lang::En).as_deref(),
        Some("[Table]")
    );
    assert_eq!(
        rows::prefix(&social, &team, &l, Lang::En).as_deref(),
        Some("[Team]")
    );
    assert_eq!(
        rows::prefix(&social, &table, &l, Lang::De).as_deref(),
        Some("[Tisch]")
    );
}

#[test]
fn a_whisper_between_two_others_has_no_prefix() {
    let social = machine(false);
    let conv = Conv::dm("1111", "2222").expect("two tags");
    let l = line(&conv, "m1", AT, said("x", false));
    assert_eq!(rows::prefix(&social, &conv, &l, Lang::En), None);
}

// ── window ──────────────────────────────────────────────────────────────

#[test]
fn a_seated_member_in_a_game_sees_every_tab_in_order() {
    let social = machine(false);
    assert_eq!(
        window::drawn_tabs(&social, true),
        [
            ChatTab::All,
            ChatTab::Global,
            ChatTab::Log,
            ChatTab::Table,
            ChatTab::Team,
            ChatTab::Private,
            ChatTab::Group,
        ]
    );
}

#[test]
fn the_log_tab_is_drawn_only_while_a_game_is_open() {
    let social = machine(false);
    assert!(window::drawn_tabs(&social, true).contains(&ChatTab::Log));
    assert!(!window::drawn_tabs(&social, false).contains(&ChatTab::Log));
}

#[test]
fn off_a_table_only_all_private_and_group_are_drawn() {
    let mut social = machine(false);
    social.on_table(None);
    assert_eq!(
        window::drawn_tabs(&social, false),
        [
            ChatTab::All,
            ChatTab::Global,
            ChatTab::Private,
            ChatTab::Group
        ]
    );
}

#[test]
fn a_guest_has_no_group_tab() {
    let mut social = machine(true);
    social.on_table(None);
    assert_eq!(
        window::drawn_tabs(&social, false),
        [ChatTab::All, ChatTab::Global, ChatTab::Private]
    );
}

#[test]
fn a_table_without_teams_draws_no_team_tab() {
    let mut social = machine(false);
    social.on_table(Some(TableContext {
        game: "g2".into(),
        playing: false,
        roster: Vec::new(),
        me_seat: Some(0),
        my_team: None,
        watching: false,
    }));
    let tabs = window::drawn_tabs(&social, false);
    assert!(tabs.contains(&ChatTab::Table));
    assert!(!tabs.contains(&ChatTab::Team));
}

#[test]
fn a_watcher_has_no_table_or_team_tab() {
    let mut social = machine(false);
    social.on_table(Some(TableContext {
        game: "g2".into(),
        playing: true,
        roster: Vec::new(),
        me_seat: None,
        my_team: Some(1),
        watching: true,
    }));
    let tabs = window::drawn_tabs(&social, true);
    assert!(!tabs.contains(&ChatTab::Table));
    assert!(!tabs.contains(&ChatTab::Team));
}

#[test]
fn every_tab_has_its_own_name_in_both_languages() {
    let english = ["All", "Lobby", "Log", "Table", "Team", "Private", "Group"];
    for (tab, name) in ChatTab::ALL.into_iter().zip(english) {
        assert_eq!(window::tab_name(tab, Lang::En), name);
    }
    let german: HashSet<&str> = ChatTab::ALL
        .into_iter()
        .map(|tab| window::tab_name(tab, Lang::De))
        .collect();
    assert_eq!(german.len(), ChatTab::ALL.len(), "no two tabs share a name");
    assert!(german.iter().all(|n| !n.is_empty()));
}

#[test]
fn the_lobby_panel_is_thirty_eight_percent_of_the_window() {
    assert!((window::lobby_panel_height(1000.0) - 380.0).abs() < 1e-3);
    assert!((window::lobby_panel_height(1200.0) - 456.0).abs() < 1e-3);
}

#[test]
fn the_lobby_panel_never_shrinks_below_320_or_grows_past_520() {
    for h in [0.0, 300.0, 600.0, 842.0] {
        assert!((window::lobby_panel_height(h) - 320.0).abs() < 1e-3, "{h}");
    }
    for h in [1369.0, 2000.0, 10_000.0] {
        assert!((window::lobby_panel_height(h) - 520.0).abs() < 1e-3, "{h}");
    }
}

// ── the header's chat buttons, the swallowed opening key, line copy ─────

/// A host with a machine in it, as the lobby has once a session stands.
fn host_with_machine() -> super::SocialHost {
    super::SocialHost {
        machine: Some(machine(false)),
        ..super::SocialHost::default()
    }
}

mod header_ask {
    use super::*;
    use crate::lobby::ShellAsk;
    use crate::social::sheet::Sheet;
    use crate::social::{SocialHost, header_ask};

    #[test]
    fn with_no_session_no_ask_does_anything() {
        let mut host = SocialHost::default();
        assert!(!host.active());
        for ask in [
            ShellAsk::Chat,
            ShellAsk::Friends,
            ShellAsk::ChatPrivacy,
            ShellAsk::Shortcuts,
            ShellAsk::Report,
        ] {
            let mut ui = ChatUi::default();
            header_ask(&mut host, &mut ui, ask);
            assert!(!ui.open && !ui.typing && !ui.drawer, "{ask:?}");
            assert!(ui.sheet.is_none(), "{ask:?}");
        }
    }

    #[test]
    fn friends_toggles_the_drawer_and_touches_nothing_else() {
        let mut host = host_with_machine();
        let mut ui = ChatUi::default();
        header_ask(&mut host, &mut ui, ShellAsk::Friends);
        assert!(ui.drawer, "the first ask opens the drawer");
        assert!(!ui.open && !ui.typing && ui.sheet.is_none());
        header_ask(&mut host, &mut ui, ShellAsk::Friends);
        assert!(!ui.drawer, "the second shuts it");
    }

    #[test]
    fn chat_privacy_opens_the_privacy_sheet() {
        let mut host = host_with_machine();
        let mut ui = ChatUi::default();
        header_ask(&mut host, &mut ui, ShellAsk::ChatPrivacy);
        assert!(matches!(ui.sheet, Some(Sheet::Privacy)));
        assert!(!ui.drawer && !ui.open);
        // Asking again keeps it up: the row opens, it never toggles.
        header_ask(&mut host, &mut ui, ShellAsk::ChatPrivacy);
        assert!(matches!(ui.sheet, Some(Sheet::Privacy)));
    }

    #[test]
    fn the_asks_that_are_not_the_chats_leave_it_alone() {
        let mut host = host_with_machine();
        let mut ui = ChatUi::default();
        header_ask(&mut host, &mut ui, ShellAsk::Shortcuts);
        header_ask(&mut host, &mut ui, ShellAsk::Report);
        assert!(!ui.open && !ui.typing && !ui.drawer && ui.sheet.is_none());
    }

    #[test]
    fn chat_opens_the_panel_with_the_caret_and_the_second_ask_hides_it() {
        let mut host = host_with_machine();
        let mut ui = ChatUi::default();
        header_ask(&mut host, &mut ui, ShellAsk::Chat);
        assert!(ui.open && ui.typing && !ui.minimised);
        assert_eq!(ui.field, crate::social::Field::Composer);
        header_ask(&mut host, &mut ui, ShellAsk::Chat);
        assert!(
            !ui.open && !ui.typing,
            "an open panel is shut by the button"
        );
        // A minimised panel is brought back, not shut.
        ui.open = true;
        ui.minimised = true;
        header_ask(&mut host, &mut ui, ShellAsk::Chat);
        assert!(ui.open && !ui.minimised && ui.typing);
    }
}

mod swallowed_key {
    use bevy::ecs::message::Messages;
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput};

    use super::*;
    use crate::social::SocialHost;

    /// The chat's keys alone: the schedules are run by hand, so that the
    /// frame the box took the caret (Update) and the next frame's start
    /// (`PreUpdate`) are two separate steps.
    fn app(ui: ChatUi) -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .insert_resource(host_with_machine())
            .insert_resource(ui);
        crate::social::keys::install(&mut app);
        app
    }

    fn press(app: &mut App, ch: &str) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::KeyA,
                logical_key: Key::Character(ch.into()),
                state: ButtonState::Pressed,
                text: Some(ch.into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
    }

    fn update(app: &mut App) {
        app.world_mut().run_schedule(Update);
    }

    fn next_frame(app: &mut App) {
        app.world_mut().run_schedule(PreUpdate);
    }

    fn composed(app: &App) -> String {
        app.world()
            .resource::<SocialHost>()
            .machine()
            .expect("a machine")
            .composer()
            .text()
            .to_string()
    }

    #[test]
    fn a_new_frame_always_lets_go_of_the_swallow() {
        // Typing or not: the old rule kept it while the box had the caret.
        for typing in [false, true] {
            let mut app = app(ChatUi {
                typing,
                swallow: true,
                ..ChatUi::default()
            });
            next_frame(&mut app);
            assert!(!app.world().resource::<ChatUi>().swallow, "typing {typing}");
        }
    }

    #[test]
    fn the_key_that_opened_the_box_is_not_typed_into_it() {
        let mut app = app(ChatUi {
            typing: true,
            swallow: true,
            ..ChatUi::default()
        });
        press(&mut app, "a");
        update(&mut app);
        assert_eq!(composed(&app), "", "the opening key is not the box's");
        assert!(
            app.world().resource::<ChatUi>().swallow,
            "the frame is not over"
        );
        // The next frame, the box is the keys' again.
        next_frame(&mut app);
        press(&mut app, "b");
        update(&mut app);
        assert_eq!(
            composed(&app),
            "b",
            "a typed character lands, and the swallowed one is not replayed"
        );
    }

    #[test]
    fn keys_pressed_while_not_typing_are_not_replayed_when_typing_starts() {
        let mut app = app(ChatUi::default());
        press(&mut app, "x");
        press(&mut app, "y");
        update(&mut app);
        assert_eq!(composed(&app), "");
        // Later the box takes the caret: what was pressed before is gone.
        next_frame(&mut app);
        app.world_mut().resource_mut::<ChatUi>().typing = true;
        update(&mut app);
        assert_eq!(composed(&app), "", "the old presses are not the box's");
        press(&mut app, "z");
        update(&mut app);
        assert_eq!(composed(&app), "z");
    }

    #[test]
    fn a_swallowed_key_is_dropped_whatever_it_is() {
        // Enter is the key that opens the box from the shell: no newline,
        // no submit, no close from it.
        let mut app = app(ChatUi {
            typing: true,
            swallow: true,
            ..ChatUi::default()
        });
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: Key::Enter,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        update(&mut app);
        let ui = app.world().resource::<ChatUi>();
        assert!(ui.typing, "the box keeps the caret");
        assert_eq!(composed(&app), "");
    }
}

mod line_copy {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{PointerOver, PointerPress};
    use bevy::picking::pointer::{Location, PointerButton, PointerId};

    use super::*;
    use crate::hud::UiFonts;
    use crate::social::SocialHost;
    use crate::social::menu::{self, Copyable, Selected};

    const BASE: Color = Color::srgb(0.2, 0.3, 0.4);

    struct Rig {
        app: App,
        list: Entity,
        rows: Vec<Entity>,
    }

    /// A list with three rows, spawned out of order (`order` 2, 0, 1), with
    /// no ground of their own (most chat rows have none).
    fn rig() -> Rig {
        rig_of(false)
    }

    /// The same, each row with a ground of its own (`BASE`), as a row that
    /// mentions the player has.
    fn rig_with_grounds() -> Rig {
        rig_of(true)
    }

    fn rig_of(grounds: bool) -> Rig {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .insert_resource(UiFonts::default())
            .insert_resource(SocialHost::default())
            .init_resource::<crate::social::zoom::Zoom>()
            .insert_resource(ChatUi {
                open: true,
                ..ChatUi::default()
            })
            .add_message::<bevy::picking::events::PointerClick>()
            .add_message::<PointerPress>()
            .add_message::<PointerOver>();
        menu::install(&mut app);
        let list = app.world_mut().spawn(Node::default()).id();
        let rows = [2usize, 0, 1]
            .into_iter()
            .map(|order| {
                let row = app
                    .world_mut()
                    .spawn((
                        Node::default(),
                        Copyable::new(order, format!("text{order}")),
                        ChildOf(list),
                    ))
                    .id();
                if grounds {
                    app.world_mut()
                        .entity_mut(row)
                        .insert(BackgroundColor(BASE));
                }
                row
            })
            .collect();
        app.update();
        Rig { app, list, rows }
    }

    impl Rig {
        /// The row with this `order`.
        fn row(&self, order: usize) -> Entity {
            self.rows[[1, 2, 0][order]]
        }

        fn location() -> Location {
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            }
        }

        fn press(&mut self, order: usize, button: PointerButton) {
            let entity = self.row(order);
            let event = crate::test_pointer::Press {
                button,
                hit: HitData::new(entity, 0.0, None, None),
                count: 1,
            };
            self.app.world_mut().write_message(crate::test_pointer::aim(
                PointerId::Mouse,
                Self::location(),
                event,
                entity,
            ));
        }

        fn over(&mut self, order: usize) {
            let entity = self.row(order);
            let event = crate::test_pointer::Over {
                hit: HitData::new(entity, 0.0, None, None),
            };
            self.app.world_mut().write_message(crate::test_pointer::aim(
                PointerId::Mouse,
                Self::location(),
                event,
                entity,
            ));
        }

        /// A primary press on a line, with the button held, and one frame.
        fn select(&mut self, order: usize) {
            self.app
                .world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            self.press(order, PointerButton::Primary);
            self.app.update();
        }

        /// The pointer drags over a line, one frame.
        fn drag_to(&mut self, order: usize) {
            self.over(order);
            self.app.update();
        }

        /// Ctrl+C, one frame, and what it put on the clipboard.
        fn copy(&mut self) -> Option<String> {
            {
                let mut keys = self.app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.press(KeyCode::ControlLeft);
                keys.press(KeyCode::KeyC);
            }
            self.app.update();
            self.app
                .world_mut()
                .resource_mut::<SocialHost>()
                .copy
                .take()
        }

        fn ground(&self, order: usize) -> Option<Color> {
            self.app
                .world()
                .get::<BackgroundColor>(self.row(order))
                .map(|g| g.0)
        }
    }

    /// A row that wears nothing: no ground, or a see-through one.
    fn bare(ground: Option<Color>) -> bool {
        ground.is_none_or(|c| c.alpha() == 0.0)
    }

    fn selected() -> Color {
        crate::shellkit::tokens::SELECTED.with_alpha(0.5)
    }

    #[test]
    fn a_drag_over_two_lines_copies_them_in_the_lists_order() {
        let mut rig = rig();
        rig.select(0);
        rig.drag_to(1);
        assert_eq!(rig.copy().as_deref(), Some("text0\ntext1"));
    }

    #[test]
    fn a_drag_upwards_copies_the_same_lines() {
        let mut rig = rig();
        rig.select(2);
        rig.drag_to(1);
        rig.drag_to(0);
        assert_eq!(rig.copy().as_deref(), Some("text0\ntext1\ntext2"));
    }

    #[test]
    fn a_single_pressed_line_copies_alone() {
        let mut rig = rig();
        rig.select(1);
        assert_eq!(rig.copy().as_deref(), Some("text1"));
    }

    #[test]
    fn selected_lines_wear_the_selection_and_the_rest_their_own_ground() {
        let mut rig = rig();
        rig.select(0);
        rig.drag_to(1);
        assert_eq!(rig.ground(0), Some(selected()));
        assert_eq!(rig.ground(1), Some(selected()));
        assert!(bare(rig.ground(2)), "the third is outside the span");
        // Dragging back shrinks the span, and the line left gets its ground back.
        rig.drag_to(0);
        assert_eq!(rig.ground(0), Some(selected()));
        assert!(bare(rig.ground(1)), "the line left is let go");
    }

    #[test]
    fn a_selected_line_gets_its_own_ground_back_when_let_go() {
        // Only the middle line has a ground of its own.
        let mut rig = rig();
        let own = rig.row(1);
        rig.app
            .world_mut()
            .entity_mut(own)
            .insert(BackgroundColor(BASE));
        rig.app.update();
        rig.select(1);
        assert_eq!(rig.ground(1), Some(selected()));
        rig.app.world_mut().resource_mut::<ChatUi>().open = false;
        rig.app.update();
        assert_eq!(rig.ground(1), Some(BASE));
    }

    /// Painting rewrites every row of the list, so a row that is not in the
    /// span and has a ground of its own (a line that mentions the player:
    /// `rows::spawn_row`) is painted over with nothing on the first paint.
    #[test]
    fn a_line_outside_the_selection_keeps_its_own_ground() {
        let mut rig = rig_with_grounds();
        rig.select(0);
        assert_eq!(rig.ground(0), Some(selected()));
        assert_eq!(rig.ground(1), Some(BASE), "an unselected line is untouched");
        assert_eq!(rig.ground(2), Some(BASE), "an unselected line is untouched");
    }

    #[test]
    fn a_shut_panel_drops_the_selection_and_the_grounds() {
        let mut rig = rig();
        rig.select(0);
        rig.drag_to(2);
        rig.app.world_mut().resource_mut::<ChatUi>().open = false;
        rig.app.update();
        for order in 0..3 {
            assert!(bare(rig.ground(order)), "line {order}");
        }
        assert_eq!(rig.copy(), None, "nothing is selected any more");
    }

    #[test]
    fn a_second_press_on_the_one_selected_line_lets_it_go() {
        let mut rig = rig();
        rig.select(1);
        assert_eq!(rig.ground(1), Some(selected()));
        rig.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        rig.app.update();
        rig.select(1);
        assert!(bare(rig.ground(1)));
        assert_eq!(rig.copy(), None);
    }

    #[test]
    fn nothing_is_copied_while_the_box_has_the_caret() {
        let mut rig = rig();
        rig.select(0);
        rig.drag_to(1);
        rig.app.world_mut().resource_mut::<ChatUi>().typing = true;
        assert_eq!(rig.copy(), None, "the box copies its own selection then");
    }

    #[test]
    fn nothing_is_copied_without_a_selection_a_modifier_or_the_c() {
        let mut rig = rig();
        assert_eq!(rig.copy(), None, "no selection");

        let mut rig = self::rig();
        rig.select(0);
        {
            let mut keys = rig.app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyC);
        }
        rig.app.update();
        assert!(
            rig.app.world().resource::<SocialHost>().copy.is_none(),
            "C alone is no copy"
        );

        let mut rig = self::rig();
        rig.select(0);
        {
            let mut keys = rig.app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ControlLeft);
        }
        rig.app.update();
        assert!(
            rig.app.world().resource::<SocialHost>().copy.is_none(),
            "Ctrl alone is no copy"
        );
    }

    #[test]
    fn the_platform_s_command_key_copies_as_ctrl_does() {
        let mut rig = rig();
        rig.select(0);
        {
            let mut keys = rig.app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::SuperLeft);
            keys.press(KeyCode::KeyC);
        }
        rig.app.update();
        assert_eq!(
            rig.app
                .world_mut()
                .resource_mut::<SocialHost>()
                .copy
                .take()
                .as_deref(),
            Some("text0")
        );
    }

    #[test]
    fn only_a_primary_press_selects() {
        let mut rig = rig();
        rig.press(0, PointerButton::Secondary);
        rig.press(1, PointerButton::Middle);
        rig.app.update();
        assert_eq!(rig.copy(), None);
        assert!(bare(rig.ground(0)));
    }

    #[test]
    fn a_drag_does_not_run_into_another_list() {
        let mut rig = rig();
        // A second list whose rows share the first's orders.
        let other = rig.app.world_mut().spawn(Node::default()).id();
        let stranger = rig
            .app
            .world_mut()
            .spawn((
                Node::default(),
                Copyable::new(1, "other1".into()),
                ChildOf(other),
            ))
            .id();
        assert_ne!(other, rig.list);
        rig.app.update();
        rig.select(0);
        // The pointer crosses a row of the other list: the span stays here.
        let event = crate::test_pointer::Over {
            hit: HitData::new(stranger, 0.0, None, None),
        };
        rig.app.world_mut().write_message(crate::test_pointer::aim(
            PointerId::Mouse,
            Rig::location(),
            event,
            stranger,
        ));
        rig.app.update();
        assert_eq!(rig.copy().as_deref(), Some("text0"));
        // And a press in the other list moves the selection over to it.
        let event = crate::test_pointer::Press {
            button: PointerButton::Primary,
            hit: HitData::new(stranger, 0.0, None, None),
            count: 1,
        };
        rig.app.world_mut().write_message(crate::test_pointer::aim(
            PointerId::Mouse,
            Rig::location(),
            event,
            stranger,
        ));
        rig.app.update();
        assert_eq!(rig.copy().as_deref(), Some("other1"));
        assert!(bare(rig.ground(0)), "the first list is let go");
    }

    #[test]
    fn a_press_on_a_child_of_a_line_selects_the_line() {
        let mut rig = rig();
        let line = rig.row(1);
        let word = rig
            .app
            .world_mut()
            .spawn((Node::default(), ChildOf(line)))
            .id();
        rig.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        let event = crate::test_pointer::Press {
            button: PointerButton::Primary,
            hit: HitData::new(word, 0.0, None, None),
            count: 1,
        };
        rig.app.world_mut().write_message(crate::test_pointer::aim(
            PointerId::Mouse,
            Rig::location(),
            event,
            word,
        ));
        rig.app.update();
        assert_eq!(rig.copy().as_deref(), Some("text1"));
    }

    #[test]
    fn the_selection_resource_starts_empty() {
        let rig = rig();
        assert!(rig.app.world().contains_resource::<Selected>());
        let mut rig = rig;
        assert_eq!(rig.copy(), None);
    }
}

// ── the lobby chat window, played through the real systems ──────────────

/// The lobby window as the app runs it: Alice's machine in the host, Bob a
/// second machine, a stand-in for the gateway carrying their sealed lines
/// between them, and `window::install`'s systems on a clock the test moves.
///
/// Nothing here is built by hand: a line Alice has not read is a line that
/// came through the seal and the key fetch, because that is the only door
/// the machine has for `unread`.
mod pane {
    use std::collections::{BTreeMap, VecDeque};
    use std::time::Duration;

    use baylee_client_core::bugreport::refs::{CardRef, RefPrint};
    use baylee_client_core::lobby::LobbyEvent;
    use baylee_client_core::social::{ComposerKey, Effect, Method, SocialLink};
    use baylee_core::ids::CardIndex;
    use baylee_protocol::social::{
        AccountKeys, Delivered, DeviceKey, DeviceList, DeviceRow, Down, Friend, Header, KeysAnswer,
        Presence, Sender, Shown, Snapshot, Up,
    };
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::hud::UiFonts;
    use crate::hud::motion;
    use crate::lobby::{HoverCard, LobbyState};
    use crate::social::drawer::DrawerSlide;
    use crate::social::rows::{ChatCardLink, ChatRow};
    use crate::social::window::{
        ARRIVE_SECS, Arrive, ChatPress, LobbyChat, Part, Pop, UnreadDivider,
    };
    use crate::social::zoom::Zoom;
    use crate::social::{Field, SocialHost, window};

    pub(super) const ALICE: &str = "af03";
    pub(super) const BOB: &str = "0012";
    /// The machines' clock, fixed: a sealed line is refused for age.
    const NOW: u64 = 1_800_000_000_000;
    /// One frame.
    const FRAME: f32 = 1.0 / 60.0;

    /// The bytes behind base64url text (the header rides in a `Sealed`).
    fn unbase64(text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let (mut acc, mut bits) = (0u32, 0u32);
        for c in text.bytes() {
            let v = match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'-' => 62,
                b'_' => 63,
                _ => continue,
            };
            acc = (acc << 6) | u32::from(v);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push(u8::try_from((acc >> bits) & 0xff).expect("a byte"));
            }
        }
        out
    }

    /// What the gateway keeps: device keys, handles, and a slot per device.
    #[derive(Default)]
    struct Gateway {
        handles: BTreeMap<String, String>,
        keys: Vec<(String, DeviceKey)>,
        slots: Vec<(String, Down)>,
        seq: u64,
    }

    impl Gateway {
        fn http(
            &mut self,
            tag: &str,
            device: &str,
            method: Method,
            path: &str,
            body: Option<&serde_json::Value>,
        ) -> (u16, Vec<u8>) {
            if method == Method::Post && path == "/social/devices" {
                let body = body.expect("a body");
                let field = |k: &str| body[k].as_str().unwrap_or_default().to_string();
                self.keys.retain(|(_, k)| k.id != device);
                self.keys.push((
                    tag.to_string(),
                    DeviceKey {
                        id: device.to_string(),
                        ik: field("ik"),
                        dk: field("dk"),
                        dsig: field("dsig"),
                        label: field("label"),
                        added_at: 0,
                    },
                ));
                return (
                    201,
                    serde_json::to_vec(&serde_json::json!({ "id": device })).expect("json"),
                );
            }
            if method == Method::Get && path == "/social/devices" {
                let devices = self
                    .keys
                    .iter()
                    .filter(|(t, _)| t == tag)
                    .map(|(_, k)| DeviceRow {
                        id: k.id.clone(),
                        label: k.label.clone(),
                        added_at: 0,
                        seen_at: 0,
                    })
                    .collect();
                return (
                    200,
                    serde_json::to_vec(&DeviceList { devices }).expect("json"),
                );
            }
            if let Some(query) = path.strip_prefix("/social/keys?tag=") {
                let keys = query
                    .split(',')
                    .filter(|t| self.handles.contains_key(*t))
                    .map(|t| AccountKeys {
                        tag: t.to_string(),
                        handle: self.handles[t].clone(),
                        devices: self
                            .keys
                            .iter()
                            .filter(|(owner, _)| owner == t)
                            .map(|(_, k)| k.clone())
                            .collect(),
                    })
                    .collect();
                return (200, serde_json::to_vec(&KeysAnswer { keys }).expect("json"));
            }
            (204, Vec::new())
        }

        /// The other account is the only friend there is.
        fn snapshot(&self, tag: &str) -> Snapshot {
            Snapshot {
                friends: self
                    .handles
                    .iter()
                    .filter(|(t, _)| t.as_str() != tag)
                    .map(|(t, handle)| Friend {
                        person: Person {
                            tag: t.clone(),
                            handle: handle.clone(),
                        },
                        since: 0,
                        presence: Presence {
                            status: Shown::Online,
                            place: None,
                        },
                    })
                    .collect(),
                ..Snapshot::default()
            }
        }

        fn up(&mut self, tag: &str, device: &str, frame: Up) -> Vec<Down> {
            match frame {
                Up::Hello { device, .. } => vec![
                    Down::Welcome {
                        tag: tag.to_string(),
                        handle: self.handles[tag].clone(),
                        device,
                        guest: false,
                    },
                    Down::Snapshot(self.snapshot(tag)),
                ],
                Up::Send { req, conv, sealed } => {
                    let header: Header =
                        serde_json::from_slice(&unbase64(&sealed.hdr)).expect("a header");
                    let at = NOW + self.seq;
                    for wrap in &sealed.wraps {
                        self.seq += 1;
                        self.slots.push((
                            wrap.d.clone(),
                            Down::Msg {
                                seq: self.seq,
                                at,
                                conv: conv.clone(),
                                from: Sender {
                                    tag: tag.to_string(),
                                    handle: self.handles[tag].clone(),
                                    device: device.to_string(),
                                },
                                sealed: Delivered {
                                    hdr: sealed.hdr.clone(),
                                    n: sealed.n.clone(),
                                    ct: sealed.ct.clone(),
                                    epk: sealed.epk.clone(),
                                    sig: sealed.sig.clone(),
                                    k: wrap.k.clone(),
                                },
                            },
                        ));
                    }
                    vec![Down::Sent {
                        req,
                        id: header.id,
                        at,
                    }]
                }
                _ => Vec::new(),
            }
        }

        /// Carries `effects` out for `social` until none is for the gateway.
        fn run(&mut self, social: &mut Social, tag: &str, effects: Vec<Effect>) {
            let device = social.device_id().to_string();
            let mut queue: VecDeque<Effect> = effects.into();
            let mut guard = 0;
            while let Some(effect) = queue.pop_front() {
                guard += 1;
                assert!(guard < 2_000, "effects never settle");
                match effect {
                    Effect::Http {
                        tag: what,
                        method,
                        path,
                        body,
                    } => {
                        let (status, answer) =
                            self.http(tag, &device, method, &path, body.as_ref());
                        queue.extend(social.on_http(what, status, &answer, NOW, &mut OsEntropy));
                    }
                    Effect::Dial => queue.extend(social.on_link(SocialLink::Up, NOW)),
                    Effect::Up(frame) => {
                        for down in self.up(tag, &device, frame) {
                            queue.extend(social.on_frame(down, NOW, &mut OsEntropy));
                        }
                    }
                    _ => {}
                }
            }
        }

        /// Hands `social` everything waiting for its device.
        fn deliver(&mut self, social: &mut Social, tag: &str) {
            let device = social.device_id().to_string();
            let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.slots)
                .into_iter()
                .partition(|(d, _)| *d == device);
            self.slots = rest;
            for (_, frame) in mine {
                let effects = social.on_frame(frame, NOW, &mut OsEntropy);
                self.run(social, tag, effects);
            }
        }
    }

    /// A machine for `tag`, registered and welcomed.
    fn account(gateway: &mut Gateway, tag: &str, handle: &str) -> Social {
        gateway.handles.insert(tag.into(), handle.into());
        let me = Me {
            tag: tag.into(),
            handle: handle.into(),
            guest: false,
            gateway: "http://127.0.0.1:1".into(),
        };
        let (mut social, effects) =
            Social::start(me, None, None, ChatPrefs::default(), NOW, &mut OsEntropy);
        gateway.run(&mut social, tag, effects);
        social
    }

    /// A card the composer knows by name.
    fn bolt() -> CardRef {
        CardRef {
            name: "Lightning Bolt".into(),
            english: "Lightning Bolt".into(),
            card: CardIndex::new(7),
            face: 0,
            art: None,
            print: Some(RefPrint {
                scryfall_id: "print-7".into(),
                lang: "en".into(),
                finish: "nonfoil".into(),
            }),
            object: None,
            zone: None,
            owner: None,
            kind: None,
        }
    }

    pub(super) struct Rig {
        pub(super) app: App,
        gateway: Gateway,
        bob: Social,
    }

    impl Rig {
        /// The lobby signed in, the window shut to its pill, the clock at
        /// sixty frames a second. `reduce_motion` as the player set it.
        pub(super) fn new(reduce_motion: bool) -> Self {
            let mut gateway = Gateway::default();
            let alice = account(&mut gateway, ALICE, "Alice#af03");
            let bob = account(&mut gateway, BOB, "Bob#0012");
            let mut prefs = crate::prefs::Prefs::default();
            prefs.edit().reduce_motion = reduce_motion;
            let mut lobby = LobbyState::new();
            lobby.lobby.apply(LobbyEvent::LoggedIn {
                token: "session".into(),
                username: Some("alice".into()),
            });
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, StatesPlugin))
                .init_state::<crate::DuelPhase>()
                .add_message::<bevy::picking::events::PointerOver>()
                .add_message::<bevy::picking::events::PointerOut>()
                .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                    FRAME,
                )))
                .insert_resource(UiFonts::default())
                .insert_resource(Zoom::default())
                .insert_resource(DrawerSlide::default())
                .insert_resource(SocialHost {
                    machine: Some(alice),
                    ..SocialHost::default()
                })
                .insert_resource(ChatUi::default())
                .insert_resource(lobby)
                .insert_resource(prefs);
            window::install(&mut app);
            app.world_mut().spawn(Window {
                focused: true,
                ..default()
            });
            let mut rig = Self { app, gateway, bob };
            // The first frame's time is nothing; the pill's own opening is
            // over before a test looks at one.
            rig.frames(30);
            rig
        }

        pub(super) fn frames(&mut self, n: usize) {
            for _ in 0..n {
                self.app.update();
            }
        }

        pub(super) fn ui(&mut self) -> Mut<'_, ChatUi> {
            self.app.world_mut().resource_mut::<ChatUi>()
        }

        /// The window open on `tab`, as a press on the pill would leave it.
        pub(super) fn open(&mut self, tab: ChatTab) {
            let mut ui = self.ui();
            ui.open = true;
            ui.minimised = false;
            ui.tab = tab;
        }

        /// The window shut as its cross does: the machine is told nothing
        /// is read any more.
        pub(super) fn close(&mut self) {
            self.app
                .world_mut()
                .resource_scope(|world, mut host: Mut<SocialHost>| {
                    host.act(baylee_client_core::social::Intent::Panel(None));
                    world.resource_mut::<ChatUi>().open = false;
                });
        }

        /// The window opened as a press on the pill does: `open_on_chat`,
        /// which has the machine read the tab's conversation as it goes.
        pub(super) fn open_by_door(&mut self) {
            self.app
                .world_mut()
                .resource_scope(|world, mut host: Mut<SocialHost>| {
                    let mut ui = world.resource_mut::<ChatUi>();
                    window::open_on_chat(&mut host, &mut ui, None);
                });
        }

        /// Bob whispers Alice `text`; it reaches her machine sealed.
        pub(super) fn bob_says(&mut self, text: &str) {
            self.bob.composer_mut().typed(&format!("/w Alice {text}"));
            let (_, effects) = self.bob.key(ComposerKey::Enter, NOW, &mut OsEntropy);
            self.gateway.run(&mut self.bob, BOB, effects);
            let mut host = self.app.world_mut().resource_mut::<SocialHost>();
            let alice = host.machine.as_mut().expect("Alice's machine");
            self.gateway.deliver(alice, ALICE);
        }

        pub(super) fn machine(&self) -> &Social {
            self.app
                .world()
                .resource::<SocialHost>()
                .machine()
                .expect("Alice's machine")
        }

        /// The ids of the whisper lines Alice holds, oldest first.
        pub(super) fn line_ids(&self) -> Vec<String> {
            self.machine()
                .conversations()
                .iter()
                .flat_map(|c| c.lines.iter())
                .filter(|l| l.body().is_some())
                .map(|l| l.key.id.clone())
                .collect()
        }

        pub(super) fn composer_says(&mut self, text: &str) {
            let host = &mut *self.app.world_mut().resource_mut::<SocialHost>();
            let machine = host.machine.as_mut().expect("Alice's machine");
            machine.set_card_candidates(vec![bolt()]);
            machine.composer_mut().typed(text);
            let mut ui = self.ui();
            ui.typing = true;
            ui.field = Field::Composer;
        }

        pub(super) fn root(&mut self) -> Entity {
            self.app
                .world_mut()
                .query_filtered::<Entity, With<LobbyChat>>()
                .single(self.app.world())
                .expect("the lobby window's root")
        }

        pub(super) fn pop(&mut self) -> Pop {
            let root = self.root();
            *self.app.world().get::<Pop>(root).expect("a Pop")
        }

        pub(super) fn scale(&mut self) -> Vec2 {
            let root = self.root();
            self.app
                .world()
                .get::<UiTransform>(root)
                .expect("a transform")
                .scale
        }

        /// The first entity wearing the part `part`.
        pub(super) fn part(&mut self, part: Part) -> Entity {
            self.app
                .world_mut()
                .query::<(Entity, &Part)>()
                .iter(self.app.world())
                .find_map(|(e, p)| (*p == part).then_some(e))
                .unwrap_or_else(|| panic!("no {part:?} is drawn"))
        }

        pub(super) fn has_part(&mut self, part: Part) -> bool {
            self.app
                .world_mut()
                .query::<&Part>()
                .iter(self.app.world())
                .any(|p| *p == part)
        }

        /// Whether the pill stands in the window.
        pub(super) fn has_pill(&mut self) -> bool {
            self.app
                .world_mut()
                .query::<&ChatPress>()
                .iter(self.app.world())
                .any(|p| *p == ChatPress::Restore)
        }

        pub(super) fn children(&self, entity: Entity) -> Vec<Entity> {
            self.app
                .world()
                .get::<Children>(entity)
                .map(|kids| kids.iter().collect())
                .unwrap_or_default()
        }

        /// Everything under `entity`.
        pub(super) fn below(&self, entity: Entity) -> Vec<Entity> {
            let mut out = Vec::new();
            let mut todo = self.children(entity);
            while let Some(next) = todo.pop() {
                out.push(next);
                todo.extend(self.children(next));
            }
            out
        }

        /// The list's children, in order.
        pub(super) fn list(&mut self) -> Vec<Entity> {
            let list = self.part(Part::List);
            self.children(list)
        }

        /// The chat rows of the list, with the id of the line each draws.
        pub(super) fn rows(&mut self) -> Vec<(Entity, String)> {
            self.list()
                .into_iter()
                .filter_map(|e| {
                    self.app
                        .world()
                        .get::<ChatRow>(e)
                        .map(|row| (e, row.key.id.clone()))
                })
                .collect()
        }

        pub(super) fn dividers(&mut self) -> Vec<Entity> {
            self.app
                .world_mut()
                .query_filtered::<Entity, With<UnreadDivider>>()
                .iter(self.app.world())
                .collect()
        }
    }

    /// With motion on, the root's [`Pop`] after a frame is where the curve
    /// puts it: not still at the start, not yet done, and the scale the one
    /// [`motion::opening`] names. Guards against a pop that never advances
    /// (a clock nobody moved) as much as one that jumps.
    #[test]
    fn the_window_pops_open_along_the_hud_curve_one_frame_at_a_time() {
        let mut rig = Rig::new(false);
        assert_eq!(rig.pop().t.to_bits(), 1.0f32.to_bits(), "the warm-up ended");
        assert_eq!(rig.scale(), Vec2::ONE);
        rig.open(ChatTab::Private);
        // The frame that sees the press restarts the pop, the next moves it.
        rig.frames(1);
        assert_eq!(rig.pop().t.to_bits(), 0.0f32.to_bits(), "restarted at 0");
        rig.frames(1);
        let pop = rig.pop();
        let want = FRAME / motion::ZOOM_IN;
        assert!(!pop.closing);
        assert!((pop.t - want).abs() < 1e-4, "t is {} not {want}", pop.t);
        let scale = rig.scale();
        assert!(
            scale.x > motion::ZOOM_FROM && scale.x < 1.0 && scale.x.to_bits() == scale.y.to_bits(),
            "one frame in the window is drawn at {scale}"
        );
        assert!((scale.x - motion::opening(pop.t)).abs() < 1e-4);
        // And it ends: at rest the scale is exactly one.
        rig.frames(20);
        assert_eq!(rig.pop().t.to_bits(), 1.0f32.to_bits());
        assert_eq!(rig.scale(), Vec2::ONE, "a finished pop draws at full size");
    }

    /// With `reduce_motion` the window is at its end on the frame after the
    /// press: the movement is over before it starts.
    #[test]
    fn with_reduced_motion_the_pop_is_over_in_one_update() {
        let mut rig = Rig::new(true);
        rig.open(ChatTab::Private);
        rig.frames(1);
        assert_eq!(rig.pop().t.to_bits(), 0.0f32.to_bits(), "restarted at 0");
        rig.frames(1);
        assert_eq!(rig.pop().t.to_bits(), 1.0f32.to_bits(), "one update");
        assert_eq!(rig.scale(), Vec2::ONE);
    }

    /// A closing window shrinks, keeps its open shape while it does, and
    /// only when the pop has finished closing does the pill take its place
    /// (and pop in turn). Each frame is checked: a pill that came early, a
    /// pop that stopped closing or a scale that grew would each fail.
    #[test]
    fn a_closed_window_shrinks_in_its_open_shape_and_then_becomes_the_pill() {
        let mut rig = Rig::new(false);
        rig.open(ChatTab::Private);
        rig.frames(30);
        assert!(
            rig.has_part(Part::Head) && !rig.has_pill(),
            "it stands open"
        );
        rig.close();
        rig.frames(1);
        assert!(rig.pop().closing, "the press started the close");
        let mut last = 1.0f32;
        let mut shrinking = 0;
        for _ in 0..40 {
            rig.frames(1);
            let pop = rig.pop();
            if !pop.closing {
                break;
            }
            assert!(
                rig.has_part(Part::Head) && !rig.has_pill(),
                "the panel kept its open shape at t = {}",
                pop.t
            );
            assert!(pop.t < 1.0, "a finished close still shows the panel");
            let scale = rig.scale().x;
            assert!(
                scale < 1.0,
                "a closing window is smaller than full: {scale}"
            );
            assert!(scale >= motion::ZOOM_FROM, "and not smaller than the floor");
            assert!(scale < last, "it grew while closing: {last} then {scale}");
            last = scale;
            shrinking += 1;
        }
        assert!(
            shrinking >= 3,
            "the close took {shrinking} frames: a shrink that is not a movement"
        );
        assert!(rig.has_pill(), "the pill took its place");
        assert!(!rig.has_part(Part::Head), "and the panel is gone");
        let pop = rig.pop();
        assert!(!pop.closing, "the pill pops open");
        assert!(pop.t < 1.0, "from the start, not already done: {}", pop.t);
    }

    /// With `reduce_motion` the pill is there within two updates of the
    /// press, and the panel is not drawn a frame longer than that.
    #[test]
    fn with_reduced_motion_the_pill_replaces_the_window_at_once() {
        let mut rig = Rig::new(true);
        rig.open(ChatTab::Private);
        rig.frames(5);
        assert!(rig.has_part(Part::Head) && !rig.has_pill());
        rig.close();
        rig.frames(2);
        assert!(rig.has_pill(), "the pill is up after two updates");
        assert!(!rig.has_part(Part::Head), "and the panel is down");
    }

    /// A line that comes through the seal while the window is open on its
    /// tab is appended to the list and rises into place; the rows drawn
    /// before it are the same entities, and none of them ever arrived.
    #[test]
    fn a_line_more_is_appended_and_rises_while_the_old_rows_stay() {
        let mut rig = Rig::new(false);
        rig.bob_says("one");
        rig.bob_says("two");
        rig.open(ChatTab::Private);
        rig.frames(30);
        let before = rig.rows();
        assert_eq!(before.len(), 2, "the two lines are drawn");
        let first_draw = rig
            .app
            .world_mut()
            .query::<&Arrive>()
            .iter(rig.app.world())
            .count();
        assert_eq!(first_draw, 0, "the rows drawn on first open do not arrive");

        rig.bob_says("three");
        rig.frames(1);
        let after = rig.rows();
        assert_eq!(after.len(), 3);
        assert_eq!(
            after[..2],
            before[..],
            "the earlier rows are the same entities"
        );
        let new = after[2].0;
        assert!(
            rig.app.world().get::<Arrive>(new).is_some(),
            "the new row arrives"
        );
        for (old, _) in &before {
            assert!(
                rig.app.world().get::<Arrive>(*old).is_none(),
                "an old row rose"
            );
        }

        // It rises: under way it sits below its place; the frames pass; it lets go.
        rig.frames(1);
        let arrive = rig
            .app
            .world()
            .get::<Arrive>(new)
            .expect("still arriving")
            .0;
        assert!(arrive > 0.0 && arrive < 1.0, "arrive is {arrive}");
        let Val::Px(down) = rig
            .app
            .world()
            .get::<UiTransform>(new)
            .expect("t")
            .translation
            .y
        else {
            panic!("the rise is in pixels");
        };
        assert!(down > 0.0, "a row on its way is below its place: {down}");
        let frames = (ARRIVE_SECS / FRAME).ceil() as usize + 2;
        rig.frames(frames);
        assert!(
            rig.app.world().get::<Arrive>(new).is_none(),
            "the arrival never ended"
        );
        assert_eq!(
            rig.app
                .world()
                .get::<UiTransform>(new)
                .expect("t")
                .translation,
            Val2::ZERO,
            "it is not at its place"
        );
        assert_eq!(rig.rows()[..2], before[..], "and the old rows still stand");
    }

    /// With `reduce_motion` the arriving row is at its place on the next
    /// update and the component is gone.
    #[test]
    fn with_reduced_motion_a_row_arrives_in_one_update() {
        let mut rig = Rig::new(true);
        rig.bob_says("one");
        rig.open(ChatTab::Private);
        rig.frames(10);
        rig.bob_says("two");
        rig.frames(1);
        let new = rig.rows()[1].0;
        assert!(
            rig.app.world().get::<Arrive>(new).is_some(),
            "it was marked"
        );
        rig.frames(1);
        assert!(rig.app.world().get::<Arrive>(new).is_none(), "and let go");
        assert_eq!(
            rig.app
                .world()
                .get::<UiTransform>(new)
                .expect("t")
                .translation,
            Val2::ZERO
        );
    }

    /// Showing another tab draws the list anew, and a row that is on it for
    /// the first time does not arrive.
    #[test]
    fn a_tab_switch_draws_the_list_anew_without_arrivals() {
        let mut rig = Rig::new(false);
        rig.bob_says("one");
        rig.bob_says("two");
        rig.open(ChatTab::Private);
        rig.frames(30);
        let before = rig.rows();
        rig.ui().tab = ChatTab::All;
        rig.frames(3);
        rig.ui().tab = ChatTab::Private;
        rig.frames(3);
        let after = rig.rows();
        assert_eq!(after.len(), 2);
        for ((old, old_id), (new, new_id)) in before.iter().zip(&after) {
            assert_eq!(old_id, new_id);
            assert_ne!(old, new, "the row was not drawn again");
        }
        let arriving = rig
            .app
            .world_mut()
            .query::<&Arrive>()
            .iter(rig.app.world())
            .count();
        assert_eq!(arriving, 0, "a tab opened shows its lines standing still");
    }

    /// Lines unread when a tab is opened get a "New" divider right above the
    /// first of them; a line that arrives while the tab is read does not move
    /// or add one; another tab has none.
    #[test]
    fn a_tab_opened_on_unread_lines_draws_one_new_divider_above_the_first() {
        let mut rig = Rig::new(true);
        rig.bob_says("old");
        rig.open(ChatTab::Private);
        rig.frames(10);
        // Unread, but first on the list: nothing above it to divide from.
        assert_eq!(rig.dividers().len(), 0);
        rig.close();
        rig.frames(5);
        assert!(rig.ui().marker.is_none(), "a shut window keeps no marker");

        rig.bob_says("new1");
        rig.bob_says("new2");
        let ids = rig.line_ids();
        assert_eq!(ids.len(), 3, "{ids:?}");
        assert_eq!(rig.machine().unread_total(), 2, "two lines wait");
        rig.open(ChatTab::Private);
        rig.frames(10);
        let marker = rig.ui().marker.clone();
        let (tab, key) = marker.expect("the first unread line is marked");
        assert_eq!(tab, ChatTab::Private);
        assert_eq!(
            key.id, ids[1],
            "the marker is the first unread, not the last"
        );
        assert_eq!(rig.machine().unread_total(), 0, "reading marked them read");
        let dividers = rig.dividers();
        assert_eq!(dividers.len(), 1);
        let list = rig.list();
        let at = list
            .iter()
            .position(|e| *e == dividers[0])
            .expect("in the list");
        let rows = rig.rows();
        assert_eq!(
            list.iter()
                .filter(|e| rows.iter().any(|(r, _)| r == *e))
                .count(),
            3,
            "the list holds the three rows and the divider"
        );
        assert_eq!(list.len(), 4);
        assert_eq!(at, 1, "after the read line");
        assert_eq!(list[at + 1], rows[1].0, "right before the first unread row");
        assert_eq!(rows[1].1, ids[1]);

        // Another line, read as it comes: the divider stays where it was.
        let before = rig.list();
        rig.bob_says("new3");
        rig.frames(3);
        let after = rig.list();
        assert_eq!(after.len(), 5);
        assert_eq!(after[..4], before[..], "the rows and the divider stand");
        assert_eq!(rig.dividers(), dividers, "one divider, the same one");
        assert_eq!(
            rig.ui().marker.as_ref().map(|(_, k)| k.id.clone()),
            Some(ids[1].clone())
        );
        assert_eq!(rig.machine().unread_total(), 0, "the new line was read");

        // Another tab has nothing unread, so nothing to divide.
        rig.ui().tab = ChatTab::All;
        rig.frames(3);
        assert!(rig.ui().marker.is_none(), "the marker went with the tab");
        assert_eq!(rig.dividers().len(), 0);
    }

    /// Opened through the door (`open_on_chat`), which has the machine read
    /// the conversation at once, the tab still draws its divider above the
    /// first line that was unread: the marker is noted before the count is
    /// cleared and survives the tab coming into view.
    #[test]
    fn a_window_opened_by_its_door_keeps_the_divider_of_what_was_unread() {
        let mut rig = Rig::new(true);
        rig.bob_says("old");
        rig.open_by_door();
        rig.frames(5);
        rig.close();
        rig.frames(5);
        assert!(rig.ui().marker.is_none(), "a shut window keeps no marker");

        rig.bob_says("new1");
        rig.bob_says("new2");
        let ids = rig.line_ids();
        assert_eq!(rig.machine().unread_total(), 2);
        rig.open_by_door();
        assert_eq!(rig.machine().unread_total(), 0, "the door read the tab");
        let (tab, key) = rig.ui().marker.clone().expect("noted before it was read");
        assert_eq!((tab, key.id.as_str()), (ChatTab::Private, ids[1].as_str()));
        rig.frames(10);
        assert_eq!(
            rig.ui().marker.as_ref().map(|(_, k)| k.id.clone()),
            Some(ids[1].clone()),
            "the tab coming into view kept it"
        );
        let dividers = rig.dividers();
        assert_eq!(dividers.len(), 1);
        let list = rig.list();
        let rows = rig.rows();
        assert_eq!(list.len(), 4, "three rows and the divider");
        assert_eq!(list[1], dividers[0], "after the read line");
        assert_eq!(list[2], rows[1].0, "right before the first unread row");
    }

    /// Shutting the window with the header's chat button must leave the
    /// machine knowing nothing is being read: a line that comes after it is
    /// unread, and counts on the pill. As the cross does it
    /// (`Intent::Panel(None)`); `header_ask` only clears `ChatUi.open`, so the
    /// machine goes on reading the conversation under the pill.
    #[test]
    fn a_line_after_the_header_button_shut_the_window_is_unread() {
        let mut rig = Rig::new(true);
        rig.bob_says("old");
        rig.open_by_door();
        rig.frames(5);
        rig.app
            .world_mut()
            .resource_scope(|world, mut host: Mut<SocialHost>| {
                let mut ui = world.resource_mut::<ChatUi>();
                crate::social::header_ask(&mut host, &mut ui, crate::lobby::ShellAsk::Chat);
                assert!(!ui.open, "the button shut it");
            });
        rig.frames(5);
        rig.bob_says("later");
        assert_eq!(
            rig.machine().unread_total(),
            1,
            "a line under the pill is unread"
        );
    }

    /// A tab opened with nothing unread has no divider, even right after
    /// one that did.
    #[test]
    fn a_tab_with_nothing_unread_has_no_divider() {
        let mut rig = Rig::new(true);
        rig.bob_says("old");
        rig.open_by_door();
        rig.frames(5);
        rig.close();
        rig.frames(5);
        rig.bob_says("new1");
        rig.open_by_door();
        rig.frames(5);
        assert!(rig.ui().marker.is_some(), "the unread line is marked");
        rig.app
            .world_mut()
            .resource_scope(|world, mut host: Mut<SocialHost>| {
                let mut ui = world.resource_mut::<ChatUi>();
                window::select_tab(&mut host, &mut ui, ChatTab::Group);
            });
        rig.frames(3);
        assert!(rig.ui().marker.is_none(), "a tab with nothing unread");
        assert_eq!(rig.dividers().len(), 0);
    }

    /// A card link in the lobby's chat gets the lobby's hover data, which is
    /// what makes the lobby's preview open for it.
    #[test]
    fn a_card_link_in_the_lobby_hovers_as_a_pool_row_does() {
        let mut rig = Rig::new(true);
        let link = rig
            .app
            .world_mut()
            .spawn(ChatCardLink {
                card: 4242,
                face: 0,
                print: None,
            })
            .id();
        rig.frames(2);
        let hover = rig.app.world().get::<HoverCard>(link).expect("a HoverCard");
        assert_eq!(hover.index, Some(4242));
    }

    /// A `[Name]` in the composer that the line would send as a card link is
    /// drawn as a span carrying that link, previewable in the lobby; plain
    /// text has none.
    #[test]
    fn a_card_link_in_the_composer_is_a_span_that_carries_the_card() {
        let mut rig = Rig::new(true);
        rig.bob_says("hi");
        rig.open(ChatTab::Private);
        rig.composer_says("see [Lightning Bolt] now");
        let linked = rig
            .machine()
            .composer()
            .body_of("see [Lightning Bolt] now", false);
        assert_eq!(linked.links.len(), 1, "the composer knows the card");
        assert_eq!(linked.links[0].card, 7);
        rig.frames(5);
        let composer = rig.part(Part::Composer);
        let spans: Vec<Entity> = rig
            .below(composer)
            .into_iter()
            .filter(|e| rig.app.world().get::<ChatCardLink>(*e).is_some())
            .collect();
        assert_eq!(spans.len(), 1, "one link, one span");
        let world = rig.app.world();
        assert!(
            world.get::<TextSpan>(spans[0]).is_some(),
            "it is a text span"
        );
        assert_eq!(
            world.get::<TextSpan>(spans[0]).expect("text").0,
            "[Lightning Bolt]"
        );
        assert_eq!(world.get::<ChatCardLink>(spans[0]).expect("link").card, 7);
        assert_eq!(
            world.get::<HoverCard>(spans[0]).expect("hover data").index,
            Some(7),
            "and it previews like a pool row"
        );

        // Plain text, even naming the card, is no link.
        let host = &mut *rig.app.world_mut().resource_mut::<SocialHost>();
        let machine = host.machine.as_mut().expect("a machine");
        machine.composer_mut().clear();
        machine.composer_mut().typed("see Lightning Bolt now");
        rig.frames(3);
        let composer = rig.part(Part::Composer);
        let any = rig
            .below(composer)
            .into_iter()
            .any(|e| rig.app.world().get::<ChatCardLink>(e).is_some());
        assert!(!any, "unbracketed text is not a link");
    }

    /// The lock is the one line under the tabs, not a tool in the head; the
    /// line shows on a tab that has a conversation and not on one that has
    /// none.
    #[test]
    fn the_note_line_is_the_only_lock_press_and_shows_with_a_conversation() {
        let mut rig = Rig::new(true);
        rig.bob_says("hi");
        rig.open(ChatTab::Private);
        rig.frames(10);
        let locks: Vec<Entity> = rig
            .app
            .world_mut()
            .query::<(Entity, &ChatPress)>()
            .iter(rig.app.world())
            .filter_map(|(e, p)| (*p == ChatPress::Lock).then_some(e))
            .collect();
        assert_eq!(locks.len(), 1, "exactly one Lock press");
        let note = rig.part(Part::Note);
        assert_eq!(locks[0], note, "and it is the note line");
        let head = rig.part(Part::Head);
        assert!(
            !rig.below(head).contains(&locks[0]),
            "the head holds no lock"
        );
        let presses: Vec<ChatPress> = rig
            .below(head)
            .into_iter()
            .filter_map(|e| rig.app.world().get::<ChatPress>(e).cloned())
            .collect();
        assert!(
            presses.contains(&ChatPress::Close),
            "the head has its tools"
        );
        assert!(!presses.contains(&ChatPress::Lock));
        assert_eq!(
            rig.app.world().get::<Node>(note).expect("node").display,
            Display::Flex,
            "the line shows on a tab with a conversation"
        );
        rig.ui().tab = ChatTab::All;
        rig.frames(3);
        let note = rig.part(Part::Note);
        assert_eq!(
            rig.app.world().get::<Node>(note).expect("node").display,
            Display::None,
            "and not where there is none"
        );
    }
}

// ── the chat's place in the stacking order ──────────────────────────────

/// The docked chat stands between the lobby's page and the shell's popovers
/// (RC: the account menu opened under the friends list, and the chat window
/// hid "Open table"), and over the table's own bands at a table.
mod stacking {
    use super::*;
    use crate::DuelPhase;
    use crate::shellkit::tokens::z;
    use crate::social::window::{G_PANEL, Stack, lobby_z, restack, stacked};
    use bevy::state::app::StatesPlugin;
    use bevy::state::state::NextState;

    /// Every offset a chat surface is built with: the window (0), the
    /// friends door (1), the drawer (10), the group bar (20), the
    /// composer's suggestions (30), toasts (40), the filter dialog (45),
    /// sheets (50) and the row menu (60).
    const OFFSETS: [i32; 9] = [0, 1, 10, 20, 30, 40, 45, 50, 60];
    /// The window, door, drawer and menus: those that must stay under the
    /// shell's popovers.
    const UNDER_POPOVERS: [i32; 7] = [0, 1, 10, 20, 30, 45, 60];

    /// In the lobby the window, the door, the drawer and every menu stand over the
    /// page and under the shell's popovers, so the account menu and the
    /// modals above them are never covered by the chat. Guards the old
    /// `G_PANEL + n` (900+), which stood over everything the shell draws.
    #[test]
    fn in_the_lobby_the_window_the_drawer_and_every_menu_stand_between_page_and_popovers() {
        for offset in UNDER_POPOVERS {
            let at = lobby_z(offset);
            assert!(at > z::PAGE, "offset {offset} at {at} is not over the page");
            assert!(
                at < z::POPOVER,
                "offset {offset} at {at} covers the shell's popovers ({})",
                z::POPOVER
            );
        }
    }

    /// Among themselves the chat keeps its order: the window under the
    /// door and the drawer, all under every menu, the menus in the order they were
    /// offset (a menu opened from another stands over it).
    #[test]
    fn in_the_lobby_the_chat_keeps_its_own_order() {
        let order = [0, 1, 10, 20, 30, 45, 60].map(lobby_z);
        assert!(
            order.windows(2).all(|pair| pair[0] < pair[1]),
            "the window, drawer and menus lost their order: {order:?}"
        );
    }

    /// Its sheets stand with the shell's sheets (not under them) and its
    /// toasts with the shell's toasts, so a notice is read over a modal.
    #[test]
    fn in_the_lobby_sheets_and_toasts_stand_with_the_shells() {
        assert!(lobby_z(50) >= z::SHEET, "a chat sheet under the shell's");
        assert_eq!(lobby_z(40), z::TOAST, "a chat toast with the shell's");
        for offset in [50, 40] {
            assert!(
                lobby_z(offset) > lobby_z(60),
                "offset {offset} stands under a menu"
            );
        }
    }

    /// `stacked` is what a surface is built with: the table's index and the
    /// offset `restack` works from.
    #[test]
    fn a_surface_is_built_at_the_tables_index_and_remembers_its_offset() {
        for offset in OFFSETS {
            let (index, stack) = stacked(offset);
            assert_eq!(index, GlobalZIndex(G_PANEL + offset));
            assert_eq!(stack, Stack(offset));
        }
    }

    /// A bare app with `restack` and one surface of every offset.
    fn surfaces() -> (App, Vec<(i32, Entity)>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<DuelPhase>()
            .add_systems(PostUpdate, restack);
        let spawned = OFFSETS
            .iter()
            .map(|&offset| (offset, app.world_mut().spawn(stacked(offset)).id()))
            .collect();
        (app, spawned)
    }

    fn go(app: &mut App, phase: DuelPhase) {
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(phase);
        app.update();
    }

    fn at(app: &App, entity: Entity) -> i32 {
        app.world()
            .get::<GlobalZIndex>(entity)
            .expect("a global index")
            .0
    }

    /// The system puts every surface in its band for where the player is,
    /// and puts it back when the player moves: closed (the lobby) is
    /// `lobby_z`, anything else (opening, playing, finished) is
    /// `G_PANEL + offset`.
    #[test]
    fn restack_follows_the_player_between_the_lobby_and_a_table() {
        let (mut app, spawned) = surfaces();
        app.update();
        for (offset, entity) in &spawned {
            assert_eq!(
                at(&app, *entity),
                lobby_z(*offset),
                "lobby, offset {offset}"
            );
        }
        for phase in [DuelPhase::Opening, DuelPhase::Playing, DuelPhase::Finished] {
            go(&mut app, phase);
            for (offset, entity) in &spawned {
                assert_eq!(
                    at(&app, *entity),
                    G_PANEL + offset,
                    "{phase:?}, offset {offset}"
                );
            }
        }
        go(&mut app, DuelPhase::Closed);
        for (offset, entity) in &spawned {
            assert_eq!(
                at(&app, *entity),
                lobby_z(*offset),
                "back in the lobby, offset {offset}"
            );
        }
    }

    /// A resting frame writes nothing: a changed `GlobalZIndex` restacks
    /// the whole UI, so `restack` writes only where the index differs.
    #[test]
    fn restack_writes_nothing_where_the_index_already_stands() {
        let (mut app, spawned) = surfaces();
        app.update();
        let marks = |app: &App| -> Vec<_> {
            spawned
                .iter()
                .map(|(_, e)| {
                    app.world()
                        .entity(*e)
                        .get_ref::<GlobalZIndex>()
                        .expect("a global index")
                        .last_changed()
                })
                .collect()
        };
        let before = marks(&app);
        app.update();
        app.update();
        assert_eq!(marks(&app), before, "a resting frame wrote an index");
    }

    /// The drawer, built by its own code, is restacked as well: it carries
    /// its offset and stands in the lobby's band, and at a table in the
    /// table's.
    #[test]
    fn the_real_drawer_is_restacked_with_the_player() {
        use crate::social::drawer::{self, Drawer};
        let mut rig = pane::Rig::new(true);
        drawer::install(&mut rig.app);
        rig.frames(3);
        let drawn = |rig: &mut pane::Rig| -> (Stack, i32) {
            let world = rig.app.world_mut();
            let (stack, index) = world
                .query_filtered::<(&Stack, &GlobalZIndex), With<Drawer>>()
                .single(world)
                .expect("the drawer is drawn while a session stands");
            (*stack, index.0)
        };
        assert_eq!(drawn(&mut rig), (Stack(10), lobby_z(10)));
        rig.app
            .world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Playing);
        rig.frames(3);
        assert_eq!(drawn(&mut rig), (Stack(10), G_PANEL + 10));
    }

    /// Every surface of the chat takes its index from `stacked`: a bare
    /// `GlobalZIndex(G_PANEL + n)` anywhere would stand over the shell's
    /// menus in the lobby, which `restack` could not move. The count has a
    /// floor (the nine surfaces) and a ceiling, so a misread marker cannot
    /// pass.
    #[test]
    fn no_chat_surface_takes_its_index_without_stacked() {
        let files = [
            ("drawer", include_str!("drawer.rs")),
            ("menu", include_str!("menu.rs")),
            ("sheet", include_str!("sheet.rs")),
            ("toasts", include_str!("toasts.rs")),
            (
                "window",
                concat!(
                    include_str!("window.rs"),
                    include_str!("window/head.rs"),
                    include_str!("window/list.rs"),
                    include_str!("window/composer.rs"),
                    include_str!("window/peek.rs"),
                    include_str!("window/filter.rs"),
                ),
            ),
        ];
        let mut calls = 0;
        for (name, source) in files {
            let bare = source.matches("GlobalZIndex(G_PANEL").count();
            // `stacked` itself builds the one.
            let allowed = usize::from(name == "window");
            assert_eq!(bare, allowed, "{name}.rs builds a GlobalZIndex by hand");
            calls += source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .map(|line| {
                    line.matches("stacked(")
                        .count()
                        .saturating_sub(usize::from(line.contains("fn stacked(")))
                })
                .sum::<usize>();
        }
        assert!(
            (9..=14).contains(&calls),
            "{calls} surfaces are stacked; nine were built when this was written"
        );
    }

    /// A surface built by the real window code carries its offset, so the
    /// lobby window is stacked in its band after a frame and moves to the
    /// table's when a duel opens.
    #[test]
    fn the_real_lobby_window_is_restacked_with_the_player() {
        let mut rig = pane::Rig::new(true);
        rig.open(ChatTab::Private);
        rig.frames(2);
        let root = rig.root();
        let index = |rig: &pane::Rig| rig.app.world().get::<GlobalZIndex>(root).expect("z").0;
        assert_eq!(
            rig.app.world().get::<Stack>(root),
            Some(&Stack(0)),
            "the window carries its offset"
        );
        assert_eq!(index(&rig), lobby_z(0));
        assert!(index(&rig) < z::POPOVER);
        rig.app
            .world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Playing);
        rig.frames(2);
        assert_eq!(index(&rig), G_PANEL, "at a table it is the table's band");
    }
}

// ── the shell's notice lane clears the chat ─────────────────────────────

/// The shell's notices ("Bob sat down at your table") were clipped by the
/// friends list and hidden behind the chat's pill (RC): `clear_the_lane`
/// keeps the lane left of the drawer and above the pill or window while a
/// session stands in the lobby, and leaves it where the shell put it
/// otherwise.
mod lane {
    use super::pane::Rig;
    use super::*;
    use crate::DuelPhase;
    use crate::lobby::ToastLane;
    use crate::social::drawer::DrawerSlide;
    use crate::social::zoom::Zoom;
    use crate::social::{SocialHost, drawer};
    use bevy::state::state::NextState;

    /// Where the shell put the lane.
    const RIGHT: f32 = 24.0;
    const BOTTOM: f32 = 24.0;

    /// The lobby with a chat session, the drawer's systems installed, and a
    /// lane where the shell puts it.
    fn rig() -> (Rig, Entity) {
        let mut rig = Rig::new(true);
        drawer::install(&mut rig.app);
        let lane = lane_at(&mut rig, RIGHT, BOTTOM);
        rig.frames(3);
        (rig, lane)
    }

    fn lane_at(rig: &mut Rig, right: f32, bottom: f32) -> Entity {
        rig.app
            .world_mut()
            .spawn((
                ToastLane,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(right),
                    bottom: Val::Px(bottom),
                    ..default()
                },
            ))
            .id()
    }

    fn where_is(rig: &Rig, lane: Entity) -> (Val, Val) {
        let node = rig.app.world().get::<Node>(lane).expect("a node");
        (node.right, node.bottom)
    }

    fn window_height(rig: &mut Rig) -> f32 {
        rig.app
            .world_mut()
            .query::<&Window>()
            .iter(rig.app.world())
            .next()
            .expect("a window")
            .height()
    }

    fn pill(zoom: f32) -> f32 {
        64.0 * zoom
    }

    /// With a session and the chat shut to its pill, the lane keeps its
    /// right edge (the drawer is shut) and rises above the pill.
    #[test]
    fn with_a_session_the_lane_rises_above_the_pill() {
        let (rig, lane) = rig();
        assert_eq!(
            where_is(&rig, lane),
            (Val::Px(RIGHT), Val::Px(pill(1.0))),
            "above the pill, the right edge unchanged"
        );
    }

    /// A lane already higher than the chat needs is left there: the lane
    /// rises to the chat, never down to it.
    #[test]
    fn a_lane_already_above_the_pill_is_not_lowered() {
        let (mut rig, _) = rig();
        let high = lane_at(&mut rig, RIGHT, 500.0);
        rig.frames(3);
        assert_eq!(where_is(&rig, high).1, Val::Px(500.0));
    }

    /// The pill's clearance follows the chat's zoom.
    #[test]
    fn the_pill_clearance_follows_the_zoom() {
        let (mut rig, lane) = rig();
        rig.app.world_mut().insert_resource(Zoom(1.5));
        rig.frames(3);
        assert_eq!(where_is(&rig, lane).1, Val::Px(pill(1.5)));
    }

    /// An open window stands higher than the pill: the lane clears it by
    /// the lobby panel's height and a gap of sixteen. Shut to the pill
    /// again (minimised) it is the pill's clearance.
    #[test]
    fn with_the_window_open_the_lane_clears_the_window() {
        let (mut rig, lane) = rig();
        rig.open(ChatTab::Private);
        rig.frames(3);
        let panel = window::lobby_panel_height(window_height(&mut rig)) + 16.0;
        assert!(
            panel > pill(1.0),
            "the window must be the taller of the two"
        );
        assert_eq!(where_is(&rig, lane).1, Val::Px(panel));

        rig.ui().minimised = true;
        rig.frames(3);
        assert_eq!(
            where_is(&rig, lane).1,
            Val::Px(pill(1.0)),
            "a minimised window is the pill"
        );
    }

    /// The open drawer takes its room from the right edge: the lane moves
    /// left by exactly that, once, however many frames pass, and comes back
    /// when the drawer shuts.
    #[test]
    fn the_open_drawer_moves_the_lane_left_by_its_room_and_no_more() {
        let (mut rig, lane) = rig();
        rig.ui().drawer = true;
        rig.frames(5);
        let slide = *rig.app.world().resource::<DrawerSlide>();
        assert_eq!(slide, DrawerSlide(1.0), "the drawer is in");
        let room = slide.room(1.0);
        assert!(room > 0.0, "an open drawer takes room");
        assert_eq!(
            where_is(&rig, lane).0,
            Val::Px(RIGHT + room),
            "left of the drawer"
        );
        rig.frames(5);
        assert_eq!(where_is(&rig, lane).0, Val::Px(RIGHT + room), "once");

        rig.ui().drawer = false;
        rig.frames(5);
        assert_eq!(where_is(&rig, lane).0, Val::Px(RIGHT), "and back again");
    }

    /// A lane the lobby builds anew (its rebuild spawns a new one where the
    /// shell puts it) is cleared from the shell's place, not from the old
    /// lane's moved one.
    #[test]
    fn a_lane_built_anew_is_cleared_from_where_the_shell_put_it() {
        let (mut rig, lane) = rig();
        rig.ui().drawer = true;
        rig.frames(3);
        rig.app.world_mut().entity_mut(lane).despawn();
        let fresh = lane_at(&mut rig, RIGHT, BOTTOM);
        rig.frames(3);
        let room = rig.app.world().resource::<DrawerSlide>().room(1.0);
        assert_eq!(
            where_is(&rig, fresh),
            (Val::Px(RIGHT + room), Val::Px(pill(1.0)))
        );
    }

    /// Without a session the lane stands where the shell put it, and a lane
    /// the chat had moved goes back when the session ends.
    #[test]
    fn without_a_session_the_lane_stands_where_the_shell_put_it() {
        let (mut rig, lane) = rig();
        rig.ui().drawer = true;
        rig.frames(3);
        assert_ne!(where_is(&rig, lane), (Val::Px(RIGHT), Val::Px(BOTTOM)));

        rig.app.world_mut().resource_mut::<SocialHost>().machine = None;
        rig.frames(5);
        assert_eq!(
            where_is(&rig, lane),
            (Val::Px(RIGHT), Val::Px(BOTTOM)),
            "the session ended: back where it was spawned"
        );

        let other = lane_at(&mut rig, 40.0, 12.0);
        rig.frames(3);
        assert_eq!(where_is(&rig, other), (Val::Px(40.0), Val::Px(12.0)));
    }

    /// At a table the chat is over the table, not beside a lobby: the lane
    /// stands where the shell put it, drawer open or not.
    #[test]
    fn at_a_table_the_lane_stands_where_the_shell_put_it() {
        let (mut rig, lane) = rig();
        rig.ui().drawer = true;
        rig.frames(3);
        assert_ne!(where_is(&rig, lane), (Val::Px(RIGHT), Val::Px(BOTTOM)));
        rig.app
            .world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Playing);
        rig.frames(3);
        assert_eq!(where_is(&rig, lane), (Val::Px(RIGHT), Val::Px(BOTTOM)));
    }

    /// A phone's lane hangs from the top with no bottom edge to raise; it
    /// is left alone.
    #[test]
    fn a_lane_hanging_from_the_top_is_left_alone() {
        let (mut rig, _) = rig();
        let hanging = rig
            .app
            .world_mut()
            .spawn((
                ToastLane,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(8.0),
                    left: Val::Px(8.0),
                    right: Val::Px(8.0),
                    bottom: Val::Auto,
                    ..default()
                },
            ))
            .id();
        rig.ui().drawer = true;
        rig.frames(5);
        assert_eq!(where_is(&rig, hanging), (Val::Px(8.0), Val::Auto));
    }
}

/// The settings standing over the table have every key
/// (`lobby::table_settings`): a `/` typed into their search opens no
/// composer. Put away, the same `/` opens it.
#[test]
fn the_settings_over_the_table_keep_the_chat_s_doors_shut() {
    use bevy::ecs::message::Messages;
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput};

    for over in [false, true] {
        let mut lobby = crate::LobbyState::new();
        crate::lobby::table_settings::stand_over_table(&mut lobby);
        if !over {
            crate::lobby::table_settings::close(&mut lobby);
        }
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .insert_state(crate::DuelPhase::Playing)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .insert_resource(host_with_machine())
            .insert_resource(ChatUi::default())
            .insert_resource(crate::Duel::default())
            .insert_resource(lobby);
        crate::social::keys::install(&mut app);
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::Slash,
                logical_key: Key::Character("/".into()),
                state: ButtonState::Pressed,
                text: Some("/".into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.world_mut().run_schedule(Update);
        assert_eq!(
            app.world().resource::<ChatUi>().typing,
            !over,
            "settings over the table: {over}"
        );
    }
}
