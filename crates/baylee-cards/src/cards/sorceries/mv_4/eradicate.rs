//! Eradicate — {2}{B}{B} — Sorcery
//! Oracle: Exile target nonblack creature. Search its controller's graveyard, hand, and library for all cards with the same name as that creature and exile them. Then that player shuffles.
//! Set: BOK #65 — Betrayers of Kamigawa | Scryfall ID: 02d6e0e5-798c-4791-85fe-24d0838df8f4 | Oracle ID: 782ff740-ff43-4c07-af0b-433cb9770661
// PARTIAL: exiles target nonblack creature; searching its controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect.

use baylee_cards_dsl::prelude::*;

static NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::ERADICATE,
    oracle_id = "782ff740-ff43-4c07-af0b-433cb9770661",
    scryfall_id = "02d6e0e5-798c-4791-85fe-24d0838df8f4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "exiles target nonblack creature; searching its controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect",
    ),
    faces = &[face!(
        name = "Eradicate",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(&NONBLACK_CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&NONBLACK_CREATURE))),
    )],
);
