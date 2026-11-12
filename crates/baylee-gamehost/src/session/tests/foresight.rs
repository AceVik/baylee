//! A house that tries its options in forks stays fair, and its search is a
//! function of what its seat sees (`session/foresight.rs`).
//!
//! The deals are `tests/fair_house.rs`'s: two games told apart only by what
//! the house may not see, the opponent's decklist, hand and library, with
//! an opponent who holds no cards at the start and only passes. With
//! foresight on, every fair level's answers are the same in both, answer
//! for answer, and searches did run. That holds the whole chain at once:
//! the candidates and the prior are read from the seat's view, the fork
//! deals nothing from the real list (`DeckKnowledge::Prior`), no chair of a
//! fork is told a real list, and the world seeds come from the policy seed
//! and the view.

use crate::Session;
use crate::record::{Line, Source};
use baylee_cards::decks::{LoadedDeck, by_name, house_deck, preset_for, probe_deck};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};

/// How far each deal is played.
const LAST_TURN: u32 = 7;

/// Two-drops and a counterspell, as `tests/fair_house.rs` deals the house.
fn merfolk_and_counters() -> LoadedDeck {
    house_deck("[deck:Merfolk]\n24 Island\n20 Coral Merfolk\n16 Counterspell\n")
        .expect("the pool has all three")
}

/// Creatures to attack with and burn to point, so every kind of root is met.
fn goblins_and_bolts() -> LoadedDeck {
    house_deck("[deck:Goblins]\n22 Mountain\n22 Goblin Piker\n16 Lightning Bolt\n")
        .expect("the pool has all three")
}

fn opponent(card: &str, copies: usize) -> LoadedDeck {
    let index = by_name(card).unwrap_or_else(|| panic!("the pool has {card}"));
    probe_deck(index, copies, 60).expect("basics to pad with")
}

fn deal(seed: u64, house: &LoadedDeck, theirs: &LoadedDeck, profile: AIProfile) -> GamePreset {
    let mut preset = preset_for(seed, house, theirs);
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].controller = SeatController::Ai(profile);
    preset.seats[1].controller = SeatController::Open;
    preset.seats[1].starting_hand = Some(Vec::new());
    preset
}

/// How a test's table is set: foresight on or off, a share of its searches
/// probed, a bound on a search's work, and what its host adds.
#[derive(Clone, Copy)]
struct Setting {
    foresight: bool,
    probe: Option<u64>,
    work: Option<u64>,
    host: Host,
}

impl Setting {
    /// Foresight on, nothing probed, no bound, no host.
    const ON: Self = Self {
        foresight: true,
        probe: None,
        work: None,
        host: Host::Bare,
    };

    /// Foresight on at a table hosted as an engine server hosts one by
    /// default ([`Host::Served`]).
    const HOSTED: Self = Self {
        host: Host::Served,
        ..Self::ON
    };
}

/// What a test's host adds to its table's searches
/// ([`Session::set_foresight_host`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Host {
    /// Nothing: each level's own bound alone, as a harness plays.
    Bare,
    /// As an engine server hosts a table by default: each level's whole
    /// bound under `expert`'s as a ceiling, one search at a time from a pool
    /// of one, a wall guard of the real clock that a test never reaches, and
    /// one searched answer a pump slice.
    Served,
    /// [`Self::Served`] with no search to spare.
    Busy,
    /// [`Self::Served`] with a wall guard that every search passes at once.
    Walled,
}

/// A real monotonic clock, as an engine server's.
fn monotonic() -> std::time::Duration {
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    ORIGIN.get_or_init(std::time::Instant::now).elapsed()
}

