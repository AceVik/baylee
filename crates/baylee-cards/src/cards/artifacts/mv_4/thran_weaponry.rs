//! Thran Weaponry — {4} — Artifact
//! Oracle: Echo {4} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}: All creatures get +2/+2 for as long as this artifact remains tapped.
//! Set: ULG #135 — Urza's Legacy | Scryfall ID: 60f005ae-fca1-4a48-84a3-4b217ac879ce | Oracle ID: 1af303ef-dd76-4684-97a1-355b7eee53e4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRAN_WEAPONRY,
    oracle_id = "1af303ef-dd76-4684-97a1-355b7eee53e4",
    scryfall_id = "60f005ae-fca1-4a48-84a3-4b217ac879ce",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thran Weaponry",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        AbilityDef::Echo { cost: mana!("{4}") },
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::PumpFilter {
                filter: &Filter::CREATURE,
                controlled_by: None,
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::WhileSourceTapped {
                    control: false,
                    power_limited: false,
                },
            }],
        ),
    ],
);
