//! Regeneration restrictions follow actual damage and live static abilities.

use super::*;
use baylee_cards_dsl::{Amount, Duration, Filter, Modifier, TargetSpec};

const SOURCE: u32 = 7950;
const VICTIM: u32 = 7951;
const TOOLS: u32 = 7952;
const CREATURE: TargetSpec = TargetSpec::Object(&Filter::CREATURE);
const PING: Effect = Effect::DealDamage {
    amount: Amount::Fixed(1),
    target: CREATURE,
};
static SOURCE_ABILITIES: &[AbilityDef] = &[
    free(&[PING], Some(TargetReq::one(CREATURE))),
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::GrantStatic {
                filter: &Filter::And(&[Filter::CREATURE, Filter::DealtDamageBySourceThisTurn]),
                modifier: &Modifier::CantBeRegenerated,
            },
            Duration::UntilEndOfTurn,
        )],
        None,
    ),
    free(
        &[PING, Effect::DamagedThisWayCantBeRegenerated],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[
            PING,
            Effect::Blink {
                target: CREATURE,
                owner_control: true,
            },
            Effect::DamagedThisWayCantBeRegenerated,
        ],
        Some(TargetReq::one(CREATURE)),
    ),
];
static TOOLS_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::Destroy {
            target: CREATURE,
            no_regen: false,
        }],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::LoseAllAbilities,
            Duration::UntilEndOfTurn,
        )],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[Effect::ReturnToHand { target: CREATURE }],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[Effect::Blink {
            target: CREATURE,
            owner_control: true,
        }],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[Effect::DestroyAll {
            filter: &Filter::CREATURE,
            no_regen: false,
        }],
        None,
    ),
    free(
        &[Effect::DealDamageEach {
            amount: Amount::Fixed(5),
            filter: &Filter::CREATURE,
        }],
        None,
    ),
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::GrantStatic {
                filter: &Filter::And(&[Filter::CREATURE, Filter::DealtDamageBySourceThisTurn]),
                modifier: &Modifier::CantBeRegenerated,
            },
            Duration::UntilEndOfTurn,
        )],
        Some(TargetReq::one(CREATURE)),
    ),
];
static REGENERATE: &[AbilityDef] = &[free(
    &[Effect::Regenerate {
        target: TargetSpec::ThisObject,
    }],
    None,
)];

fn game() -> (Bench, ObjectId, ObjectId, ObjectId) {
    game_order(false)
}

fn game_order(reverse: bool) -> (Bench, ObjectId, ObjectId, ObjectId) {
    let mut engine = bench(
        7950,
        vec![
            card(
                SOURCE,
                creature_face("Damage source", "{0}", 1, 5),
                KeywordSet::EMPTY,
                SOURCE_ABILITIES,
            ),
            card(
                VICTIM,
                creature_face("Regenerator", "{0}", 0, 5),
                KeywordSet::EMPTY,
                REGENERATE,
            ),
            card(
                TOOLS,
                face("Rule tools", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                TOOLS_ABILITIES,
            ),
        ],
        [
            Seat::with(if reverse {
                &[VICTIM, SOURCE, TOOLS]
            } else {
                &[SOURCE, VICTIM, TOOLS]
            }),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let source = the(&engine, ZoneLocation::Battlefield, SOURCE);
    let victim = the(&engine, ZoneLocation::Battlefield, VICTIM);
    let tools = the(&engine, ZoneLocation::Battlefield, TOOLS);
    (engine, source, victim, tools)
}

fn use_ability(engine: &mut Bench, source: ObjectId, index: u32, target: Option<ObjectId>) {
    activate(engine, me(), source, index);
    if let Some(target) = target {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(engine, me());
}

fn regenerate_then_destroy(engine: &mut Bench, victim: ObjectId, tools: ObjectId, survives: bool) {
    use_ability(engine, victim, 0, None);
    assert_eq!(
        engine.state().object(victim).unwrap().regeneration_shields,
        1,
        "a prohibition does not prevent creating the shield"
    );
    use_ability(engine, tools, 0, Some(victim));
    assert_eq!(
        engine.state().object(victim).unwrap().zone == crate::zone::Zone::Battlefield,
        survives
    );
}

#[test]
fn regeneration_damage_prohibition_requires_damage_in_this_resolution() {
    for forbid in [false, true] {
        let (mut engine, source, victim, tools) = game();
        use_ability(
            &mut engine,
            source,
            if forbid { 2 } else { 0 },
            Some(victim),
        );
        regenerate_then_destroy(&mut engine, victim, tools, !forbid);
    }
}

#[test]
fn regeneration_damage_prohibition_does_not_follow_a_blink_within_resolution() {
    let (mut engine, source, victim, tools) = game();
    use_ability(&mut engine, source, 3, Some(victim));
    regenerate_then_destroy(&mut engine, victim, tools, true);
}

#[test]
fn regeneration_static_reads_damage_before_or_after_the_grant() {
    for grant_first in [false, true] {
        let (mut engine, source, victim, tools) = game();
        if grant_first {
            use_ability(&mut engine, source, 1, None);
        }
        use_ability(&mut engine, source, 0, Some(victim));
        if !grant_first {
            use_ability(&mut engine, source, 1, None);
        }
        regenerate_then_destroy(&mut engine, victim, tools, false);
    }
}

#[test]
fn regeneration_static_stops_when_source_leaves_or_either_incarnation_changes() {
    for change in [0, 1, 2] {
        let (mut engine, source, victim, tools) = game();
        use_ability(&mut engine, source, 1, None);
        use_ability(&mut engine, source, 0, Some(victim));
        let target = if change == 2 { victim } else { source };
        use_ability(
            &mut engine,
            tools,
            if change == 0 { 2 } else { 3 },
            Some(target),
        );
        if change == 1 {
            use_ability(&mut engine, source, 1, None);
        }
        regenerate_then_destroy(&mut engine, victim, tools, true);
    }
}

#[test]
fn regeneration_static_disappears_with_ability_loss_and_a_later_grant_restores_it() {
    for regrant in [false, true] {
        let (mut engine, source, victim, tools) = game();
        use_ability(&mut engine, source, 1, None);
        use_ability(&mut engine, source, 0, Some(victim));
        use_ability(&mut engine, tools, 1, Some(source));
        if regrant {
            use_ability(&mut engine, tools, 6, Some(source));
        }
        regenerate_then_destroy(&mut engine, victim, tools, !regrant);
    }
}

#[test]
fn regeneration_prohibitions_expire_at_cleanup() {
    for static_grant in [false, true] {
        let (mut engine, source, victim, tools) = game();
        if static_grant {
            use_ability(&mut engine, source, 1, None);
        }
        use_ability(
            &mut engine,
            source,
            if static_grant { 0 } else { 2 },
            Some(victim),
        );
        to_main(&mut engine, them());
        to_main(&mut engine, me());
        regenerate_then_destroy(&mut engine, victim, tools, true);
    }
}

#[test]
fn regeneration_static_sees_one_board_for_simultaneous_destruction_and_lethal_sbas() {
    for reverse in [false, true] {
        for ability in [4, 5] {
            let (mut engine, source, victim, tools) = game_order(reverse);
            use_ability(&mut engine, source, 1, None);
            use_ability(&mut engine, source, 0, Some(victim));
            use_ability(&mut engine, victim, 0, None);
            use_ability(&mut engine, tools, ability, None);
            for id in [source, victim] {
                assert_eq!(
                    engine.state().object(id).unwrap().zone,
                    crate::zone::Zone::Graveyard,
                    "a simultaneous departure keeps the static prohibition for every victim"
                );
            }
        }
    }
}
