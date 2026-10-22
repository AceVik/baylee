//! Wild Wurm — {3}{R} — Creature — Wurm
//! Oracle: When this creature enters, flip a coin. If you lose the flip, return this creature to its owner's hand.
//! Set: TMP #212 — Tempest | Scryfall ID: e5a1b213-cba0-4eca-b058-93f4fde717c8 | Oracle ID: ba14d185-9432-4a67-8bfb-544c2cebe71d
// IMPLEMENTED — ETB coin flip returning this creature to its owner's hand on a lost flip.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WILD_WURM,
    oracle_id = "ba14d185-9432-4a67-8bfb-544c2cebe71d",
    scryfall_id = "e5a1b213-cba0-4eca-b058-93f4fde717c8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wild Wurm",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::FlipCoin {
            won: &[],
            lost: &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        }],
    )],
);
