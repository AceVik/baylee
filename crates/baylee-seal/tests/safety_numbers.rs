//! Safety numbers: sixty digits both ends read alike, whatever the order of tags and devices.

use std::cmp::Ordering;

use baylee_seal::testkit::registered;
use baylee_seal::{fingerprint, safety_number, tag_order};

fn ik(seed: u64) -> [u8; 32] {
    registered("af03", seed).1.ik
}

#[test]
fn a_safety_number_is_sixty_ascii_digits() {
    let number = safety_number(("af03", &[ik(1)]), ("0012", &[ik(2)]));
    assert_eq!(number.len(), 60);
    assert!(number.bytes().all(|b| b.is_ascii_digit()));
}

#[test]
fn a_fingerprint_is_thirty_ascii_digits() {
    let half = fingerprint("af03", &[ik(1), ik(2)]);
    assert_eq!(half.len(), 30);
    assert!(half.bytes().all(|b| b.is_ascii_digit()));
}

#[test]
fn a_safety_number_is_the_smaller_tags_half_then_the_others() {
    let a = [ik(1)];
    let b = [ik(2)];
    let number = safety_number(("af03", &a), ("0012", &b));
    assert_eq!(&number[..30], fingerprint("0012", &b));
    assert_eq!(&number[30..], fingerprint("af03", &a));
}

#[test]
fn a_safety_number_is_the_same_from_either_side() {
    let a = [ik(1), ik(3)];
    let b = [ik(2)];
    assert_eq!(
        safety_number(("af03", &a), ("0012", &b)),
        safety_number(("0012", &b), ("af03", &a))
    );
}

#[test]
fn the_order_an_accounts_devices_are_listed_in_does_not_matter() {
    let forward = [ik(1), ik(2), ik(3)];
    let backward = [ik(3), ik(2), ik(1)];
    let shuffled = [ik(2), ik(3), ik(1)];
    assert_eq!(
        fingerprint("af03", &forward),
        fingerprint("af03", &backward)
    );
    assert_eq!(
        fingerprint("af03", &forward),
        fingerprint("af03", &shuffled)
    );
    let other = [ik(9)];
    assert_eq!(
        safety_number(("af03", &forward), ("0012", &other)),
        safety_number(("af03", &backward), ("0012", &other))
    );
}

#[test]
fn adding_a_device_changes_the_safety_number() {
    let other = [ik(9)];
    let before = safety_number(("af03", &[ik(1)]), ("0012", &other));
    let after = safety_number(("af03", &[ik(1), ik(2)]), ("0012", &other));
    assert_ne!(before, after);
    // Only the account that gained a device changes its half.
    assert_eq!(&before[..30], &after[..30]);
    assert_ne!(&before[30..], &after[30..]);
}

#[test]
fn the_tag_is_part_of_an_accounts_fingerprint() {
    assert_ne!(fingerprint("af03", &[ik(1)]), fingerprint("af04", &[ik(1)]));
}

#[test]
fn a_different_device_gives_a_different_fingerprint() {
    assert_ne!(fingerprint("af03", &[ik(1)]), fingerprint("af03", &[ik(2)]));
}

#[test]
fn an_account_without_devices_still_has_a_fingerprint() {
    let half = fingerprint("af03", &[]);
    assert_eq!(half.len(), 30);
    assert_ne!(half, fingerprint("af04", &[]));
}

#[test]
fn tags_are_ordered_numerically() {
    assert_eq!(tag_order("0012", "af03"), Ordering::Less);
    assert_eq!(tag_order("af03", "0012"), Ordering::Greater);
    assert_eq!(tag_order("af03", "af03"), Ordering::Equal);
}

#[test]
fn a_shorter_tag_comes_first() {
    assert_eq!(tag_order("fff", "0000"), Ordering::Less);
    assert_eq!(tag_order("0000", "fff"), Ordering::Greater);
}

#[test]
fn a_safety_number_of_an_account_with_itself_is_the_same_from_either_end() {
    let a = [ik(1)];
    let b = [ik(2)];
    let one = safety_number(("af03", &a), ("af03", &b));
    let other = safety_number(("af03", &b), ("af03", &a));
    assert_eq!(one, other);
    assert_eq!(one.len(), 60);
    // Both halves are there, the smaller digits first.
    let (fa, fb) = (fingerprint("af03", &a), fingerprint("af03", &b));
    let (low, high) = if fa < fb { (fa, fb) } else { (fb, fa) };
    assert_eq!(one, format!("{low}{high}"));
}

#[test]
fn a_device_given_twice_counts_once() {
    let once = fingerprint("af03", &[ik(1), ik(2)]);
    assert_eq!(fingerprint("af03", &[ik(1), ik(2), ik(1)]), once);
    assert_eq!(fingerprint("af03", &[ik(2), ik(2), ik(1), ik(1)]), once);
    assert_eq!(
        fingerprint("af03", &[ik(1), ik(1)]),
        fingerprint("af03", &[ik(1)])
    );
    let other = [ik(9)];
    assert_eq!(
        safety_number(("af03", &[ik(1), ik(1)]), ("0012", &other)),
        safety_number(("af03", &[ik(1)]), ("0012", &other))
    );
}
