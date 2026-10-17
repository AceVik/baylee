//! `Up::Send` and the deliveries it makes (§8.2).
//!
//! The checks run in the design's order: the shapes and sizes, the header
//! (sender, device, conversation, id, clock), membership, the send
//! allowance, then completeness (the wraps name exactly the current devices
//! of every member but the sending one, blocked recipients included), the
//! per-device cap, a duplicate id, and storage. The gateway never opens
//! anything it carries: `hdr` is the only part it parses, and only for the
//! fields it checks.

use super::{members, millis, now_millis, person, uid};
use crate::Shared;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use baylee_db::social::{NewMessage, NewSlot, Stored};
use baylee_protocol::social::{
    self as wire, Conv, Delivered, Down, HEADER_VERSION, Header, Sealed, Sender, Why,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The sealed message as stored: everything but the wraps.
#[derive(Serialize, Deserialize)]
pub(crate) struct Kept {
    hdr: String,
    n: String,
    ct: String,
    epk: String,
    sig: String,
}

/// Who is sending, as the socket knows it.
pub(crate) struct Sending<'a> {
    pub(crate) account: &'a str,
    pub(crate) tag: &'a str,
    pub(crate) handle: &'a str,
    pub(crate) device: &'a str,
    pub(crate) guest: bool,
}

/// A refusal of send `req`.
fn refused(req: u32, why: Why) -> Down {
    Down::Refused {
        req: Some(req),
        why,
        missing: Vec::new(),
        extra: Vec::new(),
        message: None,
    }
}

/// `field` decoded, if it is base64url without padding of at most `max`
/// bytes (exactly `max` when `exact`).
fn bytes(field: &str, max: usize, exact: bool) -> Result<Vec<u8>, Why> {
    // Four characters carry three bytes: a longer field is too large before
    // it is decoded.
    if field.len() > max.div_ceil(3) * 4 {
        return Err(if exact { Why::BadSealed } else { Why::TooLarge });
    }
    let decoded = URL_SAFE_NO_PAD.decode(field).map_err(|_| Why::BadSealed)?;
    if (exact && decoded.len() != max) || decoded.len() > max {
        return Err(Why::BadSealed);
    }
    Ok(decoded)
}

/// The shapes of `sealed`, and its header as the gateway reads it.
/// A send's wraps, decoded: `(device, wrap)`.
type Wraps = Vec<(String, Vec<u8>)>;

fn shapes(sealed: &Sealed) -> Result<(Header, Wraps), Why> {
    let hdr = bytes(&sealed.hdr, wire::MAX_HEADER_BYTES, false)?;
    bytes(&sealed.n, 24, true)?;
    // The body's ciphertext: the plaintext bound and the 16-byte tag.
    let ct = bytes(&sealed.ct, wire::MAX_BODY_BYTES + 16, false)?;
    if ct.len() < 16 {
        return Err(Why::BadSealed);
    }
    bytes(&sealed.epk, 32, true)?;
    bytes(&sealed.sig, 64, true)?;
    let mut wraps = Vec::with_capacity(sealed.wraps.len());
    let mut seen = BTreeSet::new();
    if sealed.wraps.len() > wire::MAX_GROUP_MEMBERS * wire::MAX_DEVICES {
        return Err(Why::TooLarge);
    }
    for wrap in &sealed.wraps {
        if !wire::valid_device_id(&wrap.d) || !seen.insert(wrap.d.clone()) {
            return Err(Why::BadSealed);
        }
        wraps.push((wrap.d.clone(), bytes(&wrap.k, wire::WRAP_BYTES, true)?));
    }
    let header: Header = serde_json::from_slice(&hdr).map_err(|_| Why::BadSealed)?;
    Ok((header, wraps))
}

/// Whether `id` is a `UUIDv7`, as the header's id must be.
fn uuid_v7(id: &str) -> Option<uuid::Uuid> {
    let parsed = uuid::Uuid::parse_str(id).ok()?;
    (parsed.get_version_num() == 7 && parsed.to_string() == id).then_some(parsed)
}

/// Handles one `Up::Send`, answering `Sent` or `Refused`.
pub(crate) async fn send(
    state: &Shared,
    from: &Sending<'_>,
    req: u32,
    conv_text: &str,
    sealed: Sealed,
) -> Down {
    match try_send(state, from, req, conv_text, sealed).await {
        Ok(frame) => frame,
        Err(e) => {
            tracing::error!("social: a send failed: {e:#}");
            refused(req, Why::BadSealed)
        }
    }
}

