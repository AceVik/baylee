//! The constructed card face: what a client draws when there is no image.
//!
//! # Why a card needs a second representation
//!
//! A printed card image is one JPEG from a CDN, and it fails in three ordinary
//! ways: the network is gone, the printing has no artwork at that size, or the
//! player simply cannot read the language it was printed in. In all three the
//! game must stay playable, so the client has to be able to *build* a card face
//! out of what it already knows.
//!
//! # What wins over what
//!
//! The face merges two sources that disagree on purpose:
//!
//! - [`baylee_view::PublicObject`] carries the **projected** characteristics —
//!   the answer after the layer system ran. A Mountain animated into a 4/4 is a
//!   creature here, and a clone of Serra Angel is named Serra Angel.
//! - [`CardText`] carries the **printed** text of the card the object came
//!   from, fetched from the gateway catalog in the player's language.
//!
//! Where they disagree the projection wins, because it is the rules answer and
//! the printed card is merely where the object started. The one thing the
//! projection cannot supply is prose: rules text is only ever the printed text.
//!
//! Everything here is pure data, so a test can assert that an animated land
//! reads `Land Creature — Forest Elemental` without starting a renderer.

use baylee_core::color::ColorSet;
use baylee_core::generated::subtypes;
use baylee_core::mana::{ManaCost, ManaSymbol};
use baylee_core::types::{SubtypeKind, SubtypeSet, SupertypeSet, TypeSet};
use baylee_view::PublicObject;
use serde::{Deserialize, Serialize};

/// One printing's text, as the gateway's `/catalog/text` serves it.
///
/// The field names are the wire contract with `baylee-catalog`. The two are
/// deliberately not the same type: dragging the catalog's types over here
/// would drag an ORM and a Postgres driver into a crate that has to compile
/// for wasm. A test on each side pins the JSON so the two cannot drift.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct CardTextEntry {
    /// The printing id that was requested.
    pub scryfall_id: String,
    /// Language actually served.
    pub lang: String,
    /// Faces in printed order.
    pub faces: Vec<FaceText>,
}

/// The text of one face, as it arrives on the wire.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct FaceText {
    /// Name in the served language.
    pub name: String,
    /// English name of the same face.
    pub english_name: String,
    /// Type line in the served language.
    pub type_line: String,
    /// Rules text in the served language.
    pub oracle_text: String,
    /// Mana cost in Scryfall notation.
    pub mana_cost: String,
}

impl CardTextEntry {
    /// Turns one face into the model the renderer consumes.
    #[must_use]
    pub fn face(&self, index: usize) -> Option<CardText> {
        let face = self.faces.get(index)?;
        Some(CardText {
            lang: self.lang.clone(),
            name: face.name.clone(),
            type_line: face.type_line.clone(),
            oracle_text: face.oracle_text.clone(),
            mana_cost: face.mana_cost.clone(),
            english_name: face.english_name.clone(),
        })
    }
}

/// Printed card text for one face, resolved to a language.
///
/// The gateway falls back to English field by field, so everything here is
/// already the best text available for the requested language; the client
/// never has to reason about a partial translation.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct CardText {
    /// Language actually served (`"en"`, `"de"`, …), after the fallback.
    pub lang: String,
    /// Card name in the served language.
    pub name: String,
    /// Type line in the served language.
    pub type_line: String,
    /// Rules text in the served language, paragraphs separated by newlines.
    pub oracle_text: String,
    /// Mana cost in Scryfall notation (`{1}{W}{W}`); language-independent.
    pub mana_cost: String,
    /// The English name of the same card.
    ///
    /// Needed to answer one question the localized name cannot: is the object
    /// on the table still the card this text describes? A clone carries the
    /// copied name in its view, and printing the original's text over it would
    /// be a lie. Comparing against the English name is what detects that.
    pub english_name: String,
}

/// One block of rules text.
///
/// Reminder text is separated rather than concatenated because it is rendered
/// differently — italic, dimmer, and the first thing to drop when a card is
/// drawn small enough that only the real rules fit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TextBlock {
    /// Actual rules text.
    Rules(String),
    /// Parenthesised reminder text (CR 207.2).
    Reminder(String),
}

impl TextBlock {
    /// The text without its kind.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Rules(t) | Self::Reminder(t) => t,
        }
    }
}

/// The numbers in a card's bottom-right box.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stats {
    /// A creature's power and toughness, with damage already marked.
    PowerToughness {
        /// Projected power.
        power: i16,
        /// Projected toughness.
        toughness: i16,
        /// Damage marked this turn.
        damage: u16,
    },
    /// A planeswalker's loyalty.
    Loyalty(u16),
}

