//! Nullmage Advocate — {2}{G} — Creature — Insect Druid
//! Oracle: {T}: Return two target cards from an opponent's graveyard to their hand. Destroy target artifact or enchantment.
//! Set: PCA #70 — Planechase Anthology | Scryfall ID: f9b6338c-3ddf-47e2-8e03-ad536bcf474c | Oracle ID: 558568a6-5e22-482f-9096-e76682965c21
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NULLMAGE_ADVOCATE,
    oracle_id = "558568a6-5e22-482f-9096-e76682965c21",
    scryfall_id = "f9b6338c-3ddf-47e2-8e03-ad536bcf474c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nullmage Advocate",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
