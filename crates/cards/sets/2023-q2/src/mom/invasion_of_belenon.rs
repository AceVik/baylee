//! Invasion of Belenon // Belenon War Anthem — {2}{W} — Battle — Siege // Enchantment
//! Oracle: (As a Siege enters, choose an opponent to protect it. You and others can attack it. When it's defeated, exile it, then cast it transformed.)
//! Oracle: When this Siege enters, create a 2/2 white and blue Knight creature token with vigilance.
//! Oracle: Creatures you control get +1/+1.
//! Set: MOM #20 — March of the Machine | Scryfall ID: 4fb0eb2d-9cb7-4e72-a970-5009b046df2a | Oracle ID: e462e78d-2d26-4a2d-8d8f-5f31abe5c924
//! Face: Invasion of Belenon — {2}{W} — Battle — Siege
//! Face: Belenon War Anthem —  — Enchantment
// Full Siege front and transformed anthem back.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INVASION_OF_BELENON,
    oracle_id = "e462e78d-2d26-4a2d-8d8f-5f31abe5c924",
    scryfall_id = "4fb0eb2d-9cb7-4e72-a970-5009b046df2a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Invasion of Belenon",
            mana_cost = mana!("{2}{W}"),
            types = TypeSet::BATTLE,
            subtypes = &[subtypes::battle::SIEGE],
            defense = Some(5),
            abilities = &[triggered!(
                Trigger::ETB,
                &[Effect::CreateToken {
                    token: &crate::tokens::KNIGHT_2_2_WHITE_BLUE_VIGILANCE,
                }]
            )],
        ),
        face!(
            name = "Belenon War Anthem",
            types = TypeSet::ENCHANTMENT,
            color_indicator = ColorSet::from_slice(&[Color::White]),
            castable_from_hand = false,
            abilities = &[static_ability!(
                Filter::YOUR_CREATURE,
                Modifier::ModifyPT(1, 1)
            )],
        ),
    ],
);
