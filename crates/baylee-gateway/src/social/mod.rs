//! Friends, presence and end-to-end encrypted chat (beta.8,
//! `docs/protocol.md` §"Friends and chat (beta.8)").
//!
//! The gateway carries sealed messages it cannot open: it checks shapes,
//! sizes, membership and that a send wraps the message key for exactly the
//! recipients' current devices, stores the ciphertext until each device has
//! it, and pushes what changed. It never logs a frame, ticket, ciphertext,
//! key, tag or handle: a log line here carries counts and refusal kinds.
//!
//! - [`socket`]: `/social/ws`, one socket per device.
//! - [`route`]: `Up::Send` and `Up::Ack`.
//! - [`members`]: who a conversation's members are, from the database and
//!   from lobby memory.
//! - [`presence`]: what friends are shown, and the table memberships,
//!   published from lobby memory.
//! - [`invites`]: table invites, in memory.
//! - [`routes`]: the HTTP routes under `/social/…`.
//! - [`report`]: `POST /social/reports`.
//! - [`sweep`]: the expiry sweep.

pub(crate) mod invites;
pub(crate) mod members;
pub(crate) mod presence;
pub(crate) mod report;
pub(crate) mod route;
pub(crate) mod routes;
pub(crate) mod socket;
pub(crate) mod sweep;

use crate::Shared;
use crate::auth::RateLimiter;
use crate::seatrate::Allowance;
use baylee_db::social::Who;
use baylee_protocol::social::{self as wire, Activity, Down, Person, SocialInfo, Status};
use parking_lot::Mutex;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

/// What a socket's writer is handed.
#[derive(Debug)]
pub(crate) enum Out {
    /// A frame to send as it is.
    Frame(Box<Down>),
    /// A delivery, which the socket sends only while its window has room.
    Deliver {
        /// The slot.
        seq: i64,
        /// The `Down::Msg`.
        frame: Box<Down>,
    },
    /// Close the socket with this code and reason.
    Close(u16, &'static str),
}

/// How many frames wait for one socket's writer before the socket is taken
/// for stuck and dropped.
const OUT_QUEUE: usize = 1024;

/// One open social socket.
struct Conn {
    /// Tells this socket from a later one for the same device.
    id: u64,
    tx: mpsc::Sender<Out>,
}

/// An account with at least one social socket open.
struct Online {
    me: Person,
    status: Status,
    activity: Option<Activity>,
    share_table: bool,
    /// Its friends, by account id, kept current by every change to them.
    friends: BTreeMap<String, Person>,
    devices: BTreeSet<String>,
}

#[derive(Default)]
struct Hub {
    conns: HashMap<String, Conn>,
    online: HashMap<String, Online>,
    next: u64,
}

/// The per-account allowances in memory.
#[derive(Default)]
struct Allowances {
    sends: HashMap<String, Allowance>,
    keys: HashMap<String, Allowance>,
}

/// The gateway's social side: on or off, how long messages are held, the
/// open sockets, presence, invites and allowances. Memory only.
pub(crate) struct Social {
    on: bool,
    keep_days: u32,
    hub: Mutex<Hub>,
    allowances: Mutex<Allowances>,
    pub(crate) invites: Mutex<invites::Invites>,
    pub(crate) watching: Mutex<BTreeMap<String, BTreeMap<String, usize>>>,
    pub(crate) published: Mutex<presence::Published>,
    /// Friend requests, per account: 20 an hour.
    pub(crate) requests: RateLimiter,
    /// Group and table invites, per account: 30 an hour.
    pub(crate) invite_limiter: RateLimiter,
    /// New groups, per account: 10 a day.
    pub(crate) new_groups: RateLimiter,
    /// Device registrations, per account: 5 a day.
    pub(crate) registrations: RateLimiter,
    /// Message reports, per account: 10 an hour.
    pub(crate) reports: RateLimiter,
    /// Recent messages in the public lobby chat (`Conv::Global`), kept in
    /// memory (up to 50) for newly joining sockets.
    global_history: Mutex<std::collections::VecDeque<Down>>,
}

const HOUR: Duration = Duration::from_hours(1);
const DAY: Duration = Duration::from_hours(24);
const GLOBAL_HISTORY_CAP: usize = 50;

impl Social {
    /// From `BAYLEE_SOCIAL` and `BAYLEE_SOCIAL_KEEP_DAYS`.
    ///
    /// # Errors
    ///
    /// A sentence for the operator when either is set to something else
    /// than it may be; the gateway does not start.
    pub(crate) fn from_env(social: Option<&str>, keep_days: Option<&str>) -> Result<Self, String> {
        let on = match social.map(str::trim) {
            None | Some("" | "on") => true,
            Some("off") => false,
            Some(other) => {
                return Err(format!(
                    "BAYLEE_SOCIAL: {other:?} is neither unset, \"on\" nor \"off\""
                ));
            }
        };
        let keep_days = match keep_days.map(str::trim) {
            None | Some("") => wire::KEEP_DAYS_DEFAULT,
            Some(days) => match days.parse::<u32>() {
                Ok(days) if (wire::KEEP_DAYS_MIN..=wire::KEEP_DAYS_MAX).contains(&days) => days,
                _ => {
                    return Err(format!(
                        "BAYLEE_SOCIAL_KEEP_DAYS: {days:?} is not a whole number from {} to {}",
                        wire::KEEP_DAYS_MIN,
                        wire::KEEP_DAYS_MAX
                    ));
                }
            },
        };
        Ok(Self::new(on, keep_days))
    }

