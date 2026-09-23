//! Moorland Haunt — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.
//! Set: VOC #175 — Crimson Vow Commander | Scryfall ID: 74deeeaa-95e9-42c9-89e3-e317ea63e216 | Oracle ID: 5324192b-6687-41e4-8e56-326b21a5dbf3

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOORLAND_HAUNT,
    oracle_id = "5324192b-6687-41e4-8e56-326b21a5dbf3",
    scryfall_id = "74deeeaa-95e9-42c9-89e3-e317ea63e216",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Moorland Haunt", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{W}{U}", TapSelf, ExileFromGraveyard(&Filter::CREATURE)),
            &[Effect::CreateToken {
                token: &generated_tokens::SPIRIT_1_1_WHITE_FLYING
            }]
        ),
    ],
);
