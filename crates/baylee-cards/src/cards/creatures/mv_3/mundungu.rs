//! Mundungu — {1}{U}{B} — Creature — Human Wizard
//! Oracle: {T}: Counter target spell unless its controller pays {1} and 1 life.
//! Set: VIS #132 — Visions | Scryfall ID: d6e320ca-848b-4743-93f1-ec04ef1ce402 | Oracle ID: 3fb4161d-a5b9-411d-942c-2fdb039e7bde
// PARTIAL: {T} counters target spell unless its controller pays {1}; requiring an additional 1 life payment is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: requiring the spell's controller to pay both {1} and 1 life to prevent countering.
card!(
    index = index::MUNDUNGU,
    oracle_id = "3fb4161d-a5b9-411d-942c-2fdb039e7bde",
    scryfall_id = "d6e320ca-848b-4743-93f1-ec04ef1ce402",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "{T} counters target spell unless its controller pays {1}; requiring an additional 1 life payment is not yet supported"
    ),
    faces = &[face!(
        name = "Mundungu",
        mana_cost = mana!("{1}{U}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(1),
            effect: &Effect::CounterTargetSpell,
        }],
        target = Some(TargetSpec::Spell(&Filter::Any)),
    )],
);
