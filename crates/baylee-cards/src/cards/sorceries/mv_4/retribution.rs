//! Retribution — {2}{R}{R} — Sorcery
//! Oracle: Choose two target creatures controlled by the same opponent. That player chooses and sacrifices one of those creatures. Put a -1/-1 counter on the other.
//! Set: ME2 #148 — Masters Edition II | Scryfall ID: 55bacf5b-70d4-45bc-a7e2-44492fa182bc | Oracle ID: 2c7b0034-7eb5-4950-b6dc-8e1d07d26899
// PARTIAL — same-opponent two-target constraint and opponent choosing one of the two targets to sacrifice while the other gets a -1/-1 counter are not in Effect; puts a -1/-1 counter on two target creatures an opponent controls.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RETRIBUTION,
    oracle_id = "2c7b0034-7eb5-4950-b6dc-8e1d07d26899",
    scryfall_id = "55bacf5b-70d4-45bc-a7e2-44492fa182bc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "same-opponent two-target constraint and opponent choosing one of the two targets to sacrifice while the other gets a -1/-1 counter are not in Effect; puts a -1/-1 counter on two target creatures an opponent controls"
    ),
    faces = &[face!(
        name = "Retribution",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::exactly(
            TargetSpec::Object(&Filter::OPPONENT_CREATURE),
            2,
        ))
    )],
);
