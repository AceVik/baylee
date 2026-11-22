//! Beat 18 / Engine Wave 14c deterministic rule tests:
//! 1. `Effect::WithSecondTarget(&'static [Effect])` (`CR 608.2b`):
//!    - Immediate chains (*Aether Helix* bounce + graveyard recursion,
//!      *Angrath's Fury* destroy + player/planeswalker damage where `Filter::This`
//!      remains the source),
//!    - Suspension-safe `WithSecondTarget` across interactive `Pending` choices
//!      (`MayDo`, `PlayerMayPayOr`, `ChooseCardEx`) restoring outer `res.targets`
//!      and `res.target_players` for trailing `TargetSlot::First` effects +
//!      distinct `snapshot_hash()` while suspended,
//!    - `CR 608.2b` partial illegality: an illegal second object target skips
//!      `WithSecondTarget` without falling back to `res.source` while the first
//!      target still resolves.
//! 2. `third_targets: Option<TargetReq>` + `Effect::WithThirdTarget` +
//!    `TargetSlot::Third` (`CR 601.2c`, `CR 602.2b`, `CR 603.3d`, `CR 608.2b`):
//!    - Across `Spell` (*Cone of Flame*), `Activated`, `Triggered`, `Loyalty`,
//!      and `Saga` (`chapter!` & `chapters!`), plus `TargetSlot::Third`
//!      (`PumpTargetSlot`, `TapTargetSlot`, `UntapTargetSlot`,
//!      `DestroyTargetSlots`),
//!    - `CR 608.2b` partial vs total illegality (2 of 3 illegal still resolves
//!      the surviving target; all 3 illegal fizzles),
//!    - `CR 115.7` retargeting (`ChooseNewTargets`) across all 3 target groups.
//! 3. `TargetConstraint::DistinctFromPriorTargetGroups` (`CR 115.1`, `CR 115.7`):
//!    - Uncastable / unactivatable / untriggered when total distinct targets <
//!      mandatory groups,
//!    - Joint forward completability when Group 0 (`AnyTarget`) is wider than
//!      mandatory Group 1 (`Creature`, `DistinctFromPriorTargetGroups`),
//!    - Prior object & player exclusion across Group 1 and Group 2 (*Arm the
//!      Cathars* & *Cone of Flame*),
//!    - `CR 115.7` retargeting (`ChooseNewTargets`) enforcing cross-group
//!      distinctness and allowing cycle permutations `(A, B, C) -> (B, C, A)`.
//! 4. `Effect::AttachSlot { object, to, optional }` (`CR 301.5c`, `CR 303.4d`,
//!    `CR 701.3`, `CR 702.16c`, `CR 400.7`):
//!    - Mandatory and optional (`Yes` / `No`) Equipment and Aura attachment
//!      (`First -> Second` *Aura Finesse*, `First -> This` *Cloud, Ex-SOLDIER* /
//!      *Amy Rose*, multi-object `First -> Second` *Arm the Cathars* /
//!      *Barret, Avalanche Leader*),
//!    - `CR 301.5c` animated creature-Equipment refusal, `CR 303.4d` /
//!      `CR 702.16c` Aura enchant filter & protection refusal, and `CR 400.7`
//!      LKI incarnation safety.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::YesNoPrompt;
use crate::event::Cause;
use crate::object::Status;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    Amount, ChooseMove, ChooseZone, CounterKind, Duration, Effect, Filter, Modifier, PlayerRel,
    TargetConstraint, TargetReq, TargetSlot, TargetSpec, Trigger, activated, chapter, chapters,
    loyalty, spell, static_ability, triggered,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, enchantment};

const C0: u32 = 99_950;
const C1: u32 = 99_951;
const C2: u32 = 99_952;
const C3: u32 = 99_953;
const C4: u32 = 99_954;
const C5: u32 = 99_955;
const C6: u32 = 99_956;
const C7: u32 = 99_957;
const C8: u32 = 99_958;
const C9: u32 = 99_959;

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

fn make_equip(idx: u32, name: &'static str, abilities: &'static [AbilityDef]) -> &'static CardDef {
    let mut f = face(name, "{0}", TypeSet::ARTIFACT);
    f.subtypes = &[artifact::EQUIPMENT];
    card(idx, f, KeywordSet::EMPTY, abilities)
}

fn make_aura(
    idx: u32,
    name: &'static str,
    colors: ColorSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    let mut f = face(name, "{0}", TypeSet::ENCHANTMENT);
    f.subtypes = &[enchantment::AURA];
    f.color_indicator = colors;
    card(idx, f, KeywordSet::FLASH, abilities)
}

