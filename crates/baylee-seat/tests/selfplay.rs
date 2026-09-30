//! The bridge's seat, played for whole games in this process: the house as
//! a mind behind the real standing orders, over the acceptance decks.

mod common;

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_seat::deck::Deck;
use baylee_seat::scripted::least_answer;
use baylee_seat::seat::By;
use baylee_seat::transcript::{Event, Note};
use baylee_seat::{
    Answer, BatchMind, Batched, Deliberation, Disclosure, HouseMind, Mind, Request, ScriptedMind,
};
use common::{Table, config, median_p90};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

/// Games the wake measurement plays; `BAYLEE_SEAT_GAMES` plays more.
fn games() -> u64 {
    std::env::var("BAYLEE_SEAT_GAMES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(2)
}

/// Enough for any game the acceptance decks play; a game that needs more is
/// stuck, and says so.
const MAX_ACTIONS: u32 = 40_000;

fn house() -> Arc<dyn Mind> {
    Arc::new(HouseMind::default())
}

/// §11 of the design: how often a mind is woken in a real game.
///
/// The house plays both chairs as a mind, each behind its seat's standing
/// orders on the design's rail ([`baylee_seat::wake::MIND_STOPS`]), over the
/// acceptance decks with the seats swapped every game. A mind is woken for
/// the questions the orders do not answer; without the orders it would be
/// woken for every question, which is the "before". The next steps of a
/// payment the mind began (a tap, the colour a land makes) are counted
/// apart: the mind is asked them marked `continuing`, and one that planned
/// the payment answers from its plan (§4.3: a mana plan in flight wakes
/// nobody).
///
/// Printed per seat and game, and as the median and 90th percentile. On
/// 2026-09-30, over 24 seat-games (`BAYLEE_SEAT_GAMES=12`), that was 411
/// questions a game at the median (517 at p90), 57 wakes (84), and 123 asks
/// with the payment steps (191). The ceilings asserted sit well above that; crossing one
/// means the standing orders stopped answering something they answered.
#[tokio::test]
async fn how_often_the_mind_is_woken() {
    let allytifact = Deck::acceptance("Allytifact").unwrap();
    let victory = Deck::acceptance("Victory").unwrap();
    let mut wakes = Vec::new();
    let mut woken = Vec::new();
    let mut questions = Vec::new();
    let mut places = BTreeMap::new();
    for seed in 0..games() {
        let decks = if seed % 2 == 0 {
            [&allytifact, &victory]
        } else {
            [&victory, &allytifact]
        };
        let table = Table::new(seed, decks, [house(), house()], &config());
        let played = table.play(MAX_ACTIONS).await;
        let result = played
            .result
            .unwrap_or_else(|| panic!("seed {seed}: no result after {} actions", played.actions));
        for (seat, stats) in played.stats.iter().enumerate() {
            println!(
                "seed {seed} seat {seat} ({}): turns {}, questions {}, standing {} \
                 (nothing {}, quiet {}, no attack {}, no block {}), wakes {} {:?}, payment steps {}, \
                 outcome {:?}",
                decks[seat].name,
                stats.turns,
                stats.questions,
                stats.standing.total(),
                stats.standing.nothing_to_do,
                stats.standing.quiet_window,
                stats.standing.no_attackers,
                stats.standing.no_blockers,
                stats.wakes,
                stats.woken,
                stats.continuations,
                stats.outcome,
            );
            assert_eq!(
                stats.fallbacks.total(),
                0,
                "the house as a mind never needs the house"
            );
            assert_eq!(
                stats.refused_by_referee + stats.refused_by_table,
                0,
                "{stats:?}"
            );
            assert_eq!(stats.unanswerable, 0);
            wakes.push(stats.wakes);
            woken.push(stats.wakes + stats.continuations);
            questions.push(stats.questions);
            tally(&played.notes[seat], &mut places);
        }
        println!("seed {seed}: {result:?} after {} actions", played.actions);
    }
    let mut places: Vec<_> = places.into_iter().collect();
    places.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (at, (n, passed)) in places.iter().take(16) {
        println!("  woken {n:>5} × {at:<40} passed {passed:>5}");
    }
    let (wake_median, wake_p90) = median_p90(&wakes);
    let (woken_median, woken_p90) = median_p90(&woken);
    let (asked_median, asked_p90) = median_p90(&questions);
    println!(
        "per seat and game, over {} seat-games: questions (no standing orders) median {asked_median}, \
         p90 {asked_p90}; wakes median {wake_median}, p90 {wake_p90}; wakes with payment steps \
         median {woken_median}, p90 {woken_p90}",
        wakes.len()
    );
    assert!(
        wake_p90 <= 150,
        "the mind is woken {wake_p90} times a game at the 90th percentile"
    );
    assert!(
        wake_median * 4 <= asked_median,
        "the standing orders answer at least three questions in four"
    );
}

/// Counts where one seat's wakes were, by the kind of question, the reason,
/// whose turn and the step, and how many of them the mind only passed.
fn tally(notes: &[Note], places: &mut BTreeMap<String, (u32, u32)>) {
    let mut asked = BTreeMap::new();
    for note in notes {
        match &note.event {
            Event::Asked {
                kind,
                why: Some(why),
                step,
                their_turn,
                retry: false,
                ..
            } => {
                let side = if *their_turn { "theirs" } else { "mine" };
                asked.insert(note.question, format!("{kind} {why:?}, {side} {step:?}"));
            }
            Event::Answered {
                by: By::Mind,
                action,
                ..
            } => {
                if let Some(at) = asked.remove(&note.question) {
                    let entry = places.entry(at).or_default();
                    entry.0 += 1;
                    entry.1 += u32::from(*action == PlayerAction::PassPriority);
                }
            }
            _ => {}
        }
    }
}

/// Two seats, one mind: asked together, answered in one call.
///
/// In the opening-hand window both seats are asked at once; a batching mind
/// behind both is handed both requests in one `decide_batch`.
#[tokio::test]
async fn one_mind_answers_two_seats_in_one_call() {
    /// Each call's requests, as (seat, kind of question).
    type Calls = Arc<Mutex<Vec<Vec<(u8, &'static str)>>>>;
    struct Counting {
        calls: Calls,
    }
    impl BatchMind for Counting {
        fn decide_batch(&self, requests: Vec<Request>) -> Deliberation<'_> {
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(
                    requests
                        .iter()
                        .map(|r| {
                            (
                                r.context.seat.get(),
                                baylee_seat::scripted::kind(&r.pending),
                            )
                        })
                        .collect(),
                );
            let house = HouseMind::default();
            let answers = requests
                .iter()
                .map(|r| Ok(Answer::new(house.answer(&r.context, &r.view, &r.pending))))
                .collect();
            Box::pin(std::future::ready(answers))
        }

        fn disclosure(&self) -> Disclosure {
            Disclosure::House
        }
    }
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mind: Arc<dyn Mind> = Arc::new(Batched::spawn(
        Counting {
            calls: Arc::clone(&calls),
        },
        8,
    ));
    let allytifact = Deck::acceptance("Allytifact").unwrap();
    let victory = Deck::acceptance("Victory").unwrap();
    let mut table = Table::new(
        7,
        [&allytifact, &victory],
        [Arc::clone(&mind), mind],
        &config(),
    );
    while table.rounds.is_empty() {
        assert!(
            table.step().await,
            "the table stopped before anybody was asked"
        );
    }
    let calls = calls.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(table.rounds, vec![2], "both seats were asked in one round");
    assert_eq!(
        calls,
        vec![vec![(0, "Mulligan"), (1, "Mulligan")]],
        "and answered in one call"
    );
}

/// The house answers from what the house may see: the same answer whatever
/// the log handed over and whatever the clock and the seat's own policy
/// fields say, for every question of a whole game.
#[tokio::test]
async fn the_house_reads_neither_the_log_nor_the_clock() {
    let allytifact = Deck::acceptance("Allytifact").unwrap();
    let victory = Deck::acceptance("Victory").unwrap();
    let mut table = Table::new(3, [&allytifact, &victory], [house(), house()], &config());
    while table.result.is_none() && table.actions < MAX_ACTIONS && table.step().await {}
    assert!(table.result.is_some());
    let house = HouseMind::default();
    let mut compared = 0;
    for (_, request) in &table.requests {
        let told = house.decide(request.clone()).await.unwrap().action;
        let mut blind = request.clone();
        blind.log = baylee_view::LogTail::default();
        blind.view.decision_remaining_ms = Some(1);
        blind.view.policy_acts.clear();
        let untold = house.decide(blind).await.unwrap().action;
        assert_eq!(
            told, untold,
            "question {} ({:?})",
            request.question, request.pending
        );
        compared += 1;
    }
    assert!(
        compared > 50,
        "a whole game asks the house more than {compared} questions"
    );
}

/// Every line of the seat's log reaches the mind, once and in order: each
/// request hands over the lines since the last one, including what happened
/// while the standing orders answered.
#[tokio::test]
async fn every_log_line_reaches_the_mind_once_and_in_order() {
    let scripted = ScriptedMind::idle().rule({
        let house = HouseMind::default();
        move |request: &Request| {
            Some(house.answer(&request.context, &request.view, &request.pending))
        }
    });
    let seen = scripted.seen();
    let allytifact = Deck::acceptance("Allytifact").unwrap();
    let victory = Deck::acceptance("Victory").unwrap();
    let mut table = Table::new(
        5,
        [&allytifact, &victory],
        [Arc::new(scripted), house()],
        &config(),
    );
    while table.result.is_none() && table.actions < MAX_ACTIONS && table.step().await {}
    assert!(table.result.is_some());
    let seen = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let mut next = 0;
    for request in &seen {
        assert_eq!(
            request.log_from, next,
            "question {} starts where the last ended",
            request.question
        );
        next += u32::try_from(request.log_lines).unwrap();
    }
    // The lines after the seat's last question were never handed over:
    // nothing asked for them.
    let held = table.cores[0].memory().log().len();
    assert!(next > 0 && next as usize <= held, "handed {next} of {held}");
}

/// A mind that cannot answer is answered for by the house, and a mind that
/// fails three times in a row is taken off the table.
#[tokio::test]
async fn a_failing_mind_is_answered_for_and_taken_down() {
    let allytifact = Deck::acceptance("Allytifact").unwrap();
    let victory = Deck::acceptance("Victory").unwrap();
    let failing: Arc<dyn Mind> = Arc::new(ScriptedMind::idle().failing(u32::MAX));
    let table = Table::new(1, [&allytifact, &victory], [failing, house()], &config());
    let played = table.play(MAX_ACTIONS).await;
    let stats = &played.stats[0];
    assert!(
        played.result.is_some(),
        "the game was played out by the house"
    );
    assert_eq!(
        stats.fallbacks.unavailable, 3,
        "three failures, then the mind is down"
    );
    assert_eq!(stats.mind_down, 1);
    assert_eq!(stats.answered.mind, 0);
    assert!(
        stats.fallbacks.down > 0,
        "and the house answered while it was down"
    );
    assert_eq!(stats.answered.house, stats.wakes + stats.continuations);
}

/// An answer the referee refuses earns the mind one retry, told why; a
/// second refused answer is the house's to replace.
#[tokio::test]
async fn a_refused_answer_is_retried_once_and_then_replaced() {
    let allytifact = Deck::acceptance("Allytifact").unwrap();
    let victory = Deck::acceptance("Victory").unwrap();
    let reasons = Arc::new(Mutex::new(Vec::new()));
    let told = Arc::clone(&reasons);
    // Keeps its hand with a wrong answer twice, then plays by the house.
    let mind = ScriptedMind::idle()
        .rule(move |request: &Request| {
            if let Some(refusal) = &request.retry {
                told.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(refusal.reason.clone());
            }
            matches!(request.pending, Pending::Mulligan { .. })
                .then_some(PlayerAction::PassPriority)
        })
        .rule(|request: &Request| least_answer(&request.context, &request.view, &request.pending));
    let seen = mind.seen();
    let mut table = Table::new(
        2,
        [&allytifact, &victory],
        [Arc::new(mind), house()],
        &config(),
    );
    while table.actions < 5 && table.step().await {}
    let stats = table.cores[0].stats().clone();
    assert_eq!(stats.refused_by_referee, 2, "{stats:?}");
    assert_eq!(stats.retries, 1);
    assert_eq!(stats.fallbacks.refused, 1);
    assert_eq!(
        stats.answered.house, 1,
        "the house kept or mulliganed for it"
    );
    let seen = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(
        seen.iter()
            .take(2)
            .map(|s| (s.question, s.retry))
            .collect::<Vec<_>>(),
        vec![(1, false), (1, true)],
        "one question, asked twice"
    );
    let reasons = reasons
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(reasons.len(), 1);
    assert!(!reasons[0].is_empty());
}
