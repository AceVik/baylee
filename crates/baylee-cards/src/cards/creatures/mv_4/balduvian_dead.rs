//! Balduvian Dead — {3}{B} — Creature — Zombie
//! Oracle: {2}{R}, Exile a creature card from your graveyard: Create a 3/1 black and red Graveborn creature token with haste. Sacrifice it at the beginning of the next end step.
//! Set: ME2 #79 — Masters Edition II | Scryfall ID: c9aee135-65f6-4abb-b0ca-ee5ceb958a24 | Oracle ID: 18d6b89a-3588-43d9-a247-4747a7d784fc
// NOT SUPPORTED: sacrificing the Graveborn token at the beginning of the next end step is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BALDUVIAN_DEAD,
    oracle_id = "18d6b89a-3588-43d9-a247-4747a7d784fc",
    scryfall_id = "c9aee135-65f6-4abb-b0ca-ee5ceb958a24",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "{2}{R}, exiling a creature card from your graveyard creates a 3/1 black and red Graveborn creature token with haste; sacrificing that token at the beginning of the next end step is not supported",
    ),
    faces = &[face!(
        name = "Balduvian Dead",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{2}{R}", ExileFromGraveyard(&Filter::CREATURE)),
        &[Effect::CreateToken {
            token: &crate::generated_tokens::GRAVEBORN_3_1_BLACK_RED_HASTE,
        }],
    )],
);
