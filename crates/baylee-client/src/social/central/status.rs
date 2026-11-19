//! The contact status at Baylee Central (`docs/client.md` §"The contact
//! status"): the player's choice is kept there, and what the client is doing
//! is reported there, on the account's serial worker.
//!
//! A choice goes one at a time and the latest one wins; a refusal puts back
//! what Central last kept. What the machine reports is said after every
//! `ready` of the push socket, since Central forgets it with the socket, and
//! then only when it changes, never more often than [`REPORT_EVERY_MS`].
use super::{Error, SocialHost};
use baylee_client_core::social::presence::{Activity, Choice};
use std::sync::{Mutex, mpsc};

/// The least time between two reports. Central takes thirty a minute from a
/// session; this keeps a client at twenty-four.
pub(super) const REPORT_EVERY_MS: u64 = 2_500;

/// How long after a reading of the kept choice failed it is tried again.
pub(super) const READ_AGAIN_MS: u64 = 60_000;

/// What the client says it is doing: playing or at a table, and idle.
type Report = (Option<Activity>, bool);

/// The worker's answer to one request, once it has one.
type Answer<T> = Mutex<mpsc::Receiver<Result<T, Error>>>;

/// A choice on its way to Central.
struct Asked {
    /// The choice before it, put back if Central has kept none yet.
    was: Choice,
    answer: Answer<Choice>,
}

pub(super) struct Status {
    /// The choice Central last said or took.
    kept: Option<Choice>,
    /// The choice to send next, with the one before it.
    wanted: Option<(Choice, Choice)>,
    asked: Option<Asked>,
    /// Central's kept choice is read by request until it is known, for a
    /// push socket that never opens; the socket's own word replaces it.
    reading: Option<Answer<Option<Choice>>>,
    read: bool,
    read_at: u64,
    seen_ready_seq: u64,
    /// What was said since the push socket's last `ready`, and when.
    said: Option<Report>,
    said_at: u64,
}

impl Status {
    pub(super) fn new(ready_seq: u64) -> Self {
        Self {
            kept: None,
            wanted: None,
            asked: None,
            reading: None,
            read: false,
            read_at: 0,
            seen_ready_seq: ready_seq,
            said: None,
            said_at: 0,
        }
    }

    /// Whether a choice of this client's is still on its way: Central's
    /// frames may then still carry the choice before it.
    fn choosing(&self) -> bool {
        self.wanted.is_some() || self.asked.is_some()
    }
}

/// The machine chose: Central is asked, after any choice still on its way.
pub(in crate::social) fn choose(host: &mut SocialHost, choice: Choice, was: Choice) {
    if let Some(bridge) = host.central.as_mut() {
        // The first of several waiting names what stood before them all.
        let was = bridge.status.wanted.map_or(was, |(_, first)| first);
        bridge.status.wanted = Some((choice, was));
    }
}

/// Central's push socket said the account's own choice (`None`: it keeps
/// none). Not heard while a choice of this client's is on its way, whose
/// answer says the same or newer.
pub(super) fn heard(host: &mut SocialHost, me: Option<Choice>) {
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    if bridge.status.choosing() {
        return;
    }
    bridge.status.kept = me.or(bridge.status.kept);
    bridge.status.read = true;
    bridge.status.reading = None;
    if let Some(machine) = host.machine.as_mut() {
        let effects = machine.heard_choice(me);
        host.carry_out(effects);
    }
}

