//! Stinging Licid — {1}{U} — Creature — Licid
//! Oracle: {1}{U}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {U} to end this effect.
//! Oracle: Whenever enchanted creature becomes tapped, this creature deals 2 damage to that creature's controller.
//! Set: TMP #91 — Tempest | Scryfall ID: 807227d7-2eb2-4d47-bb3c-9d1ec9befeb7 | Oracle ID: c6e48eaf-02f8-43b3-90bf-970d4e412fa6
// PARTIAL: activates {1}{U}, {T} to deal 2 damage to target creature's controller; converting this creature into an Aura enchantment attached to target creature that triggers whenever enchanted creature becomes tapped, and paying {U} as a special action to end the effect, are not in Effect.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STINGING_LICID,
    oracle_id = "c6e48eaf-02f8-43b3-90bf-970d4e412fa6",
    scryfall_id = "807227d7-2eb2-4d47-bb3c-9d1ec9befeb7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "activates {1}{U}, {T} to deal 2 damage to target creature's controller; converting this creature into an Aura enchantment attached to target creature that triggers whenever enchanted creature becomes tapped, and paying {U} as a special action to end the effect, are not in Effect",
    ),
    faces = &[face!(
        name = "Stinging Licid",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{U}", TapSelf),
        &[Effect::DealDamageToTargetController {
            amount: Amount::Fixed(2),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
