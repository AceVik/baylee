//! Nonblocking Tower socket. Network clocks and heartbeat state never touch `LobbyState`.
use super::{Finished, Work, central};
use baylee_account::{Error, TowerHttp, TowerSocket};
use baylee_client_core::echoes::api::{
    self,
    socket::{Client, Refusal, Server},
};
use ewebsock::{WsEvent, WsMessage};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

type Dial = Arc<Mutex<Option<Result<TowerSocket, Error>>>>;

#[derive(Default)]
pub(super) struct Live {
    link: Option<Link>,
    dial: Option<Dial>,
    pending: Option<Work>,
    sent: bool,
    deadline: Option<Instant>,
    retry_at: Option<Instant>,
    ever_ready: bool,
    resync: bool,
}

struct Link {
    sender: crate::transport::WsSender,
    receiver: crate::transport::WsReceiver,
    access: TowerSocket,
    ready: bool,
    renew: Instant,
}

#[derive(Default)]
pub(super) struct Updates {
    pub(super) finished: Option<Finished>,
    pub(super) pushed: Vec<api::Response>,
    pub(super) resync: bool,
}

impl Live {
    pub(super) fn enqueue(
        &mut self,
        work: Work,
        handle: Option<&central::Handle>,
        connection: &Arc<Mutex<Option<TowerHttp>>>,
    ) -> Result<(), Error> {
        if self.pending.is_some() {
            return Err(Error::Busy);
        }
        if self.link.is_none() && self.dial.is_none() {
            self.dial(handle, connection)?;
        }
        self.pending = Some(work);
        self.sent = false;
        self.deadline = Some(Instant::now() + Duration::from_secs(45));
        Ok(())
    }

    fn dial(
        &mut self,
        handle: Option<&central::Handle>,
        connection: &Arc<Mutex<Option<TowerHttp>>>,
    ) -> Result<(), Error> {
        let handle = handle.ok_or(Error::Rejected)?;
        let dial: Dial = Arc::default();
        let output = Arc::clone(&dial);
        let connection = Arc::clone(connection);
        if !handle.submit(move |account| {
            let result = account.tower().and_then(|http| {
                let access = http.websocket();
                *connection.lock().map_err(|_| Error::Unavailable)? = Some(http);
                Ok(access)
            });
            if let Ok(mut slot) = output.lock() {
                *slot = Some(result);
            }
        }) {
            return Err(Error::Busy);
        }
        self.dial = Some(dial);
        self.deadline = Some(Instant::now() + Duration::from_secs(45));
        Ok(())
    }

    pub(super) fn poll(
        &mut self,
        showing: bool,
        handle: Option<&central::Handle>,
        connection: &Arc<Mutex<Option<TowerHttp>>>,
    ) -> Updates {
        let mut updates = Updates::default();
        if !showing && self.pending.is_none() {
            self.link = None;
            self.dial = None;
            self.deadline = None;
            return updates;
        }
        if self
            .link
            .as_ref()
            .is_some_and(|link| Instant::now() >= link.renew)
            && self.pending.is_none()
        {
            self.link = None;
        }
        if self.link.is_none()
            && self.dial.is_none()
            && self.retry_at.is_none_or(|at| Instant::now() >= at)
            && let Err(error) = self.dial(handle, connection)
        {
            self.failed(error, &mut updates);
        }
        let opened = self
            .dial
            .as_ref()
            .and_then(|mailbox| mailbox.lock().ok()?.take());
        if let Some(opened) = opened {
            self.dial = None;
            match opened.and_then(open) {
                Ok(link) => self.link = Some(link),
                Err(error) => self.failed(error, &mut updates),
            }
        }
        let events = self
            .link
            .as_ref()
            .map_or_default(|link| crate::transport::drain_socket(&link.receiver));
        for event in events {
            let result = match event {
                WsEvent::Message(WsMessage::Binary(bytes)) => self.frame(&bytes, &mut updates),
                WsEvent::Message(WsMessage::Text(text)) => {
                    self.frame(text.as_bytes(), &mut updates)
                }
                WsEvent::Closed | WsEvent::Error(_) => Err(Error::Unavailable),
                _ => Ok(()),
            };
            if let Err(error) = result {
                self.failed(error, &mut updates);
                break;
            }
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.failed(Error::Unavailable, &mut updates);
        }
        if let Some(link) = self.link.as_mut().filter(|link| link.ready)
            && let Some(work) = self.pending.as_ref().filter(|_| !self.sent)
        {
            let frame = work.request().map(|request| Client {
                serial: work.id(),
                request: request.clone(),
            });
            if let Some(bytes) = frame
                .and_then(|frame| serde_json::to_vec(&frame).ok())
                .filter(|bytes| bytes.len() <= api::socket::MAX_CLIENT_BYTES)
            {
                link.sender.send(WsMessage::Binary(bytes));
                self.sent = true;
            } else {
                self.failed(Error::Rejected, &mut updates);
            }
        }
        if self.resync && self.pending.is_none() && updates.finished.is_none() {
            updates.resync = true;
            self.resync = false;
        }
        updates
    }

