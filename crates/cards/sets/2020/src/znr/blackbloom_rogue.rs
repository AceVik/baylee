//! Blackbloom Rogue // Blackbloom Bog — {2}{B} — Creature — Human Rogue // Land
//! Oracle: Menace (This creature can't be blocked except by two or more creatures.)
//! Oracle: This creature gets +3/+0 as long as an opponent has eight or more cards in their graveyard.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #91 — Zendikar Rising | Scryfall ID: 32779721-b021-4bd4-95d1-4a19b78d9faa | Oracle ID: 34320ebf-da97-44a4-bbeb-a9da06548289
//! Face: Blackbloom Rogue — {2}{B} — Creature — Human Rogue
//! Face: Blackbloom Bog —  — Land
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The back face's whole printed text apart from the tapland clause, which
/// is an `EnterModifier` rather than an ability.
static BOG_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::BLACKBLOOM_ROGUE,
    oracle_id = "34320ebf-da97-44a4-bbeb-a9da06548289",
    scryfall_id = "32779721-b021-4bd4-95d1-4a19b78d9faa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Blackbloom Rogue",
            mana_cost = mana!("{2}{B}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
            power = Some(2),
            toughness = Some(3),
        ),
        face!(
            name = "Blackbloom Bog",
            types = TypeSet::LAND,
            abilities = BOG_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    keywords = KeywordSet::MENACE,
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ModifyPT(3, 0),
        condition = Some(Condition::OpponentGraveyardCountAtLeast(8))
    )],
);
