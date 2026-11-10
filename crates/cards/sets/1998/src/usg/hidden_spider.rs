//! Hidden Spider — {G} — Enchantment
//! Oracle: When an opponent casts a creature spell with flying, if this permanent is an enchantment, it becomes a 3/5 Spider creature with reach. (It can block creatures with flying.)
//! Set: USG #264 — Urza's Saga | Scryfall ID: ab2640cb-7d0b-40e5-9624-02c64b2f9830 | Oracle ID: b0709570-a145-4c3c-ba56-f06d25753855
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_SPIDER,
    oracle_id = "b0709570-a145-4c3c-ba56-f06d25753855",
    scryfall_id = "ab2640cb-7d0b-40e5-9624-02c64b2f9830",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hidden Spider",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::And(&[
            Filter::CREATURE,
            Filter::ControlledByOpponent,
            Filter::HasKeyword(KeywordSet::FLYING),
        ])),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::SPIDER,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::REACH),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(3, 5), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
