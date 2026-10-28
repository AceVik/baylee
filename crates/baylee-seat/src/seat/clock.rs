//! Clock preflight limits shared by the seat and CLI.
/// The seconds a question gets at a `blitz` table, the fastest preset: too
/// few for a model that thinks, so the bridge refuses such a table unless
/// told otherwise ([`super::BridgeConfig::allow_blitz`]).
pub const BLITZ_SECS: u32 = 30;

/// Whether a positive decision clock is too short for the default bridge.
/// Zero means untimed, not an immediately expiring decision (protocol clocks).
#[must_use]
pub const fn is_blitz_clock(seconds: u32) -> bool {
    seconds > 0 && seconds <= BLITZ_SECS
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_clock_and_zero_are_untimed_but_positive_short_clocks_are_blitz() {
        for (seconds, blitz) in [
            (None, false),
            (Some(0), false),
            (Some(10), true),
            (Some(30), true),
            (Some(31), false),
        ] {
            assert_eq!(seconds.is_some_and(is_blitz_clock), blitz);
        }
    }
}
