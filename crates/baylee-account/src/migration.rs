//! A gateway's side of a Central capability. A legacy account's sign-in and
//! move are Central's since the cutover (`docs/central-login.md` §"Legacy
//! migration"): no gateway receives a password, and no client posts one to a
//! gateway.
use crate::Token;
use serde::{Deserialize, Serialize};

/// One-use central capability; the receiving route fixes its purpose.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    /// Random short-lived capability, not a central session.
    pub ticket: Token,
}
