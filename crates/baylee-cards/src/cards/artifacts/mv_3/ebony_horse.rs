//! Ebony Horse — {3} — Artifact
//! Oracle: {2}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by that creature this turn.
//! Set: ME4 #198 — Masters Edition IV | Scryfall ID: 441ac6f6-c233-4fd4-8005-2ecf0579a538 | Oracle ID: 4eb67ba3-35e3-47c3-820f-62814cf202a7
// IMPLEMENTED — {2}, {T}: untaps target attacking creature you control, then prevents all combat damage to and from it this turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EBONY_HORSE,
    oracle_id = "4eb67ba3-35e3-47c3-820f-62814cf202a7",
    scryfall_id = "441ac6f6-c233-4fd4-8005-2ecf0579a538",
    faces = &[face!(
        name = "Ebony Horse",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[
            Effect::UntapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageToIt,
                Duration::UntilEndOfTurn
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn
            ),
        ],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Attacking,
            Filter::ControlledByYou,
        ])))
    )],
);
