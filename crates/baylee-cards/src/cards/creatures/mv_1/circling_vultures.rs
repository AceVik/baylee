//! Circling Vultures — {B} — Creature — Bird
//! Oracle: Flying
//! Oracle: You may discard this card any time you could cast an instant.
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.
//! Set: WTH #64 — Weatherlight | Scryfall ID: 8dae8e49-c2b6-4965-9249-49f93449d271 | Oracle ID: 7fd6fe21-8fb8-499b-9dd4-806d0d660265
// NOT SUPPORTED: requiring the exiled creature card to be the top creature card of your graveyard and the CR 116.2e special action to discard this card from hand any time you could cast an instant are not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CIRCLING_VULTURES,
    oracle_id = "7fd6fe21-8fb8-499b-9dd4-806d0d660265",
    scryfall_id = "8dae8e49-c2b6-4965-9249-49f93449d271",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "has flying and at the beginning of your upkeep sacrifices itself unless you exile a creature card from your graveyard; requiring the exiled card to be the top creature card of your graveyard and the CR 116.2e special action to discard this card from hand any time you could cast an instant are not yet modeled",
    ),
    faces = &[face!(
        name = "Circling Vultures",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(3),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
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
