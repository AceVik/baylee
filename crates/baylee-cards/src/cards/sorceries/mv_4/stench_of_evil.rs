//! Stench of Evil — {2}{B}{B} — Sorcery
//! Oracle: Destroy all Plains. For each land destroyed this way, Stench of Evil deals 1 damage to that land's controller unless they pay {2}.
//! Set: ICE #165 — Ice Age | Scryfall ID: 4c7065a2-f819-4cbe-b453-a55e904f0461 | Oracle ID: 14062abe-14df-4dbe-ad7b-530e5f6c4988

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STENCH_OF_EVIL,
    oracle_id = "14062abe-14df-4dbe-ad7b-530e5f6c4988",
    scryfall_id = "4c7065a2-f819-4cbe-b453-a55e904f0461",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "destroys all Plains; dealing 1 damage per land destroyed this way to that land's controller unless that player pays {2} is not supported",
    ),
    faces = &[face!(
        name = "Stench of Evil",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        // CR 701.8a: Destroy all Plains.
        // NOT SUPPORTED: Dealing 1 damage per land destroyed this way to that land's controller unless that player pays {2}.
        spell!(&[Effect::DestroyAll {
            filter: &Filter::HasSubtype(subtypes::land::PLAINS),
            no_regen: false,
        }]),
    ],
);
