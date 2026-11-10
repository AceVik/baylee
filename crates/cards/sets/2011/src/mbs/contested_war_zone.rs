//! Contested War Zone — (no cost) — Land
//! Oracle: Whenever a creature deals combat damage to you, that creature's controller gains control of this land.
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Attacking creatures get +1/+0 until end of turn.
//! Set: MBS #144 — Mirrodin Besieged | Scryfall ID: bee9b696-5203-4235-9bdd-f2a389d69813 | Oracle ID: ed73de2b-d7f4-48d9-9be2-aa9d111b7aa7
// IMPLEMENTED — creature combat damage to you hands this land to that creature's controller; {T}: {C}; {1}, {T}: attackers get +1/+0.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONTESTED_WAR_ZONE,
    oracle_id = "ed73de2b-d7f4-48d9-9be2-aa9d111b7aa7",
    scryfall_id = "bee9b696-5203-4235-9bdd-f2a389d69813",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Contested War Zone", types = TypeSet::LAND,)],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::PumpFilter {
                filter: &Filter::ATTACKING_CREATURE,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        // Last, so that the mana ability and the pump keep ability indices 0
        // and 1 (an `AbilityRef` index also keys a seat's standing answers).
        // No target: it moves this land. One trigger per creature (2011-06-01
        // ruling), the last to resolve decides.
        triggered!(
            Trigger::DealsCombatDamageToYou(&Filter::CREATURE),
            &[Effect::ChangeController {
                new_controller: PlayerRel::ControllerOfEvent,
            }]
        ),
    ],
);
