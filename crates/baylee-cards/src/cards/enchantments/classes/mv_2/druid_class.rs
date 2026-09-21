//! Druid Class — {1}{G} — Enchantment — Class
//! Oracle: (Gain the next level as a sorcery to add its ability.)
//! Oracle: Landfall — Whenever a land you control enters, you gain 1 life.
//! Oracle: {2}{G}: Level 2
//! Oracle: You may play an additional land on each of your turns.
//! Oracle: {4}{G}: Level 3
//! Oracle: When this Class becomes level 3, target land you control becomes a creature with haste and "This creature's power and toughness are each equal to the number of lands you control." It's still a land.
//! Set: AFR #180 — Adventures in the Forgotten Realms | Scryfall ID: 09278e95-eaae-4cd4-a0d8-a2d15b0abb58 | Oracle ID: dcbcbf42-4654-487a-acad-21f2606d229b
// PARTIAL — the Landfall trigger and both level-up abilities are built; the
// two payoffs a level grants are not (see the // NOT SUPPORTED: lines below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRUID_CLASS,
    oracle_id = "dcbcbf42-4654-487a-acad-21f2606d229b",
    scryfall_id = "09278e95-eaae-4cd4-a0d8-a2d15b0abb58",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Druid Class",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::CLASS],
    ),],
    coverage = Coverage::Partial(
        "level 2's \"you may play an additional land\" is a StaticAbility gated on the Class's level, which the DSL has no level gate for, and nothing triggers on \"becomes level 3\", so level 3's animation has no ability to hang on"
    ),
    abilities = &[
        // Landfall — "whenever a land you control enters, you gain 1 life".
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::gain_life(1)]
        ),
        // "Gain the next level as a sorcery": a level counter is the level,
        // so the second level is reachable from level 1 and nowhere else.
        activated!(
            cost!("{2}{G}"),
            &[Effect::AddCounter {
                kind: CounterKind::Level,
                amount: Amount::Fixed(1),
            }],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::CountersOnSelfExactly(CounterKind::Level, 0)),
        ),
        // NOT SUPPORTED: "You may play an additional land on each of your
        // turns." — level 2's whole ability. `Modifier::ExtraLandDrops(1)` is
        // the sentence, but a `StaticAbility` carries no `Condition`, so
        // written here it would hand out the extra land drop from level 1 on;
        // the only counter-gated modifiers are AddType/AddKeywordIfCountersAtLeast.
        activated!(
            cost!("{4}{G}"),
            &[Effect::AddCounter {
                kind: CounterKind::Level,
                amount: Amount::Fixed(1),
            }],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::CountersOnSelfExactly(CounterKind::Level, 1)),
        ),
        // NOT SUPPORTED: "When this Class becomes level 3, target land you
        // control becomes a creature with haste and \"This creature's power and
        // toughness are each equal to the number of lands you control.\" It's
        // still a land." — no `Trigger` fires when a level counter is added
        // (`SagaChapter` counts lore counters on a Saga), and the animation
        // wants an `AddType`, a `Modifier::AddKeyword(HASTE)` and a P/T
        // counted off `Filter::YOUR_LAND`, which is not one effect.
    ],
);
