//! Which kinds of mana a card's printed mana abilities name (WG-4): the
//! deck builder's "colour sources" (round 2) counts a deck's lands and rocks
//! by it, off `GET /pool`'s `produces`.
//!
//! A label's question, not a planner's: what the words say the card can add,
//! whatever the ability costs or does beside it, so a filter land and a
//! painland count for their colours. It is read with the same doors the
//! client's mana sheet reads ([`baylee_cards_dsl::mana_bundle`] for a tap
//! that adds several, [`baylee_cards_dsl::mana_written`] for one that adds
//! one), and the intrinsic ability of a land with basic land types
//! (CR 305.6) by those types. What only a game can answer is left out
//! rather than guessed: a Command Tower's colours are its controller's
//! commanders' (CR 903.4), an Exotic Orchard's are somebody else's lands', a
//! chosen colour is chosen as the permanent enters.

use baylee_cards_dsl::{AbilityDef, CardDef, ManaSource, mana_bundle, mana_written};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::SubtypeId;
use baylee_core::mana::ManaColor;

/// The kinds of mana this card's front face's mana abilities name, as
/// `WUBRGC` letters in that order; empty for a card that names none.
#[must_use]
pub fn produces(def: &CardDef) -> String {
    let mut made = [false; 6];
    let mut add = |color: ManaColor| made[slot(color)] = true;
    let face = def.faces.first();
    for ability in def.abilities_for_face(0) {
        if !ability.is_mana_ability() {
            continue;
        }
        let (AbilityDef::Activated { cost, effects, .. }
        | AbilityDef::ActivatedConditional { cost, effects, .. }) = ability
        else {
            continue;
        };
        if ability.is_intrinsic_mana_ability() {
            for (subtype, color) in BASIC_TYPES {
                if face.is_some_and(|f| f.subtypes.contains(&subtype)) {
                    add(color);
                }
            }
        } else if let Some(colors) = mana_bundle(cost, effects) {
            colors.into_iter().for_each(&mut add);
        } else if let Some((source, _, _)) = mana_written(effects) {
            match source {
                ManaSource::Fixed(color) => add(color),
                ManaSource::Choice(colors) | ManaSource::ChosenOr(colors) => {
                    colors.iter().copied().for_each(&mut add);
                }
                ManaSource::IntrinsicBasicLandTypes
                | ManaSource::CommanderIdentity
                | ManaSource::LandColor { .. }
                | ManaSource::Chosen => {}
            }
        }
    }
    ORDER
        .iter()
        .filter(|(color, _)| made[slot(*color)])
        .map(|(_, letter)| *letter)
        .collect()
}

/// The basic land types and the colour each one's intrinsic ability makes
/// (CR 305.6).
const BASIC_TYPES: [(SubtypeId, ManaColor); 5] = [
    (land::PLAINS, ManaColor::White),
    (land::ISLAND, ManaColor::Blue),
    (land::SWAMP, ManaColor::Black),
    (land::MOUNTAIN, ManaColor::Red),
    (land::FOREST, ManaColor::Green),
];

/// The wire's order and letters.
const ORDER: [(ManaColor, char); 6] = [
    (ManaColor::White, 'W'),
    (ManaColor::Blue, 'U'),
    (ManaColor::Black, 'B'),
    (ManaColor::Red, 'R'),
    (ManaColor::Green, 'G'),
    (ManaColor::Colorless, 'C'),
];

/// Where a kind of mana sits in [`ORDER`].
fn slot(color: ManaColor) -> usize {
    ORDER
        .iter()
        .position(|(c, _)| *c == color)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::rows;

    fn named(name: &str) -> &'static CardDef {
        crate::decks::by_name(name)
            .and_then(crate::by_index)
            .unwrap_or_else(|| panic!("the pool has {name}"))
    }

    #[test]
    fn a_basic_land_produces_its_colour_and_a_spell_nothing() {
        assert_eq!(produces(named("Forest")), "G");
        assert_eq!(produces(named("Island")), "U");
        assert_eq!(produces(named("Sol Ring")), "C", "an artifact taps too");
        let spell = rows()
            .iter()
            .find(|c| c.kinds == ["Instant"])
            .expect("an instant");
        assert_eq!(produces(named(&spell.english_name)), "");
    }

    /// A dual land — two basic land types, one intrinsic ability that asks —
    /// produces both, in `WUBRG` order whatever order its types print in.
    #[test]
    fn a_dual_land_produces_two() {
        let dual = rows()
            .iter()
            .find(|c| {
                c.kinds == ["Land"]
                    && c.type_line.contains("Island")
                    && c.type_line.contains("Plains")
            })
            .expect("a Plains Island");
        assert_eq!(produces(named(&dual.english_name)), "WU", "{}", dual.name);
    }

    /// Nearly every land is read as making something: a reader that silently
    /// gave up would leave a deck's sources at nothing. Measured 07.10.2026:
    /// 1033 of 1124 lands (92 %; the rest are stubs and lands whose colours
    /// only a game knows); the floor is 85 %. And no card claims more than
    /// the six kinds there are.
    #[test]
    fn most_lands_produce_something_and_no_card_more_than_six() {
        let lands: Vec<_> = rows()
            .iter()
            .filter(|c| c.kinds.contains(&"Land"))
            .collect();
        let producing = lands
            .iter()
            .filter(|c| !produces(named(&c.english_name)).is_empty())
            .count();
        assert!(lands.len() > 100, "{} lands", lands.len());
        assert!(
            producing * 100 >= lands.len() * 85,
            "{producing} of {} lands produce",
            lands.len()
        );
        for card in rows() {
            let letters = produces(named(&card.english_name));
            assert!(letters.len() <= 6, "{}: {letters}", card.name);
            assert!(
                letters.chars().all(|c| "WUBRGC".contains(c)),
                "{}",
                card.name
            );
        }
    }
}
