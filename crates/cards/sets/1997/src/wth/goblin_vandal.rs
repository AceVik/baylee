//! Goblin Vandal — {R} — Creature — Goblin Rogue
//! Oracle: Whenever this creature attacks and isn't blocked, you may pay {R}. If you do, destroy target artifact defending player controls and this creature assigns no combat damage this turn.
//! Set: WTH #105 — Weatherlight | Scryfall ID: b7ad3b81-f706-4b33-b1ec-7600182a5232 | Oracle ID: 50bebbb9-01b7-4eb6-8efd-307c9bcd2517
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DEFENDERS_ARTIFACT: Filter =
    Filter::And(&[Filter::ARTIFACT, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::GOBLIN_VANDAL,
    oracle_id = "50bebbb9-01b7-4eb6-8efd-307c9bcd2517",
    scryfall_id = "b7ad3b81-f706-4b33-b1ec-7600182a5232",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Vandal",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{R}"),
            effects: &[
                Effect::Destroy {
                    target: TargetSpec::Object(&DEFENDERS_ARTIFACT),
                    no_regen: false
                },
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn
                }
            ]
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&DEFENDERS_ARTIFACT)))
    ),],
);
