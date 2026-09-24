//! Sheltered Valley — (no cost) — Land
//! Oracle: If this land would enter, instead sacrifice each other permanent named Sheltered Valley you control, then put this land onto the battlefield.
//! Oracle: At the beginning of your upkeep, if you control three or fewer lands, you gain 1 life.
//! Oracle: {T}: Add {C}.
//! Set: ALL #142 — Alliances | Scryfall ID: 049d7a08-1605-4ce2-b8c5-634ce2a261e0 | Oracle ID: cd535fa3-6fd8-4227-97fd-3ef07cb0598d
// PARTIAL — the upkeep life and {T}: Add {C} are built; the enter
// replacement has no DSL vocabulary, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHELTERED_VALLEY,
    oracle_id = "cd535fa3-6fd8-4227-97fd-3ef07cb0598d",
    scryfall_id = "049d7a08-1605-4ce2-b8c5-634ce2a261e0",
    faces = &[face!(name = "Sheltered Valley", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the enter replacement that sacrifices the other Sheltered Valleys has \
         no EnterModifier or ReplacementRule"
    ),
    abilities = &[
        // NOT SUPPORTED: "If this land would enter, instead sacrifice each other
        // permanent named Sheltered Valley you control, then put this land onto
        // the battlefield." — no EnterModifier (and no ReplacementRule) replaces a
        // permanent's own arrival with the sacrifice of its namesakes.
        //
        // The `if` is an intervening one (CR 603.4): checked as the upkeep
        // begins and again as the ability resolves.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
            condition = Some(Condition::ControlCountAtMost(&Filter::YOUR_LAND, 3)),
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
