//! Reckless Ogre — {3}{R} — Creature — Ogre
//! Oracle: Whenever this creature attacks alone, it gets +3/+0 until end of turn.
//! Set: EXO #98 — Exodus | Scryfall ID: 90d27b79-a22d-48d9-86b2-7ad02cab8697 | Oracle ID: fe455177-d9e5-4893-ad87-ebbba5e2fa45
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RECKLESS_OGRE,
    oracle_id = "fe455177-d9e5-4893-ad87-ebbba5e2fa45",
    scryfall_id = "90d27b79-a22d-48d9-86b2-7ad02cab8697",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reckless Ogre",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAlone(&Filter::This),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(3, 0),
            duration: Duration::UntilEndOfTurn,
        }]
    ),],
);
