//! Abjure — {U} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a blue permanent.
//! Oracle: Counter target spell.
//! Set: WTH #31 — Weatherlight | Scryfall ID: fbad9449-d09c-4fd0-b2ad-2aa3a29e03bf | Oracle ID: dd97e457-07eb-4918-b780-ae7ff42a37e7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static BLUE_PERMANENT: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

card!(
    index = index::ABJURE,
    oracle_id = "dd97e457-07eb-4918-b780-ae7ff42a37e7",
    scryfall_id = "fbad9449-d09c-4fd0-b2ad-2aa3a29e03bf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Abjure",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&BLUE_PERMANENT)],
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
