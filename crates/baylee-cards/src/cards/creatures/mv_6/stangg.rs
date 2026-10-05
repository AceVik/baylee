//! Stangg — {4}{R}{G} — Legendary Creature — Human Warrior
//! Oracle: When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.
//! Set: A25 #218 — Masters 25 | Scryfall ID: a23f846e-b5e6-48c2-afea-c9be7fc8b7c3 | Oracle ID: 739eac91-3029-4ce7-9885-0af3ddea472e
// PARTIAL — the enters trigger creates Stangg Twin; the linked exile/sacrifice pair is off the card (see NOT SUPPORTED below).

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STANGG,
    oracle_id = "739eac91-3029-4ce7-9885-0af3ddea472e",
    scryfall_id = "a23f846e-b5e6-48c2-afea-c9be7fc8b7c3",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "nothing remembers the token a `CreateToken` made, so no later \
         ability can name *that* token: the two linked clauses have no \
         vocabulary"
    ),
    faces = &[face!(
        name = "Stangg",
        mana_cost = mana!("{4}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: "Exile that token when Stangg leaves the battlefield.
    // Sacrifice Stangg when that token leaves the battlefield." — Stangg
    // must remember the token it created and act when either permanent
    // leaves. `Effect::CreateToken` records no link
    // (`CreateTokenFromLinked` and its `Rider::Linked` are the reverse — a
    // token made from a card exiled with the source), so
    // `Trigger::LeavesBattlefield` has no object to exile, and a trigger on
    // `Filter::Named("Stangg Twin")` would sacrifice Stangg for a twin
    // another Stangg made.
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::CreateToken {
            token: &generated_tokens::STANGG_TWIN_LEGENDARY_3_4_RED_GREEN,
        }]
    )],
);
