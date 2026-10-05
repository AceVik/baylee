//! Shelkin Brownie — {1}{G} — Creature — Ouphe
//! Oracle: {T}: Target creature loses all "bands with other" abilities until end of turn.
//! Set: LEG #204 — Legends | Scryfall ID: fddcc557-871d-425b-b4ee-bc0c9bc717aa | Oracle ID: 507ce455-bd02-49f9-bf64-4b2d21ffe0ea
// PARTIAL — the whole `{T}` ability is off the card; see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHELKIN_BROWNIE,
    oracle_id = "507ce455-bd02-49f9-bf64-4b2d21ffe0ea",
    scryfall_id = "fddcc557-871d-425b-b4ee-bc0c9bc717aa",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the engine models no \"bands with other\" ability (CR 702.22b), so \
         there is nothing a removal effect could say or remove"
    ),
    faces = &[face!(
        name = "Shelkin Brownie",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OUPHE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// NOT SUPPORTED: "{T}: Target creature loses all \"bands with other\"
// abilities until end of turn." — the cost, the target and the duration are
// all sayable (`Cost::TAP`, `TargetSpec::Object(&Filter::CREATURE)`,
// `Duration::UntilEndOfTurn`), but the thing removed is not: "bands with
// other" (CR 702.22b) is a family of keyword abilities naming a quality
// ("other legendary creatures", "creatures named Wolves of the Hunt"), and
// `KeywordSet` carries only the data-free `BANDING` bit (CR 702.22), a
// different ability. `Modifier::RemoveKeyword(KeywordSet::BANDING)` would
// strip banding instead: `Modifier::LoseKeywords` strips every keyword, and
// `Modifier::LoseAllAbilities` far more. The ability comes off the card
// rather than removing something else.
