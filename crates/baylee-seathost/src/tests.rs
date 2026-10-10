//! The seat agent against a launcher that runs nothing: what it tells the
//! gateway, what it refuses, and where a key goes.

use super::*;
use baylee_client_core::llmseat::keys::{KeyEntry, MemoryKeys};
use launch::Running;
use serde_json::json;

/// A launcher that plays no game: a bridge "sits" at once and runs until
/// it is stopped; a check answers what the test set.
#[derive(Default)]
struct Fake {
    jobs: Mutex<Vec<(String, String, String)>>,
    probe: Mutex<Option<Probe>>,
    probes: Mutex<Vec<(String, bool)>>,
}

impl Launcher for Arc<Fake> {
    fn run(
        &self,
        job: SeatJob,
        started: oneshot::Sender<()>,
        stop: oneshot::Receiver<()>,
    ) -> Running<Exit> {
        self.jobs.lock().unwrap().push((
            job.order.clone(),
            job.chair_ticket.clone(),
            job.settings.clone(),
        ));
        Box::pin(async move {
            let _ = started.send(());
            let _ = stop.await;
            Exit::Ended
        })
    }

    fn probe(&self, _settings: String, profile: String, canary: bool) -> Running<Probe> {
        self.probes.lock().unwrap().push((profile, canary));
        let probe = self.probe.lock().unwrap().clone().unwrap_or(Probe::Ok);
        Box::pin(async move { probe })
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-seathost-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn host(name: &str, capacity: u32) -> (Arc<Host>, Arc<Fake>, Arc<MemoryKeys>) {
    let fake = Arc::new(Fake::default());
    let keys = Arc::new(MemoryKeys::default());
    let host = Host::new(Config {
        name: "test".into(),
        gateway: "http://127.0.0.1:1".into(),
        token: "t".into(),
        capacity,
        state_dir: scratch(name),
        bridge_gateway: None,
        keys: keys.clone(),
        launcher: Arc::new(fake.clone()),
    })
    .unwrap();
    (host, fake, keys)
}

fn sonnet(max_games: Option<u32>, caps: Option<serde_json::Value>) -> Definition {
    Definition {
        label: "Sonnet".into(),
        vendor: "Anthropic".into(),
        enabled: true,
        max_games,
        caps,
        canary: true,
        profile: json!({"provider": "anthropic", "model": "claude-sonnet-5-5"}),
    }
}

async fn next(rx: &mut mpsc::UnboundedReceiver<Frame>) -> Frame {
    tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("a frame in time")
        .expect("the channel is open")
}

/// The next frame that is not a profile report.
async fn next_but_reports(rx: &mut mpsc::UnboundedReceiver<Frame>) -> Frame {
    loop {
        match next(rx).await {
            Frame::Profiles { .. } => {}
            other => return other,
        }
    }
}

fn start(order: &str) -> Frame {
    Frame::StartSeat {
        order: order.into(),
        game_id: "g1".into(),
        seat: 1,
        profile: "sonnet".into(),
        chair_ticket: "the-chair-ticket".into(),
        gateway_url: "http://127.0.0.1:2".into(),
        deck_text: None,
    }
}

fn reported(host: &Host, id: &str) -> Reported {
    host.report().into_iter().find(|p| p.id == id).unwrap()
}

/// Written through the console, a profile is kept on disk (0600), reported
/// with its state, played up to its bound, and refused past it.
#[tokio::test]
async fn a_written_profile_is_kept_offered_and_bounded() {
    let (host, fake, _) = host("bounded", 0);
    let mut rx = host.attach();
    host.handle(Frame::ProfileWrite {
        request: "r1".into(),
        id: "sonnet".into(),
        definition: sonnet(Some(1), None),
        admin: "ada".into(),
    });
    assert_eq!(
        next_but_reports(&mut rx).await,
        Frame::Answer {
            request: "r1".into(),
            ok: true,
            error: None
        }
    );
    let file = host.config.state_dir.join(PROFILES_FILE);
    assert!(read_profiles(&file).unwrap().contains_key("sonnet"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    let p = reported(&host, "sonnet");
    assert_eq!(
        (p.state, p.kind.as_str(), p.key),
        (State::Available, "api", KeyKept::Absent)
    );

    host.handle(start("o1"));
    assert_eq!(
        next_but_reports(&mut rx).await,
        Frame::SeatStatus {
            order: "o1".into(),
            kind: SeatStatusKind::Started,
            detail: String::new()
        }
    );
    let jobs = fake.jobs.lock().unwrap().clone();
    assert_eq!(
        jobs[0].1, "the-chair-ticket",
        "the ticket reaches the bridge"
    );
    let settings = SeatSettings::parse(&jobs[0].2).expect("a settings file the bridge reads");
    assert_eq!(settings.default.as_deref(), Some("sonnet"));
    assert_eq!(reported(&host, "sonnet").state, State::Busy);

    host.handle(start("o2"));
    let Frame::SeatStatus {
        order,
        kind,
        detail,
    } = next_but_reports(&mut rx).await
    else {
        panic!("a status");
    };
    assert_eq!((order.as_str(), kind), ("o2", SeatStatusKind::Failed));
    assert!(detail.contains("busy"), "{detail}");

    host.handle(Frame::StopSeat { order: "o1".into() });
    let Frame::SeatStatus { order, kind, .. } = next_but_reports(&mut rx).await else {
        panic!("a status");
    };
    assert_eq!((order.as_str(), kind), ("o1", SeatStatusKind::Exited));
    assert_eq!(reported(&host, "sonnet").state, State::Available);
}

/// A profile whose own spend book reached its cap is exhausted until the
/// next UTC day, and takes no chair; with its caps switched off it plays.
#[tokio::test]
async fn a_spent_cap_exhausts_the_profile_until_the_next_day() {
    let (host, _fake, _) = host("capped", 0);
    let mut rx = host.attach();
    host.write("sonnet", sonnet(None, Some(json!({"day_usd": 1.0}))))
        .unwrap();
    // A game that spent $2 today, in the profile's own book.
    let book = host.book("sonnet");
    std::fs::create_dir_all(book.path().parent().unwrap()).unwrap();
    let day = Moment {
        unix: now(),
        offset: None,
    }
    .day();
    std::fs::write(
        book.path(),
        json!({"version": 1, "games": [{
            "id": 1, "at": now(), "day": day, "profile": "sonnet", "model": "claude-sonnet-5-5",
            "reserved_usd": 2.0, "spent_usd": 2.0, "settled": now()
        }]})
        .to_string(),
    )
    .unwrap();
    let ledger = book.read().expect("the book reads");
    assert_eq!(ledger.games.len(), 1);
    let p = reported(&host, "sonnet");
    assert_eq!(p.state, State::Exhausted);
    assert_eq!(p.until_unix, Some(state::next_utc_day(now())));
    assert!((p.spent.day_usd - 2.0).abs() < 1e-9);
    host.handle(start("o1"));
    let Frame::SeatStatus { kind, .. } = next_but_reports(&mut rx).await else {
        panic!("a status");
    };
    assert_eq!(kind, SeatStatusKind::Failed);
    // Caps off: the same book no longer stops it.
    host.write("sonnet", sonnet(None, None)).unwrap();
    assert_eq!(reported(&host, "sonnet").state, State::Available);
}

/// A key goes into the store and comes back from nowhere: the answer says
/// done, the report says `kept`, and no frame the gateway is sent holds it.
#[tokio::test]
async fn a_key_is_written_into_the_store_and_never_sent_back() {
    let (host, _fake, keys) = host("key", 0);
    let mut rx = host.attach();
    host.write("sonnet", sonnet(None, None)).unwrap();
    let key = "sk-ant-api03-written-only-written-only-0123";
    host.handle(Frame::KeyWrite {
        request: "k1".into(),
        id: "sonnet".into(),
        key: Some(key.into()),
        admin: "ada".into(),
    });
    let mut sent = Vec::new();
    let answer = loop {
        let frame = next(&mut rx).await;
        sent.push(frame.to_text());
        if let Frame::Answer { .. } = frame {
            break frame;
        }
    };
    assert_eq!(
        answer,
        Frame::Answer {
            request: "k1".into(),
            ok: true,
            error: None
        }
    );
    let entry = KeyEntry::named("ANTHROPIC_API_KEY", "api.anthropic.com").unwrap();
    assert_eq!(keys.get(&entry).unwrap().as_deref(), Some(key));
    assert_eq!(reported(&host, "sonnet").key, KeyKept::Kept);
    sent.push(
        Frame::Profiles {
            profiles: host.report(),
        }
        .to_text(),
    );
    assert!(
        sent.iter().all(|text| !text.contains("written-only")),
        "{sent:?}"
    );
    // Forgotten.
    host.handle(Frame::KeyWrite {
        request: "k2".into(),
        id: "sonnet".into(),
        key: None,
        admin: "ada".into(),
    });
    next_but_reports(&mut rx).await;
    assert_eq!(reported(&host, "sonnet").key, KeyKept::Absent);
}

/// A definition the settings file would refuse is refused, a key-shaped
/// value in it first, and nothing is written.
#[tokio::test]
async fn a_bad_definition_is_refused_and_not_written() {
    let (host, _fake, _) = host("refused", 0);
    let mut keyed = sonnet(None, None);
    keyed.profile =
        json!({"provider": "anthropic", "model": "sk-ant-api03-0123456789abcdefghijkl"});
    assert!(host.write("sonnet", keyed).is_err());
    let mut unlabelled = sonnet(None, None);
    unlabelled.label = String::new();
    assert!(host.write("sonnet", unlabelled).is_err());
    assert!(host.write("a/b", sonnet(None, None)).is_err());
    assert!(host.write("zero", sonnet(Some(0), None)).is_err());
    assert!(!host.config.state_dir.join(PROFILES_FILE).exists());
}

/// A check is run for a new profile, with its canary only when an admin
/// asks; signed out reads `needs_login`, switched off reads `disabled`.
#[tokio::test]
async fn checks_and_switches_show_in_the_state() {
    let (host, fake, _) = host("checks", 1);
    let _rx = host.attach();
    host.write("sonnet", sonnet(None, None)).unwrap();
    *fake.probe.lock().unwrap() = Some(Probe::SignedOut("claude is not signed in".into()));
    host.tick();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(reported(&host, "sonnet").state, State::NeedsLogin);
    *fake.probe.lock().unwrap() = Some(Probe::Ok);
    host.control("sonnet", ControlAction::Probe).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(reported(&host, "sonnet").state, State::Available);
    assert_eq!(
        *fake.probes.lock().unwrap(),
        [("sonnet".to_string(), false), ("sonnet".to_string(), true)],
        "the canary only when asked"
    );
    host.control("sonnet", ControlAction::Disable).unwrap();
    assert_eq!(reported(&host, "sonnet").state, State::Disabled);
    host.control("sonnet", ControlAction::Enable).unwrap();
    assert_eq!(reported(&host, "sonnet").state, State::Available);
    host.drain();
    assert_eq!(reported(&host, "sonnet").state, State::Disabled);
}
