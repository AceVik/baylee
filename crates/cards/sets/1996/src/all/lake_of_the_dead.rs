//! Lake of the Dead — (no cost) — Land
//! Oracle: If this land would enter, sacrifice a Swamp instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {B}.
//! Oracle: {T}, Sacrifice a Swamp: Add {B}{B}{B}{B}.
//! Set: VMA #302 — Vintage Masters | Scryfall ID: 1b0502c5-43d0-4c36-b585-e5507134bf9e | Oracle ID: bdf476e5-1d57-4b17-b45b-d52fd75aadeb

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SWAMP_YOU_CONTROL: Filter = f!(your Filter::HasSubtype(land::SWAMP));

card!(
    index = index::LAKE_OF_THE_DEAD,
    oracle_id = "bdf476e5-1d57-4b17-b45b-d52fd75aadeb",
    scryfall_id = "1b0502c5-43d0-4c36-b585-e5507134bf9e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Lake of the Dead",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::SacrificeOrGraveyard {
            filter: &Filter::And(&[
                Filter::ControlledByYou,
                Filter::HasSubtype(baylee_core::generated::subtypes::land::SWAMP)
            ]),
            count: 1
        }],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        mana_ability!(
            cost!(TapSelf, Sacrifice(&SWAMP_YOU_CONTROL)),
            &[Effect::mana(ManaColor::Black, 4)]
        ),
    ],
);
