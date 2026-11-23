//! Engine Wave 15c deterministic rule tests:
//! 1. Resolution-scoped `Effect::CaptureAffected`, `Effect::ForEachRemembered`,
//!    `ChooseZone::Remembered`, `Filter::{IsRemembered, NotRemembered,
//!    SharesCardTypeWithRemembered, SharesNameWithRemembered}`,
//!    `PlayerRel::RememberedController`, and `Amount::{RememberedCount,
//!    RememberedManaValue, RememberedPower, RememberedToughness}` (`CR 400.7`,
//!    `CR 608.2c`, `CR 608.2h`, `CR 107.1b`):
//!    - *Act of Authority*: optional upkeep trigger exiling target artifact or
//!      enchantment and giving control of `ThisObject` to `RememberedController`,
//!    - *Bringer of the Last Gift*: `SacrificeAll` other creatures then return
//!      all creature cards matching `Filter::NotRemembered`,
//!    - *Crovax the Cursed*: optional upkeep sacrifice branching on
//!      `Amount::RememberedCount` and reading frozen pre-departure LKI stats
//!      (`RememberedPower`, `RememberedToughness`, `RememberedManaValue`),
//!    - *Arjun, the Shifting Flame*: put hand on library bottom in chosen order
//!      then draw `Amount::RememberedCount`, with `snapshot_hash()` stability
//!      while suspended at `Pending::Arrange`,
//!    - `Dig`, `PeekAndReveal`, `RevealFromHand`, `ChooseCard` +
//!      `ChooseZone::Remembered`, `CreateToken` + `ForEachRemembered`, `Mill` +
//!      `SharesCardTypeWithRemembered` + `SharesNameWithRemembered`, and nested
//!      `CaptureAffected` scope restoration + suspended `then` hash stability.
//! 2. 3-slot independent target chains (`CR 601.2c`, `CR 602.2b`, `CR 603.3d`,
//!    `CR 608.2b`):
//!    - *Bazaar Trader*: Slot 1 `AnyPlayer` + Slot 2 `Object` -> Slot 1 gains
//!      control of Slot 2 via `PlayerRel::FirstTargetPlayerOrController`, plus
//!      `CR 608.2b` partial illegality when Slot 1 gains hexproof in response,
//!    - *Assassin Gauntlet*: ETB trigger with optional Slot 1 creature (`min=0`)
//!      + mandatory Slot 2 opponent (`TapAllOf`), including 0-creature board,
//!    - 3-creature distinct pump chain (`+3/+3`, `+2/+2`, `+1/+1`) +
//!      `Effect::WithAllTargets` ("those creatures gain vigilance until end of
//!      turn") across 3-creature, 1-creature, and partial-illegality boards.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::YesNoPrompt;
use crate::event::Cause;
use crate::object::Status;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    AbilityDef, Amount, ChooseMove, ChooseZone, Condition, CounterKind, Duration, Effect, Filter,
    Find, KeywordSet, Modifier, PeekPickDest, PeekRestDest, PlayerRel, StepKind, TargetConstraint,
    TargetReq, TargetSlot, TargetSpec, TokenDef, Trigger, activated, cost, spell, static_ability,
    triggered,
};
use baylee_core::generated::subtypes::{artifact, creature};

const C0: u32 = 99_850;
const C1: u32 = 99_851;
const C2: u32 = 99_852;
const C3: u32 = 99_853;
const C4: u32 = 99_854;
const C5: u32 = 99_855;
const C6: u32 = 99_856;
const C7: u32 = 99_857;
const C8: u32 = 99_858;
const C9: u32 = 99_859;

fn bench_main<const N: usize>(seed: u64, cards: Vec<&'static CardDef>, seats: [Seat; N]) -> Bench {
    let mut engine = bench(seed, cards, seats);
    to_main(&mut engine, me());
    engine
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

fn cast_from_hand(engine: &mut Bench, seat: PlayerId, card_idx: u32) -> ObjectId {
    let id = the(engine, ZoneLocation::Hand(seat), card_idx);
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(
        legal.castable.contains(&id),
        "expected {card_idx} ({id:?}) to be castable, got {:?}",
        legal.castable
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: id })
        .expect("cast starts");
    id
}

fn put_in_graveyard(engine: &mut Bench, owner: PlayerId, card_idx: u32) -> ObjectId {
    let id = the(engine, ZoneLocation::Hand(owner), card_idx);
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("move hand card to graveyard");
    refresh_offer(engine);
    id
}

