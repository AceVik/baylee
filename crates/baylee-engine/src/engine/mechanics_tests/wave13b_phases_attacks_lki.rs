//! Beat 18 / Engine Wave 13b deterministic rule tests:
//! 1. `StepKind::SecondMain` (`CR 505.1`, `Phase.PhaseCount$ 2`) — firing on
//!    the second actual main phase of a turn (and neither `FirstMain` nor
//!    `SecondMain` on a 3rd main phase after an extra combat + main phase),
//!    plus `Condition::DuringStep` and `Condition::BeforeStep`.
//! 2. `Trigger::AttackersDeclaredAtTarget` (`CR 508.3`, `AttackersDeclaredOneTarget`)
//!    — per-defender firing in 2p and 3p games, `exact_count: Some(1)` (rejecting
//!    when any other creature also attacks that defender), `EventPlayer` /
//!    `EventObject` / `EventAmount` / `SubjectContext::found` bindings,
//!    planeswalker defending player binding, and *Wulfgar* `TriggerMultiplier`
//!    (`TriggerEventKind::Attack`) & `TriggerSuppress`.
//! 3. LKI `ReplacementRule::ExileInsteadOfGraveyard` (`CR 603.10a`, `CR 614.6`,
//!    `Moved.ValidLKI`) — *Etching of Kumano* (`SourceControllerThisTurn`,
//!    including tokens), *Incendiary Oracle* (`SourceThisTurn`, including
//!    simultaneous combat death), *Valgavoth, Terror Eater* (any-zone exile,
//!    `Rider::Linked` + `MayCastWithAltCost::LinkedExile`, and `controller_lki`
//!    preventing re-exile of cards you controlled), and *Gisa*-style
//!    `controller_lki` + exile `counter`.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::object::Rider;
use crate::turn::Step;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    AltCastPrice, AltCastZones, Amount, AttackedTargetKind, Condition, DamagedByScope, Duration,
    Filter, Modifier, PlayerRel, ReplacementRule, StepKind, TargetReq, TargetSpec, TokenDef,
    Trigger, TriggerEventKind, ZoneRef, ZoneSel, activated, counters, static_ability, triggered,
};
use baylee_core::ids::Defender;

const C0: u32 = 99_940;
const C1: u32 = 99_941;
const C2: u32 = 99_942;
const C3: u32 = 99_943;
const C4: u32 = 99_944;
const C5: u32 = 99_945;
const C6: u32 = 99_946;

fn third() -> PlayerId {
    PlayerId::new(2)
}

fn refresh_offer(engine: &mut Bench) {
    engine.state.invalidate_projections();
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn walk_to_step(engine: &mut Bench, seat: PlayerId, step: Step) {
    walk_until(engine, |e| holds_priority_in(e, seat, step));
}

fn walk_to_main_count(engine: &mut Bench, seat: PlayerId, main_count: u32) {
    walk_until(engine, |e| {
        holds_priority_in(e, seat, Step::Main) && e.state().per_turn.main_phases_begun == main_count
    });
}

fn declare_attacks_and_settle(
    engine: &mut Bench,
    attacker: PlayerId,
    pairs: Vec<(ObjectId, Defender)>,
) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == attacker),
    );
    engine
        .apply(
            attacker,
            PlayerAction::DeclareAttackers { attackers: pairs },
        )
        .expect("declare attackers");
    settle(engine, attacker);
}

// =========================================================================
// 1. StepKind::SecondMain (CR 505.1)
// =========================================================================

