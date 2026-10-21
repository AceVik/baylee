//! Illusionary Terrain — {U}{U} — Enchantment
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: As this enchantment enters, choose two basic land types.
//! Oracle: Basic lands of the first chosen type are the second chosen type.
//! Set: ICE #77 — Ice Age | Scryfall ID: 691f4a1b-4706-41aa-82da-ae920739f036 | Oracle ID: 17ae63fa-fd63-45e6-81c1-bf30851bf77c
// NOT SUPPORTED: choosing two basic land types on entry and turning basic lands of the first chosen type into the second is not supported; only cumulative upkeep {2} is played.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILLUSIONARY_TERRAIN,
    oracle_id = "17ae63fa-fd63-45e6-81c1-bf30851bf77c",
    scryfall_id = "691f4a1b-4706-41aa-82da-ae920739f036",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "choosing two basic land types on entry and turning basic lands of the first chosen type into the second is not supported; only cumulative upkeep {2} is played",
    ),
    faces = &[face!(
        name = "Illusionary Terrain",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::AddCounter {
                kind: counters::AGE,
                amount: Amount::Fixed(1),
            },
            Effect::PlayerMayPayManaEachThenElse {
                player: PlayerRel::You,
                each: mana!("{2}"),
                times: Amount::CountersOnSource(counters::AGE),
                paid: &[],
                unpaid: &[Effect::SacrificeSelf],
            },
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield,
        ))),
    )],
);
