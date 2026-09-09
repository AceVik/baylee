//! Wizard Class — {U} — Enchantment — Class
//! Oracle: (Gain the next level as a sorcery to add its ability.)
//! Oracle: You have no maximum hand size.
//! Oracle: {2}{U}: Level 2 — When this Class becomes level 2, draw two cards.
//! Oracle: {4}{U}: Level 3 — Whenever you draw a card, put a +1/+1 counter on target creature you control.
//! Set: AFR #81 — Adventures in the Forgotten Realms | Scryfall ID: d1f629fb-b097-4240-8560-ef47f5678f48 | Oracle ID: 36f68aa3-9955-46f1-bc87-497f16ef5222
// IMPLEMENTED — all three levels: no-max-hand-size (L1), draw-two
// level-up (L2), and the draw-watcher counter grant (L3, GrantTriggered).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::enchantment;

static YOUR_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);
static LEVEL3_FX: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];

card! {
    index: 192,
    oracle_id: "36f68aa3-9955-46f1-bc87-497f16ef5222",
    scryfall_id: "d1f629fb-b097-4240-8560-ef47f5678f48",
    faces: &[face! {
        name: "Wizard Class",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::ENCHANTMENT,
        subtypes: &[enchantment::CLASS],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        // Level 1 (printed).
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::NoMaxHandSize,
            cross_zone: false,
        }),
        // {2}{U}: Level 2 (sorcery speed, requires level 1).
        AbilityDef::ActivatedConditional {
            cost: Cost {
                mana: baylee_core::mana!("{2}{U}"),
                parts: &[],
            },
            effects: &[
                Effect::AddCounter {
                    kind: CounterKind::Level,
                    amount: Amount::Fixed(1),
                },
                Effect::DrawCards {
                    amount: Amount::Fixed(2),
                },
            ],
            target: None,
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: ActivationCondition::CountersOnSelfExactly(CounterKind::Level, 0),
        },
        // {4}{U}: Level 3 (sorcery speed, requires level 2).
        AbilityDef::ActivatedConditional {
            cost: Cost {
                mana: baylee_core::mana!("{4}{U}"),
                parts: &[],
            },
            effects: &[
                Effect::AddCounter {
                    kind: CounterKind::Level,
                    amount: Amount::Fixed(1),
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Text,
                    filter: &Filter::This,
                    modifier: Modifier::GrantTriggered {
                        trigger: Trigger::Draws(PlayerRel::You),
                        effects: LEVEL3_FX,
                        target: Some(TargetSpec::Object(&YOUR_CREATURE)),
                    },
                    duration: Duration::WhileSourceOnBattlefield,
                },
            ],
            target: None,
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: ActivationCondition::CountersOnSelfExactly(CounterKind::Level, 1),
        },
    ],
}
