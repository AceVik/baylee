//! Thaumatic Compass // Spires of Orazca — {2} — Artifact // Land
//! Oracle: {3}, {T}: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
//! Oracle: At the beginning of your end step, if you control seven or more lands, transform this artifact.
//! Oracle: (Transforms from Thaumatic Compass.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Untap target attacking creature an opponent controls and remove it from combat.
//! Set: XLN #249 — Ixalan | Scryfall ID: 392af78e-34d5-4b1b-8b29-0e702271e4d7 | Oracle ID: f9085e55-2833-41b7-9100-a35dc04dee93
//! Face: Thaumatic Compass — {2} — Artifact
//! Face: Spires of Orazca —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THAUMATIC_COMPASS,
    oracle_id = "f9085e55-2833-41b7-9100-a35dc04dee93",
    scryfall_id = "392af78e-34d5-4b1b-8b29-0e702271e4d7",
    faces = &[
        face!(
            name = "Thaumatic Compass",
            mana_cost = mana!("{2}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(
            name = "Spires of Orazca",
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = &[
                mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
                activated!(
                    Cost::TAP,
                    &[
                        Effect::UntapTarget,
                        Effect::RemoveTargetFromCombat { unblock: false }
                    ],
                    target = Some(TargetSpec::Object(&Filter::And(&[
                        Filter::ATTACKING_CREATURE,
                        Filter::ControlledByOpponent,
                    ]))),
                ),
            ],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::HAND],
                optional: false,
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::TransformSource],
            condition = Some(Condition::ControlCount(&Filter::LAND, 7)),
        ),
    ],
);
