//! Femeref Knight — {2}{W} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {W}: This creature gains vigilance until end of turn.
//! Set: MIR #18 — Mirage | Scryfall ID: 915a2e07-b449-4d94-93e3-e756e891c542 | Oracle ID: 4365521e-996d-4eca-bbc9-b0988fa973ae
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FEMEREF_KNIGHT,
    oracle_id = "4365521e-996d-4eca-bbc9-b0988fa973ae",
    scryfall_id = "915a2e07-b449-4d94-93e3-e756e891c542",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Femeref Knight",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[activated!(
        cost!("{W}"),
        &[Effect::ModifySource {
            modifier: Modifier::AddKeyword(KeywordSet::VIGILANCE),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
