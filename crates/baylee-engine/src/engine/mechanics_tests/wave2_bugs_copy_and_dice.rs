//! Regression and rule tests for Wave 2 engine resolution/layer bug fixes
//! (multi-target `ExileLinked`, multi-target `Blink`, slot-isolated `TapTarget` /
//! `TapTargetSlot`, ETB + LKI `Amount::EventLastToughness`, per-creature
//! `SharesCreatureTypeWith(&Filter::This)` in `count_controlled`, and
//! `Trigger::LastCounterRemoved` with `by: None` vs `Some(You)`) plus P10 Copy
//! (`CR 707`) and P11 Coin Flip & Dice Roll (`CR 705–706`).
#![allow(clippy::too_many_lines)] // Scenario tests keep positive and negative assertions side by side.

use super::*;
use baylee_cards_dsl::{
    Amount, CopyCleanup, CopyMod, CounterKind, EnterModifier, Filter, Modifier, PlayerRel, PtCount,
    TargetSlot, TargetSpec, Trigger, spell, static_ability, triggered,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;

const HOST_EXILER: u32 = 99_700;
const RELIC_A: u32 = 99_701;
const RELIC_B: u32 = 99_702;
const SAC_HOST: u32 = 99_703;
const BLINKER: u32 = 99_704;
const ETB_PINGER: u32 = 99_705;
const DUAL_TAP_BUG_PROBE: u32 = 99_706;
const ARENA_PROBE: u32 = 99_707;
const BEAR_A: u32 = 99_708;
const BEAR_B: u32 = 99_709;
const CHORUS_ENCHANTMENT: u32 = 99_710;
const WALL_2_5: u32 = 99_711;
const INSTANT_KILL: u32 = 99_712;
const DIES_TOUGHNESS_WATCHER: u32 = 99_713;
const COAT_ARTIFACT: u32 = 99_714;
const ALPHA_AURA: u32 = 99_715;
const GOBLIN_WARRIOR_1: u32 = 99_716;
const GOBLIN_WARRIOR_2: u32 = 99_717;
const GOBLIN_SHAMAN: u32 = 99_718;
const LONE_ELF: u32 = 99_719;
const VANISHING_PROBE: u32 = 99_720;
const DIVINE_PROBE: u32 = 99_721;
const COUNTER_STRIPPER: u32 = 99_722;
const MINUS_PLACER: u32 = 99_723;
const TOKEN_COPIER: u32 = 99_724;
const CHOOSE_COPIER: u32 = 99_725;
const ORIGINAL_OGRE: u32 = 99_726;
const CLONE_SHIFTER: u32 = 99_727;
const GRAVE_DRAGON: u32 = 99_728;
const STACK_FORKER: u32 = 99_729;
const BOLT_SPELL: u32 = 99_730;
const COIN_RIG: u32 = 99_731;
const THUMB_ARTIFACT: u32 = 99_732;
const COIN_OBSERVER: u32 = 99_733;
const DICE_RIG: u32 = 99_734;
const PIXIE_ADVANTAGE: u32 = 99_735;
const DICE_OBSERVER: u32 = 99_736;

fn cast_spell_from_hand(engine: &mut Bench, seat: PlayerId, idx: u32) -> ObjectId {
    let found = objects(engine, ZoneLocation::Hand(seat), idx);
    assert!(
        !found.is_empty(),
        "expected at least one {idx} in {seat:?}'s hand"
    );
    let id = found[0];
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(
        legal.castable.contains(&id),
        "expected {idx} to be castable"
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: id })
        .expect("cast succeeds");
    id
}

/// Bug 1 (`Effect::ExileLinked`): exiles all resolved targets (not just `first()`)
/// and returns all linked cards when the host leaves the battlefield (`Petradon`).
#[test]
fn exile_linked_exiles_all_resolved_targets_and_returns_all_on_departure() {
    static HOST_ABILITIES: &[AbilityDef] = &[
        triggered!(
            Trigger::ETB,
            &[Effect::exile_linked(TargetSpec::Object(&Filter::ARTIFACT))],
            targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::ARTIFACT), 2)),
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedToBattlefield],
        ),
    ];
    static SAC_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            HOST_EXILER,
            creature_face("Host Exiler", "{0}", 4, 4),
            KeywordSet::EMPTY,
            HOST_ABILITIES,
        ),
        card(
            RELIC_A,
            face("Relic A", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            RELIC_B,
            face("Relic B", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SAC_HOST,
            face("Host Breaker", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            SAC_ABILITIES,
        ),
    ];

    let mut engine = bench(
        1,
        cards,
        [
            Seat::with(&[RELIC_A, SAC_HOST]).holding(&[HOST_EXILER]),
            Seat::with(&[RELIC_B]),
        ],
    );
    to_main(&mut engine, me());
    let relic_a = the(&engine, ZoneLocation::Battlefield, RELIC_A);
    let relic_b = the(&engine, ZoneLocation::Battlefield, RELIC_B);
    cast_spell_from_hand(&mut engine, me(), HOST_EXILER);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic_a, relic_b],
                players: vec![],
            },
        )
        .expect("choose both artifacts");
    settle(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), RELIC_A).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(them()), RELIC_B).len(),
        1
    );
    assert!(objects(&engine, ZoneLocation::Battlefield, RELIC_A).is_empty());
    assert!(objects(&engine, ZoneLocation::Battlefield, RELIC_B).is_empty());

    let host = the(&engine, ZoneLocation::Battlefield, HOST_EXILER);
    let breaker = the(&engine, ZoneLocation::Battlefield, SAC_HOST);
    activate(&mut engine, me(), breaker, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![host],
                players: vec![],
            },
        )
        .expect("target host");
    settle(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, RELIC_A).len(),
        1
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, RELIC_B).len(),
        1
    );
}