/// A clock a millisecond further on at every reading: a host whose every
/// moment is late.
fn ticking() -> std::time::Duration {
    static NOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    std::time::Duration::from_millis(NOW.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

impl Host {
    /// The host's bounds, or `None` for a table without one.
    fn bounds(self) -> Option<crate::HostForesight> {
        let served = crate::HostForesight {
            percent: 100,
            ceiling: AIProfile::EXPERT_WORK,
            wall: Some(crate::WallGuard {
                clock: monotonic,
                limit: std::time::Duration::from_secs(3600),
            }),
            permits: Some(crate::Searches::new(1)),
            yields: true,
        };
        match self {
            Self::Bare => None,
            Self::Served => Some(served),
            Self::Busy => Some(crate::HostForesight {
                permits: Some(crate::Searches::new(0)),
                ..served
            }),
            Self::Walled => Some(crate::HostForesight {
                wall: Some(crate::WallGuard {
                    clock: ticking,
                    limit: std::time::Duration::ZERO,
                }),
                ..served
            }),
        }
    }
}

/// What a table played: every answer the house gave, with the public table
/// after it; what its searches did; the leaves its probe kept.
struct Played {
    answers: Vec<(PlayerAction, String)>,
    stats: crate::ForesightStats,
    leaves: Vec<crate::ProbedLeaf>,
}

/// Every answer the house gave with foresight on at a table set as
/// `setting` says, with the public table after it, and how many answers it
/// searched.
fn house_answers(preset: &GamePreset, setting: Setting) -> (Vec<(PlayerAction, String)>, u64) {
    let table = played(preset, setting);
    (table.answers, table.stats.searched)
}

/// `preset` played to [`LAST_TURN`] at a table set as `setting` says.
fn played(preset: &GamePreset, setting: Setting) -> Played {
    let mut session = Session::new_recorded(preset, "foresight").expect("the table starts");
    session.describe("foresight".into(), vec!["house".into(), "person".into()]);
    if let Some(host) = setting.host.bounds() {
        session.set_foresight_host(host);
    }
    session.set_foresight(setting.foresight);
    session.set_foresight_probe(setting.probe);
    session.set_foresight_work(setting.work);
    let person = PlayerId::new(1);
    for _ in 0..10_000 {
        let _ = session.pump();
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        let Some((pending, _)) = session.view_for(person) else {
            continue;
        };
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        session
            .act(person, nothing)
            .expect("doing nothing is an answer");
    }
    let stats = session.foresight_stats().clone();
    let leaves = session.take_foresight_probe();
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let answers = record
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).ok()? {
            Line::Input {
                by: Source::House,
                action,
                seen,
                ..
            } => Some((action, seen.unwrap_or_default())),
            _ => None,
        })
        .collect();
    Played {
        answers,
        stats,
        leaves,
    }
}

#[test]
fn a_fair_house_with_foresight_answers_the_same_whatever_is_hidden() {
    fair_twins(Setting::ON);
}

/// The same twins at a table hosted as an engine server hosts one: the
/// host's ceiling, permits, wall guard and slices leave a fair level as fair.
#[test]
fn a_fair_house_with_foresight_answers_the_same_whatever_is_hidden_at_a_host() {
    fair_twins(Setting::HOSTED);
}

/// `tests/fair_house.rs`'s twin deals, told apart only by the opponent's
/// list, hand and library, played by every fair level at a table set as
/// `setting` says: answer for answer the same, searches and all.
fn fair_twins(setting: Setting) {
    let decks = [merfolk_and_counters(), goblins_and_bolts()];
    for (name, profile) in AIProfile::NAMED {
        if profile.peeks() {
            continue;
        }
        assert!(profile.foresight().is_some(), "{name} has a foresight tier");
        let mut searched = 0;
        for (seed, deck) in [(11, &decks[0]), (12, &decks[1]), (13, &decks[1])] {
            let (plains, n) =
                house_answers(&deal(seed, deck, &opponent("Plains", 60), profile), setting);
            let (elves, m) = house_answers(
                &deal(seed, deck, &opponent("Llanowar Elves", 30), profile),
                setting,
            );
            if let Some(at) = plains
                .iter()
                .zip(&elves)
                .position(|((a, _), (b, _))| a != b)
            {
                panic!(
                    "{name} with foresight answered differently at answer {at} when only \
                     hidden cards differed: {:?} against {:?}",
                    plains[at].0, elves[at].0
                );
            }
            assert_eq!(plains, elves, "{name}: the answers or the tables differ");
            assert_eq!(n, m, "{name}: the same positions are searched");
            searched += n;
        }
        assert!(searched > 10, "{name} searched {searched} answers only");
    }
}

