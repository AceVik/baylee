//! Clock preflight limits shared by the seat and CLI.
/// The seconds a question gets at a `blitz` table, the fastest preset: too
/// few for a model that thinks, so the bridge refuses such a table unless
/// told otherwise ([`super::BridgeConfig::allow_blitz`]).
pub const BLITZ_SECS: u32 = 30;

