//! Wave 3 casting and alternate-cost completeness:
//! - Suspend creature haste ("until you lose control"), instant-speed suspend,
//!   cast prohibitions, and external last-time-counter removal (CR 702.62a–d)
//! - Dash casting, haste, and end-step return to hand (CR 702.109a, CR 400.7)
//! - Blitz casting (mana + non-mana parts), haste, end-step sacrifice, and
//!   dies-draw trigger + ability-loss/copy/exile interactions (CR 702.152a)
//! - Miracle hidden-information safety before reveal (CR 702.94a–b)
//! - Spectacle (CR 702.137a), Surge (CR 702.117a), Prowl (CR 702.76a), and
//!   `Condition::AltCostPaid` (CR 601.2b, CR 400.7d).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, YesNoPrompt};
use crate::turn::{Phase, Step};
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    AlternativeCost, Amount, Condition, CostPart, CounterKind, Duration, Filter, Modifier,
    PlayerRel, TargetSpec, Trigger,
};
use baylee_core::generated::subtypes::creature;
use baylee_core::ids::Defender;

const SUSPEND_BEAST: u32 = 8200;
const SUSPEND_INSTANT: u32 = 8201;
const SUSPEND_FLASH_ELF: u32 = 8202;
const SILENCE_PILLAR: u32 = 8203;
const DASH_SCOUT: u32 = 8204;
const BLITZ_UNDERDOG: u32 = 8205;
const HUMBLE_SHRINE: u32 = 8206;
const EXILE_PROBE: u32 = 8207;
const MIRACLE_BOLT: u32 = 8208;
const DRAW_PROBE: u32 = 8209;
const SPECTACLE_REVELER: u32 = 8210;
const PING_PROBE: u32 = 8211;
const SURGE_TENTACLES: u32 = 8212;
const CANTRIP_SORCERY: u32 = 8213;
const PROWL_THEFT: u32 = 8214;
const ROGUE_ATTACKER: u32 = 8215;
const GOBLIN_ATTACKER: u32 = 8216;
const SAC_OUTLET: u32 = 8217;

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

/// CR 702.62a: A creature spell cast via Suspend's last-time-counter trigger
/// gains haste until its caster loses control of the spell or permanent.
/// Losing control ends haste permanently (even if control later returns).
#[test]
fn suspend_creature_gains_haste_and_loses_it_permanently_when_control_changes() {
    static SUSPEND_ABILS: [AbilityDef; 1] = [AbilityDef::Suspend {
        counters: 1,
        cost: ManaCost::parse("{G}"),
    }];
    let beast = card(
        SUSPEND_BEAST,
        creature_face("Rift Beast", "{4}{G}", 4, 4),
        KeywordSet::EMPTY,
        &SUSPEND_ABILS,
    );

    let mut engine = bench(
        8200,
        vec![beast],
        [
            Seat::with(&[forest()]).holding(&[SUSPEND_BEAST]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());

    let hand_beast = the(&engine, ZoneLocation::Hand(me()), SUSPEND_BEAST);
    let (_, legal) = offer(&engine);
    assert!(
        legal.suspendable.contains(&hand_beast),
        "creature with Suspend 1—{{G}} is suspendable in main phase"
    );
    engine
        .apply(me(), PlayerAction::Suspend { card: hand_beast })
        .expect("suspend creature");
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), SUSPEND_BEAST).len(),
        1,
        "exiled with 1 time counter"
    );

    // Advance to seat 0's next turn (turn 3): upkeep removes last time counter
    // and casts the creature for free.
    walk_until(&mut engine, |e| {
        e.state().turn.number >= 3 && holds_priority_in(e, me(), Step::Main)
    });
    let bf_beast = the(&engine, ZoneLocation::Battlefield, SUSPEND_BEAST);
    assert!(
        engine
            .state()
            .object(bf_beast)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 702.62a: creature cast via suspend has haste"
    );

    // Walk into DeclareAttackers on turn 3 and verify it is offered as a legal attacker!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        attackers.contains(&bf_beast),
        "CR 702.62a: post-suspend creature can attack the turn it enters"
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(bf_beast, Defender::Player(them()))],
            },
        )
        .expect("attack with suspended creature");
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && holds_priority_in(e, me(), Step::Main)
    });
    assert_eq!(life(&engine, them()), 16, "dealt 4 combat damage");

    // Opponent temporarily gains control UntilEndOfTurn: haste is lost permanently!
    let ts = engine.state.next_timestamp();
    let steal_id = engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(bf_beast),
            controller: them(),
            layer: baylee_cards_dsl::Layer::Control,
            filter: crate::effects::EffectFilter::object(&engine.state, bf_beast),
            modifier: Modifier::GainControl,
            timestamp: ts,
            duration: Duration::UntilEndOfTurn,
            origin: crate::effects::EffectOrigin::Resolution,
        });
    engine.state.refresh_characteristics();
    assert_eq!(
        engine.state().object(bf_beast).unwrap().controller,
        them(),
        "opponent gained control"
    );
    assert!(
        !engine
            .state()
            .object(bf_beast)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 702.62a: losing control ends post-suspend haste"
    );

    // Control-change effect ends so seat 0 regains control: haste stays gone!
    engine.state.effects.remove_where(|fx| fx.id == steal_id);
    engine.state.refresh_characteristics();
    assert_eq!(
        engine.state().object(bf_beast).unwrap().controller,
        me(),
        "seat 0 regained control"
    );
    assert!(
        !engine
            .state()
            .object(bf_beast)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 702.62a: 'until you lose control' never reinstates haste after control returns"
    );
}

