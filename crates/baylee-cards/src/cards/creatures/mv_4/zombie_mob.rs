//! Zombie Mob — {2}{B}{B} — Creature — Zombie
//! Oracle: This creature enters with a +1/+1 counter on it for each creature card in your graveyard.
//! Oracle: When this creature enters, exile all creature cards from your graveyard.
//! Set: MIR #153 — Mirage | Scryfall ID: ab85551f-c9cc-409c-9fb5-a45de695e521 | Oracle ID: c56f0f4c-c85a-4931-b22a-42dc5d1dc7dd
// PARTIAL — enters with a +1/+1 counter on it for each creature card in your graveyard and exiles your graveyard when it enters; no filtered graveyard-exile effect ("exile all creature cards from your graveyard") exists yet.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOMBIE_MOB,
    oracle_id = "c56f0f4c-c85a-4931-b22a-42dc5d1dc7dd",
    scryfall_id = "ab85551f-c9cc-409c-9fb5-a45de695e521",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "enters with a +1/+1 counter on it for each creature card in your graveyard and exiles all cards from your graveyard when it enters; no filtered graveyard-exile effect ('exile all creature cards from your graveyard') exists yet",
    ),
    faces = &[face!(
        name = "Zombie Mob",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::CountOf {
                filter: &Filter::CREATURE,
                zone: ZoneSel::GraveyardYou,
            },
        }],
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::ExileGraveyard {
            player: PlayerRel::You,
        }],
    )],
);
