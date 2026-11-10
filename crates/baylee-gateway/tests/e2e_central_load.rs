//! A session owes Central nothing once it is open (`docs/central-login.md`
//! §"Sessions"): nothing it does asks Central, and all Central can do to it is
//! end it.
//!
//! Two things have to hold at once, and each test holds one side of it. A
//! session Central revokes closes at once, whatever else is going on. And a
//! session nobody revoked stays open: through Central being away, through an
//! answer that is no answer, and through somebody else's load, which is the
//! case that started this. Against a stub Central on loopback.

#![allow(clippy::missing_docs_in_private_items)]

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

mod common;
mod stub_central;

use common::{http, lobby_url};
use futures_util::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use stub_central::{Says, Stub, gateway, gateway_starting, linked};
use tokio_tungstenite::tungstenite::Message;

const VICTIM: u8 = 1;
const ATTACKER: u8 = 2;
const NEWCOMER: u8 = 3;

/// How long a socket is watched under load.
const WATCHED: Duration = Duration::from_secs(6);

/// How long a socket is given to close, or to prove it does not.
const AT_ONCE: Duration = Duration::from_secs(3);

/// The most a gateway asks Central of its own accord in [`WATCHED`] and some:
/// a round of its catch-up, while a request that asked would come to thousands.
const ITS_OWN_ROUNDS: u64 = 8;

/// How long it took `socket` to close, if it did within `span`.
async fn closed_after(socket: &mut common::Socket, span: Duration) -> Option<Duration> {
    let started = Instant::now();
    tokio::time::timeout(span, async {
        loop {
            match socket.next().await {
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {}
            }
        }
    })
    .await
    .ok()
    .map(|()| started.elapsed())
}

/// `loops` threads asking `ask` over and over until they are told to finish.
struct Load {
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Load {
    fn start(loops: usize, ask: impl Fn() + Send + Sync + 'static) -> Self {
        let ask = Arc::new(ask);
        let stop = Arc::new(AtomicBool::new(false));
        let threads = (0..loops)
            .map(|_| {
                let (ask, stop) = (Arc::clone(&ask), Arc::clone(&stop));
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        ask();
                    }
                })
            })
            .collect();
        Self { stop, threads }
    }

    fn finish(self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads {
            let _ = thread.join();
        }
    }
}

