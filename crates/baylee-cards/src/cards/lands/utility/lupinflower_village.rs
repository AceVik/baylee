//! Lupinflower Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {W}. Spend this mana only to cast a creature spell.
//! Oracle: {1}{W}, {T}, Sacrifice this land: Look at the top six cards of your library. You may reveal a Bat, Bird, Mouse, or Rabbit card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.
//! Set: BLB #256 — Bloomburrow | Scryfall ID: 8ab9d56f-9178-4ec9-a5f6-b934f50d8d9d | Oracle ID: b6c7c708-5212-4100-b954-b77855b27915

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LUPINFLOWER_VILLAGE,
    oracle_id = "b6c7c708-5212-4100-b954-b77855b27915",
    scryfall_id = "8ab9d56f-9178-4ec9-a5f6-b934f50d8d9d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(name = "Lupinflower Village", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana(ManaColor::White, 1).restricted(&Filter::CREATURE, SpendRider::None)
        ]),
        activated!(
            cost!("{1}{W}", TapSelf, SacrificeSelf),
            &[Effect::LookAtTopFind {
                count: 6,
                filter: &Filter::Or(&[
                    Filter::HasSubtype(baylee_core::generated::subtypes::creature::BAT),
                    Filter::HasSubtype(baylee_core::generated::subtypes::creature::BIRD),
                    Filter::HasSubtype(baylee_core::generated::subtypes::creature::MOUSE),
                    Filter::HasSubtype(baylee_core::generated::subtypes::creature::RABBIT),
                ]),
                random: true
            }]
        ),
    ],
);
