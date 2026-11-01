//! Conversations through the fake gateway (§16, "client core"): two
//! parties, more devices, a new device, replay, forwarding, the roster
//! check, the absence of any plaintext path, refused and lost sends,
//! deliveries that wait for keys, blocked and ignored senders, and unread
//! counts.

use baylee_protocol::social::{Conv, Delivered, Down, FriendEvent, Person, Sender, Up, Why};
use baylee_seal::b64::url;
use baylee_seal::testkit::Seeded;
use baylee_seal::{Recipient, RestKind, at_rest_open};

use super::fakegw::{Client, Gateway};
use crate::social::convs::Body;
use crate::social::{
    ChatLine, ChatPhrase, ChatPrefs, Effect, HttpTag, Intent, LineKey, LineKind, LineState, Method,
    RosterSeat, TableContext, disk, sealing, vault,
};

const ALICE: &str = "af03";
const BOB: &str = "0012";
const MALLORY: &str = "0bad";
const CAROL: &str = "0c11";
const DM_AB: &str = "dm:0012:af03";

/// A gateway that knows four accounts' handles.
fn world() -> Gateway {
    let mut gw = Gateway::default();
    for (tag, handle) in [
        (ALICE, "Alice#af03"),
        (BOB, "Bob#0012"),
        (MALLORY, "Mallory#0bad"),
        (CAROL, "Carol#0c11"),
    ] {
        gw.handles.insert(tag.into(), handle.into());
    }
    gw
}

/// Alice and Bob, friends, one device each.
fn pair() -> (Gateway, Client, Client) {
    let mut gw = world();
    gw.befriend(ALICE, BOB);
    let alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    (gw, alice, bob)
}

/// The sender, the "mine" mark and the body of a message line.
fn message(line: &ChatLine) -> (&Person, bool, &Body) {
    match &line.kind {
        LineKind::Message { from, mine, body } => (from, *mine, body),
        other => panic!("not a message: {other:?}"),
    }
}

/// The phrases of a conversation's notice lines.
fn phrases(client: &Client, conv: &str) -> Vec<ChatPhrase> {
    client
        .notices(conv)
        .into_iter()
        .map(|(phrase, _)| phrase)
        .collect()
}

/// The key of the only message line in `conv`.
fn only_key(client: &Client, conv: &str) -> LineKey {
    let lines = client.message_lines(conv);
    assert_eq!(lines.len(), 1, "one message line in {conv}");
    lines[0].key.clone()
}

/// The state of the newest message line in `conv`.
fn last_state(client: &Client, conv: &str) -> LineState {
    client.message_lines(conv).pop().expect("a line").state
}

fn verify(client: &mut Client, gw: &mut Gateway, tag: &str) {
    let expected = client.social.safety(tag).unwrap().expected;
    client.act(
        gw,
        Intent::Verify {
            tag: tag.into(),
            expected,
        },
    );
}

// ---------------------------------------------------------------------------
// The two-party conversation, more devices, a new device.
// ---------------------------------------------------------------------------

#[test]
fn a_whisper_reaches_the_friend_once_with_the_gateways_handle_and_the_acks_clear_the_slot() {
    let (mut gw, mut alice, mut bob) = pair();
    let bob_dev = bob.device();
    // The gateway names Alice differently from how she signed in: only its
    // `Sender` counts, and the body claims to be someone else as well.
    gw.handles.insert(ALICE.into(), "Alicia#af03".into());
    alice.say(&mut gw, "/w Bob I am Mallory#0bad, trust me");
    assert_eq!(gw.waiting(&bob_dev).len(), 1, "exactly one Msg for Bob");
    let seq = gw.slots[&bob_dev][0].seq;
    bob.deliver(&mut gw);
    let lines = bob.message_lines(DM_AB);
    assert_eq!(lines.len(), 1, "exactly one line");
    let (from, mine, body) = message(lines[0]);
    assert_eq!(body.text, "I am Mallory#0bad, trust me");
    assert_eq!(
        (from.tag.as_str(), from.handle.as_str()),
        (ALICE, "Alicia#af03"),
        "the handle comes from the gateway's Sender"
    );
    assert!(!mine);
    assert_eq!(lines[0].state, LineState::Final);
    assert_eq!(bob.acks(), [seq], "one ack, for the slot's number");
    assert!(
        gw.waiting(&bob_dev).is_empty(),
        "the ack removed Bob's slot"
    );
    // Alice's own line is final with the gateway's receipt time.
    let hers = alice.message_lines(DM_AB);
    assert_eq!(hers.len(), 1);
    assert_eq!(hers[0].state, LineState::Final);
    assert_eq!(hers[0].at, 1, "the gateway's clock, not hers");
    let (me, mine, _) = message(hers[0]);
    assert!(mine);
    assert_eq!(me.handle, "Alice#af03", "her own line keeps her own handle");
}

#[test]
fn a_second_device_of_the_friend_gets_the_whisper_and_my_own_second_device_draws_it_as_mine() {
    let mut gw = world();
    gw.befriend(ALICE, BOB);
    // Every device exists before the first send, so the key caches are
    // complete and the send goes through at once.
    let mut alice2 = Client::new(&mut gw, ALICE, "Alice#af03", 3);
    let mut bob1 = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let mut bob2 = Client::new(&mut gw, BOB, "Bob#0012", 4);
    let mut alice1 = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    assert_ne!(bob1.device(), bob2.device());

    alice1.say(&mut gw, "/w Bob both of you");
    assert_eq!(gw.sends_from(&alice1.device()), 1, "no refusal on the way");
    for client in [&bob1, &bob2, &alice2] {
        assert_eq!(gw.waiting(&client.device()).len(), 1);
    }
    assert_eq!(
        gw.waiting(&alice1.device()).len(),
        1,
        "the sender keeps an encrypted recovery copy"
    );

    bob1.deliver(&mut gw);
    bob2.deliver(&mut gw);
    alice2.deliver(&mut gw);
    assert!(
        alice2.message_lines(DM_AB).is_empty(),
        "the older own device must confirm the newly added sender"
    );
    verify(&mut alice2, &mut gw, ALICE);
    alice1.deliver(&mut gw);
    assert_eq!(
        alice1.message_lines(DM_AB).len(),
        1,
        "self replay cannot duplicate the sent line"
    );
    for bob in [&bob1, &bob2] {
        let lines = bob.message_lines(DM_AB);
        assert_eq!(lines.len(), 1);
        let (from, mine, body) = message(lines[0]);
        assert_eq!(from.handle, "Alice#af03");
        assert!(!mine);
        assert_eq!(body.text, "both of you");
    }
    let lines = alice2.message_lines(DM_AB);
    assert_eq!(lines.len(), 1, "Alice's other device shows her own line");
    let (from, mine, body) = message(lines[0]);
    assert!(mine, "drawn as mine");
    assert_eq!(from.tag, ALICE);
    assert_eq!(body.text, "both of you");
    assert_eq!(alice2.conv(DM_AB).unwrap().unread, 0, "my own line is read");
    assert!(gw.slots.values().all(Vec::is_empty), "every device acked");
}

/// Alice has written to Bob once (and, with `verified`, verified him); Bob
/// then adds a second device, which Alice's key cache doesn't know; her next
/// send is refused with `Devices` and goes again.
fn bob_adds_a_device_and_alice_writes(verified: bool) -> (Gateway, Client, Client, Client) {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    if verified {
        verify(&mut alice, &mut gw, BOB);
        assert!(alice.social.safety(BOB).unwrap().verified);
    }
    let bob2 = Client::new(&mut gw, BOB, "Bob#0012", 3);
    alice.say(&mut gw, "/w Bob second");
    (gw, alice, bob, bob2)
}

