//! Scour — {2}{W}{W} — Instant
//! Oracle: Exile target enchantment. Search its controller's graveyard, hand, and library for all cards with the same name as that enchantment and exile them. Then that player shuffles.
//! Set: BOK #20 — Betrayers of Kamigawa | Scryfall ID: a9cac762-7482-471f-b4de-e0cb6da5451c | Oracle ID: f18622da-81f3-4854-a1bb-8d658751d8db
// NOT SUPPORTED: searching the enchantment's controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCOUR,
    oracle_id = "f18622da-81f3-4854-a1bb-8d658751d8db",
    scryfall_id = "a9cac762-7482-471f-b4de-e0cb6da5451c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "exiles target enchantment; searching its controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect"
    ),
    faces = &[face!(
        name = "Scour",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(&Filter::ENCHANTMENT))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT))),
    )],
);
