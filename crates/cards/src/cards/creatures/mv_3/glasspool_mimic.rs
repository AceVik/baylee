//! Glasspool Mimic // Glasspool Shore — {2}{U} — Creature — Shapeshifter Rogue // Land
//! Oracle: You may have this creature enter as a copy of a creature you control, except it's a Shapeshifter Rogue in addition to its other types.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #60 — Zendikar Rising | Scryfall ID: 5adcb500-8c77-4925-8e2c-1243502827d1 | Oracle ID: c178953c-3888-4edd-9d0c-265bd82b1d24
// IMPLEMENTED — clone-with-extra-subtypes front (CopyOnEnter) + MDFC
// land back playable via the face-choice land play. The copy target is "a
// creature **you control**": the card was printed as "any creature on the
// battlefield" and errata'd, and copying across the table is a different
// card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static SHORE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::GLASSPOOL_MIMIC,
    oracle_id = "c178953c-3888-4edd-9d0c-265bd82b1d24",
    scryfall_id = "5adcb500-8c77-4925-8e2c-1243502827d1",
    faces = &[
        face!(
            name = "Glasspool Mimic",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[creature::SHAPESHIFTER, creature::ROGUE],
            power = Some(0),
            toughness = Some(0),
        ),
        face!(
            name = "Glasspool Shore",
            types = TypeSet::LAND,
            abilities = SHORE_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::YOUR_CREATURE),
        mods: &[
            CopyMod::AddSubtype(creature::SHAPESHIFTER),
            CopyMod::AddSubtype(creature::ROGUE),
        ],
    }],
);
