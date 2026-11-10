//! Bounded account-worker jobs. No HTTP, key derivation or waits on the render thread.
use super::{http, mapping};
use baylee_account::{Account, Error, chat_api as chat};
use baylee_client_core::social::{HttpTag, Method, RetainedMessage};
use baylee_protocol::social as wire;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

pub(super) type Retries = Arc<Mutex<BTreeMap<String, chat::Sealed>>>;
#[derive(Clone)]
pub(super) enum Job {
    Connect {
        own: String,
        device: String,
    },
    Http {
        own: String,
        tag: HttpTag,
        method: Method,
        path: String,
        body: Option<Value>,
    },
    Send {
        req: u32,
        conv: String,
        tags: Vec<String>,
        sealed: wire::Sealed,
    },
    Poll {
        device: String,
        after: u64,
        generation: u64,
        metadata: bool,
    },
}
pub(super) enum Reply {
    Connected {
        registration: Value,
    },
    Http {
        tag: HttpTag,
        status: u16,
        body: Vec<u8>,
        snapshot: Option<wire::Snapshot>,
    },
    Keys {
        tags: Vec<String>,
        keys: Vec<wire::AccountKeys>,
    },
    Sent {
        req: u32,
        id: String,
        at: u64,
    },
    Poll {
        generation: u64,
        snapshot: Option<wire::Snapshot>,
        keys: Vec<wire::AccountKeys>,
        messages: Vec<RetainedMessage>,
        after: u64,
        more: bool,
    },
}
pub(super) fn run(account: &Account, job: &Job, retries: &Retries) -> Result<Reply, Error> {
    match job {
        Job::Connect { own, device } => connect(account, own, device),
        Job::Http {
            own,
            tag,
            method,
            path,
            body,
        } => {
            if let HttpTag::Keys(tags) = tag {
                return Ok(Reply::Keys {
                    tags: tags.clone(),
                    keys: http::keys(account, tags)?,
                });
            }
            let (status, body) = http::run(account, own, tag, *method, path, body.as_ref())?;
            let snapshot = matches!(
                tag,
                HttpTag::AddFriend(_) | HttpTag::Friendship(_) | HttpTag::Group
            )
            .then(|| mapping::snapshot(account).ok())
            .flatten();
            Ok(Reply::Http {
                tag: tag.clone(),
                status,
                body: serde_json::to_vec(&body).map_err(|_| Error::Rejected)?,
                snapshot,
            })
        }
        Job::Send {
            req,
            conv,
            tags,
            sealed,
        } => send(account, retries, *req, conv, tags, sealed),
        Job::Poll {
            device,
            after,
            generation,
            metadata,
        } => poll(account, device, *after, *generation, *metadata),
    }
}
pub(super) fn status(error: Error) -> u16 {
    match error {
        Error::Rejected => 403,
        Error::Conflict => 409,
        Error::Busy => 429,
        Error::UpdateRequired => 426,
        Error::TermsRequired => 451,
        Error::Unavailable => 0,
    }
}

fn connect(account: &Account, own: &str, device: &str) -> Result<Reply, Error> {
    let secrets = account.chat_identity()?;
    let keys = baylee_seal::DeviceSecrets::from_bytes(secrets.bytes());
    if keys.device_id() != device {
        return Err(Error::Rejected);
    }
    let public = keys.public(own).map_err(|_| Error::Rejected)?;
    let registration = chat::DeviceRegistration {
        ik: baylee_seal::b64::url::encode(&public.ik),
        dk: baylee_seal::b64::url::encode(&public.dk),
        dsig: baylee_seal::b64::url::encode(&public.dsig),
        label: "Account".into(),
    };
    match http::chat(
        account,
        chat::ChatRequest::Register {
            device: registration.clone(),
        },
    )? {
        chat::ChatResponse::Registered { id } if id == *device => Ok(Reply::Connected {
            registration: serde_json::to_value(registration).map_err(|_| Error::Rejected)?,
        }),
        _ => Err(Error::Rejected),
    }
}

fn send(
    account: &Account,
    retries: &Retries,
    req: u32,
    conv: &str,
    tags: &[String],
    sealed: &wire::Sealed,
) -> Result<Reply, Error> {
    let key = sealed.hdr.clone();
    let previous = retries
        .lock()
        .map_err(|_| Error::Unavailable)?
        .get(&key)
        .cloned();
    let prepared = if let Some(previous) = previous {
        previous
    } else {
        let keys = http::keys(account, tags)?;
        if keys.len() != tags.len()
            || tags
                .iter()
                .any(|tag| !keys.iter().any(|key| key.tag == *tag))
        {
            return Err(Error::Conflict);
        }
        super::directory::validate_wraps(sealed, keys.iter())?;
        let selected: chat::Sealed = serde_json::to_value(sealed)
            .and_then(serde_json::from_value)
            .map_err(|_| Error::Rejected)?;
        let mut cache = retries.lock().map_err(|_| Error::Unavailable)?;
        if cache.len() >= 64 {
            return Err(Error::Busy);
        }
        cache.insert(key.clone(), selected.clone());
        selected
    };
    let header: wire::Header = serde_json::from_slice(
        &baylee_seal::b64::url::decode(&prepared.hdr).map_err(|_| Error::Rejected)?,
    )
    .map_err(|_| Error::Rejected)?;
    let response = http::chat(
        account,
        chat::ChatRequest::Send {
            conv: conv.to_owned(),
            sealed: prepared,
        },
    );
    if matches!(response, Err(Error::Conflict)) {
        // A definite refusal committed no new ciphertext. The core may fetch and
        // confirm the new membership/key set before producing a replacement.
        retries.lock().map_err(|_| Error::Unavailable)?.remove(&key);
    }
    match response? {
        chat::ChatResponse::Sent { id, at }
            if id == header.id && header.ts.abs_diff(at) <= 600_000 =>
        {
            // Kept until the main thread applies the receipt. A network retry
            // always uses identical ciphertext, nonce and recipient wraps.
            Ok(Reply::Sent { req, id, at })
        }
        _ => Err(Error::Rejected),
    }
}

fn poll(
    account: &Account,
    device: &str,
    after: u64,
    generation: u64,
    metadata: bool,
) -> Result<Reply, Error> {
    let snapshot = if metadata {
        Some(mapping::snapshot(account)?)
    } else {
        None
    };
    let chat::ChatResponse::Inbox { messages, more } = http::chat(
        account,
        chat::ChatRequest::Inbox {
            device: device.to_owned(),
            after,
        },
    )?
    else {
        return Err(Error::Rejected);
    };
    if messages.len() > chat::PAGE_SIZE || (more && messages.len() != chat::PAGE_SIZE) {
        return Err(Error::Rejected);
    }
    let mut cursor = after;
    let mut tags = BTreeSet::new();
    let mut deliveries = Vec::new();
    for message in messages {
        if message.seq <= cursor {
            return Err(Error::Rejected);
        }
        cursor = message.seq;
        tags.insert(format!("{:04x}", message.sender.tag));
        deliveries.push(mapping::delivery(message, device)?);
    }
    let keys = if tags.is_empty() {
        Vec::new()
    } else {
        http::keys(account, &tags.into_iter().collect::<Vec<_>>())?
    };
    Ok(Reply::Poll {
        generation,
        snapshot,
        keys,
        messages: deliveries,
        after: cursor,
        more,
    })
}
