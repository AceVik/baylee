//! The deck builder against the host's own check, with no model reached:
//! a scripted chat answers fixed objects.

use super::brief::Job;
use super::check::{Checker, Rows, complete};
use super::facts::Facts;
use super::lend;
use super::pool::{Pool, Query};
use super::talk::{Chain, Chat, Replying, Who};
use super::work::{self, Base, Task};
use super::{file, history};
use crate::cli::Said;
use crate::llm::Tally;
use crate::mind::MindError;
use baylee_account::ranked::Format;
use baylee_client_core::cardquery::PoolFactsBody;
use baylee_client_core::lobby::BlockedCard;
use baylee_core::ids::CardIndex;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

const NOW: &str = "2026-11-27T03:00:00Z";

fn card(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
}

/// A table whose verdicts are, per card, three letters for commander,
/// standard and vintage (`l`, `b`, `r`, `n`, `-`).
fn facts(verdicts: &[(CardIndex, &str)]) -> Facts {
    let mut words: Vec<String> = verdicts.iter().map(|(_, v)| (*v).to_string()).collect();
    words.sort();
    words.dedup();
    let mut cards: Vec<Vec<u32>> = verdicts
        .iter()
        .map(|(card, v)| {
            let at = words.iter().position(|w| w == v).unwrap();
            vec![card.get(), u32::try_from(at).unwrap()]
        })
        .collect();
    cards.sort();
    cards.dedup_by_key(|row| row[0]);
    Facts {
        gateway: "https://gateway.test".into(),
        fetched: NOW.into(),
        pool: PoolFactsBody {
            version: "0123456789abcdef".into(),
            formats: vec!["commander".into(), "standard".into(), "vintage".into()],
            sets: Vec::new(),
            verdicts: words,
            cards,
        },
        blocked: Vec::new(),
    }
}

/// Every card this build has, legal in all three formats.
fn all_legal() -> Facts {
    let every: Vec<(CardIndex, &str)> = baylee_cards::all().map(|d| (d.index, "lll")).collect();
    facts(&every)
}

fn rows(name: &str, main: &[&str], commanders: &[&str]) -> Rows {
    Rows {
        name: name.into(),
        main: main.iter().map(|s| (*s).to_string()).collect(),
        sideboard: Vec::new(),
        commanders: commanders.iter().map(|s| (*s).to_string()).collect(),
    }
}

/// What a ranked host says: its registration, its blocks, its admission.
fn host_admits(format: Format, rows: &Rows, facts: &Facts) -> bool {
    let Ok(registration) = baylee_cards::ranked::registration(
        format.as_str(),
        &rows.main,
        &rows.sideboard,
        &rows.commanders,
    ) else {
        return false;
    };
    let deck = &registration.deck;
    let blocked = deck
        .main
        .iter()
        .chain(&deck.sideboard)
        .chain(&deck.commanders)
        .any(|c| facts.blocked_at(c.index.get(), NOW).is_some());
    !blocked && baylee_cards::ranked::admit(format.as_str(), deck, &facts.table()).is_ok()
}

/// A card this build has that the engine does not play completely.
fn a_stub() -> &'static str {
    baylee_cards::all()
        .find(|d| !complete(d.index) && baylee_cards::decks::by_name(d.name()) == Some(d.index))
        .expect("a card that is not fully scripted")
        .name()
}

/// A mono-white card that may lead alone.
fn a_white_leader() -> &'static str {
    let white = baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::White]);
    baylee_cards::all()
        .find(|d| {
            complete(d.index)
                && baylee_cards::decks::leader_of(d.index).is_some_and(|l| l.eligible_alone)
                && d.color_identity == white
                && baylee_cards::decks::by_name(d.name()) == Some(d.index)
        })
        .expect("a mono-white leader")
        .name()
}