/// CR 702.62a–c: Instant and Flash cards with Suspend can be suspended at
/// instant speed (including on an opponent's turn), non-Flash sorcery/creature
/// cards cannot, cast prohibitions forbid suspending, and removing the last
/// time counter externally triggers the free cast once.
#[test]
fn suspend_instant_speed_and_prohibitions_and_external_counter_removal() {
    static INSTANT_ABILS: [AbilityDef; 2] = [
        AbilityDef::Suspend {
            counters: 2,
            cost: ManaCost::parse("{G}"),
        },
        spell!(&[Effect::GainLife {
            amount: Amount::Fixed(3),
        }]),
    ];
    static FLASH_ABILS: [AbilityDef; 1] = [AbilityDef::Suspend {
        counters: 2,
        cost: ManaCost::parse("{G}"),
    }];
    static SORC_ABILS: [AbilityDef; 1] = [AbilityDef::Suspend {
        counters: 2,
        cost: ManaCost::parse("{G}"),
    }];
    static INSTANT_FILTER: Filter = Filter::HasType(TypeSet::INSTANT);
    static LOCK_ABILS: [AbilityDef; 1] = [static_ability!(
        Filter::HasType(TypeSet::INSTANT),
        Modifier::PlayersCantCast {
            who: PlayerRel::Opponent,
            filter: &INSTANT_FILTER,
        }
    )];

    let susp_inst = card(
        SUSPEND_INSTANT,
        face("Delayed Balm", "{2}{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &INSTANT_ABILS,
    );
    let susp_flash = card(
        SUSPEND_FLASH_ELF,
        creature_face("Flash Scout", "{2}{G}", 2, 2),
        KeywordSet::FLASH,
        &FLASH_ABILS,
    );
    let susp_beast = card(
        SUSPEND_BEAST,
        creature_face("Slow Beast", "{4}{G}", 4, 4),
        KeywordSet::EMPTY,
        &SORC_ABILS,
    );
    let lock = card(
        SILENCE_PILLAR,
        face("Instant Lock", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &LOCK_ABILS,
    );

    let mut engine = bench(
        8201,
        vec![susp_inst, susp_flash, susp_beast, lock],
        [
            Seat::with(&[forest(), forest()]).holding(&[
                SUSPEND_INSTANT,
                SUSPEND_FLASH_ELF,
                SUSPEND_BEAST,
            ]),
            Seat::with(&[SILENCE_PILLAR]),
        ],
    );

    // Walk to opponent's (seat 1's) main phase and pass priority to seat 0.
    to_main(&mut engine, them());
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    float_all(&mut engine, me());

    let inst_id = the(&engine, ZoneLocation::Hand(me()), SUSPEND_INSTANT);
    let flash_id = the(&engine, ZoneLocation::Hand(me()), SUSPEND_FLASH_ELF);
    let beast_id = the(&engine, ZoneLocation::Hand(me()), SUSPEND_BEAST);

    let (_, legal) = offer(&engine);
    assert!(
        !legal.suspendable.contains(&inst_id),
        "CR 702.62c: opponent's PlayersCantCast(INSTANT) prohibits suspending an instant"
    );
    assert!(
        engine
            .apply(me(), PlayerAction::Suspend { card: inst_id })
            .is_err(),
        "CR 702.62c: action handler also rejects prohibited suspend"
    );
    assert!(
        !legal.suspendable.contains(&beast_id),
        "CR 702.62a: non-flash creature cannot be suspended on opponent's turn"
    );
    assert!(
        legal.suspendable.contains(&flash_id),
        "CR 702.62a: flash creature CAN be suspended on opponent's turn"
    );

    // Remove the Instant Lock, refresh, and suspend the Instant on opponent's turn!
    let lock_id = the(&engine, ZoneLocation::Battlefield, SILENCE_PILLAR);
    let _ = engine.state.move_object(
        lock_id,
        ZoneLocation::Graveyard(them()),
        ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    engine.sync_static_effects();
    refresh_offer(&mut engine);
    let (_, legal) = offer(&engine);
    assert!(
        legal.suspendable.contains(&inst_id),
        "CR 702.62a: instant with Suspend is suspendable on opponent's turn once lock is gone"
    );
    engine
        .apply(me(), PlayerAction::Suspend { card: inst_id })
        .expect("suspend instant at instant speed");

    // Remove all 2 time counters externally (Clockspinning / Timebender style):
    // triggers free cast immediately!
    let exiled_inst = the(&engine, ZoneLocation::Exile(me()), SUSPEND_INSTANT);
    crate::replacement::remove_counters(&mut engine.state, exiled_inst, CounterKind::Time, 2);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    settle(&mut engine, them());
    assert_eq!(
        life(&engine, me()),
        23,
        "CR 702.62a: removing the last time counter externally casts the suspended instant"
    );
}

/// CR 702.109a, CR 400.7: Casting with Dash grants haste and bounces the
/// permanent at the beginning of the next end step; blinking makes a new object
/// that neither has dash haste nor returns at end step.
#[test]
fn dash_grants_haste_and_returns_at_end_step_unless_blinked() {
    let scout = card(
        DASH_SCOUT,
        FaceDef {
            dash: Some(ManaCost::parse("{G}")),
            ..creature_face("Dash Scout", "{2}{G}", 2, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        8204,
        vec![scout],
        [
            Seat::with(&[forest()]).holding(&[DASH_SCOUT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());

    let scout_id = the(&engine, ZoneLocation::Hand(me()), DASH_SCOUT);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&scout_id),
        "Dash {{G}} makes {{2}}{{G}} creature castable off one Forest"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: scout_id })
        .expect("cast via only affordable mode (Dash)");
    settle(&mut engine, me());

    let bf_scout = the(&engine, ZoneLocation::Battlefield, DASH_SCOUT);
    assert!(
        engine
            .state()
            .object(bf_scout)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 702.109a: dashed permanent has haste"
    );

    // End step returns it to owner's hand.
    to_main(&mut engine, them());
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), DASH_SCOUT).len(),
        1,
        "CR 702.109a: dashed creature returned to owner's hand at end step"
    );

    // Next own turn: dash it again, then blink it (Exile -> Battlefield) before
    // end step: CR 400.7 makes a new permanent with neither dash haste nor return!
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let scout_id2 = the(&engine, ZoneLocation::Hand(me()), DASH_SCOUT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: scout_id2 })
        .expect("dash again");
    settle(&mut engine, me());
    let bf_scout2 = the(&engine, ZoneLocation::Battlefield, DASH_SCOUT);
    let _ = engine.state.move_object(
        bf_scout2,
        ZoneLocation::Exile(me()),
        ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    let _ = engine.state.move_object(
        bf_scout2,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    engine.state.refresh_characteristics();
    assert!(
        !engine
            .state()
            .object(bf_scout2)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 400.7: blinked permanent has no dash cost paid for it and loses haste"
    );
    to_main(&mut engine, them());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, DASH_SCOUT).len(),
        1,
        "CR 400.7: blinked permanent stays on the battlefield past the next end step"
    );
}

/// CR 702.152a: Blitz casts for its alternative cost (mana + non-mana parts),
/// grants haste, sacrifices the permanent at the beginning of the next end
/// step, and draws a card when the permanent dies.
#[test]
fn blitz_casts_for_mana_and_life_grants_haste_sacrifices_at_end_step_and_draws_on_death() {
    static BLITZ_PARTS: [CostPart; 1] = [CostPart::PayLife(2)];
    let underdog = card(
        BLITZ_UNDERDOG,
        FaceDef {
            blitz: Some(Cost {
                mana: ManaCost::parse("{G}"),
                parts: &BLITZ_PARTS,
            }),
            ..creature_face("Blitz Brawler", "{3}{G}", 3, 2)
        },
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        8205,
        vec![underdog],
        [
            Seat::with(&[forest()]).holding(&[BLITZ_UNDERDOG]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());

    let card_id = the(&engine, ZoneLocation::Hand(me()), BLITZ_UNDERDOG);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&card_id),
        "CR 702.152a: Blitz {{G}}, Pay 2 life makes {{3}}{{G}} creature castable off one Forest"
    );
    assert!(
        legal
            .cast_ways
            .iter()
            .any(|(id, w)| *id == card_id && w.kind == CastModeKind::Blitz),
        "cast_ways publishes CastModeKind::Blitz"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    engine
        .apply(me(), PlayerAction::CastSpell { card: card_id })
        .expect("cast with blitz");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 18, "paid 2 life as part of blitz cost");

    let bf_id = the(&engine, ZoneLocation::Battlefield, BLITZ_UNDERDOG);
    assert!(
        engine
            .state()
            .object(bf_id)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 702.152a: blitzed permanent has haste"
    );

    // Attack immediately with haste!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(bf_id, Defender::Player(them()))],
            },
        )
        .expect("attack with blitzed creature");

    // Advance to opponent's main phase: end step sacrifices the blitzed creature
    // and its dies trigger draws a card!
    to_main(&mut engine, them());
    assert_eq!(life(&engine, them()), 17, "dealt 3 combat damage");
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), BLITZ_UNDERDOG).len(),
        1,
        "CR 702.152a: sacrificed at beginning of next end step"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before,
        "CR 702.152a: 'When this permanent dies, draw a card' replaced the card in hand"
    );
}

