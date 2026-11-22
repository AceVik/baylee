//! Rule tests for Engine Wave 5b:
//! - Wither (`KeywordSet::WITHER`, CR 702.80a–d, CR 120.3d–e)
//! - Horsemanship (`KeywordSet::HORSEMANSHIP`, CR 702.31b)
//! - Skulk (`KeywordSet::SKULK`, CR 702.118a–c)
//! - Spell-cast targeting / heroic (`Trigger::SpellCastTargeting`, `Trigger::HEROIC`,
//!   CR 207.2c, CR 601.2h, CR 603.2c, CR 109.2)
//! - Enlist (`KeywordSet::ENLIST`, `Trigger::Enlisted`, CR 702.154a–d, CR 508.1g)
//! - Bolster (`Effect::Bolster`, CR 701.39a, CR 120.6)

use super::*;
use crate::choice::ChoicePrompt;
use crate::object::Status;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{
    Amount, CounterKind, Duration, Filter, ManaColor, Modifier, PlayerRel, TargetSlot, TargetSpec,
    Trigger, mana_ability, static_ability, triggered,
};
use baylee_core::ids::Defender;

const CARD_A: u32 = 99_950;
const CARD_B: u32 = 99_951;
const CARD_C: u32 = 99_952;
const CARD_D: u32 = 99_953;
const CARD_E: u32 = 99_954;
const CARD_F: u32 = 99_955;
const CARD_G: u32 = 99_956;

fn declare_attackers_step(engine: &mut Bench, seat: PlayerId, pairs: Vec<(ObjectId, Defender)>) {
    walk_until(engine, |e| {
        e.state().turn.active == seat
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat)
    });
    engine
        .apply(seat, PlayerAction::DeclareAttackers { attackers: pairs })
        .expect("declare attackers");
}

fn cast_zero_targeting(
    engine: &mut Bench,
    seat: PlayerId,
    spell_id: ObjectId,
    targets: &[ObjectId],
) {
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(legal.castable.contains(&spell_id));
    engine
        .apply(seat, PlayerAction::CastSpell { card: spell_id })
        .expect("cast spell");
    for &target in targets {
        let Pending::ChooseTargets { player: p, .. } = engine.pending().clone() else {
            panic!("expected target choice, got {:?}", engine.pending());
        };
        assert_eq!(p, seat);
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .expect("choose target");
    }
}

fn pt(engine: &Bench, id: ObjectId) -> (i16, i16) {
    let chars = engine
        .state()
        .object(id)
        .expect("object on board")
        .characteristics();
    (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0))
}

fn counters(engine: &Bench, id: ObjectId, kind: CounterKind) -> u16 {
    engine
        .state()
        .object(id)
        .expect("object")
        .counters
        .get(kind)
}

// =========================================================================
// 1. Wither (CR 702.80a–d, CR 120.3d–e)
// =========================================================================

static PING_CREATURE_2: &[AbilityDef] = &[free(
    &[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::Object(&Filter::CREATURE),
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

static DESTROY_CREATURE: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::Destroy {
        target: TargetSpec::Object(&Filter::CREATURE),
        no_regen: false,
    }],
    targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    second_targets: None,
    third_targets: None,
    condition: None,
}];

