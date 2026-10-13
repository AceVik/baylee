//! Chat routing end to end (`docs/protocol.md` §"Friends and chat
//! (beta.8)"): sealed messages through a real gateway between real social
//! sockets. The gateway carries what it cannot open, so every message here is
//! sealed on the sender's device (`baylee-seal`) for the recipients' devices
//! and opened on theirs; what the gateway decides is who may write to a
//! conversation, that the wraps name exactly the right devices, what is held
//! for a device that is away, and how hard a socket may push.

#![allow(clippy::missing_docs_in_private_items, dead_code)]

mod common;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use baylee_protocol::social::{
    self as wire, Conv, Delivered, Down, Header, PresenceSet, Sender, Snapshot, Status, Up, Why,
    Wrap,
};
use baylee_seal::b64::url as b64;
use baylee_seal::testkit::{self, Seeded};
use baylee_seal::{DeviceSecrets, Envelope, Recipient};
use common::{Gateway, Socket, WAIT_BUDGET, ask_ticket, http, json_field, login, spawn_gateway};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

// ---------------------------------------------------------------------------
// The harness: accounts with chat devices, a social socket that speaks the
// wire's own frames, and sealing as a real client does it. Self-contained on
// purpose: each social test file carries its own copy.
// ---------------------------------------------------------------------------

/// An account on the gateway under test.
#[derive(Clone)]
struct Acct {
    port: u16,
    session: String,
    tag: String,
    handle: String,
    /// The account id (a UUID), which no route says but `/me`.
    id: String,
}

/// A registered account (`POST /auth/register`) and what `/me` says of it.
fn account(port: u16, username: &str, name: &str) -> Acct {
    let session = login(port, username, name);
    let (status, body) = http(port, "GET", "/me", Some(&session), "");
    assert_eq!(status, 200, "/me: {body}");
    Acct {
        port,
        session,
        tag: json_field(&body, "tag").to_owned(),
        handle: json_field(&body, "handle").to_owned(),
        id: json_field(&body, "id").to_owned(),
    }
}

/// One chat device of an account: its secrets, as the client keeps them.
struct Dev {
    acct: Acct,
    secrets: DeviceSecrets,
    device: String,
}

/// A seed no other device of this test process has drawn: the same seed is
/// the same keys, which the gateway refuses on a second account (`Taken`).
fn device_seed() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// The seed of one message's randomness, in a range of its own.
fn message_seed() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1 << 40);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// `POST /social/devices` for a new device of `acct`.
fn register(acct: &Acct) -> Dev {
    let (secrets, public) = testkit::registered(&acct.tag, device_seed());
    let body = serde_json::json!({
        "ik": b64::encode(&public.ik),
        "dk": b64::encode(&public.dk),
        "dsig": b64::encode(&public.dsig),
        "label": "macOS",
    })
    .to_string();
    let (status, answer) = http(
        acct.port,
        "POST",
        "/social/devices",
        Some(&acct.session),
        &body,
    );
    assert_eq!(status, 201, "register a device: {answer}");
    assert_eq!(json_field(&answer, "id"), public.id);
    Dev {
        acct: acct.clone(),
        secrets,
        device: public.id,
    }
}

/// Makes `a` and `b` friends: a request, then its acceptance.
fn befriend(a: &Acct, b: &Acct) {
    let body = serde_json::json!({ "handle": b.handle }).to_string();
    let (status, answer) = http(
        a.port,
        "POST",
        "/social/friends/requests",
        Some(&a.session),
        &body,
    );
    assert_eq!(status, 201, "friend request: {answer}");
    let path = format!("/social/friends/requests/{}/accept", a.tag);
    let (status, answer) = http(b.port, "POST", &path, Some(&b.session), "");
    assert!(status < 300, "accept: {status} {answer}");
}

/// A socket of one device, speaking the social frames.
struct Chat {
    ws: Socket,
}

/// What a read of the socket came to.
enum Event {
    Frame(Box<Down>),
    /// The socket closed, with the code it said (none when it said none).
    Closed(Option<u16>),
}

/// The event a websocket message is, if it is one: pings and pongs are not.
fn event_of(next: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>) -> Option<Event> {
    match next {
        Some(Ok(Message::Text(text))) => Some(Event::Frame(Box::new(
            serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("not a Down frame ({e}): {text}")),
        ))),
        Some(Ok(Message::Close(frame))) => {
            Some(Event::Closed(frame.map(|frame| u16::from(frame.code))))
        }
        Some(Ok(_)) => None,
        Some(Err(_)) | None => Some(Event::Closed(None)),
    }
}

