//! Cursed Mirror — {2}{R} — Artifact
//! Oracle: {T}: Add {R}.
//! Oracle: As this artifact enters, you may have it become a copy of any creature on the battlefield until end of turn, except it has haste.
//! Set: SOC #242 — Secrets of Strixhaven Commander | Scryfall ID: 077392b3-6b06-46c8-8737-51e85f690448 | Oracle ID: 4d67e2a7-4aa7-44cc-853b-500d7aac046d
// IMPLEMENTED — {R} mana + until-EOT clone with haste (layer-1 copy
// effect with UntilEndOfTurn duration).

use baylee_cards_dsl::prelude::*;

card! {
    index: 28,
    oracle_id: "4d67e2a7-4aa7-44cc-853b-500d7aac046d",
    scryfall_id: "077392b3-6b06-46c8-8737-51e85f690448",
    faces: &[face! {
        name: "Cursed Mirror",
        mana_cost: baylee_core::mana!("{2}{R}"),
        types: TypeSet::ARTIFACT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        AbilityDef::CopyOnEnterUntilEot {
            target: TargetSpec::Object(&Filter::CREATURE),
            mods: &[CopyMod::AddKeyword(KeywordSet::HASTE)],
        },
    ],
}
