//! Searing Spear Askari — {2}{R} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {1}{R}: This creature gains menace until end of turn. (It can't be blocked except by two or more creatures.)
//! Set: MIR #191 — Mirage | Scryfall ID: 5cf66916-7f6b-412f-acd6-f96ad4539a46 | Oracle ID: d3c68573-95b1-4db5-bbea-363b86f713c1

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEARING_SPEAR_ASKARI,
    oracle_id = "d3c68573-95b1-4db5-bbea-363b86f713c1",
    scryfall_id = "5cf66916-7f6b-412f-acd6-f96ad4539a46",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Searing Spear Askari",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}"),
        &[Effect::continuous(
            &Filter::This,
            Modifier::AddKeyword(KeywordSet::MENACE),
            Duration::UntilEndOfTurn,
        )],
    ),],
);
