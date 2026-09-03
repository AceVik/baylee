//! The choices a player answers by *position*, and what to call each row.
//!
//! Four of the engine's pending choices are not a click on the board, because
//! what they name is not on it: a colour, a seat, one of several ways to cast
//! a spell, and a creature type. The interaction model has always been able to
//! answer all four — `Interaction::choose_index` and
//! `Interaction::confirm` — and until now nothing in the client called
//! either, so a tapped Underground Sea asked "Choose a colour" and the prompt
//! bar drew that sentence with no buttons under it. The game stopped there
//! for good, the first time such a land was tapped for its mana.
//!
//! This module is the list. It is a sibling of [`crate::abilities`] and for
//! the same reason: the *label* is what needs to know about mana symbols and
//! seat names, which is knowledge `baylee-client-core` does not carry.
//!
//! [`Prompt`] is what it reads, not [`baylee_engine::choice::Pending`] — the
//! prompt already carries every option the engine offered, in the engine's
//! order, and that order is the answer, so nothing here may sort.
//!
//! **A creature type is missing on purpose.** [`Prompt::ChooseSubtype`] offers
//! all three hundred and fifty of them, and three hundred and fifty buttons is
//! not a chooser; it needs a type-to-filter field, which is its own piece of
//! work. The model answers it today ([`baylee_client_core::interaction`] has
//! the mode and the test); the renderer does not, and [`options`] returns
//! `None` for it rather than pretending.

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::interaction::Prompt;
use baylee_client_core::manapip::{self, Pip};
use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
use baylee_view::GameStatic;

/// One row of an indexed chooser.
#[derive(Clone, PartialEq, Debug)]
pub struct ChoiceOption {
    /// What the button says. Empty when the pip alone carries it, which is
    /// the colour chooser: a `{U}` disc says "blue" in every language there
    /// is, and the word beside it would only be the same claim twice.
    pub label: String,
    /// A mana symbol drawn before the label.
    pub pip: Option<Pip>,
    /// A cost drawn after the label, for the cast options that have one.
    pub cost: Option<ManaCost>,
}

impl ChoiceOption {
    /// A row that is only words.
    fn text(label: String) -> Self {
        Self {
            label,
            pip: None,
            cost: None,
        }
    }
}

/// The symbol for one colour of mana.
///
/// [`manapip::of_color`] takes a [`baylee_core::color::Color`], which has no
/// colourless arm; the engine's choice list is [`ManaColor`], which does. So
/// the mapping is spelled out here rather than half-converted.
fn colour_pip(color: ManaColor) -> Pip {
    manapip::pip(match color {
        ManaColor::White => ManaSymbol::White,
        ManaColor::Blue => ManaSymbol::Blue,
        ManaColor::Black => ManaSymbol::Black,
        ManaColor::Red => ManaSymbol::Red,
        ManaColor::Green => ManaSymbol::Green,
        ManaColor::Colorless => ManaSymbol::Colorless,
    })
}

