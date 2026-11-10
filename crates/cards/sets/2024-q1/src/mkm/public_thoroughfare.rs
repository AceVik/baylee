//! Public Thoroughfare — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, sacrifice it unless you tap an untapped artifact or land you control.
//! Oracle: {T}: Add one mana of any color.
//! Set: MKM #265 — Murders at Karlov Manor | Scryfall ID: 1f8b915f-3e82-4b05-b963-01ebff7a8f7b | Oracle ID: de5b995c-9691-4555-9070-66bcbc29f955
// IMPLEMENTED — enters tapped, ETB sacrifice unless you tap another artifact or land you control, and taps for any color.

use baylee_cards_dsl::prelude::*;

static ARTIFACT_OR_LAND_YOU_CONTROL: Filter = Filter::And(&[
    Filter::Or(&[Filter::ARTIFACT, Filter::LAND]),
    Filter::ControlledByYou,
]);

card!(
    index = index::PUBLIC_THOROUGHFARE,
    oracle_id = "de5b995c-9691-4555-9070-66bcbc29f955",
    scryfall_id = "1f8b915f-3e82-4b05-b963-01ebff7a8f7b",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Public Thoroughfare",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::TapOther(&ARTIFACT_OR_LAND_YOU_CONTROL),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        mana_ability!(&[Effect::mana_of_any_color()]),
    ],
);
