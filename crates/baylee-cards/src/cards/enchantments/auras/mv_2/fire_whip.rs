//! Fire Whip — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature you control
//! Oracle: Enchanted creature has "{T}: This creature deals 1 damage to any target."
//! Oracle: Sacrifice this Aura: It deals 1 damage to any target.
//! Set: TSB #63 — Time Spiral Timeshifted | Scryfall ID: 53948505-1057-4d68-aa6f-291dfa63d475 | Oracle ID: 9d4372f0-eedf-4897-9f03-f96fb14ac4a6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FIRE_WHIP_PING: AbilityDef = activated!(
    Cost::TAP,
    &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyTarget,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
);

card!(
    index = index::FIRE_WHIP,
    oracle_id = "9d4372f0-eedf-4897-9f03-f96fb14ac4a6",
    scryfall_id = "53948505-1057-4d68-aa6f-291dfa63d475",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fire Whip",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&FIRE_WHIP_PING)
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            target = Some(TargetSpec::AnyTarget),
        ),
    ],
);
