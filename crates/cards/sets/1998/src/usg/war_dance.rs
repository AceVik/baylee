//! War Dance — {G} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: Sacrifice this enchantment: Target creature gets +X/+X until end of turn, where X is the number of verse counters on this enchantment.
//! Set: USG #282 — Urza's Saga | Scryfall ID: 392b4acf-4548-47dd-be58-bdf4ca4ece7e | Oracle ID: ecfc81df-1271-47ca-9184-49bf5798e195
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAR_DANCE,
    oracle_id = "ecfc81df-1271-47ca-9184-49bf5798e195",
    scryfall_id = "392b4acf-4548-47dd-be58-bdf4ca4ece7e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "War Dance",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: counters::VERSE,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::PumpTarget {
                power: Amount::CountersOnSource(counters::VERSE),
                toughness: Amount::CountersOnSource(counters::VERSE),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