pub(super) fn tick(host: &mut SocialHost, now: u64) {
    answers(host, now);
    let Some(report) = host
        .machine
        .as_ref()
        .map(baylee_client_core::social::Social::report)
    else {
        return;
    };
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    let status = &mut bridge.status;
    if status.asked.is_none()
        && let Some((choice, was)) = status.wanted
    {
        let (send, answer) = mpsc::sync_channel(1);
        if bridge.handle.submit(move |account| {
            let _ = send.send(account.set_presence(choice));
        }) {
            status.wanted = None;
            status.asked = Some(Asked {
                was,
                answer: Mutex::new(answer),
            });
        }
    }
    if !status.read && status.reading.is_none() && !status.choosing() && now >= status.read_at {
        let (send, answer) = mpsc::sync_channel(1);
        if bridge.handle.submit(move |account| {
            let _ = send.send(kept_at_central(account.presence_choice(), || {
                account.check_active().is_ok()
            }));
        }) {
            status.reading = Some(Mutex::new(answer));
        }
    }
    let ready_seq = bridge.handle.ready_seq();
    if ready_seq != status.seen_ready_seq {
        status.seen_ready_seq = ready_seq;
        status.said = None;
    }
    if bridge.handle.ws_live()
        && status.said != Some(report)
        && now >= status.said_at.saturating_add(REPORT_EVERY_MS)
    {
        let (doing, idle) = report;
        // Nothing waits for the answer: Central answers alike whether it
        // heard the report or not, and the next change says it again.
        if bridge.handle.submit(move |account| {
            let _ = account.report_presence(doing, idle);
        }) {
            status.said = Some(report);
            status.said_at = now;
        }
    }
}

/// What the worker answered since the last tick.
fn answers(host: &mut SocialHost, now: u64) {
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    let status = &mut bridge.status;
    let chosen = status
        .asked
        .as_ref()
        .and_then(|asked| arrived(&asked.answer).map(|answer| (asked.was, answer)));
    if chosen.is_some() {
        status.asked = None;
    }
    // What the reading said: a choice, or that Central keeps none.
    let read = match status.reading.as_ref().and_then(arrived) {
        Some(Ok(stored)) => Some(stored),
        Some(Err(_)) => {
            status.reading = None;
            status.read_at = now.saturating_add(READ_AGAIN_MS);
            None
        }
        None => None,
    };
    if read.is_some() {
        status.reading = None;
        status.read = true;
    }
    let mut effects = Vec::new();
    if let Some((was, answer)) = chosen {
        match answer {
            Ok(stored) => {
                status.kept = Some(stored);
                status.read = true;
                if status.wanted.is_none()
                    && let Some(machine) = host.machine.as_mut()
                {
                    effects.extend(machine.heard_choice(Some(stored)));
                }
            }
            Err(error) => {
                // Whatever waited behind it was chosen on top of a choice
                // Central never took.
                status.wanted = None;
                let back = status.kept.unwrap_or(was);
                if let Some(machine) = host.machine.as_mut() {
                    effects.extend(
                        machine.choice_refused(back, matches!(error, Error::TermsRequired)),
                    );
                }
            }
        }
    }
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    if let Some(stored) = read
        && !bridge.status.choosing()
    {
        bridge.status.kept = stored.or(bridge.status.kept);
        if let Some(machine) = host.machine.as_mut() {
            effects.extend(machine.heard_choice(stored));
        }
    }
    host.carry_out(effects);
}

/// What Central keeps, from the reading of the choice: a choice, or none.
///
/// A Central from before the status cannot read the request and rejects it
/// (its `401`), exactly as it rejects a session that ended. Only a session
/// that still `stands` tells the two apart: then nobody can have chosen
/// offline there, and the account need not stay hidden from its game gateway.
/// A session that ended, or cannot be asked, is no answer.
fn kept_at_central(
    read: Result<Choice, Error>,
    stands: impl FnOnce() -> bool,
) -> Result<Option<Choice>, Error> {
    match read {
        Ok(choice) => Ok(Some(choice)),
        Err(Error::Rejected) if stands() => Ok(None),
        Err(error) => Err(error),
    }
}

/// The worker's answer, once it is there; a worker that went answers
/// `Unavailable`.
fn arrived<T>(answer: &Answer<T>) -> Option<Result<T, Error>> {
    let Ok(answer) = answer.lock() else {
        return Some(Err(Error::Unavailable));
    };
    match answer.try_recv() {
        Ok(result) => Some(result),
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => Some(Err(Error::Unavailable)),
    }
}

#[cfg(test)]
mod tests;