#[test]
fn second_main_trigger_fires_only_on_second_main_not_first_or_third_after_extra_combat() {
    static FIRST_MAIN_WATCHER: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::FirstMain,
            whose: PlayerRel::You,
        },
        &[Effect::gain_life(1)]
    )];
    static SECOND_MAIN_WATCHER: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::SecondMain,
            whose: PlayerRel::You,
        },
        &[Effect::gain_life(3)]
    )];
    static EXTRA_PHASE_ENGINE: &[AbilityDef] = &[free(
        &[Effect::AdditionalCombatPhase {
            followed_by_main_phase: true,
        }],
        None,
    )];

    let first_watcher = card(
        C0,
        face("First Watcher", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        FIRST_MAIN_WATCHER,
    );
    let second_watcher = card(
        C1,
        face("Second Watcher", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SECOND_MAIN_WATCHER,
    );
    let extra_phase = card(
        C2,
        face("Extra Combat Engine", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        EXTRA_PHASE_ENGINE,
    );

    let mut engine = bench(
        1301,
        vec![first_watcher, second_watcher, extra_phase],
        [Seat::with(&[C0, C1, C2]), Seat::default()],
    );

    // Main phase 1 (turn 1): FirstMain fires (+1 life -> 21), SecondMain does not.
    walk_to_main_count(&mut engine, me(), 1);
    let w1 = the(&engine, ZoneLocation::Battlefield, C0);
    let w2 = the(&engine, ZoneLocation::Battlefield, C1);
    let extra = the(&engine, ZoneLocation::Battlefield, C2);
    assert_eq!(triggered(&engine, w1), 1, "FirstMain fired once on main #1");
    assert_eq!(
        triggered(&engine, w2),
        0,
        "SecondMain did not fire on main #1"
    );
    assert_eq!(life(&engine, me()), 21);

    // Add an extra combat + extra main phase during FirstMain.
    // Remaining phases this turn: Extra Combat -> Extra Main (#2) -> Regular Combat -> Regular Main (#3).
    activate(&mut engine, me(), extra, 0);
    settle(&mut engine, me());

    // Main phase 2: SecondMain fires once (+3 life -> 24); FirstMain does not fire again.
    walk_to_main_count(&mut engine, me(), 2);
    assert_eq!(triggered(&engine, w1), 1, "FirstMain still 1 on main #2");
    assert_eq!(
        triggered(&engine, w2),
        1,
        "SecondMain fired once on main #2"
    );
    assert_eq!(life(&engine, me()), 24);

    // Main phase 3 (after the second combat phase): NEITHER FirstMain NOR SecondMain fires!
    walk_to_main_count(&mut engine, me(), 3);
    assert_eq!(
        triggered(&engine, w1),
        1,
        "FirstMain must not fire on main #3"
    );
    assert_eq!(
        triggered(&engine, w2),
        1,
        "SecondMain must not fire on main #3"
    );
    assert_eq!(life(&engine, me()), 24);

    // Walk through opponent's entire turn into our next turn's main #1 and main #2.
    let turn_after = engine.state().turn.number + 2;
    walk_until(&mut engine, |e| {
        e.state().turn.number == turn_after
            && holds_priority_in(e, me(), Step::Main)
            && e.state().per_turn.main_phases_begun == 1
    });
    assert_eq!(
        triggered(&engine, w1),
        2,
        "FirstMain fires again on our next turn's main #1 (counter reset)"
    );
    assert_eq!(
        triggered(&engine, w2),
        1,
        "SecondMain did not fire on opponent's turn or our turn 2 main #1"
    );
    assert_eq!(life(&engine, me()), 25);

    walk_to_main_count(&mut engine, me(), 2);
    assert_eq!(
        triggered(&engine, w2),
        2,
        "SecondMain fires again on our next turn's main #2"
    );
    assert_eq!(life(&engine, me()), 28);
}

#[test]
fn neheb_triggers_on_every_postcombat_main_and_fireglass_only_on_second_main() {
    static EXTRA_COMBAT: &[AbilityDef] = &[free(
        &[Effect::AdditionalCombatPhase {
            followed_by_main_phase: true,
        }],
        None,
    )];

    let neheb = baylee_cards::by_oracle_id("b0d3fd57-1420-4dd1-8410-9d659655287f").unwrap();
    let fireglass = baylee_cards::by_oracle_id("b173245e-2009-48ff-ac42-5f28d12aec79").unwrap();
    let extra_combat = card(
        C0,
        face("Extra Combat Engine", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        EXTRA_COMBAT,
    );
    let mut engine = bench(
        1303,
        vec![neheb, fireglass, extra_combat],
        [
            Seat::with(&[neheb.index.get(), fireglass.index.get(), C0]),
            Seat::default(),
            Seat::default(),
        ],
    );
    let neheb = the(&engine, ZoneLocation::Battlefield, neheb.index.get());
    let fireglass = the(&engine, ZoneLocation::Battlefield, fireglass.index.get());
    let extra_combat = the(&engine, ZoneLocation::Battlefield, C0);
    let neheb_trigger_count = |engine: &Bench| {
        engine
            .journal()
            .entries()
            .iter()
            .filter(|entry| {
                matches!(entry.event, crate::event::GameEvent::AbilityTriggered {
                    source, ability_index: 1, ..
                } if source == neheb)
            })
            .count()
    };

    walk_to_main_count(&mut engine, me(), 1);
    assert_eq!(neheb_trigger_count(&engine), 0);
    assert_eq!(triggered(&engine, fireglass), 0);
    engine
        .state
        .change_life(PlayerId::new(1), -2, Cause::Effect);
    engine.state.change_life(third(), -3, Cause::Effect);
    engine.apply(third(), PlayerAction::Concede).unwrap();
    activate(&mut engine, me(), extra_combat, 0);
    settle(&mut engine, me());

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert_eq!(engine.state().per_turn.main_phases_begun, 2);
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(options.len(), 2);
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    walk_to_main_count(&mut engine, me(), 2);
    assert_eq!(neheb_trigger_count(&engine), 1);
    assert_eq!(triggered(&engine, fireglass), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);

    walk_to_main_count(&mut engine, me(), 3);
    assert_eq!(neheb_trigger_count(&engine), 2);
    assert_eq!(triggered(&engine, fireglass), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
}

#[test]
fn second_main_during_step_and_before_step_conditions_respect_main_phase_count() {
    static FIRST_MAIN_ONLY: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::gain_life(1)],
        condition = Some(Condition::DuringStep(StepKind::FirstMain))
    )];
    static SECOND_MAIN_CHECKS: &[AbilityDef] = &[
        activated!(
            Cost::FREE,
            &[Effect::gain_life(1)],
            condition = Some(Condition::DuringStep(StepKind::SecondMain))
        ),
        activated!(
            Cost::FREE,
            &[Effect::gain_life(1)],
            condition = Some(Condition::DuringStep(StepKind::PostcombatMain))
        ),
    ];
    static BEFORE_SECOND_MAIN: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::gain_life(1)],
        condition = Some(Condition::BeforeStep(StepKind::SecondMain))
    )];
    static EXTRA_PHASE_ENGINE: &[AbilityDef] = &[free(
        &[Effect::AdditionalCombatPhase {
            followed_by_main_phase: true,
        }],
        None,
    )];

    let c_first = card(
        C0,
        face("During First", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        FIRST_MAIN_ONLY,
    );
    let c_second = card(
        C1,
        face("During Second", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SECOND_MAIN_CHECKS,
    );
    let c_before = card(
        C2,
        face("Before Second", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        BEFORE_SECOND_MAIN,
    );
    let c_extra = card(
        C3,
        face("Extra Engine", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        EXTRA_PHASE_ENGINE,
    );

    let mut engine = bench(
        1302,
        vec![c_first, c_second, c_before, c_extra],
        [Seat::with(&[C0, C1, C2, C3]), Seat::default()],
    );

    // Upkeep: BeforeStep(SecondMain) open; DuringStep(FirstMain) & DuringStep(SecondMain) closed.
    walk_to_step(&mut engine, me(), Step::Upkeep);
    let a_first = the(&engine, ZoneLocation::Battlefield, C0);
    let a_second = the(&engine, ZoneLocation::Battlefield, C1);
    let a_before = the(&engine, ZoneLocation::Battlefield, C2);
    let a_extra = the(&engine, ZoneLocation::Battlefield, C3);
    let (_, legal) = offer(&engine);
    assert!(!legal.abilities.contains(&(a_first, 0)));
    assert!(!legal.abilities.contains(&(a_second, 0)));
    assert!(legal.abilities.contains(&(a_before, 0)));

    // Main #1: DuringStep(FirstMain) open, BeforeStep(SecondMain) open, DuringStep(SecondMain) closed.
    walk_to_main_count(&mut engine, me(), 1);
    let (_, legal) = offer(&engine);
    assert!(legal.abilities.contains(&(a_first, 0)));
    assert!(!legal.abilities.contains(&(a_second, 0)));
    assert!(!legal.abilities.contains(&(a_second, 1)));
    assert!(legal.abilities.contains(&(a_before, 0)));

    // Queue an extra combat + main phase so Main #2 is followed by Combat #2 and Main #3.
    activate(&mut engine, me(), a_extra, 0);
    settle(&mut engine, me());

    // Combat #1 (still before Main #2): BeforeStep(SecondMain) is still open.
    walk_to_step(&mut engine, me(), Step::CombatBegin);
    let (_, legal) = offer(&engine);
    assert!(!legal.abilities.contains(&(a_first, 0)));
    assert!(!legal.abilities.contains(&(a_second, 0)));
    assert!(legal.abilities.contains(&(a_before, 0)));

    // Main #2: DuringStep(SecondMain) open; FirstMain & BeforeStep(SecondMain) closed.
    walk_to_main_count(&mut engine, me(), 2);
    let (_, legal) = offer(&engine);
    assert!(!legal.abilities.contains(&(a_first, 0)));
    assert!(legal.abilities.contains(&(a_second, 0)));
    assert!(legal.abilities.contains(&(a_second, 1)));
    assert!(!legal.abilities.contains(&(a_before, 0)));

    // Combat #2 (AFTER Main #2!): BeforeStep(SecondMain) MUST remain closed!
    walk_to_step(&mut engine, me(), Step::CombatBegin);
    let (_, legal) = offer(&engine);
    assert!(!legal.abilities.contains(&(a_first, 0)));
    assert!(!legal.abilities.contains(&(a_second, 0)));
    assert!(
        !legal.abilities.contains(&(a_before, 0)),
        "BeforeStep(SecondMain) stays false during extra combat after main #2"
    );

    // Main #3: Only DuringStep(PostcombatMain) is open.
    walk_to_main_count(&mut engine, me(), 3);
    let (_, legal) = offer(&engine);
    assert!(!legal.abilities.contains(&(a_first, 0)));
    assert!(
        !legal.abilities.contains(&(a_second, 0)),
        "DuringStep(SecondMain) is false in main phase #3"
    );
    assert!(legal.abilities.contains(&(a_second, 1)));
    assert!(!legal.abilities.contains(&(a_before, 0)));

    // End step: all three closed.
    walk_to_step(&mut engine, me(), Step::End);
    let (_, legal) = offer(&engine);
    assert!(!legal.abilities.contains(&(a_first, 0)));
    assert!(!legal.abilities.contains(&(a_second, 0)));
    assert!(!legal.abilities.contains(&(a_before, 0)));
}

// =========================================================================
// 2. Trigger::AttackersDeclaredAtTarget (CR 508.3)
// =========================================================================

#[test]
fn attackers_declared_at_target_2p_binds_event_player_first_attacker_and_found_group() {
    static YOUR_FLYERS: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByYou,
        Filter::HasKeyword(KeywordSet::FLYING),
    ]);
    static PUMP_FOUND: &[Effect] = &[Effect::PumpEventObject {
        power: Amount::Fixed(0),
        toughness: Amount::Fixed(3),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }];
    static WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::AttackersDeclaredAtTarget {
            attackers: &YOUR_FLYERS,
            attacked: AttackedTargetKind::Opponent,
            exact_count: None,
        },
        &[
            Effect::LoseLife {
                target: PlayerRel::EventPlayer,
                amount: Amount::EventAmount,
            },
            Effect::PumpEventObject {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::RepeatEachObject {
                zone: ZoneSel::Battlefield,
                filter: &Filter::Any,
                only_found: true,
                then: PUMP_FOUND,
            },
            Effect::UntapFound,
        ]
    )];

    let watcher = card(
        C0,
        face("Sky Marshal", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        WATCHER_ABILITIES,
    );
    let bear = card(
        C1,
        creature_face("Ground Bear", "{1}{G}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let bird1 = card(
        C2,
        creature_face("Hawk Alpha", "{1}{W}", 1, 1),
        KeywordSet::FLYING.union(KeywordSet::HASTE),
        &[],
    );
    let bird2 = card(
        C3,
        creature_face("Hawk Beta", "{1}{W}", 1, 1),
        KeywordSet::FLYING.union(KeywordSet::HASTE),
        &[],
    );

    let mut engine = bench(
        1303,
        vec![watcher, bear, bird1, bird2],
        [Seat::with(&[C0, C1, C2, C3]), Seat::default()],
    );

    to_main(&mut engine, me());
    let w = the(&engine, ZoneLocation::Battlefield, C0);
    let b = the(&engine, ZoneLocation::Battlefield, C1);
    let h1 = the(&engine, ZoneLocation::Battlefield, C2);
    let h2 = the(&engine, ZoneLocation::Battlefield, C3);

    // Declare non-flying Bear first, then Hawk Alpha and Hawk Beta, all attacking opponent.
    declare_attacks_and_settle(
        &mut engine,
        me(),
        vec![
            (b, Defender::Player(them())),
            (h1, Defender::Player(them())),
            (h2, Defender::Player(them())),
        ],
    );

    assert_eq!(
        triggered(&engine, w),
        1,
        "fires exactly once for the attacked opponent despite 2 matching flyers"
    );
    assert_eq!(
        life(&engine, them()),
        18,
        "EventPlayer is the attacked opponent and EventAmount is the 2 matching flyers"
    );
    let h1_chars = engine.state().object(h1).unwrap().characteristics();
    let h2_chars = engine.state().object(h2).unwrap().characteristics();
    let b_chars = engine.state().object(b).unwrap().characteristics();
    assert_eq!(
        (h1_chars.power, h1_chars.toughness),
        (Some(3), Some(4)),
        "Hawk Alpha is both EventObject (+2/+0) and in SubjectContext::found (+0/+3)"
    );
    assert_eq!(
        (h2_chars.power, h2_chars.toughness),
        (Some(1), Some(4)),
        "Hawk Beta is in SubjectContext::found (+0/+3) but NOT EventObject"
    );
    assert_eq!(
        (b_chars.power, b_chars.toughness),
        (Some(2), Some(2)),
        "Non-matching Bear is neither EventObject nor in SubjectContext::found"
    );
    assert!(
        !engine
            .state()
            .object(h1)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "UntapFound untapped Hawk Alpha"
    );
    assert!(
        !engine
            .state()
            .object(h2)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "UntapFound untapped Hawk Beta"
    );
    assert!(
        engine
            .state()
            .object(b)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "UntapFound left non-matching Bear tapped"
    );
}

#[test]
fn attackers_declared_at_target_3p_per_defender_and_exact_count_one() {
    // Breena-style: fires once per attacked opponent with 1+ attackers.
    static PER_OPPONENT_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::AttackersDeclaredAtTarget {
            attackers: &Filter::CREATURE,
            attacked: AttackedTargetKind::Opponent,
            exact_count: None,
        },
        &[Effect::LoseLife {
            target: PlayerRel::EventPlayer,
            amount: Amount::Fixed(2),
        }]
    )];
    // Jin Sakai / Jiang Yanggu style: fires only when NO other creatures attack that player.
    static ALONE_AT_PLAYER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::AttackersDeclaredAtTarget {
            attackers: &Filter::YOUR_CREATURE,
            attacked: AttackedTargetKind::Player,
            exact_count: Some(1),
        },
        &[
            Effect::PumpEventObject {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::LoseLife {
                target: PlayerRel::EventPlayer,
                amount: Amount::Fixed(3),
            },
        ]
    )];
    // Filtered lone-flyer check: still blocked when a non-flyer also attacks the same player.
    static YOUR_FLYER: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByYou,
        Filter::HasKeyword(KeywordSet::FLYING),
    ]);
    static LONE_FLYER_AT_PLAYER: &[AbilityDef] = &[triggered!(
        Trigger::AttackersDeclaredAtTarget {
            attackers: &YOUR_FLYER,
            attacked: AttackedTargetKind::Player,
            exact_count: Some(1),
        },
        &[Effect::gain_life(5)]
    )];

    let breena = card(
        C0,
        face("Per-Opponent Watcher", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        PER_OPPONENT_ABILITIES,
    );
    let jin = card(
        C1,
        face("Lone-At-Defender Watcher", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ALONE_AT_PLAYER_ABILITIES,
    );
    let lone_flyer = card(
        C2,
        face("Lone Flyer Watcher", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        LONE_FLYER_AT_PLAYER,
    );
    let a1 = card(
        C3,
        creature_face("Soldier One", "{1}", 1, 1),
        KeywordSet::HASTE,
        &[],
    );
    let a2 = card(
        C4,
        creature_face("Flyer Two", "{1}", 1, 1),
        KeywordSet::HASTE.union(KeywordSet::FLYING),
        &[],
    );
    let b1 = card(
        C5,
        creature_face("Duelist Three", "{1}", 1, 1),
        KeywordSet::HASTE,
        &[],
    );

    let mut engine = bench(
        1304,
        vec![breena, jin, lone_flyer, a1, a2, b1],
        [
            Seat::with(&[C0, C1, C2, C3, C4, C5]),
            Seat::default(),
            Seat::default(),
        ],
    );

    to_main(&mut engine, me());
    let w_opp = the(&engine, ZoneLocation::Battlefield, C0);
    let w_jin = the(&engine, ZoneLocation::Battlefield, C1);
    let w_fly = the(&engine, ZoneLocation::Battlefield, C2);
    let s1 = the(&engine, ZoneLocation::Battlefield, C3);
    let f2 = the(&engine, ZoneLocation::Battlefield, C4);
    let d3 = the(&engine, ZoneLocation::Battlefield, C5);

    // P0 attacks P1 with 2 creatures (Soldier One + Flyer Two) and P2 with 1 creature (Duelist Three).
    declare_attacks_and_settle(
        &mut engine,
        me(),
        vec![
            (s1, Defender::Player(them())),
            (f2, Defender::Player(them())),
            (d3, Defender::Player(third())),
        ],
    );

    assert_eq!(
        triggered(&engine, w_opp),
        2,
        "per-opponent trigger fires once for P1 and once for P2"
    );
    assert_eq!(
        triggered(&engine, w_jin),
        1,
        "exact_count: Some(1) fires ONLY for P2 (1 attacker), NOT for P1 (2 attackers)"
    );
    assert_eq!(
        triggered(&engine, w_fly),
        0,
        "exact_count: Some(1) with Flying filter must NOT fire on P1 where a non-flyer also attacks"
    );
    assert_eq!(life(&engine, me()), 20);
    assert_eq!(
        life(&engine, them()),
        18,
        "P1 lost 2 life from per-opponent only"
    );
    assert_eq!(
        life(&engine, third()),
        15,
        "P2 lost 2 (per-opponent) + 3 (lone-at-defender) = 5 life"
    );
    assert_eq!(
        engine.state().object(d3).unwrap().characteristics().power,
        Some(3),
        "Duelist Three received the lone-at-defender +2/+2 pump"
    );
    assert_eq!(
        engine.state().object(s1).unwrap().characteristics().power,
        Some(1)
    );
    assert_eq!(
        engine.state().object(f2).unwrap().characteristics().power,
        Some(1)
    );
}

#[test]
fn attackers_declared_at_target_planeswalker_and_wulfgar_multiplier_and_suppress() {
    static PW_WATCHER: &[AbilityDef] = &[triggered!(
        Trigger::AttackersDeclaredAtTarget {
            attackers: &Filter::YOUR_CREATURE,
            attacked: AttackedTargetKind::Planeswalker,
            exact_count: Some(1),
        },
        &[Effect::LoseLife {
            target: PlayerRel::EventPlayer,
            amount: Amount::Fixed(2),
        }]
    )];
    static PLAYER_ONLY_WATCHER: &[AbilityDef] = &[triggered!(
        Trigger::AttackersDeclaredAtTarget {
            attackers: &Filter::YOUR_CREATURE,
            attacked: AttackedTargetKind::Player,
            exact_count: None,
        },
        &[Effect::gain_life(10)]
    )];
    static FIRST_MAIN_PROBE: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::FirstMain,
            whose: PlayerRel::You,
        },
        &[Effect::gain_life(1)]
    )];
    static WULFGAR_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplier {
            source_filter: &Filter::ControlledByYou,
            event: TriggerEventKind::Attack,
        },
    )];
    static SUPPRESS_ATTACKS: &[AbilityDef] =
        &[AbilityDef::Replacement(ReplacementRule::TriggerSuppress {
            source_filter: &Filter::Any,
            event: TriggerEventKind::Attack,
        })];

    let mut pw_face = face("Test Walker", "{3}", TypeSet::PLANESWALKER);
    pw_face.loyalty = Some(5);

    let w_pw = card(
        C0,
        face("PW Watcher", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        PW_WATCHER,
    );
    let w_player = card(
        C1,
        face("Player-Only Watcher", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        PLAYER_ONLY_WATCHER,
    );
    let w_main = card(
        C2,
        face("Main Probe", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        FIRST_MAIN_PROBE,
    );
    let wulfgar = card(
        C3,
        creature_face("Wulfgar Proxy", "{3}{R}{G}", 4, 4),
        KeywordSet::EMPTY,
        WULFGAR_ABILITIES,
    );
    let raider = card(
        C4,
        creature_face("Swift Raider", "{1}{R}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let walker = card(C5, pw_face, KeywordSet::EMPTY, &[]);
    let suppressor = card(
        C6,
        face("Attack Hush", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SUPPRESS_ATTACKS,
    );

    // Part A: Wulfgar doubles AttackersDeclaredAtTarget at a Planeswalker (2x -> 4 life lost)
    // while NOT doubling FirstMain (1x -> +1 life), and Player-only trigger does not fire.
    let mut engine = bench(
        1305,
        vec![w_pw, w_player, w_main, wulfgar, raider, walker],
        [Seat::with(&[C0, C1, C2, C3, C4]), Seat::with(&[C5])],
    );
    to_main(&mut engine, me());
    let pw_watch_id = the(&engine, ZoneLocation::Battlefield, C0);
    let pl_watch_id = the(&engine, ZoneLocation::Battlefield, C1);
    let main_id = the(&engine, ZoneLocation::Battlefield, C2);
    let raider_id = the(&engine, ZoneLocation::Battlefield, C4);
    let walker_id = the(&engine, ZoneLocation::Battlefield, C5);
    assert_eq!(
        triggered(&engine, main_id),
        1,
        "Wulfgar (TriggerEventKind::Attack) must not multiply FirstMain triggers"
    );
    assert_eq!(life(&engine, me()), 21);

    declare_attacks_and_settle(
        &mut engine,
        me(),
        vec![(raider_id, Defender::Planeswalker(walker_id))],
    );
    assert_eq!(
        triggered(&engine, pw_watch_id),
        2,
        "Wulfgar doubles AttackersDeclaredAtTarget on the attacked planeswalker"
    );
    assert_eq!(
        triggered(&engine, pl_watch_id),
        0,
        "AttackedTargetKind::Player does not fire when only a planeswalker is attacked"
    );
    assert_eq!(
        life(&engine, them()),
        16,
        "EventPlayer resolves to the attacked planeswalker's controller (2 * 2 = 4 life lost)"
    );

    // Part B: TriggerSuppress with TriggerEventKind::Attack suppresses AttackersDeclaredAtTarget.
    let mut suppressed = bench(
        1306,
        vec![w_pw, wulfgar, raider, walker, suppressor],
        [Seat::with(&[C0, C3, C4]), Seat::with(&[C5, C6])],
    );
    to_main(&mut suppressed, me());
    let pw_watch_id = the(&suppressed, ZoneLocation::Battlefield, C0);
    let raider_id = the(&suppressed, ZoneLocation::Battlefield, C4);
    let walker_id = the(&suppressed, ZoneLocation::Battlefield, C5);

    declare_attacks_and_settle(
        &mut suppressed,
        me(),
        vec![(raider_id, Defender::Planeswalker(walker_id))],
    );
    assert_eq!(
        triggered(&suppressed, pw_watch_id),
        0,
        "TriggerSuppress(Attack) suppresses AttackersDeclaredAtTarget even alongside Wulfgar"
    );
    assert_eq!(life(&suppressed, them()), 20);
}

// =========================================================================
// 3. LKI ReplacementRule::ExileInsteadOfGraveyard (CR 603.10a / CR 614.6)
// =========================================================================

#[test]
fn lki_exile_etching_of_kumano_source_controller_this_turn_including_tokens() {
    static ETCHING_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ExileInsteadOfGraveyard {
            filter: &Filter::CREATURE,
            origin: Some(ZoneRef::Battlefield),
            owner: None,
            controller_lki: None,
            damaged_by: DamagedByScope::SourceControllerThisTurn,
            link_to_source: false,
            counter: None,
        },
    )];
    static DIES_GAIN_LIFE: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::gain_life(5)]
    )];
    static PING_ONE: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static KILL_TARGET: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static GOBLIN_TOKEN: TokenDef = TokenDef {
        name: "Goblin",
        types: TypeSet::CREATURE,
        power: Some(2),
        toughness: Some(2),
        abilities: DIES_GAIN_LIFE,
        ..TokenDef::DEFAULT
    };
    static MAKE_TOKEN: &[AbilityDef] = &[free(
        &[Effect::CreateToken {
            token: &GOBLIN_TOKEN,
        }],
        None,
    )];

    let etching = card(
        C0,
        creature_face("Etching Proxy", "{R}", 2, 2),
        KeywordSet::EMPTY,
        ETCHING_ABILITIES,
    );
    let pinger = card(
        C1,
        face("Pinger", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PING_ONE,
    );
    let killer = card(
        C2,
        face("Killer", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        KILL_TARGET,
    );
    let victim_a = card(
        C3,
        creature_face("Victim Damaged By Me", "{2}", 3, 3),
        KeywordSet::EMPTY,
        DIES_GAIN_LIFE,
    );
    let victim_b = card(
        C4,
        creature_face("Victim Undamaged", "{2}", 3, 3),
        KeywordSet::EMPTY,
        DIES_GAIN_LIFE,
    );
    let victim_c = card(
        C5,
        creature_face("Victim Damaged By Opponent", "{2}", 3, 3),
        KeywordSet::EMPTY,
        DIES_GAIN_LIFE,
    );
    let token_maker = card(
        C6,
        face("Token Maker", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        MAKE_TOKEN,
    );

    let mut engine = bench(
        1307,
        vec![
            etching,
            pinger,
            killer,
            victim_a,
            victim_b,
            victim_c,
            token_maker,
        ],
        [Seat::with(&[C0, C1, C2]), Seat::with(&[C1, C3, C4, C5, C6])],
    );

    to_main(&mut engine, me());
    let my_pinger = objects(&engine, ZoneLocation::Battlefield, C1)
        .into_iter()
        .find(|id| engine.state().object(*id).unwrap().controller == me())
        .unwrap();
    let opp_pinger = objects(&engine, ZoneLocation::Battlefield, C1)
        .into_iter()
        .find(|id| engine.state().object(*id).unwrap().controller == them())
        .unwrap();
    let my_killer = the(&engine, ZoneLocation::Battlefield, C2);
    let va = the(&engine, ZoneLocation::Battlefield, C3);
    let vb = the(&engine, ZoneLocation::Battlefield, C4);
    let vc = the(&engine, ZoneLocation::Battlefield, C5);
    let opp_maker = the(&engine, ZoneLocation::Battlefield, C6);

    // 1. My pinger deals 1 damage to Victim A, then Killer destroys Victim A -> EXILED, no dies trigger!
    activate(&mut engine, me(), my_pinger, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![va],
                players: vec![],
            },
        )
        .expect("target Victim A");
    settle(&mut engine, me());

    activate(&mut engine, me(), my_killer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![va],
                players: vec![],
            },
        )
        .expect("destroy Victim A");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(va).unwrap().zone,
        Zone::Exile,
        "Victim A dealt damage by a source you controlled is exiled instead of dying"
    );
    assert_eq!(
        life(&engine, them()),
        20,
        "Victim A's dies trigger must not fire when exiled instead"
    );

    // 2. Undamaged Victim B destroyed -> goes to GRAVEYARD and dies trigger fires (+5 life -> 25).
    activate(&mut engine, me(), my_killer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![vb],
                players: vec![],
            },
        )
        .expect("destroy Victim B");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(vb).unwrap().zone,
        Zone::Graveyard,
        "Undamaged Victim B dies normally"
    );
    assert_eq!(life(&engine, them()), 25);

    // 3. Opponent's pinger deals 1 damage to Victim C, then Killer destroys Victim C -> GRAVEYARD!
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass to opponent");
    activate(&mut engine, them(), opp_pinger, 0);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![vc],
                players: vec![],
            },
        )
        .expect("opp pings Victim C");
    settle(&mut engine, me());

    activate(&mut engine, me(), my_killer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![vc],
                players: vec![],
            },
        )
        .expect("destroy Victim C");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(vc).unwrap().zone,
        Zone::Graveyard,
        "Victim C damaged only by opponent's source is NOT exiled by your Etching of Kumano"
    );
    assert_eq!(life(&engine, them()), 30);

    // 4. Opponent creates a creature TOKEN; my pinger damages it; Killer destroys it ->
    //    token is exiled instead of dying (no dies trigger, life stays 30!).
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass to opponent");
    activate(&mut engine, them(), opp_maker, 0);
    settle(&mut engine, me());
    let token_id = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| !o.is_card() && o.controller == them())
        })
        .expect("opponent creature token on battlefield");

    let died_before = engine.state().per_turn.creatures_died;
    activate(&mut engine, me(), my_pinger, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![token_id],
                players: vec![],
            },
        )
        .expect("ping token");
    settle(&mut engine, me());

    activate(&mut engine, me(), my_killer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![token_id],
                players: vec![],
            },
        )
        .expect("destroy token");
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        30,
        "Damaged creature token is exiled instead of dying, so its dies trigger must not fire"
    );
    assert_eq!(
        engine.state().per_turn.creatures_died,
        died_before,
        "Exiled token did not die"
    );
}

