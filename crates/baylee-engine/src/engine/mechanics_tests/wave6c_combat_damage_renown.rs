//! Rule tests for Engine Wave 6c:
//! - `Modifier::MaxBlockers(u8)` (`CR 509.1b`, `CR 509.1c`)
//! - `Modifier::PreventNoncombatDamageFrom` & `Modifier::PreventNoncombatDamageToPlayers` (`CR 615.1a`)
//! - `Modifier::MultiplyDamageFrom` & `Modifier::AddDamageFrom` (`CR 614.1a`, `CR 614.5`, `CR 616.1`, `CR 120.8`)
//! - `KeywordSet::RENOWN`, `Rider::Renowned`, `Condition::IsRenowned`, `Effect::BecomeRenowned` (`CR 702.112a–c`)

use super::*;
use crate::choice::DamageEffectKind;
use crate::object::Rider;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, DamageRecipients, Filter, Modifier, PlayerRel, TargetSpec,
    Trigger, spell, static_ability, triggered,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::Defender;

const C0: u32 = 99_800;
const C1: u32 = 99_801;
const C2: u32 = 99_802;
const C3: u32 = 99_803;
const C5: u32 = 99_805;

static RED_SRC: Filter = Filter::HasColor(ColorSet::of(Color::Red));

fn red_creature(name: &'static str, power: i16, toughness: i16) -> FaceDef {
    creature_face(name, "{R}", power, toughness)
}

fn is_renowned(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .riders
        .contains(&Rider::Renowned)
}

fn pt(engine: &Bench, id: ObjectId) -> (i16, i16) {
    let chars = engine.state().object(id).unwrap().characteristics();
    (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0))
}

fn declare_attackers_to_blocks(engine: &mut Bench, seat: PlayerId, attackers: &[ObjectId]) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    let defender = if seat == me() { them() } else { me() };
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .expect("declare attackers");
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == defender),
    );
}

fn settle_picking_first_damage_effect(engine: &mut Bench, seat: PlayerId) {
    for _ in 0..200 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        if let Pending::ChooseDamageEffect {
            player,
            choice,
            options,
            ..
        } = engine.pending().clone()
        {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseDamageEffect {
                        choice,
                        effect: options[0].id,
                    },
                )
                .expect("pick damage effect");
        } else {
            let p = engine.pending().clone();
            assert!(walk_past(engine, &p), "unexpected question: {p:?}");
        }
    }
    panic!("never settled");
}

#[test]
fn max_blockers_one_allows_single_blocker_rejects_two_and_takes_minimum_bound() {
    static STALKER_AB: &[AbilityDef] = &[
        static_ability!(Filter::This, Modifier::MaxBlockers(2)),
        static_ability!(Filter::This, Modifier::MaxBlockers(1)),
    ];
    let defs = vec![
        card(
            C0,
            creature_face("Stalker", "{1}{G}", 3, 3),
            KeywordSet::HASTE,
            STALKER_AB,
        ),
        card(
            C1,
            creature_face("Guard", "{1}", 1, 4),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine = bench(1, defs, [Seat::with(&[C0]), Seat::with(&[C1, C1])]);
    let stalker = the(&engine, ZoneLocation::Battlefield, C0);
    let guards = objects(&engine, ZoneLocation::Battlefield, C1);

    declare_attackers_to_blocks(&mut engine, me(), &[stalker]);
    let Pending::ChooseBlockers { bounds, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers");
    };
    assert_eq!(bounds.len(), 1);
    assert_eq!((bounds[0].min_blockers, bounds[0].max_blockers), (0, 1));

    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(guards[0], stalker), (guards[1], stalker)],
                },
            )
            .is_err(),
        "CR 509.1b: two blockers violate MaxBlockers(1)"
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(guards[0], stalker)],
            },
        )
        .expect("single blocker obeys MaxBlockers(1)");
}

