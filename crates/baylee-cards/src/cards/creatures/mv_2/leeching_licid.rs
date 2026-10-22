//! Leeching Licid — {1}{B} — Creature — Licid
//! Oracle: {B}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {B} to end this effect.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, this creature deals 1 damage to that player.
//! Set: TMP #141 — Tempest | Scryfall ID: 27bffefb-23c0-4d03-b716-b1a7eff39a05 | Oracle ID: bcb5af89-a304-4de0-86af-b3f4cfabafc4
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LEECHING_LICID,
    oracle_id = "bcb5af89-a304-4de0-86af-b3f4cfabafc4",
    scryfall_id = "27bffefb-23c0-4d03-b716-b1a7eff39a05",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Pays {B}, {T} targeting a creature to deal 1 damage to that creature's controller; turning a creature permanent into an attached Aura with a '{B}' special action to end the effect and an upkeep trigger on the enchanted creature's controller is not yet modeled."
    ),
    faces = &[face!(
        name = "Leeching Licid",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}", TapSelf),
        &[Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::ControllerOfTarget),
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
