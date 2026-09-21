//! Planar Nexus — (no cost) — Land
//! Oracle: This land is every nonbasic land type. (Nonbasic land types include Cave, Desert, Gate, Lair, Locus, Mine, Power-Plant, Sphere, Tower, and Urza's.)
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Set: M3C #80 — Modern Horizons 3 Commander | Scryfall ID: 28603c1c-f9b4-4001-bc56-d1453d5cacf5 | Oracle ID: 26005003-afcb-4c32-a760-be950246ff0f
// PARTIAL — both mana abilities are built; the land's "every nonbasic land
// type" has no modifier to say it with.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_NEXUS,
    oracle_id = "26005003-afcb-4c32-a760-be950246ff0f",
    scryfall_id = "28603c1c-f9b4-4001-bc56-d1453d5cacf5",
    faces = &[face!(name = "Planar Nexus", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "this land is every nonbasic land type — Modifier carries AllCreatureTypes \
         and AllBasicLandTypes but no modifier grants a set of land types"
    ),
    // NOT SUPPORTED: "This land is every nonbasic land type." — the only two
    // all-types modifiers are AllCreatureTypes (creatures) and
    // AllBasicLandTypes (the five basics), and neither is this sentence: it
    // names the ten *nonbasic* land types and says "include", so enumerating
    // them as ten AddSubtype statics would be a hand-kept list that the next
    // printed nonbasic land type leaves wrong.
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
    ],
);
