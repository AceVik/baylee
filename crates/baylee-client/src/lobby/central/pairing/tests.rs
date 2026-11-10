//! The pairing's worker against a scripted Central, and what the lobby
//! makes of its news. The SDK's own steps against a real Central are
//! `baylee-gateway`'s `e2e_central::pairing` (`cargo dev central
//! integration`).
use super::*;
use crate::lobby::central::{Handle, Restored};
use baylee_client_core::prefs::Preferences;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// What a scripted Central answers, step by step, and what was asked of it.
struct Script {
    claim: Result<String, Error>,
    polls: VecDeque<Result<bool, Error>>,
    /// Answered once the polls run out.
    then: Result<bool, Error>,
    redeem: Result<String, Error>,
    accept: Result<(), Error>,
    asked: Arc<Mutex<Vec<&'static str>>>,
}

impl Script {
    fn new() -> Self {
        Self {
            claim: Ok("4KQ-7MX".into()),
            polls: VecDeque::new(),
            then: Ok(true),
            redeem: Ok("Pair_Player#002a (pairing_player)".into()),
            accept: Ok(()),
            asked: Arc::default(),
        }
    }

    fn note(&self, step: &'static str) {
        self.asked.lock().unwrap().push(step);
    }
}

impl Steps for Script {
    fn claim(&mut self) -> Result<String, Error> {
        self.note("claim");
        self.claim.clone()
    }

    fn approved(&mut self) -> Result<bool, Error> {
        self.note("approved");
        self.polls.pop_front().unwrap_or(self.then)
    }

    fn redeem(&mut self) -> Result<String, Error> {
        self.note("redeem");
        self.redeem.clone()
    }

    fn accept(&mut self) -> (Result<Box<Restored>, Error>, String) {
        self.note("accept");
        let restored = self.accept.map(|()| Box::new(restored()));
        (restored, "pairing_player".into())
    }

    fn decline(&mut self) {
        self.note("decline");
    }
}

/// An account as a restore leaves it, its worker a channel nobody reads.
/// No gateways: a restore asks each one about itself.
fn restored() -> Restored {
    let (send, _) = mpsc::sync_channel(1);
    Restored {
        handle: Handle::from_sender(send),
        gateways: Vec::new(),
        preferences: Preferences::default(),
        terms_pending: false,
        profile_handle: Some("Pair_Player#002a".into()),
    }
}

/// The news, by name.
fn named(news: &News) -> String {
    match news {
        News::Waiting(code) => format!("waiting {code}"),
        News::Asking(who) => format!("asking {who}"),
        News::Ended(ending) => format!("ended {ending:?}"),
        News::SignedIn(Ok(_), username) => format!("signed in {username}"),
        News::SignedIn(Err(error), _) => format!("not signed in: {error}"),
    }
}

/// How the player answers: when the worker says it waits, and when it asks.
#[derive(Clone, Copy)]
enum Player {
    /// Answers the question with this.
    Answers(Answer),
    /// Cancels while the page is asked.
    CancelsWhileWaiting,
    /// Never answers.
    Silent,
}

/// Runs a pairing to its end, quickly: what the worker said, and what it
/// asked of Central.
fn pair(script: Script, player: Player) -> (Vec<String>, Vec<&'static str>) {
    let asked = script.asked.clone();
    let (answers, heard) = mpsc::sync_channel(1);
    let said = Mutex::new(Vec::new());
    let tell = |news: News| {
        match (&news, player) {
            (News::Waiting(_), Player::CancelsWhileWaiting) => {
                answers.try_send(Answer::No).unwrap();
            }
            (News::Asking(_), Player::Answers(answer)) => {
                answers.try_send(answer).unwrap();
            }
            _ => {}
        }
        said.lock().unwrap().push(named(&news));
    };
    run(
        script,
        &heard,
        &tell,
        Duration::from_millis(60),
        Duration::from_millis(1),
    );
    let said = said.into_inner().unwrap();
    let asked = asked.lock().unwrap().clone();
    (said, asked)
}

#[test]
fn an_approved_pairing_asks_the_player_and_signs_in_on_yes() {
    let mut script = Script::new();
    script.polls = [Ok(false), Ok(false)].into();
    let (said, asked) = pair(script, Player::Answers(Answer::Yes));
    assert_eq!(
        said,
        [
            "waiting 4KQ-7MX",
            "asking Pair_Player#002a (pairing_player)",
            "signed in pairing_player",
        ]
    );
    assert_eq!(
        asked,
        [
            "claim", "approved", "approved", "approved", "redeem", "accept"
        ]
    );
}

