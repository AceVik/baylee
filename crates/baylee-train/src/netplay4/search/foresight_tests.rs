//! The fair search (v5) on real tables with the stand-in for the net: fair
//! worlds, turn horizons and macro candidates. No model is loaded.
use baylee_cards::decks::{LoadedDeck, by_name, house_deck, preset_for, probe_deck};
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::timeout_answer;

use super::tests::{ME, Mock, SEED, config, creatures, passes, steady};
use super::*;

const THEM: PlayerId = PlayerId::new(1);
/// How far each deal is played.
const LAST_TURN: u32 = 7;

/// Creatures to cast and burn to point: taps, casts and targets.
fn goblins_and_bolts() -> LoadedDeck {
    house_deck("[deck:Goblins]\n22 Mountain\n22 Goblin Piker\n16 Lightning Bolt\n")
        .expect("the pool has all three")
}

fn opponent(card: &str) -> LoadedDeck {
    let index = by_name(card).unwrap_or_else(|| panic!("the pool has {card}"));
    probe_deck(index, 20, 60).expect("basics to pad with")
}

/// I start with my deck against `theirs`; they hold no cards at the start
/// and only ever pass, so nothing of their list shows.
fn deal(seed: u64, theirs: &LoadedDeck) -> GamePreset {
    let mut preset = preset_for(seed, &goblins_and_bolts(), theirs);
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].controller = SeatController::Ai(AIProfile::STEADY);
    preset.seats[1].controller = SeatController::Open;
    preset.seats[1].starting_hand = Some(Vec::new());
    preset
}

/// The fair search with macro candidates and a horizon of two turns, the
/// stand-in answering every chair of a world.
fn foresight(k: usize, p: usize) -> SearchConfig4 {
    config(k, p, 64)
        .with_fair(true)
        .with_macros(true)
        .with_neural_opponent(true)
        .with_turns(2)
        .unwrap()
        .with_selection(Selection4::Mean)
        .unwrap()
}

/// What one searched game showed: every answer of mine with how it came
/// about, and the reports of the searched ones.
#[derive(Debug, Default, PartialEq)]
struct Played {
    answers: Vec<(PlayerAction, String)>,
    reports: Vec<SearchReport4>,
    scripts_done: u64,
}

/// Plays `preset` to [`LAST_TURN`]: I answer through the fair search (a
/// chosen planned cast followed at the table), they pass.
fn play(preset: &GamePreset, mut player: Mock, config: SearchConfig4) -> Played {
    let mut session = Session::new_recorded(preset, "test").unwrap();
    session.describe("foresight".into(), vec!["0".into(), "1".into()]);
    assert!(session.take_over(ME));
    let mut played = Played::default();
    let mut script: Option<Script4> = None;
    for _ in 0..20_000 {
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let Some((pending, view)) = session.view_for_with_context(ME) else {
            let (pending, _) = session.view_for(THEM).expect("somebody is asked");
            let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
            session.act_slice(THEM, nothing, 0).expect("they may pass");
            continue;
        };
        player.observe(&view).unwrap();
        let hash = session.snapshot_hash();
        let scripted = script.take().and_then(|mut standing| {
            match follow_script(&mut player, &view, &pending, &[], &mut standing) {
                ScriptFollow4::Answered { answer, ends } => {
                    if ends {
                        played.scripts_done += 1;
                    } else {
                        script = Some(standing);
                    }
                    Some(answer)
                }
                ScriptFollow4::Over => None,
                ScriptFollow4::Other => {
                    script = Some(standing);
                    None
                }
            }
        });
        let (answer, how) = if let Some(answer) = scripted {
            (answer, String::from("scripted"))
        } else {
            let result = search_decision(
                &mut player,
                &PriorDecks::new(&session, &view),
                &view,
                &pending,
                &[],
                config,
                SEED,
            )
            .unwrap();
            script = result.script;
            let how = match result.outcome {
                SearchOutcome4::Searched(report) => {
                    let how = format!("searched {}", report.chosen);
                    played.reports.push(*report);
                    how
                }
                SearchOutcome4::Skipped(reason) => reason.key().to_owned(),
                SearchOutcome4::Refused(refusal) => format!("refused {refusal:?}"),
            };
            (result.answer, how)
        };
        assert_eq!(
            session.snapshot_hash(),
            hash,
            "a search never moves the table"
        );
        played.answers.push((answer.action.clone(), how));
        session
            .act_slice(ME, answer.action, 0)
            .expect("every answer is legal at the table");
    }
    played
}

