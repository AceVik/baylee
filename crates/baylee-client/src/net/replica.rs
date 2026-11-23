//! A protected socket carries only this seat's certified projection. The account
//! worker owns journal submissions and may revoke this socket between frames.
use baylee_account::{
    Error,
    ranked::{
        Assignment, Id,
        replica::{CommandBytes, ConsentState, Fence, Head, Phase, SeatRequest, SeatStatus},
    },
};
use baylee_client_core::Phrase;
use baylee_protocol::v1::{Envelope, envelope::Msg};
use prost::Message as _;
use std::{
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};
#[cfg(test)]
mod tests;

#[derive(Clone, Default)]
pub(crate) struct Table(Arc<Mutex<Journal>>);

#[derive(Default)]
struct Journal {
    status: Option<SeatStatus>,
    installed: Option<u64>,
    instance: u64,
    reachable: bool,
    shown: Option<Head>,
    acknowledged: u64,
    pending: Option<Proposal>,
    connected: bool,
    last_presence: Option<(Fence, bool, Instant)>,
    sampled: u64,
}

struct Proposal {
    id: Id,
    expected: Head,
    body: CommandBytes,
    retry: Instant,
}

impl Table {
    fn lock(&self) -> MutexGuard<'_, Journal> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn observe(&self, status: SeatStatus) -> bool {
        let mut j = self.lock();
        if j.status.as_ref().is_some_and(|old| {
            old.fence.match_id != status.fence.match_id
                || old.fence.generation > status.fence.generation
        }) {
            return false;
        }
        let changed = !j.reachable
            || j.status.as_ref().is_none_or(|old| {
                old.consent != status.consent
                    || old.fence != status.fence
                    || old.phase != status.phase
                    || old.host_id != status.host_id
            });
        if j.status
            .as_ref()
            .is_none_or(|old| old.fence != status.fence || old.host_id != status.host_id)
            || status.consent != ConsentState::Ready
        {
            j.installed = None;
            j.connected = false;
        }
        j.reachable = true;
        j.sampled = j.sampled.wrapping_add(1);
        j.status = Some(status);
        changed
    }

    pub(crate) fn unavailable(&self) {
        let mut j = self.lock();
        j.reachable = false;
        j.installed = None;
        j.connected = false;
    }

    pub(crate) fn expected(&self, assignment: &Assignment) -> Option<SeatStatus> {
        let j = self.lock();
        let status = j.status.as_ref()?;
        (j.reachable
            && status.consent == ConsentState::Ready
            && status.fence.match_id == assignment.match_id
            && status.host_id == assignment.host_id
            && status.origin == assignment.origin
            && matches!(
                status.phase,
                Phase::Single | Phase::Preparing | Phase::Playing | Phase::Paused
            ))
        .then(|| status.clone())
    }

    pub(crate) fn needs_ticket(&self) -> bool {
        let j = self.lock();
        j.reachable
            && j.installed.is_none()
            && j.status.as_ref().is_some_and(|s| {
                s.consent == ConsentState::Ready
                    && matches!(
                        s.phase,
                        Phase::Single | Phase::Preparing | Phase::Playing | Phase::Paused
                    )
            })
    }

    pub(crate) fn attach(&self, status: &SeatStatus) -> Option<Binding> {
        let mut j = self.lock();
        if !j.reachable
            || j.status
                .as_ref()
                .is_none_or(|s| s.fence != status.fence || s.host_id != status.host_id)
        {
            return None;
        }
        j.instance = j.instance.wrapping_add(1);
        j.installed = Some(j.instance);
        j.connected = false;
        Some(Binding {
            table: self.clone(),
            fence: status.fence,
            instance: j.instance,
            has_view: false,
            clock_sample: None,
            dialled: Instant::now(),
            mirrored: status.phase != Phase::Single,
        })
    }

    pub(crate) fn presence_due(&self, now: Instant) -> bool {
        self.lock().presence(now).is_some()
    }

    pub(crate) fn take_presence(&self, now: Instant) -> Option<(Fence, bool)> {
        let mut j = self.lock();
        let (fence, connected) = j.presence(now)?;
        j.last_presence = Some((fence, connected, now));
        Some((fence, connected))
    }

    pub(crate) fn waiting_consent(&self) -> bool {
        self.lock()
            .status
            .as_ref()
            .is_some_and(|s| s.consent == ConsentState::Waiting)
    }

    pub(crate) fn has_submission(&self, now: Instant) -> bool {
        let j = self.lock();
        j.reachable
            && j.status.as_ref().is_some_and(|s| {
                s.phase == Phase::Playing && s.clock.is_some() && s.consent == ConsentState::Ready
            })
            && j.pending.as_ref().is_some_and(|p| now >= p.retry)
    }

    pub(crate) fn submission(&self, now: Instant) -> Option<SeatRequest> {
        let j = self.lock();
        let status = j.status.as_ref()?;
        let proposal = j.pending.as_ref()?;
        (j.reachable
            && status.phase == Phase::Playing
            && status.clock.is_some()
            && status.consent == ConsentState::Ready
            && now >= proposal.retry)
            .then(|| SeatRequest::Submit {
                fence: status.fence,
                expected: proposal.expected,
                command_id: proposal.id,
                body: proposal.body.clone(),
            })
    }

    pub(crate) fn answered(&self, id: Id, result: Result<u64, Error>, now: Instant) {
        let mut j = self.lock();
        let Some(proposal) = j.pending.as_mut().filter(|p| p.id == id) else {
            return;
        };
        match result {
            Ok(position) if proposal.expected.position.checked_add(1) == Some(position) => {
                j.acknowledged = j.acknowledged.max(position);
            }
            Err(Error::Unavailable | Error::Busy) => {
                proposal.retry = now + Duration::from_secs(2);
                return;
            }
            // A stale choice is never re-authored against a later view. A
            // fresh full snapshot restores the engine's current question.
            Err(_) | Ok(_) => j.installed = None,
        }
        j.pending = None;
    }
}

