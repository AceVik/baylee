//! The Birth of Meletis — {1}{W} — Enchantment — Saga
//! Oracle: (As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! Oracle: I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle.
//! Oracle: II — Create a 0/4 colorless Wall artifact creature token with defender.
//! Oracle: III — You gain 2 life.
//! Set: THB #5 — Theros Beyond Death | Scryfall ID: d824f8bd-58f8-4796-80b5-5cd3033d35e8 | Oracle ID: 1ae54fe7-b1d3-4c13-a8ef-f502cf3eb1a0
// IMPLEMENTED — all three Saga chapters and their actual zone changes.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BASIC_PLAINS: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasSubtype(subtypes::land::PLAINS),
]);

card!(
    index = index::THE_BIRTH_OF_MELETIS,
    oracle_id = "1ae54fe7-b1d3-4c13-a8ef-f502cf3eb1a0",
    scryfall_id = "d824f8bd-58f8-4796-80b5-5cd3033d35e8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "The Birth of Meletis",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::SAGA],
    ),],
    abilities = &[
        chapter!(
            1,
            &[Effect::SearchLibrary {
                filter: &BASIC_PLAINS,
                finds: &[Find::HAND],
                optional: false,
            }]
        ),
        chapter!(
            2,
            &[Effect::CreateToken {
                token: &generated_tokens::WALL_ARTIFACT_0_4_DEFENDER,
            }]
        ),
        chapter!(3, &[Effect::gain_life(2)]),
    ],
);
