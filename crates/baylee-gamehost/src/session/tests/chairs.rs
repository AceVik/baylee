use super::*;

/// A team wins as a team, the dead included (CR 104.2c), and the roster
/// is where that is turned back into seats: the engine names a
/// `Victor::Team` and a client's roster names chairs.
///
/// The fourth seat is on no team at all, which is the row that says the
/// answer is read off each seat rather than off the winner: a seat with
/// `None` is on no winning team however the game ended.
#[test]
fn a_team_win_names_every_chair_on_the_team() {
    use baylee_engine::win::{EndReason, GameResult, Victor};
    let session =
        Session::new(&teamed_preset([Some(1), Some(2), Some(1), None])).expect("a four-seat game");
    let seats = |winner| {
        session.winning_seats(GameResult {
            winner,
            reason: EndReason::LastTeamStanding,
        })
    };

    assert_eq!(
        seats(Some(Victor::Team(1))),
        vec![PlayerId::new(0), PlayerId::new(2)],
        "both chairs on the team, in seat order"
    );
    assert_eq!(seats(Some(Victor::Team(2))), vec![PlayerId::new(1)]);
    assert!(
        seats(Some(Victor::Team(3))).is_empty(),
        "a team nobody at this table plays for"
    );
    assert_eq!(
        seats(Some(Victor::Player(PlayerId::new(3)))),
        vec![PlayerId::new(3)],
        "a seat that won as itself, and it is on no team"
    );
    assert!(seats(None).is_empty(), "a draw has no winning seat to name");
}

/// The whole point of a driven seat: the question stops going to the
/// house AI and starts going out over the wire.
///
/// Seat 1 is an AI chair. Left alone it answers its own mulligan inside
/// `pump`, and the game never stops for it — `pump` only returns when a
/// seat that answers over a socket is on the clock. Taken over, the same
/// question comes back addressed to seat 1, which is what somebody at the
/// other end of the harness answers.
#[test]
fn a_driven_seat_is_asked_instead_of_answering_itself() {
    let human = PlayerId::new(0);
    let ai = PlayerId::new(1);

    // Left alone, nothing ever stops for seat 1.
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    let _ = session.pump();
    let _ = session.act(human, PlayerAction::MulliganKeep);
    assert_ne!(
        session.awaiting_seat(),
        Some(ai),
        "the house AI answers for itself"
    );

    // Taken over, the same question is addressed to it.
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    assert!(session.take_over(ai), "seat 1 is an AI chair");
    let _ = session.pump();
    let _ = session.act(human, PlayerAction::MulliganKeep);
    assert_eq!(
        session.awaiting_seat(),
        Some(ai),
        "a driven seat is asked over the wire"
    );
}

/// Releasing hands the chair back mid-game, so a developer who
/// disconnects leaves a playable opponent rather than a table stopped at
/// a question nobody is there to answer.
#[test]
fn releasing_a_seat_gives_it_back_to_the_house_ai() {
    let human = PlayerId::new(0);
    let ai = PlayerId::new(1);
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    assert!(session.take_over(ai));
    let _ = session.pump();
    let _ = session.act(human, PlayerAction::MulliganKeep);
    assert_eq!(session.awaiting_seat(), Some(ai), "stopped for the driver");

    assert!(session.release(ai), "it was being driven");
    // The question it was stopped on is now answered by the agent it was
    // taken from, so the table moves again without the driver.
    let _ = session.pump();
    assert_ne!(
        session.awaiting_seat(),
        Some(ai),
        "the house AI has the chair back"
    );
}

/// Both refusals, because a caller that believed it was driving a chair
/// it was not would sit waiting for a question the house AI has already
/// answered.
#[test]
fn only_an_ai_chair_can_be_taken_over_and_only_once() {
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    assert!(
        !session.take_over(PlayerId::new(0)),
        "seat 0 is a human seat"
    );
    assert!(
        !session.take_over(PlayerId::new(7)),
        "there is no seat 7 to take"
    );
    assert!(!session.release(PlayerId::new(1)), "it is not driven yet");
    assert!(session.take_over(PlayerId::new(1)));
    assert!(
        !session.take_over(PlayerId::new(1)),
        "and it cannot be taken twice"
    );
}

/// A chair does not change what it *is* when somebody takes the
/// controls, so the roster a client draws stays put.
#[test]
fn a_driven_chair_is_still_an_ai_chair_on_the_roster() {
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    session.describe("g".to_string(), vec!["You".into(), "House AI".into()]);
    let before = session.game_static(PlayerId::new(0));
    assert!(session.take_over(PlayerId::new(1)));
    let after = session.game_static(PlayerId::new(0));
    assert_eq!(
        before.seats.iter().map(|s| s.is_ai).collect::<Vec<_>>(),
        after.seats.iter().map(|s| s.is_ai).collect::<Vec<_>>(),
        "the roster says what the chair is, not who is holding it"
    );
}

