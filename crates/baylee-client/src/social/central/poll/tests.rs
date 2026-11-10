use super::super::tests::host;
use super::*;

fn poll(generation: u64) -> Job {
    Job::Poll {
        device: "synthetic".into(),
        after: 0,
        generation,
        metadata: true,
    }
}
fn answer(generation: u64) -> Reply {
    let mut snapshot = wire::Snapshot::default();
    snapshot.blocked.push(wire::Person {
        tag: "0009".into(),
        handle: "Stale#0009".into(),
        guest: false,
    });
    Reply::Poll {
        generation,
        snapshot: Some(snapshot),
        keys: vec![],
        messages: vec![],
        after: 99,
        more: false,
    }
}
#[test]
fn a_late_inbox_cannot_overwrite_a_restarted_or_locked_sessions_cursor_and_contacts() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.generation = 2;
    bridge.cursor = 7;
    complete(&mut host, poll(1), Ok(answer(1)), 100);
    let bridge = host.central.as_mut().unwrap();
    assert_eq!(bridge.cursor, 7);
    assert!(bridge.personal.blocked.is_empty());
    bridge.pause();
    let generation = bridge.generation;
    complete(&mut host, poll(generation), Ok(answer(generation)), 100);
    let bridge = host.central.as_ref().unwrap();
    assert_eq!(bridge.cursor, 0);
    assert!(bridge.personal.blocked.is_empty());
}
#[test]
fn a_disconnected_worker_releases_its_flight_and_retries_with_backoff() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    let job = bridge.jobs.pop_front().unwrap();
    let (send, receive) = mpsc::sync_channel(1);
    drop(send);
    bridge.flight = Some(Flight {
        job,
        receive: Mutex::new(receive),
    });
    tick(&mut host, 100);
    let bridge = host.central.as_ref().unwrap();
    assert!(bridge.flight.is_none());
    assert_eq!(bridge.jobs.len(), 1);
    assert_eq!(bridge.next, 2100);
    assert_eq!(bridge.backoff, 4000);
}
#[test]
fn a_rejected_inbox_drops_keys_and_prevents_an_automatic_legacy_reconnect() {
    let mut host = host();
    complete(&mut host, poll(0), Err(Error::Rejected), 100);
    assert!(host.central.is_none());
    assert!(host.machine.is_none());
    assert!(host.vault.attempted);
    assert!(host.session.is_some());
    tick(&mut host, 200);
    assert!(host.machine.is_none());
}

#[test]
fn an_uncertain_send_does_not_starve_directory_and_inbox_reads() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.jobs.clear();
    bridge.ready = true;
    bridge.welcomed = true;
    let sealed = wire::Sealed {
        hdr: "original header".into(),
        n: String::new(),
        ct: String::new(),
        epk: String::new(),
        sig: String::new(),
        wraps: vec![],
    };
    bridge.retries.lock().unwrap().insert(
        sealed.hdr.clone(),
        serde_json::from_value(serde_json::to_value(&sealed).unwrap()).unwrap(),
    );
    failed(
        &mut host,
        Job::Send {
            req: 1,
            conv: "dm:0007:0009".into(),
            tags: vec!["0007".into(), "0009".into()],
            sealed,
        },
        Error::Unavailable,
        100,
    );
    let bridge = host.central.as_ref().unwrap();
    assert!(bridge.jobs.is_empty());
    assert!(
        bridge
            .retries
            .lock()
            .unwrap()
            .contains_key("original header")
    );
    tick(&mut host, 2100);
    assert!(matches!(
        host.central.as_ref().unwrap().jobs.front(),
        Some(Job::Poll { .. })
    ));
}

#[test]
fn a_terminal_send_failure_releases_only_its_own_retry_slot() {
    for error in [Error::Rejected, Error::TermsRequired] {
        let mut host = host();
        let sealed = wire::Sealed {
            hdr: "terminal request".into(),
            n: String::new(),
            ct: String::new(),
            epk: String::new(),
            sig: String::new(),
            wraps: vec![],
        };
        let retries = host.central.as_ref().unwrap().retries.clone();
        for key in ["terminal request", "still uncertain"] {
            retries.lock().unwrap().insert(
                key.into(),
                serde_json::from_value(serde_json::to_value(&sealed).unwrap()).unwrap(),
            );
        }
        failed(
            &mut host,
            Job::Send {
                req: 1,
                conv: "dm:0007:0009".into(),
                tags: vec!["0007".into(), "0009".into()],
                sealed,
            },
            error,
            100,
        );
        let retries = retries.lock().unwrap();
        assert_eq!(
            retries.len(),
            1,
            "terminal failures must not exhaust the 64-slot cache"
        );
        assert!(retries.contains_key("still uncertain"));
    }
}

