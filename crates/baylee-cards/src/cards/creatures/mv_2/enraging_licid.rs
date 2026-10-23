//! Enraging Licid — {1}{R} — Creature — Licid
//! Oracle: {R}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {R} to end this effect.
//! Oracle: Enchanted creature has haste.
//! Set: TMP #171 — Tempest | Scryfall ID: fb7bff44-36e1-4855-aa2d-5c7bd6bf6f10 | Oracle ID: 5ec9f317-887a-4e08-a165-e7cf15eb071d
// PARTIAL — Only the 1/1 creature body is supported; changing into an attached Aura, losing the activation and paying to end the effect is unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENRAGING_LICID,
    oracle_id = "5ec9f317-887a-4e08-a165-e7cf15eb071d",
    scryfall_id = "fb7bff44-36e1-4855-aa2d-5c7bd6bf6f10",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Only the 1/1 creature body is supported; changing into an attached Aura, losing the activation and paying to end the effect is unsupported."
    ),
    faces = &[face!(
        name = "Enraging Licid",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: the Licid transformation and its linked haste grant.
    abilities = &[],
);