/// Asks `/me` as `bearer` for [`WATCHED`]: how often it was asked, and how
/// often answered.
async fn ask_while_watched(port: u16, bearer: &str) -> (u32, u32) {
    let (mut answered, mut asked) = (0, 0);
    let watching = Instant::now();
    while watching.elapsed() < WATCHED {
        let bearer = bearer.to_owned();
        let status =
            tokio::task::spawn_blocking(move || http(port, "GET", "/me", Some(&bearer), "").0)
                .await
                .expect("asked");
        asked += 1;
        answered += u32::from(status == 200);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    (asked, answered)
}

/// The case that started this: one account's requests take nothing from
/// another's. The socket stays open, its owner is still answered, and Central
/// hears of none of it.
#[tokio::test(flavor = "multi_thread")]
async fn one_account_s_load_costs_another_account_nothing() {
    let stub = Stub::serve(Duration::from_millis(50));
    let (gw, _settings) = gateway(&stub, "load_account");
    let victim = linked(&gw, &stub, VICTIM);
    let attacker = linked(&gw, &stub, ATTACKER);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the victim's feed");
    let port = gw.port;
    let before = stub.answered();
    let load = Load::start(100, move || {
        http(port, "GET", "/me", Some(&attacker), "");
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (asked, answered) = ask_while_watched(port, &victim).await;
    let closed = closed_after(&mut socket, Duration::from_millis(100)).await;
    load.finish();
    let central = stub.answered() - before;
    eprintln!("account load: victim answered {answered} of {asked}, Central asked {central}");
    assert_eq!(closed, None, "a healthy socket closed under another's load");
    assert_eq!(
        answered, asked,
        "the victim was refused under another's load"
    );
    assert!(central < ITS_OWN_ROUNDS, "requests asked Central {central}");
}

/// A sign-in takes no session, so strangers can ask as many as they like, and
/// none of that is taken from the sessions already signed in.
#[tokio::test(flavor = "multi_thread")]
async fn a_stranger_s_sign_ins_cost_a_session_nothing() {
    let stub = Stub::serve(Duration::from_millis(50));
    let (gw, _settings) = gateway(&stub, "load_stranger");
    let victim = linked(&gw, &stub, VICTIM);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the victim's feed");
    let port = gw.port;
    let before = stub.answered();
    let junk = serde_json::json!({ "token": "not.a.token" }).to_string();
    let load = Load::start(100, move || {
        http(port, "POST", "/auth/central/login", None, &junk);
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (asked, answered) = ask_while_watched(port, &victim).await;
    let closed = closed_after(&mut socket, Duration::from_millis(100)).await;
    load.finish();
    let central = stub.answered() - before;
    eprintln!("stranger load: victim answered {answered} of {asked}, Central asked {central}");
    assert_eq!(
        closed, None,
        "a healthy socket closed under strangers' sign-ins"
    );
    assert_eq!(
        answered, asked,
        "a session was refused under strangers' sign-ins"
    );
    assert!(central < ITS_OWN_ROUNDS, "sign-ins asked Central {central}");
}

/// A revocation pushed here closes the socket then and there, and its bearer
/// opens nothing more.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_central_revokes_closes_at_once() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "revoked");
    let victim = linked(&gw, &stub, VICTIM);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the feed");
    stub.revoke(VICTIM);
    stub.push(gw.port);
    let closed = closed_after(&mut socket, AT_ONCE).await;
    assert!(closed.is_some(), "a revoked session's socket stayed open");
    assert_eq!(http(gw.port, "GET", "/me", Some(&victim), "").0, 401);
}

/// `says` is no revocation. Central is asked and answers that: the session's
/// socket stays, its bearer renews, and a token it signed before still signs
/// somebody in, because none of that was ever Central's to answer. A revocation
/// lands the moment Central can be heard again.
async fn stands_through(says: Says, label: &str) {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, label);
    let victim = linked(&gw, &stub, VICTIM);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the feed");
    stub.knows(NEWCOMER);

    stub.say(says);
    let seen = stub.answered();
    stub.nudge(gw.port);
    stub.answers_after(seen);
    assert_eq!(closed_after(&mut socket, AT_ONCE).await, None);

    let (status, renewed) = http(gw.port, "POST", "/auth/session/renew", Some(&victim), "");
    assert_eq!(status, 200, "{renewed}");
    let bearer = common::json_field(&renewed, "token").to_owned();
    assert_eq!(http(gw.port, "GET", "/me", Some(&bearer), "").0, 200);
    let (status, answer) = stub.sign_in(gw.port, NEWCOMER);
    assert_eq!(status, 200, "{answer}");

    stub.say(Says::Good);
    stub.revoke(VICTIM);
    stub.nudge(gw.port);
    let closed = closed_after(&mut socket, AT_ONCE).await;
    assert!(closed.is_some(), "the revocation after it went unheard");
}

/// Central being away says nothing about any session.
#[tokio::test(flavor = "multi_thread")]
async fn central_being_away_ends_no_session() {
    stands_through(Says::Down, "central_down").await;
}

/// An answer that is no answer is not a refusal: a proxy's error page is no
/// reason to end every session it stands in front of.
#[tokio::test(flavor = "multi_thread")]
async fn a_page_that_is_no_answer_ends_no_session() {
    stands_through(Says::Garbage, "central_garbage").await;
}

/// Central refusing this gateway's credential is about the gateway: it names
/// no session, so it ends none.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_credential_ends_no_session() {
    stands_through(Says::Refused, "central_refused").await;
}

/// A gateway that has not heard from Central cannot know whom it revoked, so
/// it signs nobody in until it has; the fresh token waits and works.
#[test]
fn a_gateway_that_has_not_caught_up_signs_nobody_in() {
    let stub = Stub::serve(Duration::ZERO);
    stub.say(Says::Down);
    let (gw, _settings) = gateway_starting(&stub, "central_first_away");
    stub.knows(VICTIM);
    assert_eq!(stub.sign_in(gw.port, VICTIM).0, 503);
    stub.say(Says::Good);
    common::central_ready(gw.port);
    let (status, answer) = stub.sign_in(gw.port, VICTIM);
    assert_eq!(status, 200, "{answer}");
}
