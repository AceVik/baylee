//! Tishana's Tidebinder — {2}{U} — Creature — Merfolk Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, counter up to one target activated or triggered ability. If an ability of an artifact, creature, or planeswalker is countered this way, that permanent loses all abilities for as long as this creature remains on the battlefield. (Mana abilities can't be targeted.)
//! Set: LCI #81 — The Lost Caverns of Ixalan | Scryfall ID: 907b3d1d-8c85-4707-80b5-c4d832df9846 | Oracle ID: 2993dc7d-723d-4a9b-94bd-4bb02a9f7243
// IMPLEMENTED — flash + counter target ability + the rider that follows it.
// The count is "up to one": a Tidebinder flashed in with an empty stack is a
// 2/1 that enters, not a trigger the rules remove for want of a target.
// NOT SUPPORTED: "loses all abilities" reaches only the permanent's keyword
// abilities. Activated, triggered and static abilities survive, because
// `GameObject::abilities` reads the card (or `own_abilities`) directly and is
// not projected through the layer system the way characteristics are — so
// there is nowhere for a continuous effect to take one away. Everything the
// rider does here the printed card also does; it is the half of the sentence
// that a mana dork or a planeswalker would notice and a flier would not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = 27125,
    oracle_id = "2993dc7d-723d-4a9b-94bd-4bb02a9f7243",
    scryfall_id = "907b3d1d-8c85-4707-80b5-c4d832df9846",
    faces = &[face!(
        name = "Tishana's Tidebinder",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::MERFOLK, creature::WIZARD],
        power = Some(3),
        toughness = Some(2),
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Partial("loses all abilities reaches only keywords"),
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[
            Effect::CounterTargetAbility,
            Effect::TargetSourceLosesAbilities {
                source_filter: &Filter::Or(&[
                    Filter::ARTIFACT,
                    Filter::CREATURE,
                    Filter::PLANESWALKER,
                ]),
            },
        ],
        targets = Some(TargetReq::up_to_one(TargetSpec::AbilityOnStack(
            &Filter::Any
        )))
    )],
);
