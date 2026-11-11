//! Hooded Blightfang — {2}{B} — Creature — Snake
//! Oracle: Deathtouch
//! Oracle: Whenever a creature you control with deathtouch attacks, each opponent loses 1 life and you gain 1 life.
//! Oracle: Whenever a creature you control with deathtouch deals damage to a planeswalker, destroy that planeswalker.
//! Set: J25 #455 — Foundations Jumpstart | Scryfall ID: e419f0fc-cf43-4b7c-ae75-83b125c20da7 | Oracle ID: 342eb666-838c-43d0-bf52-2f704f2a80e3
// IMPLEMENTED — each qualifying attacker drains independently; any positive
// damage to a planeswalker uses the damaged permanent as the event object.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_DEATHTOUCH_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::HasKeyword(KeywordSet::DEATHTOUCH),
]);

card!(
    index = index::HOODED_BLIGHTFANG,
    oracle_id = "342eb666-838c-43d0-bf52-2f704f2a80e3",
    scryfall_id = "e419f0fc-cf43-4b7c-ae75-83b125c20da7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hooded Blightfang",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(1),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&YOUR_DEATHTOUCH_CREATURE),
            &[
                Effect::LoseLife {
                    amount: Amount::Fixed(1),
                    target: PlayerRel::EachOpponent,
                },
                Effect::gain_life(1),
            ]
        ),
        triggered!(
            Trigger::DealsDamageToObject {
                source: &YOUR_DEATHTOUCH_CREATURE,
                recipient: &Filter::PLANESWALKER,
            },
            &[Effect::destroy(TargetSpec::EventObject)]
        ),
    ],
);
