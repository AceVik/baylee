//! Worms of the Earth — {2}{B}{B}{B} — Enchantment
//! Oracle: Players can't play lands.
//! Oracle: Lands can't enter the battlefield.
//! Oracle: At the beginning of each upkeep, any player may sacrifice two lands of their choice or have this enchantment deal 5 damage to that player. If a player does either, destroy this enchantment.
//! Set: DRK #56 — The Dark | Scryfall ID: 65a97821-ca5b-46fb-af08-86de81d0daac | Oracle ID: 9638ec0c-c8bf-4aa5-a7ca-8677e4b9f241

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORMS_OF_THE_EARTH,
    oracle_id = "9638ec0c-c8bf-4aa5-a7ca-8677e4b9f241",
    scryfall_id = "65a97821-ca5b-46fb-af08-86de81d0daac",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Worms of the Earth",
        mana_cost = mana!("{2}{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::CantPlayLandsAndLandsCantEnter),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::WormsOfTheEarthUpkeep]
        ),
    ],
);