/// Bug 2 (`Effect::Blink`): exiles all resolved targets before returning any,
/// resetting counters/versions and firing ETB for each blinked permanent.
#[test]
fn blink_exiles_multiple_targets_before_returning_any() {
    static BLINK_TWO: &[AbilityDef] = &[free(
        &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        Some(TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2)),
    )];
    static PINGER_ETB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }],
    )];

    let cards = vec![
        card(
            BLINKER,
            face("Blinker", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BLINK_TWO,
        ),
        card(
            ETB_PINGER,
            FaceDef {
                enter_modifiers: &[EnterModifier::WithCounters {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                }],
                ..creature_face("ETB Pinger", "{0}", 2, 2)
            },
            KeywordSet::EMPTY,
            PINGER_ETB,
        ),
    ];

    let mut engine = bench(
        2,
        cards,
        [
            Seat::with(&[BLINKER, ETB_PINGER, ETB_PINGER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let before = objects(&engine, ZoneLocation::Battlefield, ETB_PINGER);
    assert_eq!(before.len(), 2);
    let v0 = engine.state().object(before[0]).unwrap().version;
    let v1 = engine.state().object(before[1]).unwrap().version;
    let life_before = life(&engine, me());

    let blinker = the(&engine, ZoneLocation::Battlefield, BLINKER);
    activate(&mut engine, me(), blinker, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![before[0], before[1]],
                players: vec![],
            },
        )
        .expect("choose both creatures");
    settle(&mut engine, me());

    let after = objects(&engine, ZoneLocation::Battlefield, ETB_PINGER);
    assert_eq!(after.len(), 2);
    assert_ne!(engine.state().object(after[0]).unwrap().version, v0);
    assert_ne!(engine.state().object(after[1]).unwrap().version, v1);
    assert_eq!(life(&engine, me()), life_before + 4);
}

/// Bug 3 (`Effect::TapTarget` vs `Effect::TapTargetSlot`): `TapTarget` taps only
/// `targets` (leaving `second_targets` untouched for `Deadshot` / `Spirit Flare`),
/// whereas `TapTargetSlot(TargetSlot::Second)` explicitly taps the second slot (`Arena`).
#[test]
fn tap_target_isolates_first_slot_while_tap_target_slot_taps_second() {
    static DEADSHOT_LIKE: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost::FREE,
        effects: &[
            Effect::TapTarget,
            Effect::DamageEqualToPower {
                dealer: TargetSlot::First,
                to: TargetSlot::Second,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];
    static ARENA_LIKE: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost::FREE,
        effects: &[Effect::TapTarget, Effect::TapTargetSlot(TargetSlot::Second)],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        second_targets: Some(TargetReq::one(TargetSpec::Object(
            &Filter::OPPONENT_CREATURE,
        ))),
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];

    let cards = vec![
        card(
            DUAL_TAP_BUG_PROBE,
            face("Deadshot Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DEADSHOT_LIKE,
        ),
        card(
            ARENA_PROBE,
            face("Arena Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            ARENA_LIKE,
        ),
        card(
            BEAR_A,
            creature_face("Bear A", "{0}", 2, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            BEAR_B,
            creature_face("Bear B", "{0}", 2, 3),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        3,
        cards,
        [
            Seat::with(&[DUAL_TAP_BUG_PROBE, ARENA_PROBE, BEAR_A]),
            Seat::with(&[BEAR_B]),
        ],
    );
    to_main(&mut engine, me());
    let bear_a = the(&engine, ZoneLocation::Battlefield, BEAR_A);
    let bear_b = the(&engine, ZoneLocation::Battlefield, BEAR_B);
    let deadshot = the(&engine, ZoneLocation::Battlefield, DUAL_TAP_BUG_PROBE);

    activate(&mut engine, me(), deadshot, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a],
                players: vec![],
            },
        )
        .expect("first target");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b],
                players: vec![],
            },
        )
        .expect("second target");
    settle(&mut engine, me());

    assert!(
        engine
            .state()
            .object(bear_a)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert!(
        !engine
            .state()
            .object(bear_b)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "TapTarget must NOT tap second_targets"
    );
    assert_eq!(engine.state().object(bear_b).unwrap().damage, 2);

    // Now Arena-like explicit `TapTargetSlot(TargetSlot::Second)` taps Bear B too.
    let arena = the(&engine, ZoneLocation::Battlefield, ARENA_PROBE);
    activate(&mut engine, me(), arena, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a],
                players: vec![],
            },
        )
        .expect("first target");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b],
                players: vec![],
            },
        )
        .expect("second target");
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(bear_b)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

