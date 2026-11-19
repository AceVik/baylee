//! The contact status on the account's worker: one choice at a time, the
//! word Central has the last of, and a report that is said after every
//! `ready` and then only when it changes.
use super::*;
use crate::lobby::central::{Handle, Job};
use baylee_client_core::social::Intent;
use baylee_protocol::social::Status as Told;

const T0: u64 = 1_000_000;

/// A host whose worker jobs the test receives.
fn host() -> (SocialHost, mpsc::Receiver<Job>) {
    let mut host = super::super::tests::host();
    let (handle, jobs) = Handle::queued_for_test();
    let bridge = host.central.as_mut().unwrap();
    bridge.status = Status::new(handle.ready_seq());
    bridge.handle = handle;
    (host, jobs)
}

/// How many jobs the worker was handed since the last count. Dropping them
/// is a worker that went: whoever waits for an answer hears `Unavailable`.
fn handed(jobs: &mpsc::Receiver<Job>) -> usize {
    jobs.try_iter().count()
}

/// The jobs the worker was handed since the last look, kept: a worker that
/// has them and has not answered yet.
fn held(jobs: &mpsc::Receiver<Job>) -> Vec<Job> {
    jobs.try_iter().collect()
}

fn status(host: &mut SocialHost) -> &mut Status {
    &mut host.central.as_mut().unwrap().status
}

fn choice(host: &SocialHost) -> Choice {
    host.machine.as_ref().unwrap().choice()
}

/// The kept choice is known, so no reading is on its way.
fn known(host: &mut SocialHost, kept: Choice) {
    heard(host, Some(kept));
}

/// Central's answer to the choice on its way.
fn answer(host: &mut SocialHost, result: Result<Choice, Error>) {
    let (send, receive) = mpsc::sync_channel(1);
    send.send(result).unwrap();
    status(host)
        .asked
        .as_mut()
        .expect("a choice on its way")
        .answer = Mutex::new(receive);
}

fn choose_in_machine(host: &mut SocialHost, choice: Choice) {
    let effects = host.machine.as_mut().unwrap().act(
        Intent::Choose(choice),
        T0,
        &mut crate::social::OsEntropy,
    );
    host.carry_out(effects);
}

#[test]
fn a_choice_goes_to_central_one_at_a_time_and_the_latest_wins() {
    let (mut host, jobs) = host();
    known(&mut host, Choice::Online);
    choose_in_machine(&mut host, Choice::Afk);
    choose_in_machine(&mut host, Choice::Offline);
    // Two choices before the worker took any: one request, for the latest,
    // and what stood before both is what a refusal goes back to.
    tick(&mut host, T0);
    let first = held(&jobs);
    assert_eq!(first.len(), 1);
    assert_eq!(
        status(&mut host).asked.as_ref().unwrap().was,
        Choice::Online
    );
    // A third while that one is out waits its turn.
    choose_in_machine(&mut host, Choice::Online);
    tick(&mut host, T0 + 10);
    assert_eq!(handed(&jobs), 0);
    answer(&mut host, Ok(Choice::Offline));
    tick(&mut host, T0 + 20);
    assert_eq!(handed(&jobs), 1, "the waiting choice goes next");
    drop(first);
    // The answer to the older choice did not undo the newer one on screen.
    assert_eq!(choice(&host), Choice::Online);
    answer(&mut host, Ok(Choice::Online));
    tick(&mut host, T0 + 30);
    assert_eq!(handed(&jobs), 0);
    assert_eq!(choice(&host), Choice::Online);
}

#[test]
fn a_refused_choice_puts_back_what_central_last_kept() {
    for refusal in [Error::TermsRequired, Error::Busy, Error::Unavailable] {
        let (mut host, jobs) = host();
        known(&mut host, Choice::Afk);
        choose_in_machine(&mut host, Choice::Offline);
        assert_eq!(choice(&host), Choice::Offline, "shown at once");
        tick(&mut host, T0);
        assert_eq!(handed(&jobs), 1);
        answer(&mut host, Err(refusal));
        tick(&mut host, T0 + 10);
        assert_eq!(choice(&host), Choice::Afk);
        assert_eq!(host.machine.as_ref().unwrap().status(), Told::Away);
        assert!(status(&mut host).asked.is_none());
        assert_eq!(handed(&jobs), 0, "nothing is asked again unasked");
    }
}