/// The check passes exactly the decks the host admits, and says why it
/// refuses the others, naming the card.
#[test]
#[allow(clippy::too_many_lines)] // one table of cases
fn the_check_agrees_with_the_host_on_every_deck() {
    let stub = a_stub();
    let white = a_white_leader();
    let base = [
        (card("Island"), "lll"),
        (card("Plains"), "lll"),
        (card("Counterspell"), "lll"),
        (card("Llanowar Elves"), "lll"),
        (card("Sol Ring"), "lll"),
        (card("Swords to Plowshares"), "lll"),
        (card("General Tazri"), "lll"),
        (card(white), "lll"),
        (card(stub), "lll"),
    ];
    let legal = facts(&base);
    let with = |changed: &[(CardIndex, &str)]| {
        let mut verdicts = changed.to_vec();
        verdicts.extend(
            base.iter()
                .filter(|(c, _)| changed.iter().all(|(d, _)| d != c))
                .copied(),
        );
        facts(&verdicts)
    };
    let standard = rows(
        "U",
        &["52 Island", "4 Counterspell", "4 Llanowar Elves"],
        &[],
    );
    let mut blocked = legal.clone();
    blocked.blocked.push(BlockedCard {
        index: card("Counterspell").get(),
        name: "Counterspell".into(),
        reason: "a fault in its script".into(),
        until: Some("2026-11-28T00:00:00Z".into()),
    });
    let mut block_ended = blocked.clone();
    block_ended.blocked[0].until = Some("2026-11-26T00:00:00Z".into());
    let singleton = rows(
        "W",
        &["98 Plains", "1 Sol Ring", "1 Swords to Plowshares"],
        &[],
    );
    let commander = rows(
        "Tazri",
        &["97 Island", "1 Sol Ring", "1 Swords to Plowshares"],
        &["General Tazri"],
    );
    let stub_row = format!("4 {stub}");
    let cases: Vec<(&str, Format, Rows, Facts, bool, &str)> = vec![
        (
            "standard, legal",
            Format::Standard,
            standard.clone(),
            legal.clone(),
            true,
            "",
        ),
        (
            "banned",
            Format::Standard,
            standard.clone(),
            with(&[(card("Counterspell"), "lbl")]),
            false,
            "«Counterspell» is banned in standard",
        ),
        (
            "not legal",
            Format::Standard,
            standard.clone(),
            with(&[(card("Llanowar Elves"), "lnl")]),
            false,
            "«Llanowar Elves» is not legal in standard",
        ),
        (
            "no verdict",
            Format::Standard,
            standard.clone(),
            facts(&base[..1]),
            false,
            "has no standard verdict for «Counterspell»",
        ),
        (
            "a fifth copy",
            Format::Standard,
            rows("U", &["55 Island", "5 Counterspell"], &[]),
            legal.clone(),
            false,
            "«Counterspell» is in the deck 5 times",
        ),
        (
            "too few",
            Format::Standard,
            rows("U", &["59 Island"], &[]),
            legal.clone(),
            false,
            "holds 59 cards",
        ),
        (
            "blocked",
            Format::Standard,
            standard.clone(),
            blocked,
            false,
            "«Counterspell» is blocked on https://gateway.test until 2026-11-28 00:00 UTC",
        ),
        (
            "a block that ended",
            Format::Standard,
            standard.clone(),
            block_ended,
            true,
            "",
        ),
        (
            "a stub",
            Format::Standard,
            rows("U", &["56 Island", &stub_row], &[]),
            legal.clone(),
            false,
            "is not fully scripted",
        ),
        (
            "no card at all",
            Format::Standard,
            rows("U", &["56 Island", "4 Not A Card At All"], &[]),
            legal.clone(),
            false,
            "«Not A Card At All» is no card this build knows",
        ),
        (
            "highlander, legal",
            Format::Highlander,
            singleton.clone(),
            legal.clone(),
            true,
            "",
        ),
        (
            "highlander, a duplicate",
            Format::Highlander,
            rows(
                "W",
                &["97 Plains", "2 Sol Ring", "1 Swords to Plowshares"],
                &[],
            ),
            legal.clone(),
            false,
            "«Sol Ring» is in the deck 2 times, and at most 1",
        ),
        (
            "highlander, a duplicate in the sideboard",
            Format::Highlander,
            Rows {
                sideboard: vec!["1 Sol Ring".into()],
                ..singleton
            },
            legal.clone(),
            false,
            "«Sol Ring» is in the deck 2 times",
        ),
        (
            "commander, legal",
            Format::Commander,
            commander.clone(),
            legal.clone(),
            true,
            "",
        ),
        (
            "commander, not a leader",
            Format::Commander,
            rows("S", &["98 Island", "1 Swords to Plowshares"], &["Sol Ring"]),
            legal.clone(),
            false,
            "«Sol Ring» may not be a commander",
        ),
        (
            "commander, outside identity",
            Format::Commander,
            rows(
                "W",
                &["97 Island", "1 Sol Ring", "1 Swords to Plowshares"],
                &[white],
            ),
            legal.clone(),
            false,
            "«Island» is outside the commanders' colour identity",
        ),
        (
            "commander, banned there",
            Format::Commander,
            commander,
            with(&[(card("Sol Ring"), "bll")]),
            false,
            "«Sol Ring» is banned in commander",
        ),
    ];
    for (what, format, rows, facts, passes, says) in cases {
        let verdict = Checker::new(&facts).check(format, &rows, NOW);
        assert_eq!(verdict.passed(), passes, "{what}: {:?}", verdict.complaints);
        assert_eq!(
            verdict.passed(),
            host_admits(format, &rows, &facts),
            "{what}: the host and the check disagree"
        );
        if passes {
            assert!(verdict.complaints.is_empty(), "{what}");
        } else {
            assert!(
                verdict.complaints.iter().any(|c| c.contains(says)),
                "{what}: {:?} says nothing of «{says}»",
                verdict.complaints
            );
        }
    }
    // A real card this build has no script of is said as such.
    let missing = baylee_cards_index::ROWS
        .iter()
        .find(|row| baylee_cards::decks::by_name(row.name).is_none() && !row.name.contains(" // "))
        .expect("a card this build lacks");
    let row = format!("4 {}", missing.name);
    let verdict =
        Checker::new(&legal).check(Format::Standard, &rows("U", &["56 Island", &row], &[]), NOW);
    assert!(
        verdict.complaints[0].contains("is a real card, but not one this build plays"),
        "{:?}",
        verdict.complaints
    );
}