/// The bug this whole mechanism exists for: seat 0 walks away holding the
/// decision, and the table waits on it forever.
///
/// A seat with no socket is on no decision clock — deliberately, because
/// nobody should lose on time to a question they never saw — so nothing
/// else was ever going to move this game again.
#[test]
fn a_chair_the_house_stands_in_for_stops_holding_up_the_table() {
    let gone = PlayerId::new(0);
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    let _ = session.pump();
    assert_eq!(session.awaiting_seat(), Some(gone), "seat 0 owes an answer");
    let stuck = session.decision_seq();
    assert_eq!(
        session
            .pump()
            .iter()
            .filter(|(_, e)| matches!(e.msg, Some(v1::envelope::Msg::ChoiceRequest(_))))
            .count(),
        1,
        "pumping again just asks seat 0 the same question"
    );
    assert_eq!(session.decision_seq(), stuck, "and the game has not moved");

    assert!(session.stand_in(gone), "seat 0 is a player's chair");
    let _ = session.pump();
    assert!(
        session.decision_seq() > stuck,
        "the house answered and the table moved on"
    );
}

/// Both refusals in one place, because `stand_in` is reached from a
/// deadline rather than from a request: a caller that stood in for the
/// wrong chair would have no one to tell.
#[test]
fn only_a_players_chair_can_be_stood_in_for() {
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    assert!(
        !session.stand_in(PlayerId::new(1)),
        "seat 1 is an AI chair; it is not waiting for anyone"
    );
    assert!(
        !session.stand_in(PlayerId::new(7)),
        "there is no seat 7 to stand in for"
    );
    assert!(
        !session.hand_back(PlayerId::new(0)),
        "seat 0 is already the player's"
    );
    assert!(session.take_over(PlayerId::new(1)), "now it is driven");
    assert!(
        !session.stand_in(PlayerId::new(1)),
        "a driven chair hands itself back when its socket drops"
    );
    assert!(session.stand_in(PlayerId::new(0)));
    assert!(
        !session.stand_in(PlayerId::new(0)),
        "and the house cannot sit down twice"
    );
}

/// The chair was only ever borrowed.
#[test]
fn a_player_who_comes_back_gets_their_chair_back() {
    let gone = PlayerId::new(0);
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    let _ = session.pump();
    assert!(session.stand_in(gone));
    assert!(session.hand_back(gone), "the house was holding it");
    let _ = session.pump();
    assert_eq!(
        session.awaiting_seat(),
        Some(gone),
        "the question is theirs again"
    );
}

/// A held chair still belongs to the player who left it.
///
/// Two fields rather than one, because a seat that renamed itself to the
/// house AI after a thirty-second hiccup would be telling the table
/// something untrue — and would go on saying it after the player was
/// back, since a roster is sent once.
#[test]
fn a_held_chair_says_away_rather_than_calling_itself_an_ai() {
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    session.describe("g".to_string(), vec!["You".into(), "House AI".into()]);
    assert!(session.stand_in(PlayerId::new(0)));
    let seen = session.game_static(PlayerId::new(1));
    assert!(seen.seats[0].away, "seat 0 is being stood in for");
    assert!(!seen.seats[0].is_ai, "but it is still a player's chair");
    assert!(session.hand_back(PlayerId::new(0)));
    assert!(
        !session.game_static(PlayerId::new(1)).seats[0].away,
        "and it stops saying so"
    );
}

/// Whether a `GameStatic` was addressed to a seat in one pump.
fn told_the_roster(out: &[(PlayerId, Envelope)], seat: PlayerId) -> bool {
    out.iter()
        .any(|(p, e)| *p == seat && matches!(e.msg, Some(v1::envelope::Msg::GameStatic(_))))
}

/// The roster travels in `GameStatic`, which is sent once when a socket
/// attaches — so a chair could change hands and the table was simply
/// never told. That had been true of `take_over`/`release` since they
/// existed; standing in is what made it visible.
#[test]
fn a_seat_learns_the_roster_changed_when_a_chair_changes_hands() {
    let watcher = PlayerId::new(0);
    let mut session = Session::new(&test_preset()).expect("the preset builds");
    session.describe("g".to_string(), vec!["You".into(), "House AI".into()]);
    let _ = session.pump();
    assert!(
        !told_the_roster(&session.pump(), watcher),
        "nothing changed, so nothing is re-sent"
    );
    assert!(session.take_over(PlayerId::new(1)));
    assert!(
        told_the_roster(&session.pump(), watcher),
        "the chair changed hands and the table is told"
    );
    assert!(
        !told_the_roster(&session.pump(), watcher),
        "told once, not on every view after"
    );
}
