//! Central's calendar is authoritative. Reconciliation never overwrites an occupied slot.
use central_api::challenges::{Challenge, HostRequest, HostResponse, Kind, Period, Publication};

/// One successfully reconciled inventory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Slots already held by Central, including intentionally withdrawn ones.
    pub present: usize,
    /// Previously empty slots admitted during this pass.
    pub published: usize,
}

/// The worker stops on uncertainty and retries its immutable staged data next pass.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid calendar horizon or an inconsistent authority response.
    #[error("invalid challenge calendar or authority response")]
    Invalid,
    /// Central did not confirm the operation.
    #[error("challenge authority unavailable: {0}")]
    Authority(#[from] central_api::Error),
    /// No certified candidate is ready. No unverified fallback is published.
    #[error("no certified candidate ready for the requested slot")]
    Uncertified,
}

/// Current and upcoming slots, daily first at shared boundaries.
///
/// # Errors
/// Invalid time or a horizon outside 1–24 days (inventory is bounded to 32 days).
pub fn periods(now: i64, days: u8) -> Result<Vec<Period>, Error> {
    if !(1..=24).contains(&days) || now <= 0 {
        return Err(Error::Invalid);
    }
    let end = now
        .checked_add(i64::from(days) * Kind::Daily.seconds())
        .ok_or(Error::Invalid)?;
    let mut periods = Vec::new();
    for kind in [Kind::Daily, Kind::Weekly] {
        let mut slot = Period::containing(kind, now)?;
        while slot.starts_at < end {
            periods.push(slot);
            slot = Period::containing(kind, slot.ends_at)?;
        }
    }
    periods.sort_by_key(|period| (period.starts_at, period.kind.seconds()));
    Ok(periods)
}

/// Fill the requested calendar from positively certified immutable packages.
/// `prepare` must durably stage its package and proof records before returning it.
/// An ambiguous publish is retried on the next pass after re-reading inventory.
///
/// # Errors
/// Invalid inventory, missing proof, malformed package or refused/ambiguous publication.
pub fn reconcile(
    now: i64,
    days: u8,
    mut exchange: impl FnMut(HostRequest) -> Result<HostResponse, central_api::Error>,
    mut prepare: impl FnMut(Period) -> Result<Publication, Error>,
) -> Result<Report, Error> {
    let wanted = periods(now, days)?;
    let from = wanted.first().ok_or(Error::Invalid)?.starts_at;
    let until = wanted
        .last()
        .ok_or(Error::Invalid)?
        .starts_at
        .checked_add(1)
        .ok_or(Error::Invalid)?;
    let HostResponse::Inventory { challenges } = exchange(HostRequest::Inventory { from, until })?
    else {
        return Err(Error::Invalid);
    };
    validate_inventory(&challenges, from, until)?;
    let mut report = Report::default();
    for period in wanted {
        if challenges
            .iter()
            .any(|task| task.description.period == period)
        {
            report.present += 1;
            continue;
        }
        let publication = prepare(period)?;
        publication.validate().map_err(|_| Error::Uncertified)?;
        if publication.description.period != period {
            return Err(Error::Invalid);
        }
        let expected = publication.description.clone();
        let digest = publication.digest;
        let HostResponse::Challenge { challenge } = exchange(HostRequest::Publish {
            publication: Box::new(publication),
        })?
        else {
            return Err(Error::Invalid);
        };
        if challenge.description != expected || challenge.digest != digest || challenge.withdrawn {
            return Err(Error::Invalid);
        }
        report.published += 1;
    }
    Ok(report)
}

fn validate_inventory(tasks: &[Challenge], from: i64, until: i64) -> Result<(), Error> {
    if tasks.len() > 64 {
        return Err(Error::Invalid);
    }
    for (at, task) in tasks.iter().enumerate() {
        task.description.validate().map_err(|_| Error::Invalid)?;
        let period = task.description.period;
        if period.starts_at < from
            || period.starts_at >= until
            || tasks[..at]
                .iter()
                .any(|prior| prior.description.period == period || prior.id == task.id)
        {
            return Err(Error::Invalid);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizon_contains_current_week_and_each_day_across_rollover() {
        let monday = 1_794_182_400;
        let slots = periods(monday + 6 * 86_400 + 86_399, 2).unwrap();
        assert_eq!(
            slots.iter().filter(|slot| slot.kind == Kind::Daily).count(),
            3
        );
        assert_eq!(
            slots
                .iter()
                .filter(|slot| slot.kind == Kind::Weekly)
                .count(),
            2
        );
        assert_eq!(slots[0].starts_at, monday);
        assert!(periods(monday, 0).is_err());
        assert!(periods(monday, 25).is_err());
        assert!(periods(i64::MAX, 1).is_err());
        assert!(periods(0, 1).is_err());
    }

    #[test]
    fn uncertain_inventory_and_uncertified_candidates_never_publish() {
        let monday = 1_794_182_400;
        let mut calls = 0;
        let result = reconcile(
            monday,
            1,
            |_| {
                calls += 1;
                Ok(HostResponse::Inventory {
                    challenges: Vec::new(),
                })
            },
            |_| Err(Error::Uncertified),
        );
        assert!(matches!(result, Err(Error::Uncertified)));
        assert_eq!(calls, 1);
        let result = reconcile(
            monday,
            1,
            |_| Err(central_api::Error::Unavailable),
            |_| panic!("no generation without authoritative inventory"),
        );
        assert!(matches!(result, Err(Error::Authority(_))));
    }
}
