//! Mtenda Lion — {G} — Creature — Cat
//! Oracle: Whenever this creature attacks, defending player may pay {U}. If that player does, prevent all combat damage that would be dealt by this creature this turn.
//! Set: MIR #230 — Mirage | Scryfall ID: cb05cf5b-2a0d-432a-b8e7-10335c2a18e8 | Oracle ID: af029853-cfb0-403b-af51-141ba02ae2e4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MTENDA_LION,
    oracle_id = "af029853-cfb0-403b-af51-141ba02ae2e4",
    scryfall_id = "cb05cf5b-2a0d-432a-b8e7-10335c2a18e8",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mtenda Lion",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::DefendingPlayer,
            cost: mana!("{U}"),
            effects: &[Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn,
            )],
        }],
    )],
);
