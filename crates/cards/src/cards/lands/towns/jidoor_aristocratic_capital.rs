//! Jidoor, Aristocratic Capital // Overture — (no cost) — Land — Town // Sorcery — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: Target opponent mills half their library, rounded down. (Then exile this card. You may play the land later from exile.)
//! Set: FIN #284 — Final Fantasy | Scryfall ID: 98b2d5b5-f85b-4c42-a0f5-a76f6af304ba | Oracle ID: bd513d9d-5aa2-4860-bd86-8b5d9430f133
//! Face: Jidoor, Aristocratic Capital —  — Land — Town
//! Face: Overture — {4}{U}{U} — Sorcery — Adventure

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JIDOOR_ARISTOCRATIC_CAPITAL,
    oracle_id = "bd513d9d-5aa2-4860-bd86-8b5d9430f133",
    scryfall_id = "98b2d5b5-f85b-4c42-a0f5-a76f6af304ba",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Jidoor, Aristocratic Capital",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Overture",
            mana_cost = mana!("{4}{U}{U}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
            adventure = true,
            abilities = &[spell!(
                &[Effect::Mill {
                    amount: Amount::Half {
                        base: &Amount::LibrarySize(PlayerRel::Chosen),
                        round_up: false
                    },
                    target: PlayerRel::Chosen,
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
            )],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
