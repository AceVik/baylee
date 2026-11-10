//! Corpse Dance — {2}{B} — Instant
//! Oracle: Buyback {2} (You may pay an additional {2} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Return the top creature card of your graveyard to the battlefield. That creature gains haste until end of turn. Exile it at the beginning of the next end step.
//! Set: TPR #89 — Tempest Remastered | Scryfall ID: aaed9b00-f6a1-4023-b9b9-22d904fe5271 | Oracle ID: 59b02ca7-e7bd-4820-ac32-87ea1a8c64c0
// IMPLEMENTED — the top creature card of the pile, named and not targeted
// (CR 404.2, 115.1), back with haste and exiled at the next end step; buyback.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORPSE_DANCE,
    oracle_id = "59b02ca7-e7bd-4820-ac32-87ea1a8c64c0",
    scryfall_id = "aaed9b00-f6a1-4023-b9b9-22d904fe5271",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Corpse Dance",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
        buyback = Some(cost!("{2}")),
    ),],
    abilities = &[spell!(&[
        Effect::TopOfYourGraveyard {
            filter: &Filter::CREATURE,
        },
        Effect::reanimate(TargetSpec::Object(&Filter::Any)),
        Effect::ModifyTargets {
            modifier: Modifier::AddKeyword(KeywordSet::HASTE),
            duration: Duration::UntilEndOfTurn,
        },
        Effect::AtNextEndStep {
            effects: &[Effect::exile(TargetSpec::EventObject)],
        },
    ])],
);
