//! Forbidden Lore — {2}{G} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: Target creature gets +2/+1 until end of turn."
//! Set: ME2 #163 — Masters Edition II | Scryfall ID: 366069fb-3322-4992-aa1e-858951a8c492 | Oracle ID: d2853bad-a751-45ee-8269-24b0cdbc5c6b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FORBIDDEN_LORE_ABILITY: AbilityDef = activated!(
    Cost::TAP,
    &[Effect::PumpTarget {
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(1),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
);

card!(
    index = index::FORBIDDEN_LORE,
    oracle_id = "d2853bad-a751-45ee-8269-24b0cdbc5c6b",
    scryfall_id = "366069fb-3322-4992-aa1e-858951a8c492",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Forbidden Lore",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
        ),
        static_ability!(
            Filter::And(&[Filter::LAND, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&FORBIDDEN_LORE_ABILITY)
        ),
    ],
);
