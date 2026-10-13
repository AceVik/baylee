//! Floral Spuzzem — {3}{G} — Creature — Elemental
//! Oracle: Whenever this creature attacks and isn't blocked, you may destroy target artifact defending player controls. If you do, this creature assigns no combat damage this turn.
//! Set: LEG #187 — Legends | Scryfall ID: d141b9e3-7129-41e5-8b44-d3867e1c7e1d | Oracle ID: 994de451-14f9-466f-a56e-da052b4666e5
// IMPLEMENTED — when it attacks and isn't blocked (declare blockers, CR 509.1h)
// its controller may destroy a target artifact the defending player controls;
// if that artifact was destroyed, this creature assigns no combat damage this
// turn (an indestructible target leaves its damage intact).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Target artifact defending player controls."
static DEFENDERS_ARTIFACT: Filter =
    Filter::And(&[Filter::ARTIFACT, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::FLORAL_SPUZZEM,
    oracle_id = "994de451-14f9-466f-a56e-da052b4666e5",
    scryfall_id = "d141b9e3-7129-41e5-8b44-d3867e1c7e1d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Floral Spuzzem",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::destroy(TargetSpec::Object(&DEFENDERS_ARTIFACT)),
                // "If you do": only an artifact that was destroyed counts.
                Effect::IfCondition {
                    condition: Condition::AmountAtLeast(
                        Amount::ThisWay(ThisWay::Died(&Filter::ARTIFACT)),
                        1
                    ),
                    then: &[Effect::continuous(
                        &Filter::This,
                        Modifier::AssignsNoCombatDamage,
                        Duration::UntilEndOfTurn
                    )],
                    otherwise: &[],
                },
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&DEFENDERS_ARTIFACT)))
    )],
);
