//! Subdue — {G} — Instant
//! Oracle: Prevent all combat damage that would be dealt by target creature this turn. That creature gets +0/+X until end of turn, where X is its mana value.
//! Set: LEG #206 — Legends | Scryfall ID: 123d6097-8021-46cd-a8c3-01013245e347 | Oracle ID: 81bac4b8-277b-415a-9064-a80a68fd7051
// IMPLEMENTED — the target's combat damage is prevented for the turn
// (`Modifier::PreventDamageFromIt`) and it gets +0/+its mana value.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUBDUE,
    oracle_id = "81bac4b8-277b-415a-9064-a80a68fd7051",
    scryfall_id = "123d6097-8021-46cd-a8c3-01013245e347",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Subdue",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn
            ),
            Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::TargetCmc,
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
