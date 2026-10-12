//! Child module of `combat::tests`. Deterministic aggregate/reference comparisons.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::ids::DamageSourceRef;

fn identity(slot: u32, version: u32) -> DamageSourceRef {
    DamageSourceRef {
        object: ObjectId::new(slot, 0),
        version,
    }
}
fn reference_sole(
    pairs: &[(DamageSourceRef, DamageSourceRef)],
    attacker: DamageSourceRef,
) -> Option<DamageSourceRef> {
    let mut blockers = pairs
        .iter()
        .filter_map(|(a, b)| (*a == attacker).then_some(*b));
    let first = blockers.next()?;
    blockers.all(|blocker| blocker == first).then_some(first)
}
fn check_history(combat: &CombatState, pairs: &[(DamageSourceRef, DamageSourceRef)]) {
    for attacker in [identity(1, 0), identity(1, 1), identity(2, 0)] {
        assert_eq!(
            combat.sole_blocker(attacker),
            reference_sole(pairs, attacker)
        );
        for blocker in [identity(3, 0), identity(3, 1), identity(4, 0)] {
            assert_eq!(
                combat.blocked_only_by(attacker, blocker),
                reference_sole(pairs, attacker) == Some(blocker)
            );
        }
    }
}
#[test]
fn aggregate_block_history_matches_full_pairs_for_every_short_operation_sequence() {
    // Enumerate all 8^4 sequences; check every prefix. No RNG or wall clock.
    // Repeated identical blocks, both incarnation changes, removal, unblock,
    // and combat reset are interleaved rather than tested only independently.
    for encoded in 0_u32..8_u32.pow(4) {
        let mut code = encoded;
        let mut combat = CombatState::default();
        let mut pairs = Vec::new();
        for _ in 0..4 {
            let operation = code % 8;
            code /= 8;
            let pair = match operation {
                0 => Some((identity(1, 0), identity(3, 0))),
                1 => Some((identity(1, 0), identity(4, 0))),
                2 => Some((identity(1, 0), identity(3, 1))),
                3 => Some((identity(1, 1), identity(3, 0))),
                4 => Some((identity(2, 0), identity(3, 0))),
                _ => None,
            };
            if let Some((attacker, blocker)) = pair {
                combat.declare_block(blocker, attacker);
                pairs.push((attacker, blocker));
            } else {
                match operation {
                    5 => {
                        combat.remove_from_combat(identity(3, 0).object);
                        combat.remove_from_combat(identity(1, 0).object);
                    }
                    6 => combat.unblock(identity(1, 0).object),
                    7 => {
                        combat = CombatState::default();
                        pairs.clear();
                    }
                    _ => unreachable!(),
                }
            }
            check_history(&combat, &pairs);
        }
    }
}

#[test]
fn identical_current_combat_distinguishes_sole_and_multiple_history_in_both_hashes() {
    let mut sole = empty_state();
    let attacker = creature(&mut sole, P0, 3, 3, KeywordSet::EMPTY);
    let first = creature(&mut sole, P1, 1, 1, KeywordSet::EMPTY);
    let second = creature(&mut sole, P1, 1, 1, KeywordSet::EMPTY);
    sole.combat.declare_attackers([AttackerInfo {
        creature: attacker,
        defending: Defender::Player(P1),
        blocked: false,
        band: None,
    }]);
    sole.declare_block(first, attacker);
    // Clone now: zones, journal, participants and current board stay identical.
    let mut multiple = sole.clone();
    let attacker_ref = multiple.source_identity(attacker).unwrap();
    let second_ref = multiple.source_identity(second).unwrap();
    multiple.combat.declare_block(second_ref, attacker_ref);
    sole.combat.remove_from_combat(first);
    multiple.combat.remove_from_combat(first);
    multiple.combat.remove_from_combat(second);
    assert!(sole.combat.blockers.is_empty());
    assert!(multiple.combat.blockers.is_empty());
    assert!(sole.combat.is_blocked(attacker));
    assert!(multiple.combat.is_blocked(attacker));
    assert!(sole.blocked_only_by(attacker, first));
    assert!(!multiple.blocked_only_by(attacker, first));
    assert_ne!(sole.snapshot_hash(), multiple.snapshot_hash());
    assert_ne!(sole.loop_signature(), multiple.loop_signature());
}
