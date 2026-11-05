//! Account-owned personal chat beside the gateway's game-specific transport.
use super::{OsEntropy, Session, SocialHost, now_ms, transport};
use crate::lobby::central::Handle;
use baylee_account::Error;
use baylee_client_core::social::{HttpTag, Method};
use baylee_protocol::social::{self as wire, Down, Up};
use std::{
    collections::{BTreeSet, VecDeque},
    sync::{Mutex, mpsc},
};
use worker::{Job, Reply};

mod directory;
mod gateway;
mod http;
mod mapping;
mod poll;
mod retries;
mod worker;
pub(super) use gateway::{gateway_http, gateway_news};
pub(super) use poll::tick;

struct Flight {
    job: Job,
    receive: Mutex<mpsc::Receiver<Result<Reply, Error>>>,
}
pub(super) struct Bridge {
    handle: Handle,
    session: Session,
    device: String,
    jobs: VecDeque<Job>,
    flight: Option<Flight>,
    retries: worker::Retries,
    personal: wire::Snapshot,
    game: wire::Snapshot,
    hello: Option<Up>,
    game_requests: BTreeSet<u32>,
    game_registration: Option<serde_json::Value>,
    game_state: gateway::Registration,
    ready: bool,
    wanted: bool,
    welcomed: bool,
    cursor: u64,
    generation: u64,
    next: u64,
    metadata_at: u64,
    backoff: u64,
}
impl Bridge {
    fn new(handle: Handle, session: Session, device: String) -> Self {
        let jobs = VecDeque::from([Job::Connect {
            own: session.tag.clone(),
            device: device.clone(),
        }]);
        Self {
            handle,
            session,
            device,
            jobs,
            flight: None,
            retries: worker::Retries::default(),
            personal: wire::Snapshot::default(),
            game: wire::Snapshot::default(),
            hello: None,
            game_requests: BTreeSet::new(),
            game_registration: None,
            game_state: gateway::Registration::Waiting(0),
            ready: false,
            wanted: true,
            welcomed: false,
            cursor: 0,
            generation: 0,
            next: 0,
            metadata_at: 0,
            backoff: 2000,
        }
    }
    pub(super) fn game_registered(&self) -> bool {
        matches!(self.game_state, gateway::Registration::Ready)
    }
    pub(super) fn resume(&mut self) {
        if !self.wanted {
            self.generation = self.generation.wrapping_add(1);
            self.cursor = 0;
        }
        self.wanted = true;
        self.welcomed = false;
        self.next = 0;
    }
    pub(super) fn pause(&mut self) {
        self.wanted = false;
        self.welcomed = false;
        self.generation = self.generation.wrapping_add(1);
        self.cursor = 0;
        self.jobs.retain(|job| !matches!(job, Job::Poll { .. }));
    }
    fn queue(&mut self, job: Job) -> bool {
        if self.jobs.len() >= 16 {
            return false;
        }
        self.jobs.push_back(job);
        true
    }
    fn snapshot(&self) -> wire::Snapshot {
        let mut snapshot = self.personal.clone();
        snapshot.tables.clone_from(&self.game.tables);
        snapshot.invites.clone_from(&self.game.invites);
        snapshot.settings = self.game.settings;
        for friend in &mut snapshot.friends {
            if let Some(peer) = self
                .game
                .friends
                .iter()
                .find(|peer| peer.person.tag == friend.person.tag)
            {
                friend.presence.clone_from(&peer.presence);
            }
        }
        snapshot
    }
    fn members(&self, conv: &str) -> Option<Vec<String>> {
        match conv.parse::<wire::Conv>().ok()? {
            wire::Conv::Dm { a, b } if a == self.session.tag || b == self.session.tag => {
                Some(vec![a, b])
            }
            wire::Conv::Group { id } => self
                .personal
                .groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| {
                    group
                        .members
                        .iter()
                        .map(|member| member.person.tag.clone())
                        .collect()
                }),
            wire::Conv::Table { .. } | wire::Conv::Team { .. } => self
                .game
                .tables
                .iter()
                .find(|table| table.conv == conv)
                .map(|table| {
                    table
                        .members
                        .iter()
                        .map(|member| member.tag.clone())
                        .collect()
                }),
            _ => None,
        }
    }
}

pub(super) fn attach(host: &mut SocialHost, handle: Option<Handle>) {
    let (Some(handle), Some(session), Some(machine)) =
        (handle, host.session.clone(), host.machine.as_mut())
    else {
        return;
    };
    machine.use_account_transport();
    host.central = Some(Bridge::new(handle, session, machine.device_id().into()));
}