/// The social socket's URL with a fresh ticket (a ticket is good for one
/// upgrade), and whatever `edit` does to the path.
fn social_url(acct: &Acct, edit: impl FnOnce(String) -> String) -> String {
    let (status, body) = ask_ticket(acct.port, &acct.session, r#"{"socket":"social"}"#);
    assert_eq!(status, 200, "a social ticket: {body}");
    let path = baylee_protocol::social::social_socket_path(json_field(&body, "ticket"));
    format!("ws://127.0.0.1:{}{}", acct.port, edit(path))
}

fn hello(device: &str, status: Status) -> Up {
    Up::Hello {
        device: device.to_owned(),
        presence: PresenceSet {
            status,
            activity: None,
        },
    }
}

impl Chat {
    /// Opens the socket of `dev` as online: its hello, the gateway's
    /// welcome and its snapshot.
    async fn open(dev: &Dev) -> (Self, Snapshot) {
        Self::open_as(dev, Status::Online).await
    }

    async fn open_as(dev: &Dev, status: Status) -> (Self, Snapshot) {
        let mut chat = Self::dial(&dev.acct, |path| path).await;
        chat.up(&hello(&dev.device, status)).await;
        let welcome = chat.next_frame().await;
        let Down::Welcome { tag, device, .. } = welcome else {
            panic!("the first frame is the welcome: {welcome:?}");
        };
        assert_eq!(
            (tag.as_str(), device.as_str()),
            (dev.acct.tag.as_str(), dev.device.as_str())
        );
        let snapshot = chat.next_frame().await;
        let Down::Snapshot(snapshot) = snapshot else {
            panic!("the welcome is followed by the snapshot: {snapshot:?}");
        };
        (chat, snapshot)
    }

    /// A socket as far as the upgrade: no hello yet.
    async fn dial(acct: &Acct, edit: impl FnOnce(String) -> String) -> Self {
        let url = social_url(acct, edit);
        let ws = common::dial(&url).await.expect("the social socket opens");
        Self { ws }
    }

    async fn up(&mut self, frame: &Up) {
        self.raw(Message::Text(
            serde_json::to_string(frame).expect("a frame").into(),
        ))
        .await;
    }

    /// A message as it is, whatever it is; a socket that is gone is fine.
    async fn raw(&mut self, message: Message) {
        let _ = self.ws.send(message).await;
    }

    /// The next event, within the suite's budget.
    async fn event(&mut self) -> Event {
        tokio::time::timeout(WAIT_BUDGET, async {
            loop {
                if let Some(event) = event_of(self.ws.next().await) {
                    return event;
                }
            }
        })
        .await
        .expect("a frame within the budget")
    }

    /// Reads frames until `pick` takes one; the rest are skipped.
    async fn until<T>(&mut self, mut pick: impl FnMut(&Down) -> Option<T>) -> T {
        loop {
            match self.event().await {
                Event::Frame(frame) => {
                    if let Some(found) = pick(&frame) {
                        return found;
                    }
                }
                Event::Closed(code) => panic!("the socket closed ({code:?}) first"),
            }
        }
    }

    /// Every frame that arrives within `window`, and whether the socket
    /// closed in it.
    async fn quiet(&mut self, window: Duration) -> (Vec<Down>, Option<Option<u16>>) {
        let end = tokio::time::Instant::now() + window;
        let mut frames = Vec::new();
        loop {
            let left = end.saturating_duration_since(tokio::time::Instant::now());
            let Ok(next) = tokio::time::timeout(left, self.ws.next()).await else {
                return (frames, None);
            };
            match event_of(next) {
                Some(Event::Frame(frame)) => frames.push(*frame),
                Some(Event::Closed(code)) => return (frames, Some(code)),
                None => {}
            }
        }
    }

    /// Reads until the socket closes; the code it said.
    async fn closed(&mut self) -> Option<u16> {
        loop {
            if let Event::Closed(code) = self.event().await {
                return code;
            }
        }
    }

    /// The next frame, whatever it is.
    async fn next_frame(&mut self) -> Down {
        self.until(|frame| Some(frame.clone())).await
    }

    /// The next delivery.
    async fn msg(&mut self) -> Incoming {
        self.until(|frame| match frame {
            Down::Msg {
                seq,
                at,
                conv,
                from,
                sealed,
            } => Some(Incoming {
                seq: *seq,
                at: *at,
                conv: conv.clone(),
                from: from.clone(),
                sealed: sealed.clone(),
            }),
            _ => None,
        })
        .await
    }

    /// The gateway's answer to send `req`.
    async fn answer(&mut self, req: u32) -> Down {
        self.until(|frame| match frame {
            Down::Sent { req: r, .. } | Down::Refused { req: Some(r), .. } if *r == req => {
                Some(frame.clone())
            }
            _ => None,
        })
        .await
    }

    /// Sends `frame` (a `Send` for `req`) and waits for the answer.
    async fn send(&mut self, req: u32, frame: &Up) -> Down {
        self.up(frame).await;
        self.answer(req).await
    }

    /// The deliveries that arrive within `window` (none is the point).
    async fn deliveries_within(&mut self, window: Duration) -> Vec<Incoming> {
        let (frames, closed) = self.quiet(window).await;
        assert_eq!(closed, None, "the socket closed");
        frames
            .into_iter()
            .filter_map(|frame| match frame {
                Down::Msg {
                    seq,
                    at,
                    conv,
                    from,
                    sealed,
                } => Some(Incoming {
                    seq,
                    at,
                    conv,
                    from,
                    sealed,
                }),
                _ => None,
            })
            .collect()
    }
}

/// A `Down::Msg`, taken apart.
struct Incoming {
    seq: u64,
    at: u64,
    conv: String,
    from: Sender,
    sealed: Delivered,
}

/// A device a message is sealed for.
struct Target {
    device: String,
    dk: [u8; 32],
}

impl Dev {
    fn target(&self) -> Target {
        Target {
            device: self.device.clone(),
            dk: self.secrets.dk(),
        }
    }
}

fn targets(devs: &[&Dev]) -> Vec<Target> {
    devs.iter().map(|dev| dev.target()).collect()
}

/// The devices of `tags` as `GET /social/keys` gives them, the sending
/// device left out (it holds the message already).
fn keys_for(acct: &Acct, tags: &[&str], except: &str) -> Vec<Target> {
    let query: Vec<String> = tags.iter().map(|tag| format!("tag={tag}")).collect();
    let path = format!("/social/keys?{}", query.join("&"));
    let (status, body) = http(acct.port, "GET", &path, Some(&acct.session), "");
    assert_eq!(status, 200, "keys: {body}");
    let answer: wire::KeysAnswer = serde_json::from_str(&body).expect("a keys answer");
    answer
        .keys
        .iter()
        .flat_map(|account| &account.devices)
        .filter(|device| device.id != except)
        .map(|device| Target {
            device: device.id.clone(),
            dk: b64::decode_array(&device.dk).expect("a device key"),
        })
        .collect()
}

fn now_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after 1970")
            .as_millis(),
    )
    .expect("a time")
}

