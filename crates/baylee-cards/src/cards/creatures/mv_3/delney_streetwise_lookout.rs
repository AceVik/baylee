//! Delney, Streetwise Lookout — {2}{W} — Legendary Creature — Human Scout
//! Oracle: Creatures you control with power 2 or less can't be blocked by creatures with power 3 or greater.
//! Oracle: If a triggered ability of a creature you control with power 2 or less triggers, that ability triggers an additional time.
//! Set: MKM #12 — Murders at Karlov Manor | Scryfall ID: be219928-3d0e-4d00-b124-152ce8a8c13b | Oracle ID: 245d0ccf-87b6-460a-8b99-9e2079f2d375
// IMPLEMENTED — your creatures with power 2 or less can't be blocked by power
// 3 or greater, and their triggered abilities trigger an additional time.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DELNEY_STREETWISE_LOOKOUT,
    oracle_id = "245d0ccf-87b6-460a-8b99-9e2079f2d375",
    scryfall_id = "be219928-3d0e-4d00-b124-152ce8a8c13b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Delney, Streetwise Lookout",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SCOUT],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::And(&[Filter::YOUR_CREATURE, Filter::PowerAtMost(2)]),
            Modifier::CantBeBlockedBy(&Filter::PowerAtLeast(3))
        ),
        AbilityDef::Replacement(ReplacementRule::TriggerMultiplier {
            source_filter: &Filter::And(&[Filter::YOUR_CREATURE, Filter::PowerAtMost(2)]),
            event: TriggerEventKind::Any,
        }),
    ],
);
