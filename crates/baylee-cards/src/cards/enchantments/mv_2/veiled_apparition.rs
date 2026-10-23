//! Veiled Apparition — {1}{U} — Enchantment
//! Oracle: When an opponent casts a spell, if this permanent is an enchantment, it becomes a 3/3 Illusion creature with flying and "At the beginning of your upkeep, sacrifice this creature unless you pay {1}{U}."
//! Set: USG #107 — Urza's Saga | Scryfall ID: 8f9ea09f-5017-423c-84ad-c332329992b3 | Oracle ID: 4187fd6d-5660-4ea7-a9dd-402487ec8721
// IMPLEMENTED — animation grants the printed colored upkeep payment.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VEILED_APPARITION,
    oracle_id = "4187fd6d-5660-4ea7-a9dd-402487ec8721",
    scryfall_id = "8f9ea09f-5017-423c-84ad-c332329992b3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Veiled Apparition",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::ControlledByOpponent),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::ILLUSION,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::Indefinitely),
            Effect::continuous(
                &Filter::This,
                Modifier::GrantTriggered {
                    trigger: Trigger::StepBegin {
                        step: StepKind::Upkeep,
                        whose: PlayerRel::You
                    },
                    effects: &[Effect::PlayerMayPayManaOr {
                        player: PlayerRel::You,
                        cost: mana!("{1}{U}"),
                        effect: &Effect::SacrificeSelf,
                    }],
                    target: None,
                },
                Duration::Indefinitely,
            ),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