#[test]
fn a_worker_that_went_refuses_the_choice() {
    let (mut host, jobs) = host();
    known(&mut host, Choice::Online);
    choose_in_machine(&mut host, Choice::Offline);
    tick(&mut host, T0);
    // The job is dropped unrun: its answer never comes.
    assert_eq!(handed(&jobs), 1);
    tick(&mut host, T0 + 10);
    assert_eq!(choice(&host), Choice::Online);
}

#[test]
fn centrals_word_is_heard_except_while_a_choice_of_this_client_is_on_its_way() {
    let (mut host, jobs) = host();
    known(&mut host, Choice::Afk);
    assert_eq!(choice(&host), Choice::Afk, "chosen on another device");
    choose_in_machine(&mut host, Choice::Offline);
    tick(&mut host, T0);
    assert_eq!(handed(&jobs), 1);
    // A frame from before Central took the choice still says the old one.
    heard(&mut host, Some(Choice::Afk));
    assert_eq!(choice(&host), Choice::Offline);
    answer(&mut host, Ok(Choice::Offline));
    tick(&mut host, T0 + 10);
    assert_eq!(choice(&host), Choice::Offline);
    // Later, another device chooses: that is the choice here too.
    heard(&mut host, Some(Choice::Online));
    assert_eq!(choice(&host), Choice::Online);
}

#[test]
fn the_kept_choice_is_read_until_it_is_known_and_uncovers_the_account() {
    let (mut host, jobs) = host();
    let effects = host.machine.as_mut().unwrap().await_choice();
    host.carry_out(effects);
    assert_eq!(host.machine.as_ref().unwrap().status(), Told::Invisible);
    tick(&mut host, T0);
    assert_eq!(handed(&jobs), 1, "the reading; no socket is live");
    // It failed (the job went): not again before a minute has passed.
    tick(&mut host, T0 + 10);
    tick(&mut host, T0 + READ_AGAIN_MS - 1);
    assert_eq!(handed(&jobs), 0);
    assert_eq!(host.machine.as_ref().unwrap().status(), Told::Invisible);
    tick(&mut host, T0 + 10 + READ_AGAIN_MS);
    assert_eq!(handed(&jobs), 1);
    let (send, receive) = mpsc::sync_channel(1);
    send.send(Ok(Some(Choice::Afk))).unwrap();
    status(&mut host).reading = Some(Mutex::new(receive));
    tick(&mut host, T0 + 20 + READ_AGAIN_MS);
    assert_eq!(choice(&host), Choice::Afk);
    assert_eq!(host.machine.as_ref().unwrap().status(), Told::Away);
    tick(&mut host, T0 + 10 * READ_AGAIN_MS);
    assert_eq!(handed(&jobs), 0, "known: never read again");
}

/// A Central from before the status keeps no choice: nobody chose offline
/// there, so the account is not kept hidden from its game gateway for good,
/// and what this device kept stands.
#[test]
fn a_central_that_keeps_no_choice_uncovers_the_account() {
    for (kept, told) in [
        (Choice::Online, Told::Online),
        (Choice::Offline, Told::Invisible),
    ] {
        let (mut host, jobs) = host();
        choose_in_machine(&mut host, kept);
        // That choice is refused by such a Central; here only the reading is
        // looked at, so it never left.
        status(&mut host).wanted = None;
        let effects = host.machine.as_mut().unwrap().await_choice();
        host.carry_out(effects);
        assert_eq!(host.machine.as_ref().unwrap().status(), Told::Invisible);
        tick(&mut host, T0);
        assert_eq!(handed(&jobs), 1);
        let (send, receive) = mpsc::sync_channel(1);
        send.send(Ok(None)).unwrap();
        status(&mut host).reading = Some(Mutex::new(receive));
        tick(&mut host, T0 + 10);
        assert_eq!(choice(&host), kept);
        assert_eq!(host.machine.as_ref().unwrap().status(), told);
        tick(&mut host, T0 + 10 * READ_AGAIN_MS);
        assert_eq!(handed(&jobs), 0, "answered: not read again");
    }
}