/// CR 702.152a, CR 613.1f, CR 603.10a, CR 707.2: Losing all abilities removes
/// both blitz haste and the dies-draw trigger; exiling a blitzed permanent does
/// not draw a card.
#[test]
fn blitz_ability_removal_and_exile_do_not_draw_and_clone_does_not_copy_blitz() {
    const CLONE_SHIFTER: u32 = 8218;
    static CLONE_ABILS: [AbilityDef; 1] = [AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[],
    }];
    static LOSE_ALL: [AbilityDef; 1] = [static_ability!(
        Filter::CREATURE,
        Modifier::LoseAllAbilities
    )];
    static EXILE_TARGET: [AbilityDef; 1] = [free(
        &[Effect::Exile {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static SAC_CREATURE: [AbilityDef; 1] = [paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::Sacrifice(&Filter::CREATURE)],
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    )];

    let underdog = card(
        BLITZ_UNDERDOG,
        FaceDef {
            blitz: Some(Cost {
                mana: ManaCost::parse("{G}"),
                parts: &[],
            }),
            ..creature_face("Blitz Brawler", "{3}{G}", 3, 2)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let shrine = card(
        HUMBLE_SHRINE,
        face("Dress Down Shrine", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        &LOSE_ALL,
    );
    let exiler = card(
        EXILE_PROBE,
        face("Exiler", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &EXILE_TARGET,
    );
    let outlet = card(
        SAC_OUTLET,
        face("Altar", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &SAC_CREATURE,
    );

    // Case 1: Exiling a blitzed creature does NOT draw a card.
    let mut engine = bench(
        8206,
        vec![underdog, shrine, exiler, outlet],
        [
            Seat::with(&[forest(), EXILE_PROBE]).holding(&[BLITZ_UNDERDOG]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let hand_c = the(&engine, ZoneLocation::Hand(me()), BLITZ_UNDERDOG);
    engine
        .apply(me(), PlayerAction::CastSpell { card: hand_c })
        .expect("cast blitz");
    settle(&mut engine, me());
    let bf_c = the(&engine, ZoneLocation::Battlefield, BLITZ_UNDERDOG);
    let exiler_id = the(&engine, ZoneLocation::Battlefield, EXILE_PROBE);
    activate(&mut engine, me(), exiler_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bf_c],
                players: vec![],
            },
        )
        .expect("target blitzed creature");
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .is_empty(),
        "exiling a blitzed permanent is not dying, so no card is drawn"
    );

    // Case 2: Under LoseAllAbilities, blitzed creature has no haste and draws
    // nothing when sacrificed!
    let mut engine2 = bench(
        8207,
        vec![underdog, shrine, exiler, outlet],
        [
            Seat::with(&[forest(), HUMBLE_SHRINE, SAC_OUTLET]).holding(&[BLITZ_UNDERDOG]),
            Seat::default(),
        ],
    );
    to_main(&mut engine2, me());
    float_all(&mut engine2, me());
    let hand_c2 = the(&engine2, ZoneLocation::Hand(me()), BLITZ_UNDERDOG);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: hand_c2 })
        .expect("cast blitz under LoseAllAbilities");
    settle(&mut engine2, me());
    let bf_c2 = the(&engine2, ZoneLocation::Battlefield, BLITZ_UNDERDOG);
    assert!(
        !engine2
            .state()
            .object(bf_c2)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 613.1f: LoseAllAbilities removes blitz haste"
    );
    let outlet_id = the(&engine2, ZoneLocation::Battlefield, SAC_OUTLET);
    activate(&mut engine2, me(), outlet_id, 0);
    engine2
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bf_c2],
            },
        )
        .expect("choose blitzed creature to sacrifice");
    settle(&mut engine2, me());
    assert!(
        engine2
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .is_empty(),
        "CR 603.10a + CR 613.1f: blitzed permanent without abilities does not trigger dies-draw"
    );

    // Case 3: A Clone copying a blitzed permanent does NOT copy blitz haste or
    // the dies-draw trigger (CR 707.2).
    let shifter = card(
        CLONE_SHIFTER,
        creature_face("Mirror Shifter", "{G}", 0, 0),
        KeywordSet::EMPTY,
        &CLONE_ABILS,
    );
    let mut engine3 = bench(
        8218,
        vec![underdog, shrine, exiler, outlet, shifter],
        [
            Seat::with(&[forest(), forest()]).holding(&[BLITZ_UNDERDOG, CLONE_SHIFTER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine3, me());
    float_all(&mut engine3, me());
    let hand_c3 = the(&engine3, ZoneLocation::Hand(me()), BLITZ_UNDERDOG);
    engine3
        .apply(me(), PlayerAction::CastSpell { card: hand_c3 })
        .expect("cast blitz");
    settle(&mut engine3, me());
    let bf_c3 = the(&engine3, ZoneLocation::Battlefield, BLITZ_UNDERDOG);

    let clone_hand = the(&engine3, ZoneLocation::Hand(me()), CLONE_SHIFTER);
    engine3
        .apply(me(), PlayerAction::CastSpell { card: clone_hand })
        .expect("cast clone");
    walk_until(&mut engine3, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine3
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bf_c3],
                players: vec![],
            },
        )
        .expect("copy blitzed creature");
    settle(&mut engine3, me());
    let bf_clone = the(&engine3, ZoneLocation::Battlefield, CLONE_SHIFTER);
    assert!(
        !engine3
            .state()
            .object(bf_clone)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "CR 707.2: copy of a blitzed creature does not gain blitz haste"
    );
    let _ = engine3
        .state
        .sacrifice_object(bf_clone, crate::event::Cause::Effect);
    engine3.collect_triggers_for_tests();
    assert!(
        engine3.state().zones.stack_is_empty(),
        "CR 707.2: copy of a blitzed creature does not trigger blitz dies-draw"
    );
}

/// CR 702.94a–b: While a player is deciding whether to reveal a Miracle card,
/// opponents receive no pending choice and no hidden card identity; only the
/// first card drawn in a turn offers Miracle.
#[test]
fn miracle_keeps_hand_hidden_until_revealed_and_only_first_draw_triggers() {
    static MIRACLE_ABILS: [AbilityDef; 1] = [spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(5),
        target: TargetSpec::Player(PlayerRel::Opponent),
    }])];
    static DRAW_ABILS: [AbilityDef; 1] = [free(
        &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        None,
    )];

    let miracle_card = card(
        MIRACLE_BOLT,
        FaceDef {
            miracle: Some(ManaCost::parse("{G}")),
            ..face("Thunder Miracle", "{5}{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        &MIRACLE_ABILS,
    );
    let drawer = card(
        DRAW_PROBE,
        face("Tome", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &DRAW_ABILS,
    );

    let mut engine = bench(
        8208,
        vec![miracle_card, drawer],
        [
            Seat::with(&[forest(), DRAW_PROBE]).holding(&[MIRACLE_BOLT, MIRACLE_BOLT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    // Put both Miracle cards on top of seat 0's library before any draw this turn
    // (turn 1 skips seat 0's draw step, so `per_turn.draws[0] == 0`!).
    let m_cards = objects(&engine, ZoneLocation::Hand(me()), MIRACLE_BOLT);
    for id in m_cards {
        let _ = engine.state.move_object(
            id,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        );
    }
    refresh_offer(&mut engine);

    let tome = the(&engine, ZoneLocation::Battlefield, DRAW_PROBE);
    activate(&mut engine, me(), tome, 0);
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Miracle { .. },
                ..
            }
        )
    });

    // Hidden-info check: opponent sees NO pending question and NO revealed hand card!
    assert!(
        engine.information_pending_for(them()).is_none(),
        "CR 702.94a: miracle offer is private until the drawer chooses to reveal"
    );
    let drawn_id = the(&engine, ZoneLocation::Hand(me()), MIRACLE_BOLT);
    let was_revealed = |e: &Bench, id: ObjectId| {
        e.journal().entries().iter().any(|entry| {
            matches!(
                &entry.event,
                crate::event::GameEvent::Revealed { cards, .. } if cards.contains(&id)
            )
        })
    };
    assert!(
        !was_revealed(&engine, drawn_id),
        "CR 702.94b: card is not revealed before answering Yes"
    );

    // Decline the first miracle: card stays unrevealed in hand.
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert!(!was_revealed(&engine, drawn_id));

    // Second draw this turn is also MIRACLE_BOLT, and MUST NOT offer Miracle!
    activate(&mut engine, me(), tome, 0);
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), MIRACLE_BOLT).len(),
        2,
        "CR 702.94a: second card drawn this turn went straight to hand with no miracle offer"
    );
}