/// The header a well-behaved `sender` writes for a message to `conv` now.
fn header(sender: &Dev, conv: &str) -> Header {
    Header {
        v: 1,
        id: uuid::Uuid::now_v7().to_string(),
        conv: conv.to_owned(),
        from: sender.acct.tag.clone(),
        dev: sender.device.clone(),
        ts: now_ms(),
        kind: "text".to_owned(),
    }
}

/// The body a text message has.
fn text_body(text: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({ "text": text })).expect("a body")
}

/// `body` sealed under `header` by `sender` for `to`, as the frame that
/// sends it to `conv` (which is not always the header's).
fn seal_up(sender: &Dev, req: u32, conv: &str, header: &Header, body: &[u8], to: &[Target]) -> Up {
    let hdr = serde_json::to_vec(header).expect("a header");
    let recipients: Vec<Recipient<'_>> = to
        .iter()
        .map(|target| Recipient {
            device: &target.device,
            dk: target.dk,
        })
        .collect();
    let sealed = baylee_seal::seal(
        &sender.secrets,
        &hdr,
        body,
        &recipients,
        &mut Seeded::new(message_seed()),
    )
    .expect("sealed");
    Up::Send {
        req,
        conv: conv.to_owned(),
        sealed: wire::Sealed {
            hdr: b64::encode(&hdr),
            n: b64::encode(&sealed.n),
            ct: b64::encode(&sealed.ct),
            epk: b64::encode(&sealed.epk),
            sig: b64::encode(&sealed.sig),
            wraps: sealed
                .wraps
                .iter()
                .map(|wrap| Wrap {
                    d: wrap.device.clone(),
                    k: b64::encode(&wrap.k),
                })
                .collect(),
        },
    }
}

/// A text message from `sender` to `conv`, sealed for `to`.
fn text(sender: &Dev, req: u32, conv: &str, words: &str, to: &[Target]) -> Up {
    seal_up(
        sender,
        req,
        conv,
        &header(sender, conv),
        &text_body(words),
        to,
    )
}

/// What the recipient device `me` makes of `message` from `sender`.
fn open(me: &Dev, sender: &Dev, message: &Delivered) -> baylee_seal::Opened {
    let decode = |field: &str| b64::decode(field).expect("base64url");
    let (hdr, n, ct, epk, sig, k) = (
        decode(&message.hdr),
        decode(&message.n),
        decode(&message.ct),
        decode(&message.epk),
        decode(&message.sig),
        decode(&message.k),
    );
    let envelope = Envelope {
        hdr: &hdr,
        n: &n,
        ct: &ct,
        epk: &epk,
        sig: &sig,
    };
    baylee_seal::open(&me.secrets, &sender.secrets.ik(), &envelope, &k).expect("the message opens")
}

/// The `Sent` an answer is: the message id and its time.
fn sent(answer: &Down) -> (String, u64) {
    match answer {
        Down::Sent { id, at, .. } => (id.clone(), *at),
        other => panic!("not stored: {other:?}"),
    }
}

/// The reason an answer refuses with, and its device lists.
fn refusal(answer: &Down) -> (Why, Vec<String>, Vec<String>) {
    match answer {
        Down::Refused {
            why,
            missing,
            extra,
            ..
        } => (*why, missing.clone(), extra.clone()),
        other => panic!("not refused: {other:?}"),
    }
}

