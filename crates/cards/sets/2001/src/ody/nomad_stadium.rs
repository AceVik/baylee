//! Nomad Stadium — (no cost) — Land
//! Oracle: {T}: Add {W}. This land deals 1 damage to you.
//! Oracle: Threshold — {W}, {T}, Sacrifice this land: You gain 4 life. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #322 — Odyssey | Scryfall ID: 64300b71-050f-47a3-83be-f24480bdc01d | Oracle ID: 4034bec6-e3c7-4d3f-81df-7c903977a606
// IMPLEMENTED — the pain mana line, and 4 life gated on threshold.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOMAD_STADIUM,
    oracle_id = "4034bec6-e3c7-4d3f-81df-7c903977a606",
    scryfall_id = "64300b71-050f-47a3-83be-f24480bdc01d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(name = "Nomad Stadium", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::White, 1),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
        activated!(
            cost!("{W}", TapSelf, SacrificeSelf),
            &[Effect::GainLife {
                amount: Amount::Fixed(4),
            }],
            condition = Some(Condition::GraveyardCountAtLeast(7)),
        ),
    ],
);
