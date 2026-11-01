//! The social machine (§10.2): every input in, every [`Effect`] out.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use baylee_protocol::social::{
    Conv, Delivered, DeviceList, Down, FriendEvent, GroupEvent, KeysAnswer, MAX_KEY_TAGS, Notice,
    Person, PresenceSet, Sender, Shown, Status, TableInvite, Up, Why,
};
use baylee_seal::b64::url;
use baylee_seal::{DeviceSecrets, Entropy, RestKind, at_rest_open, at_rest_seal};

use crate::bugreport::refs::{Candidates, CardRef, PlayerRef};

use super::commands::{GroupRef, Target};
use super::compose::{ComposeContext, Composer, ComposerKey, ComposerOutcome, LinkedCard};
use super::contacts::{Contacts, FriendsView, HeldInvite};
use super::convs::{Body, ChatLine, ConvId, ConvKind, Conversation, LineKey, LineKind, LineState};
use super::disk::{ChatDisk, STATE_FILE};
use super::history::{self, Evidence, Record};
use super::keys::{ContactKeys, KeyCache, KeyDiff};
use super::notify::{self, ChatCue, NotifyContext};
use super::sealing::{self, Decoded, MAX_AGE_MS, OpenFault, SealFault};
use super::table::TableContext;
use super::vault::{self, KeyHome, SecretSlot};
use super::words::ChatPhrase;
use super::{
    ChatPrefs, ChatToast, DeviceRow, Effect, HttpTag, Intent, Me, Method, NoteState, SafetyView,
    SecretValue, SocialLink, ToastAbout, ToastButton, device_label,
};

/// Idle time after which an online player is shown away (§6).
pub const AWAY_AFTER_MS: u64 = 10 * 60 * 1000;
/// How long a table conversation still takes lines after it ended (§4.2).
pub const ENDED_GRACE_MS: u64 = 60 * 60 * 1000;
/// The wait before a send refused for its rate goes again (§8.6).
pub const RATE_RETRY_MS: u64 = 1000;
/// How often a send refused for its devices is sealed again (§10.2).
pub const MAX_RESEALS: u8 = 2;
/// How many recent players are remembered (§13).
pub const RECENT_KEPT: usize = 30;
/// The wait before a refused registration or key fetch is tried again.
pub const BACKOFF_MS: u64 = 60 * 1000;
/// Bound ciphertext retained while a player has not confirmed a changed identity.
pub const MAX_HELD_DELIVERIES: usize = 256;
/// Aggregate retained encoded ciphertext/metadata budget, independent of count.
pub const MAX_HELD_BYTES: usize = 4 * 1024 * 1024;

const STATE_VERSION: u32 = 1;

mod archive;
pub use archive::ArchiveError;

/// What the state file holds (§10.2): no secret, only public keys, ids,
/// tags and preferences. Sealed under the vault key before it leaves.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct StoredState {
    v: u32,
    device_id: String,
    registered: bool,
    keys: KeyCache,
    /// Message ids seen, with when they were seen; kept 30 days.
    seen: BTreeMap<String, u64>,
    nicknames: BTreeMap<String, String>,
    ignored: BTreeSet<String>,
    muted: BTreeSet<String>,
    /// Conversations with history, so their logs are found at start.
    convs: Vec<StoredConv>,
    recent: Vec<RecentPlayer>,
    /// Whisper partners, most recent first (for `/r` and name matching).
    whisperers: Vec<Person>,
    last_whisper: Option<String>,
    /// Own device ids as last seen, for the new-device notice.
    own_devices: Vec<String>,
    status: Option<Status>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredConv {
    id: String,
    title: String,
}

/// A player sat with recently (§13): offered for Add friend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentPlayer {
    /// The tag.
    pub tag: String,
    /// The handle.
    pub handle: String,
    /// When, Unix milliseconds.
    pub at: u64,
}

/// A send not yet answered.
#[derive(Clone, Debug)]
struct Pending {
    req: u32,
    key: LineKey,
    hdr: Vec<u8>,
    body: Body,
    reseals: u8,
    wait: Wait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wait {
    /// For a member's keys, or to be sealed.
    Keys,
    /// A known peer's changed public keys require explicit safety confirmation.
    Verification,
    /// For the socket.
    Link,
    /// For `Sent` or `Refused`.
    Answer,
    /// Refused for its rate: again at this time.
    RetryAt(u64),
}

/// A delivery waiting for its sender's keys.
#[derive(Clone, Debug)]
struct Held {
    seq: u64,
    at: u64,
    conv: String,
    from: Sender,
    sealed: Delivered,
}

impl Held {
    fn bytes(&self) -> usize {
        [
            &self.conv,
            &self.from.tag,
            &self.from.handle,
            &self.from.device,
            &self.sealed.hdr,
            &self.sealed.n,
            &self.sealed.ct,
            &self.sealed.epk,
            &self.sealed.sig,
            &self.sealed.k,
        ]
        .iter()
        .fold(0_usize, |total, field| total.saturating_add(field.len()))
    }
}

/// The link's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    Down,
    Up,
    Welcomed,
    /// Refused for good (version, chat off): no more dialling.
    Stopped,
}

/// The social machine (§10.2).
#[allow(clippy::struct_excessive_bools)]
pub struct Social {
    me: Me,
    device: DeviceSecrets,
    state: StoredState,
    prefs: ChatPrefs,
    key_home: KeyHome,
    keep_days: u32,
    contacts: Contacts,
    convs: Vec<Conversation>,
    link: Link,
    status: Status,
    auto_away: bool,
    activity: Option<baylee_protocol::social::Activity>,
    outbox: Vec<Pending>,
    next_req: u32,
    next_local: u64,
    held: Vec<Held>,
    /// Inbox backpressure: resume only after explicit key confirmation, without acknowledging gaps.
    held_paused: bool,
    /// Tags whose keys are being fetched.
    fetching: BTreeSet<String>,
    /// Tags fetched once for a held delivery: a second miss drops it.
    refetched: BTreeSet<String>,
    acked: u64,
    high: u64,
    retry_register_at: Option<u64>,
    registering: bool,
    table: Option<TableContext>,
    composer: Composer,
    card_candidates: Vec<CardRef>,
    active: Option<ConvId>,
    panel_open: bool,
    focused: bool,
    revision: u64,
    own_devices: Vec<DeviceRow>,
    sent_invites: BTreeMap<String, Person>,
    leave_armed: Option<String>,
    last_group: Option<String>,
    /// Tags whose cached keys a refusal showed out of date: fetched again
    /// before the next seal.
    stale: BTreeSet<String>,
    /// Effects made where no output was at hand, sent with the next batch.
    queued: Vec<Effect>,
    /// The latest time an input carried.
    last_now: u64,
    /// What a report of each received line needs (§4.7), in memory.
    evidence: BTreeMap<LineKey, Option<Evidence>>,
    dirty: bool,
    removed: bool,
    /// Own-account recipients authenticated by Central, never by the gateway's
    /// directory. Absence retains the legacy TOFU/device-confirmation policy.
    trusted_own_devices: Option<Vec<([u8; 32], [u8; 32])>>,
}

