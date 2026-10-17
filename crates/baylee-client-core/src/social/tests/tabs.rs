//! The chat window's fixed tabs: what each admits (§3), the filters that ride in `ChatPrefs`, and the machine's answers to "which lines" and "how many unread" for a filter.

use baylee_protocol::social::{Conv, Down, Notice, Person};

use super::fakegw::{Client, Gateway};
use crate::social::convs::Body;
use crate::social::{
    ChatLine, ChatPhrase, ChatPrefs, ChatTab, ConvId, GroupPick, Intent, LineKey, LineKind,
    LineState, RosterSeat, TabFilter, TableContext,
};

const ALICE: &str = "af03";
const BOB: &str = "0012";
const CAROL: &str = "0c11";
const GROUP: &str = "0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b";

fn dm() -> ConvId {
    Conv::dm(ALICE, BOB).expect("two tags")
}

fn table() -> ConvId {
    Conv::Table { game: "g1".into() }
}

fn team() -> ConvId {
    Conv::Team {
        game: "g1".into(),
        team: 1,
    }
}

fn group(id: &str) -> ConvId {
    Conv::Group { id: id.into() }
}

fn line(kind: LineKind) -> ChatLine {
    ChatLine {
        key: LineKey {
            conv: table(),
            id: "x".into(),
        },
        at: 1,
        kind,
        state: LineState::Final,
        hidden: false,
        mentions_me: false,
        failure: None,
    }
}

fn message() -> ChatLine {
    line(LineKind::Message {
        from: Person {
            tag: ALICE.into(),
            handle: "Alice#af03".into(),
        },
        mine: false,
        body: Body {
            text: "hi".into(),
            ..Body::default()
        },
    })
}

fn notice() -> ChatLine {
    line(LineKind::Notice {
        phrase: ChatPhrase::RosterMismatch,
        args: Vec::new(),
    })
}

fn server() -> ChatLine {
    line(LineKind::Server {
        phrase: ChatPhrase::MemberJoined,
        args: vec!["Carol#0c11".into()],
    })
}

// ---------------------------------------------------------------------------
// The tabs and their filters, as values.
// ---------------------------------------------------------------------------

#[test]
fn the_tabs_are_drawn_in_one_fixed_order_and_each_knows_its_place_and_name() {
    let names: Vec<&str> = ChatTab::ALL.iter().map(|t| t.name()).collect();
    assert_eq!(
        names,
        ["all", "global", "log", "table", "team", "private", "group"]
    );
    for (place, tab) in ChatTab::ALL.iter().enumerate() {
        assert_eq!(tab.index(), place, "{tab:?}");
    }
    let writable: Vec<ChatTab> = ChatTab::ALL.into_iter().filter(|t| t.is_chat()).collect();
    assert_eq!(
        writable,
        [
            ChatTab::Global,
            ChatTab::Table,
            ChatTab::Team,
            ChatTab::Private,
            ChatTab::Group
        ],
        "All and Log are read, never written in"
    );
}

#[test]
fn a_conversation_is_read_on_the_tab_of_its_kind() {
    assert_eq!(ChatTab::of(&Conv::Global), ChatTab::Global);
    assert_eq!(ChatTab::of(&dm()), ChatTab::Private);
    assert_eq!(ChatTab::of(&group(GROUP)), ChatTab::Group);
    assert_eq!(ChatTab::of(&table()), ChatTab::Table);
    assert_eq!(ChatTab::of(&team()), ChatTab::Team);
}

#[test]
fn a_group_pick_admits_all_some_or_none() {
    assert!(GroupPick::All.admits("a"));
    assert!(GroupPick::Only(vec!["a".into(), "b".into()]).admits("b"));
    assert!(!GroupPick::Only(vec!["a".into()]).admits("b"));
    assert!(
        !GroupPick::Only(Vec::new()).admits("a"),
        "an empty pick is none"
    );
    assert!(!GroupPick::None.admits("a"));
    assert_eq!(GroupPick::default(), GroupPick::All);
}

