//! Dwarven Berserker — {1}{R} — Creature — Dwarf Berserker
//! Oracle: Whenever this creature becomes blocked, it gets +3/+0 and gains trample until end of turn.
//! Set: WTH #97 — Weatherlight | Scryfall ID: 7bc734e9-fb09-4094-94b6-76c0458649e9 | Oracle ID: cc43507a-ca61-45ae-ac0a-4ddb851c27ef
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_BERSERKER,
    oracle_id = "cc43507a-ca61-45ae-ac0a-4ddb851c27ef",
    scryfall_id = "7bc734e9-fb09-4094-94b6-76c0458649e9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dwarven Berserker",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::BERSERKER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlocked,
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(3, 0),
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::TRAMPLE),
                Duration::UntilEndOfTurn,
            ),
        ],
    )],
);
