//! Phyrexian Processor — {4} — Artifact
//! Oracle: As this artifact enters, pay any amount of life.
//! Oracle: {4}, {T}: Create an X/X black Phyrexian Minion creature token, where X is the life paid as this artifact entered.
//! Set: USG #306 — Urza's Saga | Scryfall ID: 6875ce99-badd-44da-8e5d-509600efa1d0 | Oracle ID: 36c800cb-b1ca-4432-ad3c-4d8b90337f4c
// PARTIAL: paying any amount of life as this artifact enters and creating an X/X black Phyrexian Minion creature token where X is the life paid is not in EnterModifier/Effect; activates {4}, {T} to create a 2/2 black Zombie creature token.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_PROCESSOR,
    oracle_id = "36c800cb-b1ca-4432-ad3c-4d8b90337f4c",
    scryfall_id = "6875ce99-badd-44da-8e5d-509600efa1d0",
    coverage = Coverage::Partial(
        "paying any amount of life as this artifact enters and creating an X/X black Phyrexian Minion creature token where X is the life paid is not in EnterModifier/Effect; activates {4}, {T} to create a 2/2 black Zombie creature token",
    ),
    faces = &[face!(
        name = "Phyrexian Processor",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[Effect::CreateToken {
            token: &crate::generated_tokens::ZOMBIE_2_2_BLACK,
        }],
    ),],
);
