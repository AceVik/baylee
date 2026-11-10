//! Keeper of Tresserhorn — {5}{B} — Creature — Avatar
//! Oracle: Whenever this creature attacks and isn't blocked, it assigns no combat damage this turn and defending player loses 2 life.
//! Set: ALL #52 — Alliances | Scryfall ID: aaf8b0ec-f81a-488c-850c-098a8a3119e5 | Oracle ID: ce976cb2-2b69-4d00-918c-d4900244ac86
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_TRESSERHORN,
    oracle_id = "ce976cb2-2b69-4d00-918c-d4900244ac86",
    scryfall_id = "aaf8b0ec-f81a-488c-850c-098a8a3119e5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Keeper of Tresserhorn",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[
            Effect::ModifySource {
                modifier: Modifier::AssignsNoCombatDamage,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::DefendingPlayer,
            },
        ],
    )],
);
