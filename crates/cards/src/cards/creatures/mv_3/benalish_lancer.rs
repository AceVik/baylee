//! Benalish Lancer — {2}{W} — Creature — Human Knight
//! Oracle: Kicker {2}{W} (You may pay an additional {2}{W} as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with two +1/+1 counters on it and with first strike.
//! Set: DDG #12 — Duel Decks: Knights vs. Dragons | Scryfall ID: 8453b08f-34cb-491d-be7d-dc4022f7c131 | Oracle ID: b5adbd37-8d98-4c65-b42d-cd9db94e7222
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BENALISH_LANCER,
    oracle_id = "b5adbd37-8d98-4c65-b42d-cd9db94e7222",
    scryfall_id = "8453b08f-34cb-491d-be7d-dc4022f7c131",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Benalish Lancer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
