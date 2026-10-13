//! A reported chat message, and what the gateway keeps of chat in its
//! log and its database (`docs/protocol.md` §"Friends and chat (beta.8)",
//! §4.7). A recipient reveals one message to the feedback service through
//! the gateway, which checks that the sending device signed it and that the
//! revealed key opens it to the revealed body; a report that fails the check
//! is still passed on, marked unverified. Beside that: the gateway logs no
//! secret of chat, and does not deliver a message past its expiry.

#![allow(clippy::missing_docs_in_private_items, dead_code)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use baylee_protocol::social::{
    self as wire, ChatReport, Conv, Delivered, Down, Header, PresenceSet, Sender, Snapshot, Status,
    Up, Why, Wrap,
};
use baylee_seal::b64::url as b64;
use baylee_seal::testkit::{self, Seeded};
use baylee_seal::{DeviceSecrets, Envelope, Recipient};
use common::{
    Gateway, Socket, WAIT_BUDGET, ask_ticket, http, json_field, login, spawn_gateway,
    spawn_gateway_with,
};
use futures_util::{SinkExt, StreamExt};
use sea_orm::ConnectionTrait as _;
use tokio_tungstenite::tungstenite::Message;

const INTAKE: &str = "intake-token-for-the-chat-e2e";
const READ: &str = "read-token-for-the-chat-e2e";

/// A feedback service in a schema of its own, dropped with it.
struct Service {
    port: u16,
    schema: String,
    server: tokio::task::JoinHandle<()>,
}

impl Service {
    async fn start() -> Self {
        let base = common::database_url();
        let schema = format!("fb_chat_e2e_{}", uuid::Uuid::now_v7().simple());
        let admin = sea_orm::Database::connect(&base).await.expect("connecting");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("a schema");
        let sep = if base.contains('?') { '&' } else { '?' };
        let scoped = format!("{base}{sep}options=-c%20search_path%3D{schema}");
        let db = baylee_feedback::connect(&scoped, 2)
            .await
            .expect("the service migrates");
        let config = baylee_feedback::Config::new(&format!("e2e={INTAKE}"), Some(READ), None)
            .expect("config");
        let app = baylee_feedback::app(Arc::new(baylee_feedback::AppState { db, config }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self {
            port,
            schema,
            server,
        }
    }

    /// A report as the read API shows it, `client` included.
    fn report(&self, id: &str) -> serde_json::Value {
        let (status, body) = http(self.port, "GET", &format!("/reports/{id}"), Some(READ), "");
        assert_eq!(status, 200, "{body}");
        serde_json::from_str(&body).expect("a report")
    }

    /// The environment that points a gateway at this service.
    fn env(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                "BAYLEE_FEEDBACK_URL",
                format!("http://127.0.0.1:{}/", self.port),
            ),
            ("BAYLEE_FEEDBACK_TOKEN", INTAKE.to_owned()),
            (
                "BAYLEE_FEEDBACK_KEY",
                "the-chat-e2e-pseudonym-key".to_owned(),
            ),
        ]
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.server.abort();
        let schema = self.schema.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime")
                .block_on(async move {
                    if let Ok(db) = sea_orm::Database::connect(&common::database_url()).await {
                        let _ = db
                            .execute_unprepared(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
                            .await;
                    }
                });
        })
        .join();
    }
}

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

/// A message that went from Alice to Bob and that Bob opened: everything he
/// needs to report it.
struct Exchange {
    alice: Acct,
    bob: Acct,
    a1: Dev,
    b1: Dev,
    conv: String,
    delivered: Delivered,
    mk: [u8; 32],
    body: Vec<u8>,
    sock_a: Chat,
    sock_b: Chat,
}

async fn exchange(port: u16, words: &str) -> Exchange {
    let alice = account(port, "alice-rep", "Alice");
    let bob = account(port, "bob-rep", "Bob");
    befriend(&alice, &bob);
    let (a1, b1) = (register(&alice), register(&bob));
    let (mut sock_a, _) = Chat::open(&a1).await;
    let (mut sock_b, _) = Chat::open(&b1).await;
    let conv = dm(&alice, &bob);
    let answer = sock_a
        .send(1, &text(&a1, 1, &conv, words, &targets(&[&b1])))
        .await;
    sent(&answer);
    let got = sock_b.msg().await;
    let opened = open(&b1, &a1, &got.sealed);
    assert_eq!(&opened.body[..], &text_body(words)[..]);
    Exchange {
        alice,
        bob,
        a1,
        b1,
        conv,
        delivered: got.sealed,
        mk: *opened.mk,
        body: opened.body.to_vec(),
        sock_a,
        sock_b,
    }
}

