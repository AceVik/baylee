//! Cryptic Command — {1}{U}{U}{U} — Instant
//! Oracle: Choose two —
//! Oracle: • Counter target spell.
//! Oracle: • Return target permanent to its owner's hand.
//! Oracle: • Tap all creatures your opponents control.
//! Oracle: • Draw a card.
//! Set: IMA #48 — Iconic Masters | Scryfall ID: 30f6fca9-003b-4f6b-9d6e-1e88adda4155 | Oracle ID: a3e51a35-09df-4189-b131-08a21e6a557d

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRYPTIC_COMMAND,
    oracle_id = "a3e51a35-09df-4189-b131-08a21e6a557d",
    scryfall_id = "30f6fca9-003b-4f6b-9d6e-1e88adda4155",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cryptic Command",
        mana_cost = mana!("{1}{U}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    // "Choose two": exactly two different modes (CR 700.2d), a mode with
    // no legal target not among them (CR 700.2a), and targets asked only
    // for the modes chosen (CR 700.2c). Chosen together, the counter takes
    // the spell's first targets and the bounce its second. The tap names
    // no target, so hexproof does not stop it.
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::TWO,
        modes: &[
            mode!(
                &[Effect::CounterTargetSpell],
                targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
            ),
            mode!(
                &[Effect::bounce(TargetSpec::Object(&Filter::Any))],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
            ),
            mode!(&[Effect::TapAll {
                filter: &Filter::OPPONENT_CREATURE,
            }]),
            mode!(&[Effect::draw(1)]),
        ],
    }],
);
