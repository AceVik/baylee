//! Presence end to end (`docs/protocol.md` §"Friends and chat (beta.8)",
//! §6): what a friend is shown of someone, and only a friend. Offline is
//! what no socket and an invisible one both read as; where someone is comes
//! from the lobby the gateway holds, and a table's id is there only when it
//! is shared and the room is not locked.

#![allow(clippy::missing_docs_in_private_items, dead_code)]

mod common;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use baylee_protocol::social::{
    self as wire, Conv, Delivered, Down, Header, Place, Presence, PresenceSet, Sender, Shown,
    Snapshot, Status, Up, Why, Wrap,
};
use baylee_seal::b64::url as b64;
use baylee_seal::testkit::{self, Seeded};
use baylee_seal::{DeviceSecrets, Envelope, Recipient};
use common::{
    Gateway, Socket, WAIT_BUDGET, ask_ticket, attach_agent, http, json_field, json_number, login,
    spawn_gateway,
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

/// How long a socket is watched for a frame that must not come: longer than
/// the second a push may be held back.
const SILENCE: Duration = Duration::from_millis(2500);

fn presence_frames(frames: &[Down]) -> Vec<(String, Presence)> {
    frames
        .iter()
        .filter_map(|frame| match frame {
            Down::Presence { tag, presence } => Some((tag.clone(), presence.clone())),
            _ => None,
        })
        .collect()
}

/// Reads `chat` until it is told `tag` is shown as `want` says.
async fn shown(chat: &mut Chat, tag: &str, want: impl Fn(&Presence) -> bool) -> Presence {
    chat.until(|frame| match frame {
        Down::Presence { tag: t, presence } if t == tag && want(presence) => Some(presence.clone()),
        _ => None,
    })
    .await
}

fn deck(acct: &Acct) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(acct.port, "POST", "/decks", Some(&acct.session), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_owned()
}

/// `acct` opens a room with `extra` JSON fields; its id.
fn open_room(acct: &Acct, extra: &str) -> String {
    let create = format!(
        "{{\"deck_id\":\"{}\",\"seats\":3,\"name\":\"Kitchen\"{extra}}}",
        deck(acct)
    );
    let (status, body) = http(
        acct.port,
        "POST",
        "/lobby/games",
        Some(&acct.session),
        &create,
    );
    assert_eq!(status, 200, "open a room: {body}");
    json_field(&body, "game_id").to_owned()
}

fn leave(acct: &Acct, game: &str) {
    let path = format!("/lobby/games/{game}/leave");
    let (status, body) = http(acct.port, "POST", &path, Some(&acct.session), "");
    assert_eq!(status, 204, "leave: {body}");
}

fn settings(acct: &Acct, share_table: bool) {
    let body = format!(
        "{{\"whispers_from\":\"friends_and_tables\",\"requests_open\":true,\"share_table\":{share_table}}}"
    );
    let (status, answer) = http(
        acct.port,
        "PUT",
        "/social/settings",
        Some(&acct.session),
        &body,
    );
    assert_eq!(status, 204, "{answer}");
}

#[tokio::test]
async fn only_a_friend_is_shown_that_someone_is_online() {
    let gw = spawn_gateway("social-presence-who");
    let port = gw.port;
    let alice = account(port, "alice-pr", "Alice");
    let stranger = account(port, "bob-pr", "Bob");
    let friend = account(port, "carol-pr", "Carol");
    befriend(&alice, &friend);
    let (a1, s1, f1) = (register(&alice), register(&stranger), register(&friend));
    let (mut sock_a, snapshot) = Chat::open(&a1).await;
    assert_eq!(
        snapshot.friends[0].presence.status,
        Shown::Offline,
        "Carol is not here yet"
    );
    let (mut sock_s, _) = Chat::open(&s1).await;

    // Carol comes online: her friend is told, with where she is.
    let (mut sock_f, snapshot) = Chat::open(&f1).await;
    assert_eq!(
        snapshot.friends[0].presence.status,
        Shown::Online,
        "Alice is, and Carol's snapshot says so"
    );
    let now = shown(&mut sock_a, &friend.tag, |p| p.status == Shown::Online).await;
    assert_eq!(now.place, Some(Place::Lobby));

    // Bob, who is Alice's and Carol's stranger, is told nothing of either
    // of them, nor are they of him: not then, not when anything changes.
    sock_f
        .up(&Up::Presence(PresenceSet {
            status: Status::Dnd,
            activity: None,
        }))
        .await;
    shown(&mut sock_a, &friend.tag, |p| p.status == Shown::Dnd).await;
    let (frames, closed) = sock_s.quiet(SILENCE).await;
    assert_eq!(closed, None);
    assert!(presence_frames(&frames).is_empty(), "{frames:?}");
    // And Alice has heard of Carol only: never of Bob's coming.
    let (frames, _) = sock_a.quiet(SILENCE).await;
    assert!(
        presence_frames(&frames)
            .iter()
            .all(|(tag, _)| *tag == friend.tag),
        "{frames:?}"
    );
    let (frames, _) = sock_f.quiet(Duration::from_millis(300)).await;
    assert!(
        presence_frames(&frames)
            .iter()
            .all(|(tag, _)| *tag == alice.tag)
    );
}

#[tokio::test]
async fn invisible_reads_as_offline_and_the_last_socket_closing_does_too() {
    let gw = spawn_gateway("social-presence-status");
    let port = gw.port;
    let alice = account(port, "alice-st", "Alice");
    let friend = account(port, "carol-st", "Carol");
    befriend(&alice, &friend);
    let a1 = register(&alice);
    let (f1, f2) = (register(&friend), register(&friend));
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_f1, _) = Chat::open(&f1).await;
    shown(&mut sock_a, &friend.tag, |p| p.status == Shown::Online).await;

    // Each status is what her friend is shown; invisible is offline, with
    // no place to give away.
    for (status, want) in [
        (Status::Away, Shown::Away),
        (Status::Dnd, Shown::Dnd),
        (Status::Invisible, Shown::Offline),
        (Status::Online, Shown::Online),
        (Status::Invisible, Shown::Offline),
    ] {
        sock_f1
            .up(&Up::Presence(PresenceSet {
                status,
                activity: None,
            }))
            .await;
        let now = shown(&mut sock_a, &friend.tag, |p| p.status == want).await;
        assert_eq!(
            now.place.is_none(),
            want == Shown::Offline,
            "{status:?}: {now:?}"
        );
    }

    // A device that opens now is told the same: no leak by snapshot.
    let a2 = register(&alice);
    let (_sock_a2, snapshot) = Chat::open(&a2).await;
    assert_eq!(snapshot.friends[0].presence, Presence::offline());

    // Back, on two devices: the first closing changes nothing, the last
    // one does, and at once.
    sock_f1
        .up(&Up::Presence(PresenceSet {
            status: Status::Online,
            activity: None,
        }))
        .await;
    shown(&mut sock_a, &friend.tag, |p| p.status == Shown::Online).await;
    let (sock_f2, _) = Chat::open(&f2).await;
    drop(sock_f1);
    let (frames, _) = sock_a.quiet(SILENCE).await;
    assert!(
        presence_frames(&frames)
            .iter()
            .all(|(_, p)| p.status != Shown::Offline),
        "one device left and one stayed: {frames:?}"
    );
    drop(sock_f2);
    let gone = shown(&mut sock_a, &friend.tag, |p| p.status == Shown::Offline).await;
    assert_eq!(gone, Presence::offline());
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one friend walking through a room, a locked room and a game
async fn a_friend_is_shown_where_the_table_is_only_when_it_is_shared_and_open() {
    let gw = spawn_gateway("social-presence-place");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = account(port, "alice-pl", "Alice");
    let friend = account(port, "carol-pl", "Carol");
    befriend(&alice, &friend);
    let (a1, f1) = (register(&alice), register(&friend));
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (_sock_f, _) = Chat::open(&f1).await;
    shown(&mut sock_a, &friend.tag, |p| p.place == Some(Place::Lobby)).await;

    // In a waiting room, shared: the game's id is there.
    let game = open_room(&friend, "");
    let now = shown(&mut sock_a, &friend.tag, |p| {
        matches!(p.place, Some(Place::Room { .. }))
    })
    .await;
    let Some(Place::Room {
        game: shown_game,
        name,
        seats,
        taken,
        locked,
    }) = now.place
    else {
        unreachable!()
    };
    assert_eq!(shown_game.as_deref(), Some(game.as_str()));
    assert_eq!(
        (name.as_str(), seats, taken, locked),
        ("Kitchen", 3, 1, false)
    );

    // She stops sharing her table: the room is still said, the id is not.
    settings(&friend, false);
    let now = shown(&mut sock_a, &friend.tag, |p| {
        matches!(p.place, Some(Place::Room { game: None, .. }))
    })
    .await;
    assert_eq!(now.status, Shown::Online);
    // And shares it again.
    settings(&friend, true);
    shown(
        &mut sock_a,
        &friend.tag,
        |p| matches!(&p.place, Some(Place::Room { game: Some(g), .. }) if *g == game),
    )
    .await;

    // Out of it, and into a room with a password: said to be locked, with
    // no id to try the lock with.
    leave(&friend, &game);
    shown(&mut sock_a, &friend.tag, |p| p.place == Some(Place::Lobby)).await;
    let locked_room = open_room(&friend, ",\"password\":\"kitchen-only\"");
    let now = shown(&mut sock_a, &friend.tag, |p| {
        matches!(p.place, Some(Place::Room { .. }))
    })
    .await;
    assert!(
        matches!(
            &now.place,
            Some(Place::Room {
                game: None,
                locked: true,
                ..
            })
        ),
        "{now:?}"
    );
    leave(&friend, &locked_room);
    shown(&mut sock_a, &friend.tag, |p| p.place == Some(Place::Lobby)).await;

    // At a running table.
    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"ai\"}}", deck(&friend));
    let (status, body) = http(port, "POST", "/lobby/games", Some(&friend.session), &create);
    assert_eq!(status, 200, "a game against the house: {body}");
    let running = json_field(&body, "game_id").to_owned();
    let now = shown(&mut sock_a, &friend.tag, |p| {
        matches!(p.place, Some(Place::Playing { .. }))
    })
    .await;
    assert!(
        matches!(&now.place, Some(Place::Playing { game: Some(g), .. }) if *g == running),
        "{now:?}"
    );
    // Not sharing her table hides a running one's id as it hides a room's.
    settings(&friend, false);
    shown(&mut sock_a, &friend.tag, |p| {
        matches!(p.place, Some(Place::Playing { game: None, .. }))
    })
    .await;
}

