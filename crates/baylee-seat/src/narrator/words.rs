//! The English words the narrator uses for the engine's names: steps,
//! keywords, counters, colours, types and the prompts of choices.
//!
//! Written here rather than borrowed from client-core's phrase tables
//! because those are a person's labels ("Main 1", a keyword icon), and a
//! model reads the Comprehensive Rules' words best: "precombat main phase"
//! (CR 505.1), "declare blockers step". Stage 1b of the design moves the
//! option labels into one describer for both; until then this is the
//! second path, named as such.

use baylee_cards_dsl::KeywordSet;
use baylee_core::mana::ManaColor;
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_engine::choice::{ChoicePrompt, Measure};
use baylee_view::{CounterKind, Phase, Step};

/// A step or phase by its name in the rules (CR 500.1, 505.1, 506.1).
#[must_use]
pub const fn step_name(phase: Phase, step: Step) -> &'static str {
    match (phase, step) {
        (_, Step::Untap) => "untap step",
        (_, Step::Upkeep) => "upkeep",
        (_, Step::Draw) => "draw step",
        (Phase::SecondMain, _) => "postcombat main phase",
        (_, Step::Main) => "precombat main phase",
        (_, Step::CombatBegin) => "beginning of combat step",
        (_, Step::DeclareAttackers) => "declare attackers step",
        (_, Step::DeclareBlockers) => "declare blockers step",
        (_, Step::CombatDamageFirst) => "first-strike combat damage step",
        (_, Step::CombatDamage) => "combat damage step",
        (_, Step::CombatEnd) => "end of combat step",
        (_, Step::End) => "end step",
        (_, Step::Cleanup) => "cleanup step",
    }
}

/// Every keyword the engine keeps as a bit, by its English name.
///
/// A positive list, so `every_keyword_a_card_prints_has_a_name` holds it
/// to the pool: a keyword a card carries and this table lacks fails there
/// rather than going silent on a board.
pub const KEYWORDS: [(KeywordSet, &str); 39] = [
    (KeywordSet::FLYING, "flying"),
    (KeywordSet::FIRST_STRIKE, "first strike"),
    (KeywordSet::DOUBLE_STRIKE, "double strike"),
    (KeywordSet::DEATHTOUCH, "deathtouch"),
    (KeywordSet::HASTE, "haste"),
    (KeywordSet::HEXPROOF, "hexproof"),
    (KeywordSet::INDESTRUCTIBLE, "indestructible"),
    (KeywordSet::LIFELINK, "lifelink"),
    (KeywordSet::MENACE, "menace"),
    (KeywordSet::REACH, "reach"),
    (KeywordSet::TRAMPLE, "trample"),
    (KeywordSet::VIGILANCE, "vigilance"),
    (KeywordSet::DEFENDER, "defender"),
    (KeywordSet::FLASH, "flash"),
    (KeywordSet::SHROUD, "shroud"),
    (KeywordSet::FEAR, "fear"),
    (KeywordSet::INTIMIDATE, "intimidate"),
    (KeywordSet::SHADOW, "shadow"),
    (KeywordSet::HORSEMANSHIP, "horsemanship"),
    (KeywordSet::INFECT, "infect"),
    (KeywordSet::WITHER, "wither"),
    (KeywordSet::PERSIST, "persist"),
    (KeywordSet::UNDYING, "undying"),
    (KeywordSet::PROWESS, "prowess"),
    (KeywordSet::SKULK, "skulk"),
    (KeywordSet::FLANKING, "flanking"),
    (KeywordSet::CHANGELING, "changeling"),
    (KeywordSet::PARTNER, "partner"),
    (KeywordSet::UNBLOCKABLE, "can't be blocked"),
    (KeywordSet::UNCOUNTERABLE, "can't be countered"),
    (KeywordSet::REBOUND, "rebound"),
    (KeywordSet::PROTECTION_BLACK, "protection from black"),
    (KeywordSet::DAYBOUND, "daybound"),
    (KeywordSet::NIGHTBOUND, "nightbound"),
    (KeywordSet::CANT_BLOCK, "can't block"),
    (KeywordSet::STORIED, "storied"),
    (KeywordSet::SPLIT_SECOND, "split second"),
    (KeywordSet::ASCEND, "ascend"),
    (KeywordSet::CANT_ATTACK, "can't attack"),
];

/// The keywords in `bits` (a view's projected keywords), in table order.
/// A bit the table does not name is said as "another keyword", never
/// dropped.
#[must_use]
pub fn keywords(bits: u128) -> Vec<&'static str> {
    let mut named = 0_u128;
    let mut words: Vec<&'static str> = KEYWORDS
        .iter()
        .filter(|(k, _)| bits & k.bits() != 0)
        .map(|(k, word)| {
            named |= k.bits();
            *word
        })
        .collect();
    if bits & !named != 0 {
        words.push("another keyword");
    }
    words
}

/// A counter by its English name, singular ("+1/+1", "lore").
#[must_use]
pub fn counter(kind: CounterKind) -> String {
    match kind {
        CounterKind::Plus { .. } | CounterKind::Minus { .. } => kind.badge().into_owned(),
        CounterKind::Loyalty => "loyalty".into(),
        CounterKind::Lore => "lore".into(),
        CounterKind::Time => "time".into(),
        CounterKind::Charge => "charge".into(),
        CounterKind::Poison => "poison".into(),
        CounterKind::Energy => "energy".into(),
        CounterKind::Rad => "rad".into(),
        CounterKind::Lifelink => "lifelink".into(),
        CounterKind::Level => "level".into(),
        CounterKind::Custom(_) => "other".into(),
    }
}

