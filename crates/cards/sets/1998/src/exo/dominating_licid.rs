//! Dominating Licid — {1}{U}{U} — Creature — Licid
//! Oracle: {1}{U}{U}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {U} to end this effect.
//! Oracle: You control enchanted creature.
//! Set: EXO #30 — Exodus | Scryfall ID: e3e03323-43e8-4ddc-a874-211a97fd7648 | Oracle ID: ae2804c6-fb38-4845-ae21-7f41a766cbba
// PARTIAL — Licid reconfiguration into an Aura enchantment and paying {U} to end the effect are not supported; provides only the static control-changing effect if attached.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Licid reconfiguration into an Aura enchantment and paying {U} to end the effect are not supported; provides only the static control-changing effect if attached.
card!(
    index = index::DOMINATING_LICID,
    oracle_id = "ae2804c6-fb38-4845-ae21-7f41a766cbba",
    scryfall_id = "e3e03323-43e8-4ddc-a874-211a97fd7648",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Licid reconfiguration into an Aura enchantment and paying {U} to end the effect are not supported; provides only the static control-changing effect if attached."
    ),
    faces = &[face!(
        name = "Dominating Licid",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::AttachedToBySource,
        Modifier::GainControl
    )],
);