#[test]
fn lki_exile_incendiary_oracle_source_this_turn_and_simultaneous_combat_death() {
    static ORACLE_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ExileInsteadOfGraveyard {
            filter: &Filter::CREATURE,
            origin: Some(ZoneRef::Battlefield),
            owner: None,
            controller_lki: None,
            damaged_by: DamagedByScope::SourceThisTurn,
            link_to_source: false,
            counter: None,
        },
    )];
    static DIES_GAIN_LIFE: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::gain_life(7)]
    )];
    static PING_ONE: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static KILL_TARGET: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let oracle = card(
        C0,
        creature_face("Incendiary Oracle Proxy", "{1}{R}", 2, 2),
        KeywordSet::HASTE,
        ORACLE_ABILITIES,
    );
    let other_pinger = card(
        C1,
        face("Other Source", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PING_ONE,
    );
    let killer = card(
        C2,
        face("Killer", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        KILL_TARGET,
    );
    let non_oracle_victim = card(
        C3,
        creature_face("Bear Damaged By Other", "{1}{G}", 3, 3),
        KeywordSet::EMPTY,
        DIES_GAIN_LIFE,
    );
    let blocker = card(
        C4,
        creature_face("Trading Blocker", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        DIES_GAIN_LIFE,
    );

    let mut engine = bench(
        1308,
        vec![oracle, other_pinger, killer, non_oracle_victim, blocker],
        [Seat::with(&[C0, C1, C2]), Seat::with(&[C3, C4])],
    );

    to_main(&mut engine, me());
    let oracle_id = the(&engine, ZoneLocation::Battlefield, C0);
    let pinger_id = the(&engine, ZoneLocation::Battlefield, C1);
    let killer_id = the(&engine, ZoneLocation::Battlefield, C2);
    let other_victim = the(&engine, ZoneLocation::Battlefield, C3);
    let blocker_id = the(&engine, ZoneLocation::Battlefield, C4);

    // 1. Another source you control deals damage to Bear Damaged By Other -> NOT exiled by Oracle!
    activate(&mut engine, me(), pinger_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![other_victim],
                players: vec![],
            },
        )
        .expect("ping other victim");
    settle(&mut engine, me());

    activate(&mut engine, me(), killer_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![other_victim],
                players: vec![],
            },
        )
        .expect("destroy other victim");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(other_victim).unwrap().zone,
        Zone::Graveyard,
        "SourceThisTurn must NOT exile creatures damaged only by a different source you control"
    );
    assert_eq!(life(&engine, them()), 27);

    // 2. Incendiary Oracle (2/2) attacks and trades with Trading Blocker (2/2) in combat!
    //    Both take lethal combat damage simultaneously and leave in the same SBA batch.
    //    Even though Oracle dies simultaneously, Blocker MUST be exiled (CR 614.6) and NOT trigger dies!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(oracle_id, Defender::Player(them()))],
            },
        )
        .expect("Oracle attacks");
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == them()),
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker_id, oracle_id)],
            },
        )
        .expect("Blocker blocks Oracle");

    // Walk into second main phase so combat damage and SBAs settle completely.
    walk_to_main_count(&mut engine, me(), 2);
    assert_eq!(
        engine.state().object(oracle_id).unwrap().zone,
        Zone::Graveyard,
        "Incendiary Oracle itself dies to combat damage"
    );
    assert_eq!(
        engine.state().object(blocker_id).unwrap().zone,
        Zone::Exile,
        "Trading Blocker dealt damage by Incendiary Oracle is exiled even when Oracle dies simultaneously"
    );
    assert_eq!(
        life(&engine, them()),
        27,
        "Exiled blocker's dies trigger must not fire"
    );
}