#[test]
fn menace_plus_max_blockers_one_makes_attacker_unblockable() {
    static MAX_ONE_AB: &[AbilityDef] = &[static_ability!(Filter::This, Modifier::MaxBlockers(1))];
    let defs = vec![
        card(
            C0,
            creature_face("Alpha Raider", "{2}{R}", 3, 2),
            KeywordSet::HASTE.union(KeywordSet::MENACE),
            MAX_ONE_AB,
        ),
        card(
            C1,
            creature_face("Guard", "{1}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine = bench(
        2,
        defs,
        [Seat::with(&[C0]).at(20), Seat::with(&[C1, C1]).at(20)],
    );
    let raider = the(&engine, ZoneLocation::Battlefield, C0);
    let guards = objects(&engine, ZoneLocation::Battlefield, C1);

    declare_attackers_to_blocks(&mut engine, me(), &[raider]);
    for illegal in [
        vec![(guards[0], raider)],
        vec![(guards[0], raider), (guards[1], raider)],
    ] {
        assert!(
            engine
                .apply(them(), PlayerAction::DeclareBlockers { blockers: illegal })
                .is_err(),
            "CR 509.1b: Menace (min 2) + MaxBlockers(1) rejects 1 and 2 blockers"
        );
    }
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("zero blockers is legal");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(life(&engine, them()), 17);
}

#[test]
fn lure_plus_max_blockers_one_satisfies_one_requirement_without_deadlocking() {
    static LURE_ONE_AB: &[AbilityDef] = &[
        static_ability!(Filter::This, Modifier::MustBeBlockedByAllAble),
        static_ability!(Filter::This, Modifier::MaxBlockers(1)),
    ];
    let defs = vec![
        card(
            C0,
            creature_face("Tempting Boar", "{2}{G}", 3, 3),
            KeywordSet::HASTE,
            LURE_ONE_AB,
        ),
        card(
            C1,
            creature_face("Guard", "{1}", 1, 4),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine = bench(3, defs, [Seat::with(&[C0]), Seat::with(&[C1, C1])]);
    let boar = the(&engine, ZoneLocation::Battlefield, C0);
    let guards = objects(&engine, ZoneLocation::Battlefield, C1);

    declare_attackers_to_blocks(&mut engine, me(), &[boar]);
    assert!(
        engine
            .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "CR 509.1c: zero blockers satisfies 0 requirements when 1 is possible"
    );
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(guards[0], boar), (guards[1], boar)],
                },
            )
            .is_err(),
        "CR 509.1b: two blockers violate MaxBlockers(1)"
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(guards[1], boar)],
            },
        )
        .expect("one blocker satisfies the maximum possible requirements without violating cap");
}

#[test]
fn prevent_noncombat_damage_stops_pings_and_sweepers_but_allows_combat_damage() {
    static SANCTUARY_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::PreventNoncombatDamageFrom(&Filter::Any),
        ),
        static_ability!(
            Filter::This,
            Modifier::PreventNoncombatDamageToPlayers {
                who: PlayerRel::You,
                from: &Filter::Any,
            },
        ),
    ];
    static SWEEP_AB: &[AbilityDef] = &[free(
        &[
            Effect::DealDamageEach {
                amount: Amount::Fixed(2),
                filter: &Filter::CREATURE,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Player(PlayerRel::EachPlayer),
            },
        ],
        None,
    )];
    let defs = vec![
        card(
            C0,
            face("Sanctuary", "{2}{W}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            SANCTUARY_AB,
        ),
        card(
            C1,
            creature_face("Ally Bear", "{1}{W}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            C2,
            creature_face("Enemy Bear", "{1}{R}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            C3,
            creature_face("Enemy Brute", "{2}{R}", 3, 3),
            KeywordSet::HASTE,
            SWEEP_AB,
        ),
    ];
    let mut engine = bench(
        4,
        defs,
        [Seat::with(&[C0, C1]).at(20), Seat::with(&[C2, C3]).at(20)],
    );
    let ally = the(&engine, ZoneLocation::Battlefield, C1);
    let enemy_bear = the(&engine, ZoneLocation::Battlefield, C2);
    let brute = the(&engine, ZoneLocation::Battlefield, C3);

    to_main(&mut engine, them());
    activate(&mut engine, them(), brute, 0);
    settle(&mut engine, them());

    assert_eq!(life(&engine, me()), 20, "noncombat player damage prevented");
    assert_eq!(life(&engine, them()), 18, "opponent still takes sweeper");
    assert_eq!(engine.state().object(ally).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        engine.state().object(enemy_bear).unwrap().zone,
        Zone::Graveyard
    );

    declare_attackers_to_blocks(&mut engine, them(), &[brute]);
    engine
        .apply(me(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(life(&engine, me()), 17, "combat damage is NOT prevented");
}

#[test]
fn multiply_and_add_damage_respect_zero_and_stack_once_each_per_cr_120_8_and_614_5() {
    static TORBRAN_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::AddDamageFrom {
            sources: &RED_SRC,
            extra: 2,
            combat_only: false,
            noncombat_only: false,
            recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::EachOpponent),
        },
    )];
    static FURNACE_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::MultiplyDamageFrom {
            sources: &Filter::Any,
            factor: 2,
            combat_only: false,
            noncombat_only: false,
            recipients: DamageRecipients::Any,
        },
    )];
    static PING_AB: &[AbilityDef] = &[
        free(
            &[Effect::DealDamage {
                amount: Amount::Fixed(0),
                target: TargetSpec::Player(PlayerRel::EachPlayer),
            }],
            None,
        ),
        free(
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Player(PlayerRel::You),
            }],
            None,
        ),
    ];
    let defs = vec![
        card(
            C0,
            red_creature("Dwarf Thane", 2, 4),
            KeywordSet::EMPTY,
            TORBRAN_AB,
        ),
        card(
            C1,
            face("Furnace", "{1}{R}{R}{R}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            FURNACE_AB,
        ),
        card(
            C2,
            red_creature("Pyromancer", 1, 3),
            KeywordSet::EMPTY,
            PING_AB,
        ),
    ];
    let mut engine = bench(
        5,
        defs,
        [Seat::with(&[C0, C1, C1, C2]).at(20), Seat::with(&[]).at(20)],
    );
    let pyro = the(&engine, ZoneLocation::Battlefield, C2);

    to_main(&mut engine, me());
    activate(&mut engine, me(), pyro, 0);
    settle(&mut engine, me());
    assert_eq!(
        (life(&engine, me()), life(&engine, them())),
        (20, 20),
        "CR 120.8: 0 damage is neither increased nor multiplied"
    );

    activate(&mut engine, me(), pyro, 1);
    settle_picking_first_damage_effect(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        12,
        "CR 614.5: two x2 doublers turn 2 -> 8 without Torbran hitting controller"
    );
}

