//! Coalesced polling, backoff and account-worker completions.
use super::{
    Down, Error, Flight, HttpTag, Job, OsEntropy, Reply, SocialHost, apply, http_answer, keys,
    refused, wire, worker,
};
use std::{
    collections::BTreeSet,
    sync::{Mutex, mpsc},
};
pub(in crate::social) fn tick(host: &mut SocialHost, now: u64) {
    if host.machine.is_none() {
        host.central = None;
        return;
    }
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    if !bridge.handle.is_active() {
        reject_session(host);
        return;
    }
    let arrived = bridge.flight.as_ref().and_then(|flight| {
        let Ok(receive) = flight.receive.lock() else {
            return Some(Err(Error::Unavailable));
        };
        match receive.try_recv() {
            Ok(reply) => Some(reply),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Error::Unavailable)),
        }
    });
    if let Some(result) = arrived {
        let Some(flight) = bridge.flight.take() else {
            return;
        };
        complete(host, flight.job, result, now);
    }
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    if bridge.ready && bridge.wanted && !bridge.welcomed {
        bridge.welcomed = true;
        let frame = Down::Welcome {
            tag: bridge.session.tag.clone(),
            handle: bridge.session.handle.clone(),
            device: bridge.device.clone(),
            guest: false,
        };
        if let Some(machine) = host.machine.as_mut() {
            let effects = machine.on_link(SocialLink::Up, now);
            host.carry_out(effects);
        }
        apply(host, frame);
    }
    super::gateway::register(host, now);
    sync_ws_signals(host, now);
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    if bridge.flight.is_some() {
        return;
    }
    if bridge.jobs.is_empty() {
        if !bridge.ready || !bridge.wanted || now < bridge.next {
            return;
        }
        bridge.jobs.push_back(Job::Poll {
            device: bridge.device.clone(),
            after: bridge.cursor,
            generation: bridge.generation,
            metadata: now >= bridge.metadata_at,
        });
    } else if bridge.next != u64::MAX && now < bridge.next {
        return;
    }
    let Some(job) = bridge.jobs.front().cloned() else {
        return;
    };
    let work = job.clone();
    let retries = bridge.retries.clone();
    let (send, receive) = mpsc::sync_channel(1);
    if bridge.handle.submit(move |account| {
        let _ = send.send(worker::run(account, &work, &retries));
    }) {
        bridge.jobs.pop_front();
        bridge.flight = Some(Flight {
            job,
            receive: Mutex::new(receive),
        });
    } else {
        bridge.next = now + 1000;
    }
}

fn sync_ws_signals(host: &mut SocialHost, now: u64) {
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    let chat_seq = bridge.handle.chat_seq();
    let social_seq = bridge.handle.social_seq();
    let presence_seq = bridge.handle.presence_seq();
    if presence_seq != bridge.seen_presence_seq {
        bridge.seen_presence_seq = presence_seq;
        if bridge.ready && bridge.welcomed {
            let snapshot = bridge.snapshot();
            apply(host, Down::Snapshot(snapshot));
        }
    }
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    if social_seq != bridge.seen_social_seq {
        bridge.seen_social_seq = social_seq;
        bridge.seen_chat_seq = chat_seq;
        bridge.metadata_at = 0;
        bridge.next = 0;
    } else if chat_seq != bridge.seen_chat_seq {
        bridge.seen_chat_seq = chat_seq;
        bridge.next = 0;
    } else if !bridge.handle.ws_live() && bridge.next == u64::MAX {
        bridge.next = now + 2000;
        bridge.metadata_at = now + 5000;
    }
}

fn apply_personal_snapshot(host: &mut SocialHost, snapshot: Option<wire::Snapshot>, now: u64) {
    let Some(snapshot) = snapshot else {
        return;
    };
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    bridge.personal = snapshot;
    bridge.metadata_at = if bridge.handle.ws_live() {
        u64::MAX
    } else {
        now + 5000
    };
    let merged = bridge.snapshot();
    apply(host, Down::Snapshot(merged));
}