#[test]
fn wither_places_m1m1_counters_on_creatures_and_normal_damage_on_players() {
    let mut engine = bench(
        1,
        vec![
            card(
                CARD_A,
                creature_face("WitherA", "{0}", 2, 2),
                KeywordSet::WITHER,
                &[],
            ),
            card(
                CARD_B,
                creature_face("WitherB", "{0}", 3, 2),
                KeywordSet::WITHER,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Indest", "{0}", 2, 2),
                KeywordSet::INDESTRUCTIBLE,
                &[],
            ),
        ],
        [Seat::with(&[CARD_A, CARD_B]), Seat::with(&[CARD_C]).at(20)],
    );
    let wa = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let wb = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let indest = the(&engine, ZoneLocation::Battlefield, CARD_C);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![
            (wa, Defender::Player(them())),
            (wb, Defender::Player(them())),
        ],
    );
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == them()),
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(indest, wa)],
            },
        )
        .expect("block wither attacker");
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Main)
    });

    // Indestructible 2/2 took 2 -1/-1 counters -> 0 toughness SBA death (CR 704.5f).
    assert_eq!(engine.state().object(indest).unwrap().zone, Zone::Graveyard);
    // Unblocked 3/2 Wither dealt normal life loss (not poison) to player.
    assert_eq!(life(&engine, them()), 17);
    assert_eq!(
        engine.state().players[them().get() as usize]
            .counters()
            .iter()
            .find(|&&(k, _)| k == CounterKind::Poison)
            .map_or(0, |&(_, n)| n),
        0
    );
}

#[test]
fn wither_with_infect_places_counters_once_and_preserves_lki_after_death() {
    let both = KeywordSet::WITHER.union(KeywordSet::INFECT);
    let mut engine = bench(
        2,
        vec![
            card(
                CARD_A,
                creature_face("BothPing", "{0}", 2, 2),
                both,
                PING_CREATURE_2,
            ),
            card(
                CARD_B,
                creature_face("WitherPing", "{0}", 2, 2),
                KeywordSet::WITHER,
                PING_CREATURE_2,
            ),
            card(
                CARD_C,
                creature_face("TargetGiant", "{0}", 4, 5),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_D,
                face("InstantKill", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                DESTROY_CREATURE,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B]).holding(&[CARD_D]),
            Seat::with(&[CARD_C]),
        ],
    );
    to_main(&mut engine, me());
    let both_src = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let wither_src = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let giant = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let kill = the(&engine, ZoneLocation::Hand(me()), CARD_D);

    // 1) Wither + Infect deals 2 damage -> exactly 2 -1/-1 counters (CR 120.3d).
    activate(&mut engine, me(), both_src, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(counters(&engine, giant, CounterKind::M1M1), 2);
    assert_eq!(engine.state().object(giant).unwrap().damage, 0);
    assert_eq!(pt(&engine, giant), (2, 3));

    // 2) Source with Wither dies in response -> LKI still places 2 -1/-1 counters (CR 702.80b).
    activate(&mut engine, me(), wither_src, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    cast_zero_targeting(&mut engine, me(), kill, &[wither_src]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(wither_src).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(counters(&engine, giant, CounterKind::M1M1), 4);
    assert_eq!(pt(&engine, giant), (0, 1));
}

// =========================================================================
// 2. Horsemanship (CR 702.31b) & Skulk (CR 702.118a–b)
// =========================================================================

static PUMP_TARGET_2_2: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::PumpTarget {
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(2),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }],
    targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    second_targets: None,
    third_targets: None,
    condition: None,
}];

fn horsemanship_skulk_cards() -> Vec<&'static CardDef> {
    vec![
        card(
            CARD_A,
            creature_face("Rider", "{0}", 2, 2),
            KeywordSet::HORSEMANSHIP,
            &[],
        ),
        card(
            CARD_B,
            creature_face("Sneak", "{0}", 2, 2),
            KeywordSet::SKULK,
            &[],
        ),
        card(
            CARD_C,
            creature_face("Griffin", "{0}", 2, 2),
            KeywordSet::FLYING.union(KeywordSet::REACH),
            &[],
        ),
        card(
            CARD_D,
            creature_face("EnemyRider", "{0}", 3, 3),
            KeywordSet::HORSEMANSHIP,
            &[],
        ),
        card(
            CARD_E,
            face("Growth", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            PUMP_TARGET_2_2,
        ),
    ]
}

