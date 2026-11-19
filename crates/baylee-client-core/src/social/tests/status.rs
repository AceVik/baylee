//! The contact status (`docs/client.md` §"The contact status"): the choice a
//! player makes, what a game gateway is told of it, what Baylee Central is
//! told the client does, and this device's own do not disturb.

use baylee_protocol::social::{PresenceSet, Status, Up};
use central_api::presence::{Activity, Choice};

use super::fakegw::{Client, Gateway};
use crate::social::machine::{AWAY_AFTER_MS, kept_choice};
use crate::social::{ChatPhrase, Effect, Intent};

fn alice() -> (Gateway, Client) {
    let mut gw = Gateway::default();
    let alice = Client::new(&mut gw, "af03", "Alice#af03", 1);
    (gw, alice)
}

/// The statuses the gateway was told, in order.
fn told(effects: &[Effect]) -> Vec<Status> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Up(Up::Presence(PresenceSet { status, .. })) => Some(*status),
            _ => None,
        })
        .collect()
}

#[test]
fn a_choice_tells_the_gateway_and_asks_central_to_keep_it() {
    let (mut gw, mut alice) = alice();
    assert_eq!(alice.social.choice(), Choice::Online);
    for (choice, was, status) in [
        (Choice::Afk, Choice::Online, Status::Away),
        (Choice::Offline, Choice::Afk, Status::Invisible),
        (Choice::Online, Choice::Offline, Status::Online),
    ] {
        let effects = alice.act(&mut gw, Intent::Choose(choice));
        assert_eq!(told(&effects), [status], "{choice:?}");
        assert!(
            effects.contains(&Effect::Choose { choice, was }),
            "{choice:?}: {effects:?}"
        );
        assert_eq!(alice.social.choice(), choice);
    }
}

#[test]
fn a_choice_outlives_a_restart_of_the_device() {
    let (mut gw, mut alice) = alice();
    alice.act(&mut gw, Intent::Choose(Choice::Offline));
    alice.act(&mut gw, Intent::Quiet(true));
    alice.tick(&mut gw, alice.now + 5_000);
    alice.restart(&mut gw);
    assert_eq!(alice.social.choice(), Choice::Offline);
    assert!(alice.social.quiet());
    assert_eq!(alice.social.status(), Status::Invisible);
}

#[test]
fn ten_idle_minutes_read_away_until_the_next_input_and_a_choice_of_away_stays() {
    let (_gw, mut alice) = alice();
    let now = alice.now;
    assert_eq!(alice.social.report(), (None, false));
    let idle = alice.social.tick(now, AWAY_AFTER_MS);
    assert_eq!(told(&idle), [Status::Away]);
    assert_eq!(alice.social.report(), (None, true));
    // Idle a little longer says nothing again.
    assert!(told(&alice.social.tick(now, AWAY_AFTER_MS + 9_000)).is_empty());
    let back = alice.social.tick(now, 0);
    assert_eq!(told(&back), [Status::Online]);
    assert_eq!(alice.social.report(), (None, false));
    // Chosen away, idleness changes what Central hears and not the status.
    let chosen = alice
        .social
        .act(Intent::Choose(Choice::Afk), now, &mut alice.rng);
    assert_eq!(told(&chosen), [Status::Away]);
    assert!(told(&alice.social.tick(now, AWAY_AFTER_MS)).is_empty());
    assert_eq!(alice.social.report(), (None, true));
    assert!(told(&alice.social.tick(now, 0)).is_empty());
    assert_eq!(alice.social.status(), Status::Away);
    // The choice was never turned into "online" by an input.
    assert_eq!(alice.social.choice(), Choice::Afk);
}

#[test]
fn what_the_client_is_doing_is_reported_and_never_told_to_the_gateway() {
    let (_gw, mut alice) = alice();
    alice.social.set_doing(Some(Activity::Table));
    assert_eq!(alice.social.report(), (Some(Activity::Table), false));
    alice.social.set_doing(Some(Activity::Playing));
    assert_eq!(alice.social.report(), (Some(Activity::Playing), false));
    assert_eq!(alice.social.status(), Status::Online);
}

