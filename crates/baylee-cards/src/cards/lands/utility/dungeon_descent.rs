//! Dungeon Descent — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}, Tap an untapped legendary creature you control: Venture into the dungeon. Activate only as a sorcery. (Enter the first room or advance to the next room.)
//! Set: AFR #255 — Adventures in the Forgotten Realms | Scryfall ID: f4cccdbc-f4f4-42b6-9747-6ef703ff949a | Oracle ID: f086a63c-0c62-4674-bd27-82e7aed12b1a
// IMPLEMENTED — enters tapped, {T}: Add {C}, and {4}, {T}, Tap an untapped
// legendary creature you control: Venture into the dungeon (as a sorcery).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DUNGEON_DESCENT,
    oracle_id = "f086a63c-0c62-4674-bd27-82e7aed12b1a",
    scryfall_id = "f4cccdbc-f4f4-42b6-9747-6ef703ff949a",
    faces = &[face!(
        name = "Dungeon Descent",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(
                "{4}",
                TapSelf,
                TapOther(&Filter::And(&[
                    Filter::LEGENDARY_CREATURE,
                    Filter::ControlledByYou,
                ]))
            ),
            &[Effect::VentureIntoTheDungeon],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