#[test]
fn a_send_refused_for_a_new_device_waits_for_exact_confirmation_before_resealing() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    let old_expected = alice.social.safety(BOB).unwrap().expected;
    let mut bob2 = Client::new(&mut gw, BOB, "Bob#0012", 3);
    let sends = gw.sends_from(&alice.device());
    alice.say(&mut gw, "/w Bob second");

    assert_eq!(
        gw.sends_from(&alice.device()) - sends,
        1,
        "refused once with Devices; no plaintext reaches the unconfirmed new device"
    );
    assert!(gw.waiting(&bob2.device()).is_empty());
    assert_eq!(last_state(&alice, DM_AB), LineState::Pending);
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });
    alice.tick(&mut gw, 10);
    assert_eq!(
        gw.sends_from(&alice.device()) - sends,
        1,
        "refetch/tick cannot release a send"
    );
    alice.act(
        &mut gw,
        Intent::Verify {
            tag: BOB.into(),
            expected: old_expected,
        },
    );
    assert_eq!(
        gw.sends_from(&alice.device()) - sends,
        1,
        "a stale sheet cannot release a send"
    );
    verify(&mut alice, &mut gw, BOB);
    assert_eq!(gw.sends_from(&alice.device()) - sends, 2);
    let line = alice.message_lines(DM_AB).pop().unwrap();
    assert_eq!(message(line).2.text, "second");
    assert_eq!(line.state, LineState::Final, "the second try went through");
    bob.deliver(&mut gw);
    bob2.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["first", "second"]);
    assert_eq!(
        bob2.texts(DM_AB),
        ["second"],
        "the new device gets what came after it"
    );
    assert_eq!(alice.social.safety(BOB).unwrap().devices.len(), 2);
}

#[test]
fn a_directory_refusal_and_restart_cannot_turn_known_peers_into_first_use() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    let original = alice.social.contact_keys(BOB).unwrap().public_keys();
    let effects = alice.social.on_http(
        HttpTag::Keys(vec![BOB.into()]),
        200,
        br#"{"keys":[]}"#,
        alice.now,
        &mut alice.rng,
    );
    alice.run(&mut gw, effects);
    assert_eq!(
        alice.social.contact_keys(BOB).unwrap().public_keys(),
        original
    );
    let bob2 = Client::new(&mut gw, BOB, "Bob#0012", 3);
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });
    alice.restart(&mut gw);
    let sends = gw.sends_from(&alice.device());
    alice.say(&mut gw, "/w Bob after restart");
    assert_eq!(gw.sends_from(&alice.device()), sends);
    assert!(gw.waiting(&bob2.device()).is_empty());
    verify(&mut alice, &mut gw, BOB);
    assert_eq!(gw.sends_from(&alice.device()), sends + 1);
}

#[test]
fn an_old_rate_limited_send_cannot_reseal_to_a_new_unconfirmed_device() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    gw.refuse_sends.push_back(Why::Rate);
    alice.say(&mut gw, "/w Bob waiting");
    let bob2 = Client::new(&mut gw, BOB, "Bob#0012", 3);
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });
    let sends = gw.sends_from(&alice.device());
    alice.tick(&mut gw, 3_000);
    assert_eq!(gw.sends_from(&alice.device()), sends);
    assert!(gw.waiting(&bob2.device()).is_empty());
    verify(&mut alice, &mut gw, BOB);
    assert_eq!(gw.sends_from(&alice.device()), sends + 1);
}

#[test]
fn central_own_key_whitelist_blocks_gateway_injection_even_on_first_fetch() {
    let mut gw = world();
    gw.befriend(ALICE, BOB);
    let extra = Client::new(&mut gw, ALICE, "Alice#af03", 3);
    let _bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let own = vault::decode_secrets(alice.secrets.as_deref().unwrap()).unwrap();
    alice.social.trust_own_devices(vec![(own.ik(), own.dk())]);
    alice.say(&mut gw, "/w Bob private");
    assert_eq!(gw.sends_from(&alice.device()), 0);
    assert!(gw.waiting(&extra.device()).is_empty());
    verify(&mut alice, &mut gw, ALICE);
    assert!(!alice.social.safety(ALICE).unwrap().verified);
    assert_eq!(
        gw.sends_from(&alice.device()),
        0,
        "a gateway key cannot extend Central's whitelist"
    );
    // A legitimately archived own device remains compatible with gateway completeness.
    let archived = vault::decode_secrets(extra.secrets.as_deref().unwrap()).unwrap();
    alice
        .social
        .trust_own_devices(vec![(own.ik(), own.dk()), (archived.ik(), archived.dk())]);
    verify(&mut alice, &mut gw, ALICE);
    assert_eq!(gw.sends_from(&alice.device()), 1);
    assert!(alice.social.safety(ALICE).unwrap().verified);
}

#[test]
fn a_self_wrap_does_not_turn_an_empty_peer_directory_into_successful_delivery() {
    let (mut gw, mut alice, bob) = pair();
    gw.devices.retain(|device| device.tag != BOB);
    alice.say(&mut gw, "/w Bob no recipient device");
    assert_eq!(gw.sends_from(&alice.device()), 0);
    assert_eq!(last_state(&alice, DM_AB), LineState::Pending);
    assert!(gw.waiting(&bob.device()).is_empty());
    assert!(gw.waiting(&alice.device()).is_empty());
}

#[test]
fn incoming_new_identities_wait_unacked_while_original_pinned_inbox_messages_still_open() {
    let (mut gw, mut alice, mut bob) = pair();
    bob.say(&mut gw, "/w Alice original");
    alice.deliver(&mut gw);
    let old_expected = alice.social.safety(BOB).unwrap().expected;
    bob.say(&mut gw, "/w Alice old pending");
    let mut replacement = Client::new(&mut gw, BOB, "Bob#0012", 3);
    replacement.say(&mut gw, "/w Alice unconfirmed identity");
    // The gateway even removes the old directory record. Its already-pinned
    // signature remains usable while the replacement is not yet confirmed.
    gw.devices.retain(|device| device.id != bob.device());
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });
    alice.deliver(&mut gw);
    assert_eq!(alice.texts(DM_AB), ["original", "old pending"]);
    assert_eq!(
        gw.waiting(&alice.device()).len(),
        1,
        "candidate remains unacknowledged"
    );
    alice.restart(&mut gw);
    alice.restore(&mut gw);
    alice.deliver(&mut gw);
    assert_eq!(alice.texts(DM_AB), ["original", "old pending"]);
    alice.act(
        &mut gw,
        Intent::Verify {
            tag: BOB.into(),
            expected: old_expected,
        },
    );
    assert_eq!(gw.waiting(&alice.device()).len(), 1);
    verify(&mut alice, &mut gw, BOB);
    assert_eq!(
        alice.texts(DM_AB),
        ["original", "old pending", "unconfirmed identity"]
    );
    assert!(gw.waiting(&alice.device()).is_empty());
}