// =========================================================================
// 1. Effect::WithSecondTarget — immediate, player-target, LKI & suspension
// =========================================================================

#[test]
fn with_second_target_immediate_chains_and_partial_illegality() {
    // Aether Helix: return target permanent to its owner's hand; return target
    // permanent card from your graveyard to your hand.
    static HELIX_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::bounce(TargetSpec::Object(&Filter::Any)),
            Effect::WithSecondTarget(&[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::NONLAND, PlayerRel::You),
            }]),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        second_targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::NONLAND,
            PlayerRel::You,
        ))),
    )];
    // Angrath's Fury: destroy target creature; ~ deals 3 damage to target player
    // or planeswalker (verifies player-only second target keeps source intact).
    static FURY_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::CREATURE)),
            Effect::WithSecondTarget(&[Effect::DealDamage {
                target: TargetSpec::AnyPlayer,
                amount: Amount::Fixed(3),
            }]),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )];
    static EXILE_GY_AB: &[AbilityDef] = &[spell!(
        &[Effect::Exile {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::Opponent),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::Opponent,
        ))),
    )];

    let helix = card(
        C0,
        face("Aether Helix Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        HELIX_AB,
    );
    let fury = card(
        C1,
        face("Angrath Fury Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        FURY_AB,
    );
    let exile_gy = card(
        C2,
        face("Surgical Exile", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        EXILE_GY_AB,
    );
    let bear_a = card(
        C3,
        creature_face("Bear A", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b = card(
        C4,
        creature_face("Bear B", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let gy_relic = card(
        C5,
        face("Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1401,
        vec![helix, fury, exile_gy, bear_a, bear_b, gy_relic],
        [
            Seat::with(&[]).holding(&[C0, C1, C5]),
            Seat::with(&[C3, C4]).holding(&[]),
        ],
    );
    let relic = put_in_graveyard(&mut engine, me(), C5);
    let opp_bear_a = the(&engine, ZoneLocation::Battlefield, C3);
    let opp_bear_b = the(&engine, ZoneLocation::Battlefield, C4);

    // 1. Both targets legal: bounces opp_bear_a and returns relic from GY to hand.
    cast_from_hand(&mut engine, me(), C0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear_a],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(opp_bear_a).unwrap().zone,
        crate::zone::Zone::Hand
    );
    assert_eq!(
        engine.state().object(relic).unwrap().zone,
        crate::zone::Zone::Hand
    );

    // 2. Angrath's Fury: destroys opp_bear_b and deals 3 damage to opponent.
    cast_from_hand(&mut engine, me(), C1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear_b],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(opp_bear_b).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(life(&engine, them()), 17);

    // 3. Partial illegality (CR 608.2b): if the 2nd target (relic in GY) is
    // exiled in response, the 1st target is still bounced and WithSecondTarget
    // safely skips without touching anything else.
    let mut engine2 = bench_main(
        1402,
        vec![helix, exile_gy, bear_a, gy_relic],
        [
            Seat::with(&[]).holding(&[C0, C5]),
            Seat::with(&[C3]).holding(&[C2]),
        ],
    );
    let relic2 = put_in_graveyard(&mut engine2, me(), C5);
    let opp_bear2 = the(&engine2, ZoneLocation::Battlefield, C3);
    cast_from_hand(&mut engine2, me(), C0);
    engine2
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear2],
                players: vec![],
            },
        )
        .unwrap();
    engine2
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic2],
                players: vec![],
            },
        )
        .unwrap();
    engine2.apply(me(), PlayerAction::PassPriority).unwrap();
    // Opponent exiles relic2 from our graveyard in response.
    cast_from_hand(&mut engine2, them(), C2);
    engine2
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![relic2],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine2, me());
    assert_eq!(
        engine2.state().object(opp_bear2).unwrap().zone,
        crate::zone::Zone::Hand,
        "surviving first target still resolves"
    );
    assert_eq!(
        engine2.state().object(relic2).unwrap().zone,
        crate::zone::Zone::Exile,
        "exiled second target stays in exile"
    );
}