/// The rows of an indexed choice, or `None` when this prompt is not one.
///
/// The position in the returned list *is* the answer, so a caller passes the
/// index straight to [`baylee_client_core::interaction::Interaction::choose_index`].
#[must_use]
pub fn options(
    prompt: &Prompt,
    lang: Lang,
    statics: Option<&GameStatic>,
) -> Option<Vec<ChoiceOption>> {
    match prompt {
        Prompt::ChooseColor { options } => Some(
            options
                .iter()
                .map(|c| ChoiceOption {
                    label: String::new(),
                    pip: Some(colour_pip(*c)),
                    cost: None,
                })
                .collect(),
        ),
        Prompt::ChoosePlayer { options } => Some(
            options
                .iter()
                .map(|p| {
                    // A seat with no identity is still a seat that has to be
                    // pickable, so it gets its number rather than no row.
                    let name = statics
                        .and_then(|s| s.seats.iter().find(|seat| seat.player == *p))
                        .map(|seat| seat.display_name.clone());
                    ChoiceOption::text(name.unwrap_or_else(|| format!("#{}", p.get())))
                })
                .collect(),
        ),
        Prompt::CastMode { options } => Some(
            options
                .iter()
                .map(|desc| ChoiceOption {
                    label: cast_label(desc.kind, lang),
                    pip: None,
                    // The cost is the part that actually distinguishes two
                    // alternative costs from each other; the words above it
                    // only say which kind of thing it is.
                    cost: (!desc.cost.is_empty()).then_some(desc.cost),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// What one cast option is called.
fn cast_label(kind: baylee_engine::choice::CastModeKind, lang: Lang) -> String {
    use baylee_engine::choice::CastModeKind as K;
    match kind {
        K::Normal => Phrase::CastNormal.text(lang).to_string(),
        K::Alternative(_) => Phrase::CastAlternative.text(lang).to_string(),
        // One-based, because the printed card numbers its modes from one and
        // a player reads the card, not the index.
        K::Mode(i) => Phrase::CastModeNumber.fill(lang, &[&(i + 1).to_string()]),
        K::Face(_) => Phrase::CastBackFace.text(lang).to_string(),
        K::PlayLandFace(_) => Phrase::CastLandFace.text(lang).to_string(),
        K::Miracle => Phrase::CastMiracle.text(lang).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::{CastModeDesc, CastModeKind};

    #[test]
    fn a_colour_row_is_a_symbol_and_no_word() {
        let rows = options(
            &Prompt::ChooseColor {
                options: vec![ManaColor::Blue, ManaColor::Black],
            },
            Lang::En,
            None,
        )
        .expect("a colour choice has rows");
        assert_eq!(rows.len(), 2, "one row per offered colour, in engine order");
        assert!(rows.iter().all(|r| r.pip.is_some() && r.label.is_empty()));
        assert_ne!(rows[0].pip, rows[1].pip, "two colours, two symbols");
    }

    #[test]
    fn a_seat_row_says_the_name_the_table_shows() {
        let statics = GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(1),
                display_name: "House AI".into(),
                is_ai: true,
                team: None,
            }],
            prints: vec![],
        };
        let rows = options(
            &Prompt::ChoosePlayer {
                options: vec![PlayerId::new(1), PlayerId::new(7)],
            },
            Lang::En,
            Some(&statics),
        )
        .expect("a player choice has rows");
        assert_eq!(rows[0].label, "House AI");
        // A seat the statics do not describe still gets a row: a chooser that
        // dropped it would offer fewer answers than the engine did, and the
        // index of everything after it would name the wrong seat.
        assert_eq!(rows[1].label, "#7");
    }

    #[test]
    fn a_cast_option_names_its_kind_and_carries_its_cost() {
        let desc = |kind, cost: &str| CastModeDesc {
            index: 0,
            kind,
            cost: ManaCost::try_parse(cost).expect("a cost"),
        };
        let rows = options(
            &Prompt::CastMode {
                options: vec![
                    desc(CastModeKind::Normal, "{2}{U}"),
                    desc(CastModeKind::Mode(1), "{U}"),
                ],
            },
            Lang::En,
            None,
        )
        .expect("a cast choice has rows");
        assert_eq!(rows[0].label, "Printed cost");
        assert_eq!(rows[1].label, "Mode 2", "modes are numbered as printed");
        assert!(rows.iter().all(|r| r.cost.is_some()));
    }

    /// The two prompts this module deliberately does not answer, and the one
    /// that is a click on the board. A chooser row drawn for any of them
    /// would be a second, wrong way to answer.
    #[test]
    fn prompts_that_are_not_indexed_choices_have_no_rows() {
        assert!(options(&Prompt::ChooseSubtype, Lang::En, None).is_none());
        assert!(options(&Prompt::OrderObjects, Lang::En, None).is_none());
        assert!(options(&Prompt::ChooseTargets { min: 1, max: 1 }, Lang::En, None).is_none());
    }
}