#[test]
fn lki_exile_valgavoth_linked_exile_alt_cast_controller_lki_and_counter() {
    static NONTOKEN_CARD: Filter = Filter::Not(&Filter::IsToken);
    static VALGAVOTH_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard {
            filter: &NONTOKEN_CARD,
            origin: None,
            owner: Some(PlayerRel::EachOpponent),
            controller_lki: Some(PlayerRel::EachOtherPlayer),
            damaged_by: DamagedByScope::Any,
            link_to_source: true,
            counter: None,
        }),
        static_ability!(
            Filter::Any,
            Modifier::MayCastWithAltCost {
                who: PlayerRel::You,
                zones: AltCastZones::LinkedExile,
                grant_zone_permission: true,
                allow_lands: false,
                with_flash: false,
                max_mana_value: None,
                once_per_turn: false,
                price: AltCastPrice::PayLifeEqualToManaValue,
            }
        ),
    ];
    static KILL_TARGET: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    // Gisa-style rule with exile counter + controller_lki: EachOpponent.
    static GISA_COUNTER_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ExileInsteadOfGraveyard {
            filter: &Filter::CREATURE,
            origin: Some(ZoneRef::Battlefield),
            owner: None,
            controller_lki: Some(PlayerRel::EachOpponent),
            damaged_by: DamagedByScope::Any,
            link_to_source: true,
            counter: Some(counters::VOID),
        },
    )];
    static STEAL_TARGET: &[AbilityDef] = &[free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::GainControl,
            Duration::Indefinitely,
        )],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let valg = card(
        C0,
        creature_face("Valgavoth Proxy", "{6}{B}{B}{B}", 9, 9),
        KeywordSet::EMPTY,
        VALGAVOTH_ABILITIES,
    );
    let killer = card(
        C1,
        face("Killer", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        KILL_TARGET,
    );
    let bear = card(
        C2,
        creature_face("Opponent Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let gisa = card(
        C3,
        creature_face("Gisa Proxy", "{2}{B}{B}", 4, 4),
        KeywordSet::EMPTY,
        GISA_COUNTER_ABILITIES,
    );
    let stealer = card(
        C4,
        face("Control Stealer", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        STEAL_TARGET,
    );
    let my_own_bear = card(
        C5,
        creature_face("My Stolen Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // Part A: Valgavoth — mill/discard from hand -> linked exile -> cast by paying life ->
    // when it later dies under YOUR control, controller_lki prevents re-exile!
    let mut engine = bench(
        1309,
        vec![valg, killer, bear],
        [Seat::with(&[C0, C1]), Seat::default().holding(&[C2])],
    );
    to_main(&mut engine, me());
    let valg_id = the(&engine, ZoneLocation::Battlefield, C0);
    let killer_id = the(&engine, ZoneLocation::Battlefield, C1);
    let opp_bear = the(&engine, ZoneLocation::Hand(them()), C2);

    // Discard opponent's Bear from Hand -> Graveyard; Valgavoth replaces with linked Exile!
    engine
        .state
        .move_object(
            opp_bear,
            ZoneLocation::Graveyard(them()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("discard opponent Bear");
    refresh_offer(&mut engine);

    let exiled_bear = engine.state().object(opp_bear).unwrap();
    assert_eq!(
        exiled_bear.zone,
        Zone::Exile,
        "Opponent card going from hand to graveyard is exiled by Valgavoth"
    );
    assert!(
        exiled_bear.riders.contains(&Rider::Linked {
            host: valg_id,
            until: None,
        }),
        "Exiled card carries Rider::Linked to Valgavoth"
    );

    // Cast opponent's exiled Bear using Valgavoth's LinkedExile PayLifeEqualToManaValue permission!
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&opp_bear),
        "Valgavoth offers opponent's linked exiled Bear to cast"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: opp_bear })
        .expect("begin casting linked exiled Bear");
    if let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    {
        assert_eq!(player, me());
        let mode_idx = options
            .iter()
            .position(|m| {
                matches!(
                    m.kind,
                    CastModeKind::LibraryTopLife { .. } | CastModeKind::Permission(_)
                )
            })
            .expect("alt-cost life mode offered");
        engine
            .apply(me(), PlayerAction::ChooseMode(mode_idx))
            .expect("choose pay-life cast mode");
    }
    settle(&mut engine, me());

    assert_eq!(
        life(&engine, me()),
        18,
        "Paid 2 life (mana value of {{1}}{{G}}) to cast Bear from Valgavoth's exile"
    );
    let live_bear = engine.state().object(opp_bear).unwrap();
    assert_eq!(live_bear.zone, Zone::Battlefield);
    assert_eq!(live_bear.owner, them());
    assert_eq!(live_bear.controller, me());

    // Now destroy Bear while YOU control it -> Valgavoth's controller_lki (EachOtherPlayer)
    // sees that YOU controlled it on the battlefield, so it goes to opponent's GRAVEYARD!
    activate(&mut engine, me(), killer_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .expect("destroy Bear we control");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(opp_bear).unwrap().zone,
        Zone::Graveyard,
        "Card owned by opponent that YOU controlled on battlefield goes to graveyard (not re-exiled)"
    );

    // Part B: Gisa-style controller_lki (EachOpponent) + counter: Some(CounterKind::Void):
    // Your own creature stolen by Opponent dies -> exiled with Void counter & linked to Gisa!
    let mut gisa_engine = bench(
        1310,
        vec![gisa, killer, stealer, my_own_bear],
        [Seat::with(&[C3, C1, C5]), Seat::with(&[C4])],
    );
    to_main(&mut gisa_engine, me());
    let gisa_id = the(&gisa_engine, ZoneLocation::Battlefield, C3);
    let killer_id = the(&gisa_engine, ZoneLocation::Battlefield, C1);
    let stealer_id = the(&gisa_engine, ZoneLocation::Battlefield, C4);
    let my_bear = the(&gisa_engine, ZoneLocation::Battlefield, C5);

    // Opponent steals My Stolen Bear via GainControl.
    gisa_engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass to opponent");
    activate(&mut gisa_engine, them(), stealer_id, 0);
    gisa_engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![my_bear],
                players: vec![],
            },
        )
        .expect("opponent steals my Bear");
    settle(&mut gisa_engine, me());
    assert_eq!(gisa_engine.state().object(my_bear).unwrap().owner, me());
    assert_eq!(
        gisa_engine.state().object(my_bear).unwrap().controller,
        them()
    );

    // Destroy My Stolen Bear while Opponent controls it -> Gisa exiles it (owner == me,
    // controller_lki == them matches EachOpponent!), links to Gisa, and places a Void counter!
    activate(&mut gisa_engine, me(), killer_id, 0);
    gisa_engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_bear],
                players: vec![],
            },
        )
        .expect("destroy stolen Bear");
    settle(&mut gisa_engine, me());
    let exiled = gisa_engine.state().object(my_bear).unwrap();
    assert_eq!(
        exiled.zone,
        Zone::Exile,
        "Creature owned by you but controlled by opponent is exiled by Gisa's controller_lki"
    );
    assert_eq!(
        exiled.counters.get(counters::VOID),
        1,
        "ExileInsteadOfGraveyard placed the configured exile counter"
    );
    assert!(
        exiled.riders.contains(&Rider::Linked {
            host: gisa_id,
            until: None,
        }),
        "Exiled card is linked to Gisa"
    );
}