#[test]
fn with_second_target_suspends_across_choices_and_restores_first_target() {
    // Tests suspension across MayDo, PlayerMayPayOr, and ChooseCardEx inside
    // WithSecondTarget, followed by a trailing outer effect on the first target!
    static SUSPENDED_CHAIN: &[AbilityDef] = &[spell!(
        &[
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::WithSecondTarget(&[
                Effect::MayDo {
                    effects: &[Effect::TapTarget],
                },
                Effect::PlayerMayPayOr {
                    player: PlayerRel::Opponent,
                    mana: Amount::Fixed(1),
                    effect: &Effect::AddCounter {
                        kind: CounterKind::Stun,
                        amount: Amount::Fixed(1),
                    },
                },
                Effect::ChooseCardEx {
                    chooser: PlayerRel::You,
                    zone: ChooseZone::Hand(PlayerRel::You),
                    filter: &Filter::Any,
                    min: 0,
                    max: Amount::Fixed(1),
                    reveal_chosen: false,
                    movement: ChooseMove::Graveyard,
                    not_chosen_movement: ChooseMove::None,
                    then: &[],
                    for_unchosen: &[],
                },
            ]),
            // Trailing outer effect must act on the FIRST target (not the second!).
            Effect::AddCounter {
                kind: CounterKind::Shield,
                amount: Amount::Fixed(1),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::OPPONENT_CREATURE
        ))),
    )];

    let chain_spell = card(
        C0,
        face("Suspended Chain", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        SUSPENDED_CHAIN,
    );
    let my_bear = card(
        C1,
        creature_face("My Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_bear = card(
        C2,
        creature_face("Opp Bear", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let fodder = card(
        C3,
        face("Hand Fodder", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1403,
        vec![chain_spell, my_bear, opp_bear, fodder],
        [
            Seat::with(&[C1]).holding(&[C0, C3]),
            Seat::with(&[C2]).holding(&[]),
        ],
    );
    let mine = the(&engine, ZoneLocation::Battlefield, C1);
    let theirs = the(&engine, ZoneLocation::Battlefield, C2);
    let hand_fodder = the(&engine, ZoneLocation::Hand(me()), C3);

    cast_from_hand(&mut engine, me(), C0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![mine],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .unwrap();
    // Pass both priorities so the spell starts resolving and suspends at MayDo.
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            ..
        } if *player == me()
    ));
    let suspended_hash_1 = engine.snapshot_hash();
    let suspended_hash_2 = engine.snapshot_hash();
    assert_eq!(
        suspended_hash_1, suspended_hash_2,
        "snapshot_hash while suspended inside WithSecondTarget is deterministic"
    );

    // Answer Yes to MayDo -> taps `theirs` (the second target), NOT `mine`,
    // and suspends a second time at PlayerMayPayOr (`them()` declines)!
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    assert!(
        engine
            .state()
            .object(theirs)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "MayDo inside WithSecondTarget tapped the second target"
    );
    assert!(
        !engine
            .state()
            .object(mine)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "first target remained untapped"
    );
    assert!(matches!(
        engine.pending(),
        Pending::YesNo { player, .. } if *player == them()
    ));
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    assert_eq!(
        engine
            .state()
            .object(theirs)
            .unwrap()
            .counters
            .get(CounterKind::Stun),
        1,
        "PlayerMayPayOr fallback inside WithSecondTarget put Stun on second target"
    );

    // Answer ChooseCardEx by discarding `hand_fodder`, which finishes
    // WithSecondTarget, restores `res.targets` to `[mine]`, and places the
    // trailing Shield counter on `mine`!
    assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![hand_fodder],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        engine
            .state()
            .object(mine)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(mine)
            .unwrap()
            .counters
            .get(CounterKind::Shield),
        1,
        "trailing outer effect after WithSecondTarget restored first target"
    );
    assert_eq!(
        engine
            .state()
            .object(theirs)
            .unwrap()
            .counters
            .get(CounterKind::Shield),
        0,
        "second target did NOT receive the outer trailing Shield counter"
    );
}

// =========================================================================
// 2 & 3. third_targets, WithThirdTarget, TargetSlot::Third & DistinctFromPriorTargetGroups
// =========================================================================

#[test]
fn cone_of_flame_three_distinct_targets_completability_and_retarget() {
    const DISTINCT_ANY: TargetReq = TargetReq::one(TargetSpec::AnyTarget)
        .with_constraint(TargetConstraint::DistinctFromPriorTargetGroups);

    static CONE_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(1),
            },
            Effect::WithSecondTarget(&[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(2),
            }]),
            Effect::WithThirdTarget(&[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(3),
            }]),
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        second_targets = Some(DISTINCT_ANY),
        third_targets = Some(DISTINCT_ANY),
    )];
    static SWERVE_AB: &[AbilityDef] = &[spell!(
        &[Effect::ChooseNewTargets],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];

    let cone = card(
        C0,
        face("Cone of Flame Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        CONE_AB,
    );
    let swerve = card(
        C1,
        face("Deflect Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        SWERVE_AB,
    );
    let bear = card(
        C2,
        creature_face("Lone Bear", "{0}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );

    // 1. With 0 creatures on the battlefield, only 2 players exist (< 3 distinct
    // targets needed), so Cone of Flame is NOT castable (`CR 601.2c`)!
    let empty_board = bench_main(
        1404,
        vec![cone, swerve, bear],
        [Seat::with(&[]).holding(&[C0]), Seat::with(&[]).holding(&[])],
    );
    let cone_empty = the(&empty_board, ZoneLocation::Hand(me()), C0);
    let (_, legal_empty) = offer(&empty_board);
    assert!(
        !legal_empty.castable.contains(&cone_empty),
        "3-target distinct spell must not be castable when only 2 targets exist"
    );

    // 2. With 1 creature + 2 players = 3 distinct targets (`{bear_id, me(), them()}`),
    // Cone of Flame IS castable, excludes prior choices at each step, and can
    // even permute `(bear_id, me(), them()) -> (me(), them(), bear_id)` under
    // `CR 115.7d` `ChooseNewTargets`!
    let mut engine = bench_main(
        1405,
        vec![cone, swerve, bear],
        [
            Seat::with(&[]).holding(&[C0, C1]),
            Seat::with(&[C2]).holding(&[]),
        ],
    );
    let bear_id = the(&engine, ZoneLocation::Battlefield, C2);

    cast_from_hand(&mut engine, me(), C0);
    // Group 0: choose `bear_id`.
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_id],
                players: vec![],
            },
        )
        .unwrap();
    // Group 1: `bear_id` must be excluded from `options`! Choose `me()`.
    match engine.pending() {
        Pending::ChooseTargets {
            options,
            player_options,
            ..
        } => {
            assert!(
                !options.contains(&bear_id),
                "Group 1 must exclude Group 0's chosen object"
            );
            assert!(player_options.contains(&me()) && player_options.contains(&them()));
        }
        other => panic!("expected Group 1 ChooseTargets, got {other:?}"),
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    // Group 2: both `bear_id` AND `me()` must be excluded, leaving ONLY `them()`!
    match engine.pending() {
        Pending::ChooseTargets {
            options,
            player_options,
            ..
        } => {
            assert!(options.is_empty());
            assert_eq!(player_options, &[them()]);
        }
        other => panic!("expected Group 2 ChooseTargets, got {other:?}"),
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();

    // Cast `ChooseNewTargets` targeting Cone of Flame on the stack, and permute
    // `(bear_id, me(), them()) -> (me(), them(), bear_id)`!
    let cone_stack = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .first()
        .unwrap();
    cast_from_hand(&mut engine, me(), C1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![cone_stack],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    // Retarget slot 0 (`bear_id` -> `me()`):
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    // Retarget slot 1 (was `me()`, which now collides with slot 0, so `min == 1`;
    // both `bear_id` [giving 2-cycle `(me(), bear_id, them())`] and `them()`
    // [giving 3-cycle `(me(), them(), bear_id)`] have legal distinct completions!):
    match engine.pending() {
        Pending::ChooseTargets {
            min,
            options,
            player_options,
            ..
        } => {
            assert_eq!(
                *min, 1,
                "slot 1 cannot keep me() after slot 0 moved to me()"
            );
            assert_eq!(options, &[bear_id]);
            assert_eq!(player_options, &[them()]);
        }
        other => panic!("expected retarget slot 1, got {other:?}"),
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    // Retarget slot 2 (was `them()`, which collides with slot 1, so only `bear_id`!):
    match engine.pending() {
        Pending::ChooseTargets {
            min,
            options,
            player_options,
            ..
        } => {
            assert_eq!(*min, 1);
            assert_eq!(options, &[bear_id]);
            assert!(player_options.is_empty());
        }
        other => panic!("expected retarget slot 2, got {other:?}"),
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_id],
                players: vec![],
            },
        )
        .unwrap();

    settle(&mut engine, me());
    // Permuted Cone of Flame dealt: 1 to me(), 2 to them(), 3 to bear_id!
    assert_eq!(life(&engine, me()), 19);
    assert_eq!(life(&engine, them()), 18);
    assert_eq!(engine.state().object(bear_id).unwrap().damage, 3);
}

#[test]
fn forward_completability_filters_wider_first_group_and_partial_vs_all_illegal() {
    // Group 0: AnyTarget (creatures + players).
    // Group 1: Creature with DistinctFromPriorTargetGroups.
    // When ONLY 1 creature exists on the battlefield, Group 0 MUST NOT offer
    // that lone creature (since picking it would starve Group 1!).
    const DISTINCT_CREATURE: TargetReq = TargetReq::one(TargetSpec::Object(&Filter::CREATURE))
        .with_constraint(TargetConstraint::DistinctFromPriorTargetGroups);

    static WIDE_THEN_NARROW: &[AbilityDef] = &[spell!(
        &[
            Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(1),
            },
            Effect::PumpTargetSlot {
                slot: TargetSlot::Second,
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::TapTargetSlot(TargetSlot::Third),
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        second_targets = Some(DISTINCT_CREATURE),
        third_targets = Some(DISTINCT_CREATURE),
    )];
    static KILL_AB: &[AbilityDef] = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let wide = card(
        C0,
        face("Wide Narrow", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        WIDE_THEN_NARROW,
    );
    let kill = card(
        C1,
        face("Instant Kill", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        KILL_AB,
    );
    let c_a = card(C2, creature_face("A", "{0}", 2, 2), KeywordSet::EMPTY, &[]);
    let c_b = card(C3, creature_face("B", "{0}", 2, 2), KeywordSet::EMPTY, &[]);

    let mut engine = bench_main(
        1406,
        vec![wide, kill, c_a, c_b],
        [
            Seat::with(&[C2]).holding(&[C0]),
            Seat::with(&[C3]).holding(&[C1, C1]),
        ],
    );
    let bear_a = the(&engine, ZoneLocation::Battlefield, C2);
    let bear_b = the(&engine, ZoneLocation::Battlefield, C3);

    // Group 1 + Group 2 both require distinct creatures, and only 2 creatures
    // (`bear_a`, `bear_b`) exist — so Group 0 (`AnyTarget`) MUST filter out
    // both creatures and offer ONLY the two players!
    cast_from_hand(&mut engine, me(), C0);
    match engine.pending() {
        Pending::ChooseTargets {
            options,
            player_options,
            ..
        } => {
            assert!(
                options.is_empty(),
                "Group 0 must filter out creatures needed by mandatory Group 1 & 2, got {options:?}"
            );
            assert_eq!(player_options.len(), 2);
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }
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
                objects: vec![bear_a],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b],
                players: vec![],
            },
        )
        .unwrap();

    // Opponent kills BOTH `bear_a` and `bear_b` in response (2 of 3 targets
    // illegal): Group 0 (`them()`) is still legal, so the spell still deals 1
    // damage to `them()` (`CR 608.2b`)!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    for victim in [bear_a, bear_b] {
        let kill_id = *objects(&engine, ZoneLocation::Hand(them()), C1)
            .first()
            .unwrap();
        engine
            .apply(them(), PlayerAction::CastSpell { card: kill_id })
            .unwrap();
        engine
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![victim],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        19,
        "2 of 3 targets illegal still resolves for surviving player target"
    );
}

#[test]
fn third_targets_across_activated_triggered_loyalty_and_saga() {
    const CREAT_REQ: Option<TargetReq> =
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)));

    // 1. Activated ability with 3 target groups + TargetSlot::Third ops.
    static ACT_THREE: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::WithSecondTarget(&[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }]),
            Effect::PumpTargetSlot {
                slot: TargetSlot::Third,
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(3),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::TapTargetSlot(TargetSlot::Third),
        ],
        targets = CREAT_REQ,
        second_targets = CREAT_REQ,
        third_targets = CREAT_REQ,
    )];

    // 2. Loyalty ability (+1) with 3 target groups.
    static LOY_THREE: &[AbilityDef] = &[loyalty!(
        1,
        &[
            Effect::UntapTargetSlot(TargetSlot::First),
            Effect::WithSecondTarget(&[Effect::DealDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::Fixed(1),
            }]),
            Effect::WithThirdTarget(&[Effect::DealDamage {
                target: TargetSpec::AnyPlayer,
                amount: Amount::Fixed(2),
            }]),
        ],
        targets = CREAT_REQ,
        second_targets = CREAT_REQ,
        third_targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )];

    // 3. ETB Triggered ability with 3 target groups.
    static ETB_THREE: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[
            Effect::DestroyTargetSlots {
                targets: &[TargetSlot::First, TargetSlot::Third],
                no_regen: false,
            },
            Effect::WithSecondTarget(&[Effect::GainLifeFor {
                who: PlayerRel::Chosen,
                amount: Amount::Fixed(4),
            }]),
        ],
        targets = CREAT_REQ,
        second_targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        third_targets = CREAT_REQ,
    )];

    // 4. Saga chapter with `third_targets` (`chapter!` lowers cleanly to
    // Chapter-triggered ability).
    static SAGA_THREE: &[AbilityDef] = &[chapter!(
        1,
        &[
            Effect::DealDamage {
                target: TargetSpec::AnyPlayer,
                amount: Amount::Fixed(1),
            },
            Effect::WithSecondTarget(&[Effect::DealDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::Fixed(1),
            }]),
            Effect::WithThirdTarget(&[Effect::DealDamage {
                target: TargetSpec::AnyPlayer,
                amount: Amount::Fixed(3),
            }]),
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets = CREAT_REQ,
        third_targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )];
    // Also check `chapters!` macro accepts `third_targets` at compile & build time:
    static _MULTI_CHAPTER_THREE: &[AbilityDef] = &[chapters!(
        &[1, 2],
        &[Effect::TapTargetSlot(TargetSlot::Third)],
        targets = CREAT_REQ,
        second_targets = CREAT_REQ,
        third_targets = CREAT_REQ,
    )];

    let relic = card(
        C0,
        face("Triad Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ACT_THREE,
    );
    let mut pw_face = face("Triad Walker", "{0}", TypeSet::PLANESWALKER);
    pw_face.loyalty = Some(3);
    let walker = card(C1, pw_face, KeywordSet::EMPTY, LOY_THREE);
    let etb_creat = card(
        C2,
        creature_face("Triad Titan", "{0}", 4, 4),
        KeywordSet::EMPTY,
        ETB_THREE,
    );
    let mut saga_f = face("Triad Saga", "{0}", TypeSet::ENCHANTMENT);
    saga_f.subtypes = &[enchantment::SAGA];
    let saga = card(C3, saga_f, KeywordSet::EMPTY, SAGA_THREE);
    let d1 = card(C4, creature_face("D1", "{0}", 2, 3), KeywordSet::EMPTY, &[]);
    let d2 = card(C5, creature_face("D2", "{0}", 2, 3), KeywordSet::EMPTY, &[]);
    let d3 = card(C6, creature_face("D3", "{0}", 2, 3), KeywordSet::EMPTY, &[]);

    let mut engine = bench_main(
        1407,
        vec![relic, walker, etb_creat, saga, d1, d2, d3],
        [
            Seat::with(&[C0, C4, C5, C6]).holding(&[C1, C2, C3]),
            Seat::with(&[]).holding(&[]),
        ],
    );
    let relic_id = the(&engine, ZoneLocation::Battlefield, C0);
    let c1 = the(&engine, ZoneLocation::Battlefield, C4);
    let c2 = the(&engine, ZoneLocation::Battlefield, C5);
    let c3 = the(&engine, ZoneLocation::Battlefield, C6);

    // 1. Activate Triad Relic targeting (c1, c2, c3).
    activate(&mut engine, me(), relic_id, 0);
    for pick in [c1, c2, c3] {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![pick],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(c1)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(c2)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(
        engine.state().object(c3).unwrap().characteristics().power,
        Some(5)
    );
    assert!(
        engine
            .state()
            .object(c3)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // 2. Cast and activate Triad Walker (+1 loyalty): untaps c3, deals 1 to c2, 2 to them().
    cast_from_hand(&mut engine, me(), C1);
    settle(&mut engine, me());
    let walker_id = the(&engine, ZoneLocation::Battlefield, C1);
    activate(&mut engine, me(), walker_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c3],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c2],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert!(
        !engine
            .state()
            .object(c3)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().object(c2).unwrap().damage, 1);
    assert_eq!(life(&engine, them()), 18);

    // 3. Cast Triad Saga: Chapter I triggers on ETB targeting (me(), c2, them()).
    cast_from_hand(&mut engine, me(), C3);
    // Pass so Saga resolves onto battlefield and triggers Chapter I!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c2],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 19);
    assert_eq!(engine.state().object(c2).unwrap().damage, 2);
    assert_eq!(life(&engine, them()), 15);

    // 4. Cast Triad Titan: ETB trigger destroys c1 & c3 (`DestroyTargetSlots`
    // with `[First, Third]`) and gains 4 life for `me()` via `WithSecondTarget`.
    cast_from_hand(&mut engine, me(), C2);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c1],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c3],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(c1).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(c3).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(c2).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(life(&engine, me()), 23);
}

