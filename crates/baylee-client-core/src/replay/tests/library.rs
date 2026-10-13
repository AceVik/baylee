//! The Replays list: the wire shape, the entries, the library, the clock.

use crate::replay::Verify;
use crate::replay::library::{
    Entry, EntryCheck, EntrySource, Filters, HostedCheck, HostedGame, HostedResult, HostedSeat,
    Library, LocalSeat, LocalSummary, Outcome, ReplayPage, ResultFilter, SourceFilter, Who, day_of,
    rfc3339_ms,
};

/// `GET /replays`' example, as the design (§8.3) gives it.
const PAGE: &str = r#"{"games":[{"game":"0199c8abcdef","started_at":"2026-10-13T18:31:02Z","ended_at":"2026-10-13T19:02:40Z",
  "complete":true,"seat":1,"turn":12,"result":"lost","public":true,
  "check":"exact","build":"0.8.0+build.212 (3f9a1c7e21)",
  "seats":[{"seat":0,"who":"player","name":"Bob#0012","commander":null},
           {"seat":1,"who":"you","name":"Alice#af03","colours":"G","commander":null},
           {"seat":2,"who":"house","name":"House AI","commander":null}]}],
 "total":57}"#;

/// The design's example parses into the shape both ends share, and what is
/// written parses back as the same page.
#[test]
fn the_replays_example_parses_and_round_trips() {
    let page: ReplayPage = serde_json::from_str(PAGE).expect("the example");
    assert_eq!(page.total, 57);
    assert_eq!(page.games.len(), 1);
    let game = &page.games[0];
    assert_eq!(game.game, "0199c8abcdef");
    assert_eq!(game.started_at, "2026-10-13T18:31:02Z");
    assert_eq!(game.ended_at.as_deref(), Some("2026-10-13T19:02:40Z"));
    assert!(game.complete);
    assert_eq!(game.seat, 1);
    assert_eq!(game.turn, Some(12));
    assert_eq!(game.result, HostedResult::Lost);
    assert!(game.public);
    assert_eq!(game.check, HostedCheck::Exact);
    assert_eq!(game.build.as_deref(), Some("0.8.0+build.212 (3f9a1c7e21)"));
    assert_eq!(
        game.seats
            .iter()
            .map(|s| (s.seat, s.who))
            .collect::<Vec<_>>(),
        [(0, Who::Player), (1, Who::You), (2, Who::House)]
    );
    assert_eq!(game.seats[1].colours.as_deref(), Some("G"));
    assert_eq!(game.seats[0].colours, None);
    assert_eq!(game.seats[0].commander, None);

    let text = serde_json::to_string(&page).expect("writes");
    assert_eq!(
        serde_json::from_str::<ReplayPage>(&text).expect("reads"),
        page
    );
    // A seat without colours writes none, as the gateway's.
    assert!(!text.contains(r#""colours":null"#));
}

/// Each word the wire has for who, result and check reads as its own value.
#[test]
fn the_wires_words_read_as_their_values() {
    let who = |w: &str| serde_json::from_str::<Who>(&format!(r#""{w}""#)).expect(w);
    assert_eq!(who("you"), Who::You);
    assert_eq!(who("player"), Who::Player);
    assert_eq!(who("former"), Who::Former);
    assert_eq!(who("house"), Who::House);
    assert_eq!(who("model"), Who::Model);
    let result = |w: &str| serde_json::from_str::<HostedResult>(&format!(r#""{w}""#)).expect(w);
    assert_eq!(result("won"), HostedResult::Won);
    assert_eq!(result("lost"), HostedResult::Lost);
    assert_eq!(result("draw"), HostedResult::Draw);
    assert_eq!(result("unfinished"), HostedResult::Unfinished);
    let check = |w: &str| serde_json::from_str::<HostedCheck>(&format!(r#""{w}""#)).expect(w);
    assert_eq!(check("exact"), HostedCheck::Exact);
    assert_eq!(check("checked"), HostedCheck::Checked);
    assert_eq!(check("unverified"), HostedCheck::Unverified);
    assert!(serde_json::from_str::<Who>(r#""admin""#).is_err());
}

/// A page the gateway may send with the optional fields left out still
/// reads.
#[test]
fn a_game_without_its_optional_fields_reads() {
    let game: HostedGame = serde_json::from_str(
        r#"{"game":"g","started_at":"2026-10-13T18:31:02Z","complete":false,"seat":0,
            "result":"unfinished","public":false,"check":"unverified",
            "seats":[{"seat":0,"who":"you","name":"A"}]}"#,
    )
    .expect("a game");
    assert_eq!(game.ended_at, None);
    assert_eq!(game.turn, None);
    assert_eq!(game.build, None);
    assert_eq!(game.seats[0].colours, None);
    assert_eq!(game.seats[0].commander, None);
}

fn seat(seat: u8, who: Who, name: &str, colours: Option<&str>) -> HostedSeat {
    HostedSeat {
        seat,
        who,
        name: name.into(),
        colours: colours.map(Into::into),
        commander: None,
        shared: None,
        mine: who == Who::You,
    }
}

fn game(id: &str, started_at: &str) -> HostedGame {
    HostedGame {
        game: id.into(),
        started_at: started_at.into(),
        ended_at: Some("2026-10-13T19:00:00Z".into()),
        complete: true,
        seat: 1,
        turn: Some(9),
        result: HostedResult::Won,
        public: true,
        check: HostedCheck::Exact,
        build: Some("b".into()),
        seats: vec![
            seat(0, Who::Player, "Bob#0012", None),
            seat(1, Who::You, "Alice#af03", Some("G")),
            seat(2, Who::House, "House AI", None),
        ],
        archived: false,
    }
}

/// A hosted game's entry has the viewer first, the others in seat order,
/// and only the viewer's colours, whatever the wire said of the others.
#[test]
fn a_hosted_entry_puts_you_first_and_keeps_only_your_colours() {
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    // The wire must not send an opponent's colours; if it does, the list
    // does not show them.
    hosted.seats[0].colours = Some("WU".into());
    hosted.seats[2].colours = Some("R".into());
    hosted.seats[0].commander = Some(77);
    let entry = Entry::hosted("https://gw", &hosted);

    assert_eq!(
        entry
            .players
            .iter()
            .map(|p| (p.seat, p.name.as_str(), p.you))
            .collect::<Vec<_>>(),
        [
            (1, "Alice#af03", true),
            (0, "Bob#0012", false),
            (2, "House AI", false)
        ]
    );
    assert_eq!(entry.players[2].house, Some(String::new()));
    assert_eq!(entry.players[1].house, None);
    assert_eq!(
        entry
            .decks
            .iter()
            .map(|d| (d.seat, d.colours.clone()))
            .collect::<Vec<_>>(),
        [(0, None), (1, Some("G".to_owned())), (2, None)]
    );
    assert_eq!(entry.decks[0].commander, Some(77));
    assert_eq!(entry.seat, 1);
    assert_eq!(entry.turns, Some(9));
    assert_eq!(entry.result, Outcome::Won);
    assert_eq!(
        entry.source,
        EntrySource::Hosted {
            gateway: "https://gw".into(),
            game: "g1".into(),
        }
    );
    assert_eq!(
        entry.started,
        rfc3339_ms("2026-10-13T18:31:02Z").expect("time")
    );
    assert_eq!(entry.ended, rfc3339_ms("2026-10-13T19:00:00Z"));
    assert_eq!(entry.minutes(), Some(29));
    assert!(!entry.is_local());
}

/// Models and former players are marked as such; the others are not.
#[test]
fn a_hosted_entry_marks_models_and_former_players() {
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    hosted.seats[0] = seat(0, Who::Former, "Former player", None);
    hosted.seats[2] = seat(2, Who::Model, "Language model", None);
    let entry = Entry::hosted("gw", &hosted);
    let by_seat = |s| {
        entry
            .players
            .iter()
            .find(|p| p.seat == s)
            .expect("a player")
    };
    assert!(by_seat(0).former && !by_seat(0).model && by_seat(0).house.is_none());
    assert!(by_seat(2).model && !by_seat(2).former && by_seat(2).house.is_none());
    assert!(by_seat(1).you && !by_seat(1).model && !by_seat(1).former);
}

/// A hosted game never offers every hand; the table's is the record's
/// `public`; the seat's is always there.
#[test]
fn a_hosted_entry_offers_every_hand_for_the_gateway_to_mask() {
    for public in [true, false] {
        let mut hosted = game("g1", "2026-10-13T18:31:02Z");
        hosted.public = public;
        let entry = Entry::hosted("gw", &hosted);
        assert!(entry.can.seat);
        assert_eq!(entry.can.public, public);
        // Any seat and all hands since §18.3: the gateway masks what a
        // seat's player did not share.
        assert!(entry.can.all_hands);
    }
}

/// An entry's check plays on at once only when nothing was checked.
#[test]
fn only_an_unverified_entry_plays_on() {
    for (wire, entry_check, verify) in [
        (HostedCheck::Exact, EntryCheck::Exact, Verify::Stop),
        (HostedCheck::Checked, EntryCheck::Checked, Verify::Stop),
        (
            HostedCheck::Unverified,
            EntryCheck::Unverified,
            Verify::Note,
        ),
    ] {
        let mut hosted = game("g1", "2026-10-13T18:31:02Z");
        hosted.check = wire;
        let entry = Entry::hosted("gw", &hosted);
        assert_eq!(entry.check, entry_check);
        assert_eq!(entry.check.verify(), verify);
    }
    assert_eq!(Verify::Stop.wire(), "stop");
    assert_eq!(Verify::Note.wire(), "note");
}

/// Each hosted result is the entry's outcome.
#[test]
fn hosted_results_are_outcomes() {
    for (result, outcome) in [
        (HostedResult::Won, Outcome::Won),
        (HostedResult::Lost, Outcome::Lost),
        (HostedResult::Draw, Outcome::Draw),
        (HostedResult::Unfinished, Outcome::Unfinished),
    ] {
        let mut hosted = game("g1", "2026-10-13T18:31:02Z");
        hosted.result = result;
        assert_eq!(Entry::hosted("gw", &hosted).result, outcome);
    }
}

fn local_seat(house: Option<&str>, colours: &str, commander: Option<u32>) -> LocalSeat {
    LocalSeat {
        house: house.map(Into::into),
        colours: colours.into(),
        commander,
    }
}

/// A record of the viewer in seat 1 against the house in seats 0 and 2.
fn summary() -> LocalSummary {
    LocalSummary {
        started: 1_000_000,
        ended: Some(1_000_000 + 25 * 60_000 + 1),
        complete: true,
        winners: vec![1],
        turn: Some(8),
        build: "this".into(),
        checked: false,
        seats: vec![
            local_seat(Some("sharp"), "R", Some(5)),
            local_seat(None, "GU", None),
            local_seat(Some("sharp"), "B", None),
        ],
    }
}

/// A kept record's result is read off its winners and whether it is whole.
#[test]
fn a_records_result_follows_winners_and_completeness() {
    let result = |winners: &[u8], complete: bool| {
        let mut s = summary();
        s.winners = winners.to_vec();
        s.complete = complete;
        Entry::of_record("a.gz", &s, "this", "Me").result
    };
    assert_eq!(result(&[1], true), Outcome::Won);
    assert_eq!(result(&[0], true), Outcome::Lost);
    assert_eq!(result(&[0, 2], true), Outcome::Lost);
    assert_eq!(result(&[0, 1], true), Outcome::Won);
    assert_eq!(result(&[], true), Outcome::Draw);
    // A record without its end has no result, whoever was ahead.
    assert_eq!(result(&[1], false), Outcome::Unfinished);
    assert_eq!(result(&[], false), Outcome::Unfinished);
}

/// A record this build wrote is exact; another build's is checked when its
/// lines carry the digest and unverified otherwise.
#[test]
fn a_records_check_follows_the_build_and_the_digest() {
    let check = |build: &str, checked: bool| {
        let mut s = summary();
        s.build = build.into();
        s.checked = checked;
        Entry::of_record("a.gz", &s, "this", "Me").check
    };
    assert_eq!(check("this", false), EntryCheck::Exact);
    assert_eq!(check("this", true), EntryCheck::Exact);
    assert_eq!(check("older", true), EntryCheck::Checked);
    assert_eq!(check("older", false), EntryCheck::Unverified);
}

/// A kept record's entry: the viewer is the first seat that is not the
/// house, listed first, and every seat's deck is shown.
#[test]
fn a_records_entry_has_the_viewer_first_and_every_deck() {
    let entry = Entry::of_record("a.gz", &summary(), "this", "Me");
    assert_eq!(entry.seat, 1);
    assert_eq!(
        entry
            .players
            .iter()
            .map(|p| (p.seat, p.you, p.name.as_str()))
            .collect::<Vec<_>>(),
        [(1, true, "Me"), (0, false, ""), (2, false, "")]
    );
    assert_eq!(entry.players[1].house.as_deref(), Some("sharp"));
    assert_eq!(entry.players[0].house, None);
    assert_eq!(entry.house.as_deref(), Some("sharp"));
    assert_eq!(
        entry
            .decks
            .iter()
            .map(|d| (d.seat, d.colours.clone()))
            .collect::<Vec<_>>(),
        [
            (0, Some("R".to_owned())),
            (1, Some("GU".to_owned())),
            (2, Some("B".to_owned()))
        ]
    );
    assert_eq!(entry.decks[0].commander, Some(5));
    assert_eq!(entry.turns, Some(8));
    assert_eq!(entry.started, 1_000_000);
    assert_eq!(entry.minutes(), Some(26));
    assert!(entry.is_local());
    assert_eq!(entry.can, crate::replay::Perspectives::LOCAL);
    assert_eq!(
        entry.source,
        EntrySource::Local {
            file: "a.gz".into()
        }
    );
}

/// Only a local record that is not the game being played can be deleted.
#[test]
fn only_a_local_record_that_is_not_live_can_be_deleted() {
    let local = Entry::of_record("a.gz", &summary(), "this", "Me");
    assert!(local.deletable(None));
    assert!(local.deletable(Some("b.gz")));
    assert!(!local.deletable(Some("a.gz")));
    let hosted = Entry::hosted("gw", &game("g1", "2026-10-13T18:31:02Z"));
    assert!(!hosted.deletable(None));
    assert!(!hosted.deletable(Some("g1")));
    assert!(!hosted.deletable(Some("b.gz")));
}

fn page(ids: &[(&str, &str)], total: u64) -> ReplayPage {
    ReplayPage {
        games: ids.iter().map(|(id, at)| game(id, at)).collect(),
        total,
    }
}

fn hosted_ids(library: &Library) -> Vec<String> {
    library
        .entries()
        .iter()
        .filter_map(|e| match &e.source {
            EntrySource::Hosted { game, .. } => Some(game.clone()),
            EntrySource::Local { .. } => None,
        })
        .collect()
}

fn local_ids(library: &Library) -> Vec<String> {
    library
        .entries()
        .iter()
        .filter_map(|e| match &e.source {
            EntrySource::Local { file } => Some(file.clone()),
            EntrySource::Hosted { .. } => None,
        })
        .collect()
}

fn local(file: &str, started: u64) -> Entry {
    let mut s = summary();
    s.started = started;
    Entry::of_record(file, &s, "this", "Me")
}

/// The first page replaces what was held, later pages add to it without
/// repeating a game, and the library knows whether the gateway has more.
#[test]
fn pages_replace_at_offset_zero_and_append_after() {
    let mut library = Library::default();
    library.set_local(vec![local("l.gz", 1)]);
    library.add_page(
        "gw",
        0,
        &page(
            &[("a", "2026-10-13T10:00:00Z"), ("b", "2026-10-12T10:00:00Z")],
            5,
        ),
    );
    assert_eq!(hosted_ids(&library), ["a", "b"]);
    assert_eq!((library.online_held, library.online_total), (2, 5));
    assert!(library.more_online());

    // The second page repeats one game (a list moved under the reader).
    library.add_page(
        "gw",
        2,
        &page(
            &[("b", "2026-10-12T10:00:00Z"), ("c", "2026-10-11T10:00:00Z")],
            5,
        ),
    );
    assert_eq!(hosted_ids(&library), ["a", "b", "c"]);
    assert_eq!(library.online_held, 3);
    assert!(library.more_online());

    library.add_page(
        "gw",
        3,
        &page(
            &[("d", "2026-10-10T10:00:00Z"), ("e", "2026-10-09T10:00:00Z")],
            5,
        ),
    );
    assert_eq!(library.online_held, 5);
    assert!(!library.more_online());

    // A first page again forgets those and keeps the local ones.
    library.add_page("gw", 0, &page(&[("z", "2026-10-14T10:00:00Z")], 1));
    assert_eq!(hosted_ids(&library), ["z"]);
    assert_eq!(local_ids(&library), ["l.gz"]);
    assert_eq!((library.online_held, library.online_total), (1, 1));
    assert!(!library.more_online());
}

/// The same game id on another gateway is another game.
#[test]
fn the_same_id_on_another_gateway_is_another_entry() {
    let mut library = Library::default();
    library.add_page("one", 0, &page(&[("a", "2026-10-13T10:00:00Z")], 1));
    library.add_page("two", 1, &page(&[("a", "2026-10-13T10:00:00Z")], 2));
    assert_eq!(library.entries().len(), 2);
}

/// Putting the local records in replaces the local ones and keeps the
/// online ones; forgetting the online ones keeps the local.
#[test]
fn local_records_replace_local_and_keep_online() {
    let mut library = Library::default();
    library.add_page("gw", 0, &page(&[("a", "2026-10-13T10:00:00Z")], 1));
    library.set_local(vec![local("one.gz", 10), local("two.gz", 20)]);
    assert_eq!(local_ids(&library), ["two.gz", "one.gz"]);
    library.set_local(vec![local("three.gz", 30)]);
    assert_eq!(local_ids(&library), ["three.gz"]);
    assert_eq!(hosted_ids(&library), ["a"]);

    library.forget_online();
    assert_eq!(hosted_ids(&library), Vec::<String>::new());
    assert_eq!(local_ids(&library), ["three.gz"]);
    assert_eq!((library.online_held, library.online_total), (0, 0));

    library.remove(&EntrySource::Local {
        file: "three.gz".into(),
    });
    assert!(library.entries().is_empty());
}

/// Online and local entries are mixed newest first.
#[test]
fn entries_are_newest_first_across_both_sources() {
    let mut library = Library::default();
    let at = |s| rfc3339_ms(s).expect("time");
    library.set_local(vec![
        local("old.gz", at("2026-10-01T00:00:00Z")),
        local("mid.gz", at("2026-10-11T12:00:00Z")),
    ]);
    library.add_page(
        "gw",
        0,
        &page(
            &[
                ("newest", "2026-10-13T00:00:00Z"),
                ("older", "2026-10-10T00:00:00Z"),
            ],
            2,
        ),
    );
    let started: Vec<u64> = library.entries().iter().map(|e| e.started).collect();
    let mut sorted = started.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(started, sorted);
    assert_eq!(started.len(), 4);
    assert!(
        matches!(&library.entries()[0].source, EntrySource::Hosted { game, .. } if game == "newest")
    );
    assert!(
        matches!(&library.entries()[1].source, EntrySource::Local { file } if file == "mid.gz")
    );
    assert!(
        matches!(&library.entries()[2].source, EntrySource::Hosted { game, .. } if game == "older")
    );
    assert!(
        matches!(&library.entries()[3].source, EntrySource::Local { file } if file == "old.gz")
    );
}

/// The filters: by source, by result and by a query over players and
/// commanders, which is not case-sensitive and asks the closure for names.
#[test]
fn filters_pass_by_source_result_and_query() {
    let mut library = Library::default();
    let mut won = game("won", "2026-10-13T10:00:00Z");
    won.seats[2].commander = Some(42);
    let mut lost = game("lost", "2026-10-12T10:00:00Z");
    lost.result = HostedResult::Lost;
    lost.seats[0].name = "Zed#0001".into();
    library.add_page(
        "gw",
        0,
        &ReplayPage {
            games: vec![won, lost],
            total: 2,
        },
    );
    let mut s = summary();
    s.started = 5;
    s.winners = vec![0];
    library.set_local(vec![Entry::of_record("l.gz", &s, "this", "Me")]);

    let names = |index: u32| (index == 42).then(|| "Atraxa, Praetors' Voice".to_owned());
    let shown = |filters: &Filters| -> Vec<String> {
        library
            .shown(filters, &names)
            .map(|e| match &e.source {
                EntrySource::Hosted { game, .. } => game.clone(),
                EntrySource::Local { file } => file.clone(),
            })
            .collect()
    };
    let by = |source, result, query: &str| Filters {
        source,
        result,
        query: query.into(),
    };

    assert_eq!(shown(&Filters::default()), ["won", "lost", "l.gz"]);
    assert_eq!(
        shown(&by(SourceFilter::House, ResultFilter::Any, "")),
        ["l.gz"]
    );
    assert_eq!(
        shown(&by(SourceFilter::Online, ResultFilter::Any, "")),
        ["won", "lost"]
    );
    assert_eq!(
        shown(&by(
            SourceFilter::All,
            ResultFilter::Only(Outcome::Lost),
            ""
        )),
        ["lost", "l.gz"]
    );
    assert_eq!(
        shown(&by(
            SourceFilter::Online,
            ResultFilter::Only(Outcome::Won),
            ""
        )),
        ["won"]
    );
    assert_eq!(
        shown(&by(SourceFilter::All, ResultFilter::Any, "zed")),
        ["lost"]
    );
    assert_eq!(
        shown(&by(SourceFilter::All, ResultFilter::Any, "  ALICE ")),
        ["won", "lost"]
    );
    assert_eq!(
        shown(&by(SourceFilter::All, ResultFilter::Any, "atraxa")),
        ["won"]
    );
    assert_eq!(
        shown(&by(SourceFilter::All, ResultFilter::Any, "nobody")),
        Vec::<String>::new()
    );
    // All three together.
    assert_eq!(
        shown(&by(
            SourceFilter::Online,
            ResultFilter::Only(Outcome::Lost),
            "zed"
        )),
        ["lost"]
    );
    assert_eq!(
        shown(&by(
            SourceFilter::House,
            ResultFilter::Only(Outcome::Lost),
            "zed"
        )),
        Vec::<String>::new()
    );
}

/// RFC 3339 times read as Unix milliseconds.
#[test]
fn rfc3339_reads_as_unix_milliseconds() {
    assert_eq!(rfc3339_ms("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(rfc3339_ms("1970-01-02T00:00:00Z"), Some(86_400_000));
    assert_eq!(rfc3339_ms("2026-10-13T18:31:02Z"), Some(1_791_916_262_000));
    // A leap day, and the day after it.
    assert_eq!(rfc3339_ms("2024-02-29T00:00:00Z"), Some(1_709_164_800_000));
    assert_eq!(rfc3339_ms("2024-03-01T00:00:00Z"), Some(1_709_251_200_000));
}

/// Fractions are milliseconds, however many digits; an offset moves the
/// instant the other way.
#[test]
fn rfc3339_reads_fractions_and_offsets() {
    let base = 1_791_916_262_000;
    assert_eq!(rfc3339_ms("2026-10-13T18:31:02.5Z"), Some(base + 500));
    assert_eq!(rfc3339_ms("2026-10-13T18:31:02.05Z"), Some(base + 50));
    assert_eq!(rfc3339_ms("2026-10-13T18:31:02.123456Z"), Some(base + 123));
    assert_eq!(rfc3339_ms("2026-10-13T20:31:02+02:00"), Some(base));
    assert_eq!(rfc3339_ms("2026-10-13T13:01:02-05:30"), Some(base));
    assert_eq!(rfc3339_ms("2026-10-13T20:31:02.5+02:00"), Some(base + 500));
}

/// What is not a time is none.
#[test]
fn rfc3339_refuses_what_is_not_a_time() {
    for text in [
        "",
        "garbage",
        "2026-10-13",
        "2026-10-13T18:31:02",
        "2026-10-13T18:31Z",
        "2026/10/13T18:31:02Z",
        "2026-10-13T18:31:0xZ",
        "2026-10-13T18:31:02Q",
    ] {
        assert_eq!(rfc3339_ms(text), None, "{text:?}");
    }
}

/// Games are grouped by the day they fall in at the viewer's offset.
#[test]
fn days_are_the_local_days() {
    let at = |s| rfc3339_ms(s).expect("time");
    let late = at("2026-10-13T22:30:00Z");
    let later = at("2026-10-13T23:30:00Z");
    let early = at("2026-10-13T21:30:00Z");
    let day = 20_739;
    assert_eq!(day_of(0, 0), 0);
    assert_eq!(day_of(late, 0), day);
    assert_eq!(day_of(later, 0), day);
    // Two hours east of UTC, 22:30 is 00:30 of the next day.
    assert_eq!(day_of(late, 7200), day + 1);
    assert_eq!(day_of(later, 7200), day + 1);
    assert_eq!(day_of(early, 7200), day);
    // West of UTC, just after midnight is still yesterday.
    assert_eq!(day_of(at("2026-10-14T00:30:00Z"), -3600), day);
    assert_eq!(day_of(at("2026-10-14T00:30:00Z"), 0), day + 1);
    // Before the epoch the day is below 0, not rounded towards it.
    assert_eq!(day_of(1_800_000, -3600), -1);
    assert_eq!(day_of(86_399_999, 0), 0);
    assert_eq!(day_of(86_400_000, 0), 1);
}

// ------------------------------------------------ keeping, hiding, sharing

/// The source of the hosted game `id` on the gateway `gw`.
fn hosted_source(id: &str) -> EntrySource {
    EntrySource::Hosted {
        gateway: "gw".into(),
        game: id.into(),
    }
}

/// What a library shows under `source`, as ids.
fn shown_ids(library: &Library, source: SourceFilter) -> Vec<String> {
    let filters = Filters {
        source,
        ..Filters::default()
    };
    library
        .shown(&filters, &|_| None)
        .map(|e| match &e.source {
            EntrySource::Hosted { game, .. } => game.clone(),
            EntrySource::Local { file } => file.clone(),
        })
        .collect()
}

/// The new wire fields read, and what the gateway leaves out reads as the
/// default (§18.1, §18.2): not archived, no consent said, not mine.
#[test]
fn the_archive_and_consent_fields_read_and_default() {
    let game: HostedGame = serde_json::from_str(
        r#"{"game":"g","started_at":"2026-10-13T18:31:02Z","complete":true,"seat":0,
            "result":"won","public":true,"check":"exact","archived":true,
            "seats":[{"seat":0,"who":"you","name":"A","shared":false,"mine":true},
                     {"seat":1,"who":"player","name":"B","shared":true},
                     {"seat":2,"who":"house","name":"House AI"}]}"#,
    )
    .expect("a game");
    assert!(game.archived);
    assert_eq!(game.seats[0].shared, Some(false));
    assert!(game.seats[0].mine);
    assert_eq!(game.seats[1].shared, Some(true));
    assert!(!game.seats[1].mine);
    assert_eq!(game.seats[2].shared, None);

    let plain: HostedGame = serde_json::from_str(
        r#"{"game":"g","started_at":"2026-10-13T18:31:02Z","complete":true,"seat":0,
            "result":"won","public":true,"check":"exact",
            "seats":[{"seat":0,"who":"you","name":"A"}]}"#,
    )
    .expect("a game");
    assert!(!plain.archived);
    assert_eq!(plain.seats[0].shared, None);
    assert!(!plain.seats[0].mine);
}

/// A consent not said is not written; one said is, `false` included, and
/// the whole round-trips.
#[test]
fn a_seats_consent_round_trips_and_an_unsaid_one_is_not_written() {
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    hosted.seats[1].shared = Some(false);
    hosted.archived = true;
    let value = serde_json::to_value(&hosted).expect("writes");
    assert_eq!(value["archived"], true);
    assert_eq!(value["seats"][1]["shared"], false);
    assert!(
        value["seats"][0].get("shared").is_none(),
        "an unsaid consent was written: {value}"
    );
    let back: HostedGame = serde_json::from_value(value).expect("reads");
    assert_eq!(back, hosted);
}

/// The viewer's own consent is the entry's `sharing`: as the viewer's seat
/// said it, ticked where it did not say, and none where the viewer sat at
/// no seat of the list's.
#[test]
fn a_hosted_entrys_sharing_is_the_viewers_seats_consent() {
    let with = |shared: Option<bool>| {
        let mut hosted = game("g1", "2026-10-13T18:31:02Z");
        hosted.seats[1].shared = shared;
        // Another player's consent never answers for the viewer's.
        hosted.seats[0].shared = Some(!shared.unwrap_or(true));
        Entry::hosted("gw", &hosted).sharing
    };
    assert_eq!(with(Some(true)), Some(true));
    assert_eq!(with(Some(false)), Some(false));
    assert_eq!(with(None), Some(true), "absent reads as ticked");

    // The seat is found by `mine` as well as by `who`.
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    hosted.seats[1].who = Who::Player;
    hosted.seats[1].mine = true;
    hosted.seats[1].shared = Some(false);
    assert_eq!(Entry::hosted("gw", &hosted).sharing, Some(false));

    // No seat is the viewer's: there is no consent to show.
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    hosted.seats[1] = seat(1, Who::Player, "Alice#af03", None);
    assert_eq!(Entry::hosted("gw", &hosted).sharing, None);
}

/// Every player row says whether its hand is shared: the house and a model
/// always, a person as they said (ticked where unsaid).
#[test]
fn a_players_hand_is_shared_as_they_said_and_always_for_the_house_and_a_model() {
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    hosted.seats = vec![
        seat(0, Who::Player, "Bob#0012", None),
        seat(1, Who::You, "Alice#af03", Some("G")),
        seat(2, Who::House, "House AI", None),
        seat(3, Who::Model, "Language model", None),
        seat(4, Who::Player, "Cy#0003", None),
        seat(5, Who::Former, "Former player", None),
    ];
    hosted.seats[0].shared = Some(false);
    hosted.seats[2].shared = Some(false);
    hosted.seats[3].shared = Some(false);
    hosted.seats[4].shared = Some(true);
    hosted.seats[5].shared = Some(false);
    let entry = Entry::hosted("gw", &hosted);
    let shared = |s: u8| {
        entry
            .players
            .iter()
            .find(|p| p.seat == s)
            .expect("a player")
            .shared
    };
    assert!(!shared(0), "a player who unticked");
    assert!(shared(1), "unsaid reads as ticked");
    assert!(shared(2), "the house always");
    assert!(shared(3), "a model always");
    assert!(shared(4), "a player who said yes");
    assert!(!shared(5), "a former player's consent stays");

    // Unsaid for a person reads as ticked.
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    hosted.seats[0].shared = None;
    let entry = Entry::hosted("gw", &hosted);
    assert!(entry.players.iter().find(|p| p.seat == 0).unwrap().shared);
}

/// A hosted entry carries the wire's `archived`, offers every hand whatever
/// the table's `public` says, and is not a local one.
#[test]
fn a_hosted_entry_carries_archived_and_offers_all_hands() {
    let mut hosted = game("g1", "2026-10-13T18:31:02Z");
    assert!(!Entry::hosted("gw", &hosted).archived);
    hosted.archived = true;
    hosted.public = false;
    let entry = Entry::hosted("gw", &hosted);
    assert!(entry.archived);
    assert!(entry.can.all_hands);
    assert!(entry.can.seat);
    assert!(!entry.can.public);
}

/// A kept record has no consent (it never leaves this computer), is not
/// archived, and offers all three.
#[test]
fn a_records_entry_has_no_sharing_and_is_not_archived() {
    let entry = Entry::of_record("a.gz", &summary(), "this", "Me");
    assert_eq!(entry.sharing, None);
    assert!(!entry.archived);
    assert!(entry.can.seat && entry.can.public && entry.can.all_hands);
}

/// Only a hosted game is hidden from the list; a kept record is deleted.
#[test]
fn only_a_hosted_entry_is_hideable() {
    let hosted = Entry::hosted("gw", &game("g1", "2026-10-13T18:31:02Z"));
    let kept = Entry::of_record("a.gz", &summary(), "this", "Me");
    assert!(hosted.hideable());
    assert!(!kept.hideable());
    // The two answers never overlap.
    assert_ne!(hosted.hideable(), hosted.deletable(None));
    assert_ne!(kept.hideable(), kept.deletable(None));
}

/// The first online page replaces the online games but keeps the local
/// records and the archived ones, whichever side they are on.
#[test]
fn the_first_page_keeps_local_and_archived_entries() {
    let mut library = Library::default();
    library.set_local(vec![local("l.gz", 1)]);
    library.add_page(
        "gw",
        0,
        &page(
            &[("a", "2026-10-13T10:00:00Z"), ("b", "2026-10-12T10:00:00Z")],
            2,
        ),
    );
    library.add_archived_page("gw", 0, &page(&[("old", "2026-10-01T10:00:00Z")], 1));
    // A local record archived by hand stays too.
    library.set_archived(
        &EntrySource::Local {
            file: "l.gz".into(),
        },
        true,
    );

    library.add_page("gw", 0, &page(&[("z", "2026-10-14T10:00:00Z")], 1));
    assert_eq!(hosted_ids(&library), ["z", "old"]);
    assert_eq!(local_ids(&library), ["l.gz"]);
    assert!(library.entries().iter().any(|e| e.archived && e.is_local()));
    assert_eq!((library.online_held, library.online_total), (1, 1));
    assert!(!library.more_online());

    // And a later page counts only what it adds, the archived ones not
    // among the held.
    library.add_page("gw", 1, &page(&[("y", "2026-10-13T10:00:00Z")], 3));
    assert_eq!(library.online_held, 2);
    assert!(library.more_online());
}

/// The archived list is its own: every game it adds is archived whatever
/// the wire said, offset 0 replaces the archived online games only, later
/// pages add without repeating, and the online counts are the other list's.
#[test]
fn an_archived_page_adds_archived_entries_and_leaves_the_online_counts_alone() {
    let mut library = Library::default();
    library.set_local(vec![local("l.gz", 1)]);
    library.add_page("gw", 0, &page(&[("a", "2026-10-13T10:00:00Z")], 4));
    let counts = (library.online_held, library.online_total);

    // The wire says `archived: false`; the list it came from says it is.
    library.add_archived_page(
        "gw",
        0,
        &page(
            &[("x", "2026-10-11T10:00:00Z"), ("y", "2026-10-10T10:00:00Z")],
            3,
        ),
    );
    assert_eq!(hosted_ids(&library), ["a", "x", "y"]);
    for entry in library.entries() {
        let EntrySource::Hosted { game, .. } = &entry.source else {
            assert!(!entry.archived, "a local record became archived");
            continue;
        };
        assert_eq!(entry.archived, game != "a", "{game}");
    }
    assert_eq!((library.online_held, library.online_total), counts);

    // The next page repeats one game and adds one.
    library.add_archived_page(
        "gw",
        2,
        &page(
            &[("y", "2026-10-10T10:00:00Z"), ("w", "2026-10-09T10:00:00Z")],
            3,
        ),
    );
    assert_eq!(hosted_ids(&library), ["a", "x", "y", "w"]);

    // A first page again forgets the archived online ones, nothing else.
    library.add_archived_page("gw", 0, &page(&[("v", "2026-10-08T10:00:00Z")], 1));
    assert_eq!(hosted_ids(&library), ["a", "v"]);
    assert_eq!(local_ids(&library), ["l.gz"]);
    assert_eq!((library.online_held, library.online_total), counts);
}

/// An archived page is mixed in newest first, like any.
#[test]
fn an_archived_page_is_sorted_newest_first() {
    let mut library = Library::default();
    library.add_page("gw", 0, &page(&[("mid", "2026-10-12T10:00:00Z")], 1));
    library.add_archived_page(
        "gw",
        0,
        &page(
            &[
                ("old", "2026-10-01T10:00:00Z"),
                ("new", "2026-10-13T10:00:00Z"),
            ],
            2,
        ),
    );
    assert_eq!(hosted_ids(&library), ["new", "mid", "old"]);
}

/// Archiving moves an entry out of every list but Archived, and bringing it
/// back moves it home; an unknown source changes nothing.
#[test]
fn archiving_moves_an_entry_between_the_lists() {
    let mut library = Library::default();
    library.set_local(vec![local("l.gz", 1)]);
    library.add_page(
        "gw",
        0,
        &page(
            &[("a", "2026-10-13T10:00:00Z"), ("b", "2026-10-12T10:00:00Z")],
            2,
        ),
    );
    assert_eq!(shown_ids(&library, SourceFilter::All), ["a", "b", "l.gz"]);
    assert!(shown_ids(&library, SourceFilter::Archived).is_empty());

    library.set_archived(&hosted_source("a"), true);
    library.set_archived(
        &EntrySource::Local {
            file: "l.gz".into(),
        },
        true,
    );
    assert_eq!(shown_ids(&library, SourceFilter::All), ["b"]);
    assert_eq!(shown_ids(&library, SourceFilter::Online), ["b"]);
    assert!(shown_ids(&library, SourceFilter::House).is_empty());
    assert_eq!(shown_ids(&library, SourceFilter::Archived), ["a", "l.gz"]);
    // Archiving removes nothing.
    assert_eq!(library.entries().len(), 3);

    let before = library.clone();
    library.set_archived(&hosted_source("nobody"), true);
    library.set_archived(
        &EntrySource::Hosted {
            gateway: "other".into(),
            game: "b".into(),
        },
        true,
    );
    assert_eq!(library, before, "another gateway's game is another entry");

    library.set_archived(&hosted_source("a"), false);
    assert_eq!(shown_ids(&library, SourceFilter::All), ["a", "b"]);
    assert_eq!(shown_ids(&library, SourceFilter::Archived), ["l.gz"]);
}

/// Archived is its own filter: it shows only archived entries, from both
/// sources, and every other source hides them; result and query still
/// apply on top.
#[test]
fn the_archived_filter_shows_only_archived_entries() {
    let mut library = Library::default();
    let mut lost = game("lost", "2026-10-12T10:00:00Z");
    lost.result = HostedResult::Lost;
    lost.seats[0].name = "Zed#0001".into();
    library.add_page(
        "gw",
        0,
        &ReplayPage {
            games: vec![game("won", "2026-10-13T10:00:00Z"), lost],
            total: 2,
        },
    );
    library.set_local(vec![local("l.gz", 5)]);
    library.set_archived(&hosted_source("won"), true);
    library.set_archived(&hosted_source("lost"), true);

    let by = |source, result, query: &str| Filters {
        source,
        result,
        query: query.into(),
    };
    let shown = |filters: &Filters| -> Vec<String> {
        library
            .shown(filters, &|_| None)
            .map(|e| match &e.source {
                EntrySource::Hosted { game, .. } => game.clone(),
                EntrySource::Local { file } => file.clone(),
            })
            .collect()
    };
    assert_eq!(shown(&Filters::default()), ["l.gz"]);
    assert_eq!(
        shown(&by(SourceFilter::Archived, ResultFilter::Any, "")),
        ["won", "lost"]
    );
    assert_eq!(
        shown(&by(
            SourceFilter::Archived,
            ResultFilter::Only(Outcome::Lost),
            ""
        )),
        ["lost"]
    );
    assert_eq!(
        shown(&by(SourceFilter::Archived, ResultFilter::Any, "zed")),
        ["lost"]
    );
    assert!(shown(&by(SourceFilter::Online, ResultFilter::Any, "")).is_empty());
    assert_eq!(
        shown(&by(SourceFilter::House, ResultFilter::Any, "")),
        ["l.gz"]
    );
    // Nothing archived at all: nothing shown.
    library.set_archived(&hosted_source("won"), false);
    library.set_archived(&hosted_source("lost"), false);
    assert!(shown_ids(&library, SourceFilter::Archived).is_empty());
}

/// The viewer's consent is written into the one hosted entry; a kept record
/// has none to write, and the other entries keep theirs.
#[test]
fn set_sharing_writes_the_consent_of_one_hosted_entry() {
    let mut library = Library::default();
    library.set_local(vec![local("l.gz", 1)]);
    library.add_page(
        "gw",
        0,
        &page(
            &[("a", "2026-10-13T10:00:00Z"), ("b", "2026-10-12T10:00:00Z")],
            2,
        ),
    );
    let sharing = |library: &Library, source: &EntrySource| {
        library
            .entries()
            .iter()
            .find(|e| &e.source == source)
            .map(|e| e.sharing)
    };
    assert_eq!(sharing(&library, &hosted_source("a")), Some(Some(true)));

    library.set_sharing(&hosted_source("a"), false);
    assert_eq!(sharing(&library, &hosted_source("a")), Some(Some(false)));
    assert_eq!(sharing(&library, &hosted_source("b")), Some(Some(true)));

    library.set_sharing(&hosted_source("a"), true);
    assert_eq!(sharing(&library, &hosted_source("a")), Some(Some(true)));

    // A kept record has no consent: asking for one gives it none.
    let record = EntrySource::Local {
        file: "l.gz".into(),
    };
    library.set_sharing(&record, true);
    assert_eq!(sharing(&library, &record), Some(None));

    // An unknown game changes nothing.
    let before = library.clone();
    library.set_sharing(&hosted_source("nobody"), false);
    assert_eq!(library, before);
}

/// Setting a consent does not archive, nor move, an entry.
#[test]
fn consent_and_archive_are_independent() {
    let mut library = Library::default();
    library.add_page("gw", 0, &page(&[("a", "2026-10-13T10:00:00Z")], 1));
    library.set_archived(&hosted_source("a"), true);
    library.set_sharing(&hosted_source("a"), false);
    let entry = &library.entries()[0];
    assert!(entry.archived);
    assert_eq!(entry.sharing, Some(false));
    library.set_archived(&hosted_source("a"), false);
    assert_eq!(library.entries()[0].sharing, Some(false));
}