/// `players_online` as the lobby stats say it.
fn online(port: u16, session: &str) -> i64 {
    let (status, body) = http(port, "GET", "/lobby/stats", Some(session), "");
    assert_eq!(status, 200, "stats: {body}");
    json_number(&body, "players_online")
}

async fn online_becomes(port: u16, session: &str, want: i64) {
    for _ in 0..common::WAIT_TRIES {
        if online(port, session) == want {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!(
        "players_online never reached {want}: {}",
        online(port, session)
    );
}

#[tokio::test]
async fn an_account_with_only_a_chat_socket_is_online_once() {
    let gw = spawn_gateway("social-presence-count");
    let port = gw.port;
    let alice = account(port, "alice-ct", "Alice");
    let bob = account(port, "bob-ct", "Bob");
    let (a1, a2, b1) = (register(&alice), register(&alice), register(&bob));
    assert_eq!(online(port, &bob.session), 0, "nobody has a socket open");

    // Two devices of one account are one player, with no lobby socket.
    let (sock_a1, _) = Chat::open(&a1).await;
    let (sock_a2, _) = Chat::open(&a2).await;
    online_becomes(port, &bob.session, 1).await;
    let (sock_b1, _) = Chat::open(&b1).await;
    online_becomes(port, &bob.session, 2).await;

    // One of Alice's goes: she is still here; both: she is not.
    drop(sock_a1);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(online(port, &bob.session), 2);
    drop(sock_a2);
    online_becomes(port, &bob.session, 1).await;
    drop(sock_b1);
    online_becomes(port, &alice.session, 0).await;
}
