//! Spirit Shield — {3} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}: Target creature gets +0/+2 for as long as this artifact remains tapped.
//! Set: FEM #90 — Fallen Empires | Scryfall ID: 213d6e0d-5ec9-441e-a38d-50ce44583e4b | Oracle ID: db53fad4-03b8-47bb-9459-b1eb6cca961c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPIRIT_SHIELD,
    oracle_id = "db53fad4-03b8-47bb-9459-b1eb6cca961c",
    scryfall_id = "213d6e0d-5ec9-441e-a38d-50ce44583e4b",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spirit Shield",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(0, 2),
                duration: Duration::WhileSourceTapped {
                    control: false,
                    power_limited: false,
                },
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
