//! Shelkin Brownie — {1}{G} — Creature — Ouphe
//! Oracle: {T}: Target creature loses all "bands with other" abilities until end of turn.
//! Set: LEG #204 — Legends | Scryfall ID: fddcc557-871d-425b-b4ee-bc0c9bc717aa | Oracle ID: 507ce455-bd02-49f9-bf64-4b2d21ffe0ea

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHELKIN_BROWNIE,
    oracle_id = "507ce455-bd02-49f9-bf64-4b2d21ffe0ea",
    scryfall_id = "fddcc557-871d-425b-b4ee-bc0c9bc717aa",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shelkin Brownie",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OUPHE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::continuous(
            &Filter::This,
            Modifier::LoseAllBandsWithOther,
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
