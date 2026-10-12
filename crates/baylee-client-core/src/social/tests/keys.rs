//! The key cache: devices are trusted on first use, checked against their account's tag, and changes are diffed (§4.1, §4.4, §4.5).

use baylee_protocol::social::{AccountKeys, DeviceKey};
use baylee_seal::b64::url;
use baylee_seal::testkit::registered;

use crate::social::keys::{KeyCache, grouped_id};

/// The row the gateway would serve for the device drawn from `seed`, its
/// statement made for `tag`.
fn row(tag: &str, seed: u64) -> DeviceKey {
    let (_secrets, public) = registered(tag, seed);
    DeviceKey {
        id: public.id.clone(),
        ik: url::encode(&public.ik),
        dk: url::encode(&public.dk),
        dsig: url::encode(&public.dsig),
        label: "macOS".to_string(),
        added_at: seed,
    }
}

fn account(tag: &str, devices: Vec<DeviceKey>) -> AccountKeys {
    AccountKeys {
        tag: tag.to_string(),
        handle: format!("Bob#{tag}"),
        devices,
    }
}

fn ids(devices: &[crate::social::keys::CachedDevice]) -> Vec<String> {
    devices.iter().map(|d| d.id.clone()).collect()
}

/// Flips one bit of a base64url-encoded value.
fn corrupt(value: &str) -> String {
    let mut bytes: Vec<u8> = url::decode_array::<32>(value).map_or_else(
        |_| url::decode_array::<64>(value).expect("64 bytes").to_vec(),
        |a| a.to_vec(),
    );
    bytes[0] ^= 1;
    url::encode(&bytes)
}

#[test]
fn the_first_fetch_caches_every_device_and_is_not_a_change() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    let diff = cache.take(&account("0012", vec![a.clone(), b.clone()]), 500);
    assert!(diff.first);
    assert!(!diff.changed(), "a first fetch is no change");
    assert_eq!(diff.tag, "0012");
    assert_eq!(diff.refused, 0);
    assert!(!diff.was_verified);
    assert_eq!(diff.added.len(), 2);
    assert!(diff.removed.is_empty());
    let keys = cache.get("0012").expect("cached");
    assert_eq!(ids(&keys.devices), [a.id.clone(), b.id.clone()]);
    assert_eq!(keys.first_seen, 500);
    assert_eq!(keys.handle, "Bob#0012");
    assert_eq!(keys.device(&a.id).map(|d| d.added_at), Some(1));
    assert!(cache.has("0012"));
    assert!(!cache.has("0013"));
    assert!(cache.get("0013").is_none());
}

#[test]
fn a_second_identical_fetch_has_no_diff() {
    let mut cache = KeyCache::default();
    let acc = account("0012", vec![row("0012", 1), row("0012", 2)]);
    cache.take(&acc, 500);
    let diff = cache.take(&acc, 900);
    assert!(!diff.first);
    assert!(!diff.changed());
    assert!(diff.added.is_empty() && diff.removed.is_empty());
    assert_eq!(
        cache.get("0012").unwrap().first_seen,
        500,
        "first seen stays"
    );
}

#[test]
fn a_device_added_later_is_in_the_diff() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    cache.take(&account("0012", vec![a.clone()]), 1);
    let diff = cache.take(&account("0012", vec![a.clone(), b.clone()]), 2);
    assert!(diff.changed());
    assert_eq!(ids(&diff.added), std::slice::from_ref(&b.id));
    assert!(diff.removed.is_empty());
    assert_eq!(cache.get("0012").unwrap().devices.len(), 2);
}

#[test]
fn a_device_removed_later_is_in_the_diff() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    cache.take(&account("0012", vec![a.clone(), b.clone()]), 1);
    let diff = cache.take(&account("0012", vec![b.clone()]), 2);
    assert!(diff.changed());
    assert_eq!(ids(&diff.removed), [a.id]);
    assert!(diff.added.is_empty());
    assert_eq!(ids(&cache.get("0012").unwrap().devices), [b.id]);
}

