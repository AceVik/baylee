//! Power Depot — (no cost) — Artifact Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast artifact spells or activate abilities of artifacts.
//! Oracle: Modular 1
//! Set: MH2 #251 — Modern Horizons 2 | Scryfall ID: 032eba04-0a63-4823-85e1-64861330a8d7 | Oracle ID: 64687880-03f9-4f38-985b-1027c797e33f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::POWER_DEPOT,
    oracle_id = "64687880-03f9-4f38-985b-1027c797e33f",
    scryfall_id = "032eba04-0a63-4823-85e1-64861330a8d7",
    faces = &[face!(
        name = "Power Depot",
        types = TypeSet::ARTIFACT.union(TypeSet::LAND),
        enter_modifiers = &[
            EnterModifier::Tapped,
            EnterModifier::WithCounters {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
        ],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted_for(
            &Filter::ARTIFACT,
            SpendRider::None,
            ManaUse::SpellsOrAbilities
        ),]),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::CountersOnSource(CounterKind::P1P1)
                }]
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
                Filter::ARTIFACT,
                Filter::CREATURE
            ]))))
        ),
    ],
);
