//! Murktide Regent — {5}{U}{U} — Creature — Dragon
//! Oracle: Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)
//! Oracle: Flying
//! Oracle: This creature enters with a +1/+1 counter on it for each instant and sorcery card exiled with it.
//! Oracle: Whenever an instant or sorcery card leaves your graveyard, put a +1/+1 counter on this creature.
//! Set: MH2 #52 — Modern Horizons 2 | Scryfall ID: 20c4aae1-7665-4df7-bd51-a1d95bf8a17d | Oracle ID: 7f993ac7-c2cd-413c-a106-2c051a77ebf6
// IMPLEMENTED — delve, enters with a +1/+1 counter for each instant/sorcery card exiled with it, and grows whenever an instant or sorcery card leaves your graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TRIGGER1: Filter = Filter::And(&[Filter::INSTANT_OR_SORCERY, Filter::OwnedByYou]);

card!(
    index = index::MURKTIDE_REGENT,
    oracle_id = "7f993ac7-c2cd-413c-a106-2c051a77ebf6",
    scryfall_id = "20c4aae1-7665-4df7-bd51-a1d95bf8a17d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Murktide Regent",
        mana_cost = mana!("{5}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
        delve = true,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::CountOf {
                filter: &Filter::INSTANT_OR_SORCERY,
                zone: ZoneSel::LinkedExile,
            },
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::LeavesGraveyard(&TRIGGER1),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }]
    )],
);
