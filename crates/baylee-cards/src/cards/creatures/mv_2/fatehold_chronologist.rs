//! Fatehold Chronologist // Peer Review — {1}{W/U} — Creature — Bird Wizard // Sorcery
//! Oracle: Flying
//! Oracle: This creature enters prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)
//! Oracle: Create a 2/2 colorless Wizard Soldier creature token named Cadet. Surveil 1.
//! Set: FRA #133 — Reality Fracture | Scryfall ID: 29e7ec16-0c16-48aa-8e09-ce6e0d5bd40b | Oracle ID: 1063822f-47d3-42e9-8a21-f62b12609fe1
//! Face: Fatehold Chronologist — {1}{W/U} — Creature — Bird Wizard
//! Face: Peer Review — {2}{W/U} — Sorcery
// PARTIAL — flying, "enters prepared" with the prepared copy ability, and the
// back face's surveil 1; the Cadet token has no entry in `crate::tokens`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PEER_REVIEW_SPELL: &[AbilityDef] = &[spell!(&[
    // NOT SUPPORTED: Create a 2/2 colorless Wizard Soldier creature token named
    // Cadet. — a token is reached through `crate::tokens`, whose ledger holds no
    // `Cadet`, and a card file may not define one of its own.
    Effect::surveil(1),
])];

card!(
    index = index::FATEHOLD_CHRONOLOGIST,
    oracle_id = "1063822f-47d3-42e9-8a21-f62b12609fe1",
    scryfall_id = "29e7ec16-0c16-48aa-8e09-ce6e0d5bd40b",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Partial("the Cadet token is not in crate::tokens"),
    faces = &[
        face!(
            name = "Fatehold Chronologist",
            mana_cost = mana!("{1}{W/U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
            power = Some(1),
            toughness = Some(2),
            keywords = KeywordSet::FLYING,
            enter_modifiers = &[EnterModifier::Prepared],
        ),
        face!(
            name = "Peer Review",
            mana_cost = mana!("{2}{W/U}"),
            types = TypeSet::SORCERY,
            abilities = PEER_REVIEW_SPELL,
        ),
    ],
    abilities = &[AbilityDef::Prepared {
        card: index::FATEHOLD_CHRONOLOGIST,
    }],
);