fn put_on_library_top(engine: &mut Bench, owner: PlayerId, card_idx: u32) -> ObjectId {
    let id = objects(engine, ZoneLocation::Hand(owner), card_idx)
        .into_iter()
        .next()
        .expect("card in hand");
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Library(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("move hand card to library top");
    refresh_offer(engine);
    id
}

/// Walk forward until `seat`'s upkeep begins (stopping immediately if an upkeep
/// trigger asks for targets or sits on the stack).
fn walk_to_upkeep(engine: &mut Bench, seat: PlayerId) {
    for _ in 0..600 {
        if engine.state().turn.active == seat
            && engine.state().turn.step == crate::turn::Step::Upkeep
            && (matches!(engine.pending(), Pending::ChooseTargets { .. })
                || !engine.state().zones.stack_is_empty()
                || holds_priority_in(engine, seat, crate::turn::Step::Upkeep))
        {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            walk_past(engine, &pending),
            "unexpected question while walking to upkeep: {pending:?}"
        );
    }
    panic!("never reached {seat:?}'s upkeep");
}

fn walk_to_end_step(engine: &mut Bench, seat: PlayerId) {
    walk_until(engine, |e| {
        holds_priority_in(e, seat, crate::turn::Step::End)
    });
}

// =========================================================================
// 1. Act of Authority — Exile target + RememberedController transfers source
// =========================================================================

#[test]
fn act_of_authority_exiles_target_and_transfers_source_to_remembered_controller() {
    static ART_OR_ENCH: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT]);
    static ACT_ETB_AND_UPKEEP: &[AbilityDef] = &[
        triggered!(
            Trigger::ETB,
            &[Effect::MayDo {
                effects: &[Effect::Exile {
                    target: TargetSpec::Object(&ART_OR_ENCH),
                }],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ART_OR_ENCH))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::CaptureAffected {
                    first: &[Effect::Exile {
                        target: TargetSpec::Object(&ART_OR_ENCH),
                    }],
                    then: &[Effect::GainControlEx {
                        target: TargetSpec::ThisObject,
                        to: PlayerRel::RememberedController,
                        duration: Duration::Indefinitely,
                        untap: false,
                        haste_until_eot: false,
                    }],
                }],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ART_OR_ENCH))),
        ),
    ];

    let act = card(
        C0,
        face("Act of Authority", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ACT_ETB_AND_UPKEEP,
    );
    let opp_relic = card(
        C1,
        face("Opp Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_idol = card(
        C2,
        face("Opp Idol", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let my_seal = card(
        C3,
        face("My Seal", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1501,
        vec![act, opp_relic, opp_idol, my_seal],
        [
            Seat::with(&[C3]).holding(&[C0]),
            Seat::with(&[C1, C2]).holding(&[]),
        ],
    );
    let relic = the(&engine, ZoneLocation::Battlefield, C1);
    let idol = the(&engine, ZoneLocation::Battlefield, C2);
    let seal = the(&engine, ZoneLocation::Battlefield, C3);

    // Cast Act of Authority; ETB exiles `relic` without changing Act's controller.
    let act_id = cast_from_hand(&mut engine, me(), C0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(relic).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(act_id).unwrap().controller, me());

    // Advance to me()'s next upkeep: target `idol` (controlled by `them()`),
    // accept `MayDo` -> `idol` is exiled AND `them()` gains control of `act_id`!
    walk_to_end_step(&mut engine, them());
    walk_to_upkeep(&mut engine, me());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![idol],
                    players: vec![],
                },
            )
            .unwrap();
    }
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());

    assert_eq!(engine.state().object(idol).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().object(act_id).unwrap().controller,
        them(),
        "RememberedController transferred Act of Authority to exiled permanent's controller"
    );

    // Advance to `them()`'s upkeep: `them()` now controls Act of Authority!
    // First decline `MayDo` -> `seal` stays on battlefield and `them()` keeps control.
    walk_to_upkeep(&mut engine, them());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![seal],
                    players: vec![],
                },
            )
            .unwrap();
    }
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, them());
    assert_eq!(engine.state().object(seal).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().object(act_id).unwrap().controller, them());

    // Next upkeep of `them()`: accept `MayDo` targeting `seal` (controlled by `me()`)
    // -> `seal` is exiled and `act_id` returns to `me()`'s control!
    walk_to_end_step(&mut engine, me());
    walk_to_upkeep(&mut engine, them());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![seal],
                    players: vec![],
                },
            )
            .unwrap();
    }
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, them());
    assert_eq!(engine.state().object(seal).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().object(act_id).unwrap().controller,
        me(),
        "Act of Authority returned to me() after them() exiled me()'s enchantment"
    );
}

// =========================================================================
// 2. Bringer of the Last Gift — SacrificeAll + Filter::NotRemembered
// =========================================================================

