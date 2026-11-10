//! Earthlink — {3}{B}{R}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {2}.
//! Oracle: Whenever a creature dies, that creature's controller sacrifices a land of their choice.
//! Set: ME2 #192 — Masters Edition II | Scryfall ID: 7d33739b-c47a-431c-b0df-e09f27b38a9e | Oracle ID: c6e4a5c6-3c9b-49d5-8288-1be46be657dc
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARTHLINK,
    oracle_id = "c6e4a5c6-3c9b-49d5-8288-1be46be657dc",
    scryfall_id = "7d33739b-c47a-431c-b0df-e09f27b38a9e",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Earthlink",
        mana_cost = mana!("{3}{B}{R}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: Amount::Fixed(2),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::CREATURE),
            &[Effect::SacrificeFilter {
                who: PlayerRel::ControllerOfEvent,
                filter: &Filter::LAND,
            }],
        ),
    ],
);
