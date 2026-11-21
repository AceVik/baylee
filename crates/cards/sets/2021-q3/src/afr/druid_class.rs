//! Druid Class — {1}{G} — Enchantment — Class
//! Oracle: (Gain the next level as a sorcery to add its ability.)
//! Oracle: Landfall — Whenever a land you control enters, you gain 1 life.
//! Oracle: {2}{G}: Level 2
//! Oracle: You may play an additional land on each of your turns.
//! Oracle: {4}{G}: Level 3
//! Oracle: When this Class becomes level 3, target land you control becomes a creature with haste and "This creature's power and toughness are each equal to the number of lands you control." It's still a land.
//! Set: AFR #180 — Adventures in the Forgotten Realms | Scryfall ID: 09278e95-eaae-4cd4-a0d8-a2d15b0abb58 | Oracle ID: dcbcbf42-4654-487a-acad-21f2606d229b
// IMPLEMENTED — Landfall, both level bars, level 2's extra land drop while
// the Class is level 2 or greater (CR 717.2a) and level 3's trigger.

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
    coverage = Coverage::Implemented,
    abilities = &[
        // Landfall — "whenever a land you control enters, you gain 1 life".
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::gain_life(1)]
        ),
        // {2}{G}: Level 2 — "activate only if this Class is level 1 and
        // only as a sorcery" (CR 717.2a).
        activated!(
            cost!("{2}{G}"),
            &[Effect::SetClassLevel(2)],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::ClassLevelExactly(1)),
        ),
        // Level 2: "You may play an additional land on each of your turns",
        // as long as the Class is level 2 or greater.
        static_ability!(
            Filter::Any,
            Modifier::ExtraLandDrops(1),
            condition = Some(Condition::ClassLevelAtLeast(2))
        ),
        // {4}{G}: Level 3.
        activated!(
            cost!("{4}{G}"),
            &[Effect::SetClassLevel(3)],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::ClassLevelExactly(2)),
        ),
        // Level 3: "When this Class becomes level 3, target land you control
        // becomes a creature with haste and 'This creature's power and
        // toughness are each equal to the number of lands you control.' It's
        // still a land." No duration is printed, so it lasts.
        triggered!(
            Trigger::ClassLevelGained(3),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::HASTE),
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPTToCount(PtCount::YouControl(&Filter::YOUR_LAND)),
                    Duration::Indefinitely,
                ),
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND)))
        ),
    ],
);