// =========================================================================
// 4. Effect::AttachSlot — mandatory, optional, Equipment, Aura, CR 301.5c & 303.4d
// =========================================================================

#[test]
fn attach_slot_moves_an_enchant_player_aura_to_the_chosen_player() {
    static CURSE: &[AbilityDef] = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::AnyPlayer
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];
    static MOVE: &[AbilityDef] = &[spell!(
        &[Effect::AttachSlot {
            object: TargetSlot::First,
            to: TargetSlot::Second,
            optional: true
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::HasSubtype(
            enchantment::AURA
        )))),
        second_targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];
    let aura = make_aura(C0, "Enchant Player", ColorSet::EMPTY, CURSE);
    let spell = card(
        C1,
        face("Move Aura", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        MOVE,
    );
    let mut engine = bench_main(
        1409,
        vec![aura, spell],
        [Seat::default().holding(&[C0, C1]), Seat::default()],
    );
    let player_target = |player| PlayerAction::ChooseTargets {
        objects: vec![],
        players: vec![player],
    };
    let curse = cast_from_hand(&mut engine, me(), C0);
    engine.apply(me(), player_target(me())).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(curse).unwrap().attached_player(),
        Some(me())
    );
    cast_from_hand(&mut engine, me(), C1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![curse],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(me(), player_target(them())).unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(curse).unwrap().attached_player(),
        Some(them())
    );
}