/// A table hosted as an engine server hosts one answers as a bare one, answer
/// for answer and search for search: at a generation-9 level the host's
/// ceiling is the level's own bound, a free permit and a wall guard nobody
/// reaches change nothing, and a slice that ends after each search only
/// sends the frames sooner.
#[test]
fn a_hosted_search_answers_as_a_bare_one() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let bare = played(&preset, Setting::ON);
    let hosted = played(&preset, Setting::HOSTED);
    assert_eq!(bare.answers, hosted.answers, "a host changed an answer");
    assert_eq!(counts(&bare), counts(&hosted), "a host changed a search");
    assert!(hosted.stats.searched > 10, "too few searches");
    assert_eq!((hosted.stats.busy, hosted.stats.walled), (0, 0));
    assert!(hosted.stats.most_work <= AIProfile::EXPERT_WORK + STEP);
    assert!(
        hosted.stats.slowest_micros > 0,
        "timed by the guard's clock"
    );
    assert_eq!(bare.stats.slowest_micros, 0, "a bare table reads no clock");
}

/// The most a search runs past its bound: the price of the step it was in,
/// one fork or one house answer on a board of fewer than a hundred objects
/// (a combat search is held to what is left).
const STEP: u64 = super::super::simulation::FORK_WORK + super::super::simulation::answer_work(100);

/// A host with no search to spare makes none: every question a search
/// would take gets the level's own answer at once, so the table plays as one
/// without foresight, answer for answer, and no fork is made.
#[test]
fn a_host_with_no_search_to_spare_gives_the_levels_own_answers() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let busy = played(
        &preset,
        Setting {
            host: Host::Busy,
            ..Setting::ON
        },
    );
    let off = played(
        &preset,
        Setting {
            foresight: false,
            ..Setting::ON
        },
    );
    assert_eq!(busy.answers, off.answers, "a busy host changed an answer");
    assert!(
        busy.stats.busy > 10,
        "too few questions: {}",
        busy.stats.busy
    );
    assert_eq!(
        busy.stats.busy, busy.stats.asked,
        "every one went unsearched"
    );
    assert_eq!((busy.stats.searched, busy.stats.rollouts), (0, 0));
}

/// A search still running when the host's wall guard says it has taken too
/// long is given up where it stands, and the level's own answer is given:
/// with a guard every search passes at once, the table plays as one without
/// foresight, answer for answer, and counts every search as given up.
#[test]
fn a_search_past_the_wall_guard_gives_the_levels_own_answer() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let walled = played(
        &preset,
        Setting {
            host: Host::Walled,
            ..Setting::ON
        },
    );
    let off = played(
        &preset,
        Setting {
            foresight: false,
            ..Setting::ON
        },
    );
    assert_eq!(
        walled.answers, off.answers,
        "a late search changed an answer"
    );
    assert!(walled.stats.searched > 10, "too few searches");
    assert_eq!(
        walled.stats.walled, walled.stats.searched,
        "every one given up"
    );
    assert_eq!(walled.stats.replaced, 0);
    assert!(
        walled
            .stats
            .costs
            .iter()
            .all(|cost| cost.walled && cost.cut)
    );
}