/// A card face ready to be laid out by a renderer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CardFace {
    /// Name as it should be shown.
    pub name: String,
    /// Mana cost symbols, left to right.
    pub cost: Vec<ManaSymbol>,
    /// Full type line, e.g. `Legendary Creature — Human Cleric`.
    pub type_line: String,
    /// Rules text, split into blocks.
    pub body: Vec<TextBlock>,
    /// Power/toughness or loyalty, when the card has either.
    pub stats: Option<Stats>,
    /// Projected colors, for the frame.
    pub colors: ColorSet,
    /// Whether the rules text is still missing.
    ///
    /// True when no catalog text was available — the face is still drawable
    /// (name, cost, type line and stats all come from the view), and a
    /// renderer uses this to show a quiet placeholder instead of a blank box.
    pub text_pending: bool,
}

/// The characteristics a face is built from.
///
/// Two sources produce this: the projection of an object on the battlefield,
/// and the printed card for anything the view describes more thinly — a card
/// in hand arrives as a [`baylee_view::HandObject`], which carries no subtypes
/// and no power, because the board never needed them. Both end up here so the
/// face is assembled once.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Characteristics {
    /// Name.
    pub name: String,
    /// Card types.
    pub types: TypeSet,
    /// Supertypes.
    pub supertypes: SupertypeSet,
    /// Subtypes.
    pub subtypes: SubtypeSet,
    /// Colors.
    pub colors: ColorSet,
    /// Power, for creatures.
    pub power: Option<i16>,
    /// Toughness, for creatures.
    pub toughness: Option<i16>,
    /// Loyalty, for planeswalkers.
    pub loyalty: Option<u16>,
    /// Damage marked this turn.
    pub damage: u16,
}

impl Characteristics {
    /// The projected characteristics of an object on the board.
    #[must_use]
    pub fn projected(object: &PublicObject) -> Self {
        Self {
            name: object.name.clone(),
            types: object.types,
            supertypes: object.supertypes,
            subtypes: object.subtypes,
            colors: object.colors,
            power: object.power,
            toughness: object.toughness,
            loyalty: object.loyalty,
            damage: object.damage,
        }
    }
}

impl CardFace {
    /// Builds the face for an object on the board.
    #[must_use]
    pub fn from_object(
        object: &PublicObject,
        printed_cost: Option<&ManaCost>,
        text: Option<&CardText>,
    ) -> Self {
        Self::build(&Characteristics::projected(object), printed_cost, text)
    }

    /// Builds the face from characteristics.
    ///
    /// `printed_cost` comes from the compiled card registry and `text` from the
    /// gateway catalog; both are optional, and the face degrades one field at a
    /// time rather than refusing to render.
    #[must_use]
    pub fn build(
        object: &Characteristics,
        printed_cost: Option<&ManaCost>,
        text: Option<&CardText>,
    ) -> Self {
        // Is the object still the card the text describes? A clone, a
        // face-down creature, or anything that changed its name is not, and
        // then the printed prose does not belong to it.
        let text = text.filter(|t| t.english_name == object.name || t.name == object.name);

        let name = text.map_or_else(|| object.name.clone(), |t| t.name.clone());

        let cost = printed_cost.map_or_else(
            || {
                text.and_then(|t| ManaCost::try_parse(&t.mana_cost).ok())
                    .map(|c| c.symbols().collect())
                    .unwrap_or_default()
            },
            |c| c.symbols().collect(),
        );

        // The printed type line is only usable when nothing changed the
        // object's types; otherwise it would contradict the board.
        let projected = projected_type_line(object);
        let type_line = match text {
            Some(t) if !t.type_line.is_empty() && same_type_line(&t.type_line, &projected) => {
                t.type_line.clone()
            }
            _ => projected,
        };

        Self {
            name,
            cost,
            type_line,
            body: text
                .map(|t| split_blocks(&t.oracle_text))
                .unwrap_or_default(),
            stats: stats(object),
            colors: object.colors,
            text_pending: text.is_none(),
        }
    }
}

/// Power/toughness for a creature, loyalty for a planeswalker.
fn stats(object: &Characteristics) -> Option<Stats> {
    if let (Some(power), Some(toughness)) = (object.power, object.toughness) {
        return Some(Stats::PowerToughness {
            power,
            toughness,
            damage: object.damage,
        });
    }
    object.loyalty.map(Stats::Loyalty)
}

