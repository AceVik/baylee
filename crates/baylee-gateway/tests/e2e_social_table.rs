//! Table and team conversations end to end (`docs/protocol.md` §"Friends
//! and chat (beta.8)", §5.1): who is a member of `tbl:<game>` and
//! `team:<game>:<n>` is read from the lobby, never declared by a client, so
//! it is the accounts in a chair and nobody else: not the house AI, not a
//! seat bridge a host handed a chair to, not a spectator. The gateway pushes
//! `Members` whenever that changes, and a whisper between table mates is
//! allowed by the recipient's setting alone.

#![allow(clippy::missing_docs_in_private_items, dead_code)]

mod common;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use baylee_protocol::social::{
    self as wire, Conv, Delivered, Down, Header, PresenceSet, Sender, Snapshot, Status, Up, Why,
    Wrap,
};
use baylee_protocol::v1;
use baylee_seal::b64::url as b64;
use baylee_seal::testkit::{self, Seeded};
use baylee_seal::{DeviceSecrets, Envelope, Recipient};
use common::{
    Gateway, Socket, WAIT_BUDGET, ask_ticket, attach_agent, attach_agent_probed, http, json_field,
    login, spawn_gateway,
};
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

/// How long a socket is watched for a frame that must not come.
const SILENCE: Duration = Duration::from_millis(900);

// ---------------------------------------------------------------------------
// The lobby, as far as these tests need it.
// ---------------------------------------------------------------------------

fn deck(acct: &Acct) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(acct.port, "POST", "/decks", Some(&acct.session), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_owned()
}

/// A waiting room of `seats` chairs `host` opens and sits in: its id and
/// the host's seat token.
fn open_room(host: &Acct, seats: u32) -> (String, String) {
    let create = format!(
        "{{\"deck_id\":\"{}\",\"seats\":{seats},\"name\":\"Social\"}}",
        deck(host)
    );
    let (status, body) = http(
        host.port,
        "POST",
        "/lobby/games",
        Some(&host.session),
        &create,
    );
    assert_eq!(status, 200, "open a room: {body}");
    (
        json_field(&body, "game_id").to_owned(),
        json_field(&body, "seat_token").to_owned(),
    )
}

/// `guest` takes a free chair of `game`; its seat token.
fn sit(guest: &Acct, game: &str) -> String {
    let join = format!("{{\"deck_id\":\"{}\"}}", deck(guest));
    let path = format!("/lobby/games/{game}/join");
    let (status, body) = http(guest.port, "POST", &path, Some(&guest.session), &join);
    assert_eq!(status, 200, "join: {body}");
    json_field(&body, "seat_token").to_owned()
}

fn leave(guest: &Acct, game: &str) {
    let path = format!("/lobby/games/{game}/leave");
    let (status, body) = http(guest.port, "POST", &path, Some(&guest.session), "");
    assert_eq!(status, 204, "leave: {body}");
}

/// The host arranges chair `seat` of `game` with `body`.
fn arrange(host: &Acct, game: &str, seat: u32, body: &str) {
    let path = format!("/lobby/games/{game}/seats/{seat}");
    let (status, answer) = http(host.port, "POST", &path, Some(&host.session), body);
    assert_eq!(status, 200, "arrange chair {seat}: {answer}");
}

/// Waits until `chat` is told the members of `conv` are exactly `want`.
async fn members_become(chat: &mut Chat, conv: &str, want: &[&Acct]) {
    let want: BTreeSet<String> = want.iter().map(|acct| acct.tag.clone()).collect();
    chat.until(|frame| match frame {
        Down::Members { conv: c, members }
            if c == conv
                && members
                    .iter()
                    .map(|p| p.tag.clone())
                    .collect::<BTreeSet<_>>()
                    == want =>
        {
            Some(())
        }
        _ => None,
    })
    .await;
}

/// The conversations whose members `frames` carry.
fn member_frames(frames: &[Down]) -> Vec<String> {
    frames
        .iter()
        .filter_map(|frame| match frame {
            Down::Members { conv, .. } => Some(conv.clone()),
            _ => None,
        })
        .collect()
}

fn deliveries(frames: &[Down]) -> usize {
    frames
        .iter()
        .filter(|frame| matches!(frame, Down::Msg { .. }))
        .count()
}

