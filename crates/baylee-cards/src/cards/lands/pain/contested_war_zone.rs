//! Contested War Zone — (no cost) — Land
//! Oracle: Whenever a creature deals combat damage to you, that creature's controller gains control of this land.
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Attacking creatures get +1/+0 until end of turn.
//! Set: MBS #144 — Mirrodin Besieged | Scryfall ID: bee9b696-5203-4235-9bdd-f2a389d69813 | Oracle ID: ed73de2b-d7f4-48d9-9be2-aa9d111b7aa7
// PARTIAL — tap for {C} and {1}, {T}: attacking creatures get +1/+0 until end of turn.
// Transfer of control on combat damage to you is not expressible in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONTESTED_WAR_ZONE,
    oracle_id = "ed73de2b-d7f4-48d9-9be2-aa9d111b7aa7",
    scryfall_id = "bee9b696-5203-4235-9bdd-f2a389d69813",
    coverage = Coverage::Partial(
        "Trigger has no variant for combat damage dealt specifically to you and transferring control of the source permanent to the combat damage source controller is not expressible",
    ),
    faces = &[face!(name = "Contested War Zone", types = TypeSet::LAND,)],
    abilities = &[
        // NOT SUPPORTED: Whenever a creature deals combat damage to you, that creature's controller gains control of this land.
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
    ],
);
