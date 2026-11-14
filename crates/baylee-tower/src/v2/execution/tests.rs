use super::*;

#[test]
fn expired_or_revoked_workers_cannot_replace_the_observed_prefix() {
    let initial = Execution::new([1; 32], 1024).unwrap();
    let first = initial.claimed(100, 50).unwrap();
    let head = first
        .appended(first.fence, 101, first.prefix, b"header\n")
        .unwrap();
    assert!(head.claimed(149, 50).is_err());
    assert!(head.renewed(first.fence, 150, 50).is_err());
    let recovered = head.claimed(150, 50).unwrap();
    assert!(recovered.fence > first.fence);
    assert_eq!(recovered.prefix, head.prefix);
    assert_eq!(recovered.package, head.package);
    assert!(
        recovered
            .appended(first.fence, 151, head.prefix, b"old worker\n")
            .is_err()
    );
    assert!(
        recovered
            .appended(recovered.fence, 151, initial.prefix, b"rewind\n")
            .is_err()
    );
    let revoked = recovered.revoked().unwrap();
    assert!(
        revoked
            .appended(recovered.fence, 151, head.prefix, b"late\n")
            .is_err()
    );
    let replayed = revoked.claimed(152, 50).unwrap();
    let next = replayed
        .appended(replayed.fence, 153, head.prefix, b"action\n")
        .unwrap();
    assert_eq!(next.prefix.bytes, 14);
    assert_eq!(next.prefix.chunks, 2);
    assert!(
        next.appended(next.fence, 153, head.prefix, b"action\n")
            .is_err()
    );
    let stored: Execution = serde_json::from_slice(&serde_json::to_vec(&next).unwrap()).unwrap();
    assert_eq!(stored, next);
    stored.validate().unwrap();
}

#[test]
fn sealing_and_technical_bounds_do_not_change_rules_or_reset_a_reservation() {
    let initial = Execution::new([2; 32], 4)
        .unwrap()
        .claimed(10, 100)
        .unwrap();
    assert!(initial.sealed(initial.fence, 11, initial.prefix).is_err());
    assert!(
        initial
            .appended(initial.fence, 11, initial.prefix, b"")
            .is_err()
    );
    assert!(
        initial
            .appended(initial.fence, 11, initial.prefix, b"12345")
            .is_err()
    );
    let head = initial
        .appended(initial.fence, 11, initial.prefix, b"1234")
        .unwrap();
    assert!(head.appended(head.fence, 12, head.prefix, b"5").is_err());
    assert!(head.sealed(head.fence, 12, initial.prefix).is_err());
    let sealed = head.sealed(head.fence, 12, head.prefix).unwrap();
    sealed.validate().unwrap();
    assert!(sealed.claimed(1000, 100).is_err());
    assert!(
        sealed
            .appended(sealed.fence, 13, sealed.prefix, b"5")
            .is_err()
    );
    assert_eq!(sealed.revoked().unwrap().prefix, head.prefix);
    assert!(sealed.revoked().unwrap().sealed);
    assert!(Execution::new([2; 32], 0).is_err());
    assert!(Execution::new([2; 32], MAX_RECORD_BYTES + 1).is_err());
    assert!(initial.revoked().unwrap().claimed(u64::MAX, 1).is_err());
}

#[test]
fn commitments_bind_package_order_boundaries_and_exact_bytes() {
    let initial = Execution::new([3; 32], 1024)
        .unwrap()
        .claimed(1, 100)
        .unwrap();
    let append = |state: &Execution, bytes: &[u8]| {
        state.appended(state.fence, 2, state.prefix, bytes).unwrap()
    };
    let split = append(&append(&initial, b"a"), b"bc");
    let other_split = append(&append(&initial, b"ab"), b"c");
    assert_ne!(split.prefix.digest, other_split.prefix.digest);
    assert_ne!(split.prefix.digest, append(&initial, b"abc").prefix.digest);
    assert_ne!(
        split.prefix.digest,
        append(&append(&initial, b"bc"), b"a").prefix.digest
    );
    let other_package = Execution::new([4; 32], 1024)
        .unwrap()
        .claimed(1, 100)
        .unwrap();
    assert_ne!(
        split.prefix.digest,
        append(&append(&other_package, b"a"), b"bc").prefix.digest
    );
}

#[test]
fn tiny_records_cannot_exhaust_unbounded_metadata() {
    let mut execution = Execution::new([7; 32], MAX_RECORD_BYTES)
        .unwrap()
        .claimed(10, 100)
        .unwrap();
    execution.prefix = Prefix {
        chunks: MAX_RECORD_CHUNKS,
        bytes: MAX_RECORD_CHUNKS,
        digest: [8; 32],
    };
    execution.validate().unwrap();
    assert_eq!(
        execution.appended(execution.fence, 11, execution.prefix, b"x"),
        Err(Refusal::Capacity)
    );
    // Reaching a storage limit neither changes the head nor fabricates a result.
    assert!(!execution.sealed);
    execution.prefix.chunks += 1;
    execution.prefix.bytes += 1;
    assert_eq!(execution.validate(), Err(Refusal::Invalid));
}
