//! Sylvan Hierophant — {1}{G} — Creature — Human Cleric
//! Oracle: When this creature dies, exile it, then return another target creature card from your graveyard to your hand.
//! Set: WTH #141 — Weatherlight | Scryfall ID: 432a6908-0ee3-45c5-9089-b7f8cf1184bb | Oracle ID: e98361d9-1e3a-464e-bac6-75314fc49f32
// IMPLEMENTED — When it dies, exiles itself and returns another target creature card from your graveyard to your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::This)]);

card!(
    index = index::SYLVAN_HIEROPHANT,
    oracle_id = "e98361d9-1e3a-464e-bac6-75314fc49f32",
    scryfall_id = "432a6908-0ee3-45c5-9089-b7f8cf1184bb",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sylvan Hierophant",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[
            Effect::ExileSource,
            Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&OTHER_CREATURE, PlayerRel::You),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &OTHER_CREATURE,
            PlayerRel::You,
        ))),
    ),],
);
