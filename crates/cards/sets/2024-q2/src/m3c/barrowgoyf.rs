//! Barrowgoyf — {2}{B} — Creature — Lhurgoyf
//! Oracle: Deathtouch, lifelink
//! Oracle: Barrowgoyf's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1.
//! Oracle: Whenever this creature deals combat damage to a player, you may mill that many cards. If you do, you may put a creature card from among them into your hand.
//! Set: PLST #M3C-102 — The List | Scryfall ID: fcf0c46b-a37c-4475-99a1-bff6bc0cf2db | Oracle ID: 74c0164c-130f-4572-9066-626c77f6e2ff
// IMPLEMENTED — graveyard-card-type CDA, deathtouch/lifelink, and the
// optional combat-damage mill with an optional creature-card return.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARROWGOYF,
    oracle_id = "74c0164c-130f-4572-9066-626c77f6e2ff",
    scryfall_id = "fcf0c46b-a37c-4475-99a1-bff6bc0cf2db",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Barrowgoyf",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LHURGOYF],
        power = Some(0),
        toughness = Some(1),
    ),],
    keywords = KeywordSet::DEATHTOUCH.union(KeywordSet::LIFELINK),
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CharacteristicPT {
                count: PtCount::CardTypesInAllGraveyards,
                toughness_plus: 1,
            }
        ),
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[Effect::MayDo {
                effects: &[Effect::MillMayTakeOne {
                    amount: Amount::EventAmount,
                    filter: &Filter::CREATURE,
                }],
            }]
        ),
    ],
);