/// Bug 4 (`Amount::EventLastToughness`): reads battlefield toughness on ETB
/// (`Angelic Chorus`), falls back to LKI toughness if the entering creature is
/// killed in response (`CR 608.2h`), and still reads `Rider::EventDeparture` on `Dies`.
#[test]
fn event_last_toughness_reads_live_and_departed_etb_toughness_plus_dies() {
    static CHORUS_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::YOUR_CREATURE),
        &[Effect::GainLife {
            amount: Amount::EventLastToughness,
        }],
    )];
    static DIES_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::YOUR_CREATURE),
        &[Effect::GainLife {
            amount: Amount::EventLastToughness,
        }],
    )];
    static KILL_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            CHORUS_ENCHANTMENT,
            face("Chorus", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            CHORUS_ABILITIES,
        ),
        card(
            DIES_TOUGHNESS_WATCHER,
            face("Death Chorus", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DIES_WATCHER_ABILITIES,
        ),
        card(
            WALL_2_5,
            creature_face("Wall 2/5", "{0}", 2, 5),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            INSTANT_KILL,
            face("Instant Kill", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            KILL_SPELL,
        ),
    ];

    let mut engine = bench(
        4,
        cards,
        [
            Seat::with(&[CHORUS_ENCHANTMENT]).holding(&[
                WALL_2_5,
                WALL_2_5,
                INSTANT_KILL,
                DIES_TOUGHNESS_WATCHER,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let start_life = life(&engine, me());

    // 1. Live ETB: Wall 2/5 enters -> Chorus gains 5 life.
    cast_spell_from_hand(&mut engine, me(), WALL_2_5);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), start_life + 5);

    // 2. ETB with creature killed in response + Dies trigger reading departure toughness.
    cast_spell_from_hand(&mut engine, me(), DIES_TOUGHNESS_WATCHER);
    settle(&mut engine, me());

    cast_spell_from_hand(&mut engine, me(), WALL_2_5);
    // Step until Wall 2/5 resolves onto the battlefield and Chorus's ETB trigger is on the stack.
    walk_until(&mut engine, |e| {
        objects(e, ZoneLocation::Battlefield, WALL_2_5).len() == 2
            && !e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    let second_wall = objects(&engine, ZoneLocation::Battlefield, WALL_2_5)[1];
    cast_spell_from_hand(&mut engine, me(), INSTANT_KILL);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![second_wall],
                players: vec![],
            },
        )
        .expect("target newly entered wall in response to ETB trigger");
    settle(&mut engine, me());

    // Dies watcher gained +5 on death AND Chorus's ETB trigger still gained +5 from LKI!
    assert_eq!(life(&engine, me()), start_life + 15);
}

/// Bug 5 (`count_controlled` + `Filter::SharesCreatureTypeWith(&Filter::This)`):
/// rebinds `Filter::This` and `Filter::Another` to each affected creature for
/// `Coat of Arms` and `Alpha Status`.
#[test]
fn shares_creature_type_with_this_counts_per_affected_creature_on_coat_and_aura() {
    static COUNT_SHARED: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::SharesCreatureTypeWith(&Filter::This),
        Filter::Another,
    ]);
    static COAT_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::ModifyPTByCount(PtCount::OnBattlefield(&COUNT_SHARED))
    )];
    static ALPHA_ABILITIES: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPTByCount(PtCount::OnBattlefield(&COUNT_SHARED))
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPTByCount(PtCount::OnBattlefield(&COUNT_SHARED))
        ),
    ];

    static GW_SUBS: &[baylee_core::ids::SubtypeId] =
        &[subtypes::creature::GOBLIN, subtypes::creature::WARRIOR];
    static GS_SUBS: &[baylee_core::ids::SubtypeId] =
        &[subtypes::creature::GOBLIN, subtypes::creature::SHAMAN];
    static ELF_SUBS: &[baylee_core::ids::SubtypeId] = &[subtypes::creature::ELF];
    static AURA_SUBS: &[baylee_core::ids::SubtypeId] = &[subtypes::enchantment::AURA];

    let cards = vec![
        card(
            COAT_ARTIFACT,
            face("Coat Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            COAT_ABILITIES,
        ),
        card(
            ALPHA_AURA,
            FaceDef {
                subtypes: AURA_SUBS,
                ..face("Alpha Probe", "{0}", TypeSet::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            ALPHA_ABILITIES,
        ),
        card(
            GOBLIN_WARRIOR_1,
            FaceDef {
                subtypes: GW_SUBS,
                ..creature_face("Goblin Warrior 1", "{0}", 1, 1)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            GOBLIN_WARRIOR_2,
            FaceDef {
                subtypes: GW_SUBS,
                ..creature_face("Goblin Warrior 2", "{0}", 1, 1)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            GOBLIN_SHAMAN,
            FaceDef {
                subtypes: GS_SUBS,
                ..creature_face("Goblin Shaman", "{0}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            LONE_ELF,
            FaceDef {
                subtypes: ELF_SUBS,
                ..creature_face("Lone Elf", "{0}", 1, 1)
            },
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        5,
        cards,
        [
            Seat::with(&[COAT_ARTIFACT, GOBLIN_WARRIOR_1, LONE_ELF]).holding(&[ALPHA_AURA]),
            Seat::with(&[GOBLIN_WARRIOR_2, GOBLIN_SHAMAN]),
        ],
    );
    to_main(&mut engine, me());

    let gw1 = the(&engine, ZoneLocation::Battlefield, GOBLIN_WARRIOR_1);
    let gw2 = the(&engine, ZoneLocation::Battlefield, GOBLIN_WARRIOR_2);
    let gs = the(&engine, ZoneLocation::Battlefield, GOBLIN_SHAMAN);
    let elf = the(&engine, ZoneLocation::Battlefield, LONE_ELF);

    // Three Goblins share Goblin -> each gets +2/+2; Lone Elf shares with none -> +0/+0.
    assert_eq!(
        engine.state().object(gw1).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(
        engine.state().object(gw2).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(
        engine.state().object(gs).unwrap().characteristics().power,
        Some(4)
    );
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().power,
        Some(1)
    );

    // Enchant Goblin Warrior 1 with Alpha Status -> +4/+4 more (total 7/7).
    cast_spell_from_hand(&mut engine, me(), ALPHA_AURA);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![gw1],
                players: vec![],
            },
        )
        .expect("enchant gw1");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(gw1).unwrap().characteristics().power,
        Some(7)
    );
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().power,
        Some(1)
    );
}

/// Bug 6 (`Trigger::LastCounterRemoved`): `by: None` (`CR 702.63a` Vanishing)
/// fires on opponent removal and SBA annihilation, while `by: Some(PlayerRel::You)`
/// (`Divine Intervention`) fires only when the controller removes the last counter.
#[test]
fn last_counter_removed_distinguishes_any_remover_sba_from_controller_only() {
    static VANISHING_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::LastCounterRemoved {
            kind: CounterKind::P1P1,
            by: None,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
    )];
    static DIVINE_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::LastCounterRemoved {
            kind: CounterKind::Charge,
            by: Some(PlayerRel::You),
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(10),
        }],
    )];
    static STRIP_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::RemoveAllCountersTarget],
        Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )];
    static MINUS_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::AddCounter {
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            VANISHING_PROBE,
            FaceDef {
                enter_modifiers: &[EnterModifier::WithCounters {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                }],
                ..creature_face("Vanishing Probe", "{0}", 2, 2)
            },
            KeywordSet::EMPTY,
            VANISHING_ABILITIES,
        ),
        card(
            DIVINE_PROBE,
            FaceDef {
                enter_modifiers: &[EnterModifier::WithCounters {
                    kind: CounterKind::Charge,
                    amount: Amount::Fixed(1),
                }],
                ..face("Divine Probe", "{0}", TypeSet::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            DIVINE_ABILITIES,
        ),
        card(
            COUNTER_STRIPPER,
            face("Counter Stripper", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            STRIP_ABILITIES,
        ),
        card(
            MINUS_PLACER,
            face("Minus Placer", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            MINUS_ABILITIES,
        ),
    ];

    let mut engine = bench(
        6,
        cards,
        [
            Seat::with(&[MINUS_PLACER]).holding(&[VANISHING_PROBE, DIVINE_PROBE, DIVINE_PROBE]),
            Seat::with(&[COUNTER_STRIPPER]),
        ],
    );
    to_main(&mut engine, me());
    let start_life = life(&engine, me());

    // 1. SBA +1/+1 & -1/-1 annihilation (`by: None` in event) triggers `LastCounterRemoved { by: None }`!
    cast_spell_from_hand(&mut engine, me(), VANISHING_PROBE);
    settle(&mut engine, me());
    let v_probe = the(&engine, ZoneLocation::Battlefield, VANISHING_PROBE);
    let minus = the(&engine, ZoneLocation::Battlefield, MINUS_PLACER);
    activate(&mut engine, me(), minus, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![v_probe],
                players: vec![],
            },
        )
        .expect("target vanishing probe");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), start_life + 3);

    // 2. Opponent removing Charge counter from `DIVINE_PROBE` (`by: Some(You)`) does NOT trigger!
    cast_spell_from_hand(&mut engine, me(), DIVINE_PROBE);
    settle(&mut engine, me());
    let d_probe = the(&engine, ZoneLocation::Battlefield, DIVINE_PROBE);
    // Pass priority to opponent so opponent activates `COUNTER_STRIPPER`.
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let opp_stripper = the(&engine, ZoneLocation::Battlefield, COUNTER_STRIPPER);
    activate(&mut engine, them(), opp_stripper, 0);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![d_probe],
                players: vec![],
            },
        )
        .expect("opponent strips charge counter");
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        start_life + 3,
        "opponent removal must NOT trigger `by: Some(PlayerRel::You)`"
    );
}

