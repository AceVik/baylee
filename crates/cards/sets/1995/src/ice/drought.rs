//! Drought — {2}{W}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.
//! Oracle: Spells cost an additional "Sacrifice a Swamp" to cast for each black mana symbol in their mana costs.
//! Oracle: Activated abilities cost an additional "Sacrifice a Swamp" to activate for each black mana symbol in their activation costs.
//! Set: ICE #21 — Ice Age | Scryfall ID: 97736696-3de3-416d-94cf-4fac792f23f0 | Oracle ID: ecba74bd-850e-4cbe-9b1c-b0d4d6f27d1c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const SWAMP: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::SWAMP)]);

card!(
    index = index::DROUGHT,
    oracle_id = "ecba74bd-850e-4cbe-9b1c-b0d4d6f27d1c",
    scryfall_id = "97736696-3de3-416d-94cf-4fac792f23f0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Drought",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{W}{W}"),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        static_ability!(
            Filter::Any,
            Modifier::SpellsCostSacrificePerManaSymbol {
                color: Color::Black,
                sacrifice: &SWAMP,
            }
        ),
        static_ability!(
            Filter::Any,
            Modifier::AbilitiesCostSacrificePerManaSymbol {
                color: Color::Black,
                sacrifice: &SWAMP,
            }
        ),
    ],
);
