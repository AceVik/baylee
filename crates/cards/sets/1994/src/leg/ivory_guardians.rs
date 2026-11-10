//! Ivory Guardians — {4}{W}{W} — Creature — Giant Cleric
//! Oracle: Protection from red
//! Oracle: Creatures named Ivory Guardians get +1/+1 as long as an opponent controls a nontoken red permanent.
//! Set: ME3 #15 — Masters Edition III | Scryfall ID: e8142471-0ded-410b-bdd5-ac241735b0ce | Oracle ID: 78171447-1bc8-4588-b427-5b82701324c5
// IMPLEMENTED — protection from red as a layer-6 static, and the +1/+1 anthem for creatures named Ivory Guardians behind the opponent-controls condition.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "A nontoken red permanent an opponent controls." The condition walks one
/// opponent at a time, but the card's sentence names whose permanents
/// (`xtask validate` holds a card printing "an opponent controls" to a
/// filter that says so).
static OPPONENT_NONTOKEN_RED: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::Not(&Filter::IsToken),
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);

/// "Creatures named Ivory Guardians."
static IVORY_GUARDIANS: Filter = Filter::And(&[Filter::CREATURE, Filter::Named("Ivory Guardians")]);

card!(
    index = index::IVORY_GUARDIANS,
    oracle_id = "78171447-1bc8-4588-b427-5b82701324c5",
    scryfall_id = "e8142471-0ded-410b-bdd5-ac241735b0ce",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ivory Guardians",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT, subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // CR 702.16a: protection is a static ability. There is no keyword
        // bit for anything but black, so the red half is the modifier it is.
        static_ability!(
            Filter::This,
            Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Red])))
        ),
        static_ability!(
            IVORY_GUARDIANS,
            Modifier::ModifyPT(1, 1),
            condition = Some(Condition::OpponentControlCount(&OPPONENT_NONTOKEN_RED, 1))
        ),
    ],
);