/// Handles one `Up::Global` public lobby chat send, answering `Sent` or
/// `Refused` and broadcasting `Down::Global` to all other open social sockets.
pub(crate) fn send_global(
    state: &Shared,
    from: &Sending<'_>,
    req: u32,
    id: &str,
    body: String,
) -> Down {
    if uuid_v7(id).is_none() {
        return refused(req, Why::BadSealed);
    }
    if body.is_empty() || body.len() > wire::MAX_BODY_BYTES {
        return refused(req, Why::TooLarge);
    }
    if !state
        .social
        .admit_send(from.account, from.guest, std::time::Instant::now())
    {
        return refused(req, Why::Rate);
    }
    let at = now_millis();
    let frame = Down::Global {
        id: id.to_owned(),
        at,
        from: Sender {
            tag: from.tag.to_owned(),
            handle: from.handle.to_owned(),
            device: from.device.to_owned(),
        },
        body,
    };
    state.social.broadcast_global(from.device, &frame);
    Down::Sent {
        req,
        id: id.to_owned(),
        at,
    }
}

#[allow(clippy::too_many_lines)] // the checks, in their order, read top to bottom
async fn try_send(
    state: &Shared,
    from: &Sending<'_>,
    req: u32,
    conv_text: &str,
    sealed: Sealed,
) -> anyhow::Result<Down> {
    let refuse = |why: Why| {
        tracing::debug!(?why, "social: a send was refused");
        Ok(refused(req, why))
    };
    let (header, wraps) = match shapes(&sealed) {
        Ok(read) => read,
        Err(why) => return refuse(why),
    };
    let Ok(conv) = conv_text.parse::<Conv>() else {
        return refuse(Why::BadSealed);
    };
    if header.v != HEADER_VERSION
        || header.from != from.tag
        || header.dev != from.device
        || header.conv != conv_text
        || header.kind != "text"
    {
        return refuse(Why::BadSealed);
    }
    let Some(id) = uuid_v7(&header.id) else {
        return refuse(Why::BadSealed);
    };
    if header.ts.abs_diff(now_millis()) > wire::MAX_CLOCK_SKEW_MS {
        return refuse(Why::Stale);
    }
    let Some(members) = members::members(state, from.account, from.tag, &conv).await? else {
        return refuse(Why::NotMember);
    };
    if !state
        .social
        .admit_send(from.account, from.guest, std::time::Instant::now())
    {
        return refuse(Why::Rate);
    }
    let ids: Vec<uuid::Uuid> = members.iter().filter_map(|m| uid(m)).collect();
    let devices = baylee_db::social::devices_of(&state.db, &ids).await?;
    let expected: BTreeSet<&str> = devices
        .iter()
        .map(|d| d.id.as_str())
        .filter(|d| *d != from.device)
        .collect();
    let named: BTreeSet<&str> = wraps.iter().map(|(d, _)| d.as_str()).collect();
    if expected != named {
        tracing::debug!(
            missing = expected.difference(&named).count(),
            extra = named.difference(&expected).count(),
            "social: a send named other devices"
        );
        return Ok(Down::Refused {
            req: Some(req),
            why: Why::Devices,
            missing: expected
                .difference(&named)
                .map(|d| (*d).to_owned())
                .collect(),
            extra: named
                .difference(&expected)
                .map(|d| (*d).to_owned())
                .collect(),
            message: None,
        });
    }
    let Some(sender) = uid(from.account) else {
        return refuse(Why::NotMember);
    };
    let blockers: BTreeSet<uuid::Uuid> = baylee_db::social::blockers_of(&state.db, sender, &ids)
        .await?
        .into_iter()
        .collect();
    let guests: BTreeSet<uuid::Uuid> = baylee_db::social::who_by_ids(&state.db, &ids)
        .await?
        .into_iter()
        .filter(|who| who.guest)
        .map(|who| who.id)
        .collect();
    let owner: BTreeMap<&str, uuid::Uuid> = devices
        .iter()
        .map(|d| (d.id.as_str(), d.account_id))
        .collect();
    let now = time::OffsetDateTime::now_utc();
    let slots: Vec<NewSlot> = wraps
        .into_iter()
        .filter_map(|(device, wrap)| {
            let account = *owner.get(device.as_str())?;
            if blockers.contains(&account) {
                return None;
            }
            let days = state.social.keep_days_for(guests.contains(&account));
            Some(NewSlot {
                device,
                wrap,
                expires_at: now + time::Duration::days(i64::from(days)),
            })
        })
        .collect();
    let kept = serde_json::to_vec(&Kept {
        hdr: sealed.hdr.clone(),
        n: sealed.n.clone(),
        ct: sealed.ct.clone(),
        epk: sealed.epk.clone(),
        sig: sealed.sig.clone(),
    })?;
    let message = NewMessage {
        id,
        conv: conv_text.to_owned(),
        sender,
        sender_dev: from.device.to_owned(),
        sealed: kept,
    };
    let (at, stored) =
        match baylee_db::social::store_message(&state.db, &message, &slots, wire::SLOTS_PER_DEVICE)
            .await?
        {
            Stored::Done { at, slots } => (at, slots),
            Stored::Duplicate => return refuse(Why::Duplicate),
            Stored::Full => return refuse(Why::RecipientFull),
        };
    let at = millis(at);
    let wraps_by_device: BTreeMap<&str, &str> = sealed
        .wraps
        .iter()
        .map(|w| (w.d.as_str(), w.k.as_str()))
        .collect();
    for (device, seq) in stored {
        let Some(k) = wraps_by_device.get(device.as_str()) else {
            continue;
        };
        state.social.deliver(
            &device,
            seq,
            Down::Msg {
                seq: u64::try_from(seq).unwrap_or(0),
                at,
                conv: conv_text.to_owned(),
                from: Sender {
                    tag: from.tag.to_owned(),
                    handle: from.handle.to_owned(),
                    device: from.device.to_owned(),
                },
                sealed: Delivered {
                    hdr: sealed.hdr.clone(),
                    n: sealed.n.clone(),
                    ct: sealed.ct.clone(),
                    epk: sealed.epk.clone(),
                    sig: sealed.sig.clone(),
                    k: (*k).to_owned(),
                },
            },
        );
    }
    Ok(Down::Sent {
        req,
        id: header.id,
        at,
    })
}

