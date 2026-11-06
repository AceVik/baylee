//! Stage 1 of `docs/proposals/central-session-tokens.md`: every session still
//! asks Central, and what Central cannot answer is not a revocation.
//!
//! Two things have to hold at once, and each test holds one side of it. A
//! session Central refuses closes at once, whatever else is going on. And a
//! session nobody refused stays open: through Central being away, through an
//! answer that is no answer, and through somebody else's load, which is the
//! case that started this. Against a stub Central on loopback.

#![allow(clippy::missing_docs_in_private_items)]

mod common;
mod stub_central;

use common::{http, lobby_url};
use futures_util::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use stub_central::{Says, Stub, gateway, linked};
use tokio_tungstenite::tungstenite::Message;

const VICTIM: u8 = 1;
const ATTACKER: u8 = 2;

/// How long a socket is watched: two of its checks and some.
const WATCHED: Duration = Duration::from_secs(12);

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

/// `loops` threads asking `ask` over and over until the guard they are given
/// is dropped; how many of their requests came back refused.
struct Load {
    stop: Arc<AtomicBool>,
    refused: Arc<AtomicU64>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Load {
    fn start(loops: usize, ask: impl Fn() -> u16 + Send + Sync + 'static) -> Self {
        let ask = Arc::new(ask);
        let stop = Arc::new(AtomicBool::new(false));
        let refused = Arc::new(AtomicU64::new(0));
        let threads = (0..loops)
            .map(|_| {
                let (ask, stop, refused) =
                    (Arc::clone(&ask), Arc::clone(&stop), Arc::clone(&refused));
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        if ask() != 200 {
                            refused.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                })
            })
            .collect();
        Self {
            stop,
            refused,
            threads,
        }
    }

    fn finish(self) -> u64 {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads {
            let _ = thread.join();
        }
        self.refused.load(Ordering::Relaxed)
    }
}

/// The case that started this: one account's requests fill nobody else's
/// place. The socket stays open, and its owner is still answered.
#[tokio::test(flavor = "multi_thread")]
async fn one_account_s_load_costs_another_account_nothing() {
    let stub = Stub::serve(Duration::from_millis(50));
    let (gw, _settings) = gateway(&stub, "load_account");
    let victim = linked(&gw, &stub, VICTIM);
    let attacker = linked(&gw, &stub, ATTACKER);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the victim's feed");
    let port = gw.port;
    let load = Load::start(100, move || http(port, "GET", "/me", Some(&attacker), "").0);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (mut answered, mut asked) = (0, 0);
    let watching = Instant::now();
    while watching.elapsed() < WATCHED {
        let victim = victim.clone();
        let status =
            tokio::task::spawn_blocking(move || http(port, "GET", "/me", Some(&victim), "").0)
                .await
                .expect("asked");
        asked += 1;
        answered += u32::from(status == 200);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let closed = closed_after(&mut socket, Duration::from_millis(100)).await;
    let refused = load.finish();
    eprintln!(
        "account load: victim answered {answered} of {asked}, attacker refused {refused}, \
         stub answered {}, at most {} at once",
        stub.answered(),
        stub.peak()
    );
    assert_eq!(closed, None, "a healthy socket closed under another's load");
    assert_eq!(
        answered, asked,
        "the victim was refused under another's load"
    );
    assert!(refused > 0, "the load never reached its share");
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
    let zeros = [0_u8; 32];
    let junk = serde_json::json!({ "ticket": zeros }).to_string();
    let load = Load::start(100, move || {
        http(port, "POST", "/auth/central/login", None, &junk).0
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (mut answered, mut asked) = (0, 0);
    let watching = Instant::now();
    while watching.elapsed() < WATCHED {
        let victim = victim.clone();
        let status =
            tokio::task::spawn_blocking(move || http(port, "GET", "/me", Some(&victim), "").0)
                .await
                .expect("asked");
        asked += 1;
        answered += u32::from(status == 200);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let closed = closed_after(&mut socket, Duration::from_millis(100)).await;
    load.finish();
    eprintln!("stranger load: victim answered {answered} of {asked}");
    assert_eq!(
        closed, None,
        "a healthy socket closed under strangers' sign-ins"
    );
    assert_eq!(
        answered, asked,
        "a session was refused under strangers' sign-ins"
    );
}

/// Central refusing a lease is a revocation, and closes at the next check,
/// whatever the grace for an unknown answer says.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_central_refuses_closes_at_once() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "revoked");
    let victim = linked(&gw, &stub, VICTIM);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the feed");
    stub.say(VICTIM, Says::Revoked);
    let closed = closed_after(&mut socket, Duration::from_secs(8)).await;
    assert!(closed.is_some(), "a revoked session's socket stayed open");
}

/// Central being away says nothing about any session.
#[tokio::test(flavor = "multi_thread")]
async fn central_being_away_closes_no_socket() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "central_down");
    let victim = linked(&gw, &stub, VICTIM);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the feed");
    stub.say(VICTIM, Says::Down);
    assert_eq!(closed_after(&mut socket, WATCHED).await, None);
    // And a real refusal still lands once it can be heard.
    stub.say(VICTIM, Says::Revoked);
    let closed = closed_after(&mut socket, Duration::from_secs(8)).await;
    assert!(
        closed.is_some(),
        "the refusal after the outage went unheard"
    );
}

/// An answer that is no answer is not a refusal: a proxy's error page is no
/// reason to end every session it stands in front of.
#[tokio::test(flavor = "multi_thread")]
async fn a_page_that_is_no_answer_closes_no_socket() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "central_garbage");
    let victim = linked(&gw, &stub, VICTIM);
    let url = lobby_url(gw.port, &victim, "");
    let mut socket = common::dial(&url).await.expect("the feed");
    stub.say(VICTIM, Says::Garbage);
    assert_eq!(closed_after(&mut socket, WATCHED).await, None);
}