impl Social {
    /// After sign-in on a gateway whose `/info` says social. `stored` is
    /// the state file's sealed bytes and `secrets` the store's value; both
    /// `None` on a new device. A new device draws its secrets from `rng`
    /// and asks to keep them ([`Effect::StoreSecrets`]); a device whose
    /// state doesn't say it was registered registers, then dials.
    #[allow(clippy::needless_pass_by_value)] // the §10.2 contract's shape
    pub fn start(
        me: Me,
        stored: Option<Vec<u8>>,
        secrets: Option<String>,
        prefs: ChatPrefs,
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> (Self, Vec<Effect>) {
        let secrets = secrets.map(Zeroizing::new);
        let mut out = Vec::new();
        let device = if let Some(device) = secrets
            .as_deref()
            .map(String::as_str)
            .and_then(vault::decode_secrets)
        {
            device
        } else {
            let device = DeviceSecrets::generate(rng);
            out.push(Effect::StoreSecrets(SecretValue(vault::encode_secrets(
                &device,
            ))));
            device
        };
        let state = stored
            .and_then(|bytes| {
                at_rest_open(device.vault_key(), RestKind::State, STATE_FILE, &bytes).ok()
            })
            .and_then(|plain| serde_json::from_slice::<StoredState>(&plain).ok())
            .filter(|s| s.device_id == device.device_id())
            .unwrap_or_else(|| StoredState {
                v: STATE_VERSION,
                device_id: device.device_id().to_string(),
                ..StoredState::default()
            });
        let status = state.status.unwrap_or(Status::Online);
        let mut social = Self {
            me,
            device,
            state,
            prefs,
            key_home: KeyHome::Unknown,
            keep_days: baylee_protocol::social::KEEP_DAYS_DEFAULT,
            contacts: Contacts::default(),
            convs: Vec::new(),
            link: Link::Down,
            status,
            auto_away: false,
            activity: None,
            outbox: Vec::new(),
            next_req: 1,
            next_local: 0,
            held: Vec::new(),
            held_paused: false,
            fetching: BTreeSet::new(),
            refetched: BTreeSet::new(),
            acked: 0,
            high: 0,
            retry_register_at: None,
            registering: false,
            table: None,
            composer: Composer::new(),
            card_candidates: Vec::new(),
            active: None,
            panel_open: false,
            focused: true,
            revision: 1,
            own_devices: Vec::new(),
            sent_invites: BTreeMap::new(),
            leave_armed: None,
            last_group: None,
            stale: BTreeSet::new(),
            queued: Vec::new(),
            last_now: now_ms,
            evidence: BTreeMap::new(),
            dirty: true,
            removed: false,
            trusted_own_devices: None,
        };
        for conv in social.state.convs.clone() {
            if let Ok(id) = conv.id.parse::<Conv>() {
                let muted = social.state.muted.contains(&conv.id);
                let c = social.conv_entry(&id, now_ms);
                c.muted = muted;
                c.open = false;
            }
        }
        social.prune_seen(now_ms);
        if social.state.registered {
            out.push(Effect::Dial);
        } else {
            social.register(&mut out);
        }
        social.flush_into(&mut out, rng);
        (social, out)
    }

    /// Loads the history logs of the conversations the state knows (the
    /// last [`super::convs::LINES_KEPT`] lines of each), and prunes what is
    /// past the player's history length by rewriting the logs.
    pub fn restore(
        &mut self,
        disk: &dyn ChatDisk,
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> Vec<Effect> {
        let mut out = Vec::new();
        let keep_after = now_ms.saturating_sub(u64::from(self.prefs.history_days) * 86_400_000);
        let vault_key = *self.device.vault_key();
        for stored in self.state.convs.clone() {
            let Ok(id) = stored.id.parse::<Conv>() else {
                continue;
            };
            let path = history::file_path(&vault_key, &id);
            let Some(bytes) = disk.read(&path) else {
                continue;
            };
            let loaded = history::read(&vault_key, &history::file_name(&vault_key, &id), &bytes);
            let total = loaded.records.len();
            let kept: Vec<Record> = if self.prefs.history_days == 0 {
                Vec::new()
            } else {
                loaded
                    .records
                    .into_iter()
                    .filter(|r| r.at >= keep_after)
                    .collect()
            };
            if kept.len() != total || loaded.skipped > 0 {
                let mut bytes = Vec::new();
                for record in &kept {
                    if let Some(framed) = history::seal_record(&vault_key, &id, record, rng) {
                        bytes.extend(framed);
                    }
                }
                out.push(Effect::RewriteHistory { file: path, bytes });
            }
            let me = self.me.tag.clone();
            let lines: Vec<ChatLine> = kept.iter().map(|r| r.line(&id, &me)).collect();
            for record in &kept {
                self.state
                    .seen
                    .entry(record.id.clone())
                    .or_insert(record.at);
                if record.evidence.is_some() {
                    self.evidence.insert(
                        LineKey {
                            conv: id.clone(),
                            id: record.id.clone(),
                        },
                        record.evidence.clone(),
                    );
                }
            }
            let conv = self.conv_entry(&id, now_ms);
            for line in lines {
                conv.insert(line);
            }
            if conv.title.is_empty() {
                conv.title.clone_from(&stored.title);
            }
        }
        self.bump();
        out
    }

    /// Where the device's secrets live, for the encryption notice (the
    /// [`super::SecretStore::home`] the client opened).
    pub fn set_key_home(&mut self, home: KeyHome) {
        self.key_home = home;
        self.bump();
    }

    /// The days the gateway keeps undelivered messages
    /// (`/info.social.keep_days`).
    pub fn set_keep_days(&mut self, days: u32) {
        self.keep_days = days;
        self.bump();
    }

    /// What the client says the player is doing, which the gateway can't
    /// see (the deck builder, a local game); sent with the status.
    pub fn set_activity(
        &mut self,
        activity: Option<baylee_protocol::social::Activity>,
    ) -> Vec<Effect> {
        if self.activity == activity {
            return Vec::new();
        }
        self.activity = activity;
        self.presence_effects()
    }

    /// Whether the window has the focus (notifications, §10.6).
    pub fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// The credential store's entry for this device.
    #[must_use]
    pub fn secret_slot(&self) -> SecretSlot {
        SecretSlot::new(&self.me.tag, &self.me.gateway)
    }

    /// Seals the state if it changed and sends what waits to be sealed (a
    /// call without entropy — the clock, the link, the table — only marks
    /// them; [`Social::wants_entropy`] says when to call this).
    pub fn flush(&mut self, now_ms: u64, rng: &mut dyn Entropy) -> Vec<Effect> {
        self.last_now = self.last_now.max(now_ms);
        let mut out = std::mem::take(&mut self.queued);
        self.pump(now_ms, rng, &mut out);
        self.flush_into(&mut out, rng);
        self.ack(&mut out);
        out
    }

    // -----------------------------------------------------------------
    // Inputs.
    // -----------------------------------------------------------------

    /// An HTTP answer for a request this machine asked for.
    #[allow(clippy::too_many_lines)] // one arm per request, read top to bottom
    pub fn on_http(
        &mut self,
        tag: HttpTag,
        status: u16,
        body: &[u8],
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> Vec<Effect> {
        let mut out = Vec::new();
        self.last_now = self.last_now.max(now_ms);
        let ok = (200..300).contains(&status);
        match tag {
            HttpTag::RegisterDevice => {
                self.registering = false;
                match status {
                    200 | 201 => {
                        self.state.registered = true;
                        self.dirty = true;
                        out.push(Effect::Dial);
                    }
                    409 => out.push(toast(ChatPhrase::ToastTooManyDevices, vec![])),
                    429 => self.retry_register_at = Some(now_ms + BACKOFF_MS),
                    _ => out.push(toast(ChatPhrase::ToastRegisterFailed, vec![])),
                }
            }
            HttpTag::OwnDevices => {
                if let Ok(list) = serde_json::from_slice::<DeviceList>(body) {
                    let me = self.device.device_id().to_string();
                    self.own_devices = list
                        .devices
                        .into_iter()
                        .map(|d| DeviceRow {
                            this: d.id == me,
                            id: d.id,
                            label: super::text::clean_name(&d.label),
                            added_at: d.added_at,
                            seen_at: d.seen_at,
                        })
                        .collect();
                }
            }
            HttpTag::Keys(tags) => {
                for tag in &tags {
                    self.fetching.remove(tag);
                }
                if ok {
                    let answer = serde_json::from_slice::<KeysAnswer>(body)
                        .unwrap_or(KeysAnswer { keys: Vec::new() });
                    let mut invalid = Vec::new();
                    let mut counts = BTreeMap::new();
                    for account in &answer.keys {
                        *counts.entry(&account.tag).or_insert(0_usize) += 1;
                    }
                    for account in &answer.keys {
                        if !tags.contains(&account.tag) {
                            continue;
                        }
                        if counts.get(&account.tag) != Some(&1) {
                            invalid.push(account.tag.clone());
                            continue;
                        }
                        let diff = self.state.keys.take(account, now_ms);
                        if diff.refused == 0 {
                            self.stale.remove(&account.tag);
                        } else {
                            invalid.push(account.tag.clone());
                        }
                        self.dirty = true;
                        self.keys_diff(&diff, now_ms, &mut out);
                    }
                    let answered: BTreeSet<&String> = answer.keys.iter().map(|k| &k.tag).collect();
                    let mut refused: Vec<String> = tags
                        .iter()
                        .filter(|t| !answered.contains(t))
                        .cloned()
                        .collect();
                    refused.extend(invalid);
                    self.keys_refused(&refused, now_ms);
                } else {
                    // A refused fetch (429 and the like): the sends waiting
                    // on it go again on the next tick.
                    for p in &mut self.outbox {
                        if p.wait == Wait::Keys {
                            p.wait = Wait::RetryAt(now_ms + RATE_RETRY_MS);
                        }
                    }
                }
                self.retry_held(now_ms, rng, &mut out);
                self.pump(now_ms, rng, &mut out);
            }
            HttpTag::AddFriend(handle) => match status {
                200 | 201 | 409 => {}
                404 => out.push(toast(ChatPhrase::ToastNoSuchPlayer, vec![handle])),
                429 => out.push(toast(ChatPhrase::ToastSlowDown, vec![])),
                _ => out.push(toast(ChatPhrase::ToastRefused, vec![])),
            },
            HttpTag::TableInvite(tag) => {
                if ok {
                    #[derive(Deserialize)]
                    struct Id {
                        id: String,
                    }
                    let person = self.person_of(&tag);
                    if let Ok(Id { id }) = serde_json::from_slice::<Id>(body) {
                        self.sent_invites.insert(id, person.clone());
                    }
                    if let Some(conv) = self.table.as_ref().and_then(TableContext::table_conv) {
                        self.notice(&conv, ChatPhrase::Invited, vec![person.handle], now_ms);
                    }
                } else {
                    out.push(refusal_toast(status));
                }
            }
            HttpTag::RemoveDevice(id) => {
                if ok && id == self.device.device_id() {
                    self.forget_device(&mut out);
                } else if ok {
                    self.own_devices.retain(|d| d.id != id);
                } else {
                    out.push(refusal_toast(status));
                }
            }
            HttpTag::Report(_) => out.push(if ok {
                toast(ChatPhrase::ToastReported, vec![])
            } else {
                refusal_toast(status)
            }),
            HttpTag::Lookup(_)
            | HttpTag::Friendship(_)
            | HttpTag::Settings
            | HttpTag::Group
            | HttpTag::AnswerInvite(_) => {
                if !ok {
                    out.push(refusal_toast(status));
                }
            }
        }
        self.finish(&mut out, rng);
        out
    }

    /// A frame from the social socket.
    #[allow(clippy::too_many_lines)] // one arm per frame, read top to bottom
    pub fn on_frame(&mut self, frame: Down, now_ms: u64, rng: &mut dyn Entropy) -> Vec<Effect> {
        let mut out = Vec::new();
        if self.held_paused {
            return out;
        }
        self.last_now = self.last_now.max(now_ms);
        match frame {
            Down::Welcome {
                tag,
                handle,
                device,
                guest,
            } => {
                if device == self.device.device_id() && tag == self.me.tag {
                    self.link = Link::Welcomed;
                    self.me.handle = super::text::clean_name(&handle);
                    self.me.guest = guest;
                    let me = self.me.tag.clone();
                    self.request_keys(&[me], &mut out);
                    out.push(http(
                        HttpTag::OwnDevices,
                        Method::Get,
                        "/social/devices".into(),
                        None,
                    ));
                    for p in &mut self.outbox {
                        if p.wait == Wait::Link || p.wait == Wait::Answer {
                            p.wait = Wait::Keys;
                        }
                    }
                    self.pump(now_ms, rng, &mut out);
                }
            }
            Down::Snapshot(snapshot) => {
                self.contacts = Contacts::from_snapshot(&snapshot, now_ms);
                let live: BTreeSet<String> =
                    snapshot.tables.iter().map(|t| t.conv.clone()).collect();
                for table in &snapshot.tables {
                    if let Ok(id) = table.conv.parse::<Conv>() {
                        self.table_members(&id, &table.members, now_ms);
                    }
                }
                for conv in &mut self.convs {
                    let table = matches!(conv.kind(), ConvKind::Table | ConvKind::Team);
                    if table && conv.ended_at.is_none() && !live.contains(&conv.id.to_string()) {
                        conv.ended_at = Some(now_ms);
                    }
                }
                for group in &snapshot.groups {
                    if let Ok(id) = format!("grp:{}", group.id).parse::<Conv>()
                        && let Some(conv) = self.convs.iter_mut().find(|c| c.id == id)
                    {
                        conv.title = super::text::clean_name(&group.name);
                        conv.members = group.members.iter().map(|m| m.person.clone()).collect();
                    }
                }
                self.refresh_candidates();
            }
            Down::Msg {
                seq,
                at,
                conv,
                mut from,
                sealed,
            } => {
                from.handle = super::text::clean_name(&from.handle);
                self.receive(
                    Held {
                        seq,
                        at,
                        conv,
                        from,
                        sealed,
                    },
                    false,
                    now_ms,
                    rng,
                    &mut out,
                );
            }
            Down::Global { id, at, from, body } => {
                let person = Person {
                    tag: from.tag,
                    handle: from.handle,
                };
                self.receive_global(&id, at, person, &body, now_ms, rng, &mut out);
            }
            Down::Sent { req, id, at } => self.sent(req, &id, at, rng, &mut out),
            Down::Refused {
                req: Some(req),
                why,
                ..
            } => self.refused(req, why, now_ms, rng, &mut out),
            Down::Refused { req: None, why, .. } => match why {
                Why::Device => {
                    self.state.registered = false;
                    self.dirty = true;
                    self.link = Link::Down;
                    out.push(Effect::Hang);
                    self.register(&mut out);
                }
                Why::Protocol | Why::Off => {
                    self.link = Link::Stopped;
                    out.push(Effect::Hang);
                }
                _ => {}
            },
            Down::Presence { tag, presence } => {
                let was = self.contacts.friend(&tag).map(|f| f.presence.status);
                let online = presence.status != Shown::Offline;
                self.contacts.set_presence(&tag, presence);
                if was == Some(Shown::Offline) && online && self.prefs.sounds.friend_online {
                    let person = self.person_of(&tag);
                    out.push(toast(
                        ChatPhrase::ToastOnline,
                        vec![self.shown_name(&person)],
                    ));
                    out.push(Effect::Sound(ChatCue::FriendRequest));
                }
            }
            Down::Friend {
                event,
                person,
                presence,
            } => {
                let was_outgoing = self.contacts.outgoing.iter().any(|p| p.tag == person.tag);
                self.contacts.friend_event(event, &person, presence, now_ms);
                if matches!(
                    event,
                    FriendEvent::Accepted | FriendEvent::RequestGone | FriendEvent::Blocked
                ) {
                    out.push(Effect::Withdraw(ToastAbout::FriendRequest(
                        person.tag.clone(),
                    )));
                }
                let name = self.shown_name(&person);
                match event {
                    FriendEvent::RequestIn => {
                        let tag = person.tag.clone();
                        out.push(Effect::Toast(ChatToast {
                            phrase: ChatPhrase::ToastFriendRequest,
                            args: vec![name],
                            conv: None,
                            actions: vec![
                                (ToastButton::Accept, Intent::Accept { tag: tag.clone() }),
                                (ToastButton::Decline, Intent::Decline { tag: tag.clone() }),
                            ],
                            until_answered: true,
                            about: Some(ToastAbout::FriendRequest(tag)),
                        }));
                        if self.status != Status::Dnd {
                            out.push(Effect::Sound(ChatCue::FriendRequest));
                        }
                    }
                    FriendEvent::Accepted if was_outgoing => {
                        out.push(toast(ChatPhrase::ToastFriendAccepted, vec![name]));
                    }
                    _ => {}
                }
                self.refresh_candidates();
            }
            Down::Group {
                event,
                group,
                inviter,
            } => {
                let invited = event == GroupEvent::Invited;
                self.contacts
                    .group_event(event, &group, inviter.as_ref(), now_ms);
                if matches!(
                    event,
                    GroupEvent::Joined
                        | GroupEvent::InviteGone
                        | GroupEvent::Left
                        | GroupEvent::Deleted
                ) {
                    out.push(Effect::Withdraw(ToastAbout::GroupInvite(group.id.clone())));
                }
                if let Ok(id) = format!("grp:{}", group.id).parse::<Conv>() {
                    match event {
                        GroupEvent::Joined | GroupEvent::Updated => {
                            let conv = self.conv_entry(&id, now_ms);
                            conv.title = super::text::clean_name(&group.name);
                            conv.members = group.members.iter().map(|m| m.person.clone()).collect();
                            conv.ended_at = None;
                        }
                        GroupEvent::Left | GroupEvent::Deleted => {
                            if let Some(conv) = self.convs.iter_mut().find(|c| c.id == id) {
                                conv.ended_at = Some(now_ms);
                                conv.members.clear();
                            }
                        }
                        _ => {}
                    }
                }
                if invited {
                    let id = group.id.clone();
                    out.push(Effect::Toast(ChatToast {
                        phrase: ChatPhrase::ToastGroupInvited,
                        args: vec![super::text::clean_name(&group.name)],
                        conv: None,
                        actions: vec![
                            (ToastButton::Accept, Intent::JoinGroup { id: id.clone() }),
                            (
                                ToastButton::Decline,
                                Intent::DeclineGroup { id: id.clone() },
                            ),
                        ],
                        until_answered: true,
                        about: Some(ToastAbout::GroupInvite(id)),
                    }));
                }
                self.refresh_candidates();
            }
            Down::Members { conv, members } => {
                if let Ok(id) = conv.parse::<Conv>() {
                    self.table_members(&id, &members, now_ms);
                }
                self.refresh_candidates();
            }
            Down::KeysChanged { tag } => {
                if tag == self.me.tag || self.state.keys.has(&tag) {
                    self.request_keys(&[tag], &mut out);
                }
            }
            Down::Invite(invite) => self.invite(invite, now_ms, &mut out),
            Down::InviteGone { id, answer } => {
                self.contacts.invites.retain(|i| i.invite.id != id);
                out.push(Effect::Withdraw(ToastAbout::TableInvite(id.clone())));
                if let Some(person) = self.sent_invites.remove(&id)
                    && answer == Some(baylee_protocol::social::InviteAnswer::Declined)
                    && let Some(conv) = self.table.as_ref().and_then(TableContext::table_conv)
                {
                    self.notice(
                        &conv,
                        ChatPhrase::InviteDeclined,
                        vec![person.handle],
                        now_ms,
                    );
                }
            }
            Down::Notice { conv, notice } => {
                if let Ok(id) = conv.parse::<Conv>() {
                    let (phrase, args) = match notice {
                        Notice::MemberJoined { person } => {
                            (ChatPhrase::MemberJoined, vec![person.handle])
                        }
                        Notice::MemberLeft { person } => {
                            (ChatPhrase::MemberLeft, vec![person.handle])
                        }
                        Notice::Renamed { name } => (ChatPhrase::GroupRenamed, vec![name]),
                        Notice::OwnerNow { person } => (ChatPhrase::OwnerNow, vec![person.handle]),
                    };
                    let args = args.iter().map(|a| super::text::clean_name(a)).collect();
                    let key = self.local_key(&id);
                    self.conv_entry(&id, now_ms).insert(ChatLine {
                        key,
                        at: now_ms,
                        kind: LineKind::Server { phrase, args },
                        state: LineState::Final,
                        hidden: false,
                        mentions_me: false,
                        failure: None,
                    });
                }
            }
        }
        self.finish(&mut out, rng);
        out
    }

    /// The socket went up or down, or its ticket was refused.
    pub fn on_link(&mut self, link: SocialLink, now_ms: u64) -> Vec<Effect> {
        let mut out = Vec::new();
        self.last_now = self.last_now.max(now_ms);
        match link {
            SocialLink::Up => {
                if self.link != Link::Stopped {
                    self.link = Link::Up;
                    out.push(Effect::Up(Up::Hello {
                        device: self.device.device_id().to_string(),
                        presence: self.presence_set(),
                    }));
                }
            }
            SocialLink::Down => {
                if self.link != Link::Stopped {
                    self.link = Link::Down;
                }
                for p in &mut self.outbox {
                    if p.wait == Wait::Answer {
                        p.wait = Wait::Link;
                    }
                }
                // A delivery not acked is delivered again after reconnecting.
                self.held.clear();
                self.high = self.acked;
            }
            SocialLink::Refused => self.link = Link::Stopped,
        }
        self.bump();
        out
    }

    /// The table the player is at, or none (§10.2). Leaving a table takes
    /// its people into the recent players.
    pub fn on_table(&mut self, table: Option<TableContext>) -> Vec<Effect> {
        if self.table == table {
            return Vec::new();
        }
        if let Some(old) = &self.table
            && table.as_ref().is_none_or(|t| t.game != old.game)
        {
            let me = self.me.tag.clone();
            let people: Vec<(String, String)> = old
                .people(&me)
                .filter_map(|s| s.tag().map(|t| (t.to_string(), s.handle.clone())))
                .collect();
            for (tag, handle) in people {
                self.state.recent.retain(|r| r.tag != tag);
                self.state.recent.insert(
                    0,
                    RecentPlayer {
                        tag,
                        handle,
                        at: self.last_now,
                    },
                );
            }
            self.state.recent.truncate(RECENT_KEPT);
            self.dirty = true;
        }
        if let Some(new) = &table {
            let table_conv = new.table_conv();
            let rematch = self.table.as_ref().is_some_and(|old| old.game != new.game);
            if let Some(conv) = table_conv {
                let fresh = !self.convs.iter().any(|c| c.id == conv);
                let entry = self.conv_entry(&conv, 0);
                entry.ended_at = None;
                if fresh && rematch {
                    let key = LineKey {
                        conv: conv.clone(),
                        id: "divider".into(),
                    };
                    self.conv_entry(&conv, 0).insert(ChatLine {
                        key,
                        at: 0,
                        kind: LineKind::Divider,
                        state: LineState::Final,
                        hidden: false,
                        mentions_me: false,
                        failure: None,
                    });
                }
                if matches!(self.composer.target(), Target::Sticky) {
                    self.composer.set_target(Target::Table);
                }
            }
            if let Some(team) = new.team_conv() {
                self.conv_entry(&team, 0).ended_at = None;
            }
        }
        self.table = table;
        self.refresh_candidates();
        self.bump();
        Vec::new()
    }

    /// The clock: auto-away after ten idle minutes and back on input,
    /// sends refused for their rate, a registration that waits, invites
    /// that lapse, table conversations past their hour.
    pub fn tick(&mut self, now_ms: u64, idle_ms: u64) -> Vec<Effect> {
        let mut out = Vec::new();
        self.last_now = self.last_now.max(now_ms);
        if self.status == Status::Online && idle_ms >= AWAY_AFTER_MS {
            self.status = Status::Away;
            self.auto_away = true;
            out.extend(self.presence_effects());
        } else if self.auto_away && idle_ms < AWAY_AFTER_MS {
            self.auto_away = false;
            self.status = Status::Online;
            out.extend(self.presence_effects());
        }
        if self.retry_register_at.is_some_and(|at| at <= now_ms) {
            self.retry_register_at = None;
            self.register(&mut out);
        }
        let expired = self.contacts.expire_invites(now_ms);
        if !expired.is_empty() {
            for id in expired {
                out.push(Effect::Withdraw(ToastAbout::TableInvite(id)));
            }
            self.bump();
        }
        let due = self
            .outbox
            .iter()
            .any(|p| matches!(p.wait, Wait::RetryAt(at) if at <= now_ms));
        if due {
            for p in &mut self.outbox {
                if matches!(p.wait, Wait::RetryAt(at) if at <= now_ms) {
                    p.wait = Wait::Keys;
                }
            }
            // Sealing needs entropy: the next call that has it sends them.
            self.bump();
        }
        out
    }

    /// Whether sends wait to be sealed; the client then calls
    /// [`Social::flush`] (or any call with entropy) soon.
    #[must_use]
    pub fn wants_entropy(&self) -> bool {
        self.dirty || self.outbox.iter().any(|p| p.wait == Wait::Keys)
    }

    /// What the player asked for.
    pub fn act(&mut self, intent: Intent, now_ms: u64, rng: &mut dyn Entropy) -> Vec<Effect> {
        let mut out = Vec::new();
        self.last_now = self.last_now.max(now_ms);
        self.intent(intent, now_ms, rng, &mut out);
        self.finish(&mut out, rng);
        out
    }

    /// A key for the open composer; Enter with something typed submits.
    pub fn key(
        &mut self,
        key: ComposerKey,
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> (ComposerOutcome, Vec<Effect>) {
        self.last_now = self.last_now.max(now_ms);
        let ctx = self.compose_context();
        let outcome = self.composer.key(key, &ctx);
        let mut out = Vec::new();
        match &outcome {
            ComposerOutcome::Submit => self.submit(now_ms, rng, &mut out),
            ComposerOutcome::Close => {
                if let Some(active) = self.active.clone() {
                    self.composer.stash_draft(&active);
                }
            }
            _ => {}
        }
        self.finish(&mut out, rng);
        (outcome, out)
    }

    /// Opens the composer: `/` with `/` typed, `R` with `/r <last> `
    /// typed, Ctrl/Cmd+Enter empty (§12.1).
    pub fn open_composer(&mut self, prefix: &str) {
        let prefix = if prefix == "/r" {
            match self.last_whisperer() {
                Some(person) => format!("/r {} ", person.handle),
                None => "/r ".to_string(),
            }
        } else {
            prefix.to_string()
        };
        self.composer.open_with(&prefix);
        self.bump();
    }

    /// A right-pressed card while the composer has the caret (§11.9); a
    /// hint to show, when there is one.
    pub fn insert_card(&mut self, card: LinkedCard) -> Option<ChatPhrase> {
        let hint = self.composer.insert_card(card);
        self.bump();
        hint
    }

    /// The cards `#` offers here: at a table `refs::candidates` (which
    /// offers nothing [`crate::bugreport::refs::zone_of`] would not), then
    /// the pool; elsewhere the pool.
    pub fn set_card_candidates(&mut self, cards: Vec<CardRef>) {
        self.card_candidates = cards;
        self.refresh_candidates();
    }

    // -----------------------------------------------------------------
    // What the client draws.
    // -----------------------------------------------------------------

    /// Bumps on any visible change (the client's rebuild guard).
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// This device's id.
    #[must_use]
    pub fn device_id(&self) -> &str {
        self.device.device_id()
    }

    /// This account.
    #[must_use]
    pub fn me(&self) -> &Me {
        &self.me
    }

    /// The friends drawer's sections.
    #[must_use]
    pub fn friends(&self) -> FriendsView<'_> {
        self.contacts.view()
    }

    /// The social graph.
    #[must_use]
    pub fn contacts(&self) -> &Contacts {
        &self.contacts
    }

    /// The tabs, in tab order: Table and Team first, then the others as
    /// they opened. Closed tabs are left out.
    #[must_use]
    pub fn conversations(&self) -> &[Conversation] {
        &self.convs
    }

    /// One conversation.
    #[must_use]
    pub fn conversation(&self, id: &ConvId) -> Option<&Conversation> {
        self.convs.iter().find(|c| c.id == *id)
    }

    /// The All tab: the latest `limit` lines of every conversation, in
    /// order.
    #[must_use]
    pub fn all_lines(&self, limit: usize) -> Vec<(&ConvId, &ChatLine)> {
        let mut lines: Vec<(&ConvId, &ChatLine)> = self
            .convs
            .iter()
            .flat_map(|c| c.lines.iter().map(move |l| (&c.id, l)))
            .collect();
        lines.sort_by(|a, b| (a.1.at, &a.1.key.id).cmp(&(b.1.at, &b.1.key.id)));
        let skip = lines.len().saturating_sub(limit);
        lines.split_off(skip)
    }

    /// The lines `filter` admits, of every conversation, oldest first, the
    /// latest `limit` (the chat window's tabs, §3; the log's lines are the
    /// renderer's and are merged there).
    #[must_use]
    pub fn lines_where(
        &self,
        filter: &super::TabFilter,
        limit: usize,
    ) -> Vec<(&ConvId, &ChatLine)> {
        let mut lines: Vec<(&ConvId, &ChatLine)> = self
            .convs
            .iter()
            .filter(|c| filter.admits_conv(&c.id))
            .flat_map(|c| {
                c.lines
                    .iter()
                    .filter(|l| filter.admits(&c.id, l))
                    .map(move |l| (&c.id, l))
            })
            .collect();
        lines.sort_by(|a, b| (a.1.at, &a.1.key.id).cmp(&(b.1.at, &b.1.key.id)));
        let skip = lines.len().saturating_sub(limit);
        lines.split_off(skip)
    }

    /// Unread lines of the conversations `filter` admits that are not
    /// muted, and whether a whisper or a mention of this player is among
    /// them (the tab's pill turns rose, §3.2).
    #[must_use]
    pub fn unread_where(&self, filter: &super::TabFilter) -> (u32, bool) {
        let mut count = 0;
        let mut urgent = false;
        for conv in self.convs.iter().filter(|c| filter.admits_conv(&c.id)) {
            if conv.muted || conv.unread == 0 {
                continue;
            }
            count += conv.unread;
            let unread = usize::try_from(conv.unread).unwrap_or(usize::MAX);
            let fresh = conv.lines.iter().rev().take(unread);
            urgent |=
                matches!(conv.id, Conv::Dm { .. }) || fresh.into_iter().any(|l| l.mentions_me);
        }
        (count, urgent)
    }

    /// The last player who whispered this one (`/r`'s partner), by tag.
    #[must_use]
    pub fn last_whisper_partner(&self) -> Option<&str> {
        self.state.last_whisper.as_deref()
    }

    /// The person a tag names, as far as this machine knows them.
    #[must_use]
    pub fn person(&self, tag: &str) -> Person {
        self.person_of(tag)
    }

    /// The table the player is at, as the client last said.
    #[must_use]
    pub fn table(&self) -> Option<&TableContext> {
        self.table.as_ref()
    }

    /// Unread lines of every conversation not muted.
    #[must_use]
    pub fn unread_total(&self) -> u32 {
        self.convs
            .iter()
            .filter(|c| !c.muted)
            .map(|c| c.unread)
            .sum()
    }

    /// The composer.
    #[must_use]
    pub fn composer(&self) -> &Composer {
        &self.composer
    }

    /// The composer, to type into.
    pub fn composer_mut(&mut self) -> &mut Composer {
        self.bump();
        &mut self.composer
    }

    /// What Tab cycles through and `/w` completes, here.
    #[must_use]
    pub fn compose_context(&self) -> ComposeContext {
        let mut targets = Vec::new();
        if let Some(table) = &self.table {
            if table.table_conv().is_some() {
                targets.push(Target::Table);
            }
            if table.team_conv().is_some() {
                targets.push(Target::Team);
            }
        }
        let mut recent: Vec<&Conversation> = self.convs.iter().filter(|c| c.open).collect();
        recent.sort_by_key(|c| std::cmp::Reverse(c.lines.last().map_or(0, |l| l.at)));
        for conv in &recent {
            if let Some(partner) = conv.id.partner(&self.me.tag) {
                targets.push(Target::Whisper(self.person_of(partner).handle));
            }
        }
        for conv in &recent {
            if let Conv::Group { id } = &conv.id
                && let Some(n) = self.contacts.groups.iter().position(|g| g.id == *id)
                && n < 9
            {
                targets.push(Target::Group(GroupRef::Number(n as u8 + 1)));
            }
        }
        let mut names: Vec<String> = self.known_people().into_iter().map(|p| p.handle).collect();
        names.dedup();
        ComposeContext { targets, names }
    }

    /// The conversation a target writes to here, if any.
    #[must_use]
    pub fn target_conv(&self, target: &Target) -> Option<ConvId> {
        self.resolve(target).ok()
    }

    /// A contact's safety number, when both accounts' keys are known.
    #[must_use]
    pub fn safety(&self, tag: &str) -> Option<SafetyView> {
        let theirs = self.state.keys.get(tag)?;
        let mine = self.state.keys.get(&self.me.tag)?;
        let digits = baylee_seal::safety_number((&self.me.tag, &mine.iks()), (tag, &theirs.iks()));
        Some(SafetyView {
            expected: theirs.public_keys(),
            groups: digits
                .as_bytes()
                .chunks(5)
                .map(|c| String::from_utf8_lossy(c).into_owned())
                .collect(),
            verified: theirs.verified() && self.keys_confirmed(tag),
            devices: theirs
                .devices
                .iter()
                .map(|d| (d.label.clone(), d.added_at))
                .collect(),
        })
    }

    /// A contact's keys, as cached.
    #[must_use]
    pub fn contact_keys(&self, tag: &str) -> Option<&ContactKeys> {
        self.state.keys.get(tag)
    }

    /// Restrict own-account encryption recipients to public keys authenticated by
    /// the Central archive. Call before publishing startup/network effects; never
    /// construct this whitelist from a gateway directory. Missing current keys or
    /// any unlisted own device block sends, including after a verification click.
    pub fn trust_own_devices(&mut self, mut keys: Vec<([u8; 32], [u8; 32])>) {
        keys.sort_unstable();
        keys.dedup();
        self.trusted_own_devices = Some(keys);
    }

    /// This account's devices (Settings → Chat & privacy).
    #[must_use]
    pub fn own_devices(&self) -> &[DeviceRow] {
        &self.own_devices
    }

    /// What the encryption notice says about this device.
    #[must_use]
    pub fn encryption_note(&self) -> NoteState {
        NoteState {
            seen: self.prefs.notice_seen,
            home: self.key_home,
            keep_days: if self.me.guest {
                baylee_protocol::social::GUEST_KEEP_DAYS
            } else {
                self.keep_days
            },
        }
    }

    /// The player's status.
    #[must_use]
    pub fn status(&self) -> Status {
        self.status
    }

    /// The device's chat settings.
    #[must_use]
    pub fn prefs(&self) -> &ChatPrefs {
        &self.prefs
    }

    /// Whether the socket is up and welcomed.
    #[must_use]
    pub fn online(&self) -> bool {
        self.link == Link::Welcomed
    }

    /// Whether the machine stopped dialling for good.
    #[must_use]
    pub fn stopped(&self) -> bool {
        self.link == Link::Stopped
    }

    /// The players sat with recently, newest first.
    #[must_use]
    pub fn recent_players(&self) -> &[RecentPlayer] {
        &self.state.recent
    }

    /// The local nickname of `tag`.
    #[must_use]
    pub fn nickname(&self, tag: &str) -> Option<&str> {
        self.state.nicknames.get(tag).map(String::as_str)
    }

    /// Whether `tag`'s lines are hidden on this device.
    #[must_use]
    pub fn is_ignored(&self, tag: &str) -> bool {
        self.state.ignored.contains(tag)
    }

    /// The open tab, and whether the panel is open.
    #[must_use]
    pub fn panel(&self) -> (bool, Option<&ConvId>) {
        (self.panel_open, self.active.as_ref())
    }

    /// The name a person is drawn by: the nickname, else the handle.
    #[must_use]
    pub fn shown_name(&self, person: &Person) -> String {
        self.state
            .nicknames
            .get(&person.tag)
            .cloned()
            .unwrap_or_else(|| person.handle.clone())
    }

    /// Who this conversation encrypts to: people and devices (§5.4).
    #[must_use]
    pub fn encrypted_to(&self, id: &ConvId) -> Option<(usize, usize)> {
        if *id == Conv::Global {
            return None;
        }
        let tags = self.members(id)?;
        let people = tags.iter().filter(|t| **t != self.me.tag).count();
        let devices = tags
            .iter()
            .filter_map(|t| self.state.keys.get(t))
            .map(|k| {
                k.devices
                    .iter()
                    .filter(|d| d.id != self.device.device_id())
                    .count()
            })
            .sum();
        Some((people, devices))
    }

    // -----------------------------------------------------------------
    // Internals.
    // -----------------------------------------------------------------

    fn bump(&mut self) {
        self.revision += 1;
    }

    fn finish(&mut self, out: &mut Vec<Effect>, rng: &mut dyn Entropy) {
        out.append(&mut self.queued);
        self.pump(self.last_now, rng, out);
        self.flush_into(out, rng);
        self.ack(out);
        self.bump();
    }

    fn flush_into(&mut self, out: &mut Vec<Effect>, rng: &mut dyn Entropy) {
        if !self.dirty || self.removed {
            return;
        }
        let Ok(json) = serde_json::to_vec(&self.state).map(Zeroizing::new) else {
            return;
        };
        if let Ok(sealed) = at_rest_seal(
            self.device.vault_key(),
            RestKind::State,
            STATE_FILE,
            &json,
            rng,
        ) {
            out.push(Effect::SaveState(sealed));
            self.dirty = false;
        }
    }

    fn register(&mut self, out: &mut Vec<Effect>) {
        if self.registering {
            return;
        }
        let Ok(public) = self.device.public(&self.me.tag) else {
            return;
        };
        self.registering = true;
        out.push(http(
            HttpTag::RegisterDevice,
            Method::Post,
            "/social/devices".into(),
            Some(serde_json::json!({
                "ik": url::encode(&public.ik),
                "dk": url::encode(&public.dk),
                "dsig": url::encode(&public.dsig),
                "label": device_label(),
            })),
        ));
    }

    fn presence_set(&self) -> PresenceSet {
        PresenceSet {
            status: self.status,
            activity: self.activity,
        }
    }

    fn presence_effects(&mut self) -> Vec<Effect> {
        self.bump();
        if self.link == Link::Welcomed {
            vec![Effect::Up(Up::Presence(self.presence_set()))]
        } else {
            Vec::new()
        }
    }

    fn prune_seen(&mut self, now_ms: u64) {
        let floor = now_ms.saturating_sub(MAX_AGE_MS);
        self.state
            .seen
            .retain(|id, at| sealing::uuid_ms(id).unwrap_or(*at) >= floor || *at >= floor);
    }

    fn local_key(&mut self, conv: &ConvId) -> LineKey {
        self.next_local += 1;
        LineKey {
            conv: conv.clone(),
            id: format!("local-{}", self.next_local),
        }
    }

    /// The conversation `id`, made when it isn't there (and remembered in
    /// the state, so its history is found next time).
    fn conv_entry(&mut self, id: &ConvId, now_ms: u64) -> &mut Conversation {
        if let Some(i) = self.convs.iter().position(|c| c.id == *id) {
            return &mut self.convs[i];
        }
        let title = self.title_of(id);
        let text = id.to_string();
        if !self.state.convs.iter().any(|c| c.id == text) {
            self.state.convs.push(StoredConv {
                id: text.clone(),
                title: title.clone(),
            });
            self.dirty = true;
        }
        let mut conv = Conversation::new(id.clone(), title);
        conv.muted = self.state.muted.contains(&text);
        let _ = now_ms;
        // Table and Team first, in that order; the rest as they come.
        let at = match conv.kind() {
            ConvKind::Table => 0,
            ConvKind::Team => self
                .convs
                .iter()
                .take_while(|c| c.kind() == ConvKind::Table)
                .count(),
            _ => self.convs.len(),
        };
        self.convs.insert(at, conv);
        &mut self.convs[at]
    }

    fn title_of(&self, id: &ConvId) -> String {
        match id {
            Conv::Global => "Lobby".into(),
            Conv::Dm { .. } => id
                .partner(&self.me.tag)
                .map(|t| self.shown_name(&self.person_of(t)))
                .unwrap_or_default(),
            Conv::Group { id } => self
                .contacts
                .group(id)
                .map(|g| super::text::clean_name(&g.name))
                .unwrap_or_default(),
            Conv::Table { .. } => "Table".into(),
            Conv::Team { .. } => "Team".into(),
        }
    }

    fn notice(&mut self, conv: &ConvId, phrase: ChatPhrase, args: Vec<String>, now_ms: u64) {
        let key = self.local_key(conv);
        self.conv_entry(conv, now_ms).insert(ChatLine {
            key,
            at: now_ms,
            kind: LineKind::Notice { phrase, args },
            state: LineState::Final,
            hidden: false,
            mentions_me: false,
            failure: None,
        });
    }

    /// A notice in the active conversation (a command's answer), or the
    /// table's, or the first one there is.
    fn answer(&mut self, phrase: ChatPhrase, args: Vec<String>, now_ms: u64) {
        let conv = self
            .active
            .clone()
            .or_else(|| self.table.as_ref().and_then(TableContext::table_conv))
            .or_else(|| self.convs.first().map(|c| c.id.clone()));
        if let Some(conv) = conv {
            self.notice(&conv, phrase, args, now_ms);
        } else {
            // Nowhere to write it (no conversation yet): a toast says it.
            self.queued.push(toast(phrase, args));
        }
    }

    fn person_of(&self, tag: &str) -> Person {
        self.known_people()
            .into_iter()
            .find(|p| p.tag == tag)
            .or_else(|| {
                self.state.keys.get(tag).map(|k| Person {
                    tag: tag.to_string(),
                    handle: k.handle.clone(),
                })
            })
            .unwrap_or_else(|| Person {
                tag: tag.to_string(),
                handle: format!("#{tag}"),
            })
    }

    /// Everyone a name may be resolved against: friends, whisper partners,
    /// the roster, members of open conversations, recent players.
    fn known_people(&self) -> Vec<Person> {
        let mut people: Vec<Person> = Vec::new();
        let mut add = |p: Person| {
            if !people.iter().any(|q| q.tag == p.tag) {
                people.push(p);
            }
        };
        for f in &self.contacts.friends {
            add(f.person.clone());
        }
        for p in &self.state.whisperers {
            add(p.clone());
        }
        if let Some(table) = &self.table {
            for seat in table.people(&self.me.tag) {
                if let Some(tag) = seat.tag() {
                    add(Person {
                        tag: tag.to_string(),
                        handle: seat.handle.clone(),
                    });
                }
            }
        }
        for conv in &self.convs {
            for m in &conv.members {
                if m.tag != self.me.tag {
                    add(m.clone());
                }
            }
        }
        for r in &self.state.recent {
            add(Person {
                tag: r.tag.clone(),
                handle: r.handle.clone(),
            });
        }
        people
    }

    fn refresh_candidates(&mut self) {
        let seats: BTreeMap<String, u8> = self
            .table
            .as_ref()
            .map(|t| {
                t.roster
                    .iter()
                    .filter_map(|s| s.tag().map(|tag| (tag.to_string(), s.seat)))
                    .collect()
            })
            .unwrap_or_default();
        let players = self
            .known_people()
            .into_iter()
            .map(|p| PlayerRef {
                seat: seats.get(&p.tag).copied(),
                name: p.handle,
                tag: Some(p.tag),
            })
            .collect();
        self.composer.set_candidates(Candidates {
            cards: self.card_candidates.clone(),
            players,
        });
        self.bump();
    }

    fn table_members(&mut self, id: &ConvId, members: &[Person], now_ms: u64) {
        let members: Vec<Person> = members
            .iter()
            .map(|p| Person {
                tag: p.tag.clone(),
                handle: super::text::clean_name(&p.handle),
            })
            .collect();
        let conv = self.conv_entry(id, now_ms);
        conv.members = members;
        conv.ended_at = None;
    }
}

enum Verdict {
    Done,
    Hold,
}

fn http(tag: HttpTag, method: Method, path: String, body: Option<serde_json::Value>) -> Effect {
    Effect::Http {
        tag,
        method,
        path,
        body,
    }
}

fn toast(phrase: ChatPhrase, args: Vec<String>) -> Effect {
    Effect::Toast(ChatToast {
        phrase,
        args,
        conv: None,
        actions: Vec::new(),
        until_answered: false,
        about: None,
    })
}

fn refusal_toast(status: u16) -> Effect {
    match status {
        429 => toast(ChatPhrase::ToastSlowDown, vec![]),
        _ => toast(ChatPhrase::ToastRefused, vec![]),
    }
}

/// A toast's preview of a text: one line, at most 80 characters.
fn preview(text: &str) -> String {
    let line: String = text.replace('\n', " ");
    let mut out: String = line.chars().take(80).collect();
    if line.chars().count() > 80 {
        out.push('…');
    }
    out
}

/// `dd.mm.` of a time (UTC; the shell draws local times itself).
fn day_month(ms: u64) -> String {
    let days = ms / 86_400_000;
    let date = time::Date::from_julian_day(2_440_588 + days as i32);
    date.map_or_else(
        |_| String::new(),
        |d| format!("{:02}.{:02}.", d.day(), u8::from(d.month())),
    )
}

/// A path segment, percent-encoded (a handle may hold any character).
fn segment(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

mod intents;
mod key_requests;
mod receiving;
mod sending;
