//! Arcane Denial — {1}{U} — Instant
//! Oracle: Counter target spell. Its controller may draw up to two cards at the beginning of the next turn's upkeep.
//! Oracle: You draw a card at the beginning of the next turn's upkeep.
//! Set: MSC #147 — Marvel Super Heroes Commander | Scryfall ID: 5eb843ef-b223-40d0-a461-54b16d188d0e | Oracle ID: ab1cc360-b9de-48d9-9983-4dfe4a7d2a37
// NOT SUPPORTED: letting the countered spell's controller draw up to two cards at the beginning of the next turn's upkeep is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARCANE_DENIAL,
    oracle_id = "ab1cc360-b9de-48d9-9983-4dfe4a7d2a37",
    scryfall_id = "5eb843ef-b223-40d0-a461-54b16d188d0e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "counters target spell and you draw a card at the beginning of the next turn's upkeep; letting the countered spell's controller draw up to two cards at the beginning of the next turn's upkeep is not supported",
    ),
    faces = &[face!(
        name = "Arcane Denial",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::CounterTargetSpell,
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
