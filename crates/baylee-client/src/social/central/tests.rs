use super::*;
use baylee_client_core::social::{ChatPrefs, Me, Social};
use baylee_seal::testkit::Seeded;

mod retries;

pub(super) fn host() -> SocialHost {
    let session = Session {
        gateway: "https://gateway.invalid".into(),
        token: "synthetic".into(),
        handle: "Alice#0007".into(),
        tag: "0007".into(),
        guest: false,
    };
    let (mut machine, _) = Social::start(
        Me {
            tag: session.tag.clone(),
            handle: session.handle.clone(),
            guest: false,
            gateway: session.gateway.clone(),
        },
        None,
        None,
        ChatPrefs::default(),
        100,
        &mut Seeded::new(39),
    );
    machine.use_account_transport();
    let bridge = Bridge::new(
        Handle::unavailable_for_test(),
        session.clone(),
        machine.device_id().into(),
    );
    SocialHost {
        machine: Some(machine),
        session: Some(session),
        central: Some(bridge),
        ..SocialHost::default()
    }
}

#[test]
fn a_full_account_queue_keeps_a_personal_message_pending() {
    use baylee_client_core::social::{ComposerKey, ComposerOutcome, Intent, LineState};
    let mut host = host();
    let conv: wire::Conv = "dm:0007:0009".parse().unwrap();
    let machine = host.machine.as_mut().unwrap();
    let mut rng = Seeded::new(40);
    machine.act(Intent::OpenConv(conv.clone()), 100, &mut rng);
    machine.composer_mut().typed("Please keep this queued");
    assert_eq!(
        machine.key(ComposerKey::Enter, 100, &mut rng).0,
        ComposerOutcome::Submit
    );
    let line = machine.conversation(&conv).unwrap().lines.last().unwrap();
    assert_eq!(line.state, LineState::Pending);
    let line_id = line.key.id.clone();
    let bridge = host.central.as_mut().unwrap();
    while bridge.jobs.len() < 16 {
        assert!(bridge.queue(Job::Connect {
            own: "0007".into(),
            device: "synthetic".into()
        }));
    }
    assert!(up(
        &mut host,
        &Up::Send {
            req: 1,
            conv: conv.to_string(),
            sealed: wire::Sealed {
                hdr: "pending ciphertext".into(),
                n: String::new(),
                ct: String::new(),
                epk: String::new(),
                sig: String::new(),
                wraps: vec![],
            },
        }
    ));
    let line = host
        .machine
        .as_ref()
        .unwrap()
        .conversation(&conv)
        .unwrap()
        .line(&line_id)
        .unwrap();
    assert_eq!(
        line.state,
        LineState::Pending,
        "a full local queue is temporary backpressure"
    );
    assert_eq!(line.failure, None);
    assert_eq!(host.central.as_ref().unwrap().jobs.len(), 16);
}

#[test]
fn backpressure_drops_queued_reads_and_invalidates_all_late_inbox_answers() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.jobs.clear();
    bridge.cursor = 99;
    let generation = bridge.generation;
    assert!(bridge.queue(Job::Poll {
        device: bridge.device.clone(),
        after: 99,
        generation,
        metadata: true
    }));
    bridge.pause();
    assert!(bridge.jobs.is_empty());
    assert_eq!(bridge.cursor, 0);
    assert_ne!(bridge.generation, generation);
    assert!(!bridge.wanted);
    let paused = bridge.generation;
    bridge.resume();
    assert_ne!(bridge.generation, paused);
    assert!(bridge.wanted);
}

#[test]
fn gateway_snapshots_cannot_create_personal_contacts_or_groups() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.personal.blocked.push(wire::Person {
        tag: "0009".into(),
        handle: "Bob#0009".into(),
        guest: false,
    });
    bridge.game.incoming.push(wire::Person {
        tag: "0011".into(),
        handle: "Injected#0011".into(),
        guest: false,
    });
    let snapshot = bridge.snapshot();
    assert!(snapshot.incoming.is_empty());
    assert_eq!(snapshot.blocked.len(), 1);
    assert_eq!(snapshot.blocked[0].tag, "0009");
}

