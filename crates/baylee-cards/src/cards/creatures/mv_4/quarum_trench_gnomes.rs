//! Quarum Trench Gnomes — {3}{R} — Creature — Gnome
//! Oracle: {T}: If target Plains is tapped for mana, it produces colorless mana instead of white mana. (This effect lasts indefinitely.)
//! Set: LEG #162 — Legends | Scryfall ID: 1c3b33bf-3074-406e-86f3-2a9843cf4862 | Oracle ID: ce5e52c9-4cb1-42aa-8403-bcd143d68704
// PARTIAL — the mana-replacement ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUARUM_TRENCH_GNOMES,
    oracle_id = "ce5e52c9-4cb1-42aa-8403-bcd143d68704",
    scryfall_id = "1c3b33bf-3074-406e-86f3-2a9843cf4862",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no Modifier replaces what a targeted land produces: SpendManaAs and \
         ManaIsAnyColor change how mana is spent, not what a land produces"
    ),
    faces = &[face!(
        name = "Quarum Trench Gnomes",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GNOME],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: "{T}: If target Plains is tapped for mana, it produces
    // colorless mana instead of white mana. (This effect lasts indefinitely.)"
    // — the {T} cost, the Plains target and the indefinite duration are all
    // sayable, but no `Modifier` rewrites a permanent's mana production
    // (white to colorless); the nearest, `Modifier::SpendManaAs`, is about
    // spending mana and is scoped to the effect's controller rather than to
    // the targeted land.
    abilities = &[],
);