    fn frame(&mut self, bytes: &[u8], updates: &mut Updates) -> Result<(), Error> {
        if bytes.len() > api::socket::MAX_SERVER_BYTES {
            return Err(Error::Rejected);
        }
        let frame: Server = serde_json::from_slice(bytes).map_err(|_| Error::Rejected)?;
        let link = self.link.as_mut().ok_or(Error::Unavailable)?;
        match frame {
            Server::Ready { version } if version == api::VERSION && !link.ready => {
                link.ready = true;
                self.resync = self.ever_ready;
                self.ever_ready = true;
                if self.pending.is_none() {
                    self.deadline = None;
                }
            }
            Server::Reply { serial, result } if link.ready => {
                let work = self
                    .pending
                    .as_ref()
                    .filter(|work| work.id() == serial)
                    .ok_or(Error::Rejected)?;
                if !self.sent {
                    return Err(Error::Rejected);
                }
                if let Ok(response) = &result {
                    link.access
                        .validate(work.request().ok_or(Error::Rejected)?, response)?;
                }
                let work = self.pending.take().ok_or(Error::Rejected)?;
                self.deadline = None;
                self.sent = false;
                let (response, failure) = match result {
                    Ok(response) => (Some(response), None),
                    Err(error) => (None, Some(refusal(error))),
                };
                updates.finished = Some(Finished {
                    work,
                    response,
                    failure,
                    seat: None,
                });
            }
            Server::Changed { response } if link.ready => {
                let selector = match &response {
                    api::Response::Run { run } => api::Request::Run { run: run.id },
                    api::Response::Ranking { board } => api::Request::Ranking {
                        cohort: board.cohort,
                        lane: board.lane,
                    },
                    api::Response::Hosts { run, .. } => api::Request::Hosts {
                        run: *run,
                        before: None,
                    },
                    api::Response::Cohorts { .. } => api::Request::Cohorts { before: None },
                    _ => return Err(Error::Rejected),
                };
                link.access.validate(&selector, &response)?;
                // At most one latest snapshot per supported response class per frame.
                if let Some(old) = updates
                    .pushed
                    .iter_mut()
                    .find(|old| std::mem::discriminant(*old) == std::mem::discriminant(&response))
                {
                    *old = response;
                } else if updates.pushed.len() < 4 {
                    updates.pushed.push(response);
                }
            }
            _ => return Err(Error::Rejected),
        }
        Ok(())
    }

    fn failed(&mut self, error: Error, updates: &mut Updates) {
        self.link = None;
        self.dial = None;
        self.sent = false;
        self.deadline = None;
        self.retry_at = Some(Instant::now() + Duration::from_secs(4));
        if let Some(work) = self.pending.take() {
            updates.finished = Some(Finished {
                work,
                response: None,
                failure: Some(error),
                seat: None,
            });
        }
    }
}

fn open(mut access: TowerSocket) -> Result<Link, Error> {
    let seconds = access
        .expires_at
        .saturating_sub(time::OffsetDateTime::now_utc().unix_timestamp());
    if seconds <= 60 {
        return Err(Error::Rejected);
    }
    let (sender, receiver) = crate::transport::ws_connect_with_bearer(
        &access.url,
        std::mem::take(&mut access.authorization),
        ewebsock::Options {
            max_incoming_frame_size: api::socket::MAX_SERVER_BYTES,
            ..Default::default()
        },
    )
    .map_err(|_| Error::Unavailable)?;
    Ok(Link {
        sender,
        receiver,
        access,
        ready: false,
        renew: Instant::now() + Duration::from_secs(u64::try_from(seconds - 60).unwrap_or(0)),
    })
}

fn refusal(error: Refusal) -> Error {
    match error {
        Refusal::Terms => Error::TermsRequired,
        Refusal::Busy => Error::Busy,
        Refusal::Conflict => Error::Conflict,
        Refusal::Unavailable => Error::Unavailable,
        Refusal::Invalid | Refusal::Unauthorized | Refusal::Missing => Error::Rejected,
    }
}