/// A lent deck is the same for the same seed, another for another, and
/// one the host admits, in every format; its file reads back as itself.
#[test]
fn every_lent_deck_is_legal_deterministic_and_admitted() {
    let facts = all_legal();
    let checker = Checker::new(&facts);
    for format in [Format::Standard, Format::Highlander, Format::Commander] {
        let pool = Pool::of(&checker, format, NOW);
        assert!(pool.len() > 1000, "{}: {}", format.as_str(), pool.len());
        let one = lend::lend(&checker, &pool, format, 7, NOW).unwrap();
        let again = lend::lend(&checker, &pool, format, 7, NOW).unwrap();
        assert_eq!(
            one.rows,
            again.rows,
            "{}: the same seed, the same deck",
            format.as_str()
        );
        assert_eq!(one.frozen.digest, again.frozen.digest);
        assert!(
            host_admits(format, &one.rows, &facts),
            "{}",
            format.as_str()
        );
        let other = lend::lend(&checker, &pool, format, 8, NOW).unwrap();
        assert_ne!(
            one.rows,
            other.rows,
            "{}: another seed, another deck",
            format.as_str()
        );
        assert!(host_admits(format, &other.rows, &facts));
        assert!(one.rows.sideboard.is_empty());
        assert_eq!(one.rows.commanders.is_empty(), format != Format::Commander);
        let text = file::text(&one.rows, format).unwrap();
        assert!(
            text.contains(&format!("# format: {}", format.as_str())),
            "{text}"
        );
        let (back, named) = file::read(&text, "x").unwrap();
        assert_eq!(named.as_deref(), Some(format.as_str()));
        assert_eq!(back, one.rows);
        let frozen = Checker::new(&facts)
            .check(format, &back, NOW)
            .frozen
            .unwrap();
        assert_eq!(
            frozen.digest, one.frozen.digest,
            "the runner freezes the file as lent"
        );
        // And as the runner reads it: the newest version, from its file.
        let dir = std::env::temp_dir().join(format!(
            "baylee-deck-lent-{}-{}",
            format.as_str(),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut record = history::Record {
            made: NOW.into(),
            how: "lend".into(),
            ..history::Record::default()
        };
        let saved = history::save(&dir, format, &text, &mut record).unwrap();
        assert_eq!(history::newest(&dir, format), Some(saved.deck.clone()));
        let offered = crate::ranked::Frozen::from_file(&saved.deck, format).unwrap();
        assert_eq!(offered.digest, one.frozen.digest, "{}", format.as_str());
        assert_eq!(offered.admitted(&facts), Ok(()), "{}", format.as_str());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// An empty table lends nothing, and says why.
#[test]
fn no_deck_is_lent_from_an_empty_pool() {
    let facts = facts(&[(card("Island"), "nnn")]);
    let checker = Checker::new(&facts);
    let pool = Pool::of(&checker, Format::Standard, NOW);
    assert!(pool.is_empty());
    let why = lend::lend(&checker, &pool, Format::Standard, 1, NOW).unwrap_err();
    assert!(why.contains("pool is empty"), "{why}");
}

/// The pool holds what the format allows, and a search narrows it.
#[test]
fn the_pool_is_the_formats_and_a_search_finds_in_it() {
    let facts = facts(&[
        (card("Island"), "lll"),
        (card("Counterspell"), "lnl"),
        (card("Llanowar Elves"), "lll"),
        (card("General Tazri"), "lll"),
    ]);
    let checker = Checker::new(&facts);
    let standard = Pool::of(&checker, Format::Standard, NOW);
    assert!(
        !standard.holds(card("Counterspell")),
        "not legal in standard"
    );
    assert!(standard.holds(card("Island")));
    let commander = Pool::of(&checker, Format::Commander, NOW);
    let (found, total) = commander
        .search(&Query {
            commander: Some(true),
            ..Query::default()
        })
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(found[0].name, "General Tazri");
    let (found, _) = commander
        .search(&Query {
            colors: Some("G".into()),
            types: Some("creature".into()),
            ..Query::default()
        })
        .unwrap();
    let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Llanowar Elves"]);
    let bad = Query {
        colors: Some("X".into()),
        ..Query::default()
    };
    assert!(commander.search(&bad).is_err());
    assert!(
        standard
            .read("Counterspell")
            .contains("not in the standard pool")
    );
    assert!(
        standard
            .read("Llanowar Elves")
            .contains("Llanowar Elves (identity G)")
    );
}

/// A chat that answers what it is given, in order, and keeps every
/// message it was sent.
struct Scripted {
    answers: Mutex<VecDeque<Result<Value, MindError>>>,
    heard: Arc<Mutex<Vec<String>>>,
    model: &'static str,
}

impl Scripted {
    fn new(
        model: &'static str,
        answers: Vec<Result<Value, MindError>>,
    ) -> (Self, Arc<Mutex<Vec<String>>>) {
        let heard = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                answers: Mutex::new(answers.into()),
                heard: Arc::clone(&heard),
                model,
            },
            heard,
        )
    }
}

impl Chat for Scripted {
    fn send<'a>(&'a self, opening: &'a str, message: &'a str, _budget: Duration) -> Replying<'a> {
        let mut heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let first = heard.is_empty();
        heard.push(if first {
            format!("{opening}\n\n{message}")
        } else {
            message.to_string()
        });
        drop(heard);
        let next = self
            .answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .unwrap_or_else(|| Err(MindError::Unavailable("the script is over".into())));
        Box::pin(std::future::ready(next.map(|value| Said {
            value: Some(value),
            text: String::new(),
            usage: None,
        })))
    }

    fn who(&self) -> Who {
        Who {
            model: self.model.into(),
            ..Who::default()
        }
    }

    fn tally(&self) -> Arc<Mutex<Tally>> {
        Arc::default()
    }
}

