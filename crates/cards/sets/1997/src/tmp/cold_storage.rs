//! Cold Storage — {4} — Artifact
//! Oracle: {3}: Exile target creature you control.
//! Oracle: Sacrifice this artifact: Return each creature card exiled with this artifact to the battlefield under your control.
//! Set: TMP #280 — Tempest | Scryfall ID: b26e28d4-50e1-41db-984d-c55781295012 | Oracle ID: e23bf6bd-bc97-425d-902a-c6b606e82b65
// PARTIAL — Only the {3} exile-target-creature-you-control ability is supported; linked returns under the activator's control are unsupported, and owner control is a different recipient.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COLD_STORAGE,
    oracle_id = "e23bf6bd-bc97-425d-902a-c6b606e82b65",
    scryfall_id = "b26e28d4-50e1-41db-984d-c55781295012",
    coverage = Coverage::Partial(
        "Only the {3} exile-target-creature-you-control ability is supported; linked returns under the activator's control are unsupported, and owner control is a different recipient."
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
        // NOT SUPPORTED: sacrifice to return linked creatures under your control, not owner control.
    ],
);