#[test]
fn until_central_says_the_choice_the_gateway_is_told_invisible() {
    let (_gw, mut alice) = alice();
    // The choice on this device may be older than one made on another.
    assert_eq!(told(&alice.social.await_choice()), [Status::Invisible]);
    assert_eq!(alice.social.status(), Status::Invisible);
    assert_eq!(
        alice.social.choice(),
        Choice::Online,
        "the menu still shows it"
    );
    // Idleness does not uncover it.
    let now = alice.now;
    assert!(told(&alice.social.tick(now, AWAY_AFTER_MS)).is_empty());
    assert!(told(&alice.social.tick(now, 0)).is_empty());
    // Central kept "away", chosen elsewhere: that is the choice now.
    assert_eq!(
        told(&alice.social.heard_choice(Some(Choice::Afk))),
        [Status::Away]
    );
    assert_eq!(alice.social.choice(), Choice::Afk);
    // The same word again changes and says nothing.
    assert!(alice.social.heard_choice(Some(Choice::Afk)).is_empty());
}

#[test]
fn a_central_that_keeps_no_choice_leaves_the_devices_own() {
    let (_gw, mut alice) = alice();
    let now = alice.now;
    alice
        .social
        .act(Intent::Choose(Choice::Afk), now, &mut alice.rng);
    alice.social.await_choice();
    assert_eq!(told(&alice.social.heard_choice(None)), [Status::Away]);
    assert_eq!(alice.social.choice(), Choice::Afk);
}

#[test]
fn a_refused_choice_goes_back_and_says_why() {
    for (terms, phrase) in [
        (true, ChatPhrase::ToastStatusTerms),
        (false, ChatPhrase::ToastStatusRefused),
    ] {
        let (_gw, mut alice) = alice();
        let now = alice.now;
        alice
            .social
            .act(Intent::Choose(Choice::Offline), now, &mut alice.rng);
        let effects = alice.social.choice_refused(Choice::Online, terms);
        assert_eq!(alice.social.choice(), Choice::Online);
        assert_eq!(told(&effects), [Status::Online]);
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Toast(toast) if toast.phrase == phrase)),
            "{effects:?}"
        );
    }
}

#[test]
fn do_not_disturb_is_this_devices_own_and_friends_are_told_nothing_of_it() {
    let (mut gw, mut alice) = alice();
    let effects = alice.act(&mut gw, Intent::Quiet(true));
    assert!(alice.social.quiet());
    assert!(told(&effects).is_empty());
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Choose { .. })),
        "{effects:?}"
    );
    assert_eq!(alice.social.status(), Status::Online);
    assert_eq!(alice.social.choice(), Choice::Online);
}

#[test]
fn a_state_file_from_before_the_status_keeps_what_its_one_status_said() {
    assert_eq!(kept_choice(None, false, None), (Choice::Online, false));
    assert_eq!(
        kept_choice(None, false, Some(Status::Online)),
        (Choice::Online, false)
    );
    assert_eq!(
        kept_choice(None, false, Some(Status::Away)),
        (Choice::Afk, false)
    );
    assert_eq!(
        kept_choice(None, false, Some(Status::Invisible)),
        (Choice::Offline, false)
    );
    // Do not disturb was a status friends saw; it is this device's quiet now.
    assert_eq!(
        kept_choice(None, false, Some(Status::Dnd)),
        (Choice::Online, true)
    );
    // Once the device wrote a choice, the old status says nothing more.
    assert_eq!(
        kept_choice(Some(Choice::Online), false, Some(Status::Dnd)),
        (Choice::Online, false)
    );
    assert_eq!(
        kept_choice(Some(Choice::Afk), true, Some(Status::Invisible)),
        (Choice::Afk, true)
    );
}
