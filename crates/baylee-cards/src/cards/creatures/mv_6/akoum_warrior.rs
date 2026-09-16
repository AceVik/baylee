//! Akoum Warrior // Akoum Teeth — {5}{R} — Creature — Minotaur Warrior // Land
//! Oracle: Trample
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #134 — Zendikar Rising | Scryfall ID: d8ed0335-daa6-4dbe-a94d-4d56c8cfd093 | Oracle ID: afedce7b-0e18-40ad-a26a-1933fddb560d
//! Face: Akoum Warrior — {5}{R} — Creature — Minotaur Warrior
//! Face: Akoum Teeth —  — Land
// IMPLEMENTED — a trampling creature on the front; the back is an MDFC land
// reached by the face choice on a land play (CR 712.4a), which comes down
// tapped and taps for {R}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TEETH_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::AKOUM_WARRIOR,
    oracle_id = "afedce7b-0e18-40ad-a26a-1933fddb560d",
    scryfall_id = "d8ed0335-daa6-4dbe-a94d-4d56c8cfd093",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Akoum Warrior",
            mana_cost = mana!("{5}{R}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::MINOTAUR, subtypes::creature::WARRIOR],
            power = Some(4),
            toughness = Some(5),
        ),
        face!(
            name = "Akoum Teeth",
            types = TypeSet::LAND,
            abilities = TEETH_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
);

// Engine-level test belongs in baylee-engine (mdfc_tests): the front face is
// the only one that is not a land, so a land play resolves straight to face 1
// with no face choice, which enters tapped and taps for {R} once it untaps.
