//! Wizard Class — {U} — Enchantment — Class
//! Oracle: (Gain the next level as a sorcery to add its ability.)
//! Oracle: You have no maximum hand size.
//! Oracle: {2}{U}: Level 2
//! Oracle: When this Class becomes level 2, draw two cards.
//! Oracle: {4}{U}: Level 3
//! Oracle: Whenever you draw a card, put a +1/+1 counter on target creature you control.
//! Set: AFR #81 — Adventures in the Forgotten Realms | Scryfall ID: d1f629fb-b097-4240-8560-ef47f5678f48 | Oracle ID: 36f68aa3-9955-46f1-bc87-497f16ef5222
// IMPLEMENTED — all three levels: no-max-hand-size (L1), draw-two
// level-up (L2), and the draw-watcher counter grant (L3, GrantTriggered).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::enchantment;

card!(
    index = index::WIZARD_CLASS,
    oracle_id = "36f68aa3-9955-46f1-bc87-497f16ef5222",
    scryfall_id = "d1f629fb-b097-4240-8560-ef47f5678f48",
    faces = &[face!(
        name = "Wizard Class",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[enchantment::CLASS],
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[
        // Level 1 (printed).
        static_ability!(Filter::Any, Modifier::NoMaxHandSize),
        // {2}{U}: Level 2 (sorcery speed, requires level 1).
        activated!(
            cost!("{2}{U}"),
            &[Effect::SetClassLevel(2)],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::ClassLevelExactly(1)),
        ),
        triggered!(Trigger::ClassLevelGained(2), &[Effect::draw(2)]),
        // {4}{U}: Level 3 (sorcery speed, requires level 2).
        activated!(
            cost!("{4}{U}"),
            &[Effect::SetClassLevel(3)],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::ClassLevelExactly(2)),
        ),
        static_ability!(
            Filter::This,
            Modifier::GrantTriggered {
                trigger: Trigger::Draws(PlayerRel::You),
                effects: &[Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                }],
                target: Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
            },
            condition = Some(Condition::ClassLevelAtLeast(3)),
        ),
    ],
);
