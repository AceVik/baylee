//! Barrow Ghoul — {1}{B} — Creature — Zombie
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.
//! Set: WTH #61 — Weatherlight | Scryfall ID: f7055007-83dd-40fe-b2a1-4b3132f636db | Oracle ID: 1610875f-48da-490c-a371-c439dc36f533
// NOT SUPPORTED: requiring the exiled creature card to be the top creature card of your graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARROW_GHOUL,
    oracle_id = "1610875f-48da-490c-a371-c439dc36f533",
    scryfall_id = "f7055007-83dd-40fe-b2a1-4b3132f636db",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "at the beginning of your upkeep, sacrifices itself unless you exile a creature card from your graveyard; requiring the exiled card to be the top creature card of your graveyard is not yet modeled",
    ),
    faces = &[face!(
        name = "Barrow Ghoul",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMayPayCostOr {
            player: PlayerRel::You,
            cost: &CostPart::ExileFromGraveyard(&Filter::CREATURE),
            effect: &Effect::SacrificeSelf,
        }],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        ))),
    )],
);
