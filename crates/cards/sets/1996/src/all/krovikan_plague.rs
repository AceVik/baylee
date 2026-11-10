//! Krovikan Plague — {2}{B} — Enchantment — Aura
//! Oracle: Enchant non-Wall creature you control
//! Oracle: When this Aura enters, draw a card at the beginning of the next turn's upkeep.
//! Oracle: Tap enchanted creature: This Aura deals 1 damage to any target. Put a -0/-1 counter on enchanted creature. Activate only if enchanted creature is untapped.
//! Set: ALL #54 — Alliances | Scryfall ID: b258e192-20af-4a45-981f-05181f4cd997 | Oracle ID: 609c3b1c-d9ad-4616-b685-52b85528fde8
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
    Filter::ControlledByYou,
]);

card!(
    index = index::KROVIKAN_PLAGUE,
    oracle_id = "609c3b1c-d9ad-4616-b685-52b85528fde8",
    scryfall_id = "b258e192-20af-4a45-981f-05181f4cd997",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Krovikan Plague",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&YOUR_NON_WALL_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&YOUR_NON_WALL_CREATURE))),
        ),
        triggered!(
            Trigger::ETB,
            &[Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            }],
        ),
        activated!(
            cost!(TapOther(&Filter::AttachedToBySource)),
            &[
                Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::AnyTarget,
                },
                Effect::AddCounterFilter {
                    filter: &Filter::AttachedToBySource,
                    kind: CounterKind::Minus {
                        power: 0,
                        toughness: 1,
                    },
                    amount: Amount::Fixed(1),
                },
            ],
            target = Some(TargetSpec::AnyTarget),
        ),
    ],
);
