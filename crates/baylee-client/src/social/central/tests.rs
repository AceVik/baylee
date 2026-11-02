use super::*;
use baylee_client_core::social::{ChatPrefs, Me, Social};
use baylee_seal::testkit::Seeded;

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
    });
    bridge.game.incoming.push(wire::Person {
        tag: "0011".into(),
        handle: "Injected#0011".into(),
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