/// A pending slot as the `Down::Msg` that delivers it.
pub(crate) fn delivery(pending: &baylee_db::social::Pending) -> Option<Down> {
    let kept: Kept = serde_json::from_slice(&pending.sealed).ok()?;
    let sender = person(&pending.sender);
    Some(Down::Msg {
        seq: u64::try_from(pending.seq).ok()?,
        at: millis(pending.at),
        conv: pending.conv.clone(),
        from: Sender {
            tag: sender.tag,
            handle: sender.handle,
            device: pending.sender_dev.clone(),
        },
        sealed: Delivered {
            hdr: kept.hdr,
            n: kept.n,
            ct: kept.ct,
            epk: kept.epk,
            sig: kept.sig,
            k: URL_SAFE_NO_PAD.encode(&pending.wrap),
        },
    })
}

/// One socket's deliveries in flight: sent and not yet acked, at most
/// [`wire::DELIVERY_WINDOW`]. A slot is deleted only once the socket sent
/// it and the client acked it, so a slot stored out of order is never
/// deleted unsent.
#[derive(Default)]
pub(crate) struct Window {
    pub(crate) inflight: BTreeSet<i64>,
    /// Slots wait in the database that were not sent (the window was full,
    /// or the socket just opened).
    pub(crate) behind: bool,
}

impl Window {
    /// Whether a live delivery of `seq` goes out now.
    pub(crate) fn take(&mut self, seq: i64) -> bool {
        if self.inflight.contains(&seq) {
            return false;
        }
        if self.inflight.len() >= wire::DELIVERY_WINDOW {
            self.behind = true;
            return false;
        }
        self.inflight.insert(seq);
        true
    }

    /// The slots up to `upto` that were sent, now acked.
    pub(crate) fn acked(&mut self, upto: u64) -> Vec<i64> {
        let upto = i64::try_from(upto).unwrap_or(i64::MAX);
        let acked: Vec<i64> = self.inflight.range(..=upto).copied().collect();
        for seq in &acked {
            self.inflight.remove(seq);
        }
        acked
    }
}

/// Fills `window` from the database: the deliveries to send now.
///
/// # Errors
///
/// When the database fails.
pub(crate) async fn pull(
    db: &sea_orm::DatabaseConnection,
    device: &str,
    window: &mut Window,
) -> anyhow::Result<Vec<Down>> {
    let room = wire::DELIVERY_WINDOW.saturating_sub(window.inflight.len());
    if room == 0 {
        window.behind = true;
        return Ok(Vec::new());
    }
    let skip: Vec<i64> = window.inflight.iter().copied().collect();
    let pending = baylee_db::social::pending(db, device, &skip, room).await?;
    window.behind = pending.len() >= room;
    let mut frames = Vec::with_capacity(pending.len());
    for slot in &pending {
        if let Some(frame) = delivery(slot) {
            window.inflight.insert(slot.seq);
            frames.push(frame);
        }
    }
    Ok(frames)
}