/// CR 702.137a (Spectacle), CR 702.117a (Surge), and `Condition::AltCostPaid`:
/// - Spectacle unlocks only after an opponent loses life this turn (not self).
/// - Surge unlocks after you or a teammate cast another spell this turn (not an opponent).
/// - `Condition::AltCostPaid` holds on both resolving spells and ETB permanents
///   when and only when cast via `CastModeKind::Alternative`.
#[test]
fn spectacle_and_surge_unlock_alternative_costs_and_alt_cost_paid() {
    static SPECTACLE_ALTS: [AlternativeCost; 1] =
        [AlternativeCost::spectacle(baylee_cards_dsl::cost!("{G}"))];
    static REVELER_ABILS: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::DrawCards {
            amount: Amount::Fixed(2),
        }],
        condition = Some(Condition::AltCostPaid)
    )];
    static PING_OPP: [AbilityDef; 2] = [
        free(
            &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::You,
            }],
            None,
        ),
        free(
            &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::Opponent,
            }],
            None,
        ),
    ];

    static SURGE_ALTS: [AlternativeCost; 1] =
        [AlternativeCost::surge(baylee_cards_dsl::cost!("{G}"))];
    static SURGE_ABILS: [AbilityDef; 1] = [spell!(&[Effect::IfCondition {
        condition: Condition::AltCostPaid,
        then: &[Effect::GainLife {
            amount: Amount::Fixed(5),
        }],
        otherwise: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
    }])];
    static CANTRIP_ABILS: [AbilityDef; 1] = [spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let reveler = card(
        SPECTACLE_REVELER,
        FaceDef {
            alternative_costs: &SPECTACLE_ALTS,
            ..creature_face("Spectacle Reveler", "{3}{G}", 2, 2)
        },
        KeywordSet::EMPTY,
        &REVELER_ABILS,
    );
    let pinger = card(
        PING_PROBE,
        face("Life Probe", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &PING_OPP,
    );
    let surge_spell = card(
        SURGE_TENTACLES,
        FaceDef {
            alternative_costs: &SURGE_ALTS,
            ..face("Surge Wave", "{4}{G}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        &SURGE_ABILS,
    );
    let cantrip = card(
        CANTRIP_SORCERY,
        face("Warmup", "{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &CANTRIP_ABILS,
    );

    let mut engine = bench(
        8210,
        vec![reveler, pinger, surge_spell, cantrip],
        [
            Seat::with(&[forest(), forest(), PING_PROBE]).holding(&[
                SPECTACLE_REVELER,
                SURGE_TENTACLES,
                CANTRIP_SORCERY,
            ]),
            Seat::with(&[forest()]).holding(&[CANTRIP_SORCERY]),
        ],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());

    let rev_id = the(&engine, ZoneLocation::Hand(me()), SPECTACLE_REVELER);
    let surge_id = the(&engine, ZoneLocation::Hand(me()), SURGE_TENTACLES);
    let pinger_id = the(&engine, ZoneLocation::Battlefield, PING_PROBE);

    // Self life loss does NOT enable Spectacle, and 0 spells cast does NOT enable Surge!
    activate(&mut engine, me(), pinger_id, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 19);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&rev_id),
        "CR 702.137a: losing your own life does not unlock Spectacle"
    );
    assert!(
        !legal.castable.contains(&surge_id),
        "CR 702.117a: Surge is locked before you or a teammate cast another spell"
    );

    // Opponent casting an instant on your turn does NOT unlock YOUR Surge!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    float_all(&mut engine, them());
    let opp_cantrip = the(&engine, ZoneLocation::Hand(them()), CANTRIP_SORCERY);
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp_cantrip })
        .expect("opponent casts instant");
    settle(&mut engine, me());
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&surge_id),
        "CR 702.117a: an opponent's spell does not unlock your Surge"
    );

    // Opponent loses 1 life -> Spectacle unlocks! Cast Reveler for {G}:
    // casting Reveler ALSO counts as a spell cast by `me()`, unlocking Surge!
    activate(&mut engine, me(), pinger_id, 1);
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 20); // gained 1 from cantrip, lost 1 = 20, and still lost life this turn!
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&rev_id),
        "CR 702.137a: opponent lost life this turn (even after gaining life) unlocks Spectacle"
    );
    let hand_before_reveler = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    engine
        .apply(me(), PlayerAction::CastSpell { card: rev_id })
        .expect("cast Spectacle Reveler for {G}");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_reveler - 1 + 2,
        "CR 603.4 + Condition::AltCostPaid: ETB trigger fired because spectacle cost was paid"
    );

    // Now `me()` has cast a spell this turn -> Surge Wave ({4}{G}, Surge {G}) is castable for {G}!
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&surge_id),
        "CR 702.117a: casting another spell this turn unlocks Surge"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: surge_id })
        .expect("cast Surge Wave for {G}");
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        24,
        "CR 702.117a + Condition::AltCostPaid: Surge spell gained 5 life instead of 1"
    );
}

