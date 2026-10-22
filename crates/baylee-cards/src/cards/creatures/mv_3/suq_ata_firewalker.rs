//! Suq'Ata Firewalker — {1}{U}{U} — Creature — Human Wizard
//! Oracle: This creature can't be the target of red spells or abilities from red sources.
//! Oracle: {T}: This creature deals 1 damage to any target.
//! Set: MIR #94 — Mirage | Scryfall ID: b8a7c22e-fe96-4960-96d4-ee85abec3281 | Oracle ID: c9c0bb7e-8dfc-481c-80f7-b6652b63df39
// IMPLEMENTED — can't be targeted by red spells or abilities from red sources, and taps to deal 1 damage to any target.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static RED_SOURCE: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

card!(
    index = index::SUQ_ATA_FIREWALKER,
    oracle_id = "c9c0bb7e-8dfc-481c-80f7-b6652b63df39",
    scryfall_id = "b8a7c22e-fe96-4960-96d4-ee85abec3281",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Suq'Ata Firewalker",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::CantBeTargetedBy(&RED_SOURCE)),
        activated!(
            Cost::TAP,
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            target = Some(TargetSpec::AnyTarget),
        ),
    ],
);