/// Waits until `query` (in the gateway's schema) answers `want`.
async fn becomes(gw: &Gateway, query: &str, want: i64) {
    for _ in 0..common::WAIT_TRIES {
        if gw.scalar(query) == want {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("{query} never answered {want}: {}", gw.scalar(query));
}

/// How many undelivered slots `device` holds.
fn slots(gw: &Gateway, device: &str) -> i64 {
    gw.scalar(&format!(
        "SELECT count(*) FROM chat_delivery WHERE device_id = '{device}'"
    ))
}

/// How many messages the gateway holds.
fn stored(gw: &Gateway) -> i64 {
    gw.scalar("SELECT count(*) FROM chat_message")
}

fn dm(a: &Acct, b: &Acct) -> String {
    Conv::dm(&a.tag, &b.tag).expect("two tags").to_string()
}

/// How long a socket is watched for a frame that must not come: the
/// gateway answers `Sent` only after it has queued every delivery, so by
/// then a wrong one is already on its way.
const SILENCE: Duration = Duration::from_millis(900);

#[tokio::test]
async fn a_friends_message_reaches_the_friend_and_the_senders_other_device_only() {
    let gw = spawn_gateway("social-online");
    let port = gw.port;
    let alice = account(port, "alice-on", "Alice");
    let bob = account(port, "bob-on", "Bob");
    befriend(&alice, &bob);
    let (a1, a2, b1) = (register(&alice), register(&alice), register(&bob));
    let (mut sock_a1, _) = Chat::open(&a1).await;
    let (mut sock_a2, _) = Chat::open(&a2).await;
    let (mut sock_b1, _) = Chat::open(&b1).await;

    // The recipients are found where a client finds them: the key directory.
    let to = keys_for(&alice, &[&alice.tag, &bob.tag], &a1.device);
    assert_eq!(
        to.len(),
        2,
        "the other device of the sender and the friend's"
    );
    let conv = dm(&alice, &bob);
    let answer = sock_a1
        .send(7, &text(&a1, 7, &conv, "hello bob", &to))
        .await;
    let (id, at) = sent(&answer);
    assert!(at > 0 && !id.is_empty());

    let got = sock_b1.msg().await;
    assert_eq!(got.conv, conv);
    assert_eq!(got.from.tag, alice.tag);
    assert_eq!(got.from.handle, alice.handle);
    assert_eq!(got.from.device, a1.device);
    assert_eq!(got.at, at, "the gateway's time is the order");
    let opened = open(&b1, &a1, &got.sealed);
    assert_eq!(&opened.body[..], &text_body("hello bob")[..]);
    let hdr: Header = serde_json::from_slice(&b64::decode(&got.sealed.hdr).unwrap()).unwrap();
    assert_eq!(
        (hdr.id, hdr.from, hdr.dev),
        (id.clone(), alice.tag.clone(), a1.device.clone())
    );

    // The sender's other device is sent it too, with a wrap of its own.
    let echoed = sock_a2.msg().await;
    assert_eq!(echoed.from.device, a1.device);
    assert_ne!(echoed.sealed.k, got.sealed.k, "a wrap per device");
    assert_eq!(
        &open(&a2, &a1, &echoed.sealed).body[..],
        &text_body("hello bob")[..]
    );

    // The device that sent it is not.
    assert!(sock_a1.deliveries_within(SILENCE).await.is_empty());
    assert_eq!(slots(&gw, &a1.device), 0);
    assert_eq!(slots(&gw, &b1.device), 1);
    assert_eq!(slots(&gw, &a2.device), 1);

    // Only the recipient's wrap opens on the recipient's device.
    let wrong = b1.secrets.dk() != a2.secrets.dk();
    assert!(wrong);
    let hdr = b64::decode(&got.sealed.hdr).unwrap();
    let (n, ct, epk, sig) = (
        b64::decode(&got.sealed.n).unwrap(),
        b64::decode(&got.sealed.ct).unwrap(),
        b64::decode(&got.sealed.epk).unwrap(),
        b64::decode(&got.sealed.sig).unwrap(),
    );
    let envelope = Envelope {
        hdr: &hdr,
        n: &n,
        ct: &ct,
        epk: &epk,
        sig: &sig,
    };
    let k = b64::decode(&got.sealed.k).unwrap();
    assert!(baylee_seal::open(&a2.secrets, &a1.secrets.ik(), &envelope, &k).is_err());
}

#[tokio::test]
async fn a_message_waits_for_a_device_that_is_away_until_it_acks() {
    let gw = spawn_gateway("social-offline");
    let port = gw.port;
    let alice = account(port, "alice-off", "Alice");
    let bob = account(port, "bob-off", "Bob");
    befriend(&alice, &bob);
    let (a1, b1) = (register(&alice), register(&bob));
    let conv = dm(&alice, &bob);
    let (mut sock_a, _) = Chat::open(&a1).await;

    // Bob is away: the gateway holds one slot for his device.
    let to = targets(&[&b1]);
    let answer = sock_a
        .send(1, &text(&a1, 1, &conv, "while you were out", &to))
        .await;
    let (id, _) = sent(&answer);
    assert_eq!(slots(&gw, &b1.device), 1);
    assert_eq!(stored(&gw), 1);

    // He connects: the snapshot comes first, then what was held.
    let (mut sock_b, snapshot) = Chat::open(&b1).await;
    assert!(snapshot.friends.iter().any(|f| f.person.tag == alice.tag));
    let got = sock_b.msg().await;
    assert_eq!(got.from.tag, alice.tag);
    assert_eq!(
        &open(&b1, &a1, &got.sealed).body[..],
        &text_body("while you were out")[..]
    );
    // Shown or stored is not yet acked: the slot is still there.
    assert_eq!(slots(&gw, &b1.device), 1);

    // The ack deletes it, and the sealed message with its last slot.
    sock_b.up(&Up::Ack { upto: got.seq }).await;
    becomes(
        &gw,
        &format!(
            "SELECT count(*) FROM chat_delivery WHERE device_id = '{}'",
            b1.device
        ),
        0,
    )
    .await;
    drop(sock_b);
    let (mut again, _) = Chat::open(&b1).await;
    assert!(
        again.deliveries_within(SILENCE).await.is_empty(),
        "an acked message is not sent again"
    );
    drop(again);

    // A dropped ack is a message sent again: at least once.
    let to = targets(&[&b1]);
    let answer = sock_a.send(2, &text(&a1, 2, &conv, "twice", &to)).await;
    let (second, _) = sent(&answer);
    assert_ne!(second, id);
    let (mut sock_b, _) = Chat::open(&b1).await;
    let first = sock_b.msg().await;
    drop(sock_b); // gone without acking
    let (mut sock_b, _) = Chat::open(&b1).await;
    let repeat = sock_b.msg().await;
    assert_eq!(repeat.seq, first.seq, "the same slot");
    assert_eq!(repeat.sealed, first.sealed);
    assert_eq!(slots(&gw, &b1.device), 1);
    sock_b.up(&Up::Ack { upto: repeat.seq }).await;
    becomes(
        &gw,
        &format!(
            "SELECT count(*) FROM chat_delivery WHERE device_id = '{}'",
            b1.device
        ),
        0,
    )
    .await;
}

#[tokio::test]
async fn a_send_must_wrap_exactly_the_current_devices_of_the_conversation() {
    let gw = spawn_gateway("social-devices");
    let port = gw.port;
    let alice = account(port, "alice-dev", "Alice");
    let bob = account(port, "bob-dev", "Bob");
    befriend(&alice, &bob);
    let a1 = register(&alice);
    let (b1, b2) = (register(&bob), register(&bob));
    let conv = dm(&alice, &bob);
    let (mut sock, _) = Chat::open(&a1).await;

    // One of Bob's devices left out.
    let answer = sock
        .send(1, &text(&a1, 1, &conv, "one", &targets(&[&b1])))
        .await;
    let (why, missing, extra) = refusal(&answer);
    assert_eq!(
        (why, missing, extra),
        (Why::Devices, vec![b2.device.clone()], vec![])
    );

    // A device that is nobody's in the conversation: named as extra.
    let ghost = Target {
        device: "0000000000000001".to_owned(),
        dk: testkit::device(u64::MAX - 7).dk(),
    };
    let mut everyone = targets(&[&b1, &b2]);
    everyone.push(ghost);
    let answer = sock.send(2, &text(&a1, 2, &conv, "two", &everyone)).await;
    let (why, missing, extra) = refusal(&answer);
    assert_eq!(
        (why, missing, extra),
        (Why::Devices, vec![], vec!["0000000000000001".to_owned()])
    );

    // The sending device's own wrap is also not a recipient.
    let mut with_self = targets(&[&b1, &b2]);
    with_self.push(a1.target());
    let answer = sock
        .send(3, &text(&a1, 3, &conv, "three", &with_self))
        .await;
    let (why, _, extra) = refusal(&answer);
    assert_eq!((why, extra), (Why::Devices, vec![a1.device.clone()]));

    assert_eq!(stored(&gw), 0, "a refused send stores nothing");
    assert_eq!(gw.scalar("SELECT count(*) FROM chat_delivery"), 0);

    // Right devices: it goes, to both of Bob's.
    let answer = sock
        .send(4, &text(&a1, 4, &conv, "four", &targets(&[&b1, &b2])))
        .await;
    sent(&answer);
    assert_eq!((slots(&gw, &b1.device), slots(&gw, &b2.device)), (1, 1));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one socket, each way a header can be wrong
async fn a_header_that_does_not_hold_is_refused_and_stores_nothing() {
    let gw = spawn_gateway("social-header");
    let port = gw.port;
    let alice = account(port, "alice-hdr", "Alice");
    let bob = account(port, "bob-hdr", "Bob");
    let carol = account(port, "carol-hdr", "Carol");
    befriend(&alice, &bob);
    let (a1, b1, c1) = (register(&alice), register(&bob), register(&carol));
    let conv = dm(&alice, &bob);
    let to = targets(&[&b1]);
    let (mut sock, _) = Chat::open(&a1).await;
    let words = text_body("hi");

    // Each of these is the same good message with one thing wrong.
    let now = now_ms();
    let minutes = |m: u64| m * 60 * 1000;
    let mut cases: Vec<(&str, Header, Why)> = Vec::new();
    let mut old = header(&a1, &conv);
    old.ts = now - minutes(11);
    cases.push(("eleven minutes old", old, Why::Stale));
    let mut future = header(&a1, &conv);
    future.ts = now + minutes(11);
    cases.push(("eleven minutes ahead", future, Why::Stale));
    let mut other_sender = header(&a1, &conv);
    other_sender.from = bob.tag.clone();
    cases.push(("another sender", other_sender, Why::BadSealed));
    let mut other_device = header(&a1, &conv);
    other_device.dev = b1.device.clone();
    cases.push(("another device", other_device, Why::BadSealed));
    let mut other_conv = header(&a1, &conv);
    other_conv.conv = dm(&alice, &carol);
    cases.push((
        "a header of another conversation",
        other_conv,
        Why::BadSealed,
    ));
    let mut version = header(&a1, &conv);
    version.v = 2;
    cases.push(("another header version", version, Why::BadSealed));
    let mut kind = header(&a1, &conv);
    kind.kind = "image".to_owned();
    cases.push(("another kind", kind, Why::BadSealed));
    let mut not_v7 = header(&a1, &conv);
    not_v7.id = uuid::Uuid::from_u128(0x0123_4567_89ab_4def_8123_4567_89ab_cdef).to_string();
    cases.push(("an id that is not a v7", not_v7, Why::BadSealed));
    let mut shouted = header(&a1, &conv);
    shouted.id = shouted.id.to_uppercase();
    cases.push(("an id in capitals", shouted, Why::BadSealed));
    for (req, (what, header, why)) in (1_u32..).zip(cases) {
        let answer = sock
            .send(req, &seal_up(&a1, req, &conv, &header, &words, &to))
            .await;
        assert_eq!(refusal(&answer).0, why, "{what}");
    }

    // The frame's conversation against the header's.
    let answer = sock
        .send(
            30,
            &seal_up(
                &a1,
                30,
                &dm(&alice, &carol),
                &header(&a1, &conv),
                &words,
                &to,
            ),
        )
        .await;
    assert_eq!(
        refusal(&answer).0,
        Why::BadSealed,
        "a frame of another conversation"
    );
    for (req, conv_text) in [(31, "dm:zz:zz"), (32, "chat:everyone"), (33, "")] {
        let answer = sock
            .send(
                req,
                &seal_up(&a1, req, conv_text, &header(&a1, conv_text), &words, &to),
            )
            .await;
        assert_eq!(
            refusal(&answer).0,
            Why::BadSealed,
            "{conv_text:?} is not a conversation"
        );
    }

    // Malformed sealing: a short nonce, a signature of the wrong size.
    let Up::Send { mut sealed, .. } = text(&a1, 40, &conv, "hi", &to) else {
        unreachable!()
    };
    sealed.n = b64::encode(&[0_u8; 23]);
    let answer = sock
        .send(
            40,
            &Up::Send {
                req: 40,
                conv: conv.clone(),
                sealed,
            },
        )
        .await;
    assert_eq!(refusal(&answer).0, Why::BadSealed, "a nonce of 23 bytes");
    let Up::Send { mut sealed, .. } = text(&a1, 41, &conv, "hi", &to) else {
        unreachable!()
    };
    sealed.sig = "not base64!".to_owned();
    let answer = sock
        .send(
            41,
            &Up::Send {
                req: 41,
                conv: conv.clone(),
                sealed,
            },
        )
        .await;
    assert_eq!(
        refusal(&answer).0,
        Why::BadSealed,
        "a signature that is not base64"
    );

    // Not a member: Carol is nobody's friend and sits at no table; there is
    // no such group or table either. The answer is the same for all three.
    let carols = dm(&alice, &carol);
    let answer = sock
        .send(50, &text(&a1, 50, &carols, "hi", &targets(&[&c1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember, "a stranger");
    let group = format!("grp:{}", uuid::Uuid::now_v7());
    let answer = sock.send(51, &text(&a1, 51, &group, "hi", &to)).await;
    assert_eq!(
        refusal(&answer).0,
        Why::NotMember,
        "a group that does not exist"
    );
    let answer = sock
        .send(52, &text(&a1, 52, "tbl:nowhere", "hi", &to))
        .await;
    assert_eq!(
        refusal(&answer).0,
        Why::NotMember,
        "a table that does not exist"
    );
    let answer = sock
        .send(53, &text(&a1, 53, "team:nowhere:1", "hi", &to))
        .await;
    assert_eq!(
        refusal(&answer).0,
        Why::NotMember,
        "a team that does not exist"
    );

    assert_eq!(stored(&gw), 0, "none of them was stored");

    // A message is sent once: the second with the same id is a duplicate,
    // and it is not stored twice.
    let once = header(&a1, &conv);
    let answer = sock
        .send(60, &seal_up(&a1, 60, &conv, &once, &words, &to))
        .await;
    sent(&answer);
    let answer = sock
        .send(61, &seal_up(&a1, 61, &conv, &once, &words, &to))
        .await;
    assert_eq!(refusal(&answer).0, Why::Duplicate);
    assert_eq!(stored(&gw), 1);
    assert_eq!(slots(&gw, &b1.device), 1);
}

#[tokio::test]
async fn a_recipient_with_a_full_shelf_is_refused_one_slot_short_of_it_is_not() {
    let gw = spawn_gateway("social-full");
    let port = gw.port;
    let alice = account(port, "alice-full", "Alice");
    let bob = account(port, "bob-full", "Bob");
    befriend(&alice, &bob);
    let (a1, b1) = (register(&alice), register(&bob));
    let conv = dm(&alice, &bob);
    let to = targets(&[&b1]);
    let (mut sock, _) = Chat::open(&a1).await;

    // One real message, whose id the rest of the shelf can hang on.
    let answer = sock.send(1, &text(&a1, 1, &conv, "first", &to)).await;
    let (id, _) = sent(&answer);
    // Up to one short of the cap (5000): 1 + 4998 = 4999 slots.
    gw.sql(&format!(
        "INSERT INTO chat_delivery (device_id, message_id, wrap, expires_at) \
         SELECT '{}', '{id}'::uuid, decode(repeat('00', 48), 'hex'), now() + interval '1 day' \
         FROM generate_series(1, {})",
        b1.device,
        wire::SLOTS_PER_DEVICE - 2
    ));
    assert_eq!(slots(&gw, &b1.device), 4999);

    let answer = sock
        .send(2, &text(&a1, 2, &conv, "the last that fits", &to))
        .await;
    sent(&answer);
    assert_eq!(slots(&gw, &b1.device), 5000);
    let answer = sock
        .send(3, &text(&a1, 3, &conv, "one too many", &to))
        .await;
    assert_eq!(refusal(&answer).0, Why::RecipientFull);
    assert_eq!(slots(&gw, &b1.device), 5000, "nothing was added");
    assert_eq!(stored(&gw), 2, "and the refused message was not kept");
}

#[tokio::test]
async fn a_blocked_sender_still_gets_sent_and_the_blocker_is_given_nothing() {
    let gw = spawn_gateway("social-blocked");
    let port = gw.port;
    let alice = account(port, "alice-blk", "Alice");
    let bob = account(port, "bob-blk", "Bob");
    let carol = account(port, "carol-blk", "Carol");
    befriend(&alice, &bob);
    befriend(&alice, &carol);
    let (a1, b1, c1) = (register(&alice), register(&bob), register(&carol));

    // A group of the three, and then Carol blocks Alice.
    let (status, body) = http(
        port,
        "POST",
        "/social/groups",
        Some(&alice.session),
        r#"{"name":"crew"}"#,
    );
    assert_eq!(status, 201, "{body}");
    let group = json_field(&body, "id").to_owned();
    for friend in [&bob, &carol] {
        let invite = serde_json::json!({ "tag": friend.tag }).to_string();
        let path = format!("/social/groups/{group}/invites");
        let (status, body) = http(port, "POST", &path, Some(&alice.session), &invite);
        assert!(status < 300, "invite: {status} {body}");
        let path = format!("/social/groups/{group}/join");
        let (status, body) = http(port, "POST", &path, Some(&friend.session), "");
        assert!(status < 300, "join: {status} {body}");
    }
    let block = serde_json::json!({ "tag": alice.tag }).to_string();
    let (status, body) = http(port, "POST", "/social/blocks", Some(&carol.session), &block);
    assert_eq!(status, 204, "{body}");

    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_b, _) = Chat::open(&b1).await;
    let (mut sock_c, _) = Chat::open(&c1).await;
    let conv = format!("grp:{group}");
    // Carol's device is still a recipient the wraps must name: whether
    // someone blocks the sender is not the sender's to learn.
    let answer = sock_a
        .send(1, &text(&a1, 1, &conv, "hello crew", &targets(&[&b1, &c1])))
        .await;
    sent(&answer);

    let got = sock_b.msg().await;
    assert_eq!(
        &open(&b1, &a1, &got.sealed).body[..],
        &text_body("hello crew")[..]
    );
    assert!(sock_c.deliveries_within(SILENCE).await.is_empty());
    assert_eq!(slots(&gw, &c1.device), 0, "no slot for the blocker");
    assert_eq!(slots(&gw, &b1.device), 1);

    // Without Carol's device in the wraps it is refused all the same.
    let answer = sock_a
        .send(2, &text(&a1, 2, &conv, "again", &targets(&[&b1])))
        .await;
    let (why, missing, _) = refusal(&answer);
    assert_eq!((why, missing), (Why::Devices, vec![c1.device.clone()]));
}

#[tokio::test]
async fn the_ninth_quick_send_is_over_the_allowance() {
    let gw = spawn_gateway("social-rate");
    let port = gw.port;
    let alice = account(port, "alice-rate", "Alice");
    let bob = account(port, "bob-rate", "Bob");
    befriend(&alice, &bob);
    let (a1, b1) = (register(&alice), register(&bob));
    let conv = dm(&alice, &bob);
    let to = targets(&[&b1]);
    let (mut sock, _) = Chat::open(&a1).await;

    // Twelve at once, all good: eight are the burst, a second more buys one.
    for req in 1..=12 {
        sock.up(&text(&a1, req, &conv, "burst", &to)).await;
    }
    let (mut stored_n, mut limited) = (0, 0);
    for req in 1..=12 {
        match sock.answer(req).await {
            Down::Sent { .. } => stored_n += 1,
            other => {
                assert_eq!(refusal(&other).0, Why::Rate, "send {req}");
                limited += 1;
            }
        }
    }
    assert!(stored_n >= 8, "the burst is eight: {stored_n} went");
    assert!(limited >= 1, "twelve at once is over it");
    assert_eq!(stored_n + limited, 12);
    assert_eq!(slots(&gw, &b1.device), stored_n);

    // The allowance is the account's, not the socket's: a second device of
    // Alice's, on a new socket, is no fresh burst.
    let a2 = register(&alice);
    let (mut sock2, _) = Chat::open(&a2).await;
    let both = targets(&[&a1, &b1]);
    let mut again = 0;
    for req in 1..=8 {
        sock2
            .up(&text(&a2, req, &conv, "other device", &both))
            .await;
    }
    for req in 1..=8 {
        if let Down::Refused { why: Why::Rate, .. } = sock2.answer(req).await {
            again += 1;
        }
    }
    assert!(again >= 1, "the account was still spent");
}

#[tokio::test]
async fn a_socket_over_its_bounds_is_closed_with_the_code_that_says_why() {
    let gw = spawn_gateway("social-bounds");
    let port = gw.port;
    let alice = account(port, "alice-bnd", "Alice");
    let a1 = register(&alice);

    // A frame between the limit (48 KiB) and the transport's: read, refused.
    let (mut sock, _) = Chat::open(&a1).await;
    sock.raw(Message::Text("x".repeat(50 * 1024).into())).await;
    assert_eq!(sock.closed().await, Some(1008), "50 KiB of text");

    // The 41st frame of a burst of 40.
    let (mut sock, _) = Chat::open(&a1).await;
    for _ in 0..60 {
        sock.up(&Up::Ack { upto: 0 }).await;
    }
    assert_eq!(sock.closed().await, Some(1008), "60 frames at once");

    // The edge of the burst is fine: forty frames are.
    let (mut sock, _) = Chat::open(&a1).await;
    for _ in 0..40 {
        sock.up(&Up::Ack { upto: 0 }).await;
    }
    let (frames, closed) = sock.quiet(SILENCE).await;
    assert!(
        frames.is_empty() && closed.is_none(),
        "{frames:?} {closed:?}"
    );

    // Text only.
    sock.raw(Message::Binary(vec![1, 2, 3].into())).await;
    assert_eq!(sock.closed().await, Some(1003), "a binary frame");

    // Not a frame at all.
    let (mut sock, _) = Chat::open(&a1).await;
    sock.raw(Message::Text(r#"{"t":"shout"}"#.into())).await;
    assert_eq!(sock.closed().await, Some(1008), "a frame of no kind");
}

#[tokio::test]
async fn a_socket_states_its_protocol_and_says_hello_first() {
    let gw = spawn_gateway("social-protocol");
    let port = gw.port;
    let alice = account(port, "alice-pro", "Alice");
    let a1 = register(&alice);

    for (what, edit) in [
        (
            "another social version",
            Box::new(|path: String| path.replace("&social=1", "&social=2"))
                as Box<dyn FnOnce(String) -> String>,
        ),
        (
            "no social version",
            Box::new(|path: String| path.replace("&social=1", "")),
        ),
        (
            "no protocol",
            Box::new(|path: String| {
                path.replace(
                    &format!("&protocol={}", baylee_protocol::PROTOCOL_VERSION),
                    "&protocol=0",
                )
            }),
        ),
        (
            "another protocol",
            Box::new(|path: String| {
                path.replace(
                    &format!("&protocol={}", baylee_protocol::PROTOCOL_VERSION),
                    "&protocol=9999",
                )
            }),
        ),
    ] {
        let mut sock = Chat::dial(&alice, edit).await;
        let refused = sock.next_frame().await;
        let Down::Refused {
            why, message, req, ..
        } = refused
        else {
            panic!("{what}: {refused:?}");
        };
        assert_eq!((why, req), (Why::Protocol, None), "{what}");
        assert!(message.is_some_and(|m| !m.is_empty()), "{what}: a sentence");
        sock.closed().await;
    }

    // Anything but a hello first closes it.
    let mut sock = Chat::dial(&alice, |path| path).await;
    sock.up(&Up::Ack { upto: 0 }).await;
    assert_eq!(sock.closed().await, Some(1008), "ack before hello");
    let mut sock = Chat::dial(&alice, |path| path).await;
    sock.raw(Message::Binary(vec![0].into())).await;
    assert_eq!(sock.closed().await, Some(1008), "binary before hello");

    // A hello for a device that is not this account's is refused.
    let bob = account(port, "bob-pro", "Bob");
    let b1 = register(&bob);
    let mut sock = Chat::dial(&alice, |path| path).await;
    sock.up(&hello(&b1.device, Status::Online)).await;
    let refused = sock.next_frame().await;
    assert!(
        matches!(
            refused,
            Down::Refused {
                why: Why::Device,
                ..
            }
        ),
        "{refused:?}"
    );
    sock.closed().await;
    let mut sock = Chat::dial(&alice, |path| path).await;
    sock.up(&hello("0000000000000000", Status::Online)).await;
    let refused = sock.next_frame().await;
    assert!(
        matches!(
            refused,
            Down::Refused {
                why: Why::Device,
                ..
            }
        ),
        "{refused:?}"
    );

    // The good one still opens.
    let (_sock, snapshot) = Chat::open(&a1).await;
    assert!(snapshot.friends.is_empty());
}
