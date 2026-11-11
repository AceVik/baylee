//! Qarsi Revenant — {1}{B}{B} — Creature — Vampire
//! Oracle: Flying, deathtouch, lifelink
//! Oracle: Renew — {2}{B}, Exile this card from your graveyard: Put a flying counter, a deathtouch counter, and a lifelink counter on target creature. Activate only as a sorcery.
//! Set: TDM #86 — Tarkir: Dragonstorm | Scryfall ID: 8c93a0f6-5e50-4dda-9ff6-da741fb839ff | Oracle ID: bbea79bc-71d4-4100-92f4-783c2c4b46da
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QARSI_REVENANT,
    oracle_id = "bbea79bc-71d4-4100-92f4-783c2c4b46da",
    scryfall_id = "8c93a0f6-5e50-4dda-9ff6-da741fb839ff",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Qarsi Revenant",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
