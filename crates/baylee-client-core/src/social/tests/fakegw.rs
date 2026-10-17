//! A fake gateway for the social machine's tests (§16): it applies §8's
//! rules in memory — device registration checked with `baylee-seal`, key
//! answers only for tags the caller may write to, the send checks
//! (header, membership, completeness), one slot per recipient device, acks
//! — and carries the machines' effects to each other. It never opens a
//! message: it sees what a real gateway sees.

#![allow(dead_code)] // helpers for every test module, not each one

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use baylee_protocol::social::{
    AccountKeys, Conv, Delivered, DeviceKey, DeviceList, DeviceRow as WireDeviceRow, Down, Friend,
    Header, KeysAnswer, Person, Presence, Sender, Shown, Snapshot, TableConv, Up, Why,
};
use baylee_seal::b64::url;
use baylee_seal::testkit::Seeded;

use crate::social::{
    ChatLine, ChatPhrase, ChatPrefs, Conversation, Effect, LineKind, Me, MemoryDisk, Method,
    Social, SocialLink, disk,
};

/// One registered device.
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub tag: String,
    pub ik: String,
    pub dk: String,
    pub dsig: String,
    pub label: String,
}

/// A delivery slot.
#[derive(Clone, Debug)]
pub struct Slot {
    pub seq: u64,
    pub frame: Down,
}

/// The gateway's state.
#[derive(Default)]
pub struct Gateway {
    pub handles: BTreeMap<String, String>,
    pub devices: Vec<Device>,
    pub friends: BTreeSet<(String, String)>,
    /// `tbl:`/`team:` conversations and their member tags.
    pub tables: BTreeMap<String, Vec<String>>,
    /// Pending slots per device.
    pub slots: BTreeMap<String, Vec<Slot>>,
    pub seq: u64,
    pub clock: u64,
    pub sent: BTreeSet<String>,
    /// Every frame a client sent, for assertions.
    pub ups: Vec<(String, Up)>,
    /// Whether a device's socket is up (welcomed).
    pub online: BTreeSet<String>,
    /// Refusals the next sends get, in order, before any rule is applied
    /// (a rate limit, a refusal the rules alone would not make here).
    pub refuse_sends: VecDeque<Why>,
}

impl Gateway {
    pub fn befriend(&mut self, a: &str, b: &str) {
        let (x, y) = if a < b { (a, b) } else { (b, a) };
        self.friends.insert((x.to_string(), y.to_string()));
    }