#[test]
fn each_tabs_standard_filter_shows_what_the_design_says() {
    // (global, log, table, team, whispers, groups, notices)
    let expect = |tab, global, log, table, team, whispers, groups, notices| {
        let want = TabFilter {
            global,
            log,
            table,
            team,
            whispers,
            groups,
            notices,
            friends_presence: false,
            timestamps: true,
        };
        assert_eq!(TabFilter::standard(tab), want, "{tab:?}");
    };
    expect(
        ChatTab::All,
        true,
        true,
        true,
        true,
        true,
        GroupPick::All,
        true,
    );
    expect(
        ChatTab::Global,
        true,
        false,
        false,
        false,
        false,
        GroupPick::None,
        true,
    );
    expect(
        ChatTab::Log,
        false,
        true,
        false,
        false,
        false,
        GroupPick::None,
        false,
    );
    expect(
        ChatTab::Table,
        false,
        false,
        true,
        false,
        false,
        GroupPick::None,
        true,
    );
    expect(
        ChatTab::Team,
        false,
        false,
        false,
        true,
        false,
        GroupPick::None,
        true,
    );
    expect(
        ChatTab::Private,
        false,
        false,
        false,
        false,
        true,
        GroupPick::None,
        true,
    );
    expect(
        ChatTab::Group,
        false,
        false,
        false,
        false,
        false,
        GroupPick::All,
        true,
    );

    let set = TabFilter::standard_set();
    assert_eq!(set.len(), ChatTab::ALL.len());
    for tab in ChatTab::ALL {
        assert_eq!(set[tab.index()], TabFilter::standard(tab), "{tab:?}");
        assert!(!set[tab.index()].is_empty(), "{tab:?}");
    }
}

#[test]
fn a_filter_that_admits_nothing_is_empty_whatever_its_timestamps_say() {
    assert!(TabFilter::nothing().is_empty());
    assert!(TabFilter::default().is_empty());
    let mut only_time = TabFilter::nothing();
    only_time.timestamps = false;
    assert!(only_time.is_empty());
    // Any one box makes it non-empty, friends coming online included.
    let boxes: [fn(&mut TabFilter); 8] = [
        |f| f.global = true,
        |f| f.log = true,
        |f| f.table = true,
        |f| f.team = true,
        |f| f.whispers = true,
        |f| f.groups = GroupPick::Only(vec!["g".into()]),
        |f| f.notices = true,
        |f| f.friends_presence = true,
    ];
    for (i, set) in boxes.iter().enumerate() {
        let mut filter = TabFilter::nothing();
        set(&mut filter);
        assert!(!filter.is_empty(), "box {i}");
    }
}

