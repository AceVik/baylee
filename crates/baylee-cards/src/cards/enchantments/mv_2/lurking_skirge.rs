//! Lurking Skirge — {1}{B} — Enchantment
//! Oracle: When a creature is put into an opponent's graveyard from the battlefield, if this permanent is an enchantment, it becomes a 3/2 Phyrexian Imp creature with flying.
//! Set: ULG #55 — Urza's Legacy | Scryfall ID: 9063cc12-a822-4488-856e-93d70ecfe37f | Oracle ID: 21e90420-aedb-4a84-ba57-d5eae497b09b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_OWNED_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::OwnedByYou)]);

card!(
    index = index::LURKING_SKIRGE,
    oracle_id = "21e90420-aedb-4a84-ba57-d5eae497b09b",
    scryfall_id = "9063cc12-a822-4488-856e-93d70ecfe37f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lurking Skirge",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&OPPONENT_OWNED_CREATURE),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::PHYREXIAN,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddSubtype(subtypes::creature::IMP),
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(3, 2), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