/// A host that asks for it gets one searched answer a pump slice, so its
/// event loop hears its sockets between two searches; a slice without a
/// search runs on as before, and the game is the same.
#[test]
fn a_host_gets_one_searched_answer_a_slice() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let mut session = Session::new_recorded(&preset, "foresight").expect("the table starts");
    session.describe("foresight".into(), vec!["house".into(), "person".into()]);
    session.set_foresight(true);
    session.set_foresight_host(Host::Served.bounds().expect("served"));
    let person = PlayerId::new(1);
    let searched = |session: &Session| session.foresight_stats().searched;
    for _ in 0..10_000 {
        let before = searched(&session);
        let _ = session.pump_slice(4096);
        assert!(
            searched(&session) - before <= 1,
            "two searches in one slice"
        );
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        if session.needs_pump() {
            continue;
        }
        let (pending, _) = session.view_for(person).expect("the person is asked");
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        // An answer from a socket is followed by a slice of the house's.
        let before = searched(&session);
        session
            .act(person, nothing)
            .expect("doing nothing is an answer");
        assert!(
            searched(&session) - before <= 1,
            "two searches in one slice"
        );
    }
    assert!(searched(&session) > 10, "too few searches");
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let answers: Vec<(PlayerAction, String)> = record
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).ok()? {
            Line::Input {
                by: Source::House,
                action,
                seen,
                ..
            } => Some((action, seen.unwrap_or_default())),
            _ => None,
        })
        .collect();
    // A slice may stop past the last turn before the house's run of answers
    // there is over, where a whole pump would not: the same game, cut sooner.
    let whole = played(&preset, Setting::ON).answers;
    assert!(answers.len() > 20, "too short a game");
    assert!(whole.starts_with(&answers), "slices changed the game");
}

/// A table that has not switched foresight on plays as its heuristics do,
/// and so does a level at a generation from before it.
#[test]
fn foresight_is_off_until_a_table_switches_it_on() {
    let deck = goblins_and_bolts();
    let preset = deal(12, &deck, &opponent("Plains", 60), AIProfile::EXPERT);
    let mut session = Session::new(&preset).expect("the table starts");
    let _ = session.pump();
    assert_eq!(session.foresight_stats().asked, 0);
    let old = AIProfile::EXPERT.at_generation(AIProfile::SEER_DRAWS);
    assert!(old.foresight().is_none());
    let preset = deal(12, &deck, &opponent("Plains", 60), old);
    let mut session = Session::new(&preset).expect("the table starts");
    session.set_foresight(true);
    let _ = session.pump();
    assert_eq!(session.foresight_stats().asked, 0);
}

/// The priority questions `profile` searched in deal `seed` of the goblins,
/// each with the spells the offer had it pay for while they are cast.
fn searched_priority(profile: AIProfile, seed: u64) -> Vec<crate::SearchedQuestion> {
    let deck = goblins_and_bolts();
    let preset = deal(seed, &deck, &opponent("Llanowar Elves", 30), profile);
    let mut session = Session::new(&preset).expect("the table starts");
    session.set_foresight(true);
    session.set_foresight_questions([true, false, false], false);
    let person = PlayerId::new(1);
    for _ in 0..10_000 {
        let _ = session.pump();
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        let Some((pending, _)) = session.view_for(person) else {
            continue;
        };
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        session
            .act(person, nothing)
            .expect("doing nothing is an answer");
    }
    session.take_foresight_questions()
}

/// The spells a searched priority question tried that the offer had the seat
/// pay for while they are cast, with what each made in its worlds.
fn paid_casts(question: &crate::SearchedQuestion) -> Vec<&Vec<Option<f64>>> {
    let Pending::Priority { legal, .. } = &question.pending else {
        return Vec::new();
    };
    question
        .tried
        .iter()
        .filter(|(answer, _, _)| {
            matches!(answer, PlayerAction::CastSpell { card } if legal.payable.contains(card))
        })
        .map(|(_, _, worlds)| worlds)
        .collect()
}