#[test]
fn a_stored_filter_missing_a_field_reads_that_field_as_off_and_the_time_as_on() {
    let read: TabFilter = serde_json::from_str(r#"{"table":true}"#).unwrap();
    assert_eq!(
        read,
        TabFilter {
            table: true,
            ..TabFilter::nothing()
        }
    );
    assert!(read.timestamps);
    let picked: TabFilter = serde_json::from_str(r#"{"groups":{"only":["g1"]}}"#).unwrap();
    assert_eq!(picked.groups, GroupPick::Only(vec!["g1".into()]));
    let round: TabFilter =
        serde_json::from_str(&serde_json::to_string(&TabFilter::standard(ChatTab::All)).unwrap())
            .unwrap();
    assert_eq!(round, TabFilter::standard(ChatTab::All));
}

#[test]
fn a_conversation_shows_on_the_tabs_whose_filter_admits_its_kind() {
    let convs = [dm(), group(GROUP), table(), team()];
    let shown = |tab| -> Vec<bool> {
        let f = TabFilter::standard(tab);
        convs.iter().map(|c| f.admits_conv(c)).collect()
    };
    // dm, group, table, team
    assert_eq!(shown(ChatTab::All), [true, true, true, true]);
    assert_eq!(shown(ChatTab::Global), [false, false, false, false]);
    assert_eq!(shown(ChatTab::Log), [false, false, false, false]);
    assert_eq!(shown(ChatTab::Table), [false, false, true, false]);
    assert_eq!(shown(ChatTab::Team), [false, false, false, true]);
    assert_eq!(shown(ChatTab::Private), [true, false, false, false]);
    assert_eq!(shown(ChatTab::Group), [false, true, false, false]);

    let only_one = TabFilter {
        groups: GroupPick::Only(vec![GROUP.into()]),
        ..TabFilter::nothing()
    };
    assert!(only_one.admits_conv(&group(GROUP)));
    assert!(!only_one.admits_conv(&group("0190a1b2-c3d4-7e5f-8a9b-000000000000")));
}

#[test]
fn a_filter_without_notices_hides_server_lines_and_dividers_but_not_messages() {
    let quiet = TabFilter {
        table: true,
        ..TabFilter::nothing()
    };
    let loud = TabFilter {
        notices: true,
        ..quiet.clone()
    };
    let at = table();
    assert!(quiet.admits(&at, &message()));
    assert!(
        quiet.admits(&at, &notice()),
        "a line this client wrote (a refusal, a command's answer) shows where its conversation does"
    );
    assert!(
        !quiet.admits(&at, &server()),
        "a server notice needs notices"
    );
    assert!(
        !quiet.admits(&at, &line(LineKind::Divider)),
        "and so does the New table divider"
    );
    for l in [message(), notice(), server(), line(LineKind::Divider)] {
        assert!(loud.admits(&at, &l), "{:?}", l.kind);
    }
}

#[test]
fn a_line_of_a_conversation_the_filter_refuses_is_never_admitted_even_with_notices_on() {
    let private = TabFilter::standard(ChatTab::Private);
    assert!(private.notices);
    for l in [message(), notice(), server(), line(LineKind::Divider)] {
        assert!(!private.admits(&table(), &l), "{:?}", l.kind);
        assert!(!private.admits(&group(GROUP), &l), "{:?}", l.kind);
    }
}

// ---------------------------------------------------------------------------
// ChatPrefs carries one filter per tab.
// ---------------------------------------------------------------------------

#[test]
fn the_default_chat_prefs_carry_the_standard_filter_of_every_tab() {
    let prefs = ChatPrefs::default();
    assert_eq!(prefs.tabs, TabFilter::standard_set());
    for tab in ChatTab::ALL {
        assert_eq!(prefs.filter(tab), TabFilter::standard(tab));
    }
}

#[test]
fn a_stored_list_shorter_than_the_tabs_falls_back_to_the_standard_filter_for_the_rest() {
    let mut prefs = ChatPrefs {
        tabs: vec![TabFilter::nothing()],
        ..ChatPrefs::default()
    };
    assert_eq!(prefs.filter(ChatTab::All), TabFilter::nothing());
    assert_eq!(
        prefs.filter(ChatTab::Group),
        TabFilter::standard(ChatTab::Group)
    );

    // Setting a later tab grows the list with the standard filters between.
    let mine = TabFilter {
        whispers: true,
        timestamps: false,
        ..TabFilter::nothing()
    };
    prefs.set_filter(ChatTab::Private, mine.clone());
    assert_eq!(prefs.tabs.len(), ChatTab::Private.index() + 1);
    assert_eq!(prefs.tabs[0], TabFilter::nothing(), "the stored one stays");
    for tab in [ChatTab::Global, ChatTab::Log, ChatTab::Table, ChatTab::Team] {
        assert_eq!(prefs.tabs[tab.index()], TabFilter::standard(tab), "{tab:?}");
    }
    assert_eq!(prefs.filter(ChatTab::Private), mine);
    assert_eq!(
        prefs.filter(ChatTab::Group),
        TabFilter::standard(ChatTab::Group),
        "past the end still falls back"
    );

    // Setting an early tab replaces it and changes nothing else.
    prefs.set_filter(ChatTab::Log, TabFilter::nothing());
    assert_eq!(prefs.filter(ChatTab::Log), TabFilter::nothing());
    assert_eq!(prefs.tabs.len(), ChatTab::Private.index() + 1);
}

#[test]
fn chat_prefs_stored_before_there_were_tabs_load_with_the_standard_tabs() {
    let read: ChatPrefs =
        serde_json::from_str(r#"{"history_days":7,"timestamps":false,"notice_seen":true}"#)
            .unwrap();
    assert_eq!(read.history_days, 7);
    assert!(!read.timestamps);
    assert!(read.notice_seen);
    assert_eq!(read.tabs, TabFilter::standard_set());
    let none: ChatPrefs = serde_json::from_str("{}").unwrap();
    assert_eq!(none, ChatPrefs::default());
}

#[test]
fn a_changed_filter_survives_the_round_trip_through_the_stored_prefs() {
    let mut prefs = ChatPrefs::default();
    prefs.set_filter(
        ChatTab::Table,
        TabFilter {
            team: true,
            groups: GroupPick::Only(vec![GROUP.into()]),
            timestamps: false,
            ..TabFilter::standard(ChatTab::Table)
        },
    );
    let read: ChatPrefs = serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
    assert_eq!(read, prefs);
}

// ---------------------------------------------------------------------------
// The machine's answers for a filter, with real lines through the gateway.
// ---------------------------------------------------------------------------

/// Alice, Bob and Carol at the table `tbl:g1`, Alice and Bob friends, one
/// device each.
fn evening() -> (Gateway, Client, Client) {
    let mut gw = Gateway::default();
    for (tag, handle) in [
        (ALICE, "Alice#af03"),
        (BOB, "Bob#0012"),
        (CAROL, "Carol#0c11"),
    ] {
        gw.handles.insert(tag.into(), handle.into());
    }
    gw.befriend(ALICE, BOB);
    gw.tables.insert(
        "tbl:g1".into(),
        vec![ALICE.into(), BOB.into(), CAROL.into()],
    );
    let bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let _carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let seat = |seat, handle: &str| RosterSeat {
        seat,
        handle: handle.into(),
        team: None,
    };
    alice.social.on_table(Some(TableContext {
        game: "g1".into(),
        playing: false,
        roster: vec![
            seat(0, "Alice#af03"),
            seat(1, "Bob#0012"),
            seat(2, "Carol#0c11"),
        ],
        me_seat: Some(0),
        my_team: None,
        watching: false,
    }));
    (gw, alice, bob)
}

/// The texts of the message lines `filter` admits at `bob`, oldest first.
fn texts(bob: &Client, filter: &TabFilter, limit: usize) -> Vec<String> {
    bob.social
        .lines_where(filter, limit)
        .into_iter()
        .filter_map(|(_, l)| l.body().map(|b| b.text.clone()))
        .collect()
}

fn std_filter(tab: ChatTab) -> TabFilter {
    TabFilter::standard(tab)
}

#[test]
fn every_tab_lists_only_the_lines_it_admits_and_all_interleaves_them_in_time_order() {
    let (mut gw, mut alice, mut bob) = evening();
    alice.say(&mut gw, "/w Bob one");
    alice.say(&mut gw, "/t two");
    alice.say(&mut gw, "/w Bob three");
    bob.deliver(&mut gw);
    assert_eq!(bob.texts("tbl:g1"), ["two"], "the table line did arrive");

    let all = std_filter(ChatTab::All);
    assert_eq!(texts(&bob, &all, 100), ["one", "two", "three"]);
    assert_eq!(
        texts(&bob, &std_filter(ChatTab::Private), 100),
        ["one", "three"]
    );
    assert_eq!(texts(&bob, &std_filter(ChatTab::Table), 100), ["two"]);
    assert!(texts(&bob, &std_filter(ChatTab::Log), 100).is_empty());
    assert!(texts(&bob, &std_filter(ChatTab::Team), 100).is_empty());
    assert!(texts(&bob, &std_filter(ChatTab::Group), 100).is_empty());
    assert!(texts(&bob, &TabFilter::nothing(), 100).is_empty());

    // Each line comes with the conversation it belongs to.
    let lines = bob.social.lines_where(&all, 100);
    let convs: Vec<String> = lines.iter().map(|(c, _)| c.to_string()).collect();
    assert_eq!(convs, ["dm:0012:af03", "tbl:g1", "dm:0012:af03"]);
}

#[test]
fn a_limit_keeps_the_latest_lines_still_oldest_first() {
    let (mut gw, mut alice, mut bob) = evening();
    for text in ["a", "b", "c", "d"] {
        alice.say(&mut gw, &format!("/w Bob {text}"));
    }
    bob.deliver(&mut gw);
    let all = std_filter(ChatTab::All);
    assert_eq!(texts(&bob, &all, 2), ["c", "d"]);
    assert_eq!(texts(&bob, &all, 4), ["a", "b", "c", "d"]);
    assert_eq!(texts(&bob, &all, 99), ["a", "b", "c", "d"]);
    assert!(texts(&bob, &all, 0).is_empty());
}

#[test]
fn server_notices_show_on_a_tab_only_when_it_admits_notices_and_the_conversation() {
    let (mut gw, mut alice, mut bob) = evening();
    alice.say(&mut gw, "/t hello");
    bob.deliver(&mut gw);
    let joined = |bob: &mut Client, gw: &mut Gateway, conv: &str| {
        bob.feed(
            gw,
            Down::Notice {
                conv: conv.into(),
                notice: Notice::MemberJoined {
                    person: Person {
                        tag: CAROL.into(),
                        handle: "Carol#0c11".into(),
                    },
                },
            },
        );
    };
    joined(&mut bob, &mut gw, "tbl:g1");
    joined(&mut bob, &mut gw, &format!("grp:{GROUP}"));

    let server_lines = |bob: &Client, filter: &TabFilter| -> Vec<String> {
        bob.social
            .lines_where(filter, 100)
            .into_iter()
            .filter(|(_, l)| matches!(l.kind, LineKind::Server { .. }))
            .map(|(c, _)| c.to_string())
            .collect()
    };
    let table_tab = std_filter(ChatTab::Table);
    assert_eq!(server_lines(&bob, &table_tab), ["tbl:g1"]);
    assert_eq!(
        server_lines(&bob, &std_filter(ChatTab::Group)),
        [format!("grp:{GROUP}")]
    );
    let mut both = server_lines(&bob, &std_filter(ChatTab::All));
    both.sort();
    assert_eq!(both, ["grp:0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b", "tbl:g1"]);
    assert!(server_lines(&bob, &std_filter(ChatTab::Private)).is_empty());

    let no_notices = TabFilter {
        notices: false,
        ..table_tab
    };
    assert!(server_lines(&bob, &no_notices).is_empty());
    assert_eq!(texts(&bob, &no_notices, 100), ["hello"], "messages stay");
}

#[test]
fn lines_of_every_conversation_come_out_sorted_by_time_then_id() {
    let (mut gw, mut alice, mut bob) = evening();
    alice.say(&mut gw, "/t one");
    bob.deliver(&mut gw);
    // Two notices at the very same moment, in two conversations.
    for conv in ["tbl:g1".to_string(), format!("grp:{GROUP}")] {
        bob.feed(
            &mut gw,
            Down::Notice {
                conv,
                notice: Notice::Renamed { name: "x".into() },
            },
        );
    }
    let lines = bob.social.lines_where(&std_filter(ChatTab::All), 100);
    assert_eq!(lines.len(), 3);
    let keys: Vec<(u64, &str)> = lines
        .iter()
        .map(|(_, l)| (l.at, l.key.id.as_str()))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted);
    assert_eq!(lines[1].1.at, lines[2].1.at, "the two notices tie");
    assert_eq!(lines[1].0, &table(), "and the earlier id comes first");
}

#[test]
fn the_unread_of_a_filter_sums_its_conversations_and_a_whisper_makes_it_urgent() {
    let (mut gw, mut alice, mut bob) = evening();
    alice.say(&mut gw, "/w Bob psst");
    alice.say(&mut gw, "/w Bob psst again");
    alice.say(&mut gw, "/t plain table talk");
    bob.deliver(&mut gw);

    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Private)),
        (2, true)
    );
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Table)),
        (1, false),
        "table talk that names nobody is not urgent"
    );
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::All)),
        (3, true)
    );
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Log)),
        (0, false)
    );
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Team)),
        (0, false)
    );
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Group)),
        (0, false)
    );
    assert_eq!(bob.social.unread_where(&TabFilter::nothing()), (0, false));
    assert_eq!(bob.social.unread_total(), 3, "the plain total agrees");
}

