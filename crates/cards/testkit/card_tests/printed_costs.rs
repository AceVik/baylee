//! The costs printed beside a card's mana cost, against the card's own
//! definition, for the cards of the wishlist and Odyssey batch.
//!
//! A play cannot tell these apart. A test that pays from a pool of one colour
//! and of exact size passes {4}{R} and {5} alike, and {2}{G}{G} and {4}, so the
//! generic mana and each colour are pinned here, read from the field the engine
//! pays. Each sentence is held to the card's Oracle text, which is Scryfall's,
//! and the expected mana is read out of that sentence.

use super::*;
use baylee_cards_dsl::{AbilityDef, CardDef, CostPart, Filter};
use baylee_core::generated::{index, subtypes};
use baylee_core::mana::{ManaCost, ManaSymbol};

/// Lava Dart's "a Mountain", spelled as the card spells it.
static A_MOUNTAIN: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
    Filter::ControlledByYou,
]);

/// The field of a [`CardDef`] the engine reads this cost from.
#[derive(Clone, Copy)]
enum Slot {
    /// `FaceDef::flashback` and `flashback_parts` (`casting`, `cast_wizard`).
    Flashback,
    /// `FaceDef::madness` (`cast_wizard::pricing`).
    Madness,
    /// `FaceDef::morph` (`disguise`).
    Morph,
    /// The cost of the first ability, an activated one.
    Activated,
}

impl Slot {
    fn read(self, def: &'static CardDef) -> (ManaCost, &'static [CostPart]) {
        let face = &def.faces[0];
        match self {
            Self::Flashback => (face.flashback.expect("flashback"), face.flashback_parts),
            Self::Madness => {
                let cost = face.madness.expect("madness");
                (cost.mana, cost.parts)
            }
            Self::Morph => (face.morph.expect("morph"), &[]),
            Self::Activated => match &def.abilities[0] {
                AbilityDef::Activated { cost, .. } => (cost.mana, cost.parts),
                other => panic!("an activated ability, not {other:?}"),
            },
        }
    }
}

/// Generic mana, then white, blue, black, red and green.
type Mana = (u32, [u32; 5]);

/// What `sentence` prints: each `{…}` of it. The table's sentences stop before
/// any reminder text, whose symbols are not part of the cost.
fn printed(sentence: &str) -> Mana {
    let mut mana = (0, [0; 5]);
    for symbol in sentence.split(['{', '}']).skip(1).step_by(2) {
        match symbol {
            "W" => mana.1[0] += 1,
            "U" => mana.1[1] += 1,
            "B" => mana.1[2] += 1,
            "R" => mana.1[3] += 1,
            "G" => mana.1[4] += 1,
            number => mana.0 += number.parse::<u32>().expect("a number or a colour"),
        }
    }
    mana
}

/// The same count of what a definition holds.
fn defined(cost: &ManaCost) -> Mana {
    let mut mana = (cost.generic_total(), [0; 5]);
    for (symbol, count) in cost.runs() {
        let count = u32::from(count);
        match symbol {
            ManaSymbol::Generic(_) => {}
            ManaSymbol::White => mana.1[0] += count,
            ManaSymbol::Blue => mana.1[1] += count,
            ManaSymbol::Black => mana.1[2] += count,
            ManaSymbol::Red => mana.1[3] += count,
            ManaSymbol::Green => mana.1[4] += count,
            other => panic!("no cost here prints {other:?}"),
        }
    }
    mana
}

#[test]
fn printed_costs_match_the_fields_the_engine_pays() {
    use Slot::{Activated, Flashback, Madness, Morph};
    const PAY_THREE: &[CostPart] = &[CostPart::PayLife(3)];
    let rows: &[(CardIndex, Slot, &str, &[CostPart])] = &[
        (
            index::LAVA_DART,
            Flashback,
            "Flashback—Sacrifice a Mountain.",
            &[CostPart::Sacrifice(&A_MOUNTAIN)],
        ),
        (
            index::CRIPPLING_FATIGUE,
            Flashback,
            "Flashback—{1}{B}, Pay 3 life.",
            PAY_THREE,
        ),
        (
            index::ACORN_HARVEST,
            Flashback,
            "Flashback—{1}{G}, Pay 3 life.",
            PAY_THREE,
        ),
        (
            index::DEEP_ANALYSIS,
            Flashback,
            "Flashback—{1}{U}, Pay 3 life.",
            PAY_THREE,
        ),
        (index::FIREBOLT, Flashback, "Flashback {4}{R}", &[]),
        (index::RECKLESS_CHARGE, Flashback, "Flashback {2}{R}", &[]),
        (index::VOLCANIC_SPRAY, Flashback, "Flashback {1}{R}", &[]),
        (
            index::CHAINER_S_EDICT,
            Flashback,
            "Flashback {5}{B}{B}",
            &[],
        ),
        (index::MOMENT_S_PEACE, Flashback, "Flashback {2}{G}", &[]),
        (index::ROAR_OF_THE_WURM, Flashback, "Flashback {3}{G}", &[]),
        (
            index::CHATTER_OF_THE_SQUIRREL,
            Flashback,
            "Flashback {1}{G}",
            &[],
        ),
        (index::SYLVAN_MIGHT, Flashback, "Flashback {2}{G}{G}", &[]),
        (index::MORGUE_THEFT, Flashback, "Flashback {4}{B}", &[]),
        (index::ENGULFING_FLAMES, Flashback, "Flashback {3}{R}", &[]),
        (index::BASKING_ROOTWALLA, Madness, "Madness {0}", &[]),
        (index::VIOLENT_ERUPTION, Madness, "Madness {1}{R}{R}", &[]),
        (index::FIERY_TEMPER, Madness, "Madness {R}", &[]),
        (index::CIRCULAR_LOGIC, Madness, "Madness {U}", &[]),
        (index::OBSESSIVE_SEARCH, Madness, "Madness {U}", &[]),
        (index::ARROGANT_WURM, Madness, "Madness {2}{G}", &[]),
        (index::FRANTIC_PURIFICATION, Madness, "Madness {W}", &[]),
        (index::PSYCHOTIC_HAZE, Madness, "Madness {1}{B}", &[]),
        (index::STRENGTH_OF_ISOLATION, Madness, "Madness {W}", &[]),
        (index::STRENGTH_OF_LUNACY, Madness, "Madness {B}", &[]),
        (index::NANTUKO_VIGILANTE, Morph, "Morph {1}{G}", &[]),
        (
            index::WITHERED_WRETCH,
            Activated,
            "{1}: Exile target card",
            &[],
        ),
    ];

    let mut wrong = Vec::new();
    for &(card, slot, sentence, parts) in rows {
        let def = baylee_cards::by_index(card).expect("the card is in the pool");
        let name = def.faces[0].name;
        let oracle = baylee_cards::oracle::face(card, 0).expect("the card has Oracle text");
        if !oracle.contains(sentence) {
            wrong.push(format!(
                "{name}: its Oracle text does not print {sentence:?}"
            ));
        }
        let (mana, defined_parts) = slot.read(def);
        if defined(&mana) != printed(sentence) {
            wrong.push(format!(
                "{name}: printed {:?} as {sentence:?}, defined {:?}",
                printed(sentence),
                defined(&mana)
            ));
        }
        if defined_parts != parts {
            wrong.push(format!(
                "{name}: printed {parts:?}, defined {defined_parts:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