/// From [`AIProfile::SPELL_ROOTS`] a search at priority tries the spells the
/// seat's lands pay for, cast and then paid in the fork; the generation
/// before it tries only what floating mana pays for, which at a root is
/// nothing with a cost.
#[test]
fn a_search_tries_the_spells_its_lands_pay_for() {
    let now = searched_priority(AIProfile::EXPERT, 12);
    let tried: Vec<_> = now.iter().flat_map(paid_casts).collect();
    assert!(!tried.is_empty(), "no spell paid while cast was tried");
    assert!(
        tried
            .iter()
            .any(|worlds| worlds.iter().all(Option::is_some)),
        "the forks paid for none of the spells tried: {tried:?}"
    );
    let before = searched_priority(AIProfile::EXPERT.at_generation(AIProfile::FORESIGHT), 12);
    assert!(!before.is_empty(), "generation 7 searched no priority");
    assert!(
        before
            .iter()
            .all(|question| paid_casts(question).is_empty()),
        "generation 7 tried a spell paid while cast"
    );
}

/// A bound under the cost of many of these deals' searches and over that of
/// others, so that some stop where they stand and some finish.
const TIGHT: u64 = 1_000_000;

/// The same deal, policy seeds and bound give the same answers and spend the
/// same work search by search, and the bound stops searches: the work is a
/// count, so a bounded search answers alike on every machine.
#[test]
fn a_bounded_search_is_deterministic() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let bounded = Setting {
        work: Some(TIGHT),
        ..Setting::ON
    };
    let first = played(&preset, bounded);
    let again = played(&preset, bounded);
    assert_eq!(
        first.answers, again.answers,
        "a bounded search answered otherwise"
    );
    assert_eq!(
        counts(&first),
        counts(&again),
        "the same searches spent otherwise"
    );
    let cut = first.stats.costs.iter().filter(|cost| cost.cut).count();
    assert!(cut > 0, "no search reached the bound of {TIGHT}");
    assert!(
        cut < first.stats.costs.len(),
        "every search stopped at the bound"
    );
    // A bound is hard but for the price of the step a search was in.
    for cost in &first.stats.costs {
        assert!(cost.work <= TIGHT + STEP, "{cost:?} ran past the bound");
    }
    assert!(first.stats.most_work <= TIGHT + STEP);
}

/// A host's share of a level's bound is a count as the level's is: a table
/// at half the work answers alike when played again, spends no more than
/// half of the level's bound on any search but for the step it was in, and
/// stops searches the level alone would have finished.
#[test]
fn a_host_share_of_the_bound_is_a_deterministic_bound() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let tightened = |session: &mut Session| {
        session.set_foresight_host(crate::HostForesight {
            percent: 25,
            ..Host::Served.bounds().expect("served")
        });
    };
    let play = || {
        let mut session = Session::new_recorded(&preset, "foresight").expect("the table starts");
        tightened(&mut session);
        session.set_foresight(true);
        let person = PlayerId::new(1);
        for _ in 0..10_000 {
            let _ = session.pump();
            if matches!(session.pending(), Pending::GameOver(_))
                || session.state().turn.number > LAST_TURN
            {
                break;
            }
            let Some((pending, _)) = session.view_for(person) else {
                continue;
            };
            let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
            session
                .act(person, nothing)
                .expect("doing nothing is an answer");
        }
        (session.take_record(), session.foresight_stats().clone())
    };
    let (first, stats) = play();
    let (again, _) = play();
    let house = |record: &[u8]| -> Vec<PlayerAction> {
        String::from_utf8_lossy(record)
            .lines()
            .filter_map(|line| match serde_json::from_str::<Line>(line).ok()? {
                Line::Input {
                    by: Source::House,
                    action,
                    ..
                } => Some(action),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        house(&first),
        house(&again),
        "a host's share answered otherwise"
    );
    let quarter = AIProfile::EXPERT_WORK / 4;
    assert!(
        stats.most_work <= quarter + STEP,
        "{} > {quarter}",
        stats.most_work
    );
    let cut = stats.costs.iter().filter(|cost| cost.cut).count();
    let whole = played(&preset, Setting::ON);
    let cut_whole = whole.stats.costs.iter().filter(|cost| cost.cut).count();
    assert!(
        cut > cut_whole,
        "a quarter of the bound stopped no more searches"
    );
}

/// What one search did, in counts: its kind, forks, combat and other
/// answers, the board those were given over, nodes, work, and whether the
/// bound stopped it.
type Counts = (u8, u32, u32, u32, u64, u64, u64, bool);

/// What a table's searches did, in counts and without their times, which no
/// answer reads and which differ from one run to the next.
fn counts(table: &Played) -> Vec<Counts> {
    table
        .stats
        .costs
        .iter()
        .map(|cost| {
            (
                cost.kind,
                cost.forks,
                cost.combat_answers,
                cost.other_answers,
                cost.board,
                cost.nodes,
                cost.work,
                cost.cut,
            )
        })
        .collect()
}

/// A bound of nothing leaves every searched question the level's own
/// answer: the table plays as one without foresight, answer for answer, and
/// its searches make no fork.
#[test]
fn a_bound_of_nothing_gives_the_levels_own_answers() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let none = played(
        &preset,
        Setting {
            work: Some(0),
            ..Setting::ON
        },
    );
    let off = played(
        &preset,
        Setting {
            foresight: false,
            ..Setting::ON
        },
    );
    assert_eq!(
        none.answers, off.answers,
        "a search with no work changed an answer"
    );
    assert!(none.stats.searched > 10, "too few questions were searched");
    assert_eq!(none.stats.replaced, 0);
    assert_eq!(none.stats.rollouts, 0);
    for cost in &none.stats.costs {
        assert_eq!((cost.forks, cost.work), (0, 0), "{cost:?}");
    }
}