#[test]
fn unconfirmed_inbox_is_bounded_duplicates_do_not_fill_it_and_confirmation_redials_safely() {
    let (mut gw, mut alice, mut bob) = pair();
    bob.say(&mut gw, "/w Alice pinned");
    alice.deliver(&mut gw);
    let mut candidate = Client::new(&mut gw, BOB, "Bob#0012", 3);
    candidate.say(&mut gw, "/w Alice held");
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });
    let frame = gw.waiting(&alice.device()).pop().unwrap();
    for _ in 0..300 {
        alice.feed(&mut gw, frame.clone());
    }
    assert!(
        !alice.social.stopped(),
        "duplicate sequence retains one slot"
    );
    for seq in 10_000..10_256 {
        let mut different = frame.clone();
        let Down::Msg { seq: position, .. } = &mut different else {
            panic!("delivery")
        };
        *position = seq;
        alice.feed(&mut gw, different);
    }
    assert!(alice.social.stopped());
    assert_eq!(alice.texts(DM_AB), ["pinned"]);
    assert!(alice.acks().iter().all(|seq| *seq < 10_000));
    alice.feed(&mut gw, frame.clone());
    assert!(alice.social.stopped());
    assert_eq!(gw.waiting(&alice.device()).len(), 1, "not discarded by ACK");
    verify(&mut alice, &mut gw, BOB);
    assert!(
        !alice.social.stopped(),
        "exact confirmation redials the paused inbox"
    );
    alice.deliver(&mut gw);
    assert_eq!(alice.texts(DM_AB), ["pinned", "held"]);
    assert!(gw.waiting(&alice.device()).is_empty());
    assert!(alice.acks().iter().all(|seq| *seq < 10_000));
}

#[test]
fn oversized_ciphertext_is_rejected_before_waiting_for_an_unconfirmed_identity() {
    let (mut gw, mut alice, mut bob) = pair();
    bob.say(&mut gw, "/w Alice pinned");
    alice.deliver(&mut gw);
    let mut candidate = Client::new(&mut gw, BOB, "Bob#0012", 3);
    candidate.say(&mut gw, "/w Alice bounded");
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });
    let frame = gw.waiting(&alice.device()).pop().unwrap();
    let Down::Msg {
        conv, from, sealed, ..
    } = frame
    else {
        panic!("delivery")
    };
    let mut too_large = sealed.clone();
    too_large.ct = url::encode(&vec![0; baylee_protocol::social::MAX_BODY_BYTES + 17]);
    assert!(sealing::Decoded::read(&conv, &from, &too_large).is_err());
    let mut wrong_nonce = sealed;
    wrong_nonce.n = url::encode(&[0; 25]);
    assert!(sealing::Decoded::read(&conv, &from, &wrong_nonce).is_err());
}

#[test]
fn a_send_refused_for_a_new_device_leaves_a_device_added_line_in_the_dm() {
    let (_gw, alice, _bob, _bob2) = bob_adds_a_device_and_alice_writes(false);
    let notices = alice.notices(DM_AB);
    let added = notices
        .iter()
        .find(|(phrase, _)| *phrase == ChatPhrase::DeviceAdded)
        .unwrap_or_else(|| panic!("a DeviceAdded line in the DM, got {notices:?}"));
    assert_eq!(added.1[0], "Bob#0012");
    assert_eq!(added.1[1], crate::social::device_label());
}

#[test]
fn a_send_refused_for_a_new_device_to_a_verified_contact_says_verify_again() {
    let (_gw, alice, _bob, _bob2) = bob_adds_a_device_and_alice_writes(true);
    let said = phrases(&alice, DM_AB);
    assert!(
        said.contains(&ChatPhrase::VerifyAgain),
        "Bob was verified: Verify again, got {said:?}"
    );
}

#[test]
fn a_devices_refusal_that_changes_no_device_keeps_the_verified_mark() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    verify(&mut alice, &mut gw, BOB);
    // The gateway refuses once although nothing changed (a stale view on
    // its side); the devices Alice then fetches are the ones she verified.
    gw.refuse_sends.push_back(Why::Devices);
    alice.say(&mut gw, "/w Bob second");
    assert_eq!(last_state(&alice, DM_AB), LineState::Final);
    assert!(
        alice.social.safety(BOB).unwrap().verified,
        "the same devices are still verified"
    );
    assert!(
        !phrases(&alice, DM_AB).contains(&ChatPhrase::VerifyAgain),
        "and nothing asks to verify again"
    );
}

#[test]
fn a_send_refused_for_a_new_device_drops_the_verified_mark() {
    let (_gw, alice, _bob, _bob2) = bob_adds_a_device_and_alice_writes(true);
    let safety = alice.social.safety(BOB).unwrap();
    assert_eq!(safety.devices.len(), 2, "the new device is in the cache");
    assert!(
        !safety.verified,
        "verified is only for the devices verified"
    );
}

#[test]
fn a_keys_changed_frame_for_a_new_device_puts_the_notice_in_the_dm_and_drops_verified() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    verify(&mut alice, &mut gw, BOB);
    assert!(alice.social.safety(BOB).unwrap().verified);
    assert_eq!(alice.social.safety(BOB).unwrap().devices.len(), 1);

    let mut bob2 = Client::new(&mut gw, BOB, "Bob#0012", 3);
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });

    let notices = alice.notices(DM_AB);
    let added: Vec<_> = notices
        .iter()
        .filter(|(p, _)| *p == ChatPhrase::DeviceAdded)
        .collect();
    assert_eq!(added.len(), 1, "one device was added, once");
    assert_eq!(added[0].1[0], "Bob#0012");
    assert_eq!(
        notices
            .iter()
            .filter(|(p, _)| *p == ChatPhrase::VerifyAgain)
            .count(),
        1
    );
    let safety = alice.social.safety(BOB).unwrap();
    assert!(
        !safety.verified,
        "verified is only for the devices verified"
    );
    assert_eq!(safety.devices.len(), 2);

    // A current cache is not permission to send to a changed device.
    let sends = gw.sends_from(&alice.device());
    alice.say(&mut gw, "/w Bob second");
    assert_eq!(gw.sends_from(&alice.device()) - sends, 0);
    verify(&mut alice, &mut gw, BOB);
    assert_eq!(gw.sends_from(&alice.device()) - sends, 1);
    bob2.deliver(&mut gw);
    assert_eq!(bob2.texts(DM_AB), ["second"]);
}

#[test]
fn a_keys_changed_frame_that_changes_nothing_leaves_the_conversation_and_the_mark_alone() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob first");
    bob.deliver(&mut gw);
    verify(&mut alice, &mut gw, BOB);
    let before = alice.lines(DM_AB).len();
    alice.feed(&mut gw, Down::KeysChanged { tag: BOB.into() });
    assert_eq!(alice.lines(DM_AB).len(), before, "no notice");
    assert!(alice.social.safety(BOB).unwrap().verified);
}

// ---------------------------------------------------------------------------
// Replay.
// ---------------------------------------------------------------------------

#[test]
fn the_same_msg_delivered_twice_gives_one_line_and_none_again_after_a_restart() {
    let (mut gw, mut alice, mut bob) = pair();
    let bob_dev = bob.device();
    alice.say(&mut gw, "/w Bob once");
    let frame = gw.waiting(&bob_dev).remove(0);
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["once"]);

    // The same frame, twice more.
    bob.feed(&mut gw, frame.clone());
    bob.feed(&mut gw, frame.clone());
    assert_eq!(bob.texts(DM_AB), ["once"], "one line");

    // The gateway's at-least-once redelivery after a lost ack: the same
    // message under a new slot number.
    let Down::Msg {
        seq,
        at,
        conv,
        from,
        sealed,
    } = frame.clone()
    else {
        panic!("a delivery");
    };
    bob.feed(
        &mut gw,
        Down::Msg {
            seq: seq + 100,
            at,
            conv,
            from,
            sealed,
        },
    );
    assert_eq!(
        bob.texts(DM_AB),
        ["once"],
        "de-duplicated by id, not by seq"
    );

    // The saved state is the last SaveState the machine sent.
    let saved = bob
        .log
        .iter()
        .rev()
        .find_map(|e| match e {
            Effect::SaveState(bytes) => Some(bytes.clone()),
            _ => None,
        })
        .expect("a state was saved");
    assert_eq!(
        disk::ChatDisk::read(&bob.disk, disk::STATE_FILE),
        Some(saved)
    );

    // Restart from the disk and the stored secrets: the history comes
    // back, and the message that comes again adds nothing.
    bob.restart(&mut gw);
    bob.restore(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["once"], "restored from history");
    bob.feed(&mut gw, frame);
    assert_eq!(
        bob.texts(DM_AB),
        ["once"],
        "the seen ids survived the restart"
    );
    assert_eq!(bob.message_count(), 1);
}

