//! Ashnod's Battle Gear — {2} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}: Target creature you control gets +2/-2 for as long as this artifact remains tapped.
//! Set: 4ED #296 — Fourth Edition | Scryfall ID: 0bc11285-0891-4cc3-a056-b698911166c7 | Oracle ID: b5a390fd-2864-4481-84b4-41e8fac91a80

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASHNOD_S_BATTLE_GEAR,
    oracle_id = "b5a390fd-2864-4481-84b4-41e8fac91a80",
    scryfall_id = "0bc11285-0891-4cc3-a056-b698911166c7",
    faces = &[face!(
        name = "Ashnod's Battle Gear",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(2, -2),
                duration: Duration::WhileSourceTapped {
                    control: false,
                    power_limited: false
                }
            }],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
        ),
    ],
);
