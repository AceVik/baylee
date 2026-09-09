//! Machine God's Effigy — {4} — Artifact
//! Oracle: You may have this artifact enter as a copy of any creature on the battlefield, except it's an artifact and it has "{T}: Add {U}." (It's not a creature.)
//! Oracle: {T}: Add {U}.
//! Set: BRC #16 — The Brothers' War Commander | Scryfall ID: 637f69c2-ba24-42d1-9345-8ebdb04b6904 | Oracle ID: 64ebdd6f-acde-4aab-a86b-2798bad5f70c
// IMPLEMENTED — clone as noncreature artifact + blue mana tap.

use baylee_cards_dsl::prelude::*;

card! {
    index: 89,
    oracle_id: "64ebdd6f-acde-4aab-a86b-2798bad5f70c",
    scryfall_id: "637f69c2-ba24-42d1-9345-8ebdb04b6904",
    faces: &[face! {
        name: "Machine God's Effigy",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&Filter::CREATURE),
            mods: &[
                CopyMod::AddType(TypeSet::ARTIFACT),
                CopyMod::RemoveType(TypeSet::CREATURE),
            ],
        },
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
    ],
}