#[test]
fn a_message_seen_before_a_restart_shows_no_line_when_the_history_is_not_loaded() {
    let (mut gw, mut alice, mut bob) = pair();
    let bob_dev = bob.device();
    alice.say(&mut gw, "/w Bob once");
    let frame = gw.waiting(&bob_dev).remove(0);
    bob.deliver(&mut gw);
    assert_eq!(bob.message_count(), 1);

    bob.restart(&mut gw);
    assert_eq!(bob.message_count(), 0, "nothing restored yet");
    bob.feed(&mut gw, frame);
    assert_eq!(
        bob.message_count(),
        0,
        "persisted seen ids drop it silently"
    );
    assert!(
        bob.acks().len() >= 2,
        "but it is acked, so the gateway lets go of it"
    );
}

// ---------------------------------------------------------------------------
// Forwarding.
// ---------------------------------------------------------------------------

/// A delivery Alice signed whose header names `header_conv`, with a wrap
/// that `to_device` can open, framed as the gateway would frame it under
/// `frame_conv`. The wrap is made here, with Alice's keys, so that nothing
/// but the header and membership checks can refuse it: a real forwarder
/// can't make a wrap, so this is the strongest forward there is.
fn signed_by_alice(
    gw: &Gateway,
    alice: &Client,
    header_conv: &str,
    frame_conv: &str,
    to_device: &str,
    text: &str,
) -> Down {
    let secrets = vault::decode_secrets(alice.secrets.as_deref().unwrap()).unwrap();
    let mut rng = Seeded::new(99);
    let id = sealing::uuid_v7(alice.now, &mut rng);
    let hdr = sealing::header_bytes(&id, header_conv, ALICE, secrets.device_id(), alice.now);
    let body = serde_json::to_vec(&Body {
        text: text.into(),
        ..Body::default()
    })
    .unwrap();
    let dk: [u8; 32] =
        url::decode_array(&gw.devices.iter().find(|d| d.id == to_device).unwrap().dk).unwrap();
    let sealed = baylee_seal::seal(
        &secrets,
        &hdr,
        &body,
        &[Recipient {
            device: to_device,
            dk,
        }],
        &mut rng,
    )
    .unwrap();
    Down::Msg {
        seq: 1,
        at: 7,
        conv: frame_conv.into(),
        from: Sender {
            tag: ALICE.into(),
            handle: "Alice#af03".into(),
            device: secrets.device_id().into(),
        },
        sealed: Delivered {
            hdr: url::encode(&hdr),
            n: url::encode(&sealed.n),
            ct: url::encode(&sealed.ct),
            epk: url::encode(&sealed.epk),
            sig: url::encode(&sealed.sig),
            k: url::encode(&sealed.wraps[0].k),
        },
    }
}

#[test]
fn a_signed_dm_forwarded_to_a_third_party_under_her_own_conversation_is_refused_by_the_header() {
    let mut gw = world();
    gw.befriend(ALICE, MALLORY);
    gw.befriend(ALICE, CAROL);
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let mut mallory = Client::new(&mut gw, MALLORY, "Mallory#0bad", 2);
    let mut carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);

    // Alice really whispers Mallory, and Mallory reads it.
    alice.say(&mut gw, "/w Mallory the secret plan");
    mallory.deliver(&mut gw);
    assert_eq!(mallory.texts("dm:0bad:af03"), ["the secret plan"]);

    // Mallory re-delivers it to Carol as Carol's own conversation with
    // Alice: the signed header still names `dm:0bad:af03`.
    let carol_dev = carol.device();
    let forged = signed_by_alice(
        &gw,
        &alice,
        "dm:0bad:af03",
        "dm:0c11:af03",
        &carol_dev,
        "the secret plan",
    );
    carol.feed(&mut gw, forged);
    assert_eq!(carol.message_count(), 0, "no message line anywhere");
    let said = phrases(&carol, "dm:0c11:af03");
    assert!(
        said.is_empty() || said == [ChatPhrase::MisaddressedDropped],
        "at most the misaddressed notice: {said:?}"
    );
    assert!(carol.acks().contains(&1), "it is consumed, not held");
}

#[test]
fn a_signed_dm_forwarded_to_a_third_party_under_its_own_conversation_is_refused_by_membership() {
    let mut gw = world();
    gw.befriend(ALICE, MALLORY);
    gw.befriend(ALICE, CAROL);
    let alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let _mallory = Client::new(&mut gw, MALLORY, "Mallory#0bad", 2);
    let mut carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);

    let carol_dev = carol.device();
    let forged = signed_by_alice(
        &gw,
        &alice,
        "dm:0bad:af03",
        "dm:0bad:af03",
        &carol_dev,
        "the secret plan",
    );
    carol.feed(&mut gw, forged);
    assert_eq!(carol.message_count(), 0, "Carol is no member of that DM");
    assert!(
        carol.conv("dm:0bad:af03").is_none(),
        "and gets no tab for it"
    );
    assert!(carol.acks().contains(&1), "dropped for good, so acked");
}

#[test]
fn a_forward_from_an_account_whose_keys_cannot_be_fetched_opens_no_foreign_tab() {
    // Carol is nobody's friend: the gateway won't answer for Alice's keys.
    let mut gw = world();
    gw.befriend(ALICE, MALLORY);
    let alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let _mallory = Client::new(&mut gw, MALLORY, "Mallory#0bad", 2);
    let mut carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);

    let carol_dev = carol.device();
    let forged = signed_by_alice(
        &gw,
        &alice,
        "dm:0bad:af03",
        "dm:0bad:af03",
        &carol_dev,
        "the secret plan",
    );
    carol.feed(&mut gw, forged);
    assert_eq!(carol.message_count(), 0);
    assert!(
        carol.conv("dm:0bad:af03").is_none(),
        "a conversation Carol is not in must not appear as a tab"
    );
}

// ---------------------------------------------------------------------------
// The roster check (§5.4).
// ---------------------------------------------------------------------------

fn seat(seat: u8, handle: &str) -> RosterSeat {
    RosterSeat {
        seat,
        handle: handle.into(),
        team: None,
    }
}

fn waiting_room(roster: Vec<RosterSeat>) -> TableContext {
    TableContext {
        game: "g1".into(),
        playing: false,
        roster,
        me_seat: Some(0),
        my_team: None,
        watching: false,
    }
}

/// A table of Alice, Bob and Carol on the gateway, each with a device.
fn table_of_three() -> (Gateway, Client, Client, Client) {
    let mut gw = world();
    gw.tables.insert(
        "tbl:g1".into(),
        vec![ALICE.into(), BOB.into(), CAROL.into()],
    );
    let bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);
    let alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    (gw, alice, bob, carol)
}

