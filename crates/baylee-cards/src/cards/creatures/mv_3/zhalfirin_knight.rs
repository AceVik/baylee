//! Zhalfirin Knight — {2}{W} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {W}{W}: This creature gains first strike until end of turn.
//! Set: MIR #50 — Mirage | Scryfall ID: eb65d104-bd50-481e-a70e-62aeb2f2c12b | Oracle ID: a534f06e-dae4-4d1a-96e9-7693bbb21e12
// IMPLEMENTED — flanking and {W}{W}: gains first strike until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZHALFIRIN_KNIGHT,
    oracle_id = "a534f06e-dae4-4d1a-96e9-7693bbb21e12",
    scryfall_id = "eb65d104-bd50-481e-a70e-62aeb2f2c12b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Zhalfirin Knight",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[activated!(
        cost!("{W}{W}"),
        &[Effect::continuous(
            &Filter::This,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
            Duration::UntilEndOfTurn,
        )],
    ),],
);