#[tokio::test]
async fn a_tables_members_are_the_people_in_its_chairs_and_are_pushed_as_they_change() {
    let gw = spawn_gateway("social-members");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let (alice, bob, carol) = (
        account(port, "alice-mem", "Alice"),
        account(port, "bob-mem", "Bob"),
        account(port, "carol-mem", "Carol"),
    );
    let (a1, a2, b1, c1) = (
        register(&alice),
        register(&alice),
        register(&bob),
        register(&carol),
    );
    // Everyone is on the gateway before the table exists, so every change
    // reaches them as a `Members` frame rather than in a snapshot.
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_b, _) = Chat::open(&b1).await;
    let (mut sock_c, _) = Chat::open(&c1).await;

    // Six chairs: the host's, then a seat bridge's chair, the house's, and
    // three for people.
    let (game, _) = open_room(&alice, 6);
    let conv = format!("tbl:{game}");
    members_become(&mut sock_a, &conv, &[&alice]).await;

    arrange(&alice, &game, 3, r#"{"kind":"ai","ai":"sharp"}"#);
    let (status, body) = http(
        port,
        "POST",
        &baylee_protocol::chair_ticket_path(&game, 2),
        Some(&alice.session),
        "{}",
    );
    assert_eq!(status, 200, "a chair ticket: {body}");
    let redeem = r#"{"display_name":"LLM-test","deck":{"name":"swamps","cards":["60 Swamp"]}}"#;
    let (status, body) = http(
        port,
        "POST",
        &baylee_protocol::chair_redeem_path(&game, 2),
        Some(json_field(&body, "ticket")),
        redeem,
    );
    assert_eq!(status, 200, "the bridge sits: {body}");

    // Bob sits down: both are told, and the AI's chair and the bridge's
    // are in no list.
    sit(&bob, &game);
    members_become(&mut sock_a, &conv, &[&alice, &bob]).await;
    members_become(&mut sock_b, &conv, &[&alice, &bob]).await;
    // The third person: all three are told, Carol too.
    sit(&carol, &game);
    for sock in [&mut sock_a, &mut sock_b, &mut sock_c] {
        members_become(sock, &conv, &[&alice, &bob, &carol]).await;
    }

    // A late device sees the table in its snapshot, with the same people.
    let (_sock_a2, snapshot) = Chat::open(&a2).await;
    let table = snapshot
        .tables
        .iter()
        .find(|table| table.conv == conv)
        .expect("the table is in the snapshot");
    let tags: BTreeSet<&str> = table.members.iter().map(|p| p.tag.as_str()).collect();
    assert_eq!(
        tags,
        BTreeSet::from([alice.tag.as_str(), bob.tag.as_str(), carol.tag.as_str()])
    );
    assert_eq!(
        table.members.len(),
        3,
        "the house and the bridge are not people at it"
    );

    // Carol gets up: the others are told, and so is she.
    leave(&carol, &game);
    members_become(&mut sock_a, &conv, &[&alice, &bob]).await;
    members_become(&mut sock_b, &conv, &[&alice, &bob]).await;
    members_become(&mut sock_c, &conv, &[&alice, &bob]).await;
    // And she has lost the right to write to it.
    let answer = sock_c
        .send(
            1,
            &text(&c1, 1, &conv, "still here?", &targets(&[&a1, &a2, &b1])),
        )
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember);
}

