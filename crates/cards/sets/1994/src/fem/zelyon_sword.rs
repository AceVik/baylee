//! Zelyon Sword — {3} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {3}, {T}: Target creature gets +2/+0 for as long as this artifact remains tapped.
//! Set: FEM #91 — Fallen Empires | Scryfall ID: 4137160b-5248-4fbd-8ae8-25e9afd8fb5c | Oracle ID: 991038e0-f8d6-42e6-b8a2-3d77aae7a256
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ZELYON_SWORD,
    oracle_id = "991038e0-f8d6-42e6-b8a2-3d77aae7a256",
    scryfall_id = "4137160b-5248-4fbd-8ae8-25e9afd8fb5c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Zelyon Sword",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(2, 0),
                duration: Duration::WhileSourceTapped {
                    control: false,
                    power_limited: false,
                },
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