#[test]
fn one_session_cannot_accumulate_unbounded_account_jobs() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.jobs.clear();
    for _ in 0..16 {
        assert!(bridge.queue(Job::Connect {
            own: "0007".into(),
            device: bridge.device.clone()
        }));
    }
    assert!(!bridge.queue(Job::Connect {
        own: "0007".into(),
        device: bridge.device.clone()
    }));
    assert_eq!(bridge.jobs.len(), 16);
}

#[test]
fn central_friend_presence_updates_from_central_ws_without_any_gateway() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.personal.friends.push(wire::Friend {
        person: wire::Person {
            tag: "002a".into(),
            handle: "Argon#002a".into(),
            guest: false,
        },
        since: 1,
        presence: wire::Presence::offline(),
    });
    assert_eq!(
        bridge.snapshot().friends[0].presence.status,
        wire::Shown::Offline
    );
    bridge.handle.set_online_tags_for_test(["002a".to_owned()]);
    assert_eq!(
        bridge.snapshot().friends[0].presence.status,
        wire::Shown::Online
    );
}

#[test]
fn a_game_gateway_opens_only_so_many_table_conversations_and_no_other_kind() {
    let mut host = host();
    let person = |tag: &str| wire::Person {
        tag: tag.into(),
        handle: format!("P#{tag}"),
        guest: false,
    };
    // `n` members: this account (0007) and `n - 1` others.
    let members = |n: usize| {
        let mut all = vec![person("0009"); n];
        if let Some(first) = all.first_mut() {
            *first = person("0007");
        }
        all
    };
    let frame = |conv: &str, n: usize| {
        transport::LinkNews::Frame(
            serde_json::to_string(&Down::Members {
                conv: conv.into(),
                members: members(n),
            })
            .unwrap(),
        )
    };
    let conversations = |host: &SocialHost| host.machine.as_ref().unwrap().conversations().len();
    let before = conversations(&host);
    for conv in ["global", "dm:0007:0009", "grp:0193", "not a conversation"] {
        assert!(gateway_news(&mut host, &frame(conv, 1)));
    }
    assert!(gateway_news(&mut host, &frame("tbl:empty", 0)));
    assert!(host.central.as_ref().unwrap().game.tables.is_empty());
    assert_eq!(
        conversations(&host),
        before,
        "no other kind reaches the chat"
    );
    for n in 0..100 {
        gateway_news(&mut host, &frame(&format!("tbl:game-{n}"), 1));
        gateway_news(&mut host, &frame(&format!("team:game-{n}:1"), 1));
    }
    let tables = &host.central.as_ref().unwrap().game.tables;
    assert_eq!(tables.len(), gateway::MAX_TABLE_CONVS);
    let opened = conversations(&host);
    assert!(opened > before, "the tables it may name reach the chat");
    assert!(opened <= before + gateway::MAX_TABLE_CONVS);
    // A known one still updates; one ended or left makes room for the next.
    gateway_news(&mut host, &frame("tbl:game-0", 2));
    assert_eq!(
        host.central.as_ref().unwrap().game.tables[0].members.len(),
        2
    );
    gateway_news(&mut host, &frame("tbl:game-0", 0));
    let left = Down::Members {
        conv: "team:game-0:1".into(),
        members: vec![person("0009")],
    };
    gateway_news(
        &mut host,
        &transport::LinkNews::Frame(serde_json::to_string(&left).unwrap()),
    );
    assert_eq!(
        host.central.as_ref().unwrap().game.tables.len(),
        gateway::MAX_TABLE_CONVS - 2
    );
    gateway_news(&mut host, &frame("tbl:game-new", 1));
    let tables = &host.central.as_ref().unwrap().game.tables;
    assert!(tables.iter().any(|t| t.conv == "tbl:game-new"));
    // A table without this account among its members is not opened at all.
    let elsewhere = Down::Members {
        conv: "tbl:elsewhere".into(),
        members: vec![person("0009")],
    };
    gateway_news(
        &mut host,
        &transport::LinkNews::Frame(serde_json::to_string(&elsewhere).unwrap()),
    );
    let tables = &host.central.as_ref().unwrap().game.tables;
    assert!(!tables.iter().any(|t| t.conv == "tbl:elsewhere"));
}
