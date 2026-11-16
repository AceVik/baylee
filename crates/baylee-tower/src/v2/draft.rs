//! Stable account-bound starter samples. The authority retains the secret seed.
use crate::Refusal;
use sha2::{Digest, Sha256};

/// Three distinct indices from a reviewed pool. A separate hash domain prevents
/// the public starter sample from revealing shop scores. Sorting full digests
/// avoids modulo bias or an unbounded retry loop; at most 64 scores are allocated.
/// # Errors
/// The expanded pool must contain fifty to sixty-four candidates.
pub fn choices(seed: &[u8; 32], count: usize) -> Result<[u8; 3], Refusal> {
    if !(50..=64).contains(&count) {
        return Err(Refusal::Invalid);
    }
    let count = u8::try_from(count).map_err(|_| Refusal::Invalid)?;
    let mut ranked: Vec<([u8; 32], u8)> = (0..count)
        .map(|index| {
            let hash = Sha256::new()
                .chain_update(b"baylee/tower/starter-draft/v1\0")
                .chain_update(seed)
                .chain_update([index])
                .finalize()
                .into();
            (hash, index)
        })
        .collect();
    ranked.sort_unstable();
    Ok([ranked[0].1, ranked[1].1, ranked[2].1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_is_stable_distinct_bounded_and_reaches_the_complete_pool() {
        assert!(choices(&[0; 32], 49).is_err());
        assert!(choices(&[0; 32], 65).is_err());
        let mut observed = std::collections::BTreeSet::new();
        for seed in 0..=255 {
            let offered = choices(&[seed; 32], 50).unwrap();
            assert_eq!(offered, choices(&[seed; 32], 50).unwrap());
            assert!(offered.iter().all(|index| *index < 50));
            assert_eq!(
                offered
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                3
            );
            observed.extend(offered);
        }
        assert_eq!(observed.len(), 50);
    }
}
