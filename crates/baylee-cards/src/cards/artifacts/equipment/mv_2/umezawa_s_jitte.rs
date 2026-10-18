//! Umezawa's Jitte — {2} — Legendary Artifact — Equipment
//! Oracle: Whenever equipped creature deals combat damage, put two charge counters on Umezawa's Jitte.
//! Oracle: Remove a charge counter from Umezawa's Jitte: Choose one —
//! Oracle: • Equipped creature gets +2/+2 until end of turn.
//! Oracle: • Target creature gets -1/-1 until end of turn.
//! Oracle: • You gain 2 life.
//! Oracle: Equip {2}
//! Set: BOK #163 — Betrayers of Kamigawa | Scryfall ID: 3b6e5956-f795-451b-bb24-56462d1ced27 | Oracle ID: 1da10d5c-36a8-473f-a5d4-782ad61d8057
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UMEZAWA_S_JITTE,
    oracle_id = "1da10d5c-36a8-473f-a5d4-782ad61d8057",
    scryfall_id = "3b6e5956-f795-451b-bb24-56462d1ced27",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Umezawa's Jitte",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        triggered!(
            Trigger::DealsCombatDamage(&Filter::AttachedToBySource),
            &[Effect::AddCounterSource {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(2),
            }]
        ),
        modal_activated!(
            cost!(RemoveCounterSelf {
                kind: CounterKind::Charge,
                n: 1
            }),
            &[
                mode!(&[Effect::PumpFilter {
                    filter: &Filter::AttachedToBySource,
                    controlled_by: None,
                    power: Amount::Fixed(2),
                    toughness: Amount::Fixed(2),
                    keywords: KeywordSet::EMPTY,
                    duration: Duration::UntilEndOfTurn,
                }]),
                mode!(
                    &[Effect::PumpTarget {
                        power: Amount::NegXFixed(1),
                        toughness: Amount::NegXFixed(1),
                        keywords: KeywordSet::EMPTY,
                        duration: Duration::UntilEndOfTurn,
                    }],
                    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
                ),
                mode!(&[Effect::gain_life(2)]),
            ]
        ),
        equip!("{2}"),
    ],
);