/// P10 (`Effect::CreateTokenCopy`): applies `CopyMod::{SetName, SetManaCost, AddColor}`,
/// `haste_until_eot`, and `CopyCleanup::ExileAtNextEndStep`.
#[test]
fn create_token_copy_applies_copy_mods_haste_and_exile_cleanup() {
    static COPY_MODS: &[CopyMod] = &[
        CopyMod::SetName("Mirage Twin"),
        CopyMod::SetManaCost(baylee_core::mana!("{3}{R}")),
        CopyMod::AddColor(ColorSet::of(Color::Blue)),
    ];
    static COPIER_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::CreateTokenCopy {
            target: TargetSpec::Object(&Filter::CREATURE),
            controller: PlayerRel::You,
            amount: Amount::Fixed(2),
            mods: COPY_MODS,
            tapped: false,
            attacking: false,
            haste_until_eot: true,
            cleanup: CopyCleanup::ExileAtNextEndStep,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            TOKEN_COPIER,
            face("Twin Maker", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            COPIER_ABILITIES,
        ),
        card(
            ORIGINAL_OGRE,
            creature_face("Original Ogre", "{1}{R}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        7,
        cards,
        [Seat::with(&[TOKEN_COPIER, ORIGINAL_OGRE]), Seat::default()],
    );
    to_main(&mut engine, me());
    let copier = the(&engine, ZoneLocation::Battlefield, TOKEN_COPIER);
    let ogre = the(&engine, ZoneLocation::Battlefield, ORIGINAL_OGRE);
    activate(&mut engine, me(), copier, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .expect("target ogre");
    settle(&mut engine, me());

    let twin_name = engine.state().names.find("Mirage Twin");
    let tokens: Vec<ObjectId> = engine
        .state()
        .battlefield_view()
        .into_iter()
        .filter(|&id| !engine.state().object(id).unwrap().is_card())
        .collect();
    assert_eq!(tokens.len(), 2);
    for &tok in &tokens {
        let chars = engine.state().object(tok).unwrap().characteristics();
        assert_eq!(Some(chars.name), twin_name);
        assert_eq!(chars.mana_value(), 4);
        assert!(chars.colors.contains(Color::Blue));
        assert!(chars.keywords.contains(KeywordSet::HASTE));
    }

    // Advance to the next turn's main phase: both tokens were exiled at the end step.
    to_main(&mut engine, them());
    let remaining_tokens = engine
        .state()
        .battlefield_view()
        .into_iter()
        .filter(|&id| !engine.state().object(id).unwrap().is_card())
        .count();
    assert_eq!(remaining_tokens, 0);
}

/// P10 (`Effect::ChooseAndCreateTokenCopy`): prompts when multiple permanents
/// match, enters attacking during combat, and sacrifices at end of combat.
#[test]
fn choose_and_create_token_copy_prompts_and_sacrifices_at_end_of_combat() {
    static CHOOSE_COPY_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::ChooseAndCreateTokenCopy {
            filter: &Filter::YOUR_CREATURE,
            amount: Amount::Fixed(1),
            mods: &[],
            tapped: true,
            attacking: true,
            haste_until_eot: false,
            cleanup: CopyCleanup::SacrificeAtEndOfCombat,
        }],
        None,
    )];

    let cards = vec![
        card(
            CHOOSE_COPIER,
            face("Combat Cloner", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            CHOOSE_COPY_ABILITIES,
        ),
        card(
            BEAR_A,
            creature_face("Small Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ORIGINAL_OGRE,
            creature_face("Big Ogre", "{0}", 4, 4),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        8,
        cards,
        [
            Seat::with(&[CHOOSE_COPIER, BEAR_A, ORIGINAL_OGRE]),
            Seat::default(),
        ],
    );
    // Walk to Turn 3 combat (turn 1 cannot attack due to summoning sickness!) or give Bear A haste!
    to_main(&mut engine, them());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    let bear = the(&engine, ZoneLocation::Battlefield, BEAR_A);
    let ogre = the(&engine, ZoneLocation::Battlefield, ORIGINAL_OGRE);
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, baylee_core::ids::Defender::Player(them()))],
            },
        )
        .expect("declare bear attacker");
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::DeclareAttackers)
    });

    let cloner = the(&engine, ZoneLocation::Battlefield, CHOOSE_COPIER);
    activate(&mut engine, me(), cloner, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .expect("choose Big Ogre to copy");
    settle(&mut engine, me());

    // Token copy is tapped and attacking `them()` -> deals 2 + 4 = 6 combat damage,
    // then sacrifices at End of Combat before PostCombatMain!
    let opp_life_before = life(&engine, them());
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(life(&engine, them()), opp_life_before - 6);
    let remaining_tokens = engine
        .state()
        .battlefield_view()
        .into_iter()
        .filter(|&id| !engine.state().object(id).unwrap().is_card())
        .count();
    assert_eq!(remaining_tokens, 0);
}

