//! Spined Sliver — {R}{G} — Creature — Sliver
//! Oracle: Whenever a Sliver becomes blocked, that Sliver gets +1/+1 until end of turn for each creature blocking it.
//! Set: TPR #213 — Tempest Remastered | Scryfall ID: 5bb97644-be16-4d14-bebd-51894772b77b | Oracle ID: eed9c168-1c78-43c6-a814-9b80054d1373
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPINED_SLIVER,
    oracle_id = "eed9c168-1c78-43c6-a814-9b80054d1373",
    scryfall_id = "5bb97644-be16-4d14-bebd-51894772b77b",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spined Sliver",
        mana_cost = mana!("{R}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::HasSubtype(subtypes::creature::SLIVER),
        Modifier::GrantTriggered {
            trigger: Trigger::BecomesBlocked,
            effects: &[Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::CountOf {
                    filter: &Filter::BlockingSource,
                    zone: ZoneSel::Battlefield,
                },
                toughness: Amount::CountOf {
                    filter: &Filter::BlockingSource,
                    zone: ZoneSel::Battlefield,
                },
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target: None,
        },
    ),],
);