impl Journal {
    fn presence(&self, now: Instant) -> Option<(Fence, bool)> {
        let status = self.status.as_ref()?;
        if !self.reachable
            || status.consent != ConsentState::Ready
            || !matches!(
                status.phase,
                Phase::Preparing | Phase::Paused | Phase::Playing
            )
        {
            return None;
        }
        let fence = status.fence;
        let due = self
            .last_presence
            .map_or(self.connected, |(old, was, when)| {
                old != fence
                    || was != self.connected
                    || (self.connected && now.duration_since(when) >= Duration::from_secs(4))
            });
        due.then_some((fence, self.connected))
    }
}

impl Drop for Binding {
    fn drop(&mut self) {
        self.lost();
    }
}

pub(crate) struct Binding {
    pub(super) mirrored: bool,
    table: Table,
    fence: Fence,
    instance: u64,
    has_view: bool,
    clock_sample: Option<u64>,
    dialled: Instant,
}

impl Binding {
    pub(super) fn current(&self) -> bool {
        let j = self.table.lock();
        j.installed == Some(self.instance)
            && j.status
                .as_ref()
                .is_some_and(|s| s.consent == ConsentState::Ready)
    }
    pub(super) fn single_snapshot(&mut self) {
        self.has_view = true;
        self.connection(true);
    }
    pub(super) fn clocks(&mut self, new_view: bool) -> Option<crate::host::RankedClocks> {
        let j = self.table.lock();
        if !self.mirrored
            || j.installed != Some(self.instance)
            || (!new_view && self.clock_sample == Some(j.sampled))
        {
            return None;
        }
        self.clock_sample = Some(j.sampled);
        let clock = j.status.as_ref()?.clock;
        Some(crate::host::RankedClocks {
            decisions: clock.map_or([None; 2], |c| c.decision_secs),
            reconnect: clock.map_or([None; 2], |c| c.reconnect_secs),
            running: clock.is_some_and(|c| c.running),
        })
    }
    pub(super) fn cursor(&self) -> Option<(u64, u64, [u8; 32])> {
        if !self.has_view {
            return None;
        }
        let j = self.table.lock();
        let shown = j.shown?;
        (j.installed == Some(self.instance)).then_some((
            self.fence.generation,
            shown.position,
            shown.chain,
        ))
    }
    pub(super) fn connection(&self, up: bool) {
        let mut j = self.table.lock();
        if j.installed == Some(self.instance) {
            j.connected = up && self.has_view;
        }
    }
    pub(super) fn lost(&self) {
        let mut j = self.table.lock();
        if j.installed == Some(self.instance) {
            j.installed = None;
            j.connected = false;
        }
    }

