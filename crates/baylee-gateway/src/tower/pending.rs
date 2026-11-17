//! Bounded local retry identities. Only the global authority grants execution.
use baylee_account::{Error, TowerRefused};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use uuid::Uuid;

const CAPACITY: usize = 32;
const PER_ACCOUNT: usize = 2;
// Long enough for the authority's maximum two-minute execution lease and bounded
// transport to expire. Retrying never extends local retention. Eviction grants no
// lease: the authority still refuses live competition and requires fenced recovery.
const RETENTION: Duration = Duration::from_mins(10);

#[derive(Default)]
pub(super) struct Claims {
    entries: BTreeMap<(Uuid, Uuid), Pending>,
}

struct Pending {
    claim: Uuid,
    created: Instant,
}

impl Claims {
    pub fn begin(&mut self, account: Uuid, run: Uuid, now: Instant) -> Result<Uuid, Error> {
        self.entries
            .retain(|_, entry| now.saturating_duration_since(entry.created) < RETENTION);
        // A foreign attempt must never occupy or clear the owner's retry slot.
        // Only the global authority knows whether this account owns the run.
        if let Some(entry) = self.entries.get(&(account, run)) {
            return Ok(entry.claim);
        }
        if self.entries.len() >= CAPACITY
            || self
                .entries
                .keys()
                .filter(|(owner, _)| *owner == account)
                .count()
                >= PER_ACCOUNT
        {
            return Err(Error::Busy);
        }
        let claim = Uuid::now_v7();
        self.entries.insert(
            (account, run),
            Pending {
                claim,
                created: now,
            },
        );
        Ok(claim)
    }

    pub fn remove(&mut self, account: Uuid, run: Uuid) {
        self.entries.remove(&(account, run));
    }

    pub fn refused(&mut self, account: Uuid, run: Uuid, refused: TowerRefused) {
        // Rejected also represents an invalid/truncated reply after a successful
        // remote commit, so it is ambiguous, just like an unavailable transport.
        // A definite refusal (the authority's 404 for a run that is not this
        // account's, among others) holds nothing: keeping its slot would let a
        // few accounts fill every slot of this gateway with foreign runs.
        if refused.definite
            || matches!(
                refused.error,
                Error::Conflict | Error::TermsRequired | Error::UpdateRequired
            )
        {
            self.remove(account, run);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn answered(error: Error) -> TowerRefused {
        TowerRefused {
            error,
            definite: true,
        }
    }

    const fn unanswered(error: Error) -> TowerRefused {
        TowerRefused {
            error,
            definite: false,
        }
    }

    #[test]
    fn foreign_runs_the_authority_refuses_never_fill_the_gateway() {
        let mut pending = Claims::default();
        let now = Instant::now();
        // Sixteen accounts, two foreign runs each: the authority answers 404
        // ("not yours", `Rejected`) to every claim, and each frees its slot.
        for _ in 0..CAPACITY / PER_ACCOUNT {
            let account = Uuid::now_v7();
            for _ in 0..PER_ACCOUNT {
                let run = Uuid::now_v7();
                pending.begin(account, run, now).unwrap();
                pending.refused(account, run, answered(Error::Rejected));
            }
        }
        assert!(pending.entries.is_empty());
        let owner = Uuid::now_v7();
        assert!(pending.begin(owner, Uuid::now_v7(), now).is_ok());
    }

    #[test]
    fn one_account_cannot_fill_the_gateway_and_retries_keep_the_same_claim() {
        let mut pending = Claims::default();
        let now = Instant::now();
        let account = Uuid::now_v7();
        let run = Uuid::now_v7();
        let claim = pending.begin(account, run, now).unwrap();
        assert_eq!(claim.get_version_num(), 7);
        pending.begin(account, Uuid::now_v7(), now).unwrap();
        assert_eq!(
            pending.begin(account, Uuid::now_v7(), now),
            Err(Error::Busy)
        );
        assert_eq!(pending.begin(account, run, now).unwrap(), claim);
        let foreign = Uuid::now_v7();
        assert_ne!(pending.begin(foreign, run, now).unwrap(), claim);
        assert_eq!(pending.begin(account, run, now).unwrap(), claim);
        pending.refused(foreign, run, answered(Error::Conflict));
        assert_eq!(pending.begin(account, run, now).unwrap(), claim);
        pending.begin(Uuid::now_v7(), Uuid::now_v7(), now).unwrap();
        assert_eq!(pending.entries.len(), 3);
    }

    #[test]
    fn global_bound_expires_without_retry_extension_or_authority_mutation() {
        let mut pending = Claims::default();
        let now = Instant::now();
        let account = Uuid::now_v7();
        let run = Uuid::now_v7();
        let first = pending.begin(account, run, now).unwrap();
        for _ in 1..CAPACITY {
            pending.begin(Uuid::now_v7(), Uuid::now_v7(), now).unwrap();
        }
        assert_eq!(
            pending.begin(Uuid::now_v7(), Uuid::now_v7(), now),
            Err(Error::Busy)
        );
        assert_eq!(
            pending
                .begin(
                    account,
                    run,
                    now + RETENTION.checked_sub(Duration::from_millis(1)).unwrap()
                )
                .unwrap(),
            first
        );
        let next = pending.begin(account, run, now + RETENTION).unwrap();
        assert_ne!(next, first);
        assert_eq!(pending.entries.len(), 1);
        // Explicit time supplied by the boundary: no sleeps or wall-clock jumps.
        assert_eq!(pending.begin(account, run, now).unwrap(), next);
    }

    #[test]
    fn ambiguous_replies_keep_claims_and_definite_refusals_release_only_their_run() {
        let mut pending = Claims::default();
        let now = Instant::now();
        let account = Uuid::now_v7();
        let run = Uuid::now_v7();
        let other = Uuid::now_v7();
        let mut claim = pending.begin(account, run, now).unwrap();
        let other_claim = pending.begin(account, other, now).unwrap();
        for error in [Error::Unavailable, Error::Busy, Error::Rejected] {
            pending.refused(account, run, unanswered(error));
            assert_eq!(pending.begin(account, run, now).unwrap(), claim);
        }
        for error in [Error::Conflict, Error::TermsRequired, Error::UpdateRequired] {
            pending.refused(account, run, answered(error));
            let next = pending.begin(account, run, now).unwrap();
            assert_ne!(next, claim);
            claim = next;
            assert_eq!(pending.begin(account, other, now).unwrap(), other_claim);
        }
        pending.remove(account, run);
        assert_eq!(pending.entries.len(), 1);
        pending.begin(account, Uuid::now_v7(), now).unwrap();
    }
}