#[test]
fn a_swapped_device_is_both_added_and_removed() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    cache.take(&account("0012", vec![a.clone()]), 1);
    let diff = cache.take(&account("0012", vec![b.clone()]), 2);
    assert_eq!(ids(&diff.added), [b.id]);
    assert_eq!(ids(&diff.removed), [a.id]);
}

#[test]
fn a_statement_made_for_another_tag_is_refused_and_counted() {
    let mut cache = KeyCache::default();
    let stolen = row("0099", 1);
    let diff = cache.take(&account("0012", vec![stolen]), 1);
    assert_eq!(diff.refused, 1);
    assert!(diff.added.is_empty());
    assert!(cache.get("0012").unwrap().devices.is_empty());
}

#[test]
fn an_id_that_is_not_the_identity_keys_is_refused() {
    let mut cache = KeyCache::default();
    let mut forged = row("0012", 1);
    forged.id = "0123456789abcdef".to_string();
    assert_ne!(forged.id, row("0012", 1).id);
    let diff = cache.take(&account("0012", vec![forged]), 1);
    assert_eq!(diff.refused, 1);
    assert!(cache.get("0012").unwrap().devices.is_empty());
}

#[test]
fn an_id_of_the_wrong_shape_is_refused() {
    let mut cache = KeyCache::default();
    let mut forged = row("0012", 1);
    forged.id = forged.id.to_uppercase();
    let diff = cache.take(&account("0012", vec![forged]), 1);
    assert_eq!(diff.refused, 1);
}

#[test]
fn a_damaged_signature_or_key_is_refused() {
    let good = row("0012", 1);
    let mut bad_sig = good.clone();
    bad_sig.dsig = corrupt(&good.dsig);
    let mut bad_dk = good.clone();
    bad_dk.dk = corrupt(&good.dk);
    let mut bad_ik = good.clone();
    bad_ik.ik = corrupt(&good.ik);
    let mut short = good.clone();
    short.ik = url::encode(&[1, 2, 3]);
    let mut garbage = good.clone();
    garbage.dsig = "!!not base64!!".to_string();
    for (case, device) in [bad_sig, bad_dk, bad_ik, short, garbage]
        .into_iter()
        .enumerate()
    {
        let mut cache = KeyCache::default();
        let diff = cache.take(&account("0012", vec![device]), 1);
        assert_eq!(diff.refused, 1, "case {case}");
        assert!(cache.get("0012").unwrap().devices.is_empty(), "case {case}");
    }
}

#[test]
fn one_bad_device_does_not_keep_the_good_ones_out() {
    let mut cache = KeyCache::default();
    let good = row("0012", 1);
    let stolen = row("0099", 2);
    let diff = cache.take(&account("0012", vec![stolen, good.clone()]), 1);
    assert_eq!(diff.refused, 1);
    assert_eq!(ids(&diff.added), std::slice::from_ref(&good.id));
    assert_eq!(ids(&cache.get("0012").unwrap().devices), [good.id]);
}

#[test]
fn a_fetch_of_nothing_but_bad_devices_still_counts_as_the_first() {
    let mut cache = KeyCache::default();
    let diff = cache.take(&account("0012", vec![row("0099", 1), row("0098", 2)]), 1);
    assert!(diff.first);
    assert_eq!(diff.refused, 2);
    assert!(!diff.changed());
}

#[test]
fn verifying_marks_the_current_devices_and_a_new_device_drops_the_mark() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    cache.take(&account("0012", vec![a.clone()]), 1);
    assert!(!cache.get("0012").unwrap().verified());
    assert!(cache.verify("0012"));
    assert!(cache.get("0012").unwrap().verified());

    let diff = cache.take(&account("0012", vec![a, b]), 2);
    assert!(diff.was_verified, "it was verified before the change");
    assert!(diff.changed());
    assert!(!cache.get("0012").unwrap().verified());
}

#[test]
fn a_removed_device_also_drops_the_verified_mark() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    cache.take(&account("0012", vec![a.clone(), b]), 1);
    cache.verify("0012");
    cache.take(&account("0012", vec![a]), 2);
    assert!(!cache.get("0012").unwrap().verified());
}