    pub(crate) fn new(on: bool, keep_days: u32) -> Self {
        Self {
            on,
            keep_days,
            hub: Mutex::new(Hub::default()),
            allowances: Mutex::new(Allowances::default()),
            invites: Mutex::new(invites::Invites::default()),
            watching: Mutex::new(BTreeMap::new()),
            published: Mutex::new(presence::Published::default()),
            requests: RateLimiter::new(HOUR, 20),
            invite_limiter: RateLimiter::new(HOUR, 30),
            new_groups: RateLimiter::new(DAY, 10),
            registrations: RateLimiter::new(DAY, 5),
            reports: RateLimiter::new(HOUR, 10),
            global_history: Mutex::new(std::collections::VecDeque::with_capacity(
                GLOBAL_HISTORY_CAP,
            )),
        }
    }

    /// Whether this gateway has chat at all.
    pub(crate) const fn on(&self) -> bool {
        self.on
    }

    /// `GET /info`'s `social`: `None` when off.
    pub(crate) const fn info(&self) -> Option<SocialInfo> {
        if self.on {
            Some(SocialInfo {
                version: wire::SOCIAL_VERSION,
                keep_days: self.keep_days,
            })
        } else {
            None
        }
    }

    /// How many days a slot for a device of `guest`'s account is held.
    pub(crate) const fn keep_days_for(&self, guest: bool) -> u32 {
        if guest {
            wire::GUEST_KEEP_DAYS
        } else {
            self.keep_days
        }
    }

    /// Whether `account` may send one more message now (8 at once then 1 a
    /// second; a guest 4 then 1 every 3 s).
    pub(crate) fn admit_send(&self, account: &str, guest: bool, now: Instant) -> bool {
        let mut allowances = self.allowances.lock();
        let allowance = allowances
            .sends
            .entry(account.to_owned())
            .or_insert_with(|| {
                if guest {
                    Allowance::every(now, Duration::from_secs(3), 4)
                } else {
                    Allowance::new(now, 1, 8)
                }
            });
        allowance.admit(now)
    }

    /// Whether `account` may fetch keys once more now (20 at once, then 1 a
    /// second).
    pub(crate) fn admit_keys(&self, account: &str, now: Instant) -> bool {
        self.allowances
            .lock()
            .keys
            .entry(account.to_owned())
            .or_insert_with(|| Allowance::new(now, 1, 20))
            .admit(now)
    }

    // -------------------------------------------------------------------
    // The socket registry.
    // -------------------------------------------------------------------

    /// Registers `device`'s socket for `account`; a socket the device had
    /// open already is told to close (4001, replaced). Answers this
    /// socket's id and whether the account was offline before.
    #[allow(clippy::too_many_arguments)]
    fn attach(
        &self,
        account: &str,
        device: &str,
        me: Person,
        presence: &wire::PresenceSet,
        share_table: bool,
        friends: BTreeMap<String, Person>,
        tx: mpsc::Sender<Out>,
    ) -> (u64, bool) {
        let mut hub = self.hub.lock();
        hub.next += 1;
        let id = hub.next;
        if let Some(old) = hub.conns.insert(device.to_owned(), Conn { id, tx }) {
            let _ = old.tx.try_send(Out::Close(4001, "replaced"));
        }
        let was_offline = !hub.online.contains_key(account);
        let online = hub
            .online
            .entry(account.to_owned())
            .or_insert_with(|| Online {
                me,
                status: presence.status,
                activity: presence.activity,
                share_table,
                friends: BTreeMap::new(),
                devices: BTreeSet::new(),
            });
        online.status = presence.status;
        online.activity = presence.activity;
        online.share_table = share_table;
        online.friends = friends;
        online.devices.insert(device.to_owned());
        (id, was_offline)
    }

