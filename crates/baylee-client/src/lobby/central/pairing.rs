//! Signing in from baylee.app (`docs/client.md` §"Signing in from the
//! website"): a `baylee://pair` link, claimed at Central, the code the page
//! shows beside its Approve shown here while the page is asked, and the
//! player's own yes to whom the session signs in as.
//!
//! The pairing runs on a worker of its own, which holds the link's secret,
//! the claim's keys and, once redeemed, the session. The lobby sees only the
//! code, the account's name, and how it ended. The worker reads the lobby's
//! answers on a channel kept in Central's state ([`super::State`]): whatever
//! ends that state (`‹`, a sign-out, the window closing) drops the channel,
//! and the worker takes that as a no and signs a session it holds out.
#![forbid(unsafe_code)]

use super::super::{Field, LobbyState, Mailbox, Reply};
use super::Restored;
use baylee_account::{Account, Endpoint, Error, PairedSession, PairingClaim};
use baylee_client_core::{Phrase, deeplink::PairLink};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

/// How long a claimed pairing is waited on, and the question after it: the
/// pairing's five minutes at Central.
const WAIT: Duration = Duration::from_mins(5);

/// How often Central is asked whether the page approved: about once a
/// second, which Central's 300 answers per pairing allow for five minutes.
const POLL_EVERY: Duration = Duration::from_secs(1);

/// Each pairing's number, so a worker's news about an earlier one, which a
/// cancel left running, is never read as the current one's.
static NEXT: AtomicU64 = AtomicU64::new(1);

/// What the player answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// Sign in as the account the question names.
    Yes,
    /// Cancel, at any stage.
    No,
}

/// Where a pairing stands, as the panel over the sign-in shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Claiming the link's pairing at Central.
    Claiming,
    /// Claimed: the code the page shows beside its Approve, while the page
    /// is asked whether the player approved there.
    Waiting {
        /// `XXX-XXX`.
        code: String,
    },
    /// Approved and redeemed: whom the session signs in as, for the
    /// player's yes (`Name#tag (username)`).
    Asking {
        /// The account, as the question names it.
        who: String,
    },
    /// Yes: the account being restored.
    Finishing,
}

impl Stage {
    /// The stage's name for dev-control's `/state`, never what it shows.
    #[cfg(feature = "dev-control")]
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Stage::Claiming => "pairing-claiming",
            Stage::Waiting { .. } => "pairing-waiting",
            Stage::Asking { .. } => "pairing-question",
            Stage::Finishing => "pairing-finishing",
        }
    }
}

/// A pairing under way, kept in Central's state.
pub(crate) struct Pairing {
    id: u64,
    answers: mpsc::SyncSender<Answer>,
    pub(crate) stage: Stage,
}

/// How a pairing ended without a sign-in: what the sign-in form says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// The player cancelled, or said no to the account.
    Cancelled,
    /// Five minutes passed without an approval, or without an answer to the
    /// question.
    Expired,
    /// Another program claimed the pairing first: the page must not approve
    /// it.
    ClaimedElsewhere,
    /// The pairing ended at Central: cancelled on the page, used, or past
    /// its time.
    Ended,
    /// Central refused the redeem: the seal did not open, or the pairing
    /// was not this client's.
    Refused,
    /// Central could not be reached.
    Unavailable,
    /// Central says this client is too old.
    UpdateRequired,
}

impl Ending {
    fn phrase(self) -> Phrase {
        match self {
            Ending::Cancelled => Phrase::PairCancelled,
            Ending::Expired => Phrase::PairExpired,
            Ending::ClaimedElsewhere => Phrase::PairClaimedElsewhere,
            Ending::Ended => Phrase::PairEnded,
            Ending::Refused => Phrase::PairRefused,
            Ending::Unavailable => Phrase::PairUnavailable,
            Ending::UpdateRequired => Phrase::RefusalUpdateRequired,
        }
    }
}

/// What the worker reports to the lobby.
pub(in crate::lobby) enum News {
    /// Claimed, with the comparison code.
    Waiting(String),
    /// Redeemed, with whom the session signs in as.
    Asking(String),
    /// Ended without a sign-in.
    Ended(Ending),
    /// The player's yes, carried out: the account restored (with its
    /// username, for the sign-in form), or why not.
    SignedIn(Result<Box<Restored>, Error>, String),
}

