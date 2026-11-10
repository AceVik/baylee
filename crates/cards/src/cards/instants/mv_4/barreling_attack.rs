//! Barreling Attack — {2}{R}{R} — Instant
//! Oracle: Target creature gains trample until end of turn. When that creature becomes blocked this turn, it gets +1/+1 until end of turn for each creature blocking it.
//! Set: MIR #157 — Mirage | Scryfall ID: a3a2955e-e714-419e-9e3d-7ae3d7fae041 | Oracle ID: f44eab45-8ff9-40eb-9919-28fe5203fc48
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BARRELING_ATTACK,
    oracle_id = "f44eab45-8ff9-40eb-9919-28fe5203fc48",
    scryfall_id = "a3a2955e-e714-419e-9e3d-7ae3d7fae041",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Barreling Attack",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::TRAMPLE,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::GrantTriggered {
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
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