#[test]
fn a_mention_among_the_unread_lines_makes_the_tab_urgent_and_one_already_read_does_not() {
    let (mut gw, mut alice, mut bob) = evening();
    alice.say(&mut gw, "/t look [@Bob#0012]");
    bob.deliver(&mut gw);
    assert!(bob.conv("tbl:g1").unwrap().lines[0].mentions_me);
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Table)),
        (1, true)
    );

    // Read it; a plain line after it is the only unread one.
    bob.act(&mut gw, Intent::MarkRead(table()));
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Table)),
        (0, false)
    );
    alice.say(&mut gw, "/t anyone there?");
    bob.deliver(&mut gw);
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Table)),
        (1, false),
        "the mention is older than the unread window"
    );
}

#[test]
fn a_muted_conversation_adds_nothing_to_a_tabs_unread_or_its_urgency() {
    let (mut gw, mut alice, mut bob) = evening();
    bob.act(&mut gw, Intent::MuteConv(dm()));
    alice.say(&mut gw, "/w Bob quiet");
    alice.say(&mut gw, "/t loud");
    bob.deliver(&mut gw);
    assert_eq!(
        bob.conv("dm:0012:af03").unwrap().unread,
        1,
        "still counted in its own tab"
    );
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::Private)),
        (0, false)
    );
    assert_eq!(
        bob.social.unread_where(&std_filter(ChatTab::All)),
        (1, false)
    );
}

#[test]
fn the_last_whisper_partner_is_the_last_one_who_whispered_me_and_a_tag_resolves_to_a_person() {
    let (mut gw, mut alice, mut bob) = evening();
    assert_eq!(bob.social.last_whisper_partner(), None);
    assert!(bob.social.table().is_none(), "Bob sits at no table");
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    assert_eq!(bob.social.last_whisper_partner(), Some(ALICE));
    assert_eq!(
        alice.social.last_whisper_partner(),
        None,
        "writing a whisper is not being whispered"
    );
    assert_eq!(
        bob.social.person(ALICE),
        Person {
            tag: ALICE.into(),
            handle: "Alice#af03".into()
        }
    );
    assert_eq!(
        bob.social.person("dead").handle,
        "#dead",
        "an unknown tag is named by itself"
    );
    assert_eq!(alice.social.table().map(|t| t.game.as_str()), Some("g1"));
}
