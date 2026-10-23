//! Walking Sponge — {1}{U} — Creature — Sponge
//! Oracle: {T}: Target creature loses your choice of flying, first strike, or trample until end of turn.
//! Set: ULG #47 — Urza's Legacy | Scryfall ID: b125d1e7-5d9b-4997-88b0-71bdfc19c6f2 | Oracle ID: 000e5d65-96c3-498b-bd01-72b1a1991850
// PARTIAL: models the choice of flying, first strike, or trample as three separate '{T}' activated abilities that each remove one of those keywords from target creature until end of turn; choosing the keyword to remove on resolution is not in Effect.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALKING_SPONGE,
    oracle_id = "000e5d65-96c3-498b-bd01-72b1a1991850",
    scryfall_id = "b125d1e7-5d9b-4997-88b0-71bdfc19c6f2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "models the choice of flying, first strike, or trample as three separate '{T}' activated abilities that each remove one of those keywords from target creature until end of turn; choosing the keyword to remove on resolution is not in Effect",
    ),
    faces = &[face!(
        name = "Walking Sponge",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPONGE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!(TapSelf),
            &[Effect::continuous(
                &Filter::Reference,
                Modifier::RemoveKeyword(KeywordSet::FLYING),
                Duration::UntilEndOfTurn,
            )],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        activated!(
            cost!(TapSelf),
            &[Effect::continuous(
                &Filter::Reference,
                Modifier::RemoveKeyword(KeywordSet::FIRST_STRIKE),
                Duration::UntilEndOfTurn,
            )],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        activated!(
            cost!(TapSelf),
            &[Effect::continuous(
                &Filter::Reference,
                Modifier::RemoveKeyword(KeywordSet::TRAMPLE),
                Duration::UntilEndOfTurn,
            )],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ],
);