#[test]
fn a_refetch_of_the_same_devices_in_another_order_stays_verified() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    cache.take(&account("0012", vec![a.clone(), b.clone()]), 1);
    cache.verify("0012");
    let diff = cache.take(&account("0012", vec![b, a]), 2);
    assert!(!diff.changed());
    assert!(diff.was_verified);
    assert!(cache.get("0012").unwrap().verified());
}

#[test]
fn verifying_again_after_a_change_marks_the_new_set() {
    let mut cache = KeyCache::default();
    let (a, b) = (row("0012", 1), row("0012", 2));
    cache.take(&account("0012", vec![a.clone()]), 1);
    cache.verify("0012");
    cache.take(&account("0012", vec![a, b]), 2);
    assert!(!cache.get("0012").unwrap().verified());
    assert!(cache.verify("0012"));
    assert!(cache.get("0012").unwrap().verified());
}

#[test]
fn verifying_an_account_that_was_never_fetched_does_nothing() {
    let mut cache = KeyCache::default();
    assert!(!cache.verify("0012"));
    assert!(!cache.has("0012"));
}

#[test]
fn unverify_drops_the_mark() {
    let mut cache = KeyCache::default();
    cache.take(&account("0012", vec![row("0012", 1)]), 1);
    cache.verify("0012");
    cache.unverify("0012");
    assert!(!cache.get("0012").unwrap().verified());
    cache.unverify("0777"); // an unknown tag is no error
}

#[test]
fn a_verified_account_with_no_devices_loses_the_mark_with_its_first_device() {
    let mut cache = KeyCache::default();
    cache.take(&account("0012", Vec::new()), 1);
    cache.verify("0012");
    // Verified over the empty set: the first device to appear drops it.
    cache.take(&account("0012", vec![row("0012", 1)]), 2);
    assert!(!cache.get("0012").unwrap().verified());
}

#[test]
fn forgetting_an_account_makes_the_next_fetch_a_first_one() {
    let mut cache = KeyCache::default();
    let acc = account("0012", vec![row("0012", 1)]);
    cache.take(&acc, 1);
    cache.verify("0012");
    cache.forget("0012");
    assert!(!cache.has("0012"));
    let diff = cache.take(&acc, 2);
    assert!(diff.first);
    assert!(!diff.was_verified);
    assert!(!cache.get("0012").unwrap().verified());
}

#[test]
fn accounts_are_cached_apart_and_listed_by_tag() {
    let mut cache = KeyCache::default();
    cache.take(&account("0013", vec![row("0013", 1)]), 1);
    cache.take(&account("0012", vec![row("0012", 2)]), 1);
    assert_eq!(cache.tags().collect::<Vec<_>>(), ["0012", "0013"]);
    cache.verify("0012");
    assert!(!cache.get("0013").unwrap().verified());
}

#[test]
fn handle_and_label_are_cleaned_and_the_label_capped() {
    let mut cache = KeyCache::default();
    let mut device = row("0012", 1);
    device.label = format!("Win\u{202E}dows\n{}", "x".repeat(60));
    let mut acc = account("0012", vec![device.clone()]);
    acc.handle = "Bo\u{202E}b\n#0012".to_string();
    cache.take(&acc, 1);
    let keys = cache.get("0012").unwrap();
    assert_eq!(keys.handle, "Bob#0012");
    let label = &keys.device(&device.id).unwrap().label;
    assert_eq!(label.chars().count(), 24);
    assert!(label.starts_with("Windowsxxx"));
}

#[test]
fn iks_are_sorted_whatever_order_the_devices_came_in() {
    let mut cache = KeyCache::default();
    cache.take(
        &account("0012", vec![row("0012", 1), row("0012", 2), row("0012", 3)]),
        1,
    );
    let iks = cache.get("0012").unwrap().iks();
    let mut sorted = iks.clone();
    sorted.sort_unstable();
    assert_eq!(iks, sorted);
    assert_eq!(iks.len(), 3);
}

#[test]
fn a_device_id_is_shown_in_four_groups_of_four() {
    assert_eq!(grouped_id("0123456789abcdef"), "0123 4567 89ab cdef");
    assert_eq!(grouped_id("abcdef"), "abcd ef");
    assert_eq!(grouped_id(""), "");
}
