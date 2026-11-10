//! Elvish Berserker — {G} — Creature — Elf Berserker
//! Oracle: Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it.
//! Set: 10E #260 — Tenth Edition | Scryfall ID: 777633d7-e31c-4ea3-a085-2858d6ee6040 | Oracle ID: c9046281-dedb-4246-8e5d-7786fe0c41c7
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_BERSERKER,
    oracle_id = "c9046281-dedb-4246-8e5d-7786fe0c41c7",
    scryfall_id = "777633d7-e31c-4ea3-a085-2858d6ee6040",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvish Berserker",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::BERSERKER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlocked,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::CountOf {
                filter: &Filter::BlockingSource,
                zone: ZoneSel::Battlefield,
            },
            toughness: Amount::CountOf {
                filter: &Filter::BlockingSource,
                zone: ZoneSel::Battlefield,
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