pub(super) fn rebind_session(host: &mut SocialHost, session: Session) {
    if let Some(bridge) = host.central.as_mut() {
        bridge.session = session;
        bridge.game = wire::Snapshot::default();
        bridge.game_requests.clear();
        bridge.game_state = gateway::Registration::Waiting(0);
    }
}

pub(super) fn request(
    host: &mut SocialHost,
    tag: &HttpTag,
    method: Method,
    path: &str,
    body: Option<&serde_json::Value>,
) -> bool {
    let Some(bridge) = host.central.as_mut() else {
        return false;
    };
    if !http::handles(tag) {
        return false;
    }
    if matches!(tag, HttpTag::RegisterDevice) {
        return true;
    }
    let queued = bridge.queue(Job::Http {
        own: bridge.session.tag.clone(),
        tag: tag.clone(),
        method,
        path: path.into(),
        body: body.cloned(),
    });
    if !queued {
        http_answer(host, tag.clone(), 429, &[]);
    }
    true
}

pub(super) fn up(host: &mut SocialHost, up: &Up) -> bool {
    let Some(bridge) = host.central.as_mut() else {
        return false;
    };
    match up {
        Up::Hello { .. } => {
            bridge.hello = Some(up.clone());
            true
        }
        Up::Send { req, conv, sealed } if personal(conv) => {
            if let Some(machine) = host.machine.as_ref() {
                retries::retain_pending(&bridge.retries, machine);
            }
            let Some(tags) = bridge.members(conv) else {
                refused(host, *req, wire::Why::NotMember);
                return true;
            };
            let job = Job::Send {
                req: *req,
                conv: conv.clone(),
                tags,
                sealed: sealed.clone(),
            };
            if !bridge.queue(job) {
                refused(host, *req, wire::Why::Rate);
            }
            true
        }
        Up::Send { req, conv, sealed } => {
            if bridge.game_requests.len() >= 64 {
                refused(host, *req, wire::Why::Rate);
                return true;
            }
            bridge.game_requests.insert(*req);
            if !serde_json::to_string(&Up::Send {
                req: *req,
                conv: conv.clone(),
                sealed: sealed.clone(),
            })
            .is_ok_and(|text| host.socket.send(text))
            {
                bridge.game_requests.remove(req);
                refused(host, *req, wire::Why::Off);
            }
            true
        }
        Up::Global { req, .. } => {
            if bridge.game_requests.len() >= 64 {
                refused(host, *req, wire::Why::Rate);
                return true;
            }
            bridge.game_requests.insert(*req);
            if !serde_json::to_string(up).is_ok_and(|text| host.socket.send(text)) {
                bridge.game_requests.remove(req);
                refused(host, *req, wire::Why::Off);
            }
            true
        }
        Up::Presence(presence) => {
            if let Some(Up::Hello { presence: old, .. }) = &mut bridge.hello {
                old.clone_from(presence);
            }
            false
        }
        Up::Ack { .. } => false,
    }
}
fn personal(conv: &str) -> bool {
    matches!(
        conv.parse::<wire::Conv>(),
        Ok(wire::Conv::Dm { .. } | wire::Conv::Group { .. })
    )
}
fn apply(host: &mut SocialHost, frame: Down) {
    if let Some(machine) = host.machine.as_mut() {
        let effects = machine.on_frame(frame, now_ms(), &mut OsEntropy);
        host.carry_out(effects);
    }
}
fn http_answer(host: &mut SocialHost, tag: HttpTag, status: u16, body: &[u8]) {
    if let Some(machine) = host.machine.as_mut() {
        let effects = machine.on_http(tag, status, body, now_ms(), &mut OsEntropy);
        host.carry_out(effects);
    }
}
fn refused(host: &mut SocialHost, req: u32, why: wire::Why) {
    apply(
        host,
        Down::Refused {
            req: Some(req),
            why,
            missing: vec![],
            extra: vec![],
            message: None,
        },
    );
}
fn keys(host: &mut SocialHost, central: bool, tags: Vec<String>, keys: Vec<wire::AccountKeys>) {
    let tag = if central {
        HttpTag::Keys(tags)
    } else {
        HttpTag::GameKeys(tags)
    };
    match serde_json::to_vec(&wire::KeysAnswer { keys }) {
        Ok(body) => http_answer(host, tag, 200, &body),
        Err(_) => http_answer(host, tag, 403, &[]),
    }
}

#[cfg(test)]
mod interop;

#[cfg(test)]
mod tests;