/// What a pairing asks of Central, step by step: the SDK's ([`Sdk`]), or a
/// script in the tests.
trait Steps: Send + 'static {
    /// Claims the link's pairing, once; answers the comparison code.
    fn claim(&mut self) -> Result<String, Error>;
    /// Whether the page approved yet.
    fn approved(&mut self) -> Result<bool, Error>;
    /// Redeems the approved pairing, once; answers whom it signs in as.
    fn redeem(&mut self) -> Result<String, Error>;
    /// The player's yes: the account restored, and its username.
    fn accept(&mut self) -> (Result<Box<Restored>, Error>, String);
    /// The player's no, or no answer: whatever is held is let go, a
    /// session signed out.
    fn decline(&mut self);
}

/// The steps through `baylee-account` and Central's SDK.
struct Sdk {
    endpoint: Result<Endpoint, Error>,
    /// The link, until it is claimed: dropped (and wiped) with the claim.
    link: Option<PairLink>,
    claim: Option<PairingClaim>,
    paired: Option<PairedSession>,
}

impl Steps for Sdk {
    fn claim(&mut self) -> Result<String, Error> {
        let link = self.link.take().ok_or(Error::Rejected)?;
        let endpoint = self.endpoint.clone()?;
        let claim = Account::claim_pairing(endpoint, link.handle(), link.secret())?;
        drop(link);
        let code = claim.comparison_code();
        self.claim = Some(claim);
        Ok(code)
    }

    fn approved(&mut self) -> Result<bool, Error> {
        self.claim.as_ref().ok_or(Error::Rejected)?.approved()
    }

    fn redeem(&mut self) -> Result<String, Error> {
        let paired = self.claim.take().ok_or(Error::Rejected)?.redeem()?;
        let who = paired.who();
        self.paired = Some(paired);
        Ok(who)
    }

    fn accept(&mut self) -> (Result<Box<Restored>, Error>, String) {
        let Some(paired) = self.paired.take() else {
            return (Err(Error::Rejected), String::new());
        };
        let username = paired.username().to_string();
        let profile_handle = paired.profile_handle();
        let restored = paired
            .accept()
            .and_then(|account| super::restored(account, profile_handle))
            .map(Box::new);
        (restored, username)
    }

    fn decline(&mut self) {
        self.link = None;
        self.claim = None;
        if let Some(paired) = self.paired.take() {
            let _ = paired.decline();
        }
    }
}

/// A `baylee://pair` link, followed over Central's sign-in (which the link
/// opened, with its way back): the pairing starts. Nothing without that
/// sign-in, over a session, or when one pairing is under way already.
pub(crate) fn start(state: &mut LobbyState, mailbox: &Mailbox, link: PairLink) {
    let Some(central) = state
        .central
        .as_mut()
        .filter(|central| central.handle.is_none() && central.pairing.is_none())
    else {
        return;
    };
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let (answers, heard) = mpsc::sync_channel(1);
    let steps = Sdk {
        endpoint: central.endpoint.clone(),
        link: Some(link),
        claim: None,
        paired: None,
    };
    let target = mailbox.clone();
    let tell = move |news: News| {
        if let Ok(mut messages) = target.0.lock() {
            messages.push(Reply::Pairing(id, news));
        }
    };
    let launched = std::thread::Builder::new()
        .name("central-pairing".into())
        .spawn(move || run(steps, &heard, &tell, WAIT, POLL_EVERY));
    if launched.is_err() {
        state.lobby.tell_refusal(Phrase::PairUnavailable, &[]);
        return;
    }
    central.pairing = Some(Pairing {
        id,
        answers,
        stage: Stage::Claiming,
    });
    state.lobby.clear_status();
}

