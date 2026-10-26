//! Amounts known when a damage distribution is announced (CR 601.2d).

/// A divided total must be settled before paying costs, not re-evaluated
/// from the battlefield when the spell resolves.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DivisionAmount {
    /// A printed number of damage.
    Fixed(u32),
    /// The announced X plus a fixed offset, saturating on overflow.
    XPlus(u32),
    /// Twice the announced X once X reaches this threshold, otherwise X.
    XDoubleFrom(u32),
}

impl DivisionAmount {
    /// The total to distribute, with overflow saturated like other damage.
    #[must_use]
    pub const fn total(self, x: u32) -> u32 {
        match self {
            Self::Fixed(n) => n,
            Self::XPlus(offset) => x.saturating_add(offset),
            Self::XDoubleFrom(threshold) if x >= threshold => x.saturating_mul(2),
            Self::XDoubleFrom(_) => x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DivisionAmount;

    #[test]
    fn announced_total_matches_wide_arithmetic_at_every_boundary() {
        for threshold in [0, 1, 6, u32::MAX / 2, u32::MAX] {
            for x in [
                0,
                1,
                5,
                6,
                7,
                threshold.saturating_sub(1),
                threshold,
                threshold.saturating_add(1),
                u32::MAX / 2,
                u32::MAX / 2 + 1,
                u32::MAX,
            ] {
                let expected = u64::from(x) * if x >= threshold { 2 } else { 1 };
                assert_eq!(
                    DivisionAmount::XDoubleFrom(threshold).total(x),
                    u32::try_from(expected).unwrap_or(u32::MAX)
                );
                assert_eq!(DivisionAmount::Fixed(4).total(x), 4);
                let sum = u64::from(x) + u64::from(threshold);
                assert_eq!(
                    DivisionAmount::XPlus(threshold).total(x),
                    u32::try_from(sum).unwrap_or(u32::MAX)
                );
            }
        }
    }
}