    pub(super) fn hold(&self) -> Option<Phrase> {
        let mut j = self.table.lock();
        if !self.has_view
            && j.installed == Some(self.instance)
            && self.dialled.elapsed() >= Duration::from_secs(30)
        {
            j.installed = None;
            j.connected = false;
        }
        let phase = j.status.as_ref().map(|s| s.phase);
        if phase == Some(Phase::Disputed) {
            Some(Phrase::RankedReviewHold)
        } else if phase == Some(Phase::Paused) {
            Some(Phrase::RankedVerifierHold)
        } else if j.reachable
            && j.installed == Some(self.instance)
            && self.has_view
            && j.status
                .as_ref()
                .is_some_and(|s| s.phase == Phase::Playing && s.clock.is_none())
        {
            Some(Phrase::RankedSeatsHold)
        } else if !j.reachable
            || j.status.as_ref().is_none_or(|s| {
                s.consent != ConsentState::Ready || (self.mirrored && s.clock.is_none())
            })
            || j.installed != Some(self.instance)
            || !matches!(phase, Some(Phase::Playing | Phase::Single))
            || !self.has_view
        {
            Some(Phrase::RankedHostHold)
        } else if self.mirrored
            && (j.pending.is_some() || j.shown.is_none_or(|h| h.position < j.acknowledged))
        {
            Some(Phrase::RankedCommandHold)
        } else {
            None
        }
    }

    pub(super) fn enqueue(&self, envelope: &Envelope) {
        if self.hold().is_some() {
            return;
        }
        let Ok(body) = CommandBytes::new(envelope.encode_to_vec()) else {
            self.lost();
            return;
        };
        let mut j = self.table.lock();
        if j.installed == Some(self.instance)
            && j.pending.is_none()
            && let Some(expected) = j.shown
        {
            j.pending = Some(Proposal {
                id: *uuid::Uuid::now_v7().as_bytes(),
                expected,
                body,
                retry: Instant::now(),
            });
        }
    }

    pub(super) fn decode(&self, envelope: Envelope) -> Option<(Envelope, Option<Head>)> {
        let Some(Msg::RankedSeatProjection(stamp)) = envelope.msg.as_ref() else {
            // Unstamped connection metadata never enters the gameplay decoder.
            return matches!(envelope.msg, Some(Msg::ClockProbe(_))).then_some((envelope, None));
        };
        let j = self.table.lock();
        let head = Head {
            position: stamp.position,
            chain: stamp.chain.as_slice().try_into().ok()?,
        };
        if stamp.generation != self.fence.generation
            || j.installed != Some(self.instance)
            || (!self.has_view && stamp.position < j.acknowledged)
            || j.shown.is_some_and(|last| {
                head.position < last.position
                    || (head.position == last.position && head.chain != last.chain)
            })
            || j.status
                .as_ref()
                .is_some_and(|s| head.position == s.head.position && head.chain != s.head.chain)
        {
            return None;
        }
        let inner = Envelope::decode(stamp.envelope.as_slice()).ok()?;
        Some((inner, Some(head)))
    }

    pub(super) fn rendered(&mut self, head: Head, message: &crate::host::HostMessage) {
        if matches!(message, crate::host::HostMessage::View(..)) {
            self.has_view = true;
            self.connection(true);
        }
        if self.has_view
            && matches!(
                message,
                crate::host::HostMessage::View(..) | crate::host::HostMessage::Choice(_)
            )
        {
            let mut j = self.table.lock();
            if j.installed == Some(self.instance) {
                j.shown = Some(head);
            }
        }
    }
}

/// The ranked WebSocket always terminates at the authenticated Central origin.
/// Only an explicitly local development authority may use cleartext.
pub(super) fn socket_url(origin: &str) -> Result<String, String> {
    use url::{Host, Url};
    let invalid = || "invalid ranked relay origin".to_string();
    let mut url = Url::parse(origin).map_err(|_| invalid())?;
    let host = url.host().ok_or_else(invalid)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    let scheme = match (url.scheme(), host) {
        ("https", _) => "wss",
        ("http", Host::Ipv4(ip)) if ip.is_loopback() => "ws",
        ("http", Host::Ipv6(ip)) if ip.is_loopback() => "ws",
        _ => return Err(invalid()),
    };
    url.set_scheme(scheme).map_err(|()| invalid())?;
    url.set_path("/v1/ranked/socket");
    Ok(url.into())
}