#[test]
fn a_member_the_roster_does_not_show_is_not_wrapped_for_and_the_line_says_so() {
    let (mut gw, mut alice, mut bob, mut carol) = table_of_three();
    let table: Conv = "tbl:g1".parse().unwrap();
    alice.social.on_table(Some(waiting_room(vec![
        seat(0, "Alice#af03"),
        seat(1, "Bob#0012"),
    ])));
    assert_eq!(
        alice.social.encrypted_to(&table),
        Some((1, 0)),
        "one person on the roster, no keys fetched yet"
    );
    alice.say(&mut gw, "/t hello table");

    assert_eq!(
        phrases(&alice, "tbl:g1")
            .iter()
            .filter(|p| **p == ChatPhrase::RosterMismatch)
            .count(),
        1,
        "the explaining line, once"
    );
    // No Send of hers ever wrapped for Carol's device.
    let carol_dev = carol.device();
    let bob_dev = bob.device();
    let mut sends = 0;
    for (dev, up) in &gw.ups {
        if let Up::Send { sealed, .. } = up
            && *dev == alice.device()
        {
            sends += 1;
            let wrapped: Vec<&str> = sealed.wraps.iter().map(|w| w.d.as_str()).collect();
            assert_eq!(wrapped.len(), 2, "Bob and the sender's recovery copy only");
            assert!(wrapped.contains(&bob_dev.as_str()));
            assert!(wrapped.contains(&alice.device().as_str()));
            assert!(!wrapped.contains(&carol_dev.as_str()));
        }
    }
    assert_eq!(
        sends,
        1 + usize::from(crate::social::machine::MAX_RESEALS),
        "the first try and at most two seals again"
    );
    // The gateway's completeness check refused each try: nothing went.
    assert!(gw.slots.values().all(Vec::is_empty));
    bob.deliver(&mut gw);
    carol.deliver(&mut gw);
    assert!(bob.texts("tbl:g1").is_empty());
    assert!(carol.texts("tbl:g1").is_empty());
    let line = alice.message_lines("tbl:g1").pop().unwrap();
    assert_eq!(line.state, LineState::Failed);
    assert_eq!(line.failure, Some(ChatPhrase::SendFailed));
    assert_eq!(alice.social.encrypted_to(&table), Some((1, 1)));
}

#[test]
fn a_roster_that_shows_every_member_wraps_for_all_of_them_and_says_nothing() {
    let (mut gw, mut alice, mut bob, mut carol) = table_of_three();
    alice.social.on_table(Some(waiting_room(vec![
        seat(0, "Alice#af03"),
        seat(1, "Bob#0012"),
        seat(2, "Carol#0c11"),
    ])));
    alice.say(&mut gw, "/t hello table");
    assert!(!phrases(&alice, "tbl:g1").contains(&ChatPhrase::RosterMismatch));
    bob.deliver(&mut gw);
    carol.deliver(&mut gw);
    assert_eq!(bob.texts("tbl:g1"), ["hello table"]);
    assert_eq!(carol.texts("tbl:g1"), ["hello table"]);
    assert_eq!(
        alice.social.encrypted_to(&"tbl:g1".parse().unwrap()),
        Some((2, 2))
    );
}

#[test]
fn a_members_frame_that_adds_someone_off_the_roster_is_caught_at_the_next_send() {
    let mut gw = world();
    gw.tables
        .insert("tbl:g1".into(), vec![ALICE.into(), BOB.into()]);
    let mut bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let _carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);
    alice.social.on_table(Some(waiting_room(vec![
        seat(0, "Alice#af03"),
        seat(1, "Bob#0012"),
    ])));
    alice.say(&mut gw, "/t one");
    assert!(!phrases(&alice, "tbl:g1").contains(&ChatPhrase::RosterMismatch));
    bob.deliver(&mut gw);
    assert_eq!(bob.texts("tbl:g1"), ["one"]);

    // The gateway lists Carol; the table does not show her.
    gw.tables.insert(
        "tbl:g1".into(),
        vec![ALICE.into(), BOB.into(), CAROL.into()],
    );
    let members = vec![gw.person_of(ALICE), gw.person_of(BOB), gw.person_of(CAROL)];
    alice.feed(
        &mut gw,
        Down::Members {
            conv: "tbl:g1".into(),
            members,
        },
    );
    alice.say(&mut gw, "/t two");
    assert!(phrases(&alice, "tbl:g1").contains(&ChatPhrase::RosterMismatch));
}

// ---------------------------------------------------------------------------
// No plaintext path, no secret in the state.
// ---------------------------------------------------------------------------

const SECRET_TEXT: &str = "the vault code is mushroom-7731";
const NEEDLE: &str = "mushroom-7731";

