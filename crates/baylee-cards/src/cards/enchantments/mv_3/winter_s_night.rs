//! Winter's Night — {R}{G}{W} — World Enchantment
//! Oracle: Whenever a player taps a snow land for mana, that player adds one mana of any type that land produced. That land doesn't untap during its controller's next untap step.
//! Set: ME2 #200 — Masters Edition II | Scryfall ID: 432d02e6-51b8-4b36-b551-e9e663ad2246 | Oracle ID: e4f73802-b639-47ab-a6c6-f761afa4e00e
// IMPLEMENTED — Whenever a player taps a snow land for mana, that player adds one mana of any type that land produced, and that land doesn't untap during its controller's next untap step.

use baylee_cards_dsl::prelude::*;

static SNOW_LAND: Filter = Filter::And(&[
    Filter::HasType(TypeSet::LAND),
    Filter::HasSupertype(SupertypeSet::SNOW),
]);

card!(
    index = index::WINTER_S_NIGHT,
    oracle_id = "e4f73802-b639-47ab-a6c6-f761afa4e00e",
    scryfall_id = "432d02e6-51b8-4b36-b551-e9e663ad2246",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Winter's Night",
        mana_cost = mana!("{R}{G}{W}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[triggered!(
        Trigger::TappedForMana {
            by: PlayerRel::EachPlayer,
            filter: &SNOW_LAND,
        },
        &[
            Effect::AddManaLikeEvent { amount: 1 },
            Effect::SuppressUntap {
                target: TargetSpec::EventObject,
                steps: 1,
            },
        ],
    )],
);