#[test]
fn unpreventable_combat_damage_is_still_increased_and_tripled() {
    static TORBRAN_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::This,
            Modifier::AddDamageFrom {
                sources: &RED_SRC,
                extra: 2,
                combat_only: false,
                noncombat_only: false,
                recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::EachOpponent),
            },
        ),
        static_ability!(Filter::CREATURE, Modifier::CombatDamageCantBePrevented),
    ];
    static TRIPLE_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::MultiplyDamageFrom {
            sources: &Filter::Any,
            factor: 3,
            combat_only: false,
            noncombat_only: false,
            recipients: DamageRecipients::Any,
        },
    )];
    let defs = vec![
        card(
            C0,
            red_creature("Dwarf Thane", 2, 4),
            KeywordSet::HASTE,
            TORBRAN_AB,
        ),
        card(
            C1,
            face("Emancipation", "{3}{R}{R}{R}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            TRIPLE_AB,
        ),
    ];
    let mut engine = bench(
        10,
        defs,
        [Seat::with(&[C0, C1]).at(20), Seat::with(&[]).at(20)],
    );
    let thane = the(&engine, ZoneLocation::Battlefield, C0);

    declare_attackers_to_blocks(&mut engine, me(), &[thane]);
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("unblocked");
    while engine.state().turn.phase != Phase::SecondMain {
        if let Pending::ChooseDamageEffect {
            player,
            choice,
            options,
            ..
        } = engine.pending().clone()
        {
            let add_first = options
                .iter()
                .find(|o| matches!(o.kind, DamageEffectKind::AddDamage { .. }))
                .unwrap_or(&options[0])
                .id;
            engine
                .apply(
                    player,
                    PlayerAction::ChooseDamageEffect {
                        choice,
                        effect: add_first,
                    },
                )
                .expect("pick AddDamage first");
        } else {
            let p = engine.pending().clone();
            assert!(walk_past(&mut engine, &p));
        }
    }
    assert_eq!(life(&engine, them()), 8, "(2 + 2) * 3 = 12 damage");
}

