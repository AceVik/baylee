//! Purraj of Urborg — {3}{B}{B} — Legendary Creature — Cat Warrior
//! Oracle: Purraj has first strike as long as it's attacking.
//! Oracle: Whenever a player casts a black spell, you may pay {B}. If you do, put a +1/+1 counter on Purraj.
//! Set: MIR #135 — Mirage | Scryfall ID: 4704bf02-c6be-4983-9bc7-a1d464d21b31 | Oracle ID: ae86965c-3330-48bf-abf7-3142c61099ea
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLACK_SPELL: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));

card!(
    index = index::PURRAJ_OF_URBORG,
    oracle_id = "ae86965c-3330-48bf-abf7-3142c61099ea",
    scryfall_id = "4704bf02-c6be-4983-9bc7-a1d464d21b31",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Purraj of Urborg",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::CAT, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[
        static_ability!(
            Filter::And(&[Filter::This, Filter::Attacking]),
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
        ),
        triggered!(
            Trigger::SpellCast(&BLACK_SPELL),
            &[Effect::PlayerMayPayManaThen {
                player: PlayerRel::You,
                cost: mana!("{B}"),
                effects: &[Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
    ],
);
