//! Apathy — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature doesn't untap during its controller's untap step.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player may discard a card at random. If the player does, untap that creature.
//! Set: WTH #33 — Weatherlight | Scryfall ID: adf3a6fe-e234-4c3f-96fc-3eb5eb22c0b8 | Oracle ID: 09ac5adf-f722-417e-b0fc-7ed0d8c9abd6
// NOT SUPPORTED: the upkeep trigger allowing enchanted creature's controller to discard a card at random to untap it is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::APATHY,
    oracle_id = "09ac5adf-f722-417e-b0fc-7ed0d8c9abd6",
    scryfall_id = "adf3a6fe-e234-4c3f-96fc-3eb5eb22c0b8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "enchants a creature and prevents it from untapping during its controller's untap step; the upkeep trigger allowing enchanted creature's controller to discard a card at random to untap it is not yet modeled",
    ),
    faces = &[face!(
        name = "Apathy",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(ENCHANTED_CREATURE, Modifier::DoesNotUntap),
    ],
);
