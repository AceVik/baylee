//! Thragtusk — {4}{G} — Creature — Beast
//! Oracle: When this creature enters, you gain 5 life.
//! Oracle: When this creature leaves the battlefield, create a 3/3 green Beast creature token.
//! Set: NCC #316 — New Capenna Commander | Scryfall ID: beda7acd-e970-4222-9577-5133765d6052 | Oracle ID: 0dd0e91a-d16b-4718-8d11-1a3fcf8e0753
// IMPLEMENTED — enters: gain 5 life; leaves the battlefield (any way, CR 603.6c): a 3/3 Beast.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRAGTUSK,
    oracle_id = "0dd0e91a-d16b-4718-8d11-1a3fcf8e0753",
    scryfall_id = "beda7acd-e970-4222-9577-5133765d6052",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Thragtusk",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::gain_life(5)]),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::CreateToken {
                token: &generated_tokens::BEAST_3_3_GREEN
            }]
        ),
    ],
);
