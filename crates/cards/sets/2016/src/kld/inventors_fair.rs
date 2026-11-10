//! Inventors' Fair — (no cost) — Legendary Land
//! Oracle: At the beginning of your upkeep, if you control three or more artifacts, you gain 1 life.
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}, Sacrifice Inventors' Fair: Search your library for an artifact card, reveal it, put it into your hand, then shuffle. Activate only if you control three or more artifacts.
//! Set: KLD #247 — Kaladesh | Scryfall ID: 275471e3-ded1-40ac-91ef-369dce5764d9 | Oracle ID: 91d4a5fe-fd6d-4b14-a63f-61b4d0ecd9c4
// IMPLEMENTED — the upkeep lifegain and the {4}, {T}, Sacrifice artifact tutor,
// each gated on controlling three or more artifacts; {T}: Add {C}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INVENTORS_FAIR,
    oracle_id = "91d4a5fe-fd6d-4b14-a63f-61b4d0ecd9c4",
    scryfall_id = "275471e3-ded1-40ac-91ef-369dce5764d9",
    faces = &[face!(
        name = "Inventors' Fair",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::gain_life(1)],
            condition = Some(Condition::ControlCount(&Filter::ARTIFACT, 3))
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::ARTIFACT,
                finds: &[Find::HAND],
                optional: false,
            }],
            condition = Some(Condition::ControlCount(&Filter::ARTIFACT, 3))
        ),
    ],
);
