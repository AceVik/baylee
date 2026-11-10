//! Opal Champion — {2}{W} — Enchantment
//! Oracle: When an opponent casts a creature spell, if this permanent is an enchantment, it becomes a 3/3 Knight creature with first strike.
//! Set: ULG #16 — Urza's Legacy | Scryfall ID: 2699cf3b-df54-4c77-ba19-3bc7598ae3fa | Oracle ID: 0d3617c3-1526-40e6-bc90-6139b5d3e1f1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OPAL_CHAMPION,
    oracle_id = "0d3617c3-1526-40e6-bc90-6139b5d3e1f1",
    scryfall_id = "2699cf3b-df54-4c77-ba19-3bc7598ae3fa",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Opal Champion",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::OPPONENT_CREATURE),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::KNIGHT,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