/// A bounded search stays fair: `tests/fair_house.rs`'s two deals, told
/// apart only by the opponent's list, hand and library, are searched alike
/// under a bound that stops searches, answer for answer and unit for unit.
#[test]
fn a_bounded_search_answers_the_same_whatever_is_hidden() {
    let decks = [merfolk_and_counters(), goblins_and_bolts()];
    let bounded = Setting {
        work: Some(TIGHT),
        ..Setting::ON
    };
    let mut cut = 0;
    for (name, profile) in [("expert", AIProfile::EXPERT), ("novice", AIProfile::NOVICE)] {
        for (seed, deck) in [(11, &decks[0]), (12, &decks[1]), (13, &decks[1])] {
            let plains = played(&deal(seed, deck, &opponent("Plains", 60), profile), bounded);
            let elves = played(
                &deal(seed, deck, &opponent("Llanowar Elves", 30), profile),
                bounded,
            );
            assert_eq!(
                plains.answers, elves.answers,
                "{name} under a bound answered differently when only hidden cards differed"
            );
            assert_eq!(
                counts(&plains),
                counts(&elves),
                "{name}: the searches spent otherwise"
            );
            cut += plains.stats.costs.iter().filter(|cost| cost.cut).count();
        }
    }
    assert!(cut > 0, "no search reached the bound");
}

