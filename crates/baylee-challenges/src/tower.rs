//! Bounded, resumable expedition stock; only the final activation makes it public.

use crate::{
    Package,
    stock::{Error, Report},
};
use central_api::{
    challenges::{HostRequest, HostResponse, Kind, Period, Publication},
    tower as api,
};
use serde::{Deserialize, Serialize};

/// A private disk bundle, never sent in a single network request.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepared {
    /// Immutable graph description and its ordered content digests.
    pub manifest: api::Manifest,
    /// Complete, independently certified encounter packages.
    pub nodes: Vec<Publication>,
}

/// Disk-only bound for all thirty-five individually bounded wire packages plus metadata.
pub const MAX_BUNDLE_BYTES: usize = api::NODES * central_api::MAX_WIRE_BYTES + 65_536;

impl Prepared {
    /// Refuse incomplete, corrupted or incompatible retained stock.
    ///
    /// # Errors
    /// Invalid graph, mismatched package/digest/build, or calendar mismatch.
    pub fn validate(&self, period: Period) -> Result<(), Error> {
        self.manifest.validate().map_err(|_| Error::Uncertified)?;
        if self.manifest.theme.period != period || self.nodes.len() != api::NODES {
            return Err(Error::Invalid);
        }
        for (index, publication) in self.nodes.iter().enumerate() {
            let index = u8::try_from(index).map_err(|_| Error::Invalid)?;
            self.manifest
                .validate_node(index, publication)
                .map_err(|_| Error::Uncertified)?;
            Package::from_publication(publication).map_err(|_| Error::Uncertified)?;
        }
        Ok(())
    }
}

/// Reconcile the current and upcoming weeks after ordinary calendar stock is ready.
/// `prepare` durably retains the whole graph and its proofs before returning.
///
/// # Errors
/// Unknown inventory, uncertified stock, conflicting publication or interrupted delivery.
pub fn reconcile(
    now: i64,
    days: u8,
    mut exchange: impl FnMut(HostRequest) -> Result<HostResponse, central_api::Error>,
    mut prepare: impl FnMut(Period) -> Result<Prepared, Error>,
) -> Result<Report, Error> {
    let wanted: Vec<_> = crate::stock::periods(now, days)?
        .into_iter()
        .filter(|p| p.kind == Kind::Weekly)
        .collect();
    let from = wanted.first().ok_or(Error::Invalid)?.starts_at;
    let until = wanted
        .last()
        .ok_or(Error::Invalid)?
        .starts_at
        .checked_add(1)
        .ok_or(Error::Invalid)?;
    let stock = send(&mut exchange, api::HostRequest::Inventory { from, until })?;
    validate_stock(&stock, from, until)?;
    let mut report = Report::default();
    for period in wanted {
        if stock
            .iter()
            .any(|slot| slot.period == period && (slot.published || slot.withdrawn))
        {
            report.present += 1;
            continue;
        }
        let prepared = prepare(period)?;
        prepared.validate(period)?;
        let mut season = stock
            .iter()
            .find(|slot| slot.period == period)
            .map(|slot| slot.id);
        for (index, publication) in prepared.nodes.into_iter().enumerate() {
            let node = u8::try_from(index).map_err(|_| Error::Invalid)?;
            let staged = send(
                &mut exchange,
                api::HostRequest::Stage {
                    manifest: Box::new(prepared.manifest.clone()),
                    node,
                    publication: Box::new(publication),
                },
            )?;
            validate_stock(&staged, period.starts_at, period.starts_at + 1)?;
            let [one] = staged.as_slice() else {
                return Err(Error::Invalid);
            };
            if one.withdrawn || !one.staged.contains(&node) || season.is_some_and(|id| id != one.id)
            {
                return Err(Error::Invalid);
            }
            season = Some(one.id);
        }
        let season = season.ok_or(Error::Invalid)?;
        let activated = send(&mut exchange, api::HostRequest::Activate { season })?;
        validate_stock(&activated, period.starts_at, period.starts_at + 1)?;
        let [one] = activated.as_slice() else {
            return Err(Error::Invalid);
        };
        if one.id != season || !one.published || one.withdrawn {
            return Err(Error::Invalid);
        }
        report.published += 1;
    }
    Ok(report)
}

fn send(
    exchange: &mut impl FnMut(HostRequest) -> Result<HostResponse, central_api::Error>,
    request: api::HostRequest,
) -> Result<Vec<api::Stock>, Error> {
    match exchange(HostRequest::Tower { request })? {
        HostResponse::Tower { stock } => Ok(stock),
        _ => Err(Error::Invalid),
    }
}

fn validate_stock(stock: &[api::Stock], from: i64, until: i64) -> Result<(), Error> {
    if stock.len() > 7 {
        return Err(Error::Invalid);
    }
    for (index, slot) in stock.iter().enumerate() {
        slot.period.validate().map_err(|_| Error::Invalid)?;
        if slot.period.kind != Kind::Weekly
            || slot.period.starts_at < from
            || slot.period.starts_at >= until
            || slot.id[6] >> 4 != 7
            || slot.id[8] >> 6 != 2
            || slot.staged.len() > api::NODES
            || slot
                .staged
                .iter()
                .any(|&node| usize::from(node) >= api::NODES)
            || slot.staged.windows(2).any(|pair| pair[0] >= pair[1])
            || (slot.published && slot.staged.len() != api::NODES)
            || stock[..index]
                .iter()
                .any(|previous| previous.id == slot.id || previous.period == slot.period)
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
    fn unavailable_or_inconsistent_inventory_never_generates_or_activates() {
        let now = 1_794_182_400;
        let result = reconcile(
            now,
            1,
            |_| Err(central_api::Error::Unavailable),
            |_| panic!("no stock without authority"),
        );
        assert!(matches!(result, Err(Error::Authority(_))));
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let slot = api::Stock {
            id,
            period: Period::containing(Kind::Weekly, now).unwrap(),
            staged: vec![0],
            published: true,
            withdrawn: false,
        };
        assert!(validate_stock(&[slot], now, now + 1).is_err());
        let result = reconcile(
            now,
            1,
            |_| Ok(HostResponse::Tower { stock: Vec::new() }),
            |_| Err(Error::Uncertified),
        );
        assert!(matches!(result, Err(Error::Uncertified)));
    }
}