#[test]
fn horsemanship_and_skulk_enforce_blocking_restrictions_and_react_to_pumps() {
    let mut engine = bench(
        3,
        horsemanship_skulk_cards(),
        [
            Seat::with(&[CARD_A, CARD_B]).holding(&[CARD_E]),
            Seat::with(&[CARD_C, CARD_D]),
        ],
    );
    let rider = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let sneak = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let griffin = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let enemy_rider = the(&engine, ZoneLocation::Battlefield, CARD_D);
    let growth = the(&engine, ZoneLocation::Hand(me()), CARD_E);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![
            (rider, Defender::Player(them())),
            (sneak, Defender::Player(them())),
        ],
    );
    // Pump defender's 2/2 Griffin to 4/4 before declare blockers so it outgrows Skulk 2/2!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    cast_zero_targeting(&mut engine, me(), growth, &[griffin]);

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == them()),
    );
    // Flying+Reach 4/4 Griffin cannot block Horsemanship Rider OR 2/2 Skulk Sneak;
    // 3/3 EnemyRider CAN block Horsemanship Rider, CANNOT block 2/2 Skulk Sneak!
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(griffin, rider)],
                },
            )
            .is_err()
    );
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(griffin, sneak)],
                },
            )
            .is_err()
    );
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(enemy_rider, sneak)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(enemy_rider, rider)],
            },
        )
        .expect("horsemanship blocks horsemanship");
}

// =========================================================================
// 3. Heroic & SpellCastTargeting (CR 207.2c, CR 601.2h, CR 603.2c, CR 109.2)
// =========================================================================

static HEROIC_COUNTER: &[AbilityDef] = &[triggered!(
    Trigger::HEROIC,
    &[Effect::AddCounterSource {
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(1),
    }]
)];

static DUAL_TARGET_SPELL: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[
        Effect::PumpTarget {
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        },
        Effect::PumpTargetSlot {
            slot: TargetSlot::Second,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        },
    ],
    targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    second_targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    third_targets: None,
    condition: None,
}];

static COPY_STACK_SPELL: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::CopySpellOrAbility {
        target: TargetSpec::Spell(&Filter::Any),
        controller: PlayerRel::You,
        amount: Amount::Fixed(1),
        mods: &[],
        may_choose_new_targets: false,
    }],
    targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    second_targets: None,
    third_targets: None,
    condition: None,
}];

#[test]
fn heroic_fires_once_per_cast_and_ignores_opponents_abilities_and_copies() {
    let mut engine = bench(
        4,
        vec![
            card(
                CARD_A,
                creature_face("Hoplite", "{0}", 1, 2),
                KeywordSet::EMPTY,
                HEROIC_COUNTER,
            ),
            card(
                CARD_B,
                creature_face("Pinger", "{0}", 1, 1),
                KeywordSet::EMPTY,
                PING_CREATURE_2,
            ),
            card(
                CARD_C,
                face("TwinBoon", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                DUAL_TARGET_SPELL,
            ),
            card(
                CARD_D,
                face("Twincast", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                COPY_STACK_SPELL,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B]).holding(&[CARD_C, CARD_D]),
            Seat::with(&[]).holding(&[CARD_C]),
        ],
    );
    to_main(&mut engine, me());
    let hoplite = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let pinger = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let my_boon = the(&engine, ZoneLocation::Hand(me()), CARD_C);
    let twincast = the(&engine, ZoneLocation::Hand(me()), CARD_D);
    let opp_boon = the(&engine, ZoneLocation::Hand(them()), CARD_C);

    // Dual-slot spell targeting Hoplite twice + Twincast copying it -> Heroic fires ONCE!
    cast_zero_targeting(&mut engine, me(), my_boon, &[hoplite, hoplite]);
    let stack_boon = engine.state().zones.list(ZoneLocation::Stack)[0];
    cast_zero_targeting(&mut engine, me(), twincast, &[stack_boon]);
    settle(&mut engine, me());
    assert_eq!(counters(&engine, hoplite, CounterKind::P1P1), 1);

    // Activated ability targeting Hoplite -> does NOT fire Heroic.
    activate(&mut engine, me(), pinger, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![hoplite],
                players: vec![],
            },
        )
        .unwrap();
    // Opponent casting a spell targeting Hoplite in response -> does NOT fire Heroic.
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    cast_zero_targeting(&mut engine, them(), opp_boon, &[hoplite, hoplite]);
    settle(&mut engine, me());
    assert_eq!(counters(&engine, hoplite, CounterKind::P1P1), 1);
}