/// A table that judges its searches answers as one that does not: the
/// worlds of the probed questions are played on to the end and kept, each
/// with the level's own answer among those tried, and no answer changes.
#[test]
fn probing_a_search_changes_no_answer() {
    let deck = goblins_and_bolts();
    let preset = deal(
        12,
        &deck,
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let plain = played(&preset, Setting::ON);
    let probed = played(
        &preset,
        Setting {
            probe: Some(3),
            ..Setting::ON
        },
    );
    let (searched, leaves) = (plain.stats.searched, probed.leaves);
    assert_eq!(
        plain.answers, probed.answers,
        "the probe changed an answer or a table"
    );
    assert_eq!(searched, probed.stats.searched);
    assert!(plain.leaves.is_empty());
    assert!(!leaves.is_empty(), "no searched question was probed");
    let mut roots = std::collections::BTreeSet::new();
    let mut with_own = std::collections::BTreeSet::new();
    for leaf in &leaves {
        assert!((0.0..=1.0).contains(&leaf.value), "{leaf:?}");
        assert!(
            [0.0, 0.5, 1.0]
                .iter()
                .any(|result| (leaf.played - result).abs() < f64::EPSILON),
            "{leaf:?}"
        );
        if leaf.ended {
            assert!((leaf.value - leaf.played).abs() < f64::EPSILON, "{leaf:?}");
        }
        roots.insert(leaf.root);
        if leaf.candidate == 0 {
            with_own.insert(leaf.root);
        }
    }
    assert_eq!(roots, with_own);
    assert!(u64::try_from(roots.len()).is_ok_and(|n| n <= searched));
}

/// Where the searched answers of `expert` at generation 7 are pinned.
const SEARCH_PIN: &str = "tests/reference/foresight-generation-7.json";

/// The turn the pinned games are played to: far enough for every kind of
/// root, near enough for a test.
const PIN_TURN: u32 = 6;

/// The pinned tables: two duels of the frozen reference's decks.
const PIN_TABLES: [(&str, &str, u64); 2] =
    [("allytifact", "victory", 1), ("maik", "schwarzrand", 3)];

/// One pinned table: its decks and seed, the FNV-1a hash of the house's
/// answers, how many there were, and how many were searched and replaced.
type SearchPin = (String, u64, String, usize, u64, u64);

/// A table of the frozen reference's decks played to [`PIN_TURN`] by
/// `profile` in both chairs at a fair table with foresight on.
fn searched_table(first: &str, second: &str, seed: u64, profile: AIProfile) -> SearchPin {
    let deck = |name: &str| {
        let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    let (a, b) = (deck(first), deck(second));
    let mut preset = baylee_cards::decks::preset_for_all(seed, &[&a, &b]);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Ai(profile);
    }
    let mut session = Session::new_recorded(&preset, "reference").expect("the table starts");
    session.describe(format!("pin/{seed}"), vec!["0".into(), "1".into()]);
    session.set_fair_scouting(true);
    session.set_foresight(true);
    for _ in 0..20_000 {
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > PIN_TURN
        {
            break;
        }
        let _ = session.pump_at_most(1);
    }
    assert!(
        matches!(session.pending(), Pending::GameOver(_)) || session.state().turn.number > PIN_TURN,
        "table {seed} stalled before turn {PIN_TURN}"
    );
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let answers: Vec<PlayerAction> = record
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).ok()? {
            Line::Input { action, .. } => Some(action),
            _ => None,
        })
        .collect();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for action in &answers {
        let text = serde_json::to_string(action).expect("an action serializes");
        for byte in text.bytes().chain(*b"\n") {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let stats = session.foresight_stats();
    (
        format!("{first} v {second}"),
        seed,
        format!("{hash:016x}"),
        answers.len(),
        stats.searched,
        stats.replaced,
    )
}

/// `expert` at generation 7 searches as it did when generation 7 was the
/// newest: the fair level a campaign of the trained AI takes as its teacher
/// (`expert:generation=7`), so a later generation must not move it.
/// `tests/generation_pins.rs` pins generation 6, whose levels never search;
/// this pins what a search adds. Recorded with
/// `BAYLEE_FORESIGHT_PIN_RECORD=1 cargo nextest run -p baylee-gamehost --lib
/// expert_at_generation_7_searches_as_pinned`, once, before generation 8.
#[test]
fn expert_at_generation_7_searches_as_pinned() {
    let profile = AIProfile::EXPERT.at_generation(AIProfile::FORESIGHT);
    let played: Vec<SearchPin> = PIN_TABLES
        .iter()
        .map(|&(first, second, seed)| searched_table(first, second, seed, profile))
        .collect();
    let path = format!("{}/{SEARCH_PIN}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("BAYLEE_FORESIGHT_PIN_RECORD").is_some() {
        let text = serde_json::to_string_pretty(&played).expect("the pin serializes");
        std::fs::write(&path, text + "\n").unwrap_or_else(|e| panic!("{path}: {e}"));
        return;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let pinned: Vec<SearchPin> = serde_json::from_str(&text).expect("the pin reads");
    for (was, is) in pinned.iter().zip(&played) {
        assert_eq!(
            was, is,
            "expert:generation=7 no longer searches this table as pinned (decks, seed, \
             answer hash, answers, searched, replaced): a change reached generation 7 and \
             belongs behind `profile.plays(...)`"
        );
    }
    assert_eq!(pinned.len(), played.len());
    assert!(
        played.iter().all(|table| table.4 > 10),
        "too few searches: {played:?}"
    );
}

/// A search's meter takes the tighter of the level's bound and the table's,
/// so a host can lower a level's bound and never raise it; and the nodes one
/// combat search in a fork may visit are the level's where it sets them,
/// whatever the bound, and a share of the bound where it does not.
#[test]
fn a_host_only_ever_tightens_a_levels_bound() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Plains", 60),
        AIProfile::EXPERT,
    );
    let mut session = Session::new(&preset).expect("the table starts");
    let free = AIProfile::EXPERT
        .at_generation(AIProfile::SPELL_ROOTS)
        .foresight()
        .expect("expert searches");
    let level = baylee_core::preset::Foresight {
        work: 4_000_000,
        nodes: 16_384,
        ..free
    };
    let meter = |session: &Session, budget| {
        let meter = session.search_meter(budget);
        (meter.left(), meter.nodes(u32::MAX))
    };
    assert_eq!(meter(&session, level), (4_000_000, 16_384));
    assert_eq!(meter(&session, free), (u64::MAX, u32::MAX));
    session.set_foresight_work(Some(1_000_000));
    assert_eq!(
        meter(&session, level),
        (1_000_000, 16_384),
        "a host tightens"
    );
    assert_eq!(
        meter(&session, free),
        (
            1_000_000,
            u32::try_from(1_000_000 / crate::SEARCH_SHARE).unwrap()
        ),
        "a level that sets no nodes has the bound's share"
    );
    session.set_foresight_work(Some(9_000_000));
    assert_eq!(
        meter(&session, level),
        (4_000_000, 16_384),
        "a host never loosens"
    );
    session.set_foresight_nodes(Some(100));
    assert_eq!(meter(&session, level), (4_000_000, 100));
}