/// A mana colour's letter, as a model answers with it.
#[must_use]
pub const fn color_letter(color: ManaColor) -> &'static str {
    match color {
        ManaColor::White => "W",
        ManaColor::Blue => "U",
        ManaColor::Black => "B",
        ManaColor::Red => "R",
        ManaColor::Green => "G",
        ManaColor::Colorless => "C",
    }
}

/// A mana colour's name.
#[must_use]
pub const fn color_name(color: ManaColor) -> &'static str {
    match color {
        ManaColor::White => "white",
        ManaColor::Blue => "blue",
        ManaColor::Black => "black",
        ManaColor::Red => "red",
        ManaColor::Green => "green",
        ManaColor::Colorless => "colorless",
    }
}

/// The card types of an object in lower case, with the legendary
/// supertype: "legendary artifact creature", "instant".
#[must_use]
pub fn types(types: TypeSet, supertypes: SupertypeSet) -> String {
    supertypes
        .words()
        .chain(types.words())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The subtypes of an object, capitalised as printed ("Goblin Warrior").
#[must_use]
pub fn subtypes(subtypes: baylee_core::types::SubtypeSet) -> String {
    subtypes
        .iter()
        .filter_map(baylee_core::generated::subtypes::name)
        .collect::<Vec<_>>()
        .join(" ")
}

/// What a [`ChoicePrompt`] asks, as an instruction.
#[must_use]
pub fn choice_prompt(prompt: ChoicePrompt) -> String {
    match prompt {
        ChoicePrompt::SearchLibrary => "Search your library: choose the cards to find".into(),
        ChoicePrompt::PutBackOnTop => "Choose the cards to put back on top of your library".into(),
        ChoicePrompt::Wish => "Choose a card from outside the game".into(),
        ChoicePrompt::Delve => "Choose cards to exile from your graveyard for delve".into(),
        ChoicePrompt::CostSacrifice => "Choose what to sacrifice to pay the cost".into(),
        ChoicePrompt::CostDiscard => "Choose the cards to discard to pay the cost".into(),
        ChoicePrompt::CostTap => "Choose what to tap to pay the cost".into(),
        ChoicePrompt::CostReturn => {
            "Choose what to return to its owner's hand to pay the cost".into()
        }
        ChoicePrompt::CostExile => "Choose the cards to exile to pay the cost".into(),
        ChoicePrompt::LeaveTapped => "Choose the permanents that stay tapped".into(),
        ChoicePrompt::OneOfType { card_type } => {
            format!("Choose one {}", types(card_type, SupertypeSet::EMPTY))
        }
        ChoicePrompt::RevealOrEnterTapped => {
            "Choose a card to reveal (choose none and it enters tapped)".into()
        }
        ChoicePrompt::PutIntoHand => "Choose the cards to put into your hand".into(),
        ChoicePrompt::PutOnBottom => "Choose the cards to put on the bottom of your library".into(),
        ChoicePrompt::PlayFromExile => "Choose a card to play from exile".into(),
        ChoicePrompt::PutOntoBattlefield => "Choose the cards to put onto the battlefield".into(),
        ChoicePrompt::FromGraveyard => "Choose cards from the graveyard".into(),
        ChoicePrompt::Discard => "Choose the cards to discard".into(),
        ChoicePrompt::PutIntoGraveyard => "Choose the cards to put into the graveyard".into(),
        ChoicePrompt::FirstPile => {
            "Separate the cards into two piles: choose the cards of the first pile (the rest \
             form the second)"
                .into()
        }
        ChoicePrompt::CostCrew { power } => {
            format!("Choose creatures to tap to crew (total power {power} or more)")
        }
        ChoicePrompt::Generic => "Choose cards".into(),
    }
}

/// What a card total counts.
#[must_use]
pub const fn measure(of: Measure) -> &'static str {
    match of {
        Measure::Power => "power",
        Measure::Toughness => "toughness",
        Measure::ManaValue => "mana value",
    }
}

/// `n` and the noun, pluralised with an `s` when `n` is not one.
#[must_use]
pub fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pool's keywords all have a name here: a card that prints one the
    /// table lacks would reach a model as "another keyword".
    #[test]
    fn every_keyword_a_card_prints_has_a_name() {
        let named = KEYWORDS.iter().fold(0_u128, |all, (k, _)| all | k.bits());
        let mut faces = 0;
        for card in baylee_cards::pool::rows() {
            let index = baylee_core::ids::CardIndex::new(card.index);
            let Some(def) = baylee_cards::by_index(index) else {
                continue;
            };
            for face in 0..def.faces.len() {
                faces += 1;
                let bits = def.keywords_for_face(face).bits();
                assert_eq!(
                    bits & !named,
                    0,
                    "{} prints a keyword the narrator cannot name",
                    def.name()
                );
            }
        }
        assert!(faces > 1000, "the pool walk read {faces} faces");
    }

    #[test]
    fn keyword_bits_are_distinct_and_unnamed_bits_are_said() {
        let mut seen = 0_u128;
        for (k, word) in KEYWORDS {
            assert_eq!(seen & k.bits(), 0, "{word} shares a bit");
            seen |= k.bits();
        }
        assert_eq!(
            keywords(KeywordSet::FLYING.bits() | KeywordSet::VIGILANCE.bits()),
            ["flying", "vigilance"]
        );
        assert_eq!(keywords(1 << 120), ["another keyword"]);
    }
}
