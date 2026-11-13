//! Quash — {2}{U}{U} — Instant
//! Oracle: Counter target instant or sorcery spell. Search its controller's graveyard, hand, and library for all cards with the same name as that spell and exile them. Then that player shuffles.
//! Set: BOK #47 — Betrayers of Kamigawa | Scryfall ID: 48ca8c31-a9ea-4388-b257-951c1c68b86d | Oracle ID: 255131a2-616c-4c8d-a36a-14e5b5ab345e
// NOT SUPPORTED: searching the spell's controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::QUASH,
    oracle_id = "255131a2-616c-4c8d-a36a-14e5b5ab345e",
    scryfall_id = "48ca8c31-a9ea-4388-b257-951c1c68b86d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "counters target instant or sorcery spell; searching its controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect"
    ),
    faces = &[face!(
        name = "Quash",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(
            &Filter::INSTANT_OR_SORCERY
        ))),
    )],
);