/// A host's ceiling bounds a level that bounds nothing itself (a generation
/// from before 9, which no lobby offers but a preset could name), and its
/// share lowers every level's bound; neither ever raises one, whatever the
/// host or the preset says.
#[test]
fn a_host_ceiling_and_share_bound_every_level() {
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Plains", 60),
        AIProfile::EXPERT,
    );
    let mut session = Session::new(&preset).expect("the table starts");
    let left = |session: &Session, profile: AIProfile| {
        session
            .search_meter(profile.foresight().expect("a tier"))
            .left()
    };
    let unbounded = AIProfile::EXPERT.at_generation(AIProfile::SPELL_ROOTS);
    // A tier past the last counts as the last.
    let odd = AIProfile {
        foresight: u8::MAX,
        ..unbounded
    };
    assert_eq!(left(&session, unbounded), u64::MAX, "no host, no ceiling");
    session.set_foresight_host(Host::Served.bounds().expect("served"));
    for profile in [unbounded, odd, AIProfile::EXPERT, AIProfile::SEER] {
        assert_eq!(
            left(&session, profile),
            AIProfile::EXPERT_WORK,
            "{profile:?}"
        );
    }
    assert_eq!(
        left(&session, AIProfile::NOVICE),
        1_333_333,
        "its own, under the ceiling"
    );
    for (percent, expert) in [
        (50, 4_000_000),
        (1, 80_000),
        (100, 8_000_000),
        (200, 8_000_000),
    ] {
        session.set_foresight_host(crate::HostForesight {
            percent,
            ..Host::Served.bounds().expect("served")
        });
        assert_eq!(left(&session, AIProfile::EXPERT), expert, "{percent} %");
        assert_eq!(left(&session, odd), expert, "{percent} % of the ceiling");
        assert!(
            left(&session, AIProfile::NOVICE) <= 1_333_333,
            "{percent} %"
        );
    }
}
