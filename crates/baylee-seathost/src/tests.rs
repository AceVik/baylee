//! The seat agent against a launcher that runs nothing: what it tells the
//! gateway, what it refuses, and where a key goes.

use super::*;
use baylee_client_core::llmseat::keys::{KeyEntry, MemoryKeys};
use launch::Running;
use serde_json::json;

/// A launcher that plays no game: a bridge "sits" at once and runs until
/// it is stopped; a check answers what the test set. It keeps every job it
/// was handed whole, so a test can read what the bridge would have been
/// given and speak on the job's `heard` channel as the bridge's output does.
#[derive(Default)]
struct Fake {
    jobs: Mutex<Vec<SeatJob>>,
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
        self.jobs.lock().unwrap().push(job);
        Box::pin(async move {
            let _ = started.send(());
            let _ = stop.await;
            Exit::Ended(String::new())
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
        fallback: None,
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
    let (ticket, file) = job(&fake, "o1", |j| {
        (j.chair_ticket.clone(), j.settings.clone())
    });
    assert_eq!(ticket, "the-chair-ticket", "the ticket reaches the bridge");
    let settings = SeatSettings::parse(&file).expect("a settings file the bridge reads");
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

#[tokio::test]
async fn managed_session_profiles_reach_hosted_bridges_unchanged() {
    let (host, fake, _) = host("managed-sessions", 0);
    let mut definition = sonnet(None, None);
    definition.profile = json!({"provider":"openai", "model":"local-model",
        "base_url":"http://127.0.0.1:1234/v1", "answer":"json",
        "session_api":true,"game_tokens":5_000_000});
    host.write("sonnet", definition).unwrap();
    let mut rx = host.attach();
    host.handle(start("managed-game"));
    let frame = next_but_reports(&mut rx).await;
    assert!(
        matches!(
            frame,
            Frame::SeatStatus {
                kind: SeatStatusKind::Started,
                ..
            }
        ),
        "{frame:?}"
    );
    let settings = SeatSettings::parse(&fake.jobs.lock().unwrap()[0].settings).unwrap();
    assert_eq!(settings.profiles["sonnet"].session_api, Some(true));
}

// ------------------------------------------------------------------ fallback
//
// A profile may name another profile of the seat agent as its fallback
// (`Definition::fallback`): what the seat agent refuses to keep, where a new
// game of a full or limited profile plays, how that game is counted and
// released, and what its bridge is given.

/// The profile that names a fallback in the tests below, and the one it names.
const PRIMARY: &str = "primary";
const BACKUP: &str = "backup";

/// A profile behind an OpenAI-compatible endpoint, the vendor `Example`:
/// two of them may be a profile and its fallback.
fn example(label: &str, model: &str, max_games: Option<u32>, fallback: Option<&str>) -> Definition {
    Definition {
        label: label.into(),
        vendor: "Example".into(),
        enabled: true,
        max_games,
        caps: None,
        canary: false,
        fallback: fallback.map(Into::into),
        profile: json!({"provider": "openai", "model": model,
            "base_url": "https://api.example.invalid/v1"}),
    }
}

/// A profile that plays an agent CLI, which takes no fallback and is none.
fn cli_definition() -> Definition {
    Definition {
        label: "Agent CLI".into(),
        vendor: "Example".into(),
        enabled: true,
        max_games: None,
        caps: None,
        canary: false,
        fallback: None,
        profile: json!({"provider": "cli", "model": "claude"}),
    }
}

/// A seat agent with no profile yet, and the channel of what it tells the
/// gateway.
fn bare(name: &str) -> (Arc<Host>, Arc<Fake>, mpsc::UnboundedReceiver<Frame>) {
    let (host, fake, _) = host(name, 0);
    let rx = host.attach();
    (host, fake, rx)
}

/// A seat agent holding [`PRIMARY`] (naming [`BACKUP`] as its fallback) and [`BACKUP`],
/// each with its own bound on games.
fn pair(
    name: &str,
    primary_max: Option<u32>,
    fallback_max: Option<u32>,
) -> (Arc<Host>, Arc<Fake>, mpsc::UnboundedReceiver<Frame>) {
    let (host, fake, rx) = bare(name);
    host.write(
        PRIMARY,
        example("Model A", "model-a", primary_max, Some(BACKUP)),
    )
    .unwrap();
    host.write(BACKUP, example("Model B", "model-b", fallback_max, None))
        .unwrap();
    (host, fake, rx)
}

/// An order for a chair of `profile`.
fn start_on(order: &str, profile: &str) -> Frame {
    Frame::StartSeat {
        order: order.into(),
        game_id: "g1".into(),
        seat: 1,
        profile: profile.into(),
        chair_ticket: "the-chair-ticket".into(),
        gateway_url: "http://127.0.0.1:2".into(),
        deck_text: None,
    }
}

/// The next status frame: the order, what became of it, and why.
async fn status(rx: &mut mpsc::UnboundedReceiver<Frame>) -> (String, SeatStatusKind, String) {
    let Frame::SeatStatus {
        order,
        kind,
        detail,
    } = next_but_reports(rx).await
    else {
        panic!("a status");
    };
    (order, kind, detail)
}

/// Orders `order` for `profile` and waits until its bridge sits.
async fn seat(
    host: &Arc<Host>,
    rx: &mut mpsc::UnboundedReceiver<Frame>,
    order: &str,
    profile: &str,
) {
    host.handle(start_on(order, profile));
    assert_eq!(
        status(rx).await,
        (order.to_string(), SeatStatusKind::Started, String::new())
    );
}

/// Orders `order` for `profile` and expects the refusal that names the
/// profile's `state` (`busy`, `disabled`, `failing`).
async fn refused_busy(
    host: &Arc<Host>,
    rx: &mut mpsc::UnboundedReceiver<Frame>,
    order: &str,
    profile: &str,
    state: &str,
) {
    host.handle(start_on(order, profile));
    let (got, kind, detail) = status(rx).await;
    assert_eq!((got.as_str(), kind), (order, SeatStatusKind::Failed));
    assert_eq!(detail, format!("busy: the profile is {state}"));
}

/// Reads the job the launcher was handed for `order`.
fn job<T>(fake: &Fake, order: &str, read: impl FnOnce(&SeatJob) -> T) -> T {
    let jobs = fake.jobs.lock().unwrap();
    read(
        jobs.iter()
            .find(|j| j.order == order)
            .unwrap_or_else(|| panic!("the launcher was handed no job for {order}")),
    )
}

/// Says `heard` on the channel the job of `order` was given, as its bridge's
/// output does.
fn hear(fake: &Fake, order: &str, heard: Heard) {
    job(fake, order, |j| {
        j.heard
            .as_ref()
            .expect("a game with a fallback is listened to")
            .send(heard)
            .expect("the seat agent listens");
    });
}

/// Waits until `done`, which looks at the seat agent's state, holds.
async fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..500 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{what} did not happen in time");
}

/// The seconds `why` (a placement's reason) names after `lead`.
fn seconds_after(why: &str, lead: &str) -> i64 {
    why.strip_prefix(lead)
        .and_then(|rest| rest.strip_suffix(" s"))
        .and_then(|secs| secs.parse().ok())
        .unwrap_or_else(|| panic!("«{why}» does not say «{lead}» N s"))
}

/// A definition naming itself as its fallback, a fallback that is no
/// profile's id, one named twice, and fallbacks written into the profile's
/// own settings are each refused, with the sentence that says which; a valid
/// pair, a chain, and a CLI with or without fallbacks are kept. (Before,
/// a CLI took no fallback.)
#[test]
fn a_definition_whose_fallback_is_itself_malformed_or_named_twice_is_refused() {
    let own = definition_fault(PRIMARY, &example("Model A", "model-a", None, Some(PRIMARY)));
    assert_eq!(own.as_deref(), Some("a profile cannot be its own fallback"));
    for bad in ["a/b", "", "..", &"x".repeat(65)] {
        let fault = definition_fault(PRIMARY, &example("Model A", "model-a", None, Some(bad)))
            .unwrap_or_else(|| panic!("«{bad}» is no fallback"));
        assert!(fault.contains("a fallback is a profile's id"), "{fault}");
    }
    let mut cli = cli_definition();
    cli.fallback = Some(BACKUP.into());
    assert_eq!(
        definition_fault("agent-cli", &cli),
        None,
        "a subscription that runs out hands over too"
    );
    cli.fallback = None;
    assert_eq!(definition_fault("agent-cli", &cli), None);
    let pair = example("Model A", "model-a", Some(2), Some(BACKUP));
    assert_eq!(definition_fault(PRIMARY, &pair), None);
    let mut chain = pair.clone();
    chain.fallback = Some(vec![BACKUP.to_owned(), "spare".to_owned()].into());
    assert_eq!(definition_fault(PRIMARY, &chain), None);
    chain.fallback = Some(vec![BACKUP.to_owned(), BACKUP.to_owned()].into());
    let twice = definition_fault(PRIMARY, &chain).expect("named twice");
    assert!(twice.contains("named twice"), "{twice}");
    let mut inside = pair;
    inside.profile["fallback"] = serde_json::json!(BACKUP);
    let inside = definition_fault(PRIMARY, &inside).expect("in its settings");
    assert!(inside.contains("in the definition's fallback"), "{inside}");
}

/// A set of definitions in which a profile's fallback plays an agent CLI, or
/// names another vendor, is refused as a set; a fallback no definition has
/// yet, a fallback of the same vendor in other letters and blanks, and a pair
/// that name each other are not faults.
#[test]
fn a_set_of_definitions_is_refused_when_a_fallback_plays_a_cli_or_names_another_vendor() {
    let primary = example("Model A", "model-a", None, Some(BACKUP));
    let set = |theirs: Definition| {
        BTreeMap::from([(PRIMARY.into(), primary.clone()), (BACKUP.into(), theirs)])
    };

    let fault = definitions_fault(&set(cli_definition())).expect("a CLI is no fallback");
    assert!(fault.contains("plays an agent CLI"), "{fault}");
    assert!(fault.contains(PRIMARY) && fault.contains(BACKUP), "{fault}");

    let mut other = example("Model B", "model-b", None, None);
    other.vendor = "Other".into();
    let fault = definitions_fault(&set(other)).expect("another vendor is no fallback");
    assert!(fault.contains("their vendors differ"), "{fault}");
    assert!(
        fault.contains("Example") && fault.contains("Other"),
        "{fault}"
    );

    let mut spelled = example("Model B", "model-b", None, None);
    spelled.vendor = "  example ".into();
    assert_eq!(
        definitions_fault(&set(spelled)),
        None,
        "case and outer blanks aside"
    );

    let alone = BTreeMap::from([(PRIMARY.into(), primary.clone())]);
    assert_eq!(definitions_fault(&alone), None, "it may be written next");

    let mut back = example("Model B", "model-b", None, Some(PRIMARY));
    back.vendor = "Example".into();
    assert_eq!(
        definitions_fault(&set(back)),
        None,
        "a fallback's own is not followed"
    );
}

/// Through the console's `ProfileWrite`: a profile may name a fallback that is
/// not there yet, and the fallback written afterwards is refused when it
/// plays a CLI or names another vendor, with nothing written and the profile
/// not reported; written as an API of the same vendor it is kept.
#[tokio::test]
async fn a_fallback_written_after_its_profile_is_refused_when_it_is_a_cli_or_another_vendor() {
    let (host, _fake, _) = host("late-fallback", 0);
    let mut rx = host.attach();
    let write = |request: &str, id: &str, definition: Definition| Frame::ProfileWrite {
        request: request.into(),
        id: id.into(),
        definition,
        admin: "ada".into(),
    };
    let answer = |request: &str, ok: bool| Frame::Answer {
        request: request.into(),
        ok,
        error: None,
    };

    host.handle(write(
        "r1",
        PRIMARY,
        example("Model A", "model-a", None, Some(BACKUP)),
    ));
    assert_eq!(next_but_reports(&mut rx).await, answer("r1", true));
    assert_eq!(reported(&host, PRIMARY).definition.fallbacks(), [BACKUP]);

    host.handle(write("r2", BACKUP, cli_definition()));
    let Frame::Answer { request, ok, error } = next_but_reports(&mut rx).await else {
        panic!("an answer");
    };
    assert_eq!((request.as_str(), ok), ("r2", false));
    let error = error.expect("a sentence");
    assert!(error.contains("plays an agent CLI"), "{error}");

    let mut other = example("Model B", "model-b", None, None);
    other.vendor = "Other".into();
    host.handle(write("r3", BACKUP, other));
    let Frame::Answer { ok, error, .. } = next_but_reports(&mut rx).await else {
        panic!("an answer");
    };
    assert!(!ok);
    assert!(error.expect("a sentence").contains("their vendors differ"));

    assert!(
        host.report().iter().all(|p| p.id != BACKUP),
        "nothing of it was kept"
    );
    let kept = read_profiles(&host.config.state_dir.join(PROFILES_FILE)).unwrap();
    assert_eq!(kept.keys().collect::<Vec<_>>(), [PRIMARY]);

    host.handle(write(
        "r4",
        BACKUP,
        example("Model B", "model-b", None, None),
    ));
    assert_eq!(next_but_reports(&mut rx).await, answer("r4", true));
    assert_eq!(reported(&host, BACKUP).kind, "api");
}

/// A profiles file that holds a profile whose fallback plays a CLI (written
/// by hand) is not read, by `read_profiles` and so not by `Host::new`: the
/// seat agent does not start on a pairing it would refuse to be told.
#[test]
fn a_profiles_file_whose_fallback_plays_a_cli_is_not_read() {
    let dir = scratch("bad-file");
    let file = dir.join(PROFILES_FILE);
    let defs = BTreeMap::from([
        (
            PRIMARY.to_string(),
            example("Model A", "model-a", None, Some("agent-cli")),
        ),
        ("agent-cli".to_string(), cli_definition()),
    ]);
    write_profiles(&file, &defs).unwrap();
    let why = read_profiles(&file).expect_err("a CLI is no fallback");
    assert!(why.contains("plays an agent CLI"), "{why}");

    let (fake, keys) = (Arc::new(Fake::default()), Arc::new(MemoryKeys::default()));
    let started = Host::new(Config {
        name: "test".into(),
        gateway: "http://127.0.0.1:1".into(),
        token: "t".into(),
        capacity: 0,
        state_dir: dir,
        bridge_gateway: None,
        keys,
        launcher: Arc::new(fake),
    });
    assert!(started.is_err(), "the seat agent does not start on it");

    let own = BTreeMap::from([(
        PRIMARY.to_string(),
        example("Model A", "model-a", None, Some(PRIMARY)),
    )]);
    let dir = scratch("own-file");
    write_profiles(&dir.join(PROFILES_FILE), &own).unwrap();
    let why = read_profiles(&dir.join(PROFILES_FILE)).expect_err("its own fallback");
    assert!(why.contains("its own fallback"), "{why}");
}

/// Why a fallback cannot be taken along, one reason each: no profile of that
/// name here, itself, an agent CLI, another vendor, switched off; and the
/// profile that can be taken along is named.
#[test]
fn a_fallback_is_taken_along_only_when_it_is_a_usable_profile_of_the_seat_agent() {
    // The first fallback `PRIMARY` names, and whether a game may take it.
    fn fallback_of(
        defs: &BTreeMap<String, Definition>,
        id: &str,
    ) -> Option<Result<String, &'static str>> {
        fallbacks_of(defs, id)
            .into_iter()
            .next()
            .map(|(name, usable)| usable.map(|()| name.to_owned()))
    }
    let with = |theirs: Option<Definition>| {
        let mut defs = BTreeMap::from([(
            PRIMARY.to_string(),
            example("Model A", "model-a", None, Some(BACKUP)),
        )]);
        defs.extend(theirs.map(|d| (BACKUP.to_string(), d)));
        fallback_of(&defs, PRIMARY)
    };
    let backup = || example("Model B", "model-b", None, None);

    assert_eq!(with(Some(backup())), Some(Ok(BACKUP.to_string())));
    assert_eq!(with(None), Some(Err("it is not on this seat agent")));
    assert_eq!(
        with(Some(cli_definition())),
        Some(Err("a fallback is a model behind an API"))
    );
    let mut other = backup();
    other.vendor = "Other".into();
    assert_eq!(with(Some(other)), Some(Err("its vendor is another")));
    let mut off = backup();
    off.enabled = false;
    assert_eq!(with(Some(off)), Some(Err("it is switched off")));

    let own = BTreeMap::from([(
        PRIMARY.to_string(),
        example("Model A", "model-a", None, Some(PRIMARY)),
    )]);
    assert_eq!(
        fallback_of(&own, PRIMARY),
        Some(Err("it is the profile itself"))
    );
    let none = BTreeMap::from([(
        PRIMARY.to_string(),
        example("Model A", "model-a", None, None),
    )]);
    assert_eq!(fallback_of(&none, PRIMARY), None, "no fallback named");
}

/// A game of a profile that is available plays the profile, with the
/// fallback standing by: its job holds the profile's settings file and spend
/// book, the seat agent's reports directory, the fallback's own settings
/// file and spend book, the chair's name
/// after the profile's label, and a channel for what the bridge says of its
/// model. (Before, a job had no fallback and no channel.)
#[tokio::test]
async fn a_game_of_an_available_profile_takes_its_fallback_along() {
    let (host, fake, mut rx) = pair("stands-by", Some(2), None);
    let spend = host.config.state_dir.join("spend");
    assert_eq!(
        host.place(PRIMARY),
        Ok(Placement {
            plays: PRIMARY.into(),
            fallbacks: vec![BACKUP.into()],
            diverted: None
        })
    );

    seat(&host, &mut rx, "o1", PRIMARY).await;

    job(&fake, "o1", |j| {
        assert_eq!((j.profile.as_str(), j.plays.as_str()), (PRIMARY, PRIMARY));
        assert_eq!(j.name, "Model-A");
        assert_eq!(j.chair_ticket, "the-chair-ticket");
        assert_eq!(j.ledger, spend.join("primary.json"));
        assert_eq!(j.reports, host.config.state_dir.join(REPORTS_DIR));
        let mine = SeatSettings::parse(&j.settings).unwrap();
        assert_eq!(mine.default.as_deref(), Some(PRIMARY));
        assert_eq!(mine.profiles.keys().collect::<Vec<_>>(), [PRIMARY]);
        let [standing] = j.fallbacks.as_slice() else {
            panic!("the fallback stands by, alone");
        };
        assert_eq!(standing.profile, BACKUP);
        assert_eq!(standing.ledger, spend.join("backup.json"));
        let theirs = SeatSettings::parse(&standing.settings).unwrap();
        assert_eq!(theirs.default.as_deref(), Some(BACKUP));
        assert_eq!(theirs.profiles.keys().collect::<Vec<_>>(), [BACKUP]);
        assert!(
            j.heard.is_some(),
            "what the bridge says of its model is heard"
        );
    });
    let p = reported(&host, PRIMARY);
    assert_eq!((p.state, p.diverts, p.games), (State::Available, None, 1));
}

/// A new game of a profile at its `max_games` starts on its fallback: it
/// plays the fallback's settings and spend book under the chair name of the
/// profile ordered, without a fallback of its own and with nobody listening
/// for a limit. The placement says why. (Before, the order was refused as
/// busy.)
#[tokio::test]
async fn a_new_game_of_a_profile_at_its_bound_starts_on_its_fallback() {
    let (host, fake, mut rx) = pair("divert-full", Some(1), None);
    let spend = host.config.state_dir.join("spend");
    seat(&host, &mut rx, "o1", PRIMARY).await;
    assert_eq!(
        host.place(PRIMARY),
        Ok(Placement {
            plays: BACKUP.into(),
            fallbacks: Vec::new(),
            diverted: Some("primary at max_games 1/1".into())
        })
    );

    seat(&host, &mut rx, "o2", PRIMARY).await;

    job(&fake, "o2", |j| {
        assert_eq!((j.profile.as_str(), j.plays.as_str()), (PRIMARY, BACKUP));
        assert_eq!(
            j.name, "Model-A",
            "the chair keeps the ordered profile's name"
        );
        assert_eq!(j.chair_ticket, "the-chair-ticket");
        assert_eq!(j.ledger, spend.join("backup.json"));
        let settings = SeatSettings::parse(&j.settings).unwrap();
        assert_eq!(settings.default.as_deref(), Some(BACKUP));
        assert_eq!(settings.profiles.keys().collect::<Vec<_>>(), [BACKUP]);
        assert!(j.fallbacks.is_empty(), "the fallback has none of its own");
        assert!(j.heard.is_none(), "nobody listens where nothing stands by");
    });
}

/// A profile that diverts is reported available with no end in sight, its
/// own games and bound unchanged, naming the profile its new games start on;
/// the fallback is reported as ever. (Before it was reported busy.)
#[tokio::test]
async fn a_profile_that_diverts_is_reported_available_with_its_own_games_and_bound() {
    let (host, _fake, mut rx) = pair("report-divert", Some(1), Some(3));
    let p = reported(&host, PRIMARY);
    assert_eq!((p.state, p.diverts, p.games), (State::Available, None, 0));

    seat(&host, &mut rx, "o1", PRIMARY).await;

    let p = reported(&host, PRIMARY);
    assert_eq!(
        p.state,
        State::Available,
        "offered: a new game starts on the fallback"
    );
    assert_eq!(p.until_unix, None);
    assert_eq!(p.diverts.as_deref(), Some(BACKUP));
    assert_eq!((p.games, p.max_games), (1, Some(1)), "its own numbers");
    let f = reported(&host, BACKUP);
    assert_eq!((f.state, f.diverts, f.games), (State::Available, None, 0));
}

/// A model its bridge heard rate limited (`Heard::Limited`) holds the profile
/// back from new games until the limit lifts, though it has room: the next
/// game starts on the fallback, and the profile is reported available while
/// its running game stays counted to it. (Before nothing was heard.)
#[tokio::test]
async fn a_limit_the_bridge_heard_sends_the_next_game_to_the_fallback() {
    let (host, fake, mut rx) = pair("divert-heard", None, None);
    seat(&host, &mut rx, "o1", PRIMARY).await;
    assert_eq!(
        host.place(PRIMARY).unwrap().plays,
        PRIMARY,
        "room, and not limited"
    );

    hear(
        &fake,
        "o1",
        Heard::Limited {
            of: None,
            lifts_secs: Some(600),
        },
    );
    eventually("the profile is held back", || {
        reported(&host, PRIMARY).diverts.is_some()
    })
    .await;

    let placement = host.place(PRIMARY).unwrap();
    assert_eq!(
        (placement.plays.as_str(), placement.fallbacks.is_empty()),
        (BACKUP, true)
    );
    let lifts = seconds_after(
        &placement.diverted.expect("the reason"),
        "primary limited for another ",
    );
    assert!((590..=600).contains(&lifts), "{lifts}");
    let p = reported(&host, PRIMARY);
    assert_eq!((p.state, p.until_unix), (State::Available, None));
    assert_eq!((p.games, p.max_games), (1, None));

    seat(&host, &mut rx, "o2", PRIMARY).await;
    job(&fake, "o2", |j| assert_eq!(j.plays, BACKUP));
    assert_eq!(reported(&host, PRIMARY).games, 1, "o1 is still its game");
    assert_eq!(reported(&host, BACKUP).games, 1);
}

/// A limit a check found (the provider said so) sends the next game to the
/// fallback as well, and a spend cap its own book reached does the same, each
/// with its own reason and the time it lasts. (Before: refused, exhausted.)
#[tokio::test]
async fn a_limit_a_check_found_or_a_spent_cap_sends_the_next_game_to_the_fallback() {
    let (host, fake, mut rx) = pair("divert-check", None, None);
    *fake.probe.lock().unwrap() = Some(Probe::Limited {
        why: "rate limit".into(),
        lifts_secs: Some(1200),
    });
    host.control(PRIMARY, ControlAction::Probe).unwrap();
    // While the check runs the profile diverts too (being checked): wait
    // for what it found.
    eventually("the check is read", || {
        host.place(PRIMARY).is_ok_and(|p| {
            p.diverted
                .as_deref()
                .is_some_and(|why| why.starts_with("primary limited"))
        })
    })
    .await;
    assert_eq!(
        *fake.probes.lock().unwrap(),
        [(PRIMARY.to_string(), false)],
        "only the profile was checked"
    );
    let reason = host.place(PRIMARY).unwrap().diverted.expect("the reason");
    assert!((1190..=1200).contains(&seconds_after(&reason, "primary limited for another ")));
    seat(&host, &mut rx, "o1", PRIMARY).await;
    job(&fake, "o1", |j| assert_eq!(j.plays, BACKUP));

    let (host, fake, mut rx) = pair("divert-cap", None, None);
    let mut capped = example("Model A", "model-a", None, Some(BACKUP));
    capped.caps = Some(json!({"day_usd": 1.0}));
    host.write(PRIMARY, capped).unwrap();
    let book = host.book(PRIMARY);
    std::fs::create_dir_all(book.path().parent().unwrap()).unwrap();
    let day = Moment {
        unix: now(),
        offset: None,
    }
    .day();
    std::fs::write(
        book.path(),
        json!({"version": 1, "games": [{
            "id": 1, "at": now(), "day": day, "profile": PRIMARY, "model": "model-a",
            "reserved_usd": 2.0, "spent_usd": 2.0, "settled": now()
        }]})
        .to_string(),
    )
    .unwrap();
    let placement = host.place(PRIMARY).unwrap();
    assert_eq!(placement.plays, BACKUP);
    let lasts = seconds_after(
        &placement.diverted.expect("the reason"),
        "primary's spend cap reached for another ",
    );
    assert!(
        (1..=86_400).contains(&lasts),
        "until the next UTC day: {lasts}"
    );
    let p = reported(&host, PRIMARY);
    assert_eq!((p.state, p.until_unix), (State::Available, None));
    assert!(
        (p.spent.day_usd - 2.0).abs() < 1e-9,
        "its own book, still shown"
    );
    seat(&host, &mut rx, "o1", PRIMARY).await;
    job(&fake, "o1", |j| {
        assert_eq!(j.plays, BACKUP);
        assert_eq!(
            SeatSettings::parse(&j.settings).unwrap().caps,
            Caps::default()
        );
    });
}

/// Fallback or not, a refused game is refused as before: when the fallback
/// is at its own bound the order is `busy` and the profile is reported busy;
/// the same when it is limited, switched off, or no profile of this seat agent.
#[tokio::test]
async fn a_full_profile_is_refused_as_busy_when_its_fallback_cannot_take_the_game() {
    // The fallback at its own bound.
    let (host, _fake, mut rx) = pair("both-full", Some(1), Some(1));
    seat(&host, &mut rx, "o1", PRIMARY).await;
    seat(&host, &mut rx, "o2", PRIMARY).await;
    refused_busy(&host, &mut rx, "o3", PRIMARY, "busy").await;
    let p = reported(&host, PRIMARY);
    assert_eq!((p.state, p.diverts), (State::Busy, None));
    assert_eq!(host.place(PRIMARY), Err("busy: the profile is busy".into()));
    assert_eq!(host.live(), 2, "the refused order took no game");

    // The fallback limited.
    let (host, fake, mut rx) = pair("fallback-limited", Some(1), None);
    seat(&host, &mut rx, "o1", PRIMARY).await;
    *fake.probe.lock().unwrap() = Some(Probe::Limited {
        why: "quota".into(),
        lifts_secs: Some(900),
    });
    host.control(BACKUP, ControlAction::Probe).unwrap();
    eventually("the fallback is limited", || {
        reported(&host, BACKUP).state == State::Exhausted
    })
    .await;
    refused_busy(&host, &mut rx, "o2", PRIMARY, "busy").await;
    assert_eq!(reported(&host, PRIMARY).state, State::Busy);

    // The fallback switched off.
    let (host, _fake, mut rx) = pair("fallback-off", Some(1), None);
    seat(&host, &mut rx, "o1", PRIMARY).await;
    host.control(BACKUP, ControlAction::Disable).unwrap();
    refused_busy(&host, &mut rx, "o2", PRIMARY, "busy").await;
    assert_eq!(reported(&host, PRIMARY).diverts, None);

    // A fallback this seat agent does not hold: kept (it may be written
    // next), no fallback until it is.
    let (host, fake, mut rx) = bare("fallback-missing");
    host.write(
        PRIMARY,
        example("Model A", "model-a", Some(1), Some("nothing")),
    )
    .unwrap();
    assert_eq!(
        host.place(PRIMARY).unwrap().fallbacks,
        Vec::<String>::new(),
        "nothing stands by"
    );
    seat(&host, &mut rx, "o1", PRIMARY).await;
    job(&fake, "o1", |j| {
        assert!(j.fallbacks.is_empty() && j.heard.is_none());
    });
    refused_busy(&host, &mut rx, "o2", PRIMARY, "busy").await;
}

/// Places a new game of [`PRIMARY`], expecting it to start on [`BACKUP`]
/// for `why`; seats it as `order` and checks the bridge plays the fallback
/// alone, counted on the fallback and not on the profile.
async fn diverted_for(
    host: &Arc<Host>,
    fake: &Fake,
    rx: &mut mpsc::UnboundedReceiver<Frame>,
    order: &str,
    why: &str,
) {
    let placed = host.place(PRIMARY).expect("the fallback takes the game");
    assert_eq!(
        (placed.plays.as_str(), placed.diverted.as_deref()),
        (BACKUP, Some(why))
    );
    let p = reported(host, PRIMARY);
    assert_eq!(
        (p.state, p.diverts.as_deref()),
        (State::Available, Some(BACKUP))
    );
    seat(host, rx, order, PRIMARY).await;
    job(fake, order, |j| {
        assert_eq!(j.plays, BACKUP);
        assert!(j.fallbacks.is_empty(), "no further fallback");
    });
    assert_eq!(
        (reported(host, PRIMARY).games, reported(host, BACKUP).games),
        (0, 1)
    );
}

/// A profile an admin switched off hands its new games to its fallback
/// (owner, 13.10.2026: it is to run steadily), and is still reported off.
/// Before, the order was refused as `busy: the profile is disabled`.
#[tokio::test]
async fn a_switched_off_profile_hands_new_games_to_its_fallback() {
    let (host, fake, mut rx) = pair("divert-off", None, None);
    host.control(PRIMARY, ControlAction::Disable).unwrap();
    diverted_for(&host, &fake, &mut rx, "o1", "primary switched off").await;
    assert!(!reported(&host, PRIMARY).enabled);
}

/// A profile whose check failed hands its new games to its fallback, the
/// log's reason saying what failed. Before, the order was refused as
/// `busy: the profile is failing`.
#[tokio::test]
async fn a_failing_profile_hands_new_games_to_its_fallback() {
    let (host, fake, mut rx) = pair("divert-failing", None, None);
    *fake.probe.lock().unwrap() = Some(Probe::Failing("no key".into()));
    host.control(PRIMARY, ControlAction::Probe).unwrap();
    // Its own state, before the fallback is looked at: what `place` reads.
    eventually("the check failed", || {
        host.place(PRIMARY)
            .is_ok_and(|p| p.diverted.as_deref() != Some("primary being checked"))
    })
    .await;
    diverted_for(&host, &fake, &mut rx, "o1", "primary failing: no key").await;
}

/// A profile whose CLI-style check says it is signed out hands its new
/// games to its fallback. Before, the order was refused as `busy: the
/// profile is needs_login`.
#[tokio::test]
async fn a_signed_out_profile_hands_new_games_to_its_fallback() {
    let (host, fake, mut rx) = pair("divert-login", None, None);
    *fake.probe.lock().unwrap() = Some(Probe::SignedOut("not signed in".into()));
    host.control(PRIMARY, ControlAction::Probe).unwrap();
    eventually("the check said signed out", || {
        host.place(PRIMARY)
            .is_ok_and(|p| p.diverted.as_deref() != Some("primary being checked"))
    })
    .await;
    diverted_for(
        &host,
        &fake,
        &mut rx,
        "o1",
        "primary needs login: not signed in",
    )
    .await;
}

/// A profile being checked hands a new game to its fallback rather than
/// keep the chair waiting. Before, the order was refused as `busy: the
/// profile is probing`.
#[tokio::test]
async fn a_profile_being_checked_hands_new_games_to_its_fallback() {
    let (host, fake, mut rx) = pair("divert-probing", None, None);
    // The check runs on a task of its own: until that task runs, the
    // profile is being checked.
    host.control(PRIMARY, ControlAction::Probe).unwrap();
    let placed = host.place(PRIMARY).expect("the fallback takes the game");
    assert_eq!(
        (placed.plays.as_str(), placed.diverted.as_deref()),
        (BACKUP, Some("primary being checked"))
    );
    host.handle(start_on("o1", PRIMARY));
    assert_eq!(
        status(&mut rx).await,
        ("o1".to_string(), SeatStatusKind::Started, String::new())
    );
    job(&fake, "o1", |j| assert_eq!(j.plays, BACKUP));
}

/// Where the fallback cannot take the game either, a profile that is not
/// available is refused as its state says, as it always was.
#[tokio::test]
async fn a_profile_that_is_off_with_its_fallback_off_too_is_refused_as_ever() {
    let (host, fake, mut rx) = pair("divert-both-off", None, None);
    host.control(PRIMARY, ControlAction::Disable).unwrap();
    host.control(BACKUP, ControlAction::Disable).unwrap();
    refused_busy(&host, &mut rx, "o1", PRIMARY, "disabled").await;
    assert_eq!(reported(&host, PRIMARY).diverts, None);
    assert!(fake.jobs.lock().unwrap().is_empty(), "no bridge started");
}

/// A diverted game counts as the fallback's and not the profile's, and is
/// released from the fallback when its bridge ends: with both orders stopped
/// both profiles are back to no game and the profile no longer diverts; with
/// only the first stopped, the profile has room again and plays itself.
#[tokio::test]
async fn a_diverted_game_is_counted_to_the_fallback_and_released_from_it() {
    let (host, _fake, mut rx) = pair("divert-count", Some(1), Some(2));
    seat(&host, &mut rx, "o1", PRIMARY).await;
    seat(&host, &mut rx, "o2", PRIMARY).await;

    let (p, f) = (reported(&host, PRIMARY), reported(&host, BACKUP));
    assert_eq!(
        (p.games, f.games),
        (1, 1),
        "the diverted game is the fallback's"
    );
    assert_eq!(
        (p.state, p.until_unix, p.diverts.as_deref()),
        (State::Available, None, Some(BACKUP))
    );
    assert_eq!((f.state, f.diverts), (State::Available, None));
    assert_eq!(host.live(), 2);

    // The game that played the profile ends: the profile has room again.
    host.handle(Frame::StopSeat { order: "o1".into() });
    assert_eq!(
        status(&mut rx).await,
        ("o1".to_string(), SeatStatusKind::Exited, String::new())
    );
    let (p, f) = (reported(&host, PRIMARY), reported(&host, BACKUP));
    assert_eq!(
        (p.games, f.games),
        (0, 1),
        "the fallback's game is still its"
    );
    assert_eq!((p.state, p.diverts), (State::Available, None));
    let placement = host.place(PRIMARY).unwrap();
    assert_eq!(
        (placement.plays.as_str(), placement.diverted),
        (PRIMARY, None)
    );

    // The diverted one ends: nothing is left counted.
    host.handle(Frame::StopSeat { order: "o2".into() });
    assert_eq!(
        status(&mut rx).await,
        ("o2".to_string(), SeatStatusKind::Exited, String::new())
    );
    let (p, f) = (reported(&host, PRIMARY), reported(&host, BACKUP));
    assert_eq!((p.games, f.games), (0, 0));
    assert_eq!((p.diverts, f.diverts), (None, None));
    assert_eq!(host.live(), 0);
}

/// The other way round: stopping the diverted game first releases the
/// fallback and leaves the profile's own game counted, and the profile keeps
/// diverting while it is full.
#[tokio::test]
async fn stopping_the_diverted_game_first_leaves_the_profiles_own_game_counted() {
    let (host, _fake, mut rx) = pair("divert-release", Some(1), None);
    seat(&host, &mut rx, "o1", PRIMARY).await;
    seat(&host, &mut rx, "o2", PRIMARY).await;

    host.handle(Frame::StopSeat { order: "o2".into() });
    assert_eq!(
        status(&mut rx).await,
        ("o2".to_string(), SeatStatusKind::Exited, String::new())
    );
    let (p, f) = (reported(&host, PRIMARY), reported(&host, BACKUP));
    assert_eq!((p.games, f.games), (1, 0));
    assert_eq!(
        p.diverts.as_deref(),
        Some(BACKUP),
        "still full, so it still diverts"
    );
}

/// A fallback's own fallback is not followed: where profiles name each other
/// the game that plays the fallback has none standing by.
#[tokio::test]
async fn a_fallbacks_own_fallback_is_not_followed() {
    let (host, fake, mut rx) = pair("not-followed", Some(1), None);
    host.write(BACKUP, example("Model B", "model-b", None, Some(PRIMARY)))
        .unwrap();
    seat(&host, &mut rx, "o1", PRIMARY).await;
    seat(&host, &mut rx, "o2", PRIMARY).await;
    job(&fake, "o1", |j| assert_eq!(j.fallbacks.len(), 1));
    job(&fake, "o2", |j| {
        assert_eq!(j.plays, BACKUP);
        assert!(j.fallbacks.is_empty() && j.heard.is_none());
    });
    // Ordered directly, the fallback profile has its own fallback stand by.
    seat(&host, &mut rx, "o3", BACKUP).await;
    job(&fake, "o3", |j| {
        assert_eq!(
            j.fallbacks.first().map(|f| f.profile.as_str()),
            Some(PRIMARY)
        );
    });
}

/// Without a fallback nothing changes: a profile at its bound refuses as
/// busy, a game carries no fallback and nobody listens for a limit, and the
/// report names no diversion. (Before there was nothing to compare.)
#[tokio::test]
async fn a_profile_without_a_fallback_refuses_at_its_bound_as_ever() {
    let (host, fake, mut rx) = bare("no-fallback");
    host.write(BACKUP, example("Model B", "model-b", Some(1), None))
        .unwrap();
    host.write(PRIMARY, example("Model A", "model-a", Some(1), None))
        .unwrap();
    assert_eq!(
        host.place(PRIMARY),
        Ok(Placement {
            plays: PRIMARY.into(),
            fallbacks: Vec::new(),
            diverted: None
        })
    );
    seat(&host, &mut rx, "o1", PRIMARY).await;
    job(&fake, "o1", |j| {
        assert_eq!(j.plays, PRIMARY);
        assert!(j.fallbacks.is_empty(), "no fallback in the job");
        assert!(j.heard.is_none(), "nobody listens");
    });
    refused_busy(&host, &mut rx, "o2", PRIMARY, "busy").await;
    let p = reported(&host, PRIMARY);
    assert_eq!((p.state, p.diverts), (State::Busy, None));
    assert_eq!(
        reported(&host, BACKUP).games,
        0,
        "the other profile played nothing"
    );
    assert_eq!(
        host.place("nothing"),
        Err("this seat agent has no such profile".into())
    );
}

/// A draining seat agent places nothing, not on a fallback either: the order
/// is refused with the word that says why, whatever the profiles' states.
#[tokio::test]
async fn a_draining_seat_agent_places_no_game_even_on_a_fallback() {
    let (host, fake, mut rx) = pair("draining", Some(1), None);
    seat(&host, &mut rx, "o1", PRIMARY).await;
    assert_eq!(
        host.place(PRIMARY).unwrap().plays,
        BACKUP,
        "diverting until it drains"
    );

    host.drain();

    assert_eq!(host.place(PRIMARY), Err("draining".into()));
    host.handle(start_on("o2", PRIMARY));
    let (order, kind, detail) = status(&mut rx).await;
    assert_eq!(
        (order.as_str(), kind, detail.as_str()),
        ("o2", SeatStatusKind::Failed, "draining")
    );
    assert_eq!(fake.jobs.lock().unwrap().len(), 1, "no second bridge");
    assert_eq!(reported(&host, PRIMARY).diverts, None);
}

/// The second fallback of the chains below.
const SPARE: &str = "spare";

/// A seat agent holding [`PRIMARY`] (naming [`BACKUP`], then [`SPARE`], as
/// its fallbacks), [`BACKUP`] and [`SPARE`], each with its own bound.
fn chain(
    name: &str,
    maxes: [Option<u32>; 3],
) -> (Arc<Host>, Arc<Fake>, mpsc::UnboundedReceiver<Frame>) {
    let (host, fake, rx) = bare(name);
    let mut primary = example("Model A", "model-a", maxes[0], None);
    primary.fallback = Some(vec![BACKUP.to_owned(), SPARE.to_owned()].into());
    host.write(PRIMARY, primary).unwrap();
    host.write(BACKUP, example("Model B", "model-b", maxes[1], None))
        .unwrap();
    host.write(SPARE, example("Model C", "model-c", maxes[2], None))
        .unwrap();
    (host, fake, rx)
}

/// A game of a profile with a chain takes the whole chain along, in its
/// order, each with its own settings file and spend book; while the profile
/// is full a new game starts on the first fallback that is available and
/// takes the ones after it along; when none is, the order is refused.
#[tokio::test]
async fn a_chain_is_taken_along_in_order_and_a_full_profile_starts_on_the_first_free_fallback() {
    let (host, fake, mut rx) = chain("chain-place", [Some(1), Some(1), Some(1)]);
    let spend = host.config.state_dir.join("spend");
    let fallbacks = |j: &SeatJob| -> Vec<(String, PathBuf)> {
        j.fallbacks
            .iter()
            .map(|f| (f.profile.clone(), f.ledger.clone()))
            .collect()
    };
    seat(&host, &mut rx, "o1", PRIMARY).await;
    job(&fake, "o1", |j| {
        assert_eq!(j.plays, PRIMARY);
        assert_eq!(
            fallbacks(j),
            [
                (BACKUP.to_owned(), spend.join("backup.json")),
                (SPARE.to_owned(), spend.join("spare.json")),
            ]
        );
        assert!(j.heard.is_some());
    });
    assert_eq!(reported(&host, PRIMARY).diverts.as_deref(), Some(BACKUP));
    seat(&host, &mut rx, "o2", PRIMARY).await;
    job(&fake, "o2", |j| {
        assert_eq!(j.plays, BACKUP, "the first available fallback");
        assert_eq!(fallbacks(j), [(SPARE.to_owned(), spend.join("spare.json"))]);
        assert_eq!(j.name, "Model-A");
    });
    assert_eq!(reported(&host, PRIMARY).diverts.as_deref(), Some(SPARE));
    seat(&host, &mut rx, "o3", PRIMARY).await;
    job(&fake, "o3", |j| {
        assert_eq!(j.plays, SPARE);
        assert!(j.fallbacks.is_empty() && j.heard.is_none());
    });
    refused_busy(&host, &mut rx, "o4", PRIMARY, "busy").await;
    let games = [PRIMARY, BACKUP, SPARE].map(|id| reported(&host, id).games);
    assert_eq!(games, [1, 1, 1]);
}

/// A fallback that took over and is limited in its turn (the bridge's `the
/// fallback <profile> is rate limited`) is held back itself, not the
/// profile: the next game plays the profile, and a profile the game does
/// not take along is never held back by what its bridge says.
#[tokio::test]
async fn a_limit_heard_of_a_fallback_holds_that_fallback_back() {
    let (host, fake, mut rx) = chain("chain-heard", [None, None, None]);
    host.write("stranger", example("Model D", "model-d", None, None))
        .unwrap();
    seat(&host, &mut rx, "o1", PRIMARY).await;
    hear(
        &fake,
        "o1",
        Heard::Limited {
            of: Some(BACKUP.into()),
            lifts_secs: Some(600),
        },
    );
    eventually("the fallback is held back", || {
        reported(&host, BACKUP).state == State::Exhausted
    })
    .await;
    assert_eq!(reported(&host, PRIMARY).state, State::Available);
    assert_eq!(host.place(PRIMARY).unwrap().plays, PRIMARY);
    hear(
        &fake,
        "o1",
        Heard::Limited {
            of: Some("stranger".into()),
            lifts_secs: Some(600),
        },
    );
    hear(
        &fake,
        "o1",
        Heard::Limited {
            of: None,
            lifts_secs: Some(600),
        },
    );
    eventually("the profile is held back", || {
        reported(&host, PRIMARY).diverts.is_some()
    })
    .await;
    assert_eq!(reported(&host, "stranger").state, State::Available);
    assert_eq!(
        reported(&host, PRIMARY).diverts.as_deref(),
        Some(SPARE),
        "the next of the chain that is available"
    );
}

/// A chain may stand behind an agent CLI: a subscription that runs out
/// mid-game hands over as an API's quota does.
#[tokio::test]
async fn an_agent_cli_takes_its_chain_along() {
    let (host, fake, mut rx) = bare("cli-chain");
    host.write(BACKUP, example("Model B", "model-b", None, None))
        .unwrap();
    let mut cli = cli_definition();
    cli.fallback = Some(BACKUP.into());
    host.write("agent-cli", cli).unwrap();
    seat(&host, &mut rx, "o1", "agent-cli").await;
    job(&fake, "o1", |j| {
        assert_eq!(j.plays, "agent-cli");
        assert_eq!(
            j.fallbacks.first().map(|f| f.profile.as_str()),
            Some(BACKUP)
        );
    });
}