#[test]
fn no_to_the_account_signs_the_session_out() {
    let (said, asked) = pair(Script::new(), Player::Answers(Answer::No));
    assert_eq!(said.last().unwrap(), "ended Cancelled");
    assert_eq!(asked.last().unwrap(), &"decline");
    assert!(!asked.contains(&"accept"));
}

#[test]
fn a_question_nobody_answers_runs_out_and_signs_the_session_out() {
    let (said, asked) = pair(Script::new(), Player::Silent);
    assert_eq!(said.last().unwrap(), "ended Expired");
    assert_eq!(asked.last().unwrap(), &"decline");
}

#[test]
fn a_cancel_while_the_page_is_asked_lets_the_claim_go() {
    let mut script = Script::new();
    script.then = Ok(false);
    let (said, asked) = pair(script, Player::CancelsWhileWaiting);
    assert_eq!(said, ["waiting 4KQ-7MX", "ended Cancelled"]);
    assert_eq!(
        asked,
        ["claim", "decline"],
        "not asked again after the cancel"
    );
}

/// The lobby let the pairing go (`‹`, a sign-out, the form closed): the
/// worker hears the dropped channel as a no, at every stage.
#[test]
fn a_pairing_the_lobby_let_go_ends_and_lets_its_session_go() {
    for asking in [false, true] {
        let mut script = Script::new();
        script.then = Ok(asking);
        let asked = script.asked.clone();
        let (answers, heard) = mpsc::sync_channel(1);
        let said = Mutex::new(Vec::new());
        // `tell` is `Fn`: the channel is let go through a cell.
        let answers = std::cell::RefCell::new(Some(answers));
        let tell = |news: News| {
            let news = named(&news);
            if news.starts_with(if asking { "asking" } else { "waiting" }) {
                answers.borrow_mut().take();
            }
            said.lock().unwrap().push(news);
        };
        run(
            script,
            &heard,
            &tell,
            Duration::from_secs(5),
            Duration::from_millis(1),
        );
        let said = said.into_inner().unwrap();
        assert_eq!(said.last().unwrap(), "ended Cancelled", "asking: {asking}");
        assert_eq!(asked.lock().unwrap().last().unwrap(), &"decline");
    }
}

#[test]
fn a_failed_claim_ends_the_pairing_and_is_never_sent_again() {
    for (error, ending) in [
        (Error::Conflict, "ended ClaimedElsewhere"),
        (Error::Rejected, "ended Ended"),
        (Error::Unavailable, "ended Unavailable"),
        (Error::Busy, "ended Unavailable"),
        (Error::UpdateRequired, "ended UpdateRequired"),
    ] {
        let mut script = Script::new();
        script.claim = Err(error);
        let (said, asked) = pair(script, Player::Answers(Answer::Yes));
        assert_eq!(said, [ending], "{error}");
        assert_eq!(asked, ["claim"], "{error}");
    }
}

#[test]
fn a_pairing_that_ended_at_central_ends_here() {
    let mut script = Script::new();
    script.polls = [Ok(false)].into();
    script.then = Err(Error::Rejected);
    let (said, asked) = pair(script, Player::Answers(Answer::Yes));
    assert_eq!(said, ["waiting 4KQ-7MX", "ended Ended"]);
    assert_eq!(asked, ["claim", "approved", "approved", "decline"]);
}

/// A Central that does not answer for a while is asked again, as long as
/// the pairing lasts; one that answers again carries on.
#[test]
fn central_out_of_reach_is_asked_again_until_the_pairing_runs_out() {
    let mut script = Script::new();
    script.polls = [Err(Error::Unavailable), Err(Error::Busy), Ok(false)].into();
    let (said, _) = pair(script, Player::Answers(Answer::Yes));
    assert_eq!(said.last().unwrap(), "signed in pairing_player");

    let mut script = Script::new();
    script.then = Err(Error::Unavailable);
    let (said, asked) = pair(script, Player::Answers(Answer::Yes));
    assert_eq!(said.last().unwrap(), "ended Unavailable");
    assert_eq!(asked.last().unwrap(), &"decline");

    let mut script = Script::new();
    script.then = Ok(false);
    let (said, _) = pair(script, Player::Answers(Answer::Yes));
    assert_eq!(said.last().unwrap(), "ended Expired");
}

#[test]
fn a_refused_redeem_ends_the_pairing_without_a_question() {
    for (error, ending) in [
        (Error::Rejected, "ended Refused"),
        (Error::Conflict, "ended Refused"),
        (Error::Unavailable, "ended Unavailable"),
    ] {
        let mut script = Script::new();
        script.redeem = Err(error);
        let (said, asked) = pair(script, Player::Answers(Answer::Yes));
        assert_eq!(said, ["waiting 4KQ-7MX", ending], "{error}");
        assert_eq!(asked.last().unwrap(), &"redeem", "{error}: sent once");
    }
}

