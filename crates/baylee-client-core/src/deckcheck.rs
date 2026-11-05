//! A deck check in the player's words.
//!
//! `baylee_cards::formats::validate` decides, on either end; this says what
//! it found. The deck builder's overview and a room's refusal read the same
//! [`problem`], so a player meets one sentence for one fault wherever it comes
//! from. The gateway sends a problem as data for that reason, and its English
//! sentence is only what an older client shows.

use crate::i18n::{Lang, Phrase};
use baylee_cardtext::Problem;
use baylee_core::ids::CardIndex;

/// What a format is called, or `None` for one this build has no words for:
/// the list of formats follows the feed, so that one is shown as the feed
/// spells it, as an unknown key is. The word is the data's, and a blank
/// button would be worse.
#[must_use]
pub const fn format_phrase(format: &str) -> Option<Phrase> {
    Some(match format.as_bytes() {
        b"alchemy" => Phrase::PlayFormatAlchemy,
        b"brawl" => Phrase::PlayFormatBrawl,
        b"commander" => Phrase::PlayFormatCommander,
        b"competitivebrawl" => Phrase::PlayFormatCompetitiveBrawl,
        b"duel" => Phrase::PlayFormatDuel,
        b"future" => Phrase::PlayFormatFuture,
        b"gladiator" => Phrase::PlayFormatGladiator,
        b"historic" => Phrase::PlayFormatHistoric,
        b"legacy" => Phrase::PlayFormatLegacy,
        b"modern" => Phrase::PlayFormatModern,
        b"oathbreaker" => Phrase::PlayFormatOathbreaker,
        b"oldschool" => Phrase::PlayFormatOldschool,
        b"pauper" => Phrase::PlayFormatPauper,
        b"paupercommander" => Phrase::PlayFormatPauperCommander,
        b"penny" => Phrase::PlayFormatPenny,
        b"pioneer" => Phrase::PlayFormatPioneer,
        b"predh" => Phrase::PlayFormatPredh,
        b"premodern" => Phrase::PlayFormatPremodern,
        b"standard" => Phrase::PlayFormatStandard,
        b"standardbrawl" => Phrase::PlayFormatStandardBrawl,
        b"timeless" => Phrase::PlayFormatTimeless,
        b"vintage" => Phrase::PlayFormatVintage,
        b"freeform" => Phrase::CheckNoFormat,
        _ => return None,
    })
}

/// What a format is called in `lang`.
#[must_use]
pub fn format_name(format: &str, lang: Lang) -> &str {
    format_phrase(format).map_or(format, |phrase| phrase.text(lang))
}

/// The card this registry index names, in English: what a sentence says of
/// a card when nothing has a better name for it.
#[must_use]
pub fn english_name(card: u32) -> String {
    baylee_cards::by_index(CardIndex::new(card))
        .map_or_else(|| format!("#{card}"), |def| def.name().to_string())
}

/// What a sentence about a problem is told.
#[derive(Clone, Copy, Debug)]
pub struct Context<'a> {
    /// The format, as [`format_name`] says it.
    pub format: &'a str,
    /// The chosen list's name, as its owner wrote it.
    pub list: &'a str,
    /// The player's language.
    pub lang: Lang,
}

/// A problem as one sentence; `name` says what a card is called.
#[must_use]
pub fn problem(problem: Problem, context: Context<'_>, name: &dyn Fn(u32) -> String) -> String {
    let Context { format, list, lang } = context;
    let format = format_name(format, lang);
    match problem {
        Problem::UnknownCard { card } => Phrase::ProblemUnknownCard.fill(lang, &[&name(card)]),
        Problem::TooFew { have, need } => {
            Phrase::ProblemTooFew.fill(lang, &[&have.to_string(), &need.to_string()])
        }
        Problem::WrongSize { have, need } => {
            Phrase::ProblemWrongSize.fill(lang, &[&have.to_string(), &need.to_string()])
        }
        Problem::TooManyCopies { card, have, limit } => Phrase::ProblemTooManyCopies
            .fill(lang, &[&name(card), &have.to_string(), &limit.to_string()]),
        Problem::SideboardTooBig { have, limit: 0 } => {
            Phrase::ProblemNoSideboard.fill(lang, &[&have.to_string()])
        }
        Problem::SideboardTooBig { have, limit } => {
            Phrase::ProblemSideboardTooBig.fill(lang, &[&have.to_string(), &limit.to_string()])
        }
        Problem::Leaders { have: 0 } => Phrase::ProblemNoLeader.text(lang).to_string(),
        Problem::Leaders { have } => Phrase::ProblemLeaders.fill(lang, &[&have.to_string()]),
        Problem::NotALeader { card } => Phrase::ProblemNotALeader.fill(lang, &[&name(card)]),
        Problem::CannotLeadTogether { card, with } => {
            Phrase::ProblemCannotLeadTogether.fill(lang, &[&name(card), &name(with)])
        }
        Problem::OutsideIdentity { card } => {
            Phrase::ProblemOutsideIdentity.fill(lang, &[&name(card)])
        }
        Problem::LeadersNotTaken => Phrase::ProblemLeadersNotTaken.text(lang).to_string(),
        Problem::Banned { card } => Phrase::ProblemBanned.fill(lang, &[&name(card), format]),
        Problem::Restricted { card, have } => {
            Phrase::ProblemRestricted.fill(lang, &[&name(card), format, &have.to_string()])
        }
        Problem::NotLegal { card } => Phrase::ProblemNotLegal.fill(lang, &[&name(card), format]),
        Problem::ListBanned { card } => Phrase::ProblemListBanned.fill(lang, &[&name(card), list]),
        Problem::ListRestricted { card, have } => {
            Phrase::ProblemListRestricted.fill(lang, &[&name(card), list, &have.to_string()])
        }
        Problem::ListBannedAsCommander { card } => {
            Phrase::ProblemListBannedAsCommander.fill(lang, &[&name(card), list])
        }
    }
}

