//! Aggression — {2}{R} — Enchantment — Aura
//! Oracle: Enchant non-Wall creature
//! Oracle: Enchanted creature has first strike and trample.
//! Oracle: At the beginning of the end step of enchanted creature's controller, destroy that creature if it didn't attack this turn.
//! Set: ICE #169 — Ice Age | Scryfall ID: f3f26060-0c24-496c-b8e2-4dac7ea6166b | Oracle ID: b6c38a2b-2fe8-4080-80ce-cfa12aaf57f0

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

static ENCHANTED_DIDNT_ATTACK: Filter = Filter::And(&[
    Filter::AttachedToBySource,
    Filter::Not(&Filter::AttackedThisTurn),
]);

card!(
    index = index::AGGRESSION,
    oracle_id = "b6c38a2b-2fe8-4080-80ce-cfa12aaf57f0",
    scryfall_id = "f3f26060-0c24-496c-b8e2-4dac7ea6166b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aggression",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&NON_WALL_CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&NON_WALL_CREATURE)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE.union(KeywordSet::TRAMPLE))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::destroy_all(&ENCHANTED_DIDNT_ATTACK)]
        ),
    ],
);
