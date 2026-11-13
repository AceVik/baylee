//! Beasts of Bogardan — {4}{R} — Creature — Beast
//! Oracle: Protection from red
//! Oracle: This creature gets +1/+1 as long as an opponent controls a nontoken white permanent.
//! Set: CHR #45 — Chronicles | Scryfall ID: cec0fe2c-e7e6-42d1-8128-58d70a7f1177 | Oracle ID: 9b0bc2d3-d64b-455a-a258-d3c91892f1a7
// IMPLEMENTED — protection from red, and +1/+1 while an opponent controls a
// nontoken white permanent.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Scoped to the other side of the table even though
/// `Condition::OpponentControlCount` already walks one opponent at a time:
/// the filter is what the card's own sentence says, and `xtask validate`
/// holds a card printing "an opponent controls" to a filter that says so.
static OPPONENT_NONTOKEN_WHITE: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::ControlledByOpponent,
    Filter::Not(&Filter::IsToken),
]);

card!(
    index = index::BEASTS_OF_BOGARDAN,
    oracle_id = "9b0bc2d3-d64b-455a-a258-d3c91892f1a7",
    scryfall_id = "cec0fe2c-e7e6-42d1-8128-58d70a7f1177",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Beasts of Bogardan",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Red])))
        ),
        // "as long as" is the static's own condition: the effect is
        // registered while an opponent holds a match and removed when the
        // last one leaves (CR 611.2b), never a filter that reads the source.
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(1, 1),
            condition = Some(Condition::OpponentControlCount(&OPPONENT_NONTOKEN_WHITE, 1))
        ),
    ],
);
