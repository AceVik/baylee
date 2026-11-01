//! Stinging Licid — {1}{U} — Creature — Licid
//! Oracle: {1}{U}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {U} to end this effect.
//! Oracle: Whenever enchanted creature becomes tapped, this creature deals 2 damage to that creature's controller.
//! Set: TMP #91 — Tempest | Scryfall ID: 807227d7-2eb2-4d47-bb3c-9d1ec9befeb7 | Oracle ID: c6e48eaf-02f8-43b3-90bf-970d4e412fa6

// PARTIAL — Licid Aura transformation, special action to end it, and the enchanted creature becoming tapped trigger are not supported; only the 1/1 body is played.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STINGING_LICID,
    oracle_id = "c6e48eaf-02f8-43b3-90bf-970d4e412fa6",
    scryfall_id = "807227d7-2eb2-4d47-bb3c-9d1ec9befeb7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Licid Aura transformation, special action to end it, and the enchanted creature becoming tapped trigger are not supported; only the 1/1 body is played."
    ),
    faces = &[face!(
        name = "Stinging Licid",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: Licid Aura transformation, special action to end it, and the enchanted creature becoming tapped trigger are not supported; only the 1/1 body is played.
);