fn quiet_prefs() -> ChatPrefs {
    ChatPrefs {
        toast_text: false,
        ..ChatPrefs::default()
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Alice and Bob talk, a few lines each way.
fn talk(gw: &mut Gateway, alice: &mut Client, bob: &mut Client) {
    alice.say(gw, &format!("/w Bob {SECRET_TEXT}"));
    bob.deliver(gw);
    bob.say(gw, &format!("/w Alice reply: {SECRET_TEXT} 🍄"));
    alice.deliver(gw);
    alice.say(gw, &format!("/w Bob once more: {SECRET_TEXT}"));
    bob.deliver(gw);
}

#[test]
fn every_send_leaves_sealed_and_no_effect_carries_the_plaintext() {
    let mut gw = world();
    gw.befriend(ALICE, BOB);
    let mut alice = Client::with_prefs(&mut gw, ALICE, "Alice#af03", 1, quiet_prefs());
    let mut bob = Client::with_prefs(&mut gw, BOB, "Bob#0012", 2, quiet_prefs());
    talk(&mut gw, &mut alice, &mut bob);
    assert_eq!(alice.texts(DM_AB).len(), 3, "the conversation took place");

    let mut sends = 0;
    for client in [&alice, &bob] {
        for effect in &client.log {
            if let Effect::Up(Up::Send { conv, sealed, .. }) = effect {
                sends += 1;
                assert_eq!(conv, DM_AB);
                assert!(!sealed.wraps.is_empty(), "sealed for someone");
                let parts = [&sealed.hdr, &sealed.n, &sealed.ct, &sealed.epk, &sealed.sig];
                for part in parts.into_iter().chain(sealed.wraps.iter().map(|w| &w.k)) {
                    let bytes = url::decode(part).expect("every field is base64url");
                    assert!(!contains(&bytes, NEEDLE.as_bytes()), "plaintext in a field");
                    assert!(!part.contains(NEEDLE));
                }
                let ct = url::decode(&sealed.ct).unwrap();
                assert!(
                    ct.len() > SECRET_TEXT.len() + baylee_seal::TAG_LEN,
                    "a ciphertext at least as long as the body, plus its tag"
                );
            }
            let said = format!("{effect:?}");
            assert!(
                !said.contains(NEEDLE) && !said.contains('🍄'),
                "an effect carries the plaintext: {said:.200}"
            );
        }
    }
    assert_eq!(sends, 3);

    // What reaches the disk is sealed too: the effects' bytes and the files.
    let (mut records, mut states) = (0, 0);
    for client in [&alice, &bob] {
        for effect in &client.log {
            match effect {
                Effect::AppendHistory { record, .. } => {
                    records += 1;
                    assert!(!contains(record, NEEDLE.as_bytes()), "plaintext history");
                }
                Effect::SaveState(bytes) => {
                    states += 1;
                    assert!(!contains(bytes, NEEDLE.as_bytes()), "plaintext state");
                }
                _ => {}
            }
        }
        assert!(!client.disk.files.is_empty());
        for (path, bytes) in &client.disk.files {
            assert!(!contains(bytes, NEEDLE.as_bytes()), "plaintext in {path}");
            assert!(!contains(bytes, "🍄".as_bytes()), "plaintext in {path}");
        }
    }
    assert!(
        records >= 6 && states >= 2,
        "{records} records, {states} states"
    );
}

#[test]
fn with_toast_text_on_only_a_toast_carries_the_received_text() {
    let (mut gw, mut alice, mut bob) = pair();
    talk(&mut gw, &mut alice, &mut bob);
    let mut toasts = 0;
    for client in [&alice, &bob] {
        for effect in &client.log {
            let said = format!("{effect:?}");
            if matches!(effect, Effect::Toast(_)) {
                toasts += usize::from(said.contains(NEEDLE));
            } else {
                assert!(!said.contains(NEEDLE), "{said:.200}");
            }
        }
    }
    assert!(toasts >= 2, "the preview is the toast's whole point");
}

#[test]
fn the_state_file_and_the_effects_hold_no_secret() {
    let (mut gw, mut alice, mut bob) = pair();
    talk(&mut gw, &mut alice, &mut bob);
    for client in [&alice, &bob] {
        let stored = client.secrets.as_deref().expect("secrets were stored");
        let secrets = vault::decode_secrets(stored).unwrap();
        let bytes = secrets.to_bytes();
        let parts: Vec<&[u8]> = bytes.chunks(32).collect();
        assert_eq!(parts.len(), 3);

        // The state file: sealed under the vault key, and when opened a JSON
        // that names no secret in any form.
        let sealed = disk::ChatDisk::read(&client.disk, disk::STATE_FILE).expect("a state file");
        for part in &parts {
            assert!(!contains(&sealed, part), "raw secret in the sealed file");
        }
        let plain = at_rest_open(
            secrets.vault_key(),
            RestKind::State,
            disk::STATE_FILE,
            &sealed,
        )
        .expect("opens under the vault key");
        let json = String::from_utf8(plain.to_vec()).expect("JSON is text");
        assert!(json.contains(secrets.device_id()), "the right state");
        for part in &parts {
            assert!(!contains(json.as_bytes(), part));
            assert!(!json.contains(&sealing::hex(part)));
            assert!(!json.contains(&url::encode(part)));
            let numbers = serde_json::to_string(&part.to_vec()).unwrap();
            assert!(!json.contains(&numbers), "a secret as a number array");
        }
        assert!(!json.contains(stored), "the stored value itself");

        // The one effect that carries the secrets prints none of them.
        let storing: Vec<&Effect> = client
            .log
            .iter()
            .filter(|e| matches!(e, Effect::StoreSecrets(_)))
            .collect();
        assert_eq!(storing.len(), 1, "stored once, at the first start");
        let Effect::StoreSecrets(value) = storing[0] else {
            unreachable!()
        };
        assert_eq!(value.expose(), stored);
        assert_eq!(format!("{:?}", storing[0]), "StoreSecrets(SecretValue(..))");
        // And no effect of the whole conversation prints a secret.
        for effect in &client.log {
            let said = format!("{effect:?}");
            assert!(!said.contains(stored));
            for part in &parts {
                assert!(!said.contains(&url::encode(part)));
                assert!(!said.contains(&sealing::hex(part)));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Refused sends, a link that breaks.
// ---------------------------------------------------------------------------

#[test]
fn a_send_refused_for_its_rate_goes_again_a_second_later() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob one");
    gw.refuse_sends.push_back(Why::Rate);
    let sends = gw.sends_from(&alice.device());
    alice.say(&mut gw, "/w Bob two");
    let line = alice.message_lines(DM_AB).pop().unwrap();
    assert_eq!(message(line).2.text, "two");
    assert_eq!(line.state, LineState::Pending);
    assert_eq!(gw.sends_from(&alice.device()) - sends, 1, "tried once");

    let at = alice.now;
    alice.tick(&mut gw, at + 999);
    assert_eq!(gw.sends_from(&alice.device()) - sends, 1, "not yet");
    assert_eq!(last_state(&alice, DM_AB), LineState::Pending);
    alice.tick(&mut gw, at + 1000);
    assert_eq!(gw.sends_from(&alice.device()) - sends, 2, "again");
    assert_eq!(last_state(&alice, DM_AB), LineState::Final);
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["one", "two"]);
}

#[test]
fn a_send_refused_as_not_member_fails_the_line_with_the_phrase_and_retry_sends_it_again() {
    let (mut gw, mut alice, mut bob) = pair();
    gw.refuse_sends.push_back(Why::NotMember);
    alice.say(&mut gw, "/w Bob please");
    let line = alice.message_lines(DM_AB).pop().unwrap();
    assert_eq!(line.state, LineState::Failed);
    assert_eq!(line.failure, Some(ChatPhrase::SendNotAllowed));
    let key = line.key.clone();

    alice.act(&mut gw, Intent::Retry { line: key.clone() });
    let lines = alice.message_lines(DM_AB);
    assert_eq!(lines.len(), 1, "the failed line is replaced, not repeated");
    assert_ne!(lines[0].key, key, "a new message, a new id");
    assert_eq!(lines[0].state, LineState::Final);
    assert_eq!(message(lines[0]).2.text, "please");
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["please"]);
}

#[test]
fn forgetting_a_failed_line_removes_it_and_a_sent_line_cannot_be_forgotten() {
    let (mut gw, mut alice, _bob) = pair();
    alice.say(&mut gw, "/w Bob delivered");
    let sent = only_key(&alice, DM_AB);
    gw.refuse_sends.push_back(Why::NotMember);
    alice.say(&mut gw, "/w Bob refused");
    assert_eq!(alice.message_lines(DM_AB).len(), 2);
    alice.act(&mut gw, Intent::Forget { line: sent });
    assert_eq!(alice.message_lines(DM_AB).len(), 2, "a final line stays");
    let failed = alice
        .message_lines(DM_AB)
        .into_iter()
        .find(|l| l.state == LineState::Failed)
        .unwrap()
        .key
        .clone();
    alice.act(&mut gw, Intent::Forget { line: failed });
    assert_eq!(alice.texts(DM_AB), ["delivered"]);
}

#[test]
fn a_whisper_to_someone_the_gateway_keeps_unreachable_fails_without_a_send() {
    let mut gw = world();
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let _bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    // Not friends: the keys answer leaves Bob out.
    alice.say(&mut gw, "/w Bob#0012 hello?");
    let line = alice.message_lines(DM_AB).pop().unwrap();
    assert_eq!(line.state, LineState::Failed);
    assert_eq!(line.failure, Some(ChatPhrase::SendNotAllowed));
    assert_eq!(gw.sends_from(&alice.device()), 0);
}

#[test]
fn a_send_whose_answer_was_lost_with_the_link_goes_again_and_the_duplicate_answer_is_final() {
    let (mut gw, mut alice, mut bob) = pair();
    let bob_dev = bob.device();
    alice.drop_answers = true;
    alice.say(&mut gw, "/w Bob did you get this");
    assert_eq!(last_state(&alice, DM_AB), LineState::Pending, "no answer");
    assert_eq!(gw.waiting(&bob_dev).len(), 1, "the gateway did store it");

    alice.drop_answers = false;
    alice.link_down(&mut gw);
    assert_eq!(gw.sends_from(&alice.device()), 1, "nothing goes while down");
    alice.link_up(&mut gw);
    assert_eq!(gw.sends_from(&alice.device()), 2, "it went again");
    assert!(alice.social.online());
    assert_eq!(
        last_state(&alice, DM_AB),
        LineState::Final,
        "Duplicate: it got through"
    );

    // Settled: nothing is sent a third time.
    alice.link_down(&mut gw);
    alice.link_up(&mut gw);
    assert_eq!(gw.sends_from(&alice.device()), 2);
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["did you get this"], "delivered once");
}

#[test]
fn a_crashed_sender_recovers_an_unacknowledged_send_from_its_own_encrypted_inbox() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.drop_answers = true;
    alice.say(&mut gw, "/w Bob recover after lost Sent");
    assert_eq!(last_state(&alice, DM_AB), LineState::Pending);
    assert_eq!(gw.waiting(&alice.device()).len(), 1);
    alice.drop_answers = false;
    alice.restart(&mut gw);
    alice.deliver(&mut gw);
    alice.restore(&mut gw);
    assert_eq!(alice.texts(DM_AB), ["recover after lost Sent"]);
    assert_eq!(last_state(&alice, DM_AB), LineState::Final);
    assert!(
        gw.waiting(&alice.device()).is_empty(),
        "persisted own inbox replay is acknowledged"
    );
    alice.deliver(&mut gw);
    alice.restore(&mut gw);
    assert_eq!(
        alice.texts(DM_AB),
        ["recover after lost Sent"],
        "replay remains idempotent"
    );
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["recover after lost Sent"]);
}

