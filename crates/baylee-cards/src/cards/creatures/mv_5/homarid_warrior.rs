//! Homarid Warrior — {4}{U} — Creature — Homarid Warrior
//! Oracle: {U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap it. (A creature with shroud can't be the target of spells or abilities.)
//! Set: 5ED #92 — Fifth Edition | Scryfall ID: 80f20d8e-bbdf-4af8-9505-d5604b04ad72 | Oracle ID: 6facb0f6-fa49-4ce6-9f95-15bb3ce4bcf2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HOMARID_WARRIOR,
    oracle_id = "6facb0f6-fa49-4ce6-9f95-15bb3ce4bcf2",
    scryfall_id = "80f20d8e-bbdf-4af8-9505-d5604b04ad72",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Homarid Warrior",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HOMARID, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{U}"),
        &[
            Effect::ModifySource {
                modifier: Modifier::AddKeyword(KeywordSet::SHROUD),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifySource {
                modifier: Modifier::DoesNotUntap,
                duration: Duration::UntilYourNextUntapStep,
            },
            Effect::TapSelf,
        ],
    ),],
);
