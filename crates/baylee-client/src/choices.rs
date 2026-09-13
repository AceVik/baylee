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
//! card faces, which is knowledge `baylee-client-core` does not carry. A
//! seat's name is the one that came back — every line that talks about
//! another chair wants it, so it is [`baylee_client_core::i18n::seat_name`]
//! and this list asks for it like everybody else.
//!
//! [`Prompt`] is what it reads, not [`baylee_engine::choice::Pending`] — the
//! prompt already carries every option the engine offered, in the engine's
//! order, and that order is the answer, so nothing here may sort.
//!
//! **A creature type is the awkward one.** [`Prompt::ChooseSubtype`] offers all
//! three hundred and fifty of them, and three hundred and fifty buttons is not
//! a chooser — so it is the one choice with a `filter` in front of it, and the
//! only one where a row's position in the returned list is *not* its answer.
//! That is what [`ChoiceOption::index`] is for. Cavern of Souls asks this
//! question as it *enters*, so a client that cannot answer it loses the game
//! on a land drop.

use baylee_client_core::i18n::{Lang, Phrase, seat_name};
use baylee_client_core::interaction::Prompt;
use baylee_client_core::manapip::{self, Pip};
use baylee_core::generated::subtypes;
use baylee_core::ids::{ObjectId, SubtypeId};
use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
use baylee_view::GameStatic;

/// Where a row that names a card face looks that name up.
///
/// Two steps, and they live in two places: the object says which card and
/// which printing (the view), and the printing says what the face is called
/// in the player's own language (the catalog's text). Both are optional, and
/// each absence is honest rather than a failure — the view has not arrived
/// yet, the gateway serves no card text, or the caller is one of the two that
/// read only the *shape* of the list and never draw a label at all.
///
/// A face that cannot be named falls back to the phrase the row carried
/// before, which still says what kind of option it is.
#[derive(Clone, Copy, Default)]
pub struct FaceNames<'a> {
    /// The seat's view. Without it nothing can be named.
    pub view: Option<&'a baylee_view::PlayerView>,
    /// The catalog's text. Without it a name is the registry's English.
    pub texts: Option<&'a crate::cardtext::CardTexts>,
}

impl FaceNames<'_> {
    /// What face `face` of `object` is called.
    fn of(self, object: ObjectId, face: usize) -> Option<String> {
        crate::face::face_name(object, face, self.view?, self.texts)
    }
}

/// One row of an indexed chooser.
#[derive(Clone, PartialEq, Debug)]
pub struct ChoiceOption {
    /// Which of the engine's options this row answers.
    ///
    /// The same as the row's position for every choice but a creature type,
    /// where a filter has hidden most of the list. Carrying it means the
    /// filter can never make a button send the wrong answer.
    pub index: usize,
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
    fn text(index: usize, label: String) -> Self {
        Self {
            index,
            label,
            pip: None,
            cost: None,
        }
    }
}

/// How many creature types the chooser draws at once.
///
/// The engine offers every type there is, which is not a list anybody reads;
/// it is one they filter. Twelve is what the prompt bar holds, and the hint
/// line under the headline is what says so.
pub const SUBTYPE_ROWS: usize = 12;

