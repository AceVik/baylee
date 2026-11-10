//! Shivan Gorge — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}{R}, {T}: Shivan Gorge deals 1 damage to each opponent.
//! Set: DSC #297 — Duskmourn: House of Horror Commander | Scryfall ID: c1752b65-dace-4c87-b102-23df276b2e41 | Oracle ID: e90a1381-c9c1-4f57-928c-5d19dc065274
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHIVAN_GORGE,
    oracle_id = "e90a1381-c9c1-4f57-928c-5d19dc065274",
    scryfall_id = "c1752b65-dace-4c87-b102-23df276b2e41",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shivan Gorge",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}{R}", TapSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::EachOpponent),
            }]
        ),
    ],
);