#[test]
fn a_restore_that_fails_after_the_yes_is_said() {
    let mut script = Script::new();
    script.accept = Err(Error::Rejected);
    let (said, _) = pair(script, Player::Answers(Answer::Yes));
    assert!(
        said.last().unwrap().starts_with("not signed in"),
        "{said:?}"
    );
}

// ------------------------------------------------------------- the lobby

/// Central's sign-in with a pairing under way at `stage`, and the channel
/// its worker would read.
fn lobby_pairing(stage: Stage) -> (LobbyState, mpsc::Receiver<Answer>, u64) {
    let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
    super::super::from_link(&mut state);
    let (answers, heard) = mpsc::sync_channel(1);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    state.central.as_mut().unwrap().pairing = Some(Pairing { id, answers, stage });
    (state, heard, id)
}

fn tell(state: &mut LobbyState, id: u64, news: News) {
    receive(
        state,
        &mut crate::prefs::Prefs::default(),
        &Mailbox::default(),
        id,
        news,
    );
}

#[test]
fn the_panel_follows_the_worker_and_says_how_it_ended() {
    let (mut state, _heard, id) = lobby_pairing(Stage::Claiming);
    tell(&mut state, id, News::Waiting("4KQ-7MX".into()));
    assert_eq!(
        state.pairing().unwrap().stage,
        Stage::Waiting {
            code: "4KQ-7MX".into()
        }
    );
    tell(
        &mut state,
        id,
        News::Asking("Pair_Player#002a (pairing_player)".into()),
    );
    assert!(
        matches!(&state.pairing().unwrap().stage, Stage::Asking { who } if who.contains("Pair_Player"))
    );
    tell(&mut state, id, News::Ended(Ending::ClaimedElsewhere));
    assert!(state.pairing().is_none());
    assert!(
        state.central_pending(),
        "Central's sign-in stays, with the reason"
    );
    assert_eq!(
        state.lobby.status(),
        Phrase::PairClaimedElsewhere.text(state.lobby.lang())
    );
}

#[test]
fn news_of_another_pairing_is_not_read() {
    let (mut state, _heard, id) = lobby_pairing(Stage::Claiming);
    tell(&mut state, id + 1000, News::Ended(Ending::Refused));
    tell(&mut state, id + 1000, News::Waiting("ZZZ-ZZZ".into()));
    assert_eq!(state.pairing().unwrap().stage, Stage::Claiming);
}

#[test]
fn the_yes_is_heard_only_when_the_question_stands() {
    let (mut state, heard, _) = lobby_pairing(Stage::Waiting {
        code: "4KQ-7MX".into(),
    });
    confirm(&mut state);
    assert!(heard.try_recv().is_err(), "no yes before the question");
    let (mut state, heard, _) = lobby_pairing(Stage::Asking {
        who: "pairing_player".into(),
    });
    confirm(&mut state);
    assert_eq!(heard.try_recv(), Ok(Answer::Yes));
    assert_eq!(state.pairing().unwrap().stage, Stage::Finishing);
}

#[test]
fn a_cancel_tells_the_worker_and_closes_the_panel() {
    let (mut state, heard, _) = lobby_pairing(Stage::Asking {
        who: "pairing_player".into(),
    });
    cancel(&mut state);
    assert!(state.pairing().is_none());
    assert_eq!(heard.try_recv(), Ok(Answer::No));
    assert!(state.central_pending());
}

/// Back from Central's sign-in (`‹`) drops the pairing with the form: its
/// worker hears the channel go.
#[test]
fn leaving_central_s_sign_in_lets_the_pairing_go() {
    let (mut state, heard, _) = lobby_pairing(Stage::Waiting {
        code: "4KQ-7MX".into(),
    });
    super::super::leave(&mut state);
    assert!(state.central.is_none());
    assert_eq!(heard.try_recv(), Err(mpsc::TryRecvError::Disconnected));
}

/// A yes the worker carried out signs the account in as a password's
/// sign-in does, the form's name its username.
#[test]
fn a_signed_in_pairing_is_a_signed_in_account() {
    let (mut state, _heard, id) = lobby_pairing(Stage::Finishing);
    tell(
        &mut state,
        id,
        News::SignedIn(Ok(Box::new(restored())), "pairing_player".into()),
    );
    assert!(state.pairing().is_none());
    assert!(state.central_ready());
    assert_eq!(state.lobby.field(Field::Username), "pairing_player");
    assert_eq!(
        state.central.as_ref().unwrap().profile_handle.as_deref(),
        Some("Pair_Player#002a")
    );
}
