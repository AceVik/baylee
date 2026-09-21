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

/// Does this printed text still describe an object called `name`?
///
/// The **clone guard**. A copy carries the copied name in its view while its
/// cardboard — and therefore its printing, and therefore this text — is still
/// the Clone's own, so the two disagree and only the name says what a player
/// is looking at. The same is true of a face-down permanent and of anything an
/// effect renamed. Comparing against *both* names is what makes it work in a
/// translated client: `name` is the served language and `english_name` is the
/// one the engine projects, and an ordinary card matches on the second.
#[must_use]
pub fn describes(text: &CardText, name: &str) -> bool {
    text.english_name == name || text.name == name
}

/// The name to write for an object whose projected name is `name`.
///
/// This is the **one** place a card name is translated, and it exists because
/// there were six others. Everything a player reads a name off — the stack's
/// title and its subtitle and its target chips, the combat line's aim and the
/// ends of a combat tally, the rows of the zone browser — reads
/// [`PublicObject::name`], which comes out of the compiled card registry and
/// is therefore always English. The localized name is one lookup away in the
/// catalog and was reached only by whatever happened to be building a whole
/// [`CardFace`], so a German client drew a German sentence under an English
/// title.
///
/// Falling back to the projected name is not a failure case. A token, an
/// emblem, a card whose text has not arrived yet and a card the seat may not
/// identify all have nothing else to be called, and [`describes`] deliberately
/// sends a clone the same way: the printing's name would be a lie about which
/// card is on the table.
#[must_use]
pub fn shown_name<'a>(name: &'a str, text: Option<&'a CardText>) -> &'a str {
    match text {
        Some(text) if describes(text, name) => &text.name,
        _ => name,
    }
}

/// What a card's cardboard says it is, out of the compiled registry.
///
/// [`Characteristics`] are what an object **is** right now, after every
/// continuous effect; this is what was **printed** on it. The pair answers
/// the one question the printed type line has to pass before it may be drawn:
/// did anything change this object's types? An animated land, a clone, a
/// changeling and a face-down creature all differ here, and none of them may
/// be described by the line on their own card.
///
/// Three bitsets and not a string, because the string form of that comparison
/// is only ever right in one language — see [`CardFace::build`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrintedTypes {
    /// Supertypes as printed (CR 205.4).
    pub supertypes: SupertypeSet,
    /// Card types as printed (CR 205.2).
    pub types: TypeSet,
    /// Subtypes as printed (CR 205.3).
    pub subtypes: SubtypeSet,
}

impl PrintedTypes {
    /// Whether the object still has exactly the types it was printed with.
    fn still_hold(self, object: &Characteristics) -> bool {
        self.supertypes == object.supertypes
            && self.types == object.types
            && self.subtypes == object.subtypes
    }
}

impl CardFace {
    /// Builds the face for an object on the board.
    #[must_use]
    pub fn from_object(
        object: &PublicObject,
        printed_cost: Option<&ManaCost>,
        printed_types: Option<PrintedTypes>,
        text: Option<&CardText>,
    ) -> Self {
        Self::build(
            &Characteristics::projected(object),
            printed_cost,
            printed_types,
            text,
        )
    }

