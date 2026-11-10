//! Invoke Prejudice — {U}{U}{U}{U} — Enchantment
//! Oracle: Whenever an opponent casts a creature spell that doesn't share a color with a creature you control, counter that spell unless that player pays {X}, where X is its mana value.
//! Set: LEG #62 — Legends | Scryfall ID: 903d9fde-d7da-4a0e-a337-b63023c6d74b | Oracle ID: 854ad486-0c59-4c57-9a76-ab1dff0ff37c
// IMPLEMENTED — whenever an opponent casts a creature spell that doesn't share
// a color with a creature you control, counter it unless that player pays its
// mana value.

use baylee_cards_dsl::prelude::*;

// "a creature spell that doesn't share a color with a creature you control":
// the spell is cast by an opponent; "a creature you control" is read off the
// battlefield, so the spell itself is never the creature compared.
const UNLIKE_OPPONENT_CREATURE_SPELL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByOpponent,
    Filter::Not(&Filter::SharesAColorWith(&Filter::YOUR_CREATURE)),
]);

card!(
    index = index::INVOKE_PREJUDICE,
    oracle_id = "854ad486-0c59-4c57-9a76-ab1dff0ff37c",
    scryfall_id = "903d9fde-d7da-4a0e-a337-b63023c6d74b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Invoke Prejudice",
        mana_cost = mana!("{U}{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&UNLIKE_OPPONENT_CREATURE_SPELL),
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfEvent,
            mana: Amount::TargetCmc,
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::EventObject))
    )],
);
