//! Shared test-acceptance holds for generation and test-debt tooling.

use std::collections::{BTreeMap, BTreeSet};

/// A malformed or stale test-acceptance hold.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct CoverageHoldError(String);

/// Reviewed reasons for keeping fully read machine-owned cards unaccepted.
#[derive(Default)]
pub struct CoverageHolds(BTreeMap<String, String>);

impl CoverageHolds {
    /// Parse Oracle ID and nonempty reason rows, rejecting duplicate IDs.
    ///
    /// # Errors
    /// Returns an error for malformed IDs, empty reasons, duplicate IDs or extra columns.
    pub fn parse(text: &str) -> Result<Self, CoverageHoldError> {
        let mut holds = BTreeMap::new();
        for (line, row) in text.lines().enumerate() {
            if row.trim().is_empty() || row.trim_start().starts_with('#') {
                continue;
            }
            let Some((id, reason)) = row.split_once('\t') else {
                return Err(CoverageHoldError(format!(
                    "card-test-holds.tsv:{}: expected Oracle ID and reason",
                    line + 1
                )));
            };
            let valid_id = id.len() == 36
                && id.bytes().enumerate().all(|(at, byte)| {
                    if matches!(at, 8 | 13 | 18 | 23) {
                        byte == b'-'
                    } else {
                        byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                    }
                });
            if !valid_id {
                return Err(CoverageHoldError(format!("invalid hold Oracle ID: {id}")));
            }
            if reason.trim().is_empty() {
                return Err(CoverageHoldError(format!("hold {id} needs a reason")));
            }
            if reason.contains('\t') {
                return Err(CoverageHoldError(format!("hold {id} has extra TSV fields")));
            }
            if holds
                .insert(id.to_owned(), reason.trim().to_owned())
                .is_some()
            {
                return Err(CoverageHoldError(format!("duplicate hold Oracle ID: {id}")));
            }
        }
        Ok(Self(holds))
    }

    /// Check that every hold still names an eligible machine-owned pool card.
    ///
    /// # Errors
    /// Returns an error when any held identity is absent from `eligible`.
    pub fn validate_ids(&self, eligible: &BTreeSet<String>) -> Result<(), CoverageHoldError> {
        for id in self.0.keys() {
            if !eligible.contains(id) {
                return Err(CoverageHoldError(format!(
                    "stale card-test hold {id}: expected a fully read machine-owned pool card"
                )));
            }
        }
        Ok(())
    }

    /// The single promotion policy, shared by emission and the source invariant.
    #[must_use]
    pub fn accepts_tests(&self, oracle_id: &str, has_tests: bool) -> bool {
        has_tests && !self.0.contains_key(oracle_id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::CoverageHolds;

    const HELD: &str = "f34b9bc4-7bfe-47fd-ba23-4eeeb46026eb";
    const ACCEPTED: &str = "aafd44c1-74a9-4aa7-a4a2-67eb39a07478";

    #[test]
    fn tested_cards_promote_only_after_their_hold_is_removed() {
        let holds = CoverageHolds::parse(&format!("{HELD}\tBattle damage gap\n")).unwrap();
        for (id, tested, expected) in [
            (HELD, true, "coverage = Coverage::Generated"),
            (HELD, false, "coverage = Coverage::Generated"),
            (ACCEPTED, true, "coverage = Coverage::Implemented"),
            (ACCEPTED, false, "coverage = Coverage::Generated"),
        ] {
            assert_eq!(
                crate::stubgen::read_coverage(holds.accepts_tests(id, tested)),
                expected
            );
        }
        let released = CoverageHolds::parse("").unwrap();
        assert!(released.accepts_tests(HELD, true));
    }

    #[test]
    fn malformed_duplicate_and_unexplained_holds_are_rejected() {
        for text in [
            "bad-id\treason".to_owned(),
            format!("{HELD}\t   "),
            format!("{HELD}\treason\textra"),
            format!("{HELD}\treason\n{HELD}\tsecond reason"),
            HELD.to_owned(),
        ] {
            assert!(CoverageHolds::parse(&text).is_err(), "{text}");
        }
    }

    #[test]
    fn holds_for_cards_no_longer_eligible_are_rejected() {
        let holds = CoverageHolds::parse(&format!("{HELD}\tBattle damage gap")).unwrap();
        assert!(holds.validate_ids(&BTreeSet::new()).is_err());
        assert!(
            holds
                .validate_ids(&BTreeSet::from([HELD.to_owned()]))
                .is_ok()
        );
    }
}