/// Whether a printed type line describes the same types as the projected one.
///
/// Compared on words rather than bytes: the em dash, spacing and the exact
/// subtype order differ harmlessly between Scryfall's string and the one built
/// here, and none of those differences mean the object changed.
fn same_type_line(printed: &str, projected: &str) -> bool {
    let words = |s: &str| {
        let mut w: Vec<String> = s
            .split(|c: char| c.is_whitespace() || c == '—' || c == '-')
            .filter(|p| !p.is_empty())
            .map(str::to_lowercase)
            .collect();
        w.sort();
        w
    };
    words(printed) == words(projected)
}

/// Builds a type line out of an object's projected characteristics.
///
/// Subtypes are grouped behind the type they belong to and in the same order,
/// which is what makes Dryad Arbor read `Land Creature — Forest Dryad` and not
/// `Land Creature — Dryad Forest`.
fn projected_type_line(object: &Characteristics) -> String {
    let mut line = String::with_capacity(48);
    for word in object.supertypes.words() {
        line.push_str(word);
        line.push(' ');
    }
    let types: Vec<&str> = object.types.words().collect();
    line.push_str(&types.join(" "));

    let subs = subtype_words(object.types, object.subtypes);
    if !subs.is_empty() {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str("— ");
        line.push_str(&subs.join(" "));
    }
    line.trim().to_string()
}

