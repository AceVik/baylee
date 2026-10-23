//! Greener Pastures — {2}{G} — Enchantment
//! Oracle: At the beginning of each player's upkeep, if that player controls more lands than each other player, the player creates a 1/1 green Saproling creature token.
//! Set: USG #258 — Urza's Saga | Scryfall ID: 55c3222e-ac18-4f57-9c9e-50deb0a69db3 | Oracle ID: f9e1346a-af72-4644-b384-8f7b0a35ec31
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREENER_PASTURES,
    oracle_id = "f9e1346a-af72-4644-b384-8f7b0a35ec31",
    scryfall_id = "55c3222e-ac18-4f57-9c9e-50deb0a69db3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "checking whether the active player controls more lands than each other player and creating the token for that player is not in Condition/Effect; triggers at the beginning of your upkeep to create a 1/1 green Saproling creature token",
    ),
    faces = &[face!(
        name = "Greener Pastures",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::CreateToken {
            token: &crate::generated_tokens::SAPROLING_1_1_GREEN,
        }],
    )],
);
