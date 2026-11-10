//! `--memory-reset` on a real game against what it promises: a decision reset
//! answers as a player with no memory shown only its view, and a turn reset
//! forgets exactly at turn changes. They need an exported model:
//! `BAYLEE_TEST_MODEL=DIR BAYLEE_EP=cpu`, run with `--ignored`.
use super::*;
use baylee_core::preset::AIProfile;

const ME: PlayerId = PlayerId::new(0);
const TEAMS: [Option<u8>; 2] = [None, None];

/// The model a test runs, when the environment names one: greedy, raw, K=2.
fn model() -> Option<NetPlayer4> {
    let dir = std::env::var("BAYLEE_TEST_MODEL").ok()?;
    Some(
        NetPlayer4::load_with(Path::new(&dir), CatalogPolicy4::Drift)
            .unwrap()
            .with_thinking_steps(2)
            .with_policy_mode(PolicyMode4::Raw),
    )
}

enum Event {
    /// Seat 0's view before any seat acts, as `eval4` observes every net seat.
    Observe(PlayerView),
    /// A question to seat 0.
    Decide(PlayerView, Box<Pending>),
}

/// Seat 0's observations and questions over the first `limit` of its
/// questions of a game the house plays for both seats.
fn game(limit: usize) -> (GamePreset, Vec<Event>) {
    let victory = crate::housedeck::HouseDeck::named("victory").unwrap();
    let allytifact = crate::housedeck::HouseDeck::named("allytifact").unwrap();
    let preset =
        crate::selfplay::table_for(97, &[&victory, &allytifact], &[AIProfile::SHARP; 2], &[]);
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    for seat in 0..2 {
        session.take_over(PlayerId::new(seat));
    }
    let mut events = Vec::new();
    let mut asked_me = 0;
    while asked_me < limit {
        let Some(asked) = session.pending().asked() else {
            break;
        };
        events.push(Event::Observe(session.observe_view_with_context(ME)));
        if asked == ME {
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            events.push(Event::Decide(view, Box::new(pending)));
            asked_me += 1;
        }
        let action = session.house_action(asked).unwrap();
        if session.act(asked, action).is_err() {
            break;
        }
    }
    (preset, events)
}

/// A player of the same explicit state as `player` before its latest
/// observation (`ledger`), but no memory: shown only `view`.
fn fresh_from(
    fresh: &mut NetPlayer4,
    preset: &GamePreset,
    player: &NetPlayer4,
    ledger: &SeatKnowledge4,
    view: &PlayerView,
) {
    fresh.init_game(preset);
    fresh.knowledge.insert(ME.get(), ledger.clone());
    fresh.seen_hashes.clone_from(&player.seen_hashes);
    fresh
        .last_announced_source
        .clone_from(&player.last_announced_source);
    fresh.observe(view).unwrap();
}

#[test]
#[ignore = "needs BAYLEE_TEST_MODEL, an exported v4/v5 model directory"]
fn a_decision_reset_answers_as_a_player_with_no_memory_shown_only_its_view() {
    let (Some(mut forgetful), Some(mut normal), Some(mut fresh)) = (model(), model(), model())
    else {
        return;
    };
    forgetful.set_memory_reset(MemoryReset4::Decision);
    forgetful.keep_menus();
    let (preset, events) = game(30);
    forgetful.init_game(&preset);
    normal.init_game(&preset);
    // The ledger before the latest observation that advanced it, and that view.
    let mut latest: Option<(SeatKnowledge4, PlayerView)> = None;
    let (mut checked, mut memory_differs) = (0, 0);
    for event in &events {
        match event {
            Event::Observe(view) => {
                let ledger = forgetful.knowledge[&ME.get()].clone();
                forgetful.observe(view).unwrap();
                normal.observe(view).unwrap();
                if forgetful.knowledge[&ME.get()].cursor != ledger.cursor {
                    latest = Some((ledger, view.clone()));
                }
            }
            Event::Decide(view, pending) => {
                let (ledger, seen) = latest.as_ref().unwrap();
                fresh_from(&mut fresh, &preset, &forgetful, ledger, seen);
                assert_eq!(fresh.h_states[&ME.get()], forgetful.h_states[&ME.get()]);
                let expected = fresh.act_detailed(view, pending, &TEAMS);
                let answer = forgetful.act_detailed(view, pending, &TEAMS);
                assert_eq!(answer.action, expected.action);
                assert_eq!(answer.value.to_bits(), expected.value.to_bits());
                assert_eq!(answer.provenance, expected.provenance);
                // A menu with logits for each neural answer, taken once.
                let menu = forgetful.take_menu();
                assert_eq!(menu.is_some(), answer.provenance.source == SourceKind4::Net);
                assert!(forgetful.take_menu().is_none());
                memory_differs +=
                    usize::from(normal.h_states[&ME.get()] != forgetful.h_states[&ME.get()]);
                normal.act_detailed(view, pending, &TEAMS);
                checked += 1;
            }
        }
    }
    assert!(checked >= 20, "{checked} decisions");
    assert!(memory_differs > 0, "the game's memory mattered somewhere");
}

#[test]
#[ignore = "needs BAYLEE_TEST_MODEL, an exported v4/v5 model directory"]
fn a_turn_reset_forgets_exactly_at_turn_changes() {
    let (Some(mut turnwise), Some(mut fresh)) = (model(), model()) else {
        return;
    };
    turnwise.set_memory_reset(MemoryReset4::Turn);
    let (preset, events) = game(60);
    turnwise.init_game(&preset);
    let mut last_turn = None;
    let (mut resets, mut kept) = (0, 0);
    for event in &events {
        match event {
            Event::Observe(view) => {
                let ledger = turnwise.knowledge[&ME.get()].clone();
                turnwise.observe(view).unwrap();
                if turnwise.knowledge[&ME.get()].cursor == ledger.cursor {
                    continue;
                }
                fresh_from(&mut fresh, &preset, &turnwise, &ledger, view);
                let forgot = fresh.h_states[&ME.get()] == turnwise.h_states[&ME.get()];
                if last_turn == Some(view.turn) {
                    assert!(!forgot, "turn {}: H kept within a turn", view.turn);
                    kept += 1;
                } else {
                    assert!(forgot, "turn {}: H forgotten at its first view", view.turn);
                    resets += 1;
                }
                last_turn = Some(view.turn);
            }
            Event::Decide(view, pending) => {
                turnwise.act_detailed(view, pending, &TEAMS);
            }
        }
    }
    assert!(resets >= 3 && kept > 0, "{resets} resets, {kept} kept");
}