fn complete(host: &mut SocialHost, job: Job, result: Result<Reply, Error>, now: u64) {
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    match result {
        Err(error) => failed(host, job, error, now),
        Ok(reply) => {
            bridge.backoff = 2000;
            bridge.next = 0;
            match reply {
                Reply::Connected { registration } => {
                    bridge.ready = true;
                    bridge.game_registration = Some(registration);
                    http_answer(host, HttpTag::RegisterDevice, 201, &[]);
                }
                Reply::Http {
                    tag,
                    status,
                    body,
                    snapshot,
                } => {
                    bridge.metadata_at = 0;
                    apply_personal_snapshot(host, snapshot, now);
                    http_answer(host, tag, status, &body);
                }
                Reply::Keys {
                    tags,
                    keys: fetched,
                } => keys(host, true, tags, fetched),
                Reply::Sent { req, id, at } => {
                    if let Job::Send { sealed, .. } = &job
                        && let Ok(mut cache) = bridge.retries.lock()
                    {
                        cache.remove(&sealed.hdr);
                    }
                    apply(host, Down::Sent { req, id, at });
                }
                Reply::Poll {
                    generation,
                    snapshot,
                    keys: fetched,
                    messages,
                    after,
                    more,
                } => {
                    if generation != bridge.generation || !bridge.wanted {
                        return;
                    }
                    apply_personal_snapshot(host, snapshot, now);
                    let tags = messages
                        .iter()
                        .map(|message| message.from.tag.clone())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>();
                    if !tags.is_empty() {
                        keys(host, true, tags, fetched);
                    }
                    for message in messages {
                        let Some(machine) =
                            host.machine.as_mut().filter(|machine| !machine.stopped())
                        else {
                            break;
                        };
                        let effects = machine.on_retained(message, now, &mut OsEntropy);
                        host.carry_out(effects);
                    }
                    if let Some(bridge) = host.central.as_mut()
                        && bridge.generation == generation
                        && bridge.wanted
                    {
                        bridge.cursor = after;
                        let pending_push = bridge.handle.chat_seq() != bridge.seen_chat_seq
                            || bridge.handle.social_seq() != bridge.seen_social_seq;
                        bridge.next = if more || pending_push {
                            0
                        } else if bridge.handle.ws_live() {
                            u64::MAX
                        } else {
                            now + 2000
                        };
                    }
                }
            }
        }
    }
}

fn failed(host: &mut SocialHost, job: Job, error: Error, now: u64) {
    let Some(bridge) = host.central.as_mut() else {
        return;
    };

    bridge.next = now + bridge.backoff;
    bridge.backoff = (bridge.backoff * 2).min(30_000);
    match job {
        job @ Job::Connect { .. }
            if matches!(
                error,
                Error::Unavailable | Error::Busy | Error::TermsRequired
            ) =>
        {
            bridge.jobs.push_front(job);
        }
        Job::Connect { .. } => {
            http_answer(host, HttpTag::RegisterDevice, worker::status(error), &[]);
        }
        Job::Http { tag, .. } => http_answer(host, tag, worker::status(error), &[]),
        Job::Send { req, sealed, .. } => {
            // These refusals end the core request. A manual retry creates a
            // new message ID, so retaining this ciphertext would leak a slot.
            // Busy/unavailable requests still need their exact sealed retry.
            if matches!(error, Error::Rejected | Error::TermsRequired)
                && let Ok(mut retries) = bridge.retries.lock()
            {
                retries.remove(&sealed.hdr);
            }
            let why = match error {
                Error::Conflict => wire::Why::Devices,
                // Keep the core's request ID and the worker's exact ciphertext,
                // while allowing directory/inbox jobs between bounded retries.
                Error::Busy | Error::Unavailable => wire::Why::Rate,
                // Terminal: a client too old gets no further by trying again.
                Error::Rejected | Error::TermsRequired | Error::UpdateRequired => {
                    wire::Why::NotMember
                }
            };
            refused(host, req, why);
        }
        Job::Poll { .. } if error == Error::Rejected => reject_session(host),
        Job::Poll { .. } => {}
    }
}

use baylee_client_core::social::SocialLink;

fn reject_session(host: &mut SocialHost) {
    let session = host.session.clone();
    host.end();
    host.session = session;
    host.vault.reject_transport();
}

#[cfg(test)]
mod tests;