/// P10 (`Effect::BecomeCopy`): copies graveyard creature card with `CopyMod::KeepName`
/// (`Lazav`) and supports `subject_filter` (`Mirrorweave`) until end of turn.
#[test]
fn become_copy_handles_graveyard_card_with_keep_name_and_filtered_subjects() {
    static LAZAV_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::BecomeCopy {
            subject: TargetSlot::This,
            subject_filter: None,
            original: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            original_is_second_target: false,
            until_eot: false,
            mods: &[CopyMod::KeepName],
        }],
        Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )];
    static MIRRORWEAVE_BEAR: &[AbilityDef] = &[
        free(
            &[Effect::DiscardHand {
                who: PlayerRel::You,
            }],
            None,
        ),
        free(
            &[Effect::BecomeCopy {
                subject: TargetSlot::First,
                subject_filter: Some(&Filter::CREATURE),
                original: TargetSpec::Object(&Filter::CREATURE),
                original_is_second_target: false,
                until_eot: true,
                mods: &[],
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];

    let cards = vec![
        card(
            CLONE_SHIFTER,
            creature_face("Shifter", "{0}", 1, 3),
            KeywordSet::EMPTY,
            LAZAV_ABILITIES,
        ),
        card(
            GRAVE_DRAGON,
            creature_face("Grave Dragon", "{4}{R}{R}", 6, 6),
            KeywordSet::FLYING,
            &[],
        ),
        card(
            BEAR_A,
            creature_face("Bystander Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            MIRRORWEAVE_BEAR,
        ),
    ];

    let mut engine = bench(
        9,
        cards,
        [
            Seat::with(&[CLONE_SHIFTER, BEAR_A]).holding(&[GRAVE_DRAGON]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let bear = the(&engine, ZoneLocation::Battlefield, BEAR_A);
    let shifter = the(&engine, ZoneLocation::Battlefield, CLONE_SHIFTER);
    let shifter_name = engine
        .state()
        .object(shifter)
        .unwrap()
        .characteristics()
        .name;

    // Put Grave Dragon into graveyard.
    activate(&mut engine, me(), bear, 0);
    settle(&mut engine, me());
    let dragon_gy = the(&engine, ZoneLocation::Graveyard(me()), GRAVE_DRAGON);

    // 1. Shifter becomes a 6/6 flying copy of Grave Dragon, keeping name "Shifter"!
    activate(&mut engine, me(), shifter, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![dragon_gy],
                players: vec![],
            },
        )
        .expect("target graveyard dragon");
    settle(&mut engine, me());

    let chars = engine.state().object(shifter).unwrap().characteristics();
    assert_eq!(chars.name, shifter_name);
    assert_eq!(chars.power, Some(6));
    assert_eq!(chars.toughness, Some(6));
    assert!(chars.keywords.contains(KeywordSet::FLYING));

    // 2. Mirrorweave-like `subject_filter` targeting Shifter: Bear becomes a 6/6 flying copy until EOT!
    activate(&mut engine, me(), bear, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![shifter],
                players: vec![],
            },
        )
        .expect("target shifter as Mirrorweave template");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(bear).unwrap().characteristics().power,
        Some(6)
    );
    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    // At next turn, Bear reverts to 2/2 while Shifter's indefinite copy stays 6/6!
    to_main(&mut engine, them());
    assert_eq!(
        engine.state().object(bear).unwrap().characteristics().power,
        Some(2)
    );
    assert_eq!(
        engine
            .state()
            .object(shifter)
            .unwrap()
            .characteristics()
            .power,
        Some(6)
    );
}

/// P10 (`Effect::CopySpellOrAbility`): copies a spell multiple times (`amount: Fixed(2)`)
/// both keeping targets and choosing new targets (`CR 707.10c`).
#[test]
fn copy_spell_or_ability_copies_multiple_times_on_stack() {
    static BOLT_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )];
    static FORK_TWICE_ABILITIES: &[AbilityDef] = &[
        free(
            &[Effect::CopySpellOrAbility {
                target: TargetSpec::Spell(&Filter::Any),
                controller: PlayerRel::You,
                amount: Amount::Fixed(2),
                mods: &[],
                may_choose_new_targets: false,
            }],
            Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        ),
        free(
            &[Effect::CopySpellOrAbility {
                target: TargetSpec::Spell(&Filter::Any),
                controller: PlayerRel::You,
                amount: Amount::Fixed(1),
                mods: &[],
                may_choose_new_targets: true,
            }],
            Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        ),
    ];

    let cards = vec![
        card(
            BOLT_SPELL,
            face("Bolt", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BOLT_ABILITIES,
        ),
        card(
            STACK_FORKER,
            face("Double Forker", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            FORK_TWICE_ABILITIES,
        ),
        card(
            BEAR_B,
            creature_face("Targetable Bear", "{0}", 2, 3),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        10,
        cards,
        [
            Seat::with(&[STACK_FORKER]).holding(&[BOLT_SPELL, BOLT_SPELL]),
            Seat::with(&[BEAR_B]),
        ],
    );
    to_main(&mut engine, me());
    let opp_start = life(&engine, them());

    let bolt = cast_spell_from_hand(&mut engine, me(), BOLT_SPELL);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("aim bolt at opponent");

    let forker = the(&engine, ZoneLocation::Battlefield, STACK_FORKER);
    activate(&mut engine, me(), forker, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bolt],
                players: vec![],
            },
        )
        .expect("target bolt on stack");
    settle(&mut engine, me());

    // Original Bolt (3) + 2 stack copies (6) = 9 total damage!
    assert_eq!(life(&engine, them()), opp_start - 9);

    // Second Bolt: copy once with `may_choose_new_targets: true` and redirect copy to Bear B!
    let bear_b = the(&engine, ZoneLocation::Battlefield, BEAR_B);
    let bolt2 = cast_spell_from_hand(&mut engine, me(), BOLT_SPELL);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("aim bolt2 at opponent");
    activate(&mut engine, me(), forker, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bolt2],
                players: vec![],
            },
        )
        .expect("target bolt2");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b],
                players: vec![],
            },
        )
        .expect("retarget copy onto Bear B");
    settle(&mut engine, me());

    assert!(objects(&engine, ZoneLocation::Battlefield, BEAR_B).is_empty());
    assert_eq!(life(&engine, them()), opp_start - 12);
}