    /// Takes socket `id` of `device` out; answers the friends to tell when
    /// that was the account's last socket.
    fn detach(&self, account: &str, device: &str, id: u64) -> Option<(Person, Vec<String>)> {
        let mut hub = self.hub.lock();
        if hub.conns.get(device).is_some_and(|conn| conn.id == id) {
            hub.conns.remove(device);
            if let Some(online) = hub.online.get_mut(account) {
                online.devices.remove(device);
            }
        }
        let last = hub
            .online
            .get(account)
            .is_some_and(|online| online.devices.is_empty());
        if last {
            let online = hub.online.remove(account)?;
            return Some((online.me, online.friends.into_keys().collect()));
        }
        None
    }

    /// Sends `out` to one device's socket, if it has one open. A socket
    /// whose writer is that far behind is dropped.
    fn to_device(hub: &mut Hub, device: &str, out: Out) {
        let stuck = match hub.conns.get(device) {
            Some(conn) => conn.tx.try_send(out).is_err(),
            None => false,
        };
        if stuck {
            tracing::debug!("a social socket fell behind and was dropped");
            hub.conns.remove(device);
        }
    }

    /// Sends `frame` to every open socket of `account`.
    pub(crate) fn push(&self, account: &str, frame: &Down) {
        let mut hub = self.hub.lock();
        let devices: Vec<String> = hub
            .online
            .get(account)
            .map(|online| online.devices.iter().cloned().collect())
            .unwrap_or_default();
        for device in devices {
            Self::to_device(&mut hub, &device, Out::Frame(Box::new(frame.clone())));
        }
    }

    /// Broadcasts a `Down::Global` frame to all connected social sockets
    /// except `sender_device`, and appends it to the in-memory global history.
    pub(crate) fn broadcast_global(&self, sender_device: &str, frame: &Down) {
        {
            let mut history = self.global_history.lock();
            if history.len() >= GLOBAL_HISTORY_CAP {
                history.pop_front();
            }
            history.push_back(frame.clone());
        }
        let mut hub = self.hub.lock();
        let devices: Vec<String> = hub
            .conns
            .keys()
            .filter(|d| d.as_str() != sender_device)
            .cloned()
            .collect();
        for device in devices {
            Self::to_device(&mut hub, &device, Out::Frame(Box::new(frame.clone())));
        }
    }

    /// Recent `Down::Global` frames kept in memory.
    pub(crate) fn recent_global(&self) -> Vec<Down> {
        self.global_history.lock().iter().cloned().collect()
    }

    /// `account` opened a watch socket on `game`.
    pub(crate) fn watch_start(&self, account: &str, game: &str) {
        *self
            .watching
            .lock()
            .entry(account.to_owned())
            .or_default()
            .entry(game.to_owned())
            .or_default() += 1;
    }

