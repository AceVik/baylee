//! Bast, Panther Goddess — {2}{G}{W} — Legendary Creature — Cat God
//! Oracle: Reach, trample, indestructible
//! Oracle: Bast can't attack or block unless you control three or more creatures.
//! Oracle: Whenever you attack, target attacking creature gets +X/+X until end of turn, where X is the number of creatures you control.
//! Set: MSC #75 — Marvel Super Heroes Commander | Scryfall ID: c53a05f1-8e95-4a39-b2a3-a21d3d19b5b3 | Oracle ID: e3233686-7703-460c-80ee-8cd37b3014ff
// IMPLEMENTED — keywords, attack/block restriction unless controlling 3+ creatures, and attack trigger pumping target attacking creature +X/+X.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURES_YOU_CONTROL_COUNT: Amount = Amount::CountOf {
    filter: &Filter::YOUR_CREATURE,
    zone: ZoneSel::Battlefield,
};

card!(
    index = index::BAST_PANTHER_GODDESS,
    oracle_id = "e3233686-7703-460c-80ee-8cd37b3014ff",
    scryfall_id = "c53a05f1-8e95-4a39-b2a3-a21d3d19b5b3",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::REACH
        .union(KeywordSet::TRAMPLE)
        .union(KeywordSet::INDESTRUCTIBLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bast, Panther Goddess",
        mana_cost = mana!("{2}{G}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::CAT, subtypes::creature::GOD],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)),
            condition = Some(Condition::Not(&Condition::ControlCount(
                &Filter::CREATURE,
                3
            ))),
        ),
        triggered!(
            Trigger::OneOrMoreAttack(&Filter::YOUR_CREATURE),
            &[Effect::PumpTarget {
                power: CREATURES_YOU_CONTROL_COUNT,
                toughness: CREATURES_YOU_CONTROL_COUNT,
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &Filter::ATTACKING_CREATURE
            ))),
        ),
    ],
);
