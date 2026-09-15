//! Ojer Axonil, Deepest Might // Temple of Power — {2}{R}{R} — Legendary Creature — God // Land
//! Oracle: Trample
//! Oracle: If a red source you control would deal an amount of noncombat damage less than Ojer Axonil's power to an opponent, that source deals damage equal to Ojer Axonil's power instead.
//! Oracle: When Ojer Axonil dies, return it to the battlefield tapped and transformed under its owner's control.
//! Oracle: (Transforms from Ojer Axonil, Deepest Might.)
//! Oracle: {T}: Add {R}.
//! Oracle: {2}{R}, {T}: Transform this land. Activate only if red sources you controlled dealt 4 or more noncombat damage this turn and only as a sorcery.
//! Set: LCI #158 — The Lost Caverns of Ixalan | Scryfall ID: 50f8e2b6-98c7-4f28-bb39-e1fbe841f1ee | Oracle ID: d3b7b541-6f05-46c1-8031-c848c4bd4635
//! Face: Ojer Axonil, Deepest Might — {2}{R}{R} — Legendary Creature — God
//! Face: Temple of Power —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 826,
    oracle_id: "d3b7b541-6f05-46c1-8031-c848c4bd4635",
    scryfall_id: "50f8e2b6-98c7-4f28-bb39-e1fbe841f1ee",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    commander: CommanderRule::Legendary,
    faces: &[
    face! {
        name: "Ojer Axonil, Deepest Might",
        mana_cost: mana!("{2}{R}{R}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::GOD],
        power: Some(4),
        toughness: Some(4),
    },
    face! {
        name: "Temple of Power",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