    pub fn friends_of(&self, tag: &str) -> Vec<String> {
        self.friends
            .iter()
            .filter_map(|(a, b)| {
                if a == tag {
                    Some(b.clone())
                } else if b == tag {
                    Some(a.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn person(&self, tag: &str) -> Person {
        Person {
            tag: tag.to_string(),
            handle: self.handles.get(tag).cloned().unwrap_or_default(),
        }
    }

    fn may_write(&self, from: &str, to: &str) -> bool {
        from == to
            || self.friends_of(from).iter().any(|t| t == to)
            || self
                .tables
                .values()
                .any(|m| m.iter().any(|t| t == from) && m.iter().any(|t| t == to))
    }

    /// The members of a conversation, by §5.
    pub fn members(&self, conv: &str) -> Option<Vec<String>> {
        match conv.parse::<Conv>().ok()? {
            Conv::Dm { a, b } => self.may_write(&a, &b).then(|| vec![a, b]),
            Conv::Table { .. } | Conv::Team { .. } => self.tables.get(conv).cloned(),
            Conv::Group { .. } | Conv::Global => None,
        }
    }

    pub fn snapshot(&self, tag: &str) -> Snapshot {
        Snapshot {
            friends: self
                .friends_of(tag)
                .into_iter()
                .map(|t| Friend {
                    person: self.person(&t),
                    since: 0,
                    presence: Presence {
                        status: Shown::Online,
                        place: None,
                    },
                })
                .collect(),
            tables: self
                .tables
                .iter()
                .filter(|(_, m)| m.iter().any(|t| t == tag))
                .map(|(conv, m)| TableConv {
                    conv: conv.clone(),
                    name: "Kitchen table".into(),
                    members: m.iter().map(|t| self.person(t)).collect(),
                })
                .collect(),
            ..Snapshot::default()
        }
    }

    /// The ids of the devices registered for `tag`, in registration order.
    pub fn device_ids(&self, tag: &str) -> Vec<String> {
        self.devices
            .iter()
            .filter(|d| d.tag == tag)
            .map(|d| d.id.clone())
            .collect()
    }

    /// The frames waiting in a device's slots.
    pub fn waiting(&self, dev: &str) -> Vec<Down> {
        self.slots
            .get(dev)
            .into_iter()
            .flatten()
            .map(|s| s.frame.clone())
            .collect()
    }

    /// How many `Up::Send` frames the device `dev` has put on the wire.
    pub fn sends_from(&self, dev: &str) -> usize {
        self.ups
            .iter()
            .filter(|(d, up)| d == dev && matches!(up, Up::Send { .. }))
            .count()
    }

    /// `tag` as the gateway names a person.
    pub fn person_of(&self, tag: &str) -> Person {
        self.person(tag)
    }

    fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Answers an HTTP effect from the device `from_dev` of `tag`.
    pub fn http(
        &mut self,
        tag: &str,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> (u16, Vec<u8>) {
        if method == Method::Post && path == "/social/devices" {
            let body = body.expect("a body");
            let field = |k: &str| body[k].as_str().unwrap_or_default().to_string();
            let (Ok(ik), Ok(dk), Ok(dsig)) = (
                url::decode_array::<32>(&field("ik")),
                url::decode_array::<32>(&field("dk")),
                url::decode_array::<64>(&field("dsig")),
            ) else {
                return (400, Vec::new());
            };
            if baylee_seal::verify_device(tag, &ik, &dk, &dsig).is_err() {
                return (400, Vec::new());
            }
            let id = baylee_seal::device_id(&ik);
            self.devices.retain(|d| d.id != id);
            self.devices.push(Device {
                id: id.clone(),
                tag: tag.to_string(),
                ik: field("ik"),
                dk: field("dk"),
                dsig: field("dsig"),
                label: field("label"),
            });
            return (
                201,
                serde_json::to_vec(&serde_json::json!({ "id": id })).unwrap(),
            );
        }
        if method == Method::Get && path == "/social/devices" {
            let list = DeviceList {
                devices: self
                    .devices
                    .iter()
                    .filter(|d| d.tag == tag)
                    .map(|d| WireDeviceRow {
                        id: d.id.clone(),
                        label: d.label.clone(),
                        added_at: 0,
                        seen_at: 0,
                    })
                    .collect(),
            };
            return (200, serde_json::to_vec(&list).unwrap());
        }
        if let Some(query) = path.strip_prefix("/social/keys?tag=") {
            let keys = query
                .split(',')
                .filter(|t| self.may_write(tag, t))
                .filter(|t| self.handles.contains_key(*t))
                .map(|t| AccountKeys {
                    tag: t.to_string(),
                    handle: self.handles[t].clone(),
                    devices: self
                        .devices
                        .iter()
                        .filter(|d| d.tag == t)
                        .map(|d| DeviceKey {
                            id: d.id.clone(),
                            ik: d.ik.clone(),
                            dk: d.dk.clone(),
                            dsig: d.dsig.clone(),
                            label: d.label.clone(),
                            added_at: 0,
                        })
                        .collect(),
                })
                .collect();
            return (200, serde_json::to_vec(&KeysAnswer { keys }).unwrap());
        }
        (204, Vec::new())
    }

    /// Answers a frame from device `dev` of `tag`: the frames that go back
    /// to it now. Deliveries to others go into their slots.
    #[allow(clippy::too_many_lines)] // §8.2's send checks, in order
    pub fn up(&mut self, tag: &str, dev: &str, frame: Up) -> Vec<Down> {
        self.ups.push((dev.to_string(), frame.clone()));
        match frame {
            Up::Hello { device, .. } => {
                if self.device(&device).is_none_or(|d| d.tag != tag) {
                    return vec![Down::Refused {
                        req: None,
                        why: Why::Device,
                        missing: Vec::new(),
                        extra: Vec::new(),
                        message: None,
                    }];
                }
                self.online.insert(device.clone());
                let mut out = vec![
                    Down::Welcome {
                        tag: tag.to_string(),
                        handle: self.handles[tag].clone(),
                        device: device.clone(),
                        guest: false,
                    },
                    Down::Snapshot(self.snapshot(tag)),
                ];
                out.extend(
                    self.slots
                        .get(&device)
                        .into_iter()
                        .flatten()
                        .map(|s| s.frame.clone()),
                );
                out
            }
            Up::Ack { upto } => {
                if let Some(slots) = self.slots.get_mut(dev) {
                    slots.retain(|s| s.seq > upto);
                }
                Vec::new()
            }
            Up::Presence(_) => Vec::new(),
            Up::Send { req, conv, sealed } => {
                let refuse = |why: Why, missing: Vec<String>, extra: Vec<String>| {
                    vec![Down::Refused {
                        req: Some(req),
                        why,
                        missing,
                        extra,
                        message: None,
                    }]
                };
                if let Some(why) = self.refuse_sends.pop_front() {
                    return refuse(why, vec![], vec![]);
                }
                let Ok(hdr) = url::decode(&sealed.hdr) else {
                    return refuse(Why::BadSealed, vec![], vec![]);
                };
                let Ok(header) = serde_json::from_slice::<Header>(&hdr) else {
                    return refuse(Why::BadSealed, vec![], vec![]);
                };
                if header.from != tag || header.dev != dev || header.conv != conv {
                    return refuse(Why::BadSealed, vec![], vec![]);
                }
                let Some(members) = self.members(&conv) else {
                    return refuse(Why::NotMember, vec![], vec![]);
                };
                if !members.iter().any(|m| m == tag) {
                    return refuse(Why::NotMember, vec![], vec![]);
                }
                let want: BTreeSet<String> = self
                    .devices
                    .iter()
                    .filter(|d| members.contains(&d.tag) && d.id != dev)
                    .map(|d| d.id.clone())
                    .collect();
                let got: BTreeSet<String> = sealed.wraps.iter().map(|w| w.d.clone()).collect();
                if want != got {
                    return refuse(
                        Why::Devices,
                        want.difference(&got).cloned().collect(),
                        got.difference(&want).cloned().collect(),
                    );
                }
                if !self.sent.insert(header.id.clone()) {
                    return refuse(Why::Duplicate, vec![], vec![]);
                }
                self.clock += 1;
                let at = self.clock;
                for wrap in &sealed.wraps {
                    self.seq += 1;
                    let frame = Down::Msg {
                        seq: self.seq,
                        at,
                        conv: conv.clone(),
                        from: Sender {
                            tag: tag.to_string(),
                            handle: self.handles[tag].clone(),
                            device: dev.to_string(),
                        },
                        sealed: Delivered {
                            hdr: sealed.hdr.clone(),
                            n: sealed.n.clone(),
                            ct: sealed.ct.clone(),
                            epk: sealed.epk.clone(),
                            sig: sealed.sig.clone(),
                            k: wrap.k.clone(),
                        },
                    };
                    self.slots.entry(wrap.d.clone()).or_default().push(Slot {
                        seq: self.seq,
                        frame,
                    });
                }
                vec![Down::Sent {
                    req,
                    id: header.id,
                    at,
                }]
            }
            Up::Global { req, id, body } => {
                self.clock += 1;
                let at = self.clock;
                vec![
                    Down::Sent {
                        req,
                        id: id.clone(),
                        at,
                    },
                    Down::Global {
                        id,
                        at,
                        from: Sender {
                            tag: tag.to_string(),
                            handle: self.handles[tag].clone(),
                            device: dev.to_string(),
                        },
                        body,
                    },
                ]
            }
        }
    }
}

/// One client: a machine, its disk, its stored secret, its entropy.
pub struct Client {
    pub social: Social,
    pub disk: MemoryDisk,
    pub secrets: Option<String>,
    pub rng: Seeded,
    pub now: u64,
    pub tag: String,
    /// The handle this account signed in with.
    pub handle: String,
    /// The chat settings this device runs with.
    pub prefs: ChatPrefs,
    /// Every effect the machine produced, in order, disk effects included.
    pub log: Vec<Effect>,
    /// Frames the gateway answers are lost (the link broke before they
    /// came): the gateway still handles the send.
    pub drop_answers: bool,
    /// Effects that were not for the gateway (toasts, sounds, …).
    pub seen: Vec<Effect>,
    /// Frames waiting for this client.
    pub inbox: Vec<Down>,
    pub linked: bool,
}

impl Client {
    /// A new device for `tag` (`handle`), entropy from `seed`.
    pub fn new(gw: &mut Gateway, tag: &str, handle: &str, seed: u64) -> Self {
        Self::with_prefs(gw, tag, handle, seed, ChatPrefs::default())
    }

    /// A new device that runs with `prefs`.
    pub fn with_prefs(
        gw: &mut Gateway,
        tag: &str,
        handle: &str,
        seed: u64,
        prefs: ChatPrefs,
    ) -> Self {
        gw.handles.insert(tag.to_string(), handle.to_string());
        let mut rng = Seeded::new(seed);
        let me = Me {
            tag: tag.to_string(),
            handle: handle.to_string(),
            guest: false,
            gateway: "https://gw.test".into(),
        };
        let (social, effects) = Social::start(me, None, None, prefs.clone(), 1_000, &mut rng);
        let mut client = Self {
            social,
            disk: MemoryDisk::new(),
            secrets: None,
            rng,
            now: 1_000,
            tag: tag.to_string(),
            handle: handle.to_string(),
            prefs,
            log: Vec::new(),
            drop_answers: false,
            seen: Vec::new(),
            inbox: Vec::new(),
            linked: false,
        };
        client.run(gw, effects);
        client
    }

    /// This device's id.
    pub fn device(&self) -> String {
        self.social_device()
    }

    /// Carries out effects until none are left for the gateway.
    pub fn run(&mut self, gw: &mut Gateway, effects: Vec<Effect>) {
        let mut queue: std::collections::VecDeque<Effect> = effects.into();
        let mut guard = 0;
        while let Some(effect) = queue.pop_front() {
            guard += 1;
            assert!(guard < 10_000, "effects never settle");
            self.log.push(effect.clone());
            if let Some(result) = disk::apply(&effect, &mut self.disk) {
                result.expect("memory disk");
                continue;
            }
            match effect {
                Effect::StoreSecrets(value) => self.secrets = Some(value.expose().to_string()),
                Effect::ForgetSecrets => self.secrets = None,
                Effect::Http {
                    tag,
                    method,
                    path,
                    body,
                } => {
                    let (status, answer) = gw.http(&self.tag, method, &path, body.as_ref());
                    let more = self
                        .social
                        .on_http(tag, status, &answer, self.now, &mut self.rng);
                    queue.extend(more);
                }
                Effect::Dial => {
                    self.linked = true;
                    queue.extend(self.social.on_link(SocialLink::Up, self.now));
                }
                Effect::Hang => self.linked = false,
                Effect::Up(frame) => {
                    let dev = self.social_device();
                    let lost = self.drop_answers && matches!(frame, Up::Send { .. });
                    let back = gw.up(&self.tag, &dev, frame);
                    if lost {
                        continue;
                    }
                    for down in back {
                        let more = self.social.on_frame(down, self.now, &mut self.rng);
                        queue.extend(more);
                    }
                }
                other => self.seen.push(other),
            }
        }
    }

    fn social_device(&self) -> String {
        // The Hello names it; read it back from the state the machine
        // keeps (its own device row, else its first registration).
        self.social.device_id().to_string()
    }

    /// Takes everything waiting in the gateway's slots for this device.
    pub fn deliver(&mut self, gw: &mut Gateway) {
        let dev = self.social_device();
        let frames: Vec<Down> = gw
            .slots
            .get(&dev)
            .into_iter()
            .flatten()
            .map(|s| s.frame.clone())
            .collect();
        for frame in frames {
            let effects = self.social.on_frame(frame, self.now, &mut self.rng);
            self.run(gw, effects);
        }
    }

    /// Types `text` and presses Enter.
    pub fn say(&mut self, gw: &mut Gateway, text: &str) {
        self.social.composer_mut().typed(text);
        let (_, effects) =
            self.social
                .key(crate::social::ComposerKey::Enter, self.now, &mut self.rng);
        self.run(gw, effects);
    }

    /// The texts of a conversation's message lines.
    pub fn texts(&self, conv: &str) -> Vec<String> {
        let id: Conv = conv.parse().expect("a conversation");
        self.social
            .conversation(&id)
            .map(|c| {
                c.lines
                    .iter()
                    .filter_map(|l| l.body().map(|b| b.text.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Driving a client further: restarts, the clock, the link, injected frames
/// and a reader's view of the lines.
impl Client {
    /// Hands the machine one frame from the socket and carries out what
    /// comes of it.
    pub fn feed(&mut self, gw: &mut Gateway, frame: Down) {
        let effects = self.social.on_frame(frame, self.now, &mut self.rng);
        self.run(gw, effects);
    }

    /// Quits and starts again from what is on its disk and in its secret
    /// store, as the app does: the state file's bytes, the stored secret,
    /// this device's settings. The history is not restored; call
    /// [`Client::restore`] for that.
    pub fn restart(&mut self, gw: &mut Gateway) {
        let me = Me {
            tag: self.tag.clone(),
            handle: self.handle.clone(),
            guest: false,
            gateway: "https://gw.test".into(),
        };
        let stored = disk::ChatDisk::read(&self.disk, disk::STATE_FILE);
        let (social, effects) = Social::start(
            me,
            stored,
            self.secrets.clone(),
            self.prefs.clone(),
            self.now,
            &mut self.rng,
        );
        self.social = social;
        self.linked = false;
        self.run(gw, effects);
    }

    /// Loads the history from the disk; returns what the machine said.
    pub fn restore(&mut self, gw: &mut Gateway) -> Vec<Effect> {
        let effects = self.social.restore(&self.disk, self.now, &mut self.rng);
        self.run(gw, effects.clone());
        effects
    }

    /// The player asks for something.
    pub fn act(&mut self, gw: &mut Gateway, intent: crate::social::Intent) -> Vec<Effect> {
        let effects = self.social.act(intent, self.now, &mut self.rng);
        self.run(gw, effects.clone());
        effects
    }

    /// The socket closed.
    pub fn link_down(&mut self, gw: &mut Gateway) {
        self.linked = false;
        let effects = self.social.on_link(SocialLink::Down, self.now);
        self.run(gw, effects);
    }

    /// The socket opened: the machine says Hello and goes on from the
    /// gateway's Welcome.
    pub fn link_up(&mut self, gw: &mut Gateway) {
        self.linked = true;
        let effects = self.social.on_link(SocialLink::Up, self.now);
        self.run(gw, effects);
    }

    /// The clock reached `now`: the machine's tick, then whatever waits to
    /// be sealed.
    pub fn tick(&mut self, gw: &mut Gateway, now: u64) {
        self.now = now;
        let effects = self.social.tick(now, 0);
        self.run(gw, effects);
        let effects = self.social.flush(now, &mut self.rng);
        self.run(gw, effects);
    }

    /// The conversation `conv`, if the machine has it.
    pub fn conv(&self, conv: &str) -> Option<&Conversation> {
        let id: Conv = conv.parse().expect("a conversation");
        self.social.conversation(&id)
    }

    /// Every line of `conv`.
    pub fn lines(&self, conv: &str) -> Vec<&ChatLine> {
        self.conv(conv)
            .map_or_else(Vec::new, |c| c.lines.iter().collect())
    }

    /// The message lines of `conv` (not the notices).
    pub fn message_lines(&self, conv: &str) -> Vec<&ChatLine> {
        self.lines(conv)
            .into_iter()
            .filter(|l| matches!(l.kind, LineKind::Message { .. }))
            .collect()
    }

    /// The notice lines of `conv`: what they say and their values.
    pub fn notices(&self, conv: &str) -> Vec<(ChatPhrase, Vec<String>)> {
        self.lines(conv)
            .into_iter()
            .filter_map(|l| match &l.kind {
                LineKind::Notice { phrase, args } => Some((*phrase, args.clone())),
                _ => None,
            })
            .collect()
    }

    /// How many message lines the machine holds in all its conversations.
    pub fn message_count(&self) -> usize {
        self.social
            .conversations()
            .iter()
            .flat_map(|c| c.lines.iter())
            .filter(|l| matches!(l.kind, LineKind::Message { .. }))
            .count()
    }

    /// The `upto` of every ack this client sent, in order.
    pub fn acks(&self) -> Vec<u64> {
        self.log
            .iter()
            .filter_map(|e| match e {
                Effect::Up(Up::Ack { upto }) => Some(*upto),
                _ => None,
            })
            .collect()
    }

    /// Whether the effects hold a toast or a sound.
    pub fn made_noise(effects: &[Effect]) -> bool {
        effects
            .iter()
            .any(|e| matches!(e, Effect::Toast(_) | Effect::Sound(_)))
    }
}