/// Fair by construction: two deals told apart only by what I may not see
/// (their decklist, and so their library and every card they draw) are
/// searched alike, answer for answer and report for report. That holds the
/// whole chain: the candidates are read off my view and the offer, the
/// worlds are dealt from the public prior and never from their list, and
/// the stand-in in their chair of a world sees that world alone.
#[test]
fn a_fair_search_answers_the_same_whatever_is_hidden() {
    for seed in [11, 12] {
        let bears = play(
            &deal(seed, &opponent("Grizzly Bears")),
            Mock::new(steady, creatures),
            foresight(6, 2),
        );
        let walls = play(
            &deal(seed, &opponent("Wall of Wood")),
            Mock::new(steady, creatures),
            foresight(6, 2),
        );
        assert!(bears.reports.len() > 5, "searches ran: {:?}", bears.answers);
        assert!(
            bears.reports.iter().any(|r| r.rollout_steps > 0),
            "the worlds were played in"
        );
        assert_eq!(bears, walls, "seed {seed}");
    }
}

/// The same deals under known decks are told apart: the test above is a
/// test, and the lists do differ in what a world deals.
#[test]
fn known_decks_tell_the_twin_deals_apart() {
    let deals = [opponent("Grizzly Bears"), opponent("Wall of Wood")].map(|theirs| {
        let preset = deal(11, &theirs);
        let mut session = Session::new_recorded(&preset, "test").unwrap();
        assert!(session.take_over(ME));
        // To my first priority of the game: opening hands are kept there.
        let view = loop {
            if session.needs_pump() {
                session.pump_at_most(1);
                continue;
            }
            let Some((pending, view)) = session.view_for_with_context(ME) else {
                let (pending, _) = session.view_for(THEM).expect("somebody is asked");
                let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
                session.act_slice(THEM, nothing, 0).expect("they may pass");
                continue;
            };
            if matches!(pending, Pending::Priority { .. }) {
                break view;
            }
            let action = session.house_action(ME).unwrap();
            session.act_slice(ME, action, 0).unwrap();
        };
        let known = KnownDecks(&session).fork(ME, 7).unwrap().snapshot_hash();
        let fair = PriorDecks::new(&session, &view)
            .fork(ME, 7)
            .unwrap()
            .snapshot_hash();
        (known, fair)
    });
    assert_ne!(deals[0].0, deals[1].0, "the real lists deal other worlds");
    assert_eq!(deals[0].1, deals[1].1, "the prior deals the same world");
}