#[test]
fn bringer_of_the_last_gift_sacrifices_all_and_returns_only_not_remembered() {
    static OTHER_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Another]);
    static CREATURE_NOT_REMEMBERED: Filter =
        Filter::And(&[Filter::CREATURE, Filter::NotRemembered]);
    static BRINGER_AB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[Effect::CaptureAffected {
            first: &[Effect::SacrificeAll {
                filter: &OTHER_CREATURE,
            }],
            then: &[Effect::EachPlayerGraveyardToBattlefield {
                filter: &CREATURE_NOT_REMEMBERED,
            }],
        }],
        condition = Some(Condition::SourceWasCastByYou),
    )];

    let bringer = card(
        C0,
        creature_face("Bringer of the Last Gift", "{0}", 6, 6),
        KeywordSet::FLYING,
        BRINGER_AB,
    );
    let my_live = card(
        C1,
        creature_face("My Live Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_live = card(
        C2,
        creature_face("Opp Live Bear", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let my_dead = card(
        C3,
        creature_face("My Fallen Titan", "{0}", 5, 5),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_dead = card(
        C4,
        creature_face("Opp Fallen Elf", "{0}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1502,
        vec![bringer, my_live, opp_live, my_dead, opp_dead],
        [
            Seat::with(&[C1]).holding(&[C0, C3]),
            Seat::with(&[C2]).holding(&[C4]),
        ],
    );
    let my_live_id = the(&engine, ZoneLocation::Battlefield, C1);
    let opp_live_id = the(&engine, ZoneLocation::Battlefield, C2);
    let my_dead_id = put_in_graveyard(&mut engine, me(), C3);
    let opp_dead_id = put_in_graveyard(&mut engine, them(), C4);

    let bringer_id = cast_from_hand(&mut engine, me(), C0);
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(bringer_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(my_live_id).unwrap().zone,
        Zone::Graveyard,
        "sacrificed creature must NOT return under Filter::NotRemembered"
    );
    assert_eq!(
        engine.state().object(opp_live_id).unwrap().zone,
        Zone::Graveyard,
        "opponent's sacrificed creature must NOT return under Filter::NotRemembered"
    );
    assert_eq!(
        engine.state().object(my_dead_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(my_dead_id).unwrap().controller, me());
    assert_eq!(
        engine.state().object(opp_dead_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(opp_dead_id).unwrap().controller,
        them()
    );
}

// =========================================================================
// 3. Crovax — Optional sacrifice -> branch on RememberedCount + LKI stats
// =========================================================================

#[test]
fn crovax_optional_sacrifice_branches_on_remembered_count_and_reads_lki_stats() {
    static ANTHEM_AB: &[AbilityDef] = &[static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::ModifyPT(1, 2)
    )];
    static CROVAX_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::ETB,
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(4),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::CaptureAffected {
                first: &[Effect::MayDo {
                    effects: &[Effect::SacrificeFilter {
                        who: PlayerRel::You,
                        filter: &Filter::CREATURE,
                    }],
                }],
                then: &[Effect::IfCondition {
                    condition: Condition::AmountAtLeast(Amount::RememberedCount, 1),
                    then: &[
                        Effect::AddCounterSource {
                            kind: CounterKind::P1P1,
                            amount: Amount::RememberedCount,
                        },
                        Effect::GainLife {
                            amount: Amount::RememberedToughness,
                        },
                        Effect::LoseLife {
                            target: PlayerRel::EachOpponent,
                            amount: Amount::RememberedPower,
                        },
                        Effect::mana_dynamic(
                            baylee_core::mana::ManaColor::Colorless,
                            Amount::RememberedManaValue,
                        ),
                    ],
                    otherwise: &[Effect::RemoveCounterSelf {
                        kind: CounterKind::P1P1,
                        n: 1,
                    }],
                }],
            }],
        ),
    ];

    let crovax = card(
        C0,
        creature_face("Crovax Probe", "{0}", 0, 0),
        KeywordSet::EMPTY,
        CROVAX_AB,
    );
    // Printed 2/3 with MV 3; under +1/+2 anthem it is 3/5 on the battlefield!
    let fodder = card(
        C1,
        creature_face("Vanguard Fodder", "{2}{B}", 2, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let anthem = card(
        C2,
        face("Dark Crusade", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ANTHEM_AB,
    );

    let mut engine = bench_main(
        1503,
        vec![crovax, fodder, anthem],
        [
            Seat::with(&[C1, C2]).holding(&[C0]),
            Seat::with(&[]).holding(&[]),
        ],
    );
    let fodder_id = the(&engine, ZoneLocation::Battlefield, C1);
    let crovax_id = cast_from_hand(&mut engine, me(), C0);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(crovax_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );

    // First upkeep: ACCEPT `MayDo` and sacrifice `fodder` (projected 3/5, MV 3).
    walk_to_end_step(&mut engine, them());
    walk_to_upkeep(&mut engine, me());
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder_id],
            },
        )
        .unwrap();

    assert_eq!(
        engine
            .state()
            .object(crovax_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5
    );
    assert_eq!(
        life(&engine, me()),
        25,
        "RememberedToughness read projected 5"
    );
    assert_eq!(
        life(&engine, them()),
        17,
        "RememberedPower read projected 3"
    );
    assert_eq!(
        engine.state().players[usize::from(me().get())]
            .mana_pool
            .total(),
        3,
        "RememberedManaValue read MV 3"
    );
    settle(&mut engine, me());

    // Second upkeep: DECLINE `MayDo` -> `RememberedCount == 0` -> `otherwise`
    // removes 1 +1/+1 counter from Crovax (5 -> 4)!
    walk_to_end_step(&mut engine, them());
    walk_to_upkeep(&mut engine, me());
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(crovax_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4,
        "declining sacrifice left RememberedCount == 0 and removed a +1/+1 counter"
    );
}

// =========================================================================
// 4. Arjun — Hand to library bottom + Draw RememberedCount + snapshot_hash()
// =========================================================================

#[test]
fn arjun_hand_to_library_bottom_draws_remembered_count_and_snapshot_hash_stability() {
    static YOUR_SPELL: Filter = Filter::ControlledByYou;
    static ARJUN_AB: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&YOUR_SPELL),
        &[Effect::CaptureAffected {
            first: &[Effect::ChooseCardEx {
                chooser: PlayerRel::You,
                zone: ChooseZone::Hand(PlayerRel::You),
                filter: &Filter::Any,
                min: 0,
                max: Amount::Fixed(0),
                reveal_chosen: false,
                movement: ChooseMove::None,
                not_chosen_movement: ChooseMove::BottomOfLibraryChosenOrder,
                then: &[],
                for_unchosen: &[],
            }],
            then: &[Effect::DrawCards {
                amount: Amount::RememberedCount,
            }],
        }],
    )];
    static SPARK_AB: &[AbilityDef] = &[spell!(&[Effect::gain_life(1)])];

    let arjun = card(
        C0,
        creature_face("Arjun Probe", "{0}", 5, 5),
        KeywordSet::FLYING,
        ARJUN_AB,
    );
    let cantrip = card(
        C1,
        face("Spark", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        SPARK_AB,
    );
    let h1 = card(
        C2,
        face("Hand One", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let h2 = card(
        C3,
        face("Hand Two", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let h3 = card(
        C4,
        face("Hand Three", "{3}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let d1 = card(
        C5,
        face("Deck One", "{4}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let d2 = card(
        C6,
        face("Deck Two", "{5}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let d3 = card(
        C7,
        face("Deck Three", "{6}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1504,
        vec![arjun, cantrip, h1, h2, h3, d1, d2, d3],
        [
            Seat::with(&[C0]).holding(&[C1, C2, C3, C4, C5, C6, C7]),
            Seat::with(&[]).holding(&[]),
        ],
    );
    let top_d3 = put_on_library_top(&mut engine, me(), C7);
    let top_d2 = put_on_library_top(&mut engine, me(), C6);
    let top_d1 = put_on_library_top(&mut engine, me(), C5);
    let id_h1 = the(&engine, ZoneLocation::Hand(me()), C2);
    let id_h2 = the(&engine, ZoneLocation::Hand(me()), C3);
    let id_h3 = the(&engine, ZoneLocation::Hand(me()), C4);

    // Cast `cantrip`: hand still has `[id_h1, id_h2, id_h3]` (3 cards).
    cast_from_hand(&mut engine, me(), C1);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    // Suspended at `Pending::Arrange` inside `CaptureAffected::first`!
    assert!(
        matches!(engine.pending(), Pending::Arrange { .. }),
        "expected Pending::Arrange for Arjun bottom order, got {:?}",
        engine.pending()
    );
    let snap_a = engine.snapshot_hash();
    let snap_b = engine.snapshot_hash();
    assert_eq!(
        snap_a, snap_b,
        "snapshot_hash() must be deterministic while suspended inside CaptureAffected::first"
    );
    let state_snap_a = engine.state().snapshot_hash();
    let state_snap_b = engine.state().snapshot_hash();
    assert_eq!(state_snap_a, state_snap_b);

    // Arrange `[id_h2, id_h3, id_h1]` onto the bottom -> draws 3 fresh cards!
    engine
        .apply(
            me(),
            PlayerAction::Arrange {
                piles: vec![vec![id_h2, id_h3, id_h1]],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(me()));
    assert_eq!(
        hand_after.len(),
        3,
        "Arjun drew RememberedCount == 3 cards after bottoming 3"
    );
    assert!(
        hand_after.contains(&top_d1)
            && hand_after.contains(&top_d2)
            && hand_after.contains(&top_d3),
        "Arjun drew the exact top 3 library cards"
    );
    let lib_after = engine.state().zones.list(ZoneLocation::Library(me()));
    assert_eq!(
        &lib_after[..3],
        &[id_h1, id_h3, id_h2],
        "bottom of library preserved chosen order (last placed at index 0)"
    );
}

// =========================================================================
// 5. Dig, ChooseZone::Remembered, ForEachRemembered, Mill +
//    SharesCardType/SharesName, nested CaptureAffected & suspended `then` hash
// =========================================================================

#[test]
fn capture_affected_dig_tokens_mill_shares_filters_and_nested_scopes() {
    static SOLDIER_TOKEN: TokenDef = TokenDef {
        name: "Soldier",
        types: TypeSet::CREATURE,
        subtypes: &[creature::SOLDIER],
        power: Some(2),
        toughness: Some(2),
        ..TokenDef::DEFAULT
    };

    static DIG_AND_NESTED_AB: &[AbilityDef] = &[spell!(&[Effect::CaptureAffected {
        first: &[Effect::LookAtTopFind {
            count: 3,
            filter: &Filter::CREATURE,
            found: Find::HAND,
            random: true,
        }],
        then: &[
            Effect::CaptureAffected {
                first: &[Effect::CreateTokenN {
                    token: &SOLDIER_TOKEN,
                    amount: Amount::Fixed(2),
                }],
                then: &[Effect::ForEachRemembered {
                    filter: &Filter::CREATURE,
                    effects: &[Effect::MayDo {
                        effects: &[Effect::GainLife {
                            amount: Amount::RememberedCount,
                        }],
                    }],
                }],
            },
            // Outer scope restored: outer RememberedCount is 1 (the 1 dig-kept card),
            // NOT 2 (the inner tokens)!
            Effect::LoseLife {
                target: PlayerRel::EachOpponent,
                amount: Amount::RememberedCount,
            },
        ],
    }])];

    static SHARES_OR: Filter = Filter::Or(&[
        Filter::SharesCardTypeWithRemembered,
        Filter::SharesNameWithRemembered,
    ]);
    static MILL_SHARES_AND_CHOOSE_REM_AB: &[AbilityDef] = &[spell!(&[Effect::CaptureAffected {
        first: &[Effect::Mill {
            target: PlayerRel::You,
            amount: Amount::Fixed(2),
        }],
        then: &[
            Effect::ExileAll {
                filter: &SHARES_OR,
                linked: false,
            },
            Effect::ChooseCard {
                chooser: PlayerRel::You,
                zone: ChooseZone::Remembered,
                filter: &Filter::Any,
                min: 1,
                max: Amount::Fixed(1),
                movement: ChooseMove::Hand,
                then: &[],
            },
        ],
    }])];

    let dig_spell = card(
        C0,
        face("Nested Dig Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        DIG_AND_NESTED_AB,
    );
    let mill_spell = card(
        C1,
        face("Mill Echo Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        MILL_SHARES_AND_CHOOSE_REM_AB,
    );
    let deck_bear = card(
        C2,
        creature_face("Echo Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let deck_relic = card(
        C3,
        face("Echo Relic", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let deck_bolt = card(
        C4,
        face("Echo Bolt", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &[],
    );
    let board_ench = card(
        C5,
        face("Safe Enchantment", "{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1505,
        vec![
            dig_spell, mill_spell, deck_bear, deck_relic, deck_bolt, board_ench,
        ],
        [
            Seat::with(&[C5]).holding(&[C0, C1, C4, C3, C2]),
            Seat::with(&[C3]).holding(&[]),
        ],
    );
    let _ = put_on_library_top(&mut engine, me(), C4);
    let _ = put_on_library_top(&mut engine, me(), C3);
    let top_bear = put_on_library_top(&mut engine, me(), C2);
    let opp_relic = the(&engine, ZoneLocation::Battlefield, C3);
    let my_ench = the(&engine, ZoneLocation::Battlefield, C5);

    // Clear forest cards below our 3 placed cards? No: `LookAtTopFind { count: 3 }`
    // looks at ONLY the top 3 cards (`top_bear`, `C3`, `C4`), picks `top_bear` to hand,
    // and bottoms `C3` and `C4`! So for `mill_spell` right after, put two fresh cards
    // on top!
    cast_from_hand(&mut engine, me(), C0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![top_bear],
            },
        )
        .unwrap();

    // Suspended at `MayDo` inside `ForEachRemembered` (pass 1 of 2 tokens)!
    assert!(matches!(engine.pending(), Pending::YesNo { .. }));
    let h_then_1 = engine.state().snapshot_hash();
    let h_then_2 = engine.state().snapshot_hash();
    assert_eq!(
        h_then_1, h_then_2,
        "state.snapshot_hash() must be stable while suspended inside CaptureAffected::then"
    );
    let eng_then_1 = engine.snapshot_hash();
    let eng_then_2 = engine.snapshot_hash();
    assert_eq!(eng_then_1, eng_then_2);

    // Accept pass 1 (+2 life from inner `RememberedCount == 2`), decline pass 2 (+0).
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());

    assert_eq!(
        life(&engine, me()),
        22,
        "inner CaptureAffected saw 2 created tokens"
    );
    assert_eq!(
        life(&engine, them()),
        19,
        "outer CaptureAffected restored RememberedCount == 1 (only the dig-kept card)"
    );

    // Move the bottomed `Echo Relic` and `Echo Bolt` back to library top so `Mill(2)`
    // mills them deterministically.
    let lib_cards = engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .clone();
    let bottom_two = [lib_cards[0], lib_cards[1]];
    for id in bottom_two {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    refresh_offer(&mut engine);

    cast_from_hand(&mut engine, me(), C1);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    let chosen_milled = match engine.pending() {
        Pending::ChooseCards { options, .. } => {
            assert_eq!(
                options.len(),
                2,
                "ChooseZone::Remembered offered exactly the 2 milled cards"
            );
            options[0]
        }
        other => panic!("expected ChooseCards from ChooseZone::Remembered, got {other:?}"),
    };
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![chosen_milled],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(opp_relic).unwrap().zone,
        Zone::Exile,
        "SharesCardTypeWithRemembered / SharesNameWithRemembered exiled matching permanent"
    );
    assert_eq!(
        engine.state().object(my_ench).unwrap().zone,
        Zone::Battlefield,
        "non-sharing enchantment stayed on battlefield"
    );
    assert_eq!(
        engine.state().object(chosen_milled).unwrap().zone,
        Zone::Hand,
        "ChooseZone::Remembered moved chosen milled card to hand"
    );
}

// =========================================================================
// 6. Bazaar Trader & Assassin Gauntlet — 2-slot independent target chains
// =========================================================================

#[test]
fn bazaar_trader_and_assassin_gauntlet_independent_multi_target_chains() {
    static TRADER_PERM: Filter = Filter::And(&[
        Filter::ControlledByYou,
        Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]),
    ]);
    static TRADER_AB: &[AbilityDef] = &[activated!(
        cost!(TapSelf),
        &[Effect::WithSecondTarget(&[Effect::GainControlEx {
            target: TargetSpec::Object(&Filter::Any),
            to: PlayerRel::FirstTargetPlayerOrController,
            duration: Duration::Indefinitely,
            untap: false,
            haste_until_eot: false,
        }])],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&TRADER_PERM))),
    )];
    static AEGIS_AB: &[AbilityDef] = &[spell!(&[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::PlayerHexproof {
            who: PlayerRel::You,
        },
        duration: Duration::UntilEndOfTurn,
    }])];
    static GAUNTLET_AB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[
            Effect::AttachSlot {
                object: TargetSlot::This,
                to: TargetSlot::First,
                optional: false,
            },
            Effect::WithSecondTarget(&[Effect::TapAllOf {
                who: PlayerRel::TargetPlayerOrController,
                filter: &Filter::CREATURE,
            }]),
        ],
        targets = Some(TargetReq::up_to_one(TargetSpec::Object(
            &Filter::YOUR_CREATURE,
        ))),
        second_targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )];

    let trader = card(
        C0,
        creature_face("Bazaar Trader", "{0}", 1, 1),
        KeywordSet::HASTE,
        TRADER_AB,
    );
    let relic = card(
        C1,
        face("Gift Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let idol = card(
        C2,
        face("Kept Idol", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let aegis = card(
        C3,
        face("Instant Aegis", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        AEGIS_AB,
    );
    let mut gauntlet_face = face("Assassin Gauntlet", "{0}", TypeSet::ARTIFACT);
    gauntlet_face.subtypes = &[artifact::EQUIPMENT];
    let gauntlet = card(C4, gauntlet_face, KeywordSet::EMPTY, GAUNTLET_AB);
    let opp_guard = card(
        C5,
        creature_face("Opp Guard", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1506,
        vec![trader, relic, idol, aegis, gauntlet, opp_guard],
        [
            Seat::with(&[C0, C1, C2]).holding(&[C4]),
            Seat::with(&[C5]).holding(&[C3]),
        ],
    );
    let trader_id = the(&engine, ZoneLocation::Battlefield, C0);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C1);
    let idol_id = the(&engine, ZoneLocation::Battlefield, C2);
    let opp_guard_id = the(&engine, ZoneLocation::Battlefield, C5);

    // 6a. Activate Bazaar Trader: Slot 1 = `them()`, Slot 2 = `relic_id` ->
    // `them()` permanently gains control of `relic_id`!
    activate(&mut engine, me(), trader_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic_id],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(relic_id).unwrap().controller,
        them(),
        "FirstTargetPlayerOrController transferred Slot 2 permanent to Slot 1 player"
    );

    // 6b. Next turn of `me()`: activate Bazaar Trader targeting `them()` + `idol_id`,
    // and in response `them()` casts `Instant Aegis` gaining player hexproof!
    walk_to_end_step(&mut engine, them());
    to_main(&mut engine, me());
    activate(&mut engine, me(), trader_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![idol_id],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, them(), C3);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(idol_id).unwrap().controller,
        me(),
        "illegal Slot 1 player target must NOT gain control even when Slot 2 stays legal"
    );

    // 6c. Assassin Gauntlet ETB on next turn (hexproof expired): attaches to
    // `trader_id` AND taps `opp_guard_id`!
    walk_to_end_step(&mut engine, them());
    to_main(&mut engine, me());
    let gauntlet_id = cast_from_hand(&mut engine, me(), C4);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![trader_id],
                players: vec![],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![them()],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(gauntlet_id).unwrap().attached_to,
        Some(trader_id),
        "Assassin Gauntlet attached to Slot 1 creature"
    );
    assert!(
        engine
            .state()
            .object(opp_guard_id)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Assassin Gauntlet tapped all creatures Slot 2 opponent controlled"
    );

    // 6d. 0-creature board for Assassin Gauntlet (`min=0` Slot 1 + mandatory Slot 2):
    let mut engine_empty = bench_main(
        1507,
        vec![gauntlet, opp_guard],
        [
            Seat::with(&[]).holding(&[C4]),
            Seat::with(&[C5]).holding(&[]),
        ],
    );
    let opp_guard2 = the(&engine_empty, ZoneLocation::Battlefield, C5);
    cast_from_hand(&mut engine_empty, me(), C4);
    engine_empty
        .apply(me(), PlayerAction::PassPriority)
        .unwrap();
    engine_empty
        .apply(them(), PlayerAction::PassPriority)
        .unwrap();
    if matches!(engine_empty.pending(), Pending::ChooseTargets { .. }) {
        engine_empty
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![them()],
                },
            )
            .unwrap();
    }
    settle(&mut engine_empty, me());
    assert!(
        engine_empty
            .state()
            .object(opp_guard2)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Assassin Gauntlet with 0 own creatures still fired and tapped opponent's creatures"
    );
}

// =========================================================================
// 7. 3-Creature Distinct Pump Chain + WithAllTargets ("those creatures")
// =========================================================================

#[test]
fn three_creature_distinct_pump_chain_with_all_targets_union_vigilance() {
    const OPT_OTHER_CREATURE: TargetReq =
        TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))
            .with_constraint(TargetConstraint::DistinctFromPriorTargetGroups);

    static TRIPLE_RALLY_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(3),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WithSecondTarget(&[Effect::PumpTarget {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]),
            Effect::WithThirdTarget(&[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]),
            Effect::WithAllTargets(&[Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::VIGILANCE),
                duration: Duration::UntilEndOfTurn,
            }]),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets = Some(OPT_OTHER_CREATURE),
        third_targets = Some(OPT_OTHER_CREATURE),
    )];
    static DOOM_AB: &[AbilityDef] = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let rally = card(
        C0,
        face("Triple Rally", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        TRIPLE_RALLY_AB,
    );
    let doom = card(
        C1,
        face("Instant Doom", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        DOOM_AB,
    );
    let c_a = card(
        C2,
        creature_face("Alpha", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let c_b = card(
        C3,
        creature_face("Beta", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let c_c = card(
        C4,
        creature_face("Gamma", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let c_d = card(
        C5,
        creature_face("Bystander", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1508,
        vec![rally, doom, c_a, c_b, c_c, c_d],
        [
            Seat::with(&[C2, C3, C4, C5]).holding(&[C0]),
            Seat::with(&[]).holding(&[C1]),
        ],
    );
    let a = the(&engine, ZoneLocation::Battlefield, C2);
    let b = the(&engine, ZoneLocation::Battlefield, C3);
    let c = the(&engine, ZoneLocation::Battlefield, C4);
    let d = the(&engine, ZoneLocation::Battlefield, C5);

    cast_from_hand(&mut engine, me(), C0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![a],
                players: vec![],
            },
        )
        .unwrap();
    match engine.pending() {
        Pending::ChooseTargets { options, min, .. } => {
            assert_eq!(*min, 0);
            assert!(!options.contains(&a));
            assert!(options.contains(&b) && options.contains(&c) && options.contains(&d));
        }
        other => panic!("expected Slot 2 ChooseTargets, got {other:?}"),
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![b],
                players: vec![],
            },
        )
        .unwrap();
    match engine.pending() {
        Pending::ChooseTargets { options, min, .. } => {
            assert_eq!(*min, 0);
            assert!(!options.contains(&a) && !options.contains(&b));
            assert!(options.contains(&c) && options.contains(&d));
        }
        other => panic!("expected Slot 3 ChooseTargets, got {other:?}"),
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c],
                players: vec![],
            },
        )
        .unwrap();

    // Opponent destroys `b` (Slot 2) in response; `a` (Slot 1) and `c` (Slot 3) survive!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, them(), C1);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![b],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    let chars_a = engine.state().object(a).unwrap().characteristics();
    let chars_c = engine.state().object(c).unwrap().characteristics();
    let chars_d = engine.state().object(d).unwrap().characteristics();
    assert_eq!(
        (chars_a.power, chars_a.toughness),
        (Some(5), Some(5)),
        "Slot 1 got +3/+3"
    );
    assert!(
        chars_a.keywords.contains(KeywordSet::VIGILANCE),
        "WithAllTargets granted Vigilance to Slot 1"
    );
    assert_eq!(
        (chars_c.power, chars_c.toughness),
        (Some(3), Some(3)),
        "Slot 3 got +1/+1"
    );
    assert!(
        chars_c.keywords.contains(KeywordSet::VIGILANCE),
        "WithAllTargets granted Vigilance to Slot 3"
    );
    assert_eq!((chars_d.power, chars_d.toughness), (Some(2), Some(2)));
    assert!(
        !chars_d.keywords.contains(KeywordSet::VIGILANCE),
        "unchosen bystander did NOT gain Vigilance"
    );

    // Only 1 creature on the battlefield: Slots 2 and 3 (`min=0`) auto-skip
    // with 0 remaining distinct creatures, and `lone` gets +3/+3 and Vigilance!
    let mut engine_lone = bench_main(
        1509,
        vec![rally, c_a],
        [
            Seat::with(&[C2]).holding(&[C0]),
            Seat::with(&[]).holding(&[]),
        ],
    );
    let lone = the(&engine_lone, ZoneLocation::Battlefield, C2);
    cast_from_hand(&mut engine_lone, me(), C0);
    engine_lone
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![lone],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine_lone, me());
    let lone_chars = engine_lone.state().object(lone).unwrap().characteristics();
    assert_eq!((lone_chars.power, lone_chars.toughness), (Some(5), Some(5)));
    assert!(lone_chars.keywords.contains(KeywordSet::VIGILANCE));
}

// =========================================================================
// 8. PeekAndReveal & RevealFromHand inside CaptureAffected
// =========================================================================

#[test]
fn peek_and_reveal_and_reveal_from_hand_capture_remembered_stats() {
    static PEEK_CAPTURE_AB: &[AbilityDef] = &[spell!(&[Effect::CaptureAffected {
        first: &[Effect::PeekAndReveal {
            library: PlayerRel::You,
            count: Amount::Fixed(3),
            reveal_all: false,
            reveal_chosen: true,
            filter: &Filter::CREATURE,
            min_pick: 1,
            max_pick: 1,
            picked: PeekPickDest::Hand,
            rest: PeekRestDest::BottomOfLibrary { random: true },
            if_picked: &[],
        }],
        then: &[Effect::GainLife {
            amount: Amount::RememberedPower,
        }],
    }])];
    static REVEAL_HAND_CAPTURE_AB: &[AbilityDef] = &[spell!(&[Effect::CaptureAffected {
        first: &[Effect::RevealFromHand {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
            min: 0,
            max: Amount::Fixed(2),
            if_revealed: &[],
        }],
        then: &[Effect::LoseLife {
            target: PlayerRel::EachOpponent,
            amount: Amount::RememberedManaValue,
        }],
    }])];

    let peek_spell = card(
        C6,
        face("Peek Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        PEEK_CAPTURE_AB,
    );
    let rev_spell = card(
        C7,
        face("Reveal Hand Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        REVEAL_HAND_CAPTURE_AB,
    );
    let wurm = card(
        C8,
        creature_face("Forest Wurm", "{4}{G}", 6, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let drake = card(
        C9,
        creature_face("Sky Drake", "{2}{U}", 2, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let filler = card(
        C0,
        face("Filler Stone", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1510,
        vec![peek_spell, rev_spell, wurm, drake, filler],
        [
            Seat::with(&[]).holding(&[C6, C7, C9, C0, C0, C8]),
            Seat::with(&[]).holding(&[]),
        ],
    );
    let _ = put_on_library_top(&mut engine, me(), C0);
    let _ = put_on_library_top(&mut engine, me(), C0);
    let wurm_id = put_on_library_top(&mut engine, me(), C8);
    let drake_id = the(&engine, ZoneLocation::Hand(me()), C9);

    cast_from_hand(&mut engine, me(), C6);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![wurm_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        26,
        "PeekAndReveal captured picked Wurm's power 6"
    );

    cast_from_hand(&mut engine, me(), C7);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![wurm_id, drake_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        12,
        "RevealFromHand captured revealed cards' total MV (5 + 3 = 8)"
    );
}