/// The short word a deck row wears for a problem: a badge, read at a glance.
/// The sentence is in the overview and on the card.
#[must_use]
pub const fn badge(problem: Problem) -> Option<Phrase> {
    Some(match problem {
        Problem::Banned { .. } | Problem::ListBanned { .. } => Phrase::BadgeBanned,
        Problem::Restricted { .. } | Problem::ListRestricted { .. } => Phrase::BadgeRestricted,
        Problem::NotLegal { .. } | Problem::UnknownCard { .. } => Phrase::BadgeNotLegal,
        Problem::TooManyCopies { .. } => Phrase::BadgeTooMany,
        Problem::OutsideIdentity { .. } => Phrase::BadgeOffColour,
        Problem::NotALeader { .. }
        | Problem::CannotLeadTogether { .. }
        | Problem::ListBannedAsCommander { .. } => Phrase::BadgeNoLeader,
        Problem::TooFew { .. }
        | Problem::WrongSize { .. }
        | Problem::SideboardTooBig { .. }
        | Problem::Leaders { .. }
        | Problem::LeadersNotTaken => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(card: u32) -> String {
        format!("card {card}")
    }

    fn context(lang: Lang) -> Context<'static> {
        Context {
            format: "modern",
            list: "Kitchen table",
            lang,
        }
    }

    #[test]
    fn a_format_is_called_by_its_name_and_an_unknown_one_by_its_key() {
        assert_eq!(format_name("modern", Lang::En), "Modern");
        assert_eq!(format_name("paupercommander", Lang::En), "Pauper Commander");
        assert_eq!(format_name("frobnicate", Lang::En), "frobnicate");
    }

    /// Every kind says what is wrong and which card it is about, in both
    /// languages: one sentence for one fault wherever it comes from.
    #[test]
    fn every_problem_names_its_card() {
        let card = 7;
        for lang in [Lang::En, Lang::De] {
            for fault in [
                Problem::TooManyCopies {
                    card,
                    have: 5,
                    limit: 4,
                },
                Problem::NotALeader { card },
                Problem::CannotLeadTogether { card, with: 8 },
                Problem::OutsideIdentity { card },
                Problem::Banned { card },
                Problem::Restricted { card, have: 2 },
                Problem::NotLegal { card },
                Problem::ListBanned { card },
                Problem::ListRestricted { card, have: 2 },
                Problem::ListBannedAsCommander { card },
                Problem::UnknownCard { card },
            ] {
                let said = problem(fault, context(lang), &named);
                assert!(said.contains("card 7"), "{fault:?}: {said}");
                assert!(!said.contains('{'), "{fault:?}: {said}");
            }
        }
    }

    #[test]
    fn a_list_is_named_and_a_format_is_named() {
        let said = problem(Problem::ListBanned { card: 1 }, context(Lang::En), &named);
        assert!(said.contains("Kitchen table"), "{said}");
        let said = problem(Problem::Banned { card: 1 }, context(Lang::En), &named);
        assert!(said.contains("Modern"), "{said}");
    }

    #[test]
    fn a_commander_deck_is_told_it_has_no_sideboard_rather_than_a_limit_of_nought() {
        let said = problem(
            Problem::SideboardTooBig { have: 3, limit: 0 },
            context(Lang::En),
            &named,
        );
        assert!(!said.contains('0'), "{said}");
    }

    #[test]
    fn a_row_wears_a_badge_only_for_what_is_about_its_card() {
        assert_eq!(
            badge(Problem::Banned { card: 1 }),
            Some(Phrase::BadgeBanned)
        );
        assert_eq!(badge(Problem::TooFew { have: 1, need: 60 }), None);
    }
}