/// P11 (`Effect::FlipCoins`, `Effect::FlipUntilLose`, `Amount::CoinsWonThisResolution`,
/// `Trigger::CoinFlipped`, `Modifier::FlipTwoCoinsIgnoreOne`): deterministic multi-flip
/// resolution and coin-flip triggers.
#[test]
fn flip_coins_and_flip_until_lose_track_wins_and_fire_coin_flipped_triggers() {
    static COIN_RIG_ABILITIES: &[AbilityDef] = &[
        free(
            &[Effect::FlipCoins {
                count: Amount::Fixed(3),
                stop_on_loss: false,
                per_win: &[Effect::GainLife {
                    amount: Amount::Fixed(2),
                }],
                if_won_all: &[Effect::GainLife {
                    amount: Amount::Fixed(10),
                }],
            }],
            None,
        ),
        free(
            &[Effect::FlipUntilLose {
                max_flips: Some(4),
                per_win: &[],
                after: &[Effect::GainLife {
                    amount: Amount::CoinsWonThisResolution,
                }],
            }],
            None,
        ),
    ];
    static THUMB_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::FlipTwoCoinsIgnoreOne
    )];
    static COIN_OBS_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::CoinFlipped {
            who: PlayerRel::You,
            outcome: Some(true),
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
    )];

    let cards = vec![
        card(
            COIN_RIG,
            face("Coin Rig", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            COIN_RIG_ABILITIES,
        ),
        card(
            THUMB_ARTIFACT,
            face("Thumb", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            THUMB_ABILITIES,
        ),
        card(
            COIN_OBSERVER,
            face("Coin Observer", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            COIN_OBS_ABILITIES,
        ),
    ];

    // Find a seed where all 3 flips in ability 0 can be won with Thumb choosing winning coins!
    let mut won_all_seen = false;
    for seed in 1..=30 {
        let mut engine = bench(
            seed,
            cards.clone(),
            [
                Seat::with(&[COIN_RIG, THUMB_ARTIFACT, COIN_OBSERVER]),
                Seat::default(),
            ],
        );
        to_main(&mut engine, me());
        let start_life = life(&engine, me());
        let rig = the(&engine, ZoneLocation::Battlefield, COIN_RIG);
        let obs = the(&engine, ZoneLocation::Battlefield, COIN_OBSERVER);

        activate(&mut engine, me(), rig, 0);
        // Answer every Thumb `YesNo` by inspecting the latest two `CoinFlipped` journal events
        // and keeping a `true` when present!
        while !engine.state().zones.stack_is_empty() {
            match engine.pending().clone() {
                Pending::YesNo { player, .. } => {
                    let recent: Vec<bool> = engine
                        .journal()
                        .entries()
                        .iter()
                        .rev()
                        .filter_map(|e| match e.event {
                            crate::event::GameEvent::CoinFlipped { won, .. } => Some(won),
                            _ => None,
                        })
                        .take(2)
                        .collect();
                    // `recent[0]` is second flip, `recent[1]` is first flip.
                    // `YesNo(true)` ignores first flip (`recent[1]`), keeping `recent[0]`.
                    let ignore_first = recent.first().copied().unwrap_or(false);
                    engine
                        .apply(player, PlayerAction::YesNo(ignore_first))
                        .unwrap();
                }
                pending => assert!(walk_past(&mut engine, &pending)),
            }
        }
        settle(&mut engine, me());
        let gained = life(&engine, me()) - start_life;
        let obs_triggers = triggered(&engine, obs);
        assert!(obs_triggers > 0 || gained == 0);
        if gained >= 16 + obs_triggers as i32 {
            // 3 wins * 2 life + 10 `if_won_all` + `obs_triggers` * 1 life!
            assert_eq!(gained, 16 + obs_triggers as i32);
            won_all_seen = true;

            // Also verify ability 1 (`FlipUntilLose` + `Amount::CoinsWonThisResolution`).
            let before_until = life(&engine, me());
            activate(&mut engine, me(), rig, 1);
            while !engine.state().zones.stack_is_empty() {
                match engine.pending().clone() {
                    Pending::YesNo { player, .. } => {
                        let recent: Vec<bool> = engine
                            .journal()
                            .entries()
                            .iter()
                            .rev()
                            .filter_map(|e| match e.event {
                                crate::event::GameEvent::CoinFlipped { won, .. } => Some(won),
                                _ => None,
                            })
                            .take(2)
                            .collect();
                        let ignore_first = recent.first().copied().unwrap_or(false);
                        engine
                            .apply(player, PlayerAction::YesNo(ignore_first))
                            .unwrap();
                    }
                    pending => assert!(walk_past(&mut engine, &pending)),
                }
            }
            settle(&mut engine, me());
            assert!(life(&engine, me()) >= before_until);
            break;
        }
    }
    assert!(
        won_all_seen,
        "expected at least one 3-of-3 win with Thumb across seeds 1..=30"
    );
}

/// P11 (`Effect::RollDice`, `Effect::IfDieRollInRange`, `Amount::LastDieRoll`,
/// `Modifier::RollAdditionalDieIgnoreLowest`, `Trigger::DieRolled`, `CR 706.6`):
/// rolls dice deterministically, ignores lowest roll without triggering on the
/// ignored die, populates `Amount::LastDieRoll`, and branches on `IfDieRollInRange`.
#[test]
fn roll_dice_if_in_range_and_ignore_lowest_modifier_follow_cr_706() {
    static DICE_RIG_ABILITIES: &[AbilityDef] = &[free(
        &[
            Effect::RollDice {
                count: Amount::Fixed(1),
                sides: 20,
                modifier: 2,
            },
            Effect::GainLife {
                amount: Amount::LastDieRoll,
            },
            Effect::IfDieRollInRange {
                min: 1,
                max: 11,
                then: &[Effect::GainLife {
                    amount: Amount::Fixed(100),
                }],
                otherwise: &[Effect::IfDieRollInRange {
                    min: 12,
                    max: 22,
                    then: &[Effect::GainLife {
                        amount: Amount::Fixed(200),
                    }],
                    otherwise: &[],
                }],
            },
        ],
        None,
    )];
    static PIXIE_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::RollAdditionalDieIgnoreLowest
    )];
    static DICE_OBS_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::DieRolled {
            who: PlayerRel::You,
            min_result: None,
        },
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }],
    )];

    let cards = vec![
        card(
            DICE_RIG,
            face("d20 Rig", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DICE_RIG_ABILITIES,
        ),
        card(
            PIXIE_ADVANTAGE,
            face("Pixie Advantage", "{0}", TypeSet::CREATURE),
            KeywordSet::EMPTY,
            PIXIE_ABILITIES,
        ),
        card(
            DICE_OBSERVER,
            face("Die Observer", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DICE_OBS_ABILITIES,
        ),
    ];

    let mut engine = bench(
        12,
        cards,
        [
            Seat::with(&[DICE_RIG, PIXIE_ADVANTAGE, DICE_OBSERVER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let start_life = life(&engine, me());
    let rig = the(&engine, ZoneLocation::Battlefield, DICE_RIG);
    let obs = the(&engine, ZoneLocation::Battlefield, DICE_OBSERVER);

    activate(&mut engine, me(), rig, 0);
    settle(&mut engine, me());

    // Under CR 706.6 (`RollAdditionalDieIgnoreLowest`), rolling 1d20 rolls 2d20
    // and ignores the lowest: ONLY 1 `GameEvent::DiceRolled` is journalled and
    // `DICE_OBSERVER` triggers EXACTLY ONCE!
    let rolled_events: Vec<u32> = engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            crate::event::GameEvent::DiceRolled {
                player,
                sides,
                result,
            } => {
                assert_eq!(player, me());
                assert_eq!(sides, 20);
                Some(result)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        rolled_events.len(),
        1,
        "CR 706.6: ignored die roll must not emit DiceRolled"
    );
    assert_eq!(triggered(&engine, obs), 1);

    let natural = rolled_events[0] as i32;
    let modified = natural + 2; // modifier: +2
    let table_bonus = if modified <= 11 { 100 } else { 200 };
    // Total life gained = `modified` (LastDieRoll) + `table_bonus` (IfDieRollInRange) + `natural` (EventAmount on DieRolled trigger)!
    assert_eq!(
        life(&engine, me()) - start_life,
        modified + table_bonus + natural
    );
}
