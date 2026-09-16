//! Tangled Florahedron // Tangled Vale — {1}{G} — Creature — Elemental // Land
//! Oracle: {T}: Add {G}.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #211 — Zendikar Rising | Scryfall ID: 235d1ffc-72aa-40a2-95dc-3f6a8d495061 | Oracle ID: 53542c79-a62a-4d6a-97db-5296e9c68302
//! Face: Tangled Florahedron — {1}{G} — Creature — Elemental
//! Face: Tangled Vale —  — Land
// IMPLEMENTED — both faces tap for {G}: a 1/1 mana creature on the front,
// and on the back an MDFC land reached by the face choice on a land play
// (CR 712.4a), which comes down tapped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static VALE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card!(
    index = index::TANGLED_FLORAHEDRON,
    oracle_id = "53542c79-a62a-4d6a-97db-5296e9c68302",
    scryfall_id = "235d1ffc-72aa-40a2-95dc-3f6a8d495061",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Tangled Florahedron",
            mana_cost = mana!("{1}{G}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::ELEMENTAL],
            power = Some(1),
            toughness = Some(1),
        ),
        face!(
            name = "Tangled Vale",
            types = TypeSet::LAND,
            abilities = VALE_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);

// Engine-level test belongs in baylee-engine (mdfc_tests): the front face is
// a creature whose {T} is summoning-sick the turn it enters (CR 302.6), while
// the back face is offered as a land play, enters tapped, and taps for {G}
// once it untaps.
