//! Flailing Drake — {3}{G} — Creature — Drake
//! Oracle: Flying
//! Oracle: Whenever this creature blocks or becomes blocked by a creature, that creature gets +1/+1 until end of turn.
//! Set: TMP #227 — Tempest | Scryfall ID: 43d246ac-1ca5-4c55-856c-4a83a4d638ab | Oracle ID: 93b6a170-a816-4556-aa27-a729231715e7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLAILING_DRAKE,
    oracle_id = "93b6a170-a816-4556-aa27-a729231715e7",
    scryfall_id = "43d246ac-1ca5-4c55-856c-4a83a4d638ab",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flailing Drake",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE),
        &[Effect::ModifyEvent {
            modifier: Modifier::ModifyPT(1, 1),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
