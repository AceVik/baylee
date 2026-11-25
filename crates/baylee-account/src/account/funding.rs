//! Central's donation figures, read without signing in: sums and counts the
//! platforms publish, never a donation, a time or anything about who gave
//! (`docs/legal.md` §"Donations").
use crate::{
    CentralHttp, Endpoint, Error,
    wire::{Request, Response},
};
use central_api::funding::Funding;

/// Reads the donation figures Central publishes, as its website shows them.
///
/// # Errors
/// An unavailable Central, an answer of another kind, or a figure beyond
/// its bound (the whole answer is refused, as `Funding::validate` says).
pub fn funding_read(endpoint: &Endpoint) -> Result<Funding, Error> {
    let Response::PublicFunding { funding } =
        CentralHttp::new(endpoint.clone()).exchange(&Request::PublicFunding {}, None)?
    else {
        return Err(Error::Rejected);
    };
    funding.validate()?;
    Ok(funding)
}