/// A rejected reading is no answer by itself: a session that ended is
/// rejected too, and another device may have chosen offline.
#[test]
fn a_rejected_reading_keeps_the_account_covered() {
    let (mut host, jobs) = host();
    let effects = host.machine.as_mut().unwrap().await_choice();
    host.carry_out(effects);
    tick(&mut host, T0);
    assert_eq!(handed(&jobs), 1);
    let (send, receive) = mpsc::sync_channel(1);
    send.send(Err(Error::Rejected)).unwrap();
    status(&mut host).reading = Some(Mutex::new(receive));
    tick(&mut host, T0 + 10);
    assert_eq!(host.machine.as_ref().unwrap().status(), Told::Invisible);
    tick(&mut host, T0 + 20 + READ_AGAIN_MS);
    assert_eq!(handed(&jobs), 1, "asked again after a minute");
}

#[test]
fn only_a_standing_session_makes_a_rejection_a_central_without_the_status() {
    assert_eq!(
        kept_at_central(Ok(Choice::Afk), || unreachable!()),
        Ok(Some(Choice::Afk))
    );
    assert_eq!(kept_at_central(Err(Error::Rejected), || true), Ok(None));
    assert_eq!(
        kept_at_central(Err(Error::Rejected), || false),
        Err(Error::Rejected)
    );
    for other in [Error::Unavailable, Error::Busy, Error::TermsRequired] {
        assert_eq!(kept_at_central(Err(other), || unreachable!()), Err(other));
    }
}

