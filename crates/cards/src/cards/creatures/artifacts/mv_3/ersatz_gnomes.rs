//! Ersatz Gnomes — {3} — Artifact Creature — Gnome
//! Oracle: {T}: Target spell becomes colorless.
//! Oracle: {T}: Target permanent becomes colorless until end of turn.
//! Set: MIR #301 — Mirage | Scryfall ID: 5a2747ab-00c8-4f59-b9a6-54ff4e99f6c8 | Oracle ID: ea016b84-7f77-42e7-bd14-2e62a1e35f3b
// IMPLEMENTED — {T} makes target spell colorless or target permanent colorless until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ERSATZ_GNOMES,
    oracle_id = "ea016b84-7f77-42e7-bd14-2e62a1e35f3b",
    scryfall_id = "5a2747ab-00c8-4f59-b9a6-54ff4e99f6c8",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ersatz Gnomes",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GNOME],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::continuous(
                &Filter::This,
                Modifier::SetColor(ColorSet::EMPTY),
                Duration::Indefinitely,
            )],
            target = Some(TargetSpec::Spell(&Filter::Any)),
        ),
        activated!(
            Cost::TAP,
            &[Effect::continuous(
                &Filter::This,
                Modifier::SetColor(ColorSet::EMPTY),
                Duration::UntilEndOfTurn,
            )],
            target = Some(TargetSpec::Object(&Filter::Any)),
        ),
    ],
);