#[test]
fn live_central_websocket_suppresses_timer_polling_and_wakes_on_push() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.jobs.clear();
    bridge.ready = true;
    bridge.welcomed = true;
    bridge.handle.set_ws_live_for_test(true);

    complete(&mut host, poll(0), Ok(answer(0)), 100);
    let bridge = host.central.as_ref().unwrap();
    assert_eq!(bridge.next, u64::MAX);
    assert_eq!(bridge.metadata_at, 100 + LIVE_POLL_MS);

    tick(&mut host, 10_000);
    assert!(
        host.central.as_ref().unwrap().jobs.is_empty(),
        "no timer poll while Central WebSocket is live"
    );

    host.central.as_ref().unwrap().handle.push_social_for_test();
    tick(&mut host, 10_100);
    assert!(
        matches!(
            host.central.as_ref().unwrap().jobs.front(),
            Some(Job::Poll { metadata: true, .. })
        ),
        "social WebSocket push immediately schedules metadata+inbox poll"
    );
}

#[test]
fn a_live_socket_that_pushes_nothing_still_gets_a_slow_read_with_contacts() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.jobs.clear();
    bridge.ready = true;
    bridge.welcomed = true;
    bridge.handle.set_ws_live_for_test(true);
    complete(&mut host, poll(0), Ok(answer(0)), 100);
    tick(&mut host, 100 + LIVE_POLL_MS - 1);
    assert!(host.central.as_ref().unwrap().jobs.is_empty());
    tick(&mut host, 100 + LIVE_POLL_MS);
    assert!(
        matches!(
            host.central.as_ref().unwrap().jobs.front(),
            Some(Job::Poll { metadata: true, .. })
        ),
        "a half-open socket cannot stop chat for good"
    );
}

#[test]
fn a_socket_lost_mid_read_leaves_no_contacts_read_waiting_for_ever() {
    let mut host = host();
    let bridge = host.central.as_mut().unwrap();
    bridge.jobs.clear();
    bridge.ready = true;
    bridge.welcomed = true;
    bridge.handle.set_ws_live_for_test(true);
    complete(&mut host, poll(0), Ok(answer(0)), 100);
    // A push wakes a read without contacts; the socket goes while it runs.
    let bridge = host.central.as_mut().unwrap();
    bridge.next = 0;
    bridge.handle.set_ws_live_for_test(false);
    let mut quiet = answer(0);
    if let Reply::Poll { snapshot, .. } = &mut quiet {
        *snapshot = None;
    }
    complete(&mut host, poll(0), Ok(quiet), 200);
    let bridge = host.central.as_ref().unwrap();
    assert_eq!(bridge.next, 2200);
    assert!(bridge.metadata_at <= 100 + LIVE_POLL_MS);
}

#[test]
fn while_the_socket_is_live_a_queued_job_leaves_at_once_and_a_failure_still_waits() {
    let mut host = host();
    let (handle, sent) = crate::lobby::central::Handle::queued_for_test();
    let bridge = host.central.as_mut().unwrap();
    bridge.handle = handle;
    bridge.jobs.clear();
    bridge.ready = true;
    bridge.welcomed = true;
    bridge.handle.set_ws_live_for_test(true);
    complete(&mut host, poll(0), Ok(answer(0)), 100);
    let connect = || Job::Connect {
        own: "0007".into(),
        device: "synthetic".into(),
    };
    let bridge = host.central.as_mut().unwrap();
    assert!(bridge.queue(connect()));
    tick(&mut host, 200);
    assert!(
        host.central.as_ref().unwrap().flight.is_some(),
        "not held a minute"
    );
    assert!(sent.try_recv().is_ok());
    let bridge = host.central.as_mut().unwrap();
    bridge.flight = None;
    failed(&mut host, connect(), Error::Unavailable, 300);
    tick(&mut host, 400);
    let bridge = host.central.as_ref().unwrap();
    assert!(bridge.flight.is_none(), "the backoff still holds");
    assert_eq!(bridge.jobs.len(), 1);
}