#[test]
fn affected_player_orders_multiplier_and_adder_per_cr_616_1() {
    static TORBRAN_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::AddDamageFrom {
            sources: &RED_SRC,
            extra: 2,
            combat_only: false,
            noncombat_only: false,
            recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::EachOpponent),
        },
    )];
    static FURNACE_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::MultiplyDamageFrom {
            sources: &Filter::Any,
            factor: 2,
            combat_only: false,
            noncombat_only: false,
            recipients: DamageRecipients::Any,
        },
    )];
    static PING_TWO_AB: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::EachOpponent),
        }],
        None,
    )];
    let defs = vec![
        card(
            C0,
            red_creature("Thane", 2, 4),
            KeywordSet::EMPTY,
            TORBRAN_AB,
        ),
        card(
            C1,
            face("Furnace", "{3}{R}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            FURNACE_AB,
        ),
        card(
            C2,
            red_creature("Mage", 1, 2),
            KeywordSet::EMPTY,
            PING_TWO_AB,
        ),
    ];

    let run_order = |pick_mult_first: bool| -> i32 {
        let mut engine = bench(
            6,
            defs.clone(),
            [Seat::with(&[C0, C1, C2]).at(20), Seat::with(&[]).at(20)],
        );
        let mage = the(&engine, ZoneLocation::Battlefield, C2);
        to_main(&mut engine, me());
        activate(&mut engine, me(), mage, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseDamageEffect { .. })
        });
        let Pending::ChooseDamageEffect {
            player,
            choice,
            options,
            ..
        } = engine.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!(player, them(), "CR 616.1: affected player chooses order");
        let chosen = options
            .iter()
            .find(|o| matches!(o.kind, DamageEffectKind::MultiplyDamage { .. }) == pick_mult_first)
            .unwrap()
            .id;
        engine
            .apply(
                them(),
                PlayerAction::ChooseDamageEffect {
                    choice,
                    effect: chosen,
                },
            )
            .expect("apply chosen damage modifier");
        settle(&mut engine, me());
        life(&engine, them())
    };
    assert_eq!(run_order(true), 14, "(2 * 2) + 2 = 6 damage");
    assert_eq!(run_order(false), 12, "(2 + 2) * 2 = 8 damage");
}

#[test]
fn preventing_all_damage_first_stops_adder_from_creating_damage_per_cr_120_8() {
    static TORBRAN_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::AddDamageFrom {
            sources: &RED_SRC,
            extra: 2,
            combat_only: false,
            noncombat_only: false,
            recipients: DamageRecipients::PlayersOrTheirPermanents(PlayerRel::EachOpponent),
        },
    )];
    static SHIELD_PING_AB: &[AbilityDef] = &[
        free(
            &[Effect::PreventNextDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::EachOpponent),
            }],
            None,
        ),
        free(
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::EachOpponent),
            }],
            None,
        ),
    ];
    let defs = vec![
        card(
            C0,
            red_creature("Thane", 2, 4),
            KeywordSet::EMPTY,
            TORBRAN_AB,
        ),
        card(
            C2,
            red_creature("Mage", 1, 2),
            KeywordSet::EMPTY,
            SHIELD_PING_AB,
        ),
    ];
    let mut engine = bench(
        7,
        defs,
        [Seat::with(&[C0, C2]).at(20), Seat::with(&[]).at(20)],
    );
    let mage = the(&engine, ZoneLocation::Battlefield, C2);
    to_main(&mut engine, me());
    activate(&mut engine, me(), mage, 0);
    settle(&mut engine, me());
    activate(&mut engine, me(), mage, 1);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    let Pending::ChooseDamageEffect {
        player,
        choice,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    let prevent = options
        .iter()
        .find(|o| matches!(o.kind, DamageEffectKind::PreventNext { .. }))
        .unwrap()
        .id;
    engine
        .apply(
            player,
            PlayerAction::ChooseDamageEffect {
                choice,
                effect: prevent,
            },
        )
        .expect("prevent 1 first -> 0 remaining -> AddDamage stops applying");
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 20);
}

