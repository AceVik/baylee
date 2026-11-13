//! Aether Barrier — {2}{U} — Enchantment
//! Oracle: Whenever a player casts a creature spell, that player sacrifices a permanent of their choice unless they pay {1}.
//! Set: NEM #27 — Nemesis | Scryfall ID: 36298f9f-12cc-43bd-adda-ccabd67a9568 | Oracle ID: 0fe602b7-9f88-4d3d-af24-7790df867ed5
// IMPLEMENTED — creature-cast trigger taxing the spell's caster {1} or a permanent sacrifice.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_BARRIER,
    oracle_id = "0fe602b7-9f88-4d3d-af24-7790df867ed5",
    scryfall_id = "36298f9f-12cc-43bd-adda-ccabd67a9568",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aether Barrier",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::CREATURE),
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::EventPlayer,
            mana: Amount::Fixed(1),
            effect: &Effect::SacrificeFilter {
                who: PlayerRel::EventPlayer,
                filter: &Filter::Any,
            },
        }],
    ),],
);
