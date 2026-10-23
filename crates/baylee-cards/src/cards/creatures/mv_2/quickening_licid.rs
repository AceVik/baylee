//! Quickening Licid — {1}{W} — Creature — Licid
//! Oracle: {1}{W}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {W} to end this effect.
//! Oracle: Enchanted creature has first strike.
//! Set: TMP #36 — Tempest | Scryfall ID: e6e91f3d-5a23-4df1-a879-d18a3af92a28 | Oracle ID: 2bdd9ca3-542a-43c0-9831-c6add23890d6

// PARTIAL — Licid Aura transformation and special action to end it are not supported; only the 1/1 body is played.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUICKENING_LICID,
    oracle_id = "2bdd9ca3-542a-43c0-9831-c6add23890d6",
    scryfall_id = "e6e91f3d-5a23-4df1-a879-d18a3af92a28",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Licid Aura transformation and special action to end it are not supported; only the 1/1 body is played."
    ),
    faces = &[face!(
        name = "Quickening Licid",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: Licid Aura transformation and special action to end it are not supported; only the 1/1 body is played.
);