#[test]
fn renown_grants_counters_and_status_once_and_activates_conditional_statics() {
    static RENOWN_HERO_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[Effect::BecomeRenowned(2)],
            condition = Some(Condition::Not(&Condition::IsRenowned)),
        ),
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(1, 1),
            condition = Some(Condition::IsRenowned),
        ),
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::VIGILANCE),
            condition = Some(Condition::IsRenowned),
        ),
    ];
    let defs = vec![
        card(
            C0,
            creature_face("Freeblade", "{1}{W}", 2, 2),
            KeywordSet::HASTE.union(KeywordSet::RENOWN),
            RENOWN_HERO_AB,
        ),
        card(
            C1,
            creature_face("Wall", "{1}", 0, 5),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine = bench(
        8,
        defs,
        [Seat::with(&[C0]).at(20), Seat::with(&[C1]).at(20)],
    );
    let hero = the(&engine, ZoneLocation::Battlefield, C0);
    let wall = the(&engine, ZoneLocation::Battlefield, C1);

    declare_attackers_to_blocks(&mut engine, me(), &[hero]);
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, hero)],
            },
        )
        .expect("wall blocks");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(triggered(&engine, hero), 0);
    assert!(!is_renowned(&engine, hero));

    declare_attackers_to_blocks(&mut engine, me(), &[hero]);
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("unblocked");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(triggered(&engine, hero), 1);
    assert!(is_renowned(&engine, hero));
    assert_eq!(
        engine
            .state()
            .object(hero)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(pt(&engine, hero), (5, 5));
    assert!(
        engine
            .state()
            .object(hero)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::VIGILANCE)
    );

    declare_attackers_to_blocks(&mut engine, me(), &[hero]);
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("unblocked again");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        triggered(&engine, hero),
        1,
        "already renowned -> does not trigger"
    );
}

#[test]
fn blinking_renowned_creature_clears_renowned_so_new_incarnation_triggers_again() {
    static RENOWN_HERO_AB: &[AbilityDef] = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::BecomeRenowned(2)],
        condition = Some(Condition::Not(&Condition::IsRenowned)),
    )];
    static BLINK_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let defs = vec![
        card(
            C0,
            creature_face("Freeblade", "{1}{W}", 2, 2),
            KeywordSet::HASTE.union(KeywordSet::RENOWN),
            RENOWN_HERO_AB,
        ),
        card(
            C2,
            face("Flicker", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BLINK_SPELL,
        ),
    ];
    let mut engine = bench(
        11,
        defs,
        [
            Seat::with(&[C0]).holding(&[C2]).at(20),
            Seat::with(&[]).at(20),
        ],
    );
    let hero = the(&engine, ZoneLocation::Battlefield, C0);

    declare_attackers_to_blocks(&mut engine, me(), &[hero]);
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("unblocked");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(is_renowned(&engine, hero));

    let flicker = the(&engine, ZoneLocation::Hand(me()), C2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: flicker })
        .expect("cast flicker");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![hero],
                players: vec![],
            },
        )
        .expect("target hero");
    settle(&mut engine, me());
    let reborn = the(&engine, ZoneLocation::Battlefield, C0);
    assert!(
        !is_renowned(&engine, reborn),
        "CR 400.7: blink resets renowned"
    );
    assert_eq!(pt(&engine, reborn), (2, 2));

    declare_attackers_to_blocks(&mut engine, me(), &[reborn]);
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("unblocked after blink");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(is_renowned(&engine, reborn));
    assert_eq!(pt(&engine, reborn), (4, 4));
}

#[test]
fn two_renown_instances_both_trigger_first_hit_and_second_checks_intervening_if_on_resolution() {
    const RENOWN_ONE: AbilityDef = triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::BecomeRenowned(1)],
        condition = Some(Condition::Not(&Condition::IsRenowned)),
    );
    static DOUBLE_RENOWN_AB: &[AbilityDef] = &[RENOWN_ONE, RENOWN_ONE];
    let defs = vec![card(
        C5,
        creature_face("Twin Renown", "{1}{W}", 2, 2),
        KeywordSet::HASTE.union(KeywordSet::RENOWN),
        DOUBLE_RENOWN_AB,
    )];
    let mut engine = bench(9, defs, [Seat::with(&[C5]).at(20), Seat::with(&[]).at(20)]);
    let twin = the(&engine, ZoneLocation::Battlefield, C5);

    declare_attackers_to_blocks(&mut engine, me(), &[twin]);
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("unblocked");
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    let fired = engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                crate::event::GameEvent::AbilityTriggered { source, .. } if source == twin
            )
        })
        .count();
    assert_eq!(fired, 2, "CR 702.112c: both instances trigger on first hit");
    assert!(is_renowned(&engine, twin));
    assert_eq!(
        engine
            .state()
            .object(twin)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "CR 702.112c: second instance fails intervening-if on resolution and adds 0 counters"
    );
}
