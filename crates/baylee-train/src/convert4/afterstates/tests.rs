use super::*;

#[test]
fn seeds_are_a_function_of_the_root_and_differ_between_forks() {
    assert_eq!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 300, 0));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 300, 1));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 0, 300, 0));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(8, 1, 300, 0));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 301, 0));
}

#[test]
fn only_a_reached_position_is_encoded() {
    assert!(AfterStop4::Question.reached() && AfterStop4::GameOver.reached());
    for stop in [
        AfterStop4::ForkRefused,
        AfterStop4::AnswerRefused,
        AfterStop4::ScriptMismatch,
        AfterStop4::ScriptBroken,
        AfterStop4::TooLong,
    ] {
        assert!(!stop.reached(), "{}", stop.key());
    }
}

#[test]
fn a_card_leaving_a_library_is_a_hidden_event_and_a_life_change_is_not() {
    let draw = GameEvent::ZoneChanged {
        object: baylee_core::ids::ObjectId::new(3, 0),
        from: Zone::Library,
        to: Zone::Hand,
        cause: baylee_engine::event::Cause::TurnBased,
        place: None,
    };
    assert!(hidden_event(&draw));
    let played = GameEvent::ZoneChanged {
        object: baylee_core::ids::ObjectId::new(3, 0),
        from: Zone::Hand,
        to: Zone::Battlefield,
        cause: baylee_engine::event::Cause::Spell,
        place: None,
    };
    assert!(!hidden_event(&played));
    assert!(hidden_event(&GameEvent::Shuffled {
        player: PlayerId::new(0),
        zone: Zone::Library,
    }));
}

/// A house self-play record and, for each of seat 0's inputs, the
/// candidates asked for: its recorded answer, a pass, a land that is no
/// card of the game, and a planned "cast" that is no cast.
fn game_and_roots() -> (Vec<u8>, super::super::search_roots::Roots4) {
    use crate::housedeck::HouseDeck;
    use crate::selfplay::{Caps, play_with_options, table};
    let a = HouseDeck::named("allytifact").unwrap();
    let b = HouseDeck::named("victory").unwrap();
    let preset = table(23, &a, &b, [baylee_core::preset::AIProfile::SHARP; 2]);
    let played = play_with_options(
        &preset,
        "t-afterstates-23",
        Caps {
            answers: 10_000,
            wall: std::time::Duration::from_secs(60),
        },
        true,
    );
    assert!(played.outcome.finished(), "{:?}", played.outcome);
    let mut afterstates = std::collections::BTreeMap::new();
    for line in played
        .record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
    {
        if let Ok(baylee_gamehost::record::Line::Input {
            seat: 0, n, action, ..
        }) = serde_json::from_slice(line)
        {
            let plain = |action: PlayerAction| AfterCandidate4 {
                action,
                script: None,
            };
            afterstates.insert(
                (0, n),
                vec![
                    plain(action),
                    plain(PlayerAction::PassPriority),
                    plain(PlayerAction::PlayLand {
                        card: baylee_core::ids::ObjectId::new(9_999, 0),
                    }),
                    AfterCandidate4 {
                        action: PlayerAction::PassPriority,
                        script: Some(PlayerAction::PassPriority),
                    },
                ],
            );
        }
    }
    let roots = super::super::search_roots::Roots4 {
        roots: afterstates.keys().copied().collect(),
        menu: |_, _, own, _| (own.clone(), own.candidates.len()),
        afterstates,
    };
    (played.record, roots)
}

/// Every root the replay captured, with its frame: (root frame,
/// afterstates).
fn replay(
    record: &[u8],
    roots: &super::super::search_roots::Roots4,
) -> Vec<(Step4, RootAfterstates4)> {
    let mut out = Vec::new();
    super::super::stream_game4_asking(
        record,
        5,
        true,
        None,
        None,
        None,
        false,
        10,
        None,
        super::super::TeamHands::Private,
        super::super::Asks4 {
            roots: Some(roots),
            ..super::super::Asks4::default()
        },
        |mut step| {
            if let Some(root) = step.search_root.take() {
                let captured = root.afterstates.clone().expect("asked for afterstates");
                out.push((step, captured));
            }
            Ok(())
        },
    )
    .unwrap();
    out
}

use super::super::Step4;

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn afterstates_are_a_function_of_the_record_and_stop_at_the_next_question() {
    let (record, roots) = game_and_roots();
    let first = replay(&record, &roots);
    let again = replay(&record, &roots);
    assert!(first.len() > 10, "a game has priority roots of seat 0");
    assert_eq!(first.len(), again.len());
    let (mut passes_to_other, mut own_answers, mut unforked) = (0, 0, 0);
    for ((step, a), (_, b)) in first.iter().zip(&again) {
        assert_eq!(a.candidates.len(), 4);
        for (x, y) in a.candidates.iter().zip(&b.candidates) {
            assert_eq!(
                (x.stop, x.answers, x.asked_seat, x.next_kind),
                (y.stop, y.answers, y.asked_seat, y.next_kind)
            );
            for (p, q) in x.frames.iter().zip(&y.frames) {
                assert_eq!(
                    p.as_ref().map(|f| json(&f.state)),
                    q.as_ref().map(|f| json(&f.state))
                );
            }
        }
        // Mid-chain (mana floating, a cast under way) the engine will not
        // fork: nothing is played or encoded there.
        let Some((state, cands)) = a.root.as_ref() else {
            assert!(
                a.candidates
                    .iter()
                    .all(|c| c.stop == AfterStop4::ForkRefused
                        && c.frames.iter().all(Option::is_none))
            );
            unforked += 1;
            continue;
        };
        // The fork's frame of the root is the replay's own.
        assert_eq!(
            json(state),
            json(&step.state),
            "root at {}",
            step.step_cursor
        );
        assert_eq!(cands.candidates.len(), step.cand_feat.len());
        // The recorded answer is the table's: it reaches the next question
        // after one answer, as do both forks.
        let own = &a.candidates[0];
        assert!(own.stop.reached(), "{:?} at {}", own.stop, step.step_cursor);
        assert_eq!(own.answers, 1);
        assert!(own.frames.iter().all(Option::is_some));
        own_answers += 1;
        // A pass with priority moves the question on; where it goes to the
        // other seat, the seat is not asked.
        let pass = &a.candidates[1];
        if pass.stop.reached() && !pass.asked_seat {
            passes_to_other += 1;
        }
        // No such land: the fork refuses it and nothing is encoded.
        let land = &a.candidates[2];
        assert_eq!(land.stop, AfterStop4::AnswerRefused);
        assert!(land.frames.iter().all(Option::is_none));
        // A script that is no planned cast is never played.
        assert_eq!(a.candidates[3].stop, AfterStop4::ScriptMismatch);
        assert_eq!(a.candidates[3].answers, 0);
    }
    assert!(
        own_answers > 10 && passes_to_other > 0,
        "{own_answers} {passes_to_other} {unforked}"
    );
}
