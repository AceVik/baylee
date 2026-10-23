//! Sulfuric Vapors — {3}{R} — Enchantment
//! Oracle: If a red spell would deal damage to a permanent or player, it deals that much damage plus 1 to that permanent or player instead.
//! Set: USG #220 — Urza's Saga | Scryfall ID: e133997f-1620-4fe9-8275-057edc998fba | Oracle ID: 40b14aaa-f29e-4e84-9bdc-bc2ac9762ce9
// PARTIAL: adding 1 to damage dealt by red spells is not in DamageAmountReplacement; gives red creatures +1/+0 as a placeholder.

use baylee_cards_dsl::prelude::*;

static RED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);

card!(
    index = index::SULFURIC_VAPORS,
    oracle_id = "40b14aaa-f29e-4e84-9bdc-bc2ac9762ce9",
    scryfall_id = "e133997f-1620-4fe9-8275-057edc998fba",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "adding 1 to damage dealt by red spells is not in DamageAmountReplacement; gives red creatures +1/+0 as a placeholder",
    ),
    faces = &[face!(
        name = "Sulfuric Vapors",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(RED_CREATURE, Modifier::ModifyPT(1, 0)),],
);
