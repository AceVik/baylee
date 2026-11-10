//! Pact of Negation — {0} — Instant
//! Oracle: Counter target spell.
//! Oracle: At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.
//! Set: TSR #77 — Time Spiral Remastered | Scryfall ID: 1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8 | Oracle ID: f3e213a4-ba5a-468a-93b3-c0a34e1bd725
// IMPLEMENTED — zero-cost counter + delayed pay-or-lose upkeep. The second
// ability is that delayed one (CR 603.7): the spell creates it, and being
// listed is what gives it a sentence on the stack.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PACT_OF_NEGATION,
    oracle_id = "f3e213a4-ba5a-468a-93b3-c0a34e1bd725",
    scryfall_id = "1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8",
    faces = &[face!(
        name = "Pact of Negation",
        // Printed `{0}`; `face!`'s default is *no* cost, which is a
        // different card (CR 202.1b). See `casting::has_a_printed_cost`.
        mana_cost = mana!("{0}"),
        types = TypeSet::INSTANT,
        color_indicator = ColorSet::from_slice(&[Color::Blue]),
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(
            &[
                Effect::CounterTargetSpell,
                Effect::AtNextUpkeepPrinted { ability: 1 },
            ],
            targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PayManaOrLose {
                cost: mana!("{3}{U}{U}"),
            }],
        ),
    ],
);
