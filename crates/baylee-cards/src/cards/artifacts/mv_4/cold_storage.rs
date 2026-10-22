//! Cold Storage — {4} — Artifact
//! Oracle: {3}: Exile target creature you control.
//! Oracle: Sacrifice this artifact: Return each creature card exiled with this artifact to the battlefield under your control.
//! Set: TMP #280 — Tempest | Scryfall ID: b26e28d4-50e1-41db-984d-c55781295012 | Oracle ID: e23bf6bd-bc97-425d-902a-c6b606e82b65
// PARTIAL — {3} exiles target creature you control linked to Cold Storage, and sacrificing Cold Storage returns each creature card exiled with it to the battlefield under its owner's control; Effect::ReturnLinkedToBattlefield returns linked cards under their owner's control rather than your control when you exiled a creature you controlled but did not own.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COLD_STORAGE,
    oracle_id = "e23bf6bd-bc97-425d-902a-c6b606e82b65",
    scryfall_id = "b26e28d4-50e1-41db-984d-c55781295012",
    coverage = Coverage::Partial(
        "{3} exiles target creature you control linked to Cold Storage, and sacrificing Cold Storage returns each creature card exiled with it to the battlefield under its owner's control; Effect::ReturnLinkedToBattlefield returns linked cards under their owner's control rather than your control when you exiled a creature you controlled but did not own."
    ),
    faces = &[face!(
        name = "Cold Storage",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{3}"),
            &[Effect::exile_linked(TargetSpec::Object(
                &Filter::YOUR_CREATURE
            ))],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        ),
        activated!(cost!(SacrificeSelf), &[Effect::ReturnLinkedToBattlefield]),
    ],
);
