//! Daze — {1}{U} — Instant
//! Oracle: You may return an Island you control to its owner's hand rather than pay this spell's mana cost.
//! Oracle: Counter target spell unless its controller pays {1}.
//! Set: EMA #44 — Eternal Masters | Scryfall ID: f05e9a3e-8a35-4687-85cb-e31b3927a5e2 | Oracle ID: 70486bee-6ee7-41ea-b834-8caf4699302b
// NOT SUPPORTED: counters target spell unless its controller pays {1} for {1}{U}; the alternative cost of returning an Island you control to its owner's hand is not yet modeled
// PARTIAL: counters target spell unless its controller pays {1} for {1}{U}; the alternative cost of returning an Island you control to its owner's hand is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAZE,
    oracle_id = "70486bee-6ee7-41ea-b834-8caf4699302b",
    scryfall_id = "f05e9a3e-8a35-4687-85cb-e31b3927a5e2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "counters target spell unless its controller pays {1} for {1}{U}; the alternative cost of returning an Island you control to its owner's hand is not yet modeled",
    ),
    faces = &[face!(
        name = "Daze",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(1),
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    ),],
);