/// The worker: claim, wait for the page, redeem, ask, and sign in or let go.
fn run(
    mut steps: impl Steps,
    answers: &mpsc::Receiver<Answer>,
    tell: &impl Fn(News),
    wait: Duration,
    every: Duration,
) {
    let code = match steps.claim() {
        Ok(code) => code,
        // Never claimed again: the SDK makes a new key pair for every
        // claim, which Central would take for another program's.
        Err(error) => return tell(News::Ended(claim_ending(error))),
    };
    if cancelled(answers) {
        steps.decline();
        return tell(News::Ended(Ending::Cancelled));
    }
    tell(News::Waiting(code));
    let until = Instant::now() + wait;
    let mut unreachable = false;
    loop {
        match answers.recv_timeout(every) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                steps.decline();
                return tell(News::Ended(Ending::Cancelled));
            }
        }
        if Instant::now() >= until {
            steps.decline();
            let ending = if unreachable {
                Ending::Unavailable
            } else {
                Ending::Expired
            };
            return tell(News::Ended(ending));
        }
        match steps.approved() {
            Ok(true) => break,
            Ok(false) => unreachable = false,
            // A Central restarting, or a network dropping out: asked again,
            // as long as the pairing lasts.
            Err(Error::Unavailable | Error::Busy) => unreachable = true,
            Err(error) => {
                steps.decline();
                return tell(News::Ended(poll_ending(error)));
            }
        }
    }
    let who = match steps.redeem() {
        Ok(who) => who,
        Err(error) => return tell(News::Ended(redeem_ending(error))),
    };
    if cancelled(answers) {
        steps.decline();
        return tell(News::Ended(Ending::Cancelled));
    }
    tell(News::Asking(who));
    match answers.recv_timeout(wait) {
        Ok(Answer::Yes) => {
            let (restored, username) = steps.accept();
            tell(News::SignedIn(restored, username));
        }
        Ok(Answer::No) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            steps.decline();
            tell(News::Ended(Ending::Cancelled));
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            steps.decline();
            tell(News::Ended(Ending::Expired));
        }
    }
}

/// Whether the player cancelled, or the lobby let the pairing go.
fn cancelled(answers: &mpsc::Receiver<Answer>) -> bool {
    matches!(
        answers.try_recv(),
        Ok(_) | Err(mpsc::TryRecvError::Disconnected)
    )
}

/// Why a claim failed, as far as Central says.
fn claim_ending(error: Error) -> Ending {
    match error {
        Error::Conflict => Ending::ClaimedElsewhere,
        Error::Unavailable | Error::Busy => Ending::Unavailable,
        Error::UpdateRequired => Ending::UpdateRequired,
        _ => Ending::Ended,
    }
}

/// Why asking for the approval failed: the pairing ended.
fn poll_ending(error: Error) -> Ending {
    match error {
        Error::UpdateRequired => Ending::UpdateRequired,
        _ => Ending::Ended,
    }
}

/// Why a redeem failed. It is never sent again: Central ends the pairing
/// whatever the redeem finds.
fn redeem_ending(error: Error) -> Ending {
    match error {
        Error::Unavailable | Error::Busy => Ending::Unavailable,
        Error::UpdateRequired => Ending::UpdateRequired,
        _ => Ending::Refused,
    }
}

/// What the worker said, read by the lobby: a stage, an ending said on the
/// sign-in form, or the account signed in. News of a pairing that is not the
/// current one is dropped (a session in it with it, which signs it out).
pub(in crate::lobby) fn receive(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    mailbox: &Mailbox,
    id: u64,
    news: News,
) {
    let Some(pairing) = state
        .central
        .as_mut()
        .and_then(|central| central.pairing.as_mut())
        .filter(|pairing| pairing.id == id)
    else {
        return;
    };
    match news {
        News::Waiting(code) => pairing.stage = Stage::Waiting { code },
        News::Asking(who) => pairing.stage = Stage::Asking { who },
        News::Ended(ending) => {
            end(state);
            let phrase = ending.phrase();
            if ending == Ending::Cancelled {
                state.lobby.tell(phrase, &[]);
            } else {
                state.lobby.tell_refusal(phrase, &[]);
            }
        }
        News::SignedIn(result, username) => {
            end(state);
            if result.is_ok() {
                state.lobby.set_field(Field::Username, &username);
            }
            super::receive(state, prefs, mailbox, result);
        }
    }
}

/// The pairing is over: its panel goes, and its channel with it.
fn end(state: &mut LobbyState) {
    if let Some(central) = &mut state.central {
        central.pairing = None;
    }
}

/// The player cancelled, at any stage: the worker hears it and lets go; a
/// session it redeemed is signed out.
pub(crate) fn cancel(state: &mut LobbyState) {
    let Some(pairing) = state
        .central
        .as_mut()
        .and_then(|central| central.pairing.take())
    else {
        return;
    };
    let _ = pairing.answers.try_send(Answer::No);
    state.lobby.tell(Phrase::PairCancelled, &[]);
}

/// The player's yes to the question: the worker restores the account.
/// Nothing at any other stage.
pub(crate) fn confirm(state: &mut LobbyState) {
    let Some(pairing) = state
        .central
        .as_mut()
        .and_then(|central| central.pairing.as_mut())
        .filter(|pairing| matches!(pairing.stage, Stage::Asking { .. }))
    else {
        return;
    };
    if pairing.answers.try_send(Answer::Yes).is_ok() {
        pairing.stage = Stage::Finishing;
    }
}

#[cfg(test)]
mod tests;