/// A policy that only passes, a critic that likes my permanents: the search
/// plays my lands and finds the creature in my hand through the planner (its taps and the cast
/// are one candidate, named by the cast), plays the first tap, and the rest
/// of the plan is followed at the table until the spell is cast.
#[test]
fn a_planned_cast_is_one_candidate_and_is_followed_at_the_table() {
    let played = play(
        &deal(11, &opponent("Grizzly Bears")),
        Mock::new(passes, permanents),
        foresight(6, 2),
    );
    let planned: Vec<&SearchReport4> = played
        .reports
        .iter()
        .filter(|report| report.scripts.iter().any(Option::is_some))
        .collect();
    assert!(!planned.is_empty(), "{:?}", played.answers);
    for report in &planned {
        assert_eq!(report.actions[0], PlayerAction::PassPriority);
        assert_eq!(report.scripts[0], None, "the policy's answer is its own");
        for (action, script) in report.actions.iter().zip(&report.scripts) {
            if let Some(then) = script {
                assert!(is_mana_action_kind(action), "a plan starts at a tap");
                assert!(matches!(then, PlayerAction::CastSpell { .. }));
            }
        }
    }
    assert!(
        planned
            .iter()
            .any(|report| report.scripts[report.chosen].is_some()),
        "a planned cast was chosen: {:?}",
        played.answers
    );
    assert!(played.scripts_done > 0, "{:?}", played.answers);
    assert!(
        played.answers.iter().any(
            |(action, how)| matches!(action, PlayerAction::CastSpell { .. }) && how == "scripted"
        ),
        "the cast came from the plan: {:?}",
        played.answers
    );
}

/// A critic that likes what I have on the battlefield, lands and creatures
/// alike: playing a land and casting a creature both score.
fn permanents(view: &PlayerView) -> f32 {
    let mine = view
        .battlefield
        .iter()
        .filter(|object| object.controller == view.seat)
        .count();
    (0.1 + 0.05 * f32::from(u16::try_from(mine).unwrap_or(u16::MAX))).min(1.0)
}

fn is_mana_action_kind(action: &PlayerAction) -> bool {
    matches!(
        action,
        PlayerAction::ActivateManaAbility { .. } | PlayerAction::ActivateAbility { .. }
    )
}

/// With a turn horizon no rollout stops at my settled pass or at the end of
/// my next turn: each is played to the turn boundary or the game's end, the
/// other chair's whole turn included.
#[test]
fn a_turn_horizon_plays_through_the_other_players_turn() {
    let played = play(
        &deal(12, &opponent("Grizzly Bears")),
        Mock::new(steady, creatures),
        foresight(4, 2),
    );
    let mut leaves = LeafCounts4::default();
    let mut rollouts = 0;
    for report in &played.reports {
        leaves.add(&report.leaves);
        rollouts += report.rollouts;
    }
    assert!(leaves.boundary > 0, "{leaves:?}");
    assert_eq!(leaves.settled + leaves.turn_end + leaves.immediate, 0);
    assert_eq!(
        leaves.boundary + leaves.terminal + leaves.horizon + leaves.stopped,
        rollouts
    );
    assert!(
        leaves.boundary * 10 >= rollouts * 9,
        "nearly every world reaches its boundary: {leaves:?} of {rollouts}"
    );
}

/// The roots of a search with macro candidates: never in the middle of a
/// chain, and a skipped priority says why.
#[test]
fn macro_roots_start_a_chain() {
    let played = play(
        &deal(12, &opponent("Grizzly Bears")),
        Mock::new(steady, creatures),
        foresight(4, 1),
    );
    let hows: Vec<&str> = played.answers.iter().map(|(_, how)| how.as_str()).collect();
    assert!(hows.contains(&"mid_chain"), "{hows:?}");
    assert!(
        hows.iter().any(|how| how.starts_with("searched")),
        "{hows:?}"
    );
    for report in &played.reports {
        assert!(report.candidates >= 2);
        assert!(report.candidates <= 4);
    }
}

#[test]
fn a_fair_search_names_its_contract() {
    let plain = config(4, 2, 6);
    assert_eq!(plain.contract(), SEARCH_CONTRACT);
    for config in [
        plain.with_fair(true),
        plain.with_macros(true),
        plain.with_turns(2).unwrap(),
    ] {
        assert_eq!(config.contract(), SEARCH_CONTRACT_FORESIGHT);
    }
    let metadata = foresight(6, 2).metadata();
    assert_eq!(metadata["turns"], 2);
    assert!(
        metadata["decks"]
            .as_str()
            .unwrap()
            .starts_with("public_prior")
    );
    assert!(plain.with_turns(SearchConfig4::MAX_TURNS + 1).is_err());
}
