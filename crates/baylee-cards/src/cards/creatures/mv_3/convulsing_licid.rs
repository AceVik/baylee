//! Convulsing Licid — {2}{R} — Creature — Licid
//! Oracle: {R}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {R} to end this effect.
//! Oracle: Enchanted creature can't block.
//! Set: STH #77 — Stronghold | Scryfall ID: d5e50a1d-b9f5-4a03-a1c2-ca45ace53a52 | Oracle ID: 893c88cb-72ba-4f54-809c-bfac4d6efb74
// PARTIAL: {R}, {T} prevents target creature from blocking this turn; becoming an Aura attached to target creature and paying {R} to end the effect are not supported

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONVULSING_LICID,
    oracle_id = "893c88cb-72ba-4f54-809c-bfac4d6efb74",
    scryfall_id = "d5e50a1d-b9f5-4a03-a1c2-ca45ace53a52",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "{R}, {T} prevents target creature from blocking this turn; becoming an Aura attached to target creature and paying {R} to end the effect are not supported"
    ),
    faces = &[face!(
        name = "Convulsing Licid",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{R}", TapSelf),
        &[Effect::ModifyTargets {
            modifier: Modifier::CantBlock(&Filter::Any),
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
