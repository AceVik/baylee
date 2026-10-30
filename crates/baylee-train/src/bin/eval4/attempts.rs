//! Operational denominator accounting, enabled only for failure-terminal research.
use super::{Args, HouseDeck, collector};
use anyhow::{Context as _, ensure};
use serde_json::Value;
use std::sync::Arc;

pub(super) const OBSERVATION_ERROR: &str = "research observation failed";

pub(super) struct Ledger {
    #[cfg(target_os = "linux")]
    inner: baylee_train_ops::research_attempts::Ledger,
}

pub(super) fn prepare(args: &Args, decks: &[HouseDeck]) -> anyhow::Result<Option<Arc<Ledger>>> {
    if args.collection_purpose != collector::Purpose::FailureTerminalResearchV1 {
        return Ok(None);
    }
    #[cfg(target_os = "linux")]
    {
        let end = args
            .deal_offset
            .checked_add(args.deals)
            .context("research denominator overflow")?;
        let mut attempts = Vec::new();
        for deal in args.deal_offset..end {
            let table = usize::try_from(deal)? / args.seats;
            attempts.push(baylee_train_ops::research_attempts::Attempt {
                deal,
                seed: args
                    .first_seed
                    .checked_add(u64::try_from(table)?)
                    .context("research seed overflow")?,
                focal_seat: usize::try_from(deal)? % args.seats,
                policy_seed: args.policy_seed,
                deck_keys: (0..args.seats)
                    .map(|seat| decks[(table * args.seats + seat) % decks.len()].key.clone())
                    .collect(),
            });
        }
        Ok(Some(Arc::new(Ledger {
            inner: baylee_train_ops::research_attempts::Ledger::create(
                args.record_out.as_deref().context("research output")?,
                attempts,
            )?,
        })))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = decks;
        anyhow::bail!("research attempt accounting requires Linux");
    }
}

impl Ledger {
    pub(super) fn claim(&self, deal: u64, worker: usize) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.inner.claim(deal, worker)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (deal, worker);
            anyhow::bail!("research attempt accounting requires Linux");
        }
    }
    pub(super) fn completed(&self, row: &Value) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.inner.completed(row)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = row;
            anyhow::bail!("research attempt accounting requires Linux");
        }
    }
    pub(super) fn worker_failed(&self, worker: usize, reason: &str) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.inner.worker_failed(worker, reason)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (worker, reason);
            anyhow::bail!("research attempt accounting requires Linux");
        }
    }
}

pub(super) fn join_workers(
    handles: Vec<std::thread::JoinHandle<anyhow::Result<()>>>,
    ledger: &Ledger,
) -> anyhow::Result<()> {
    let mut failed = false;
    let mut receipt_failed = false;
    for (worker, handle) in handles.into_iter().enumerate() {
        let reason = match handle.join() {
            Ok(Ok(())) => continue,
            Ok(Err(error))
                if error
                    .chain()
                    .any(|cause| cause.to_string() == OBSERVATION_ERROR) =>
            {
                "observation_error"
            }
            Ok(Err(_)) => "worker_error",
            Err(_) => "worker_panic",
        };
        failed = true;
        receipt_failed |= ledger.worker_failed(worker, reason).is_err();
    }
    ensure!(
        !receipt_failed,
        "research workers joined but abort receipt publication failed; preserve source for guard-bound recovery"
    );
    ensure!(
        !failed,
        "research worker aborted; assigned attempts retained as unknown; finalize after guard cleanup"
    );
    Ok(())
}