/// The creature types matching `filter`, in the engine's order.
///
/// A prefix match and not a substring one, because typing `el` to be offered
/// "Elf" and "Elemental" is the behaviour a player predicts; `Rebel` turning
/// up as well is not. A type the generated table has no name for still gets a
/// row — by its number, the same rule as a seat the statics do not describe —
/// because the alternative is an option the engine offered and nobody can
/// pick.
fn subtype_rows(options: &[SubtypeId], filter: &str) -> Vec<ChoiceOption> {
    let needle = filter.trim().to_ascii_lowercase();
    options
        .iter()
        .enumerate()
        .filter_map(|(i, id)| match subtypes::name(*id) {
            Some(name) => name
                .to_ascii_lowercase()
                .starts_with(&needle)
                .then(|| ChoiceOption::text(i, name.to_string())),
            None => needle
                .is_empty()
                .then(|| ChoiceOption::text(i, format!("#{}", id.get()))),
        })
        .take(SUBTYPE_ROWS)
        .collect()
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
/// [`ChoiceOption::index`] is the answer, and a caller passes *that* to
/// `Interaction::choose_index` — not the row's position, which stops being
/// the same thing as soon as `filter` hides part of the list. `filter` is
/// read for a creature type and ignored by every other choice.
///
/// The overlay and the click handler call this same function, so a button can
/// only ever send an answer the current prompt is still offering.
#[must_use]
pub fn options(
    prompt: &Prompt,
    lang: Lang,
    statics: Option<&GameStatic>,
    filter: &str,
    names: FaceNames<'_>,
) -> Option<Vec<ChoiceOption>> {
    match prompt {
        Prompt::ChooseColor { options } => Some(
            options
                .iter()
                .enumerate()
                .map(|(i, c)| ChoiceOption {
                    index: i,
                    label: String::new(),
                    pip: Some(colour_pip(*c)),
                    cost: None,
                })
                .collect(),
        ),
        Prompt::ChoosePlayer { options } => Some(
            options
                .iter()
                .enumerate()
                // A seat with no identity is still a seat that has to be
                // pickable, so `seat_name` numbers it rather than dropping
                // the row — in the words a player is shown everywhere else,
                // which this list used to spell `#7`.
                .map(|(i, p)| ChoiceOption::text(i, seat_name(lang, statics, *p)))
                .collect(),
        ),
        Prompt::CastMode { object, options } => Some(
            options
                .iter()
                .enumerate()
                .map(|(i, desc)| ChoiceOption {
                    index: i,
                    label: cast_label(desc.kind, lang, *object, names),
                    pip: None,
                    // The cost is the part that actually distinguishes two
                    // alternative costs from each other; the words above it
                    // only say which kind of thing it is. It is *also* what
                    // fails for a pathway, whose two options cost nothing and
                    // differ only in the name they print — which is why the
                    // label names the face rather than the kind.
                    cost: (!desc.cost.is_empty()).then_some(desc.cost),
                })
                .collect(),
        ),
        Prompt::ChooseSubtype { options } => Some(subtype_rows(options, filter)),
        _ => None,
    }
}

/// What one cast option is called.
///
/// The two options that name a **face** say the face's own name where it can
/// be found, because that is the only thing that tells them apart: a pathway's
/// two land faces are the same kind at the same empty cost and differ in
/// nothing else a row draws. The phrase stays as the fallback, which is what
/// every row said before.
fn cast_label(
    kind: baylee_engine::choice::CastModeKind,
    lang: Lang,
    object: ObjectId,
    names: FaceNames<'_>,
) -> String {
    use baylee_engine::choice::CastModeKind as K;
    match kind {
        K::Normal => Phrase::CastNormal.text(lang).to_string(),
        K::Alternative(_) => Phrase::CastAlternative.text(lang).to_string(),
        // One-based, because the printed card numbers its modes from one and
        // a player reads the card, not the index.
        K::Mode(i) => Phrase::CastModeNumber.fill(lang, &[&(i + 1).to_string()]),
        K::Face(i) => names
            .of(object, i)
            .unwrap_or_else(|| Phrase::CastBackFace.text(lang).to_string()),
        K::PlayLandFace(i) => names
            .of(object, i)
            .unwrap_or_else(|| Phrase::CastLandFace.text(lang).to_string()),
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
            "",
            FaceNames::default(),
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
                away: false,
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
            "",
            FaceNames::default(),
        )
        .expect("a player choice has rows");
        assert_eq!(rows[0].label, "House AI");
        // A seat the statics do not describe still gets a row: a chooser that
        // dropped it would offer fewer answers than the engine did, and the
        // index of everything after it would name the wrong seat. It is
        // numbered in the words the rest of the interface numbers a seat in,
        // which this row used to spell `#7`.
        assert_eq!(rows[1].label, "Seat 7");
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
                object: ObjectId::new(1, 0),
                options: vec![
                    desc(CastModeKind::Normal, "{2}{U}"),
                    desc(CastModeKind::Mode(1), "{U}"),
                ],
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a cast choice has rows");
        assert_eq!(rows[0].label, "Printed cost");
        assert_eq!(rows[1].label, "Mode 2", "modes are numbered as printed");
        assert!(rows.iter().all(|r| r.cost.is_some()));
    }

    /// Brightclimb Pathway, in the hand, as one object with two land faces.
    ///
    /// The hand and not the battlefield because that is where the question is
    /// asked from — and because `PlayerView::object` does not look in the
    /// hand, which is the whole reason [`crate::face::face_name`] searches it
    /// first.
    fn pathway_in_hand() -> (baylee_view::PlayerView, ObjectId) {
        let def = baylee_cards::by_oracle_id("1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9")
            .expect("Brightclimb Pathway is in the pool");
        let id = ObjectId::new(11, 0);
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.hand = vec![baylee_view::HandObject {
            id,
            card: baylee_view::CardIdentity {
                index: def.index,
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            },
            name: def.faces[0].name.to_string(),
            mana_value: 0,
            colors: baylee_core::color::ColorSet::default(),
            types: baylee_core::types::TypeSet::LAND,
            commander: false,
        }];
        (view, id)
    }

    /// The owner's AE11: two buttons that said the same thing.
    ///
    /// A pathway's two options are the same kind at the same empty cost, so
    /// every column a row draws was identical — the label, the missing pip,
    /// the missing cost — and the choice was made blind.
    #[test]
    fn a_pathways_two_land_faces_are_two_different_buttons() {
        let (view, object) = pathway_in_hand();
        let desc = |face: usize| CastModeDesc {
            index: u8::try_from(face).expect("a face index"),
            kind: CastModeKind::PlayLandFace(face),
            cost: ManaCost::ZERO,
        };
        let rows = options(
            &Prompt::CastMode {
                object,
                options: vec![desc(0), desc(1)],
            },
            Lang::En,
            None,
            "",
            FaceNames {
                view: Some(&view),
                texts: None,
            },
        )
        .expect("a cast choice has rows");
        assert_eq!(rows[0].label, "Brightclimb Pathway");
        assert_eq!(rows[1].label, "Grimclimb Pathway");
        assert!(
            rows.iter().all(|r| r.cost.is_none()),
            "a land face costs nothing, which is why the label has to carry it"
        );
    }

    /// The rung under it: nothing to look a name up with is not a blank row.
    ///
    /// A view that has not arrived, an object the seat cannot place, a card
    /// the registry does not have — each leaves the phrase that says what kind
    /// of option this is, which is what every row said before.
    #[test]
    fn a_face_that_cannot_be_named_keeps_the_words_it_had() {
        let rows = options(
            &Prompt::CastMode {
                object: ObjectId::new(11, 0),
                options: vec![CastModeDesc {
                    index: 0,
                    kind: CastModeKind::PlayLandFace(1),
                    cost: ManaCost::ZERO,
                }],
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a cast choice has rows");
        assert_eq!(rows[0].label, Phrase::CastLandFace.text(Lang::En));
    }

    /// The prompts this module does not answer: they are clicks on the board
    /// or the table, and a chooser row drawn for one would be a second, wrong
    /// way to answer.
    #[test]
    fn prompts_that_are_not_indexed_choices_have_no_rows() {
        assert!(
            options(
                &Prompt::OrderObjects,
                Lang::En,
                None,
                "",
                FaceNames::default(),
            )
            .is_none()
        );
        assert!(
            options(
                &Prompt::ChooseTargets {
                    min: 1,
                    max: 1,
                    reason: baylee_engine::choice::TargetPrompt::Targets,
                },
                Lang::En,
                None,
                "",
                FaceNames::default(),
            )
            .is_none()
        );
    }

    /// Every subtype the generated table knows, which is what the engine
    /// hands over for a Cavern of Souls.
    fn all_subtypes() -> Vec<SubtypeId> {
        (0..400).map(SubtypeId::new).collect()
    }

    #[test]
    fn an_unfiltered_type_list_is_cut_to_what_the_bar_holds() {
        let rows = options(
            &Prompt::ChooseSubtype {
                options: all_subtypes(),
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a subtype choice has rows");
        assert_eq!(
            rows.len(),
            SUBTYPE_ROWS,
            "the bar holds twelve, not four hundred"
        );
        // Cut from the front, in the engine's order, so the indices are the
        // engine's own.
        assert_eq!(rows[0].index, 0);
        assert_eq!(rows[SUBTYPE_ROWS - 1].index, SUBTYPE_ROWS - 1);
    }

    /// The property the whole `index` field exists for: once a filter has
    /// hidden part of the list, a row's position is no longer its answer.
    #[test]
    fn a_filtered_row_still_carries_the_engine_s_own_index() {
        let all = all_subtypes();
        let elf = all
            .iter()
            .position(|id| subtypes::name(*id) == Some("Elf"))
            .expect("the generated table knows about elves");
        let rows = options(
            &Prompt::ChooseSubtype { options: all },
            Lang::En,
            None,
            "elf",
            FaceNames::default(),
        )
        .expect("a subtype choice has rows");
        assert_eq!(rows[0].label, "Elf", "a prefix match, case-insensitively");
        assert_ne!(rows[0].index, 0, "an elf is not the first creature type");
        assert_eq!(rows[0].index, elf, "the row answers the engine's option");
    }

    #[test]
    fn a_filter_that_matches_nothing_offers_nothing() {
        let rows = options(
            &Prompt::ChooseSubtype {
                options: all_subtypes(),
            },
            Lang::En,
            None,
            "zzzz",
            FaceNames::default(),
        )
        .expect("a subtype choice still answers, with no rows");
        assert!(
            rows.is_empty(),
            "nothing may be pickable that does not match"
        );
    }
}
