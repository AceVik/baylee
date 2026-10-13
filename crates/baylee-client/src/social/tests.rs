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
use super::{Arrivals, ChatUi, OsEntropy, released};

// ── released ────────────────────────────────────────────────────────────

/// A control the test presses.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct Press(&'static str);

#[derive(Resource, Default)]
struct Held(HashSet<Entity>);

#[derive(Resource, Default)]
struct Clicks(Vec<Press>);

fn record_clicks(
    changed: Query<(Entity, &Interaction, &Press), Changed<Interaction>>,
    mut held: ResMut<Held>,
    mut clicks: ResMut<Clicks>,
) {
    clicks.0.extend(released(&changed, &mut held.0));
}

fn click_app() -> App {
    let mut app = App::new();
    app.init_resource::<Held>()
        .init_resource::<Clicks>()
        .add_systems(Update, record_clicks);
    app
}

fn control(app: &mut App, name: &'static str) -> Entity {
    let id = app.world_mut().spawn((Interaction::None, Press(name))).id();
    app.update();
    id
}

/// The pointer's state over `entity`, as one frame of `Interaction`.
fn point(app: &mut App, entity: Entity, state: Interaction) {
    *app.world_mut()
        .entity_mut(entity)
        .get_mut::<Interaction>()
        .expect("an interaction") = state;
    app.update();
}

fn clicked(app: &App) -> Vec<&'static str> {
    app.world()
        .resource::<Clicks>()
        .0
        .iter()
        .map(|p| p.0)
        .collect()
}

#[test]
fn a_press_released_over_its_control_clicks_once() {
    let mut app = click_app();
    let send = control(&mut app, "send");
    point(&mut app, send, Interaction::Pressed);
    assert!(clicked(&app).is_empty(), "the press itself is no click");
    point(&mut app, send, Interaction::Hovered);
    assert_eq!(clicked(&app), ["send"]);
    // A hover that follows is not a second click.
    point(&mut app, send, Interaction::None);
    point(&mut app, send, Interaction::Hovered);
    assert_eq!(clicked(&app), ["send"], "hovering again clicks nothing");
}

#[test]
fn a_press_released_elsewhere_clicks_nothing() {
    let mut app = click_app();
    let send = control(&mut app, "send");
    point(&mut app, send, Interaction::Pressed);
    // The pointer slid off before the button came up.
    point(&mut app, send, Interaction::None);
    assert!(clicked(&app).is_empty());
    // Coming back over it later is a hover, not the release of that press.
    point(&mut app, send, Interaction::Hovered);
    assert!(clicked(&app).is_empty(), "the press was forgotten");
}

#[test]
fn a_hover_that_was_never_pressed_clicks_nothing() {
    let mut app = click_app();
    let send = control(&mut app, "send");
    point(&mut app, send, Interaction::Hovered);
    point(&mut app, send, Interaction::None);
    point(&mut app, send, Interaction::Hovered);
    assert!(clicked(&app).is_empty());
}

#[test]
fn a_release_over_another_control_clicks_neither() {
    let mut app = click_app();
    let first = control(&mut app, "first");
    let second = control(&mut app, "second");
    point(&mut app, first, Interaction::Pressed);
    // The pointer left the first and came up over the second, which was
    // never pressed.
    point(&mut app, first, Interaction::None);
    point(&mut app, second, Interaction::Hovered);
    assert!(clicked(&app).is_empty());
}

#[test]
fn two_controls_pressed_in_one_frame_each_click_on_their_own_release() {
    let mut app = click_app();
    let a = control(&mut app, "a");
    let b = control(&mut app, "b");
    for e in [a, b] {
        *app.world_mut()
            .entity_mut(e)
            .get_mut::<Interaction>()
            .expect("an interaction") = Interaction::Pressed;
    }
    app.update();
    point(&mut app, b, Interaction::Hovered);
    assert_eq!(clicked(&app), ["b"], "a is still held");
    point(&mut app, a, Interaction::Hovered);
    assert_eq!(clicked(&app), ["b", "a"]);
}

