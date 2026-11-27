//! The donation goal as the client shows it: what a month costs, how much of
//! it the platforms report as covered, and the three places to give. The
//! same figures and words the Central website shows (its `data::Funding`),
//! read from Central's public `PublicFunding` answer.
//!
//! What binds every place this is shown (`docs/legal.md` §"Donations"): a
//! donation buys nothing, no donor is named, Wizards is never put beside a
//! request for money, and the words say that donations are not
//! tax-deductible. Only sums and counts reach this module, never a donation.

use central_api::funding::Monthly;

/// Central's public figures, re-exported so the client reads them on every
/// target: `baylee-account`, which also names this type, is not built for
/// wasm32.
pub use central_api::funding::Funding;

/// The monthly goal in euro cents: what the work costs a month (the owner's
/// breakdown of 18.11.2026, the website's `Funding::GOAL`).
pub const GOAL: u64 = 80_000;

/// A place to give, in the order the website shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// GitHub Sponsors: monthly or once.
    GitHub,
    /// Liberapay: a weekly or monthly gift.
    Liberapay,
    /// Ko-fi: a coffee, once or monthly.
    Kofi,
}

impl Platform {
    /// The three, in the website's order.
    pub const ALL: [Self; 3] = [Self::GitHub, Self::Liberapay, Self::Kofi];

    /// Its name as the platform writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub Sponsors",
            Self::Liberapay => "Liberapay",
            Self::Kofi => "Ko-fi",
        }
    }

    /// Where it takes a gift (the repository's `.github/FUNDING.yml`).
    #[must_use]
    pub const fn address(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/sponsors/AceVik",
            Self::Liberapay => "https://liberapay.com/AceVik",
            Self::Kofi => "https://ko-fi.com/acevik",
        }
    }

    /// What the platform reports a month, if Central has a figure from it.
    #[must_use]
    pub const fn monthly(self, funding: &Funding) -> Option<Monthly> {
        match self {
            Self::GitHub => funding.github,
            Self::Liberapay => funding.liberapay,
            Self::Kofi => funding.kofi,
        }
    }
}

/// What is covered a month, in euro cents, and by how many supporters,
/// over the platforms that report a figure; `None` when none does.
#[must_use]
pub fn covered(funding: &Funding) -> Option<(u64, u64)> {
    Platform::ALL
        .into_iter()
        .filter_map(|platform| platform.monthly(funding))
        .map(|monthly| (monthly.month, monthly.supporters))
        .reduce(|sum, part| (sum.0.saturating_add(part.0), sum.1.saturating_add(part.1)))
}

/// The share of the goal `month` euro cents cover, in whole percent: never
/// above 100, and never 0 for a month that has anything.
#[must_use]
pub const fn percent(month: u64) -> u64 {
    let percent = month.saturating_mul(100) / GOAL;
    if percent > 100 {
        100
    } else if percent == 0 && month > 0 {
        1
    } else {
        percent
    }
}

/// The share of the goal `month` covers, as a bar draws it (0..=1).
#[must_use]
#[allow(clippy::cast_precision_loss)] // euro cents far below 2^24 per month
pub fn share(month: u64) -> f32 {
    (month.min(GOAL) as f32) / (GOAL as f32)
}

/// Whole euros, rounded to the nearest, as `lang` writes an amount.
#[must_use]
pub fn euros(cents: u64, lang: crate::i18n::Lang) -> String {
    let whole = cents.saturating_add(50) / 100;
    let mut digits = whole.to_string();
    let mark = if lang == crate::i18n::Lang::De {
        '.'
    } else {
        ','
    };
    let mut at = digits.len();
    while at > 3 {
        at -= 3;
        digits.insert(at, mark);
    }
    match lang {
        crate::i18n::Lang::De => format!("{digits}\u{a0}€"),
        crate::i18n::Lang::En => format!("€{digits}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn monthly(month: u64, supporters: u64) -> Monthly {
        Monthly { month, supporters }
    }

    #[test]
    fn the_covered_month_adds_what_the_platforms_report_and_nothing_else() {
        assert_eq!(covered(&Funding::default()), None);
        let funding = Funding {
            github: None,
            liberapay: Some(monthly(10_010, 3)),
            kofi: Some(monthly(0, 0)),
            once: None,
        };
        assert_eq!(covered(&funding), Some((10_010, 3)));
    }

    #[test]
    fn the_share_of_the_goal_is_whole_and_never_hides_a_first_gift() {
        assert_eq!(percent(0), 0);
        assert_eq!(percent(1), 1);
        assert_eq!(percent(40_000), 50);
        assert_eq!(percent(GOAL * 2), 100);
        assert!((share(40_000) - 0.5).abs() < f32::EPSILON);
        assert!((share(GOAL * 3) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn amounts_read_as_each_language_writes_euros() {
        assert_eq!(euros(80_000, Lang::De), "800\u{a0}€");
        assert_eq!(euros(80_000, Lang::En), "€800");
        assert_eq!(euros(12_345_649, Lang::De), "123.456\u{a0}€");
        assert_eq!(euros(149, Lang::En), "€1");
        assert_eq!(euros(150, Lang::En), "€2");
    }

    #[test]
    fn the_places_to_give_are_the_repositorys_own() {
        let funding = include_str!("../../../.github/FUNDING.yml");
        for (platform, handle) in [
            (Platform::GitHub, "AceVik"),
            (Platform::Liberapay, "AceVik"),
            (Platform::Kofi, "acevik"),
        ] {
            assert!(platform.address().ends_with(handle));
            assert!(funding.contains(handle), "{funding}");
        }
    }
}