#[test]
fn attach_slot_mandatory_optional_multi_object_and_cr_legality_guards() {
    static EQUIP_FILTER: Filter = Filter::HasSubtype(artifact::EQUIPMENT);
    static AURA_FILTER: Filter = Filter::HasSubtype(enchantment::AURA);
    static BLACK_FILTER: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));
    static PRO_BLACK_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&BLACK_FILTER)
    )];
    static EQUIP_BOOST: &[AbilityDef] = &[static_ability!(
        Filter::AttachedToBySource,
        Modifier::ModifyPT(2, 1)
    )];
    static RED_AURA_AB: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
    ];
    // Barret / Arm the Cathars style: attach up to two target Equipment to
    // target creature (mandatory).
    static MULTI_ATTACH_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::AttachSlot {
            object: TargetSlot::First,
            to: TargetSlot::Second,
            optional: false,
        }],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&EQUIP_FILTER), 2)),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    // Cloud, Ex-SOLDIER style ETB: attach up to one target Equipment to `This`
    // (`optional: true`).
    static CLOUD_ETB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[Effect::AttachSlot {
            object: TargetSlot::First,
            to: TargetSlot::This,
            optional: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&EQUIP_FILTER))),
    )];
    // Aura Finesse style: attach target Aura you control to target creature.
    static AURA_FINESSE_AB: &[AbilityDef] = &[spell!(
        &[Effect::AttachSlot {
            object: TargetSlot::First,
            to: TargetSlot::Second,
            optional: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&AURA_FILTER))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    static RECONFIGURING: &[AbilityDef] = &[
        EQUIP_BOOST[0],
        activated!(
            baylee_cards_dsl::cost!("{1}"),
            &[Effect::ReconfigureAttach],
            target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL))
        ),
        static_ability!(
            Filter::This,
            Modifier::RemoveType(TypeSet::CREATURE),
            condition = Some(baylee_cards_dsl::Condition::SourceMatches(
                &Filter::IsAttached
            ))
        ),
    ];
    let sword_a = make_equip(C0, "Sword A", EQUIP_BOOST);
    let sword_b = card(
        C1,
        {
            let mut f = creature_face("Reconfiguring Sword", "{0}", 2, 2);
            f.types = TypeSet::ARTIFACT.union(TypeSet::CREATURE);
            f.subtypes = &[artifact::EQUIPMENT];
            f
        },
        KeywordSet::EMPTY,
        RECONFIGURING,
    );
    let anim_sword = card(
        C2,
        {
            let mut f = creature_face("Living Sword", "{0}", 2, 2);
            f.types = TypeSet::ARTIFACT.union(TypeSet::CREATURE);
            f.subtypes = &[artifact::EQUIPMENT];
            f
        },
        KeywordSet::EMPTY,
        EQUIP_BOOST,
    );
    let red_aura = make_aura(
        C3,
        "Your-Only Black Aura",
        ColorSet::from_slice(&[Color::Black]),
        RED_AURA_AB,
    );
    let multi_attach = card(
        C4,
        face("Arm Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        MULTI_ATTACH_SPELL,
    );
    let cloud = card(
        C5,
        creature_face("Cloud Probe", "{0}", 3, 3),
        KeywordSet::EMPTY,
        CLOUD_ETB,
    );
    let finesse = card(
        C6,
        face("Finesse Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        AURA_FINESSE_AB,
    );
    let hero_a = card(
        C7,
        creature_face("Hero A", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let pro_red_hero = card(
        C8,
        creature_face("Pro-Black Hero", "{0}", 2, 2),
        KeywordSet::PROTECTION_BLACK,
        PRO_BLACK_AB,
    );
    let opp_hero = card(
        C9,
        creature_face("Opp Hero", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench_main(
        1408,
        vec![
            sword_a,
            sword_b,
            anim_sword,
            red_aura,
            multi_attach,
            cloud,
            finesse,
            hero_a,
            pro_red_hero,
            opp_hero,
        ],
        [
            Seat::with(&[C0, C1, C2, C7, C8]).holding(&[C3, C4, C5, C5, C5, C6, C6, C6]),
            Seat::with(&[C9]).holding(&[]),
        ],
    );
    let sw_a = the(&engine, ZoneLocation::Battlefield, C0);
    let sw_b = the(&engine, ZoneLocation::Battlefield, C1);
    let living_sw = the(&engine, ZoneLocation::Battlefield, C2);
    let hero = the(&engine, ZoneLocation::Battlefield, C7);
    let pro_red = the(&engine, ZoneLocation::Battlefield, C8);
    let opp = the(&engine, ZoneLocation::Battlefield, C9);

    // 1. Multi-object `AttachSlot` (`First -> Second`): attaches both `sw_a`
    // and `sw_b` to `hero`, giving +4/+2 -> 6/4!
    cast_from_hand(&mut engine, me(), C4);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![sw_a, sw_b],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![hero],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(sw_a).unwrap().attached_to, Some(hero));
    assert_eq!(engine.state().object(sw_b).unwrap().attached_to, Some(hero));
    assert!(
        !engine
            .state()
            .object(sw_b)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(
        engine.state().object(hero).unwrap().characteristics().power,
        Some(6)
    );

    // 2. `Cloud` #1 (`optional: true`, answer `No`): `sw_a` stays on `hero`!
    let cloud_hand_1 = objects(&engine, ZoneLocation::Hand(me()), C5)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: cloud_hand_1 })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![sw_a],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sw_a).unwrap().attached_to,
        Some(hero),
        "declining optional AttachSlot leaves Equipment on previous creature"
    );

    // 3. `Cloud` #2 (`optional: true`, answer `Yes`): moves `sw_a` onto Cloud #2!
    // Also verify `living_sw` (an artifact CREATURE Equipment, CR 301.5c) does
    // NOT even prompt `YesNo` if targeted because `can_attach_object` is false!
    let cloud_hand_2 = objects(&engine, ZoneLocation::Hand(me()), C5)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: cloud_hand_2 })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![sw_a],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    let cloud_2_bf = *objects(&engine, ZoneLocation::Battlefield, C5)
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(sw_a).unwrap().attached_to,
        Some(cloud_2_bf),
        "accepting optional AttachSlot attaches Equipment to This"
    );
    assert_eq!(engine.state().object(living_sw).unwrap().attached_to, None);

    // 4. Cast `red_aura` onto `hero`, then test `Aura Finesse` (`AttachSlot`):
    // - Moving `red_aura` (Enchant YOUR_CREATURE, Red) onto `opp` (opponent's
    //   creature, CR 303.4d) or `pro_red` (Protection from Red, CR 702.16c)
    //   must NOT move the Aura!
    // - Moving `red_aura` onto `cloud_2_bf` (our creature without pro-red)
    //   succeeds cleanly!
    cast_from_hand(&mut engine, me(), C3);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![hero],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let aura_id = the(&engine, ZoneLocation::Battlefield, C3);
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(hero)
    );

    // Illegal destination 1: `pro_red` (CR 702.16c protection from red).
    let fin_1 = objects(&engine, ZoneLocation::Hand(me()), C6)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: fin_1 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![aura_id],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![pro_red],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(hero),
        "Aura with Red color must not attach to Pro-Red creature (CR 702.16c)"
    );

    // Illegal destination 2: `opp` fails enchant filter `YOUR_CREATURE` (CR 303.4d),
    // and creature-Equipment `living_sw` fails `CR 301.5c` (does not prompt YesNo!).
    let cloud_hand_3 = objects(&engine, ZoneLocation::Hand(me()), C5)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: cloud_hand_3 })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![living_sw],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(living_sw).unwrap().attached_to,
        None,
        "Creature-Equipment cannot equip (CR 301.5c) and skips YesNo prompt"
    );

    let fin_opp = objects(&engine, ZoneLocation::Hand(me()), C6)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: fin_opp })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![aura_id],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(hero),
        "Enchant YOUR_CREATURE Aura cannot enchant opponent's creature (CR 303.4d)"
    );

    let fin_2 = objects(&engine, ZoneLocation::Hand(me()), C6)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: fin_2 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![aura_id],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![cloud_2_bf],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(cloud_2_bf),
        "Aura Finesse moves Aura onto legal controlled creature"
    );
}