    /// `account` closed a watch socket on `game`.
    pub(crate) fn watch_end(&self, account: &str, game: &str) {
        let mut watching = self.watching.lock();
        if let Some(games) = watching.get_mut(account) {
            if let Some(count) = games.get_mut(game) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    games.remove(game);
                }
            }
            if games.is_empty() {
                watching.remove(account);
            }
        }
    }

    /// Closes `device`'s socket, if open (removed or replaced).
    pub(crate) fn close_device(&self, device: &str, code: u16, reason: &'static str) {
        let mut hub = self.hub.lock();
        Self::to_device(&mut hub, device, Out::Close(code, reason));
    }

    /// Sends a delivery to `device`'s socket, if open.
    pub(crate) fn deliver(&self, device: &str, seq: i64, frame: Down) {
        let mut hub = self.hub.lock();
        Self::to_device(
            &mut hub,
            device,
            Out::Deliver {
                seq,
                frame: Box::new(frame),
            },
        );
    }

    /// Whether `account` has a social socket open.
    pub(crate) fn is_online(&self, account: &str) -> bool {
        self.hub.lock().online.contains_key(account)
    }

    /// How many social sockets are open, for the console.
    pub(crate) fn sockets(&self) -> usize {
        self.hub.lock().conns.len()
    }

    /// The accounts online now, with what presence needs of each.
    pub(crate) fn online_accounts(&self) -> Vec<presence::OnlineView> {
        self.hub
            .lock()
            .online
            .iter()
            .map(|(account, online)| presence::OnlineView {
                account: account.clone(),
                me: online.me.clone(),
                status: online.status,
                activity: online.activity,
                share_table: online.share_table,
                friends: online.friends.keys().cloned().collect(),
            })
            .collect()
    }

    /// A new status for `account`.
    pub(crate) fn set_status(&self, account: &str, set: &wire::PresenceSet) {
        if let Some(online) = self.hub.lock().online.get_mut(account) {
            online.status = set.status;
            online.activity = set.activity;
        }
    }

    /// `share_table` changed for `account`.
    pub(crate) fn set_share_table(&self, account: &str, share: bool) {
        if let Some(online) = self.hub.lock().online.get_mut(account) {
            online.share_table = share;
        }
    }

    /// `account`'s friends are now `friends` (after any change to them).
    pub(crate) fn set_friends(&self, account: &str, friends: BTreeMap<String, Person>) {
        if let Some(online) = self.hub.lock().online.get_mut(account) {
            online.friends = friends;
        }
    }

    /// Deleted accounts (#292): their sockets close, their friends are
    /// told the friendship is gone, and their table invites lapse.
    pub(crate) fn forget(&self, accounts: &[String]) {
        let gone: BTreeSet<&String> = accounts.iter().collect();
        let mut tell: Vec<(String, Down)> = Vec::new();
        {
            let mut hub = self.hub.lock();
            for account in accounts {
                if let Some(online) = hub.online.remove(account) {
                    for device in online.devices {
                        if let Some(conn) = hub.conns.remove(&device) {
                            let _ = conn.tx.try_send(Out::Close(1000, "account deleted"));
                        }
                    }
                }
            }
            for (account, online) in &mut hub.online {
                let left: Vec<String> = online
                    .friends
                    .keys()
                    .filter(|id| gone.contains(id))
                    .cloned()
                    .collect();
                for id in left {
                    if let Some(person) = online.friends.remove(&id) {
                        tell.push((
                            account.clone(),
                            Down::Friend {
                                event: wire::FriendEvent::Removed,
                                person,
                                presence: None,
                            },
                        ));
                    }
                }
            }
        }
        for (account, frame) in tell {
            self.push(&account, &frame);
        }
        let lapsed = self.invites.lock().forget(accounts);
        for (to, frame) in lapsed {
            self.push(&to, &frame);
        }
        let mut watching = self.watching.lock();
        for account in accounts {
            watching.remove(account);
        }
        let mut published = self.published.lock();
        for account in accounts {
            published.forget(account);
        }
    }
}

/// `who` as another account sees it.
pub(crate) fn person(who: &Who) -> Person {
    Person {
        tag: crate::handle::tag_text(who.tag),
        handle: crate::handle::handle(&who.name, who.tag),
    }
}

/// An account id as the database's key.
pub(crate) fn uid(account: &str) -> Option<Uuid> {
    Uuid::parse_str(account).ok()
}

/// A tag's text as the database's number, if it is one as the gateway
/// renders tags.
pub(crate) fn tag_number(tag: &str) -> Option<i32> {
    if !wire::valid_tag(tag) {
        return None;
    }
    i32::try_from(u32::from_str_radix(tag, 16).ok()?).ok()
}

/// A time as Unix milliseconds.
pub(crate) fn millis(at: time::OffsetDateTime) -> u64 {
    u64::try_from(at.unix_timestamp_nanos() / 1_000_000).unwrap_or(0)
}

/// Now, in Unix milliseconds.
pub(crate) fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The friends of `account`, by account id, from the database.
pub(crate) async fn friends_of(
    state: &Shared,
    account: Uuid,
) -> Result<BTreeMap<String, Person>, sea_orm::DbErr> {
    Ok(baylee_db::social::friends(&state.db, account)
        .await?
        .iter()
        .map(|(who, _)| (who.id.to_string(), person(who)))
        .collect())
}

/// Reloads the friends of each of `accounts` that is online, after a
/// change to them.
pub(crate) async fn refresh_friends(state: &Shared, accounts: &[Uuid]) {
    for account in accounts {
        let id = account.to_string();
        if !state.social.is_online(&id) {
            continue;
        }
        match friends_of(state, *account).await {
            Ok(friends) => state.social.set_friends(&id, friends),
            Err(e) => tracing::warn!("social: friends not reloaded: {e}"),
        }
    }
}

/// `GET /admin/stats`' `social`: counts only, `null` when off.
pub(crate) async fn console_counts(state: &crate::AppState) -> serde_json::Value {
    if !state.social.on() {
        return serde_json::Value::Null;
    }
    match baylee_db::social::counts(&state.db).await {
        Ok(counts) => serde_json::json!({
            "devices": counts.devices,
            "friendships": counts.friendships,
            "groups": counts.groups,
            "pending_slots": counts.pending_slots,
            "sockets": state.social.sockets(),
        }),
        Err(e) => {
            tracing::warn!("social: no counts: {e}");
            serde_json::Value::Null
        }
    }
}