#[test]
fn the_report_is_said_after_every_ready_and_then_only_when_it_changes() {
    let (mut host, jobs) = host();
    known(&mut host, Choice::Online);
    // No socket: Central would hear nothing.
    tick(&mut host, T0);
    assert_eq!(handed(&jobs), 0);
    let handle = host.central.as_ref().unwrap().handle.clone();
    handle.set_ws_live_for_test(true);
    handle.say_ready_for_test();
    tick(&mut host, T0 + 10);
    assert_eq!(handed(&jobs), 1, "said even though it is nothing");
    tick(&mut host, T0 + 10 + REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 0, "unchanged");
    // Two changes within the floor: one report, of the later.
    host.machine
        .as_mut()
        .unwrap()
        .set_doing(Some(Activity::Table), None);
    tick(&mut host, T0 + 20 + REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    host.machine
        .as_mut()
        .unwrap()
        .set_doing(Some(Activity::Playing), None);
    tick(&mut host, T0 + 30 + REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 0, "too soon after the last");
    tick(&mut host, T0 + 20 + 2 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(
        status(&mut host).said,
        Some((Some(Activity::Playing), false, None))
    );
    // The socket dialled again: Central forgot, so the same is said again.
    handle.say_ready_for_test();
    tick(&mut host, T0 + 20 + 3 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    // A socket that is down hears nothing.
    handle.set_ws_live_for_test(false);
    host.machine.as_mut().unwrap().set_doing(None, None);
    tick(&mut host, T0 + 20 + 4 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 0);
}

/// The player's own social settings, as the drawer sends them to the game
/// gateway: the bridge takes them on the way.
fn share(host: &mut SocialHost, share_table: bool) {
    let settings = serde_json::to_value(baylee_protocol::social::SocialSettings {
        share_table,
        ..Default::default()
    })
    .unwrap();
    // The gateway, not the account worker, carries them.
    assert!(!super::super::request(
        host,
        &baylee_client_core::social::HttpTag::Settings,
        baylee_client_core::social::Method::Put,
        "/social/settings",
        Some(&settings),
    ));
}

#[test]
fn a_table_goes_only_while_it_is_shared_and_central_said_it_takes_one() {
    let (mut host, jobs) = host();
    known(&mut host, Choice::Online);
    let handle = host.central.as_ref().unwrap().handle.clone();
    handle.set_ws_live_for_test(true);
    handle.say_ready_for_test();
    let table = Table {
        origin: "https://gateway.invalid".into(),
        game: "g-1".into(),
        name: "Friday".into(),
        seats: 4,
        taken: 2,
    };
    host.machine
        .as_mut()
        .unwrap()
        .set_doing(Some(Activity::Table), Some(table.clone()));
    let said = |host: &mut SocialHost| status(host).said.clone().map(|said| said.2);
    // A Central that did not announce tables, a setting nobody has said yet
    // (the default shares): no table.
    tick(&mut host, T0);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(said(&mut host), Some(None));
    let takes = crate::lobby::central::Seen {
        takes_tables: true,
        ..Default::default()
    };
    handle.set_seen_for_test(takes.clone());
    tick(&mut host, T0 + REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 0, "the default is not the player's word");
    // The player shares it: the table goes.
    share(&mut host, true);
    tick(&mut host, T0 + 2 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(said(&mut host), Some(Some(table.clone())));
    // The player stops sharing: a report without it, at once.
    share(&mut host, false);
    tick(&mut host, T0 + 3 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(said(&mut host), Some(None));
    // Shared again, then the socket comes back from a Central without
    // tables: a report without it.
    share(&mut host, true);
    tick(&mut host, T0 + 4 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(said(&mut host), Some(Some(table.clone())));
    handle.set_seen_for_test(crate::lobby::central::Seen::default());
    handle.say_ready_for_test();
    tick(&mut host, T0 + 5 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(said(&mut host), Some(None));
    // A table the machine no longer names (a password set, the player got
    // up) goes from the report too.
    handle.set_seen_for_test(takes);
    tick(&mut host, T0 + 6 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(said(&mut host), Some(Some(table)));
    host.machine
        .as_mut()
        .unwrap()
        .set_doing(Some(Activity::Table), None);
    tick(&mut host, T0 + 7 * REPORT_EVERY_MS);
    assert_eq!(handed(&jobs), 1);
    assert_eq!(said(&mut host), Some(None));
}

#[test]
fn a_game_gateway_s_snapshot_is_the_player_s_word_on_sharing() {
    let (mut host, jobs) = host();
    known(&mut host, Choice::Online);
    let handle = host.central.as_ref().unwrap().handle.clone();
    handle.set_ws_live_for_test(true);
    handle.set_seen_for_test(crate::lobby::central::Seen {
        takes_tables: true,
        ..Default::default()
    });
    handle.say_ready_for_test();
    let table = Table {
        origin: "https://gateway.invalid".into(),
        game: "g-1".into(),
        name: String::new(),
        seats: 2,
        taken: 1,
    };
    host.machine
        .as_mut()
        .unwrap()
        .set_doing(Some(Activity::Playing), Some(table.clone()));
    // The gateway's snapshot, with sharing off: nothing.
    let snapshot = |share_table: bool| {
        serde_json::to_string(&baylee_protocol::social::Down::Snapshot(
            baylee_protocol::social::Snapshot {
                settings: baylee_protocol::social::SocialSettings {
                    share_table,
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .unwrap()
    };
    super::super::gateway_news(
        &mut host,
        &super::super::transport::LinkNews::Frame(snapshot(false)),
    );
    tick(&mut host, T0);
    assert_eq!(
        status(&mut host).said.clone().map(|said| said.2),
        Some(None)
    );
    // With sharing on: the table.
    super::super::gateway_news(
        &mut host,
        &super::super::transport::LinkNews::Frame(snapshot(true)),
    );
    tick(&mut host, T0 + REPORT_EVERY_MS);
    assert_eq!(
        status(&mut host).said.clone().map(|said| said.2),
        Some(Some(table))
    );
    let _ = handed(&jobs);
}