#[tokio::test]
async fn table_chat_goes_to_the_people_at_the_table_and_to_nobody_else() {
    let gw = spawn_gateway("social-tablechat");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let (alice, bob, carol) = (
        account(port, "alice-tc", "Alice"),
        account(port, "bob-tc", "Bob"),
        account(port, "carol-tc", "Carol"),
    );
    let (a1, b1, c1) = (register(&alice), register(&bob), register(&carol));
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_b, _) = Chat::open(&b1).await;
    let (mut sock_c, _) = Chat::open(&c1).await;
    let (game, _) = open_room(&alice, 2);
    sit(&bob, &game);
    let conv = format!("tbl:{game}");
    members_become(&mut sock_b, &conv, &[&alice, &bob]).await;

    // Alice to the table: Bob gets it, Alice's own device does not, and
    // Carol, who is on the gateway and not at the table, hears nothing.
    let answer = sock_a
        .send(1, &text(&a1, 1, &conv, "good luck", &targets(&[&b1])))
        .await;
    sent(&answer);
    let got = sock_b.msg().await;
    assert_eq!(
        (got.conv.as_str(), got.from.tag.as_str()),
        (conv.as_str(), alice.tag.as_str())
    );
    assert_eq!(
        &open(&b1, &a1, &got.sealed).body[..],
        &text_body("good luck")[..]
    );
    assert!(sock_a.deliveries_within(SILENCE).await.is_empty());
    assert!(sock_c.deliveries_within(SILENCE).await.is_empty());

    // Bob answers the table.
    let answer = sock_b
        .send(1, &text(&b1, 1, &conv, "you too", &targets(&[&a1])))
        .await;
    sent(&answer);
    let back = sock_a.msg().await;
    assert_eq!(back.from.tag, bob.tag);

    // Carol is not at it: she may not write, and may not be written to
    // either: her device in Alice's wraps is a device that is no recipient.
    let answer = sock_c
        .send(1, &text(&c1, 1, &conv, "let me in", &targets(&[&a1, &b1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember);
    let answer = sock_a
        .send(2, &text(&a1, 2, &conv, "psst", &targets(&[&b1, &c1])))
        .await;
    let (why, missing, extra) = refusal(&answer);
    assert_eq!(
        (why, missing, extra),
        (Why::Devices, vec![], vec![c1.device.clone()])
    );

    // The key directory agrees with the table: Alice may fetch Bob's keys
    // (a table mate) and not Carol's (a stranger).
    let (status, body) = http(
        port,
        "GET",
        &format!("/social/keys?tag={}&tag={}", bob.tag, carol.tag),
        Some(&alice.session),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let keys: wire::KeysAnswer = serde_json::from_str(&body).unwrap();
    let tags: Vec<&str> = keys.keys.iter().map(|k| k.tag.as_str()).collect();
    assert_eq!(tags, [bob.tag.as_str()]);
}

#[tokio::test]
async fn table_mates_may_whisper_unless_the_recipient_takes_friends_only() {
    let gw = spawn_gateway("social-whisper");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let (alice, bob) = (
        account(port, "alice-wh", "Alice"),
        account(port, "bob-wh", "Bob"),
    );
    let (a1, b1) = (register(&alice), register(&bob));
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_b, _) = Chat::open(&b1).await;
    let whisper = dm(&alice, &bob);

    // Strangers, not at a table: no.
    let answer = sock_a
        .send(1, &text(&a1, 1, &whisper, "hi", &targets(&[&b1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember, "strangers");

    // At one table, they are not strangers: the default takes tables.
    let (game, _) = open_room(&alice, 2);
    sit(&bob, &game);
    members_become(&mut sock_b, &format!("tbl:{game}"), &[&alice, &bob]).await;
    let answer = sock_a
        .send(2, &text(&a1, 2, &whisper, "nice deck", &targets(&[&b1])))
        .await;
    sent(&answer);
    let got = sock_b.msg().await;
    assert_eq!(got.conv, whisper);
    assert_eq!(
        &open(&b1, &a1, &got.sealed).body[..],
        &text_body("nice deck")[..]
    );

    // Bob takes whispers from friends only: the same whisper is refused,
    // and in the same words as any other refusal to write there.
    let friends_only = r#"{"whispers_from":"friends","requests_open":true,"share_table":true}"#;
    let (status, body) = http(
        port,
        "PUT",
        "/social/settings",
        Some(&bob.session),
        friends_only,
    );
    assert_eq!(status, 204, "{body}");
    let answer = sock_a
        .send(3, &text(&a1, 3, &whisper, "and now?", &targets(&[&b1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember, "friends only");
    // It is the recipient's setting: Bob may still whisper Alice, who
    // takes tables.
    let answer = sock_b
        .send(1, &text(&b1, 1, &whisper, "fair", &targets(&[&a1])))
        .await;
    sent(&answer);
    assert_eq!(sock_a.msg().await.from.tag, bob.tag);

    // Back to the default, and then Bob leaves: no table, no whisper.
    let both = r#"{"whispers_from":"friends_and_tables","requests_open":true,"share_table":true}"#;
    let (status, body) = http(port, "PUT", "/social/settings", Some(&bob.session), both);
    assert_eq!(status, 204, "{body}");
    let answer = sock_a
        .send(4, &text(&a1, 4, &whisper, "again", &targets(&[&b1])))
        .await;
    sent(&answer);
    leave(&bob, &game);
    let answer = sock_a
        .send(5, &text(&a1, 5, &whisper, "gone?", &targets(&[&b1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember, "no longer at a table");
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one table, arranged into sides, and what each side may do
async fn a_team_conversation_exists_only_at_a_team_table_and_only_for_its_team() {
    let gw = spawn_gateway("social-teams");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let (alice, bob, carol) = (
        account(port, "alice-tm", "Alice"),
        account(port, "bob-tm", "Bob"),
        account(port, "carol-tm", "Carol"),
    );
    let (a1, b1, c1) = (register(&alice), register(&bob), register(&carol));
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_b, _) = Chat::open(&b1).await;
    let (mut sock_c, _) = Chat::open(&c1).await;
    let (game, _) = open_room(&alice, 4);
    sit(&bob, &game);
    sit(&carol, &game);
    arrange(&alice, &game, 3, r#"{"kind":"ai","ai":"sharp"}"#);
    let table = format!("tbl:{game}");
    let (team1, team2) = (format!("team:{game}:1"), format!("team:{game}:2"));
    for sock in [&mut sock_a, &mut sock_b, &mut sock_c] {
        members_become(sock, &table, &[&alice, &bob, &carol]).await;
    }

    // No sides yet, so no team to write to.
    let answer = sock_a
        .send(1, &text(&a1, 1, &team1, "us", &targets(&[&b1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember, "a table without sides");

    // Alice and Bob against Carol and the house.
    for (seat, team) in [(0, 1), (1, 1), (2, 2), (3, 2)] {
        arrange(&alice, &game, seat, &format!("{{\"team\":{team}}}"));
    }
    members_become(&mut sock_a, &team1, &[&alice, &bob]).await;
    members_become(&mut sock_b, &team1, &[&alice, &bob]).await;
    members_become(&mut sock_c, &team2, &[&carol]).await;
    // Nobody is told of a team they are not on.
    let (frames, _) = sock_c.quiet(SILENCE).await;
    assert!(!member_frames(&frames).contains(&team1), "{frames:?}");

    // Alice to her team: Bob hears it, Carol does not.
    let answer = sock_a
        .send(2, &text(&a1, 2, &team1, "plan: attack", &targets(&[&b1])))
        .await;
    sent(&answer);
    let got = sock_b.msg().await;
    assert_eq!(got.conv, team1);
    assert_eq!(
        &open(&b1, &a1, &got.sealed).body[..],
        &text_body("plan: attack")[..]
    );
    assert!(sock_c.deliveries_within(SILENCE).await.is_empty());

    // The other side's conversation is shut to her, as are teams that are
    // not there.
    let answer = sock_a
        .send(3, &text(&a1, 3, &team2, "spying", &targets(&[&c1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember, "the other team");
    let answer = sock_c
        .send(1, &text(&c1, 1, &team1, "spying", &targets(&[&a1, &b1])))
        .await;
    assert_eq!(
        refusal(&answer).0,
        Why::NotMember,
        "the other team, the other way"
    );
    let answer = sock_a
        .send(
            4,
            &text(&a1, 4, &format!("team:{game}:3"), "hi", &targets(&[&b1])),
        )
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember, "a team nobody is on");
    // And a team's message wrapped for the whole table names devices that
    // are not the team's.
    let answer = sock_a
        .send(5, &text(&a1, 5, &team1, "oops", &targets(&[&b1, &c1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::Devices);
    // The table as a whole is still one conversation for all three.
    let answer = sock_c
        .send(2, &text(&c1, 2, &table, "gg", &targets(&[&a1, &b1])))
        .await;
    sent(&answer);
}

/// Reads a seat until it falls quiet; how many frames it was sent.
async fn seat_frames_for(ws: &mut Socket, window: Duration) -> usize {
    let mut seen = 0;
    while let Ok(Some(_)) = tokio::time::timeout(window, common::next_msg(ws)).await {
        seen += 1;
    }
    seen
}

/// Reads a seat until the table opens.
async fn curtain(ws: &mut Socket) {
    tokio::time::timeout(WAIT_BUDGET, async {
        loop {
            match common::next_msg(ws).await {
                Some(v1::envelope::Msg::Curtain(_)) => break,
                Some(_) => {}
                None => panic!("the seat closed before the table opened"),
            }
        }
    })
    .await
    .expect("the table opened");
}

/// A watch socket on a running table, for a spectator.
async fn watch(acct: &Acct, game: &str) -> Socket {
    for _ in 0..100 {
        let body = format!("{{\"socket\":\"watch\",\"game\":\"{game}\"}}");
        let (status, body) = ask_ticket(acct.port, &acct.session, &body);
        if status == 200 {
            let url = format!(
                "ws://127.0.0.1:{}{}",
                acct.port,
                baylee_protocol::watch_socket_path(game, json_field(&body, "ticket"))
            );
            if let Some(ws) = common::dial(&url).await {
                return ws;
            }
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the watch socket never opened");
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one running table: chat on it, a spectator at it, the engine beside it
async fn a_running_tables_chat_never_reaches_a_spectator_or_the_engine() {
    let gw = spawn_gateway("social-running");
    let port = gw.port;
    let (_agent, probe) = attach_agent_probed(&gw).await;
    let (alice, bob, wendy) = (
        account(port, "alice-run", "Alice"),
        account(port, "bob-run", "Bob"),
        account(port, "wendy-run", "Wendy"),
    );
    let (a1, b1, w1) = (register(&alice), register(&bob), register(&wendy));
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_b, _) = Chat::open(&b1).await;
    let (mut sock_w, _) = Chat::open(&w1).await;

    let (game, seat_a) = open_room(&alice, 2);
    let seat_b = sit(&bob, &game);
    let conv = format!("tbl:{game}");
    for acct in [&alice, &bob] {
        let path = format!("/lobby/games/{game}/ready");
        assert_eq!(http(port, "POST", &path, Some(&acct.session), "{}").0, 200);
    }
    let path = format!("/lobby/games/{game}/start");
    assert_eq!(http(port, "POST", &path, Some(&alice.session), "").0, 200);
    let mut seat_a = common::dial_seat(port, &game, &seat_a).await;
    let mut seat_b = common::dial_seat(port, &game, &seat_b).await;
    curtain(&mut seat_a).await;
    curtain(&mut seat_b).await;
    // Wendy watches the running table.
    let mut watching = watch(&wendy, &game).await;

    members_become(&mut sock_a, &conv, &[&alice, &bob]).await;
    // Let the opening of the table settle on the seats before chat starts.
    seat_frames_for(&mut seat_a, Duration::from_millis(1500)).await;
    seat_frames_for(&mut seat_b, Duration::from_millis(1500)).await;
    let before = *probe.hash.borrow();
    assert!(before.is_some(), "the engine built the game");

    // The players talk; the spectator is told nothing of it.
    let answer = sock_a
        .send(1, &text(&a1, 1, &conv, "your turn", &targets(&[&b1])))
        .await;
    sent(&answer);
    assert_eq!(sock_b.msg().await.from.tag, alice.tag);
    let answer = sock_b
        .send(1, &text(&b1, 1, &conv, "thanks", &targets(&[&a1])))
        .await;
    sent(&answer);
    assert_eq!(sock_a.msg().await.from.tag, bob.tag);
    let (frames, closed) = sock_w.quiet(SILENCE).await;
    assert_eq!(closed, None);
    assert_eq!(deliveries(&frames), 0, "{frames:?}");
    assert!(
        member_frames(&frames).is_empty(),
        "a spectator is told of no table: {frames:?}"
    );

    // She is no member: not to write, not to be wrapped for, not to whisper.
    let answer = sock_w
        .send(1, &text(&w1, 1, &conv, "hello?", &targets(&[&a1, &b1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember);
    let answer = sock_a
        .send(2, &text(&a1, 2, &conv, "psst", &targets(&[&b1, &w1])))
        .await;
    assert_eq!(refusal(&answer).0, Why::Devices);
    let answer = sock_w
        .send(
            2,
            &text(&w1, 2, &dm(&wendy, &alice), "hi", &targets(&[&a1])),
        )
        .await;
    assert_eq!(
        refusal(&answer).0,
        Why::NotMember,
        "watching is not sitting"
    );

    // The engine heard nothing: its game is as it was, and the seats were
    // sent no frame for any of it.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(*probe.hash.borrow(), before, "chat moved the game");
    assert_eq!(
        seat_frames_for(&mut seat_a, Duration::from_millis(1500)).await,
        0
    );
    assert_eq!(
        seat_frames_for(&mut seat_b, Duration::from_millis(1500)).await,
        0
    );
    // The spectator's table is still hers.
    watching
        .send(Message::Ping(Vec::new().into()))
        .await
        .expect("the watch socket is open");
}
