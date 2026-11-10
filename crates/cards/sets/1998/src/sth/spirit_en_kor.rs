//! Spirit en-Kor — {3}{W} — Creature — Kor Spirit
//! Oracle: Flying
//! Oracle: {0}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.
//! Set: TPR #34 — Tempest Remastered | Scryfall ID: 8c57ca87-75b0-4058-acd8-5394545d0846 | Oracle ID: f066174a-a959-4365-920f-c04506d94a5a
// PARTIAL: {0} activated ability redirecting the next 1 damage dealt to this creature to target creature you control is not supported; plays as a 2/2 Kor Spirit with flying.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPIRIT_EN_KOR,
    oracle_id = "f066174a-a959-4365-920f-c04506d94a5a",
    scryfall_id = "8c57ca87-75b0-4058-acd8-5394545d0846",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "{0} activated ability redirecting the next 1 damage dealt to this creature to target creature you control is not supported; plays as a 2/2 Kor Spirit with flying",
    ),
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Spirit en-Kor",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KOR, subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
);