    /// Builds the face from characteristics.
    ///
    /// `printed_cost` and `printed_types` come from the compiled card registry
    /// and `text` from the gateway catalog; all three are optional, and the
    /// face degrades one field at a time rather than refusing to render.
    #[must_use]
    pub fn build(
        object: &Characteristics,
        printed_cost: Option<&ManaCost>,
        printed_types: Option<PrintedTypes>,
        text: Option<&CardText>,
    ) -> Self {
        // Is the object still the card the text describes? A clone, a
        // face-down creature, or anything that changed its name is not, and
        // then the printed prose does not belong to it.
        let text = text.filter(|t| describes(t, &object.name));

        let name = shown_name(&object.name, text).to_string();

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
        //
        // Asked of the **types** and not of the two lines. Comparing the
        // strings was right in English and wrong everywhere else: the printed
        // line arrives translated (the catalog serves `printed_type_line`
        // where it has one) and the projected line is built out of the
        // engine's own English type words, so the two word sets can never be
        // equal in a German client and every card fell back — a Nistende
        // Falkentaube read `Creature — Bird` under its own name. The bitsets
        // say the same thing exactly, in no language at all.
        //
        // A card the registry cannot be asked about keeps the fallback: a
        // token has no printed line to be right or wrong about, and an
        // ability on the stack borrows its source's text but not its types.
        let type_line = match (text, printed_types) {
            (Some(t), Some(printed)) if !t.type_line.is_empty() && printed.still_hold(object) => {
                t.type_line.clone()
            }
            _ => projected_type_line(object),
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

/// Builds a type line out of an object's projected characteristics.
///
/// Subtypes are grouped behind the type they belong to and in the same order,
/// which is what makes Dryad Arbor read `Land Creature — Forest Dryad` and not
/// `Land Creature — Dryad Forest`.
pub(crate) fn projected_type_line(object: &Characteristics) -> String {
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
            // `Some(kind)`, because an id this build has no name for has no
            // kind either (#43): a host written later assigns numbers past
            // this table, and the old `kind()` answered those with the last
            // kind in the list rather than with nothing.
            if subtypes::kind(id) == Some(kind)
                && let Some(name) = subtypes::name(id)
            {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// Which subtype kind belongs to which card type, in printed order.
fn type_kind_order() -> [(TypeSet, SubtypeKind); 7] {
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
        (TypeSet::BATTLE, SubtypeKind::Battle),
    ]
}

/// Splits oracle text into rules and reminder blocks.
///
/// Paragraphs are newline-separated on Scryfall; within a paragraph, anything
/// in parentheses is reminder text. Nesting does not occur in printed text, so
/// a depth counter is enough and an unbalanced parenthesis degrades to rules
/// text rather than swallowing the rest of the card.
///
/// Public for the one caller that wants a whole card body and has no
/// [`CardFace`] to take it off: the slip under the hover preview, which draws
/// what a *spell* on the stack is about to do from the printing it already
/// holds. [`sentence_blocks`] is this same split over one sentence, which is
/// what an **ability** on the stack needs instead.
#[must_use]
pub fn split_blocks(oracle: &str) -> Vec<TextBlock> {
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

/// The one printed sentence a stack entry stands for, split into rules and
/// reminder blocks.
///
/// The host says *which* sentence an ability on the stack is printed as, as a
/// coordinate into the text rather than as prose, because the text belongs to
/// the client: the player's own printing, in the player's own language. What
/// holds the two together is `of` — the number of sentences the host counted.
/// A text of a different length is refused whole rather than indexed, and
/// that is the case worth the field: an index merely out of range is caught
/// by anyone, while a text one sentence shorter puts the index *in* range and
/// on the sentence beside the right one, which is drawn to the player as
/// precise text and is worse than the label it replaced.
///
/// The split is [`split_blocks`], the same one a whole card is drawn with, so
/// a stack entry and the card it came from never disagree about where the
/// reminder text starts.
#[must_use]
pub fn sentence_blocks(oracle: &str, line: u8, of: u8) -> Option<Vec<TextBlock>> {
    if baylee_core::oracle::sentence_count(oracle) != usize::from(of) {
        return None;
    }
    let sentence = baylee_core::oracle::sentences(oracle).nth(usize::from(line))?;
    Some(split_blocks(sentence))
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

    /// The registry's answer for a fixture nothing has changed: the types it
    /// is standing there with are the types it was printed with.
    fn as_printed(obj: &PublicObject) -> PrintedTypes {
        PrintedTypes {
            supertypes: obj.supertypes,
            types: obj.types,
            subtypes: obj.subtypes,
        }
    }

    /// The fixture names a real card, so it has to name it as the printing
    /// does: Ondu Cleric is a **Kor** Cleric Ally, and said Human here for as
    /// long as the card's own file did. Nothing failed — the fixture supplies
    /// both halves of its own comparison — which is exactly why a wrong word
    /// could sit in it. `xtask validate` now asks the printing.
    /// A bare set of characteristics with the three fields a type line is
    /// made of.
    fn typed(
        supertypes: SupertypeSet,
        types: TypeSet,
        subtypes: &[baylee_core::ids::SubtypeId],
    ) -> Characteristics {
        let mut set = SubtypeSet::EMPTY;
        for id in subtypes {
            set.insert(*id);
        }
        Characteristics {
            supertypes,
            types,
            subtypes: set,
            ..Characteristics::default()
        }
    }

    /// **A supertype is printed in front of the types and the em dash only
    /// appears when something is behind it.** That the subtypes group behind
    /// their own type is [`subtypes_follow_the_type_they_belong_to`]; this is
    /// the rest of the line, which nothing named.
    ///
    /// The dash half is the one worth pinning: `projected_type_line` decides
    /// it from the words it produced rather than from the subtype *set*, so a
    /// permanent carrying a subtype it no longer has a type for still reads
    /// `Land` and never `Land —`.
    #[test]
    fn a_supertype_leads_the_line_and_an_empty_tail_prints_no_dash() {
        use baylee_core::generated::subtypes::land;

        assert_eq!(
            projected_type_line(&typed(
                SupertypeSet::LEGENDARY,
                TypeSet::LAND,
                &[land::FOREST],
            )),
            "Legendary Land — Forest",
            "a supertype is printed before the types and never behind the dash"
        );
        assert_eq!(
            projected_type_line(&typed(SupertypeSet::EMPTY, TypeSet::ARTIFACT, &[])),
            "Artifact",
            "and nothing behind the dash means no dash"
        );
        assert_eq!(
            projected_type_line(&typed(
                SupertypeSet::BASIC,
                TypeSet::LAND,
                &[land::MOUNTAIN]
            )),
            "Basic Land — Mountain"
        );
    }

    /// A subtype whose own type is no longer there is **not printed**, which
    /// is the case an animated land makes: the creature types a Dryad Arbor
    /// carries have nothing over them the moment it stops being a creature,
    /// and a line reading `Land — Dryad` is a permanent no player can parse.
    ///
    /// Pinned as what the renderer does rather than as a rules claim — the
    /// grouping is by the types present, and this is the same walk seen from
    /// the other side.
    #[test]
    fn a_subtype_with_no_type_over_it_is_left_off_the_line() {
        use baylee_core::generated::subtypes::{creature, land};

        assert_eq!(
            projected_type_line(&typed(
                SupertypeSet::EMPTY,
                TypeSet::LAND,
                &[creature::DRYAD, land::FOREST],
            )),
            "Land — Forest",
            "the creature half of Dryad Arbor, with the creature type gone"
        );
        assert_eq!(
            projected_type_line(&typed(
                SupertypeSet::EMPTY,
                TypeSet::LAND,
                &[creature::DRYAD]
            )),
            "Land",
            "and with nothing left to print behind the dash, no dash"
        );
    }

    /// The changeling phrase (CR 702.73) is **one kind's worth of the line
    /// and not the whole of it**, and it is printed only when the set really
    /// holds every creature type.
    ///
    /// That a plain changeling collapses is
    /// [`changeling_collapses_instead_of_printing_three_hundred_types`]; what
    /// is left unsaid there is the pair of cases on either side of it. A
    /// changeling that is also a land keeps its land subtype in front of the
    /// phrase, because the collapse replaces the creature block alone — and
    /// one Shapeshifter is one word, which is the assertion that tells
    /// `contains_all` apart from "has any creature type at all".
    #[test]
    fn the_changeling_phrase_replaces_one_block_and_only_when_it_is_earned() {
        use baylee_core::generated::subtypes::{creature, land};

        let mut all = Characteristics {
            types: TypeSet::LAND.union(TypeSet::CREATURE),
            subtypes: SubtypeSet::ALL_CREATURE,
            ..Characteristics::default()
        };
        all.subtypes.insert(land::FOREST);
        assert_eq!(
            projected_type_line(&all),
            "Land Creature — Forest All creature types"
        );

        assert_eq!(
            projected_type_line(&typed(
                SupertypeSet::EMPTY,
                TypeSet::CREATURE,
                &[creature::SHAPESHIFTER]
            )),
            "Creature — Shapeshifter",
            "one creature type is one word, not the phrase"
        );
    }

    #[test]
    fn a_plain_creature_reads_like_its_printed_card() {
        let mut obj = token(1, 0, "Ondu Cleric", 1, 1);
        obj.supertypes = SupertypeSet::EMPTY;
        obj.subtypes = SubtypeSet::from_slice(&[
            subtypes::creature::KOR,
            subtypes::creature::CLERIC,
            subtypes::creature::ALLY,
        ]);
        let t = text("Ondu Cleric", "Creature — Kor Cleric Ally", "Whenever...");
        let face = CardFace::from_object(
            &obj,
            Some(&baylee_core::mana!("{1}{W}")),
            Some(as_printed(&obj)),
            Some(&t),
        );

        assert_eq!(face.name, "Ondu Cleric");
        assert_eq!(face.type_line, "Creature — Kor Cleric Ally");
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
        // The catalog still describes the printed land, and so does the
        // registry: `Basic Land — Forest` is what the cardboard says, and the
        // types no longer match it.
        let t = text("Forest", "Basic Land — Forest", "({T}: Add {G}.)");
        let printed = PrintedTypes {
            supertypes: SupertypeSet::BASIC,
            types: TypeSet::LAND,
            subtypes: SubtypeSet::from_slice(&[subtypes::land::FOREST]),
        };
        let face = CardFace::from_object(&obj, None, Some(printed), Some(&t));

        assert_eq!(face.type_line, "Land Creature — Forest Elemental");
    }

    /// The same refusal in a language where it is the only thing standing
    /// between a player and a lie.
    ///
    /// The guard used to compare the two *strings*, which meant a German
    /// client refused every printed line and this animated Wald read like an
    /// animated Wald by accident. Now the German line is taken whenever the
    /// types still hold, so the refusal here has to come from the types.
    #[test]
    fn a_localized_line_is_refused_when_something_changed_the_types() {
        let mut obj = token(11, 0, "Wald", 4, 4);
        obj.types = TypeSet::LAND.union(TypeSet::CREATURE);
        obj.supertypes = SupertypeSet::BASIC;
        obj.subtypes =
            SubtypeSet::from_slice(&[subtypes::land::FOREST, subtypes::creature::ELEMENTAL]);
        let mut t = text("Wald", "Basisland — Wald", "({T}: Erzeuge {G}.)");
        t.lang = "de".to_string();
        t.english_name = "Forest".to_string();
        let printed = PrintedTypes {
            supertypes: SupertypeSet::BASIC,
            types: TypeSet::LAND,
            subtypes: SubtypeSet::from_slice(&[subtypes::land::FOREST]),
        };
        let face = CardFace::from_object(&obj, None, Some(printed), Some(&t));

        assert_eq!(face.name, "Wald", "the name is the card@ own either way");
        assert_eq!(face.type_line, "Basic Land Creature — Forest Elemental");
    }

    /// Subtypes group behind their own type, in the type's printed order —
    /// Dryad Arbor is `Land Creature — Forest Dryad`, never the reverse.
    #[test]
    fn subtypes_follow_the_type_they_belong_to() {
        let mut obj = token(3, 0, "Dryad Arbor", 1, 1);
        obj.types = TypeSet::LAND.union(TypeSet::CREATURE);
        obj.subtypes = SubtypeSet::from_slice(&[subtypes::creature::DRYAD, subtypes::land::FOREST]);
        let face = CardFace::from_object(&obj, None, None, None);
        assert_eq!(face.type_line, "Land Creature — Forest Dryad");
    }

    /// Changeling sets every creature type; printing them would be unreadable
    /// and would wreck any layout.
    #[test]
    fn changeling_collapses_instead_of_printing_three_hundred_types() {
        let mut obj = token(4, 0, "Woodland Changeling", 2, 2);
        obj.subtypes = SubtypeSet::ALL_CREATURE;
        let face = CardFace::from_object(&obj, None, None, None);
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
        let face = CardFace::from_object(&obj, None, Some(as_printed(&obj)), Some(&t));

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
        let face = CardFace::from_object(&obj, None, Some(as_printed(&obj)), Some(&t));

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
        let face = CardFace::from_object(&obj, None, Some(as_printed(&obj)), Some(&t));
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
        let face = CardFace::from_object(&obj, Some(&baylee_core::mana!("{1}{G}")), None, None);

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
        let face = CardFace::from_object(&obj, None, None, None);

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
        let face = CardFace::from_object(&obj, None, Some(as_printed(&obj)), Some(&t));
        assert_eq!(face.name, "Wald");
        // What the test was named for and never asked. It could not have
        // asked it: the guard compared the German line against an English one
        // built from the engine's own type words, so this read
        // `Basic Land — Forest` under a card called Wald — which is the whole
        // of V7, and it was sitting inside a test whose title claims the
        // opposite.
        assert_eq!(face.type_line, "Basisland — Wald");
    }

    /// Jace is the card the whole feature was asked for: four loyalty
    /// abilities, and a stack that says `+1` tells a player nothing.
    #[test]
    fn a_stack_entry_draws_the_sentence_its_ability_is_printed_as() {
        let jace = "+2: Look at the top card of target player's library. You may put that \
                    card on the bottom of that player's library.\n\
                    0: Draw three cards, then put two cards from your hand on top of your \
                    library in any order.\n\
                    −1: Return target creature to its owner's hand.\n\
                    −12: Exile all cards from target player's library, then that player \
                    shuffles their hand into their library.";
        let blocks = sentence_blocks(jace, 1, 4).expect("four sentences, asked for four");
        assert_eq!(
            blocks,
            vec![TextBlock::Rules(
                "0: Draw three cards, then put two cards from your hand on top of your \
                 library in any order."
                    .to_string()
            )]
        );
    }

    /// The guard, and the reason `of` is on the wire at all.
    ///
    /// An index that is merely out of range is caught by anyone. This is
    /// the other case: a text one sentence shorter, where the index lands
    /// *in* range and points at the sentence beside the right one — which
    /// is drawn to the player as precise text and is worse than the label
    /// it replaced. The count is the only thing that can tell them apart.
    #[test]
    fn a_text_of_a_different_length_is_refused_rather_than_indexed() {
        let three = "+1: Do a thing.\n0: Do another.\n−7: Win.";
        assert_eq!(
            sentence_blocks(three, 1, 3),
            Some(vec![TextBlock::Rules("0: Do another.".to_string())])
        );
        assert_eq!(
            sentence_blocks(three, 1, 4),
            None,
            "the host counted four sentences and this text has three"
        );
        assert_eq!(sentence_blocks(three, 3, 3), None, "past the last sentence");
    }

    /// The whole of V1 in one assertion: a translated client showed a German
    /// sentence under an English title, because six places read the
    /// projection and only [`CardFace`] ever reached the catalog.
    #[test]
    fn a_card_is_named_in_the_language_it_was_printed_in() {
        let mut german = text("Gefluteter Strand", "Land", "");
        german.lang = "de".to_string();
        german.english_name = "Flooded Strand".to_string();

        assert_eq!(
            shown_name("Flooded Strand", Some(&german)),
            "Gefluteter Strand"
        );
        // Already translated — a client asking twice must not lose the name.
        assert_eq!(
            shown_name("Gefluteter Strand", Some(&german)),
            "Gefluteter Strand"
        );
        // Nothing arrived yet, and a token that has no printing at all.
        assert_eq!(shown_name("Flooded Strand", None), "Flooded Strand");
    }

    /// The clone guard, which is the reason the lookup cannot simply be
    /// "whatever the printing says": a Clone carries its own cardboard into
    /// every zone, so the printing beside it is the *Clone's* and naming the
    /// object after it would be a lie about what is on the table.
    #[test]
    fn a_copy_keeps_the_name_it_copied() {
        let mut german = text("Klon", "Kreatur — Gestaltwandler", "");
        german.lang = "de".to_string();
        german.english_name = "Clone".to_string();

        assert!(describes(&german, "Clone"));
        assert!(describes(&german, "Klon"));
        assert!(!describes(&german, "Snapcaster Mage"));
        assert_eq!(
            shown_name("Snapcaster Mage", Some(&german)),
            "Snapcaster Mage"
        );
    }

    /// A sentence carries its own reminder text, and it is greyed like any
    /// other — the panel is drawing one paragraph of a card, not a label.
    #[test]
    fn a_sentences_reminder_text_stays_its_own_block() {
        let text = "Flying (This creature can't be blocked except by creatures with flying.)\n\
                    {T}: Add {G}.";
        assert_eq!(
            sentence_blocks(text, 0, 2),
            Some(vec![
                TextBlock::Rules("Flying".to_string()),
                TextBlock::Reminder(
                    "This creature can't be blocked except by creatures with flying.".to_string()
                ),
            ])
        );
    }
}
