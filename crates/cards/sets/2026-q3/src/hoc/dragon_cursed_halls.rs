//! Dragon-Cursed Halls — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Until end of turn, target creature gains "Whenever this creature deals combat damage to a player, create a Treasure token."
//! Set: HOC #8 — The Hobbit Eternal | Scryfall ID: 506b9df7-8236-4c6e-aebc-6b7e6fcd7e88 | Oracle ID: 5be7a4d5-33b7-464b-8851-d4ad35302e62
// IMPLEMENTED — {T}: Add {C}, and {1}, {T} grants target creature a combat-damage-to-player Treasure trigger until end of turn.

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;

static GRANT_TREASURE_TRIGGER: &[Effect] = &[Effect::CreateToken { token: &TREASURE }];

card!(
    index = index::DRAGON_CURSED_HALLS,
    oracle_id = "5be7a4d5-33b7-464b-8851-d4ad35302e62",
    scryfall_id = "506b9df7-8236-4c6e-aebc-6b7e6fcd7e88",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Dragon-Cursed Halls", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::GrantTriggered {
                    trigger: Trigger::DealsCombatDamageToPlayer(&Filter::This),
                    effects: GRANT_TREASURE_TRIGGER,
                    target: None,
                },
                Duration::UntilEndOfTurn,
            )],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