/// CR 702.76a (Prowl): Unlocked if and only if a player was dealt combat damage
/// this turn by a source that, at the time it dealt that damage, was under your
/// control and shared a creature type with this spell (even if that creature
/// later left the battlefield before casting).
#[test]
fn prowl_tracks_combat_damage_by_shared_creature_type_even_after_attacker_dies() {
    static PROWL_ALTS: [AlternativeCost; 1] =
        [AlternativeCost::prowl(baylee_cards_dsl::cost!("{G}"))];
    static PROWL_ABILS: [AbilityDef; 1] = [spell!(&[
        Effect::LoseLife {
            amount: Amount::Fixed(3),
            target: PlayerRel::Opponent,
        },
        Effect::IfCondition {
            condition: Condition::AltCostPaid,
            then: &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            otherwise: &[],
        },
    ])];
    static ROGUE_SUBTYPES: [baylee_core::ids::SubtypeId; 1] = [creature::ROGUE];
    static GOBLIN_SUBTYPES: [baylee_core::ids::SubtypeId; 1] = [creature::GOBLIN];
    static SAC_CREATURE: [AbilityDef; 1] = [paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::Sacrifice(&Filter::CREATURE)],
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    )];

    let theft = card(
        PROWL_THEFT,
        FaceDef {
            types: TypeSet::KINDRED.union(TypeSet::SORCERY),
            subtypes: &ROGUE_SUBTYPES,
            alternative_costs: &PROWL_ALTS,
            ..face("Rogue Theft", "{3}{G}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        &PROWL_ABILS,
    );
    let rogue = card(
        ROGUE_ATTACKER,
        FaceDef {
            subtypes: &ROGUE_SUBTYPES,
            ..creature_face("Sneak Rogue", "{G}", 1, 1)
        },
        KeywordSet::HASTE,
        &[],
    );
    let goblin = card(
        GOBLIN_ATTACKER,
        FaceDef {
            subtypes: &GOBLIN_SUBTYPES,
            ..creature_face("Brawler Goblin", "{G}", 1, 1)
        },
        KeywordSet::HASTE,
        &[],
    );
    let outlet = card(
        SAC_OUTLET,
        face("Altar", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &SAC_CREATURE,
    );

    // Turn 1: Attack with ONLY the Goblin -> deals combat damage, does NOT unlock Rogue Prowl!
    let mut engine = bench(
        8214,
        vec![theft, rogue, goblin, outlet],
        [
            Seat::with(&[forest(), GOBLIN_ATTACKER, ROGUE_ATTACKER, SAC_OUTLET])
                .holding(&[PROWL_THEFT]),
            Seat::default(),
        ],
    );
    let gob_id = the(&engine, ZoneLocation::Battlefield, GOBLIN_ATTACKER);
    let rog_id = the(&engine, ZoneLocation::Battlefield, ROGUE_ATTACKER);
    let theft_id = the(&engine, ZoneLocation::Hand(me()), PROWL_THEFT);
    let altar_id = the(&engine, ZoneLocation::Battlefield, SAC_OUTLET);

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(gob_id, Defender::Player(them()))],
            },
        )
        .expect("attack with goblin only");
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && holds_priority_in(e, me(), Step::Main)
    });
    float_all(&mut engine, me());
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&theft_id),
        "CR 702.76a: combat damage by a Goblin does not unlock a Rogue Prowl spell"
    );

    // Next own turn (turn 3): Attack with the Rogue, then SACRIFICE the Rogue in
    // Second Main BEFORE casting Prowl Theft — CR 702.76a still unlocks Prowl!
    walk_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me())
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(rog_id, Defender::Player(them()))],
            },
        )
        .expect("attack with rogue");
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && holds_priority_in(e, me(), Step::Main)
    });

    // Sacrifice the Rogue (and the Goblin is still there; choose Rogue if asked).
    let _ = engine
        .state
        .sacrifice_object(rog_id, crate::event::Cause::Cost);
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), ROGUE_ATTACKER).len(),
        1,
        "Rogue left the battlefield before Prowl Theft is cast"
    );
    let _ = altar_id;
    float_all(&mut engine, me());
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&theft_id),
        "CR 702.76a: Prowl checks source characteristics & controller at the time combat damage was dealt"
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    engine
        .apply(me(), PlayerAction::CastSpell { card: theft_id })
        .expect("cast Rogue Theft for Prowl {G}");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before,
        "CR 702.76a + Condition::AltCostPaid: Prowl spell drew a card because prowl cost was paid"
    );
}
