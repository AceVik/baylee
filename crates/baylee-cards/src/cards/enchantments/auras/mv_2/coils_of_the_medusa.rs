//! Coils of the Medusa — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/-1.
//! Oracle: Sacrifice this Aura: Destroy all non-Wall creatures blocking enchanted creature.
//! Set: WTH #65 — Weatherlight | Scryfall ID: 502bfb38-4a37-4053-af20-d5606ffc67c8 | Oracle ID: 02a8a4e6-2081-4903-81ad-96e5a949dff8
// NOT SUPPORTED: narrowing the destroyed blockers specifically to creatures blocking enchanted creature requires a BlockingAttached filter variant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NON_WALL_BLOCKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Blocking,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::COILS_OF_THE_MEDUSA,
    oracle_id = "02a8a4e6-2081-4903-81ad-96e5a949dff8",
    scryfall_id = "502bfb38-4a37-4053-af20-d5606ffc67c8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "enchants a creature, gives enchanted creature +1/-1, and has 'Sacrifice this Aura: Destroy all non-Wall blocking creatures'; narrowing the destroyed blockers specifically to creatures blocking enchanted creature requires a BlockingAttached filter variant",
    ),
    faces = &[face!(
        name = "Coils of the Medusa",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(1, -1)
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::destroy_all(&NON_WALL_BLOCKING_CREATURE)],
        ),
    ],
);