/// The subtype words, grouped by kind in printed type order.
fn subtype_words(types: TypeSet, subtypes: SubtypeSet) -> Vec<String> {
    if subtypes.is_empty() {
        return Vec::new();
    }
    // Changeling (CR 702.73) sets every creature type at once. Printing three
    // hundred of them is useless to a player and would blow up the layout, so
    // the whole block collapses to the phrase the rules themselves use.
    let all_creature_types = subtypes.contains_all(SubtypeSet::ALL_CREATURE);

    let mut out = Vec::new();
    for (mask, kind) in type_kind_order() {
        if !types.contains(mask) {
            continue;
        }
        if kind == SubtypeKind::Creature && all_creature_types {
            out.push("All creature types".to_string());
            continue;
        }
        for id in subtypes.iter() {
            if subtypes::kind(id) == kind
                && let Some(name) = subtypes::name(id)
            {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// Which subtype kind belongs to which card type, in printed order.
fn type_kind_order() -> [(TypeSet, SubtypeKind); 6] {
    [
        (TypeSet::ARTIFACT, SubtypeKind::Artifact),
        (TypeSet::ENCHANTMENT, SubtypeKind::Enchantment),
        (TypeSet::LAND, SubtypeKind::Land),
        (TypeSet::CREATURE, SubtypeKind::Creature),
        (TypeSet::PLANESWALKER, SubtypeKind::Planeswalker),
        (
            TypeSet::INSTANT
                .union(TypeSet::SORCERY)
                .union(TypeSet::KINDRED),
            SubtypeKind::Spell,
        ),
    ]
}

/// Splits oracle text into rules and reminder blocks.
///
/// Paragraphs are newline-separated on Scryfall; within a paragraph, anything
/// in parentheses is reminder text. Nesting does not occur in printed text, so
/// a depth counter is enough and an unbalanced parenthesis degrades to rules
/// text rather than swallowing the rest of the card.
fn split_blocks(oracle: &str) -> Vec<TextBlock> {
    let mut blocks = Vec::new();
    for paragraph in oracle.split('\n') {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        let mut buffer = String::new();
        let mut depth = 0usize;
        for ch in paragraph.chars() {
            match ch {
                '(' if depth == 0 => {
                    push_block(&mut blocks, &mut buffer, false);
                    depth = 1;
                }
                '(' => depth += 1,
                ')' if depth == 1 => {
                    push_block(&mut blocks, &mut buffer, true);
                    depth = 0;
                }
                ')' if depth > 1 => depth -= 1,
                _ => buffer.push(ch),
            }
        }
        // An unclosed parenthesis leaves depth > 0; the tail is still text the
        // player needs to read, so it is kept rather than dropped.
        push_block(&mut blocks, &mut buffer, depth > 0);
    }
    blocks
}

/// Flushes the buffer into a block, dropping it when it holds only spacing.
fn push_block(blocks: &mut Vec<TextBlock>, buffer: &mut String, reminder: bool) {
    let text = buffer.trim().to_string();
    buffer.clear();
    if text.is_empty() {
        return;
    }
    blocks.push(if reminder {
        TextBlock::Reminder(text)
    } else {
        TextBlock::Rules(text)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::token;
    use baylee_core::types::SupertypeSet;

    fn text(name: &str, type_line: &str, oracle: &str) -> CardText {
        CardText {
            lang: "en".to_string(),
            name: name.to_string(),
            type_line: type_line.to_string(),
            oracle_text: oracle.to_string(),
            mana_cost: "{1}{W}".to_string(),
            english_name: name.to_string(),
        }
    }

    #[test]
    fn a_plain_creature_reads_like_its_printed_card() {
        let mut obj = token(1, 0, "Ondu Cleric", 1, 1);
        obj.supertypes = SupertypeSet::EMPTY;
        obj.subtypes = SubtypeSet::from_slice(&[
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC,
            subtypes::creature::ALLY,
        ]);
        let t = text("Ondu Cleric", "Creature — Human Cleric Ally", "Whenever...");
        let face = CardFace::from_object(&obj, Some(&baylee_core::mana!("{1}{W}")), Some(&t));

        assert_eq!(face.name, "Ondu Cleric");
        assert_eq!(face.type_line, "Creature — Human Cleric Ally");
        assert_eq!(face.cost.len(), 2);
        assert!(!face.text_pending);
        assert_eq!(
            face.stats,
            Some(Stats::PowerToughness {
                power: 1,
                toughness: 1,
                damage: 0
            })
        );
    }

    /// The whole point of the projection: an animated land is a creature on
    /// the board, and its face has to say so even though the printed card is
    /// a land with no power at all.
    #[test]
    fn an_animated_land_reads_as_the_creature_it_became() {
        let mut obj = token(2, 0, "Forest", 4, 4);
        obj.types = TypeSet::LAND.union(TypeSet::CREATURE);
        obj.subtypes =
            SubtypeSet::from_slice(&[subtypes::land::FOREST, subtypes::creature::ELEMENTAL]);
        // The catalog still describes the printed land.
        let t = text("Forest", "Basic Land — Forest", "({T}: Add {G}.)");
        let face = CardFace::from_object(&obj, None, Some(&t));

        assert_eq!(face.type_line, "Land Creature — Forest Elemental");
    }

    /// Subtypes group behind their own type, in the type's printed order —
    /// Dryad Arbor is `Land Creature — Forest Dryad`, never the reverse.
    #[test]
    fn subtypes_follow_the_type_they_belong_to() {
        let mut obj = token(3, 0, "Dryad Arbor", 1, 1);
        obj.types = TypeSet::LAND.union(TypeSet::CREATURE);
        obj.subtypes = SubtypeSet::from_slice(&[subtypes::creature::DRYAD, subtypes::land::FOREST]);
        let face = CardFace::from_object(&obj, None, None);
        assert_eq!(face.type_line, "Land Creature — Forest Dryad");
    }

    /// Changeling sets every creature type; printing them would be unreadable
    /// and would wreck any layout.
    #[test]
    fn changeling_collapses_instead_of_printing_three_hundred_types() {
        let mut obj = token(4, 0, "Woodland Changeling", 2, 2);
        obj.subtypes = SubtypeSet::ALL_CREATURE;
        let face = CardFace::from_object(&obj, None, None);
        assert_eq!(face.type_line, "Creature — All creature types");
    }

    /// A clone carries the copied name; the original's rules text must not be
    /// printed over it.
    #[test]
    fn text_is_dropped_when_the_object_is_no_longer_that_card() {
        let obj = token(5, 0, "Serra Angel", 4, 4);
        let t = text(
            "Clone",
            "Creature — Shapeshifter",
            "You may have Clone enter...",
        );
        let face = CardFace::from_object(&obj, None, Some(&t));

        assert_eq!(face.name, "Serra Angel");
        assert!(face.body.is_empty());
        assert!(face.text_pending);
    }

    #[test]
    fn reminder_text_is_separated_from_rules_text() {
        let obj = token(6, 0, "Flier", 2, 2);
        let t = text(
            "Flier",
            "Creature — Bird",
            "Flying (This creature can't be blocked except by creatures with flying or reach.)\nVigilance",
        );
        let face = CardFace::from_object(&obj, None, Some(&t));

        assert_eq!(
            face.body,
            vec![
                TextBlock::Rules("Flying".to_string()),
                TextBlock::Reminder(
                    "This creature can't be blocked except by creatures with flying or reach."
                        .to_string()
                ),
                TextBlock::Rules("Vigilance".to_string()),
            ]
        );
    }

    /// An unbalanced parenthesis is a data bug, but losing the rest of a
    /// card's text over it would be a gameplay bug.
    #[test]
    fn an_unclosed_parenthesis_keeps_its_text() {
        let obj = token(7, 0, "Broken", 1, 1);
        let t = text("Broken", "Creature — Ox", "Trample (this never closes");
        let face = CardFace::from_object(&obj, None, Some(&t));
        assert_eq!(face.body.len(), 2);
        assert_eq!(face.body[0], TextBlock::Rules("Trample".to_string()));
        assert_eq!(face.body[1].text(), "this never closes");
    }

    /// Without catalog text the face still has to be drawable — that is the
    /// offline and first-launch case, not an error state.
    #[test]
    fn a_face_without_catalog_text_still_carries_everything_the_view_knows() {
        let mut obj = token(8, 0, "Grizzly Bears", 2, 2);
        obj.subtypes = SubtypeSet::from_slice(&[subtypes::creature::BEAR]);
        let face = CardFace::from_object(&obj, Some(&baylee_core::mana!("{1}{G}")), None);

        assert_eq!(face.name, "Grizzly Bears");
        assert_eq!(face.type_line, "Creature — Bear");
        assert_eq!(face.cost.len(), 2);
        assert!(face.text_pending);
        assert!(face.body.is_empty());
    }

    #[test]
    fn a_planeswalker_shows_loyalty_instead_of_power() {
        let mut obj = token(9, 0, "Teferi", 0, 0);
        obj.types = TypeSet::PLANESWALKER;
        obj.power = None;
        obj.toughness = None;
        obj.loyalty = Some(4);
        obj.subtypes = SubtypeSet::from_slice(&[subtypes::planeswalker::TEFERI]);
        let face = CardFace::from_object(&obj, None, None);

        assert_eq!(face.type_line, "Planeswalker — Teferi");
        assert_eq!(face.stats, Some(Stats::Loyalty(4)));
    }

    /// The wire shape between the gateway catalog and this crate is two
    /// independent structs that must serialize identically. `baylee-catalog`
    /// has the mirror of this test; together they turn a rename on either side
    /// into a failure instead of a card that silently loses its text.
    #[test]
    fn the_catalog_wire_shape_is_pinned() {
        let entry = CardTextEntry {
            scryfall_id: "id".to_string(),
            lang: "de".to_string(),
            faces: vec![FaceText {
                name: "Wald".to_string(),
                english_name: "Forest".to_string(),
                type_line: "Basisland — Wald".to_string(),
                oracle_text: "({T}: Erzeuge {G}.)".to_string(),
                mana_cost: String::new(),
            }],
        };
        assert_eq!(
            serde_json::to_string(&entry).expect("serializes"),
            r#"{"scryfall_id":"id","lang":"de","faces":[{"name":"Wald","english_name":"Forest","type_line":"Basisland — Wald","oracle_text":"({T}: Erzeuge {G}.)","mana_cost":""}]}"#
        );
    }

    #[test]
    fn a_wire_entry_becomes_the_model_the_renderer_uses() {
        let entry = CardTextEntry {
            scryfall_id: "id".to_string(),
            lang: "en".to_string(),
            faces: vec![FaceText {
                name: "Forest".to_string(),
                english_name: "Forest".to_string(),
                type_line: "Basic Land — Forest".to_string(),
                oracle_text: "({T}: Add {G}.)".to_string(),
                mana_cost: String::new(),
            }],
        };
        let text = entry.face(0).expect("face 0 exists");
        assert_eq!(text.name, "Forest");
        assert_eq!(text.lang, "en");
        assert!(entry.face(1).is_none());
    }

    /// The localized line is kept only while it still describes the object.
    #[test]
    fn a_localized_type_line_survives_when_nothing_changed_the_types() {
        let mut obj = token(10, 0, "Wald", 0, 0);
        obj.types = TypeSet::LAND;
        obj.power = None;
        obj.toughness = None;
        obj.supertypes = SupertypeSet::BASIC;
        obj.subtypes = SubtypeSet::from_slice(&[subtypes::land::FOREST]);
        let mut t = text("Wald", "Basisland — Wald", "({T}: Erzeuge {G}.)");
        t.lang = "de".to_string();
        t.english_name = "Forest".to_string();
        // The object shows the localized name, which is how the client knows
        // it is still that card.
        let face = CardFace::from_object(&obj, None, Some(&t));
        assert_eq!(face.name, "Wald");
    }
}