impl Exchange {
    /// The report Bob would send for the message, as it was delivered.
    fn report(&self, note: &str) -> ChatReport {
        ChatReport {
            conv: self.conv.clone(),
            sealed: self.delivered.clone(),
            mk: b64::encode(&self.mk),
            body: b64::encode(&self.body),
            note: note.to_owned(),
        }
    }

    /// `POST /social/reports` as `by`: the status and the body.
    fn post(&self, by: &Acct, report: &ChatReport) -> (u16, String) {
        let body = serde_json::to_string(report).expect("a report");
        http(
            self.alice.port,
            "POST",
            "/social/reports",
            Some(&by.session),
            &body,
        )
    }
}

/// `field` of the report, flipped in its first byte and written again.
fn flipped(field: &str) -> String {
    let mut bytes = b64::decode(field).expect("base64url");
    bytes[0] ^= 1;
    b64::encode(&bytes)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // one message, reported intact and then broken each way
async fn a_reported_message_is_checked_and_reaches_the_service_as_a_chat_report() {
    let service = Service::start().await;
    let gw = spawn_gateway_with("social-report", &service.env());
    let port = gw.port;
    let x = exchange(port, "you are all useless").await;
    let at: u64 = serde_json::from_slice::<Header>(&b64::decode(&x.delivered.hdr).unwrap())
        .unwrap()
        .ts;

    // The message as it was delivered, with its key and its body.
    let (status, body) = x.post(&x.bob, &x.report("rude"));
    assert_eq!(status, 201, "{body}");
    let report = service.report(json_field(&body, "report_id"));
    assert_eq!(report["kind"], "chat");
    assert_eq!(report["client"]["chat"]["verified"], true, "{report}");
    assert_eq!(report["client"]["chat"]["conversation"], "whisper");
    assert_eq!(report["client"]["chat"]["message"], "you are all useless");
    assert_eq!(report["client"]["chat"]["note"], "rude");
    assert_eq!(report["client"]["chat"]["sent_at"], at);
    let text = report["text"].as_str().expect("the report's text");
    assert!(
        text.contains("rude") && text.contains("you are all useless"),
        "{text}"
    );
    // No sender pseudonym rides beside the tag: it is the keyed hash a
    // reporter is known by, so the pair would resolve the sender's own
    // reports. The reporter is a pseudonym, never an account.
    assert!(
        report["client"]["chat"].get("sender").is_none(),
        "no sender pseudonym: {report}"
    );
    assert_ne!(
        report["reporter"].as_str(),
        Some(x.bob.id.as_str()),
        "{report}"
    );
    // A verified message names its sender by public tag, so a moderator
    // can find the account (`GET /admin/accounts?tag=`); the tag is the
    // sender's, and the reporter is still only the keyed pseudonym.
    assert_eq!(
        report["client"]["chat"]["sender_tag"], x.alice.tag,
        "the sender's tag, in hex: {report}"
    );
    let reporter = report["reporter"].as_str().expect("a reporter");
    assert!(!reporter.is_empty(), "{report}");
    for who in [x.bob.tag.as_str(), x.bob.handle.as_str(), x.bob.id.as_str()] {
        assert_ne!(reporter, who, "the reporter is no tag, handle or id");
    }
    assert!(
        !report.to_string().contains(&x.bob.handle) && !report.to_string().contains(&x.bob.id),
        "nothing of the reporter's account rides in the report: {report}"
    );

    // Each way of not being the message the sender sent is still reported,
    // and says so: the body altered, the key altered, the ciphertext, the
    // signature, the header.
    let mut broken: Vec<(&str, ChatReport)> = Vec::new();
    let mut body_changed = x.report("");
    let mut bytes = x.body.clone();
    bytes[9] ^= 1; // the first letter of the text
    body_changed.body = b64::encode(&bytes);
    broken.push(("a body altered by one byte", body_changed));
    let mut key_changed = x.report("");
    key_changed.mk = flipped(&key_changed.mk);
    broken.push(("a message key altered", key_changed));
    let mut ct_changed = x.report("");
    ct_changed.sealed.ct = flipped(&ct_changed.sealed.ct);
    broken.push(("a ciphertext altered", ct_changed));
    let mut sig_changed = x.report("");
    sig_changed.sealed.sig = flipped(&sig_changed.sealed.sig);
    broken.push(("a signature altered", sig_changed));
    let mut nonce_changed = x.report("");
    nonce_changed.sealed.n = flipped(&nonce_changed.sealed.n);
    broken.push(("a nonce altered", nonce_changed));
    for (what, report) in &broken {
        let (status, body) = x.post(&x.bob, report);
        assert_eq!(status, 201, "{what}: {body}");
        let stored = service.report(json_field(&body, "report_id"));
        assert_eq!(stored["kind"], "chat", "{what}");
        assert_eq!(
            stored["client"]["chat"]["verified"], false,
            "{what}: {stored}"
        );
        // Unverified, so it could name anyone: no tag is given.
        assert!(
            stored["client"]["chat"]["sender_tag"].is_null(),
            "{what}: {stored}"
        );
    }

    // The sending device removed since: nothing to check the signature
    // with, so the report goes on unverified rather than refused.
    let path = format!("/social/devices/{}", x.a1.device);
    let (status, body) = http(port, "DELETE", &path, Some(&x.alice.session), "");
    assert_eq!(status, 204, "{body}");
    let (status, body) = x.post(&x.bob, &x.report("device gone"));
    assert_eq!(status, 201, "{body}");
    let stored = service.report(json_field(&body, "report_id"));
    assert_eq!(stored["client"]["chat"]["verified"], false, "{stored}");
    assert!(stored["client"]["chat"]["sender_tag"].is_null(), "{stored}");
}

#[tokio::test]
async fn a_report_without_a_service_or_in_the_wrong_shape_is_refused() {
    // No feedback service: a good report is 503, and the shape comes first.
    let gw = spawn_gateway("social-report-off");
    let port = gw.port;
    let x = exchange(port, "hello").await;
    let (status, body) = x.post(&x.bob, &x.report("rude"));
    assert_eq!(status, 503, "{body}");
    assert!(body.contains("not configured"), "{body}");

    let post = |report: &ChatReport| x.post(&x.bob, report).0;
    let mut other_conv = x.report("");
    other_conv.conv = format!("grp:{}", uuid::Uuid::now_v7());
    assert_eq!(
        post(&other_conv),
        400,
        "a conversation the message is not of"
    );
    let mut not_conv = x.report("");
    not_conv.conv = "everyone".to_owned();
    assert_eq!(post(&not_conv), 400, "not a conversation");
    let mut short_key = x.report("");
    short_key.mk = b64::encode(&x.mk[..31]);
    assert_eq!(post(&short_key), 400, "a key of 31 bytes");
    let mut padded = x.report("");
    padded.mk.push('=');
    assert_eq!(post(&padded), 400, "padded base64");
    let mut not_hdr = x.report("");
    not_hdr.sealed.hdr = "not base64 at all!".to_owned();
    assert_eq!(post(&not_hdr), 400, "a header that is not base64");
    let mut hdr_not_json = x.report("");
    hdr_not_json.sealed.hdr = b64::encode(b"this is not a header");
    assert_eq!(post(&hdr_not_json), 400, "a header that is not JSON");
    let mut big_body = x.report("");
    big_body.body = b64::encode(&vec![b'a'; wire::MAX_BODY_BYTES + 1]);
    assert_eq!(post(&big_body), 400, "a body past the limit");
    let mut long_note = x.report(&"n".repeat(wire::MAX_REPORT_NOTE + 1));
    assert_eq!(post(&long_note), 400, "a note past the limit");
    long_note.note = "n".repeat(wire::MAX_REPORT_NOTE);
    assert_eq!(
        post(&long_note),
        503,
        "a note at the limit is a good report"
    );

    let (status, body) = http(port, "POST", "/social/reports", Some(&x.bob.session), "{}");
    assert_eq!(status, 400, "{body}");
    let (status, body) = http(
        port,
        "POST",
        "/social/reports",
        Some(&x.bob.session),
        "nope",
    );
    assert_eq!(status, 400, "{body}");
    let huge = format!("{{\"note\":\"{}\"}}", "x".repeat(70 * 1024));
    let (status, body) = http(port, "POST", "/social/reports", Some(&x.bob.session), &huge);
    assert_eq!(status, 413, "{body}");
    let good = serde_json::to_string(&x.report("")).unwrap();
    let (status, body) = http(port, "POST", "/social/reports", None, &good);
    assert_eq!(status, 401, "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn eleven_reports_in_an_hour_are_too_many_for_one_account() {
    let service = Service::start().await;
    let gw = spawn_gateway_with("social-report-rate", &service.env());
    let port = gw.port;
    let x = exchange(port, "spam").await;
    for n in 1..=10 {
        let (status, body) = x.post(&x.bob, &x.report("again"));
        assert_eq!(status, 201, "report {n}: {body}");
    }
    let (status, body) = x.post(&x.bob, &x.report("again"));
    assert_eq!(status, 429, "{body}");
    // The allowance is the reporter's own.
    let (status, body) = x.post(&x.alice, &x.report("mine"));
    assert_eq!(status, 201, "{body}");
}

// ---------------------------------------------------------------------------
// What the log never holds.
// ---------------------------------------------------------------------------

/// `log` without the colour codes the gateway's log lines carry.
fn plain(log: &str) -> String {
    let mut out = String::with_capacity(log.len());
    let mut chars = log.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Where `needle` stands in `log` as a word of its own (no letter or digit
/// on either side), with the line around it; a four-digit tag is otherwise
/// found inside every number.
fn word_in(log: &str, needle: &str) -> Option<String> {
    let bytes = log.as_bytes();
    let mut from = 0;
    while let Some(at) = log[from..].find(needle) {
        let start = from + at;
        let end = start + needle.len();
        let before = start.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(end).copied();
        let part = |b: Option<u8>| b.is_some_and(|b| b.is_ascii_alphanumeric());
        if !part(before) && !part(after) {
            let line_start = log[..start].rfind('\n').map_or(0, |i| i + 1);
            let line_end = log[end..].find('\n').map_or(log.len(), |i| end + i);
            return Some(log[line_start..line_end].to_owned());
        }
        from = end;
    }
    None
}

/// A social socket opened with a ticket the caller keeps, to look for it.
async fn open_noting(dev: &Dev, tickets: &mut Vec<String>) -> Chat {
    let acct = &dev.acct;
    let (status, body) = ask_ticket(acct.port, &acct.session, r#"{"socket":"social"}"#);
    assert_eq!(status, 200, "{body}");
    let ticket = json_field(&body, "ticket").to_owned();
    let url = format!(
        "ws://127.0.0.1:{}{}",
        acct.port,
        baylee_protocol::social::social_socket_path(&ticket)
    );
    tickets.push(ticket);
    let mut chat = Chat {
        ws: common::dial(&url).await.expect("the social socket opens"),
    };
    chat.up(&hello(&dev.device, Status::Online)).await;
    let welcome = chat.next_frame().await;
    assert!(matches!(welcome, Down::Welcome { .. }), "{welcome:?}");
    let snapshot = chat.next_frame().await;
    assert!(matches!(snapshot, Down::Snapshot(_)), "{snapshot:?}");
    chat
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one conversation, then every secret of it looked for
async fn nothing_secret_reaches_the_log() {
    let gw = spawn_gateway_with("social-log", &[("RUST_LOG", "debug".to_owned())]);
    let port = gw.port;
    let alice = account(port, "alice-log", "Alicia");
    let bob = account(port, "bob-log", "Robert");
    befriend(&alice, &bob);
    let (a1, b1) = (register(&alice), register(&bob));
    let mut tickets = Vec::new();
    let mut sock_a = open_noting(&a1, &mut tickets).await;
    let mut sock_b = open_noting(&b1, &mut tickets).await;

    // A message that goes, one that is refused, an ack, presence.
    let conv = dm(&alice, &bob);
    let secret_words = "the-moon-is-made-of-green-cheese-7731";
    let Up::Send { sealed, .. } = text(&a1, 1, &conv, secret_words, &targets(&[&b1])) else {
        unreachable!()
    };
    let answer = sock_a
        .send(
            1,
            &Up::Send {
                req: 1,
                conv: conv.clone(),
                sealed: sealed.clone(),
            },
        )
        .await;
    sent(&answer);
    let got = sock_b.msg().await;
    assert_eq!(
        &open(&b1, &a1, &got.sealed).body[..],
        &text_body(secret_words)[..]
    );
    let stranger_conv = format!("grp:{}", uuid::Uuid::now_v7());
    let answer = sock_a
        .send(
            2,
            &text(&a1, 2, &stranger_conv, secret_words, &targets(&[&b1])),
        )
        .await;
    assert_eq!(refusal(&answer).0, Why::NotMember);
    sock_b.up(&Up::Ack { upto: got.seq }).await;
    sock_b
        .up(&Up::Presence(PresenceSet {
            status: Status::Away,
            activity: None,
        }))
        .await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let (status, body) = http(
        port,
        "GET",
        &format!("/social/keys?tag={}", bob.tag),
        Some(&alice.session),
        "",
    );
    assert_eq!(status, 200, "{body}");
    drop((sock_a, sock_b));
    tokio::time::sleep(Duration::from_millis(700)).await;
    let log = plain(&gw.logs());

    // The scan can see: what the gateway does say is there, at the level the
    // test asked for, including this exchange's own lines.
    assert!(
        log.contains("baylee-gateway listening"),
        "no startup line:\n{log}"
    );
    assert!(
        log.contains("social: a send was refused"),
        "no debug line of the exchange; the scan reads nothing:\n{log}"
    );

    // The word scan sees a real word of the real log (the port it listens
    // on) as well as the made-up one below.
    assert!(
        word_in(&log, &port.to_string()).is_some(),
        "the scan cannot find a number the log has:\n{log}"
    );

    // And what it must not say is not.
    let mut secrets: Vec<(String, String)> = Vec::new();
    for ticket in &tickets {
        secrets.push(("a social ticket".to_owned(), ticket.clone()));
    }
    for (what, value) in [
        ("the ciphertext", &sealed.ct),
        ("the header", &sealed.hdr),
        ("the nonce", &sealed.n),
        ("the ephemeral key", &sealed.epk),
        ("the signature", &sealed.sig),
        ("a wrap", &got.sealed.k),
        ("Alice's session", &alice.session),
        ("Bob's session", &bob.session),
        ("the plaintext", &secret_words.to_owned()),
        (
            "the body",
            &String::from_utf8(text_body(secret_words)).unwrap(),
        ),
        ("Alice's handle", &alice.handle),
        ("Bob's handle", &bob.handle),
        ("Alice's device key", &b64::encode(&a1.secrets.dk())),
        ("Bob's device key", &b64::encode(&b1.secrets.dk())),
    ] {
        secrets.push((what.to_owned(), value.clone()));
    }
    for (what, secret) in &secrets {
        assert!(
            !log.contains(secret.as_str()),
            "{what} is in the log:\n{}",
            log.lines()
                .find(|line| line.contains(secret.as_str()))
                .unwrap_or_default()
        );
    }
    for (what, tag) in [("Alice's tag", &alice.tag), ("Bob's tag", &bob.tag)] {
        assert_eq!(word_in(&log, tag), None, "{what} is in the log");
    }
    // The display names are part of the handles and so of this scan, and
    // the scan finds a handle when one is there.
    assert_eq!(
        word_in("a line #af03 here", "af03").as_deref(),
        Some("a line #af03 here")
    );
    assert_eq!(word_in("a line 1af03 here", "af03"), None);
}

// ---------------------------------------------------------------------------
// Expiry.
// ---------------------------------------------------------------------------

/// A second gateway on the first one's schema, for its start-up sweep.
struct Second {
    child: std::process::Child,
    files: Vec<std::path::PathBuf>,
    log: std::path::PathBuf,
}

impl Second {
    async fn start(gw: &Gateway) -> Self {
        let id = uuid::Uuid::now_v7().simple().to_string();
        let tmp = std::env::temp_dir();
        let port_file = tmp.join(format!("baylee-social-second-{id}.port"));
        let log = tmp.join(format!("baylee-social-second-{id}.log"));
        let stderr = tmp.join(format!("baylee-social-second-{id}.stderr"));
        let cwd = tmp.join(format!("baylee-social-second-{id}.cwd"));
        std::fs::create_dir_all(&cwd).expect("a working directory");
        let child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
            .current_dir(&cwd)
            .env("PORT", "0")
            .env("BAYLEE_PORT_FILE", &port_file)
            .env(
                "STORE_PATH",
                tmp.join(format!("baylee-social-second-{id}.json")),
            )
            .env("DATABASE_URL", gw.database_url())
            .env("BAYLEE_DB_POOL", "2")
            .env("BAYLEE_ART_PATH", "off")
            .env("BAYLEE_DECK_IMAGE_PATH", "off")
            .env("RUST_LOG", "info")
            .stdout(std::fs::File::create(&log).expect("a log"))
            .stderr(std::fs::File::create(&stderr).expect("a stderr"))
            .spawn()
            .expect("a second gateway");
        let mut second = Self {
            child,
            files: vec![port_file.clone(), log.clone(), stderr.clone()],
            log,
        };
        // The port file is written once it listens, after the sweep it
        // makes as it starts.
        for _ in 0..common::WAIT_TRIES {
            if std::fs::read_to_string(&port_file).is_ok_and(|text| !text.trim().is_empty()) {
                return second;
            }
            if let Ok(Some(status)) = second.child.try_wait() {
                panic!(
                    "the second gateway exited ({status}): {}",
                    std::fs::read_to_string(&stderr).unwrap_or_default()
                );
            }
            tokio::time::sleep(common::WAIT_STEP).await;
        }
        panic!("the second gateway never listened");
    }
}

impl Drop for Second {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for file in &self.files {
            let _ = std::fs::remove_file(file);
        }
    }
}

#[tokio::test]
async fn a_message_past_its_expiry_is_not_delivered_and_is_swept_at_start() {
    let gw = spawn_gateway("social-expiry");
    let port = gw.port;
    let alice = account(port, "alice-exp", "Alice");
    let bob = account(port, "bob-exp", "Bob");
    befriend(&alice, &bob);
    let (a1, b1) = (register(&alice), register(&bob));
    let conv = dm(&alice, &bob);
    let (mut sock_a, _) = Chat::open(&a1).await;
    let to = targets(&[&b1]);

    // Two messages wait for Bob; the first's slot runs out.
    let (stale, _) = sent(&sock_a.send(1, &text(&a1, 1, &conv, "old news", &to)).await);
    let (fresh, _) = sent(&sock_a.send(2, &text(&a1, 2, &conv, "fresh", &to)).await);
    assert_eq!((slots(&gw, &b1.device), stored(&gw)), (2, 2));
    gw.sql(&format!(
        "UPDATE chat_delivery SET expires_at = now() - interval '1 hour' \
         WHERE message_id = '{stale}'::uuid"
    ));

    // On connecting, Bob is sent the fresh one only.
    let (mut sock_b, _) = Chat::open(&b1).await;
    let got = sock_b.msg().await;
    let hdr: Header = serde_json::from_slice(&b64::decode(&got.sealed.hdr).unwrap()).unwrap();
    assert_eq!(hdr.id, fresh);
    assert_eq!(
        &open(&b1, &a1, &got.sealed).body[..],
        &text_body("fresh")[..]
    );
    assert!(
        sock_b
            .deliveries_within(Duration::from_millis(1200))
            .await
            .is_empty()
    );
    drop(sock_b);

    // Nothing has removed the lapsed slot yet; the sweep a gateway makes as
    // it starts does, and the message with it, and leaves the fresh one.
    assert_eq!((slots(&gw, &b1.device), stored(&gw)), (2, 2));
    let second = Second::start(&gw).await;
    assert_eq!((slots(&gw, &b1.device), stored(&gw)), (1, 1));
    let said = plain(&std::fs::read_to_string(&second.log).unwrap_or_default());
    assert!(said.contains("undelivered chat messages expired"), "{said}");
    assert!(
        said.contains("slots=1") && said.contains("messages=1"),
        "{said}"
    );
    // Counts only: no id of the message is in what it said.
    assert!(!said.contains(&stale) && !said.contains(&fresh), "{said}");
}
