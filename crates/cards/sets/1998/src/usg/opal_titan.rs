//! Opal Titan — {2}{W}{W} — Enchantment
//! Oracle: When an opponent casts a creature spell, if this permanent is an enchantment, it becomes a 4/4 Giant creature with protection from each of that spell's colors.
//! Set: USG #26 — Urza's Saga | Scryfall ID: 379f1f01-88c6-4cc2-9049-078aa6980582 | Oracle ID: ce06fe83-a8c5-46b7-b30c-b8189d217741
// PARTIAL — granting protection from each of the triggering spell's colors is not in Modifier::ProtectionFrom; becomes a 4/4 Giant creature when an opponent casts a creature spell while it is an enchantment

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);

card!(
    index = index::OPAL_TITAN,
    oracle_id = "ce06fe83-a8c5-46b7-b30c-b8189d217741",
    scryfall_id = "379f1f01-88c6-4cc2-9049-078aa6980582",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "granting protection from each of the triggering spell's colors is not in Modifier::ProtectionFrom; becomes a 4/4 Giant creature when an opponent casts a creature spell while it is an enchantment",
    ),
    faces = &[face!(
        name = "Opal Titan",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: granting protection from each of the triggering spell's colors is not in Modifier::ProtectionFrom; becomes a 4/4 Giant creature when an opponent casts a creature spell while it is an enchantment.
    abilities = &[triggered!(
        Trigger::SpellCast(&OPPONENT_CREATURE_SPELL),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::GIANT,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(4, 4), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
