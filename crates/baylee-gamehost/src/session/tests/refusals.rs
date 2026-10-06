use super::*;

/// **A chair and whoever is answering for it are two different
/// questions, and there are three of them.** Each seat kind is a
/// different combination, which is the whole reason all four exist —
/// reading "does it answer over the wire" as "is it a human" in even one
/// place leaves a driven seat being played by the house AI it was taken
/// from, and reading "is it an AI chair" as "is the house answering"
/// renames a player's chair in the lobby thirty seconds after their
/// laptop shut.
///
/// The four rows are four distinct answers, which is asserted rather
/// than left to be read: two kinds agreeing on all three would be two
/// names for one seat.
#[test]
fn a_seat_is_read_three_ways_and_no_two_kinds_answer_alike() {
    let table = [
        // (kind, answers over a socket, is an AI chair, is away)
        (SeatKind::Human, true, false, false),
        (SeatKind::Ai(agent()), false, true, false),
        (SeatKind::Driven(agent()), true, true, false),
        (SeatKind::StandIn(agent()), false, false, true),
    ];
    let mut answers = Vec::new();
    for (kind, socket, ai_chair, away) in &table {
        assert_eq!(
            (
                kind.answers_over_socket(),
                kind.is_ai_chair(),
                kind.is_away()
            ),
            (*socket, *ai_chair, *away),
            "seat kind {}",
            kind_index(kind)
        );
        answers.push((*socket, *ai_chair, *away));
    }
    assert_eq!(
        table
            .iter()
            .map(|(k, ..)| kind_index(k))
            .collect::<Vec<_>>(),
        (0..4).collect::<Vec<_>>(),
        "the table is every kind exactly once, in declaration order"
    );
    answers.sort_unstable();
    answers.dedup();
    assert_eq!(answers.len(), 4, "two kinds are one seat under two names");
}

#[test]
fn issue_180_a_refused_non_priority_action_recovers() {
    let mut session = Session::new(&test_preset()).unwrap();
    let player = session.awaiting_seat().expect("opening mulligan");
    assert!(matches!(session.engine.pending(), Pending::Mulligan { .. }));
    let before = session.engine.snapshot_hash();
    assert!(session.apply_house_action(
        player,
        PlayerAction::PlayLand {
            card: baylee_core::ids::ObjectId::new(999_999, 0),
        }
    ));
    assert_ne!(session.engine.snapshot_hash(), before);
}

/// A house proposal the engine refuses leaves a record that replays.
///
/// r001's games 1581, 3288 and 3554: the house said yes to a Banishing
/// Stroke miracle with nothing to target, the engine refused it after
/// spending the offer and moving the game on, and the fallback's answer
/// to the question *after* went into the record in its place. The replay
/// stood at the miracle and was handed a pass.
///
/// A "yes" the payment cannot follow is taken now, and the cast reversed,
/// so the proposal here is one no question takes: a mode, handed to
/// Temporal Mastery's miracle offer. The engine turns it down, and the
/// answer that stands is the offer's own fallback, "no".
#[test]
fn a_refused_house_proposal_leaves_a_record_that_replays() {
    let mastery = baylee_cards::by_oracle_id("5c58b8e6-c572-461e-893e-a8c05f20ba17")
        .expect("Temporal Mastery is in the pool")
        .index;
    let mut preset = test_preset();
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Open;
    for entry in &mut preset.seats[1].deck {
        entry.card = mastery;
    }
    let mastery_seat = PlayerId::new(1);
    let mut session = Session::new_recorded(&preset, "test").expect("the table builds");
    session.tell_time(1_000);
    session.pump();
    let mut refusals = 0;
    for step in 0..300_u64 {
        let Some(seat) = session.awaiting_seat() else {
            break;
        };
        session.tell_time(1_000 + step);
        let miracle = matches!(
            session.engine.pending_for(seat),
            Some(Pending::YesNo {
                prompt: baylee_engine::choice::YesNoPrompt::Miracle { .. },
                ..
            })
        );
        if miracle && seat == mastery_seat && refusals < 2 {
            let before = session.engine.snapshot_hash();
            assert!(
                session
                    .engine
                    .apply(seat, PlayerAction::ChooseMode(0))
                    .is_err(),
                "a mode answered a yes-or-no question"
            );
            assert_eq!(session.engine.snapshot_hash(), before);
            session.apply_house_action(seat, PlayerAction::ChooseMode(0));
            session.pump();
            refusals += 1;
            continue;
        }
        let action = session
            .house_action(seat)
            .expect("a question has an answer");
        session
            .act(seat, action)
            .expect("the house's answer stands");
    }
    assert_eq!(refusals, 2, "the game never offered the miracle twice");
    let record = session.take_record();
    let declined = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_slice::<crate::record::Line>(l).ok())
        .filter(|line| {
            matches!(
                line,
                crate::record::Line::Input {
                    seat: 1,
                    action: PlayerAction::YesNo(false),
                    ..
                }
            )
        })
        .count();
    assert!(
        declined >= 2,
        "the refused miracles were not answered with a no"
    );
    let replayed = crate::record::replay(&record).expect("the record replays");
    assert_eq!(replayed.engine.snapshot_hash(), session.snapshot_hash());
}

/// A refused house proposal is followed by the standing question's answer
/// that does nothing, not by the house asked again. A refusal leaves the
/// engine as it was, so the house asked again at its own refused proposal
/// would propose it once more, and the fallback's `expect` would end the
/// game (not shown here: no house proposal is known to be refused).
///
/// The seat here would play a land, and is handed a refused proposal at
/// that priority: the answer that stands is a pass, and the land stays in
/// hand. Asked again, the house played it.
#[test]
fn a_refused_house_proposal_is_followed_by_the_answer_that_does_nothing() {
    let mut preset = test_preset();
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Open;
    let mut session = Session::new_recorded(&preset, "test").expect("the table builds");
    session.tell_time(1_000);
    session.pump();
    for step in 0..300_u64 {
        let Some(seat) = session.awaiting_seat() else {
            break;
        };
        session.tell_time(1_000 + step);
        let action = session
            .house_action(seat)
            .expect("a question has an answer");
        if matches!(action, PlayerAction::PlayLand { .. }) {
            let hand =
                |session: &Session| session.state().zones.list(ZoneLocation::Hand(seat)).len();
            let before = hand(&session);
            session.apply_house_action(seat, PlayerAction::ChooseMode(0));
            assert_eq!(hand(&session), before, "the land stays in hand");
            let record = session.take_record();
            let last = record
                .split(|&b| b == b'\n')
                .filter(|l| !l.is_empty())
                .filter_map(|l| serde_json::from_slice::<crate::record::Line>(l).ok())
                .filter_map(|line| match line {
                    crate::record::Line::Input { action, .. } => Some(action),
                    _ => None,
                })
                .next_back();
            assert_eq!(last, Some(PlayerAction::PassPriority));
            return;
        }
        session
            .act(seat, action)
            .expect("the house's answer stands");
    }
    panic!("the house never played a land");
}