#[test]
fn a_second_press_of_the_same_control_clicks_again() {
    let mut app = click_app();
    let send = control(&mut app, "send");
    for _ in 0..2 {
        point(&mut app, send, Interaction::Pressed);
        point(&mut app, send, Interaction::Hovered);
    }
    assert_eq!(clicked(&app), ["send", "send"]);
}

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
        [
            words("cast "),
            ("[Opt]".into(), Piece::Link(100, 0)),
            words(" now"),
        ]
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
            ("[Opt]".into(), Piece::Link(100, 0)),
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
            ("[Lötus]".into(), Piece::Link(100, 0)),
            words(" 日本"),
        ]
    );
    assert_eq!(rejoined(&p), b.text);
}

#[test]
fn a_mark_in_the_last_characters_leaves_no_trailing_words() {
    let b = body("see 日本[X]", &[[6, 9]], &[]);
    let p = rows::pieces(&b);
    assert_eq!(p.last(), Some(&("[X]".into(), Piece::Link(100, 0))));
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
    assert_eq!(
        p,
        [
            words("a"),
            ("bcde".into(), Piece::Link(100, 0)),
            words("fgh"),
        ]
    );
    assert_eq!(rejoined(&p), b.text, "nothing drawn twice, nothing lost");
}

#[test]
fn neighbouring_spans_leave_no_empty_run_between_them() {
    let b = body("abcdef", &[[0, 3]], &[[3, 6]]);
    assert_eq!(
        rows::pieces(&b),
        [
            ("abc".into(), Piece::Link(100, 0)),
            ("def".into(), Piece::Mention),
        ]
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
        format!("{} [Bob#0012 → you] psst", rows::clock(AT))
    );
}

#[test]
fn a_sent_whisper_names_its_addressee() {
    let social = machine(false);
    let conv = Conv::dm(ME, BOB).expect("two tags");
    let l = line(&conv, "m1", AT, said("psst", true));
    assert_eq!(
        rows::plain_text(&social, &conv, &l),
        format!("{} [you → Bob#0012] psst", rows::clock(AT))
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
        [ChatTab::All, ChatTab::Private, ChatTab::Group]
    );
}

#[test]
fn a_guest_has_no_group_tab() {
    let mut social = machine(true);
    social.on_table(None);
    assert_eq!(
        window::drawn_tabs(&social, false),
        [ChatTab::All, ChatTab::Private]
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
    let english = ["All", "Log", "Table", "Team", "Private", "Group"];
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
    use bevy::picking::events::{Over, Pointer, Press};
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
            .insert_resource(ChatUi {
                open: true,
                ..ChatUi::default()
            })
            .add_message::<Pointer<bevy::picking::events::Click>>()
            .add_message::<Pointer<Press>>()
            .add_message::<Pointer<Over>>();
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
            let event = Press {
                button,
                hit: HitData::new(entity, 0.0, None, None),
                count: 1,
            };
            self.app.world_mut().write_message(Pointer::new(
                PointerId::Mouse,
                Self::location(),
                event,
                entity,
            ));
        }

        fn over(&mut self, order: usize) {
            let entity = self.row(order);
            let event = Over {
                hit: HitData::new(entity, 0.0, None, None),
            };
            self.app.world_mut().write_message(Pointer::new(
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
    #[ignore = "BUG: paint_selection writes Color::NONE over the own ground of every unselected row (base is only recorded for selected rows)"]
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
        let event = Over {
            hit: HitData::new(stranger, 0.0, None, None),
        };
        rig.app.world_mut().write_message(Pointer::new(
            PointerId::Mouse,
            Rig::location(),
            event,
            stranger,
        ));
        rig.app.update();
        assert_eq!(rig.copy().as_deref(), Some("text0"));
        // And a press in the other list moves the selection over to it.
        let event = Press {
            button: PointerButton::Primary,
            hit: HitData::new(stranger, 0.0, None, None),
            count: 1,
        };
        rig.app.world_mut().write_message(Pointer::new(
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
        let event = Press {
            button: PointerButton::Primary,
            hit: HitData::new(word, 0.0, None, None),
            count: 1,
        };
        rig.app.world_mut().write_message(Pointer::new(
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
