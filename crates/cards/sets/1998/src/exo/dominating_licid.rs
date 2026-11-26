//! Dominating Licid — {1}{U}{U} — Creature — Licid
//! Oracle: {1}{U}{U}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {U} to end this effect.
//! Oracle: You control enchanted creature.
//! Set: EXO #30 — Exodus | Scryfall ID: e3e03323-43e8-4ddc-a874-211a97fd7648 | Oracle ID: ae2804c6-fb38-4845-ae21-7f41a766cbba

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DOMINATING_LICID,
    oracle_id = "ae2804c6-fb38-4845-ae21-7f41a766cbba",
    scryfall_id = "e3e03323-43e8-4ddc-a874-211a97fd7648",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dominating Licid",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(1),
        toughness = Some(1),
    )],
    abilities = &[
        activated!(
            cost!("{1}{U}{U}", TapSelf),
            &[Effect::BecomeAura {
                target: TargetSpec::Object(&Filter::CREATURE),
                enchant: TargetSpec::Object(&Filter::CREATURE),
                lose_this_ability: true,
                revert_cost: Some(mana!("{U}")),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::GainControl),
    ],
);
