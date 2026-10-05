//! Dream Coat — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {0}: Enchanted creature becomes the color or colors of your choice. Activate only once each turn.
//! Set: LEG #51 — Legends | Scryfall ID: 07edbbf4-c3d6-4ec1-ae9b-4ae202fb6998 | Oracle ID: 988aaf54-2e46-4afc-ae37-b0a01191a1f1
// PARTIAL — enchant creature is written; the color-changing ability is off the
// card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DREAM_COAT,
    oracle_id = "988aaf54-2e46-4afc-ae37-b0a01191a1f1",
    scryfall_id = "07edbbf4-c3d6-4ec1-ae9b-4ae202fb6998",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no Effect asks a player for a color as it resolves \
         (ProtectionFromChosenColor asks for protection only) and \
         Modifier::SetColor takes a fixed ColorSet, so the color-changing \
         ability is off the card"
    ),
    faces = &[face!(
        name = "Dream Coat",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "{0}: Enchanted creature becomes the color or colors of
    // your choice. Activate only once each turn." — the activation itself
    // (`cost!("{0}")` with `limit = ActivationLimit::PerTurn(1)`) and
    // `Modifier::SetColor` on the enchanted creature are sayable, but nothing
    // asks a player for colors during resolution: the only color choice is
    // `EnterModifier::ChooseColor`, made as a permanent enters, and
    // `Effect::ProtectionFromChosenColor` asks for protection alone.
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