fn proposal(cards: &[(u32, &str)]) -> Value {
    let cards: Vec<Value> = cards
        .iter()
        .map(|(n, name)| json!({"count": n, "name": name}))
        .collect();
    json!({"propose": {
        "name": "Blue",
        "cards": cards,
        "note": "Counter everything, win with whatever is left.",
    }})
}

fn task(job: Job, base: Option<Base>) -> Task {
    Task {
        format: Format::Standard,
        job,
        rounds: 8,
        reply: Duration::from_secs(5),
        base,
        now: NOW.into(),
    }
}

fn heard(heard: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    heard.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// A model's pick that is not in the pool is refused with the fault
/// named, and it gets to fix it.
#[tokio::test]
async fn a_model_is_told_each_fault_and_fixes_it() {
    let facts = facts(&[
        (card("Island"), "lll"),
        (card("Counterspell"), "lbl"),
        (card("Llanowar Elves"), "lll"),
    ]);
    let checker = Checker::new(&facts);
    let pool = Pool::of(&checker, Format::Standard, NOW);
    let (chat, told) = Scripted::new(
        "scripted",
        vec![
            Ok(json!({"search": {"text": "counter target spell"}, "say": "looking"})),
            Ok(proposal(&[
                (52, "Island"),
                (4, "Llanowar Elves"),
                (4, "Made Up Card"),
            ])),
            Ok(proposal(&[
                (52, "Island"),
                (4, "Counterspell"),
                (4, "Llanowar Elves"),
            ])),
            Ok(json!({"propose": null, "search": null, "read": ["Llanowar Elves"]})),
            Ok(proposal(&[(56, "Island"), (4, "Llanowar Elves")])),
        ],
    );
    let chain = Chain::new(vec![Box::new(chat)]);
    let mut lines = Vec::new();
    let made = work::run(
        &chain,
        &checker,
        &pool,
        &task(Job::Build, None),
        &mut |l: &str| {
            lines.push(l.to_string());
        },
    )
    .await
    .unwrap();
    assert_eq!(made.rounds, 5);
    assert_eq!(made.rows.main, ["56 Island", "4 Llanowar Elves"]);
    assert!(made.note.starts_with("Counter everything"));
    let told = heard(&told);
    assert!(
        told[0].contains("The standard pool holds 2 cards"),
        "{}",
        told[0]
    );
    assert!(
        told[1].contains("0 cards of the standard pool match"),
        "{}",
        told[1]
    );
    assert!(
        told[2].contains("«Made Up Card» is no card this build knows"),
        "{}",
        told[2]
    );
    assert!(
        told[3].contains("«Counterspell» is banned in standard"),
        "{}",
        told[3]
    );
    assert!(
        told[4].contains("Llanowar Elves (identity G)"),
        "{}",
        told[4]
    );
    assert!(lines.iter().any(|l| l.contains("looking")), "{lines:?}");
}

/// A model that never passes ends the work with its last faults; a
/// limited model hands the same message to its fallback, which is told the
/// task from the start.
#[tokio::test]
async fn the_replies_run_out_and_a_fallback_takes_over() {
    let facts = facts(&[(card("Island"), "lll")]);
    let checker = Checker::new(&facts);
    let pool = Pool::of(&checker, Format::Standard, NOW);
    let (short, _) = Scripted::new(
        "short",
        (0..8).map(|_| Ok(proposal(&[(59, "Island")]))).collect(),
    );
    let chain = Chain::new(vec![Box::new(short)]);
    let why = work::run(
        &chain,
        &checker,
        &pool,
        &task(Job::Build, None),
        &mut |_: &str| {},
    )
    .await
    .unwrap_err();
    assert!(
        why.contains("in 8 replies") && why.contains("holds 59 cards"),
        "{why}"
    );

    let (limited, _) = Scripted::new(
        "limited",
        vec![Err(MindError::Limited {
            why: "429".into(),
            lifts_in: None,
        })],
    );
    let (fallback, told) = Scripted::new("fallback", vec![Ok(proposal(&[(60, "Island")]))]);
    let chain = Chain::new(vec![Box::new(limited), Box::new(fallback)]);
    let made = work::run(
        &chain,
        &checker,
        &pool,
        &task(Job::Build, None),
        &mut |_: &str| {},
    )
    .await
    .unwrap();
    assert_eq!(made.rows.main, ["60 Island"]);
    assert_eq!(chain.current().unwrap().who().model, "fallback");
    assert!(heard(&told)[0].contains("The standard pool holds"));
    let said = chain
        .said
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert!(said[0].contains("fallback takes over"), "{said:?}");
}

/// An improvement moves at most its cards, only cards the deck holds, and
/// is held to the check like a new deck.
#[tokio::test]
async fn an_improvement_is_bounded_and_checked() {
    let facts = facts(&[
        (card("Island"), "lll"),
        (card("Counterspell"), "lll"),
        (card("Llanowar Elves"), "lll"),
    ]);
    let checker = Checker::new(&facts);
    let pool = Pool::of(&checker, Format::Standard, NOW);
    let base = Base {
        rows: rows("Blue", &["56 Island", "4 Counterspell"], &[]),
        note: "Counter everything.".into(),
        games: "2026-11-27T02:00:00Z · version 1 · lost in 9 turns".into(),
    };
    let change = |out: &[(u32, &str)], put: &[(u32, &str)]| {
        let listed = |r: &[(u32, &str)]| -> Vec<Value> {
            r.iter()
                .map(|(n, name)| json!({"count": n, "name": name}))
                .collect()
        };
        json!({"change": {"out": listed(out), "in": listed(put), "note": "Elves came too late."}})
    };
    let (chat, told) = Scripted::new(
        "scripted",
        vec![
            Ok(change(&[(4, "Island")], &[(4, "Llanowar Elves")])),
            Ok(proposal(&[(60, "Island")])),
            Ok(change(&[(3, "Swamp")], &[(3, "Llanowar Elves")])),
            Ok(change(&[(3, "Island")], &[(3, "Llanowar Elves")])),
        ],
    );
    let chain = Chain::new(vec![Box::new(chat)]);
    let improve = task(Job::Improve(6), Some(base));
    let made = work::run(&chain, &checker, &pool, &improve, &mut |_: &str| {})
        .await
        .unwrap();
    let told = heard(&told);
    assert!(told[0].contains("YOUR DECK «Blue»") && told[0].contains("lost in 9 turns"));
    assert!(
        told[1].contains("moves 8 cards, and at most 6"),
        "{}",
        told[1]
    );
    assert!(told[2].contains("last step is change"), "{}", told[2]);
    assert!(told[3].contains("holds no «Swamp»"), "{}", told[3]);
    assert_eq!(
        made.rows.main,
        ["4 Counterspell", "53 Island", "3 Llanowar Elves"]
    );
    assert_eq!(made.removed, ["3 Island"]);
    assert_eq!(made.added, ["3 Llanowar Elves"]);
    assert_eq!(made.note, "Elves came too late.");
}

/// The runner's games are kept a line each and read back in order.
#[test]
fn games_are_kept_per_format() {
    let dir = std::env::temp_dir().join(format!("baylee-deck-games-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (n, result) in [(1, "won"), (2, "lost")] {
        history::record_game(
            &dir,
            Format::Commander,
            &history::Game {
                at: NOW.into(),
                version: n,
                result: result.into(),
                turns: Some(9),
                ..history::Game::default()
            },
        )
        .unwrap();
    }
    let games = history::games(&dir, Format::Commander);
    let results: Vec<&str> = games.iter().map(|g| g.result.as_str()).collect();
    assert_eq!(results, ["won", "lost"]);
    assert!(history::games(&dir, Format::Standard).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The answer schema holds every action, and the instructions name the
/// format's shape and the replies there are.
#[test]
fn the_brief_names_its_actions_and_the_format() {
    let schema = super::brief::schema();
    for action in ["search", "read", "propose", "change", "say"] {
        assert!(schema["properties"].get(action).is_some(), "{action}");
    }
    let system = super::brief::system(Format::Commander, Job::Build, 40);
    assert!(system.contains("exactly 100 cards") && system.contains("40 replies"));
    assert!(super::brief::system(Format::Standard, Job::Improve(6), 9).contains("at most 6 cards"));
}
