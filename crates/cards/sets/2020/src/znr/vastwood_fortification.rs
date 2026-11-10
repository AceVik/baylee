//! Vastwood Fortification // Vastwood Thicket — {G} — Instant // Land
//! Oracle: Put a +1/+1 counter on target creature.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #216 — Zendikar Rising | Scryfall ID: 3a7fd24e-84d8-405d-86e4-0571a9e23cc2 | Oracle ID: ce148a0c-6c63-49d5-a156-99efae4e367a
//! Face: Vastwood Fortification — {G} — Instant
//! Face: Vastwood Thicket —  — Land
// IMPLEMENTED — the spell face puts a +1/+1 counter on target creature; the
// land face enters tapped and taps for {G}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VASTWOOD_FORTIFICATION,
    oracle_id = "ce148a0c-6c63-49d5-a156-99efae4e367a",
    scryfall_id = "3a7fd24e-84d8-405d-86e4-0571a9e23cc2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Vastwood Fortification",
            mana_cost = mana!("{G}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Vastwood Thicket",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