static ZADA_LIKE: &[AbilityDef] = &[triggered!(
    Trigger::SpellCastTargeting {
        who: PlayerRel::You,
        spell: &Filter::Any,
        target: Some(&Filter::YOUR_CREATURE),
        targets_you: false,
        only: true,
    },
    &[Effect::GainLife {
        amount: Amount::Fixed(3),
    }]
)];

static FEATHER_LIKE: &[AbilityDef] = &[triggered!(
    Trigger::SpellCastTargeting {
        who: PlayerRel::You,
        spell: &Filter::Any,
        target: Some(&Filter::YOUR_CREATURE),
        targets_you: false,
        only: false,
    },
    &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }]
)];

static RAISE_DEAD_SPELL: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::GraveyardToHand {
        target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
    }],
    targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
        &Filter::CREATURE,
        PlayerRel::You,
    ))),
    second_targets: None,
    third_targets: None,
    condition: None,
}];

#[test]
fn spell_cast_targeting_enforces_battlefield_zone_and_only_single_target() {
    let mut engine = bench(
        5,
        vec![
            card(
                CARD_A,
                creature_face("ZadaWatcher", "{0}", 2, 2),
                KeywordSet::EMPTY,
                ZADA_LIKE,
            ),
            card(
                CARD_B,
                creature_face("FeatherWatcher", "{0}", 2, 2),
                KeywordSet::EMPTY,
                FEATHER_LIKE,
            ),
            card(
                CARD_C,
                creature_face("DeadBear", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_D,
                face("Kill", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                DESTROY_CREATURE,
            ),
            card(
                CARD_E,
                face("Raise", "{0}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                RAISE_DEAD_SPELL,
            ),
            card(
                CARD_F,
                face("Dual", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                DUAL_TARGET_SPELL,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B, CARD_C])
                .holding(&[CARD_D, CARD_E, CARD_F, CARD_F])
                .at(20),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    let zada = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let feather = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let bear = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let kill = the(&engine, ZoneLocation::Hand(me()), CARD_D);
    let raise = the(&engine, ZoneLocation::Hand(me()), CARD_E);
    let duals = objects(&engine, ZoneLocation::Hand(me()), CARD_F);

    // 1) Kill DeadBear (single battlefield target -> both Zada +3 and Feather +1 fire = 24 life).
    cast_zero_targeting(&mut engine, me(), kill, &[bear]);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);

    // 2) Raise targeting DeadBear in graveyard -> NEITHER fires (CR 109.2)!
    cast_zero_targeting(&mut engine, me(), raise, &[bear]);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);

    // 3) Dual targeting two distinct creatures (Zada + Feather) -> Feather (+1) fires, Zada does NOT!
    cast_zero_targeting(&mut engine, me(), duals[0], &[zada, feather]);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 25);

    // 4) Dual targeting ONLY Zada in both slots -> BOTH Zada (+3) and Feather (+1) fire (= 29)!
    cast_zero_targeting(&mut engine, me(), duals[1], &[zada, zada]);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 29);
}

// =========================================================================
// 4. Enlist (CR 702.154a–d, CR 508.1g)
// =========================================================================

static ENLIST_PAYOFF: &[AbilityDef] = &[triggered!(
    Trigger::ENLISTED_THIS,
    &[Effect::GainLife {
        amount: Amount::Fixed(2),
    }]
)];

static GRANT_ENLIST: &[AbilityDef] = &[static_ability!(
    Filter::YOUR_CREATURE,
    Modifier::AddKeyword(KeywordSet::ENLIST)
)];

static MANA_DORK: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

fn enlist_test_cards() -> Vec<&'static CardDef> {
    vec![
        card(
            CARD_A,
            creature_face("Enlister", "{0}", 2, 2),
            KeywordSet::ENLIST,
            ENLIST_PAYOFF,
        ),
        card(
            CARD_B,
            creature_face("Wall3_4", "{0}", 3, 4),
            KeywordSet::DEFENDER,
            &[],
        ),
        card(
            CARD_C,
            creature_face("TappedBear", "{0}", 4, 4),
            KeywordSet::EMPTY,
            MANA_DORK,
        ),
        card(
            CARD_D,
            creature_face("SickLion", "{0}", 5, 5),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CARD_E,
            creature_face("FellowAtk", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CARD_F,
            face("Growth", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            PUMP_TARGET_2_2,
        ),
    ]
}

#[test]
fn enlist_filters_helpers_taps_defender_and_reads_pumped_power_on_resolution() {
    let mut engine = bench(
        6,
        enlist_test_cards(),
        [
            Seat::with(&[CARD_A, CARD_B, CARD_C, CARD_E])
                .holding(&[CARD_D, CARD_F])
                .at(20),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    let enlister = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let wall = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let tapped = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let fellow = the(&engine, ZoneLocation::Battlefield, CARD_E);
    let sick_card = the(&engine, ZoneLocation::Hand(me()), CARD_D);
    let growth = the(&engine, ZoneLocation::Hand(me()), CARD_F);

    // Cast SickLion this turn (summoning sick!) and tap TappedBear for mana.
    engine
        .apply(me(), PlayerAction::CastSpell { card: sick_card })
        .unwrap();
    settle(&mut engine, me());
    activate(&mut engine, me(), tapped, 0);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![
            (enlister, Defender::Player(them())),
            (fellow, Defender::Player(them())),
        ],
    );
    let Pending::ChooseCards {
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::CostTap,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected Enlist CostTap prompt, got {:?}", engine.pending());
    };
    // Only Wall3_4 (Defender, controlled since turn began) is eligible!
    assert_eq!(options, vec![wall]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();
    assert!(
        engine
            .state()
            .object(wall)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Pump Wall3_4 (+2/+2 -> power 5) in response to the Enlist trigger (CR 608.2h)!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    cast_zero_targeting(&mut engine, me(), growth, &[wall]);
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 22);
    assert_eq!(pt(&engine, enlister), (7, 2));
}

#[test]
fn enlist_multiple_instances_function_independently_and_skip_empty_board() {
    // Part A: solo Enlist attacker with no helpers skips prompt completely.
    let mut solo = bench(
        7,
        vec![card(
            CARD_A,
            creature_face("SoloEnlist", "{0}", 2, 2),
            KeywordSet::ENLIST,
            &[],
        )],
        [Seat::with(&[CARD_A]), Seat::with(&[])],
    );
    let a = the(&solo, ZoneLocation::Battlefield, CARD_A);
    declare_attackers_step(&mut solo, me(), vec![(a, Defender::Player(them()))]);
    assert!(
        !matches!(solo.pending(), Pending::ChooseCards { .. }),
        "no helper -> must auto-skip empty enlist prompt"
    );

    // Part B: 2 instances of Enlist (printed + granted, CR 702.154d) with 2 helpers:
    // paying the first (power 3) and declining the second (`[]`) pumps +3/+0 once!
    let mut dual = bench(
        8,
        vec![
            card(
                CARD_A,
                creature_face("DualEnlist", "{0}", 2, 2),
                KeywordSet::ENLIST,
                &[],
            ),
            card(
                CARD_B,
                creature_face("Helper3", "{0}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Helper4", "{0}", 4, 4),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_D,
                face("Banner", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                GRANT_ENLIST,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B, CARD_C, CARD_D]),
            Seat::with(&[]),
        ],
    );
    let attacker = the(&dual, ZoneLocation::Battlefield, CARD_A);
    let h3 = the(&dual, ZoneLocation::Battlefield, CARD_B);
    let h4 = the(&dual, ZoneLocation::Battlefield, CARD_C);
    declare_attackers_step(&mut dual, me(), vec![(attacker, Defender::Player(them()))]);

    dual.apply(me(), PlayerAction::ChooseObjects { objects: vec![h3] })
        .expect("pay first enlist instance");
    let Pending::ChooseCards { options, .. } = dual.pending().clone() else {
        panic!("expected second enlist prompt");
    };
    assert_eq!(
        options,
        vec![h4],
        "already-tapped h3 cannot pay second enlist"
    );
    dual.apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .expect("decline second enlist instance");
    settle(&mut dual, me());
    assert_eq!(pt(&dual, attacker), (5, 2));
}

// =========================================================================
// 5. Bolster (CR 701.39a, CR 120.6)
// =========================================================================

static BOLSTER_2: &[AbilityDef] = &[free(&[Effect::Bolster(Amount::Fixed(2))], None)];
static BOLSTER_1: &[AbilityDef] = &[free(&[Effect::Bolster(Amount::Fixed(1))], None)];

fn ping_self_twice(engine: &mut Bench, id: ObjectId) {
    for _ in 0..2 {
        activate(engine, me(), id, 0);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![id],
                    players: vec![],
                },
            )
            .unwrap();
        settle(engine, me());
    }
}

#[test]
fn bolster_auto_picks_least_toughness_ignoring_damage_and_shroud_and_prompts_on_tie() {
    let mut engine = bench(
        9,
        vec![
            card(
                CARD_A,
                face("Relic2", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                BOLSTER_2,
            ),
            card(
                CARD_B,
                face("Relic1", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                BOLSTER_1,
            ),
            card(
                CARD_C,
                creature_face("Damaged4_5", "{0}", 4, 5),
                KeywordSet::EMPTY,
                PING_CREATURE_2,
            ),
            card(
                CARD_D,
                creature_face("Shroud2_2", "{0}", 2, 2),
                KeywordSet::SHROUD,
                &[],
            ),
            card(
                CARD_E,
                creature_face("Vanilla4_4", "{0}", 4, 4),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_G,
                creature_face("Enemy1_1", "{0}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B, CARD_C, CARD_D, CARD_E]),
            Seat::with(&[CARD_G]),
        ],
    );
    to_main(&mut engine, me());
    let relic2 = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let relic1 = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let damaged_4_5 = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let shroud_2_2 = the(&engine, ZoneLocation::Battlefield, CARD_D);
    let vanilla_4_4 = the(&engine, ZoneLocation::Battlefield, CARD_E);
    let enemy_1_1 = the(&engine, ZoneLocation::Battlefield, CARD_G);

    // Deal 4 damage to our 4/5: CR 120.6 says damage does NOT reduce toughness!
    ping_self_twice(&mut engine, damaged_4_5);
    assert_eq!(engine.state().object(damaged_4_5).unwrap().damage, 4);

    // 1) Bolster 2 ignores opponent's 1/1 and our damaged 4/5 (toughness 5),
    //    auto-picking Shroud2_2 without targeting or prompting -> becomes 4/4!
    activate(&mut engine, me(), relic2, 0);
    settle(&mut engine, me());
    assert_eq!(counters(&engine, shroud_2_2, CounterKind::P1P1), 2);
    assert_eq!(counters(&engine, enemy_1_1, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, shroud_2_2), (4, 4));

    // 2) Now Shroud2_2 (4/4) and Vanilla4_4 (4/4) tie for least toughness (4 < 5):
    //    Bolster 1 prompts ChooseCards over [shroud_2_2, vanilla_4_4]!
    activate(&mut engine, me(), relic1, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        min: 1,
        max: 1,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected Bolster tie prompt");
    };
    assert_eq!(options, vec![shroud_2_2, vanilla_4_4]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![vanilla_4_4],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(counters(&engine, vanilla_4_4, CounterKind::P1P1), 1);
}