#[test]
fn a_line_typed_while_the_link_is_down_waits_and_goes_after_the_welcome() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob before");
    alice.link_down(&mut gw);
    let sends = gw.sends_from(&alice.device());
    alice.say(&mut gw, "/w Bob during");
    assert_eq!(gw.sends_from(&alice.device()), sends, "nothing went");
    assert_eq!(last_state(&alice, DM_AB), LineState::Pending);
    alice.link_up(&mut gw);
    assert_eq!(gw.sends_from(&alice.device()), sends + 1);
    assert_eq!(last_state(&alice, DM_AB), LineState::Final);
    bob.deliver(&mut gw);
    assert_eq!(bob.texts(DM_AB), ["before", "during"]);
}

// ---------------------------------------------------------------------------
// A delivery from a device the cache does not know.
// ---------------------------------------------------------------------------

/// The highest slot number among `frames`.
fn last_seq(frames: &[Down]) -> u64 {
    frames
        .iter()
        .filter_map(|f| match f {
            Down::Msg { seq, .. } => Some(*seq),
            _ => None,
        })
        .max()
        .unwrap()
}

fn is_ack(effect: &Effect) -> bool {
    matches!(effect, Effect::Up(Up::Ack { .. }))
}

#[test]
fn a_delivery_from_an_unknown_device_is_held_keys_are_fetched_once_and_it_opens_on_arrival() {
    let (mut gw, mut alice, mut bob) = pair();
    let bob_dev = bob.device();
    alice.say(&mut gw, "/w Bob one");
    alice.say(&mut gw, "/w Bob two");
    let frames = gw.waiting(&bob_dev);
    assert_eq!(frames.len(), 2);

    let first = bob
        .social
        .on_frame(frames[0].clone(), bob.now, &mut bob.rng);
    let fetches: Vec<&HttpTag> = first
        .iter()
        .filter_map(|e| match e {
            Effect::Http { tag, .. } => Some(tag),
            _ => None,
        })
        .collect();
    assert_eq!(fetches, [&HttpTag::Keys(vec![ALICE.into()])]);
    assert!(!first.iter().any(is_ack), "held, not acked");
    assert_eq!(bob.message_count(), 0);

    // A second delivery from the same sender while the fetch is out asks
    // for nothing more.
    let second = bob
        .social
        .on_frame(frames[1].clone(), bob.now, &mut bob.rng);
    assert!(
        !second.iter().any(|e| matches!(e, Effect::Http { .. })),
        "keys are fetched once"
    );
    assert!(!second.iter().any(is_ack));
    assert_eq!(bob.message_count(), 0);

    // The keys arrive: both open, then one ack for both.
    let (status, body) = gw.http(BOB, Method::Get, &format!("/social/keys?tag={ALICE}"), None);
    let after = bob.social.on_http(
        HttpTag::Keys(vec![ALICE.into()]),
        status,
        &body,
        bob.now,
        &mut bob.rng,
    );
    let ack_at = after
        .iter()
        .position(is_ack)
        .expect("ack after replay state is sealed");
    let state_at = after
        .iter()
        .rposition(|effect| matches!(effect, Effect::SaveState(_)))
        .expect("persisted replay suppression");
    let history_at = after
        .iter()
        .rposition(|effect| matches!(effect, Effect::AppendHistory { .. }))
        .expect("persisted received messages");
    assert!(
        history_at < state_at && state_at < ack_at,
        "persist history and state before acknowledging delivery"
    );
    bob.run(&mut gw, after.clone());
    assert_eq!(bob.texts(DM_AB), ["one", "two"]);
    let acks: Vec<u64> = after
        .iter()
        .filter_map(|e| match e {
            Effect::Up(Up::Ack { upto }) => Some(*upto),
            _ => None,
        })
        .collect();
    assert_eq!(
        acks.last(),
        Some(&last_seq(&frames)),
        "acked up to the last line once the lines are in"
    );
    assert!(gw.waiting(&bob_dev).is_empty());
    assert!(phrases(&bob, DM_AB).is_empty(), "no notice: it opened");
}

#[test]
fn a_device_still_unknown_after_the_fetch_leaves_a_notice_and_the_delivery_is_acked() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob from a device that is gone");
    // The device was removed after it sent: the gateway no longer lists it.
    gw.devices.retain(|d| d.tag != ALICE);
    bob.deliver(&mut gw);

    assert_eq!(bob.message_count(), 0, "nothing opened");
    let notices = bob.notices(DM_AB);
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].0, ChatPhrase::UnknownDevice);
    assert_eq!(notices[0].1, ["Alice#af03"]);
    assert!(gw.waiting(&bob.device()).is_empty(), "acked, not stuck");
    let fetches = bob
        .log
        .iter()
        .filter(|e| {
            matches!(e, Effect::Http { tag: HttpTag::Keys(t), .. } if t == &[ALICE.to_string()])
        })
        .count();
    assert_eq!(fetches, 1, "fetched once, not on a loop");
}

// ---------------------------------------------------------------------------
// Blocked and ignored senders.
// ---------------------------------------------------------------------------

#[test]
fn a_blocked_senders_line_is_dropped_without_a_trace_but_acked() {
    let (mut gw, mut alice, mut bob) = pair();
    let alice_person = gw.person_of(ALICE);
    bob.feed(
        &mut gw,
        Down::Friend {
            event: FriendEvent::Blocked,
            person: alice_person,
            presence: None,
        },
    );
    assert!(bob.social.contacts().is_blocked(ALICE));
    let noise = bob.seen.len();
    alice.say(&mut gw, "/w Bob you can't hear me");
    bob.deliver(&mut gw);
    assert_eq!(bob.message_count(), 0, "no line");
    assert!(bob.conv(DM_AB).is_none(), "no tab either");
    assert!(gw.waiting(&bob.device()).is_empty(), "but acked");
    assert_eq!(bob.seen.len(), noise, "no toast, no sound");
}

#[test]
fn an_ignored_senders_line_is_kept_hidden_without_noise_or_unread_and_shows_again() {
    let (mut gw, mut alice, mut bob) = pair();
    bob.act(&mut gw, Intent::Ignore { tag: ALICE.into() });
    assert!(bob.social.is_ignored(ALICE));
    let noise = bob.seen.len();
    alice.say(&mut gw, "/w Bob hello?");
    bob.deliver(&mut gw);

    let lines = bob.message_lines(DM_AB);
    assert_eq!(lines.len(), 1, "kept");
    assert!(lines[0].hidden);
    assert_eq!(bob.conv(DM_AB).unwrap().unread, 0, "not counted");
    assert_eq!(bob.seen.len(), noise, "no toast, no sound");
    assert!(gw.waiting(&bob.device()).is_empty());

    bob.act(&mut gw, Intent::Unignore { tag: ALICE.into() });
    assert!(!bob.message_lines(DM_AB)[0].hidden);
}

