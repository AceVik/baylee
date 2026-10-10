//! Tawnos's Weaponry — {2} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}: Target creature gets +1/+1 for as long as this artifact remains tapped.
//! Set: ME4 #232 — Masters Edition IV | Scryfall ID: b73fe4b5-142f-4b1f-b48d-7aa65782bdb5 | Oracle ID: f07f98bb-4190-4643-aeb9-c5eaf358c97c
// IMPLEMENTED — the untap-step choice, and the +1/+1 for as long as this
// artifact remains tapped (Duration::WhileSourceTapped).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TAWNOS_S_WEAPONRY,
    oracle_id = "f07f98bb-4190-4643-aeb9-c5eaf358c97c",
    scryfall_id = "b73fe4b5-142f-4b1f-b48d-7aa65782bdb5",
    faces = &[face!(
        name = "Tawnos's Weaponry",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::WhileSourceTapped,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
