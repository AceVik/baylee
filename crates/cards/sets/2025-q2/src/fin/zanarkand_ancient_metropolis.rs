//! Zanarkand, Ancient Metropolis // Lasting Fayth — (no cost) — Land — Town // Sorcery — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: Create a 1/1 colorless Hero creature token. Put a +1/+1 counter on it for each land you control. (Then exile this card. You may play the land later from exile.)
//! Set: FIN #293 — Final Fantasy | Scryfall ID: 881e4c00-3b9a-47a1-bf66-1badda994c88 | Oracle ID: 5f2b3ea8-99ee-47a4-8a1c-4b27478d524c
//! Face: Zanarkand, Ancient Metropolis —  — Land — Town
//! Face: Lasting Fayth — {4}{G}{G} — Sorcery — Adventure

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZANARKAND_ANCIENT_METROPOLIS,
    oracle_id = "5f2b3ea8-99ee-47a4-8a1c-4b27478d524c",
    scryfall_id = "881e4c00-3b9a-47a1-bf66-1badda994c88",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Zanarkand, Ancient Metropolis",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Lasting Fayth",
            mana_cost = mana!("{4}{G}{G}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
            adventure = true,
            abilities = &[spell!(&[Effect::CreateTokenThenCounters {
                token: &crate::generated_tokens::HERO_1_1,
                kind: CounterKind::P1P1,
                amount: Amount::CountOf {
                    filter: &f!(your LAND),
                    zone: ZoneSel::Battlefield
                },
            }])],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
