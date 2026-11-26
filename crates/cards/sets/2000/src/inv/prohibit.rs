//! Prohibit — {1}{U} — Instant
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: Counter target spell if its mana value is 2 or less. If this spell was kicked, counter that spell if its mana value is 4 or less instead.
//! Set: MH1 #64 — Modern Horizons | Scryfall ID: 0ae544bf-7229-4b82-99ad-32c3af36e30f | Oracle ID: d95f1797-56f9-41be-a5fe-8398961f4b8b
// IMPLEMENTED — kicker {2} expands the mana value threshold from 2 to 4 on resolution.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PROHIBIT,
    oracle_id = "d95f1797-56f9-41be-a5fe-8398961f4b8b",
    scryfall_id = "0ae544bf-7229-4b82-99ad-32c3af36e30f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Prohibit",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
        additional_costs = &[cost!("{2}")],
    ),],
    abilities = &[spell!(
        &[Effect::IfKicked {
            then: &[Effect::IfTargetMatches {
                filter: &Filter::CmcAtMost(4),
                then: &[Effect::CounterTargetSpell],
            }],
            otherwise: &[Effect::IfTargetMatches {
                filter: &Filter::CmcAtMost(2),
                then: &[Effect::CounterTargetSpell],
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    ),],
);