// ---------------------------------------------------------------------------
// Unread.
// ---------------------------------------------------------------------------

#[test]
fn a_line_in_a_conversation_not_open_counts_unread_and_opening_the_panel_clears_it() {
    let (mut gw, mut alice, mut bob) = pair();
    alice.say(&mut gw, "/w Bob one");
    alice.say(&mut gw, "/w Bob two");
    bob.deliver(&mut gw);
    let conv: Conv = DM_AB.parse().unwrap();
    assert_eq!(bob.conv(DM_AB).unwrap().unread, 2);
    assert_eq!(bob.social.unread_total(), 2);
    assert_eq!(
        alice.conv(DM_AB).unwrap().unread,
        0,
        "my own lines are read"
    );

    bob.act(&mut gw, Intent::Panel(Some(conv.clone())));
    assert_eq!(bob.conv(DM_AB).unwrap().unread, 0);
    assert_eq!(bob.social.unread_total(), 0);

    // While the tab is open and the window has the focus, lines are read.
    alice.say(&mut gw, "/w Bob three");
    bob.deliver(&mut gw);
    assert_eq!(bob.conv(DM_AB).unwrap().unread, 0);

    // Closing the panel makes the next one unread again.
    bob.act(&mut gw, Intent::Panel(None));
    alice.say(&mut gw, "/w Bob four");
    bob.deliver(&mut gw);
    assert_eq!(bob.conv(DM_AB).unwrap().unread, 1);
    bob.act(&mut gw, Intent::MarkRead(conv));
    assert_eq!(bob.social.unread_total(), 0);
}

#[test]
fn the_unread_total_leaves_muted_conversations_out() {
    let mut gw = world();
    gw.befriend(ALICE, BOB);
    gw.befriend(CAROL, BOB);
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let mut carol = Client::new(&mut gw, CAROL, "Carol#0c11", 3);
    let mut bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let alice_conv: Conv = DM_AB.parse().unwrap();
    bob.act(&mut gw, Intent::MuteConv(alice_conv.clone()));

    alice.say(&mut gw, "/w Bob muted one");
    alice.say(&mut gw, "/w Bob muted two");
    carol.say(&mut gw, "/w Bob loud");
    bob.deliver(&mut gw);
    assert_eq!(
        bob.conv(DM_AB).unwrap().unread,
        2,
        "still counted in its tab"
    );
    assert_eq!(bob.conv("dm:0012:0c11").unwrap().unread, 1);
    assert_eq!(bob.social.unread_total(), 1, "but not in the total");

    bob.act(&mut gw, Intent::UnmuteConv(alice_conv));
    assert_eq!(bob.social.unread_total(), 3);
}

#[test]
fn a_table_line_sent_before_the_member_list_waits_for_it() {
    let mut gw = world();
    // Alice connects before the gateway lists the table: her snapshot
    // does not carry it. Bob's does.
    let mut alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    gw.tables
        .insert("tbl:g1".into(), vec![ALICE.into(), BOB.into()]);
    let mut bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    alice.social.on_table(Some(waiting_room(vec![
        seat(0, "Alice#af03"),
        seat(1, "Bob#0012"),
    ])));

    let sends_before = gw.sends_from(&alice.device());
    alice.say(&mut gw, "/t gl hf");
    assert_eq!(
        gw.sends_from(&alice.device()),
        sends_before,
        "no member list yet: nothing is sealed for nobody"
    );
    let line = alice.message_lines("tbl:g1").pop().unwrap();
    assert_eq!(line.state, LineState::Pending);
    assert_eq!(line.failure, None);

    // The gateway's member list arrives; the line goes.
    let members = vec![gw.person_of(ALICE), gw.person_of(BOB)];
    alice.feed(
        &mut gw,
        Down::Members {
            conv: "tbl:g1".into(),
            members,
        },
    );
    assert_eq!(
        gw.sends_from(&alice.device()),
        sends_before + 1,
        "sent once, no refusal and no reseal"
    );
    let line = alice.message_lines("tbl:g1").pop().unwrap();
    assert_eq!(line.state, LineState::Final);
    assert!(!phrases(&alice, "tbl:g1").contains(&ChatPhrase::RosterMismatch));
    bob.deliver(&mut gw);
    assert_eq!(bob.texts("tbl:g1"), ["gl hf"]);
}

// ---------------------------------------------------------------------------
// A toast that waits for an answer.
// ---------------------------------------------------------------------------

/// Feedback of 29.10.2026: a friend request accepted in the drawer left its
/// toast standing with Accept and Decline. The request's toast says what it
/// asks about, and whatever ends the request (accepted from any device,
/// declined or taken back, a block) withdraws it.
#[test]
fn whatever_ends_a_request_withdraws_its_toast() {
    let mut gw = world();
    let _alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let mut bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let person = gw.person_of(ALICE);
    let about = crate::social::ToastAbout::FriendRequest(ALICE.into());
    for gone in [
        FriendEvent::Accepted,
        FriendEvent::RequestGone,
        FriendEvent::Blocked,
    ] {
        bob.seen.clear();
        bob.feed(
            &mut gw,
            Down::Friend {
                event: FriendEvent::RequestIn,
                person: person.clone(),
                presence: None,
            },
        );
        assert!(
            bob.seen.iter().any(|effect| matches!(
                effect,
                Effect::Toast(toast) if toast.until_answered && toast.about.as_ref() == Some(&about)
            )),
            "the request's toast says what it asks"
        );
        bob.feed(
            &mut gw,
            Down::Friend {
                event: gone,
                person: person.clone(),
                presence: None,
            },
        );
        assert!(
            bob.seen
                .iter()
                .any(|effect| matches!(effect, Effect::Withdraw(withdrawn) if *withdrawn == about)),
            "{gone:?} withdrew it"
        );
    }
}

/// An invite to a group is listed at once where the gateway names who
/// invited (feedback of 29.10.2026: it showed only after a reconnect), and
/// its toast goes when it is answered or withdrawn.
#[test]
fn a_group_invite_is_listed_at_once_and_its_toast_goes_with_it() {
    let mut gw = world();
    let _alice = Client::new(&mut gw, ALICE, "Alice#af03", 1);
    let mut bob = Client::new(&mut gw, BOB, "Bob#0012", 2);
    let group = baylee_protocol::social::GroupInfo {
        id: "01a18961-489c-7546-b62d-7a3cecbc55d3".into(),
        name: "Spieleabend".into(),
        members: Vec::new(),
        invited: vec![gw.person_of(BOB)],
    };
    let inviter = gw.person_of(ALICE);
    bob.feed(
        &mut gw,
        Down::Group {
            event: baylee_protocol::social::GroupEvent::Invited,
            group: group.clone(),
            inviter: Some(inviter),
        },
    );
    let listed = &bob.social.contacts().group_invites;
    assert_eq!(listed.len(), 1, "listed in the drawer at once");
    assert_eq!(listed[0].inviter.tag, ALICE);
    bob.seen.clear();
    bob.feed(
        &mut gw,
        Down::Group {
            event: baylee_protocol::social::GroupEvent::InviteGone,
            group: group.clone(),
            inviter: None,
        },
    );
    assert!(bob.social.contacts().group_invites.is_empty());
    assert!(bob.seen.iter().any(|effect| matches!(
        effect,
        Effect::Withdraw(crate::social::ToastAbout::GroupInvite(id)) if *id == group.id
    )));
}
