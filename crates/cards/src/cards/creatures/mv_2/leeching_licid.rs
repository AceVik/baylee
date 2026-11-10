//! Leeching Licid — {1}{B} — Creature — Licid
//! Oracle: {B}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {B} to end this effect.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, this creature deals 1 damage to that player.
//! Set: TMP #141 — Tempest | Scryfall ID: 27bffefb-23c0-4d03-b716-b1a7eff39a05 | Oracle ID: bcb5af89-a304-4de0-86af-b3f4cfabafc4
// PARTIAL — Only the 1/1 creature body is supported; the Aura transformation, termination payment and attached-controller upkeep damage are unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LEECHING_LICID,
    oracle_id = "bcb5af89-a304-4de0-86af-b3f4cfabafc4",
    scryfall_id = "27bffefb-23c0-4d03-b716-b1a7eff39a05",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Only the 1/1 creature body is supported; the Aura transformation, termination payment and attached-controller upkeep damage are unsupported."
    ),
    faces = &[face!(
        name = "Leeching Licid",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: the Licid transformation and attached-controller upkeep damage.
    abilities = &[],
);
