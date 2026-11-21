//! Wave 7a engine rule tests:
//! - `ChooseMove::ExileLinked` (`CR 607.2a`): untargeted `Effect::ChooseCard`
//!   moving chosen permanents to exile with `Rider::Linked { host: source, until: None }`
//!   so `Effect::ReturnLinkedToBattlefield` returns only the cards exiled by that
//!   source incarnation when it leaves the battlefield.
//! - `EnterModifier::WithCountersIf` (`CR 614.1c`, `CR 614.12`, `CR 614.16`):
//!   conditional ETB replacement placing counters only when `condition` holds
//!   at entry, interacting once (never double-applying) with counter-doubling
//!   replacements.
//! - `Impending N—[cost]` (`CR 702.176a`, `CR 205.1a`, `CR 208.3`, `CR 603.4`):
//!   alternative cost (`CastModeKind::Impending`, `Rider::ImpendingPaid`) that
//!   stays a Creature spell on the stack, enters with `N` `CounterKind::Time`
//!   counters, removes `TypeSet::CREATURE`, creature subtypes, and P/T in
//!   layer 4 on the battlefield while `Time >= 1`, and removes one time counter
//!   at the beginning of each of its controller's end steps until it awakens.
//! - `Gift` (`CR 702.174a–m`): optional additional cost (`YesNoPrompt::Gift`,
//!   `WizardStage::Gift`, `Rider::GiftPromisedTo`) auto-selecting the sole
//!   opponent in duels and prompting `Pending::ChoosePlayer` across living
//!   opponents in multiplayer (untargeted, bypassing Hexproof), exposing
//!   `Condition::GiftWasPromised` and `PlayerRel::GiftRecipient`, delivering all
//!   6 `GiftKind`s on instant/sorcery resolution (before other spell effects,
//!   suppressed on fizzle/counter per `CR 702.174j`) or permanent ETB trigger,
//!   and firing `Trigger::GaveGift` (`CR 702.174c`).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, YesNoPrompt};
use crate::object::Rider;
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, ChooseMove, ChooseZone, Condition, CounterKind, EnterModifier, Filter, GiftKind,
    Modifier, PlayerRel, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::mana::ManaColor;

const LINKED_EXILE_BEAST: u32 = 8700;
const KILL_SPELL: u32 = 8701;
const BLOODTHIRST_SCOUT: u32 = 8702;
const POKE_INSTANT: u32 = 8703;
const COUNTER_DOUBLER: u32 = 8704;
const IMPENDING_AVATAR: u32 = 8705;
const CREATURE_DOOM: u32 = 8706;
const GIFT_WATCHER: u32 = 8707;
const GIFT_CARD_SORCERY: u32 = 8708;
const GIFT_FISH_OTTER: u32 = 8709;
const COUNTER_INSTANT: u32 = 8710;
const HEXPROOF_AEGIS: u32 = 8711;
const GIFT_FOOD_SPELL: u32 = 8712;
const GIFT_TREASURE_SPELL: u32 = 8713;
const GIFT_OCTOPUS_SPELL: u32 = 8714;
const GIFT_TURN_SPELL: u32 = 8715;

fn leak(id: u32, face: FaceDef) -> &'static CardDef {
    let keywords = face.keywords;
    let abilities = face.abilities;
    card(id, face, keywords, abilities)
}

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    refresh_offer(engine);
}

fn drain_stack(engine: &mut Bench) {
    while !engine.state.zones.stack_is_empty()
        || matches!(engine.pending(), Pending::Arrange { .. })
    {
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            other => panic!("unexpected pending while draining stack: {other:?}"),
        }
    }
}

fn advance_to_end_step_resolved(engine: &mut Bench, target_active: PlayerId) {
    for _ in 0..256 {
        let in_end_step = engine.state.turn.active == target_active
            && matches!(engine.state.turn.step, crate::turn::Step::End);
        if in_end_step
            && engine.state.zones.stack_is_empty()
            && engine.trigger_queue.is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == target_active)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .expect("no attackers");
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .expect("no blockers");
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine.state.zones.list(ZoneLocation::Hand(player)).clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..usize::from(count)].to_vec(),
                        },
                    )
                    .expect("cleanup discard");
            }
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            other => panic!("unexpected pending while advancing to end step: {other:?}"),
        }
    }
    panic!("failed to reach resolved end step for {target_active:?}");
}

fn advance_past_current_end_step(engine: &mut Bench) {
    let start_turn = engine.state.turn.number;
    let start_step = engine.state.turn.step;
    for _ in 0..64 {
        if engine.state.turn.number != start_turn || engine.state.turn.step != start_step {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine.state.zones.list(ZoneLocation::Hand(player)).clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..usize::from(count)].to_vec(),
                        },
                    )
                    .expect("cleanup discard");
            }
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            other => panic!("unexpected pending while leaving end step: {other:?}"),
        }
    }
    panic!("failed to leave current end step");
}

fn linked_exile_beast() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 2] = [
        triggered!(
            Trigger::ETB,
            &[Effect::ChooseCard {
                chooser: PlayerRel::You,
                zone: ChooseZone::Battlefield,
                filter: &Filter::YOUR_LAND,
                min: 1,
                max: Amount::Fixed(1),
                movement: ChooseMove::ExileLinked,
                then: &[],
            }]
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedToBattlefield]
        ),
    ];
    leak(
        LINKED_EXILE_BEAST,
        FaceDef {
            abilities: &ABILITIES,
            ..creature_face("Linked Exile Beast", "{1}{U}", 2, 2)
        },
    )
}

fn kill_spell() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    leak(
        KILL_SPELL,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Kill Spell", "{B}", TypeSet::INSTANT)
        },
    )
}

fn bloodthirst_scout() -> &'static CardDef {
    static ENTERS: [EnterModifier; 1] = [EnterModifier::WithCountersIf {
        kind: CounterKind::P1P1,
        count: Amount::Fixed(2),
        condition: Condition::OpponentLostLifeThisTurn,
    }];
    leak(
        BLOODTHIRST_SCOUT,
        FaceDef {
            enter_modifiers: &ENTERS,
            ..creature_face("Bloodthirst Scout", "{1}{R}", 2, 2)
        },
    )
}

fn poke_instant() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::Player(PlayerRel::EachOpponent),
    }])];
    leak(
        POKE_INSTANT,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Poke Instant", "{R}", TypeSet::INSTANT)
        },
    )
}

fn counter_doubler() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [static_ability!(
        Filter::Any,
        Modifier::DoubleCountersPlacedByYou
    )];
    leak(
        COUNTER_DOUBLER,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Counter Doubler", "{4}{G}", TypeSet::ENCHANTMENT)
        },
    )
}

fn impending_avatar() -> &'static CardDef {
    static SUBTYPES: [baylee_core::ids::SubtypeId; 1] = [subtypes::creature::AVATAR];
    leak(
        IMPENDING_AVATAR,
        FaceDef {
            mana_cost: ManaCost::parse("{4}{W}{W}"),
            types: TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
            subtypes: &SUBTYPES,
            power: Some(4),
            toughness: Some(5),
            impending: Some((2, ManaCost::parse("{2}{W}"))),
            ..land_face("Impending Avatar")
        },
    )
}

fn creature_doom() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    leak(
        CREATURE_DOOM,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Creature Doom", "{1}{B}", TypeSet::INSTANT)
        },
    )
}

fn gift_watcher() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [triggered!(
        Trigger::GaveGift(PlayerRel::You),
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }]
    )];
    leak(
        GIFT_WATCHER,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Gift Watcher", "{1}{W}", TypeSet::ENCHANTMENT)
        },
    )
}

fn gift_card_sorcery() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::IfCondition {
        condition: Condition::GiftWasPromised,
        then: &[Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::GiftRecipient,
        }],
        otherwise: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
    }])];
    leak(
        GIFT_CARD_SORCERY,
        FaceDef {
            abilities: &ABILITIES,
            gift: Some(GiftKind::Card),
            ..face("Gift Card Sorcery", "{1}{B}", TypeSet::SORCERY)
        },
    )
}

fn gift_fish_otter() -> &'static CardDef {
    leak(
        GIFT_FISH_OTTER,
        FaceDef {
            gift: Some(GiftKind::TappedFish),
            ..creature_face("Gift Fish Otter", "{1}{U}", 2, 1)
        },
    )
}

fn counter_instant() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::CounterTargetSpell],
        targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        second_targets: None,
        condition: None,
    }];
    leak(
        COUNTER_INSTANT,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Counter Instant", "{U}", TypeSet::INSTANT)
        },
    )
}

fn hexproof_aegis() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [static_ability!(
        Filter::This,
        Modifier::PlayerHexproof {
            who: PlayerRel::You
        }
    )];
    leak(
        HEXPROOF_AEGIS,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Hexproof Aegis", "{1}{W}", TypeSet::ENCHANTMENT)
        },
    )
}

fn gift_spell(id: u32, name: &'static str, kind: GiftKind) -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];
    leak(
        id,
        FaceDef {
            abilities: &ABILITIES,
            gift: Some(kind),
            ..face(name, "{G}", TypeSet::SORCERY)
        },
    )
}

/// 1. `ChooseMove::ExileLinked` (`CR 607.2a`):
/// - Untargeted `Effect::ChooseCard` exiles the chosen permanent and links it
///   to `res.source` (`Rider::Linked { host: beast, until: None }`).
/// - An unrelated exiled card is left in exile when the host leaves.
/// - Destroying the host triggers `Effect::ReturnLinkedToBattlefield`, returning
///   the exact linked land to the battlefield.
#[test]
fn wave7a_choose_move_exile_linked_returns_on_source_ltb_and_tracks_incarnation() {
    let mut engine = bench(
        1,
        vec![linked_exile_beast(), kill_spell()],
        [
            Seat::with(&[forest(), forest()]).holding(&[LINKED_EXILE_BEAST, KILL_SPELL]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let lands = objects(&engine, ZoneLocation::Battlefield, forest());
    assert_eq!(lands.len(), 2);
    let unrelated_land = lands[0];
    let chosen_land = lands[1];
    engine
        .state
        .move_object(
            unrelated_land,
            ZoneLocation::Exile(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("exile unrelated land beforehand");

    let beast = the(&engine, ZoneLocation::Hand(me()), LINKED_EXILE_BEAST);
    let kill = the(&engine, ZoneLocation::Hand(me()), KILL_SPELL);

    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: beast })
        .expect("cast LINKED_EXILE_BEAST");

    // Resolve beast onto battlefield -> ETB trigger goes on stack -> resolves asking ChooseCards!
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            Pending::ChooseCards {
                player,
                options,
                prompt: ChoicePrompt::Generic,
                ..
            } => {
                assert_eq!(options, vec![chosen_land]);
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![chosen_land],
                        },
                    )
                    .expect("choose land to exile linked");
                break;
            }
            other => panic!("unexpected pending during linked exile ETB: {other:?}"),
        }
    }
    drain_stack(&mut engine);

    assert_eq!(
        engine.state.object(chosen_land).expect("land").zone,
        Zone::Exile,
        "chosen land was moved to exile"
    );
    assert!(
        engine
            .state
            .object(chosen_land)
            .expect("land")
            .riders
            .contains(&Rider::Linked {
                host: beast,
                until: None,
            }),
        "ChooseMove::ExileLinked attached Rider::Linked to the exiled land"
    );

    // Destroy LINKED_EXILE_BEAST -> LTB trigger returns `chosen_land` to battlefield,
    // leaving `letunrelated_land` in exile!
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: kill })
        .expect("cast KILL_SPELL");
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![beast],
                    players: vec![],
                },
            )
            .expect("target beast");
    }
    drain_stack(&mut engine);

    assert_eq!(
        engine.state.object(chosen_land).expect("chosen land").zone,
        Zone::Battlefield,
        "ReturnLinkedToBattlefield returned the land exiled by ChooseMove::ExileLinked"
    );
    assert_eq!(
        engine
            .state
            .object(unrelated_land)
            .expect("unrelated land")
            .zone,
        Zone::Exile,
        "unrelated exiled card remained in exile"
    );
}

/// 2. `EnterModifier::WithCountersIf` (`CR 614.1c`, `CR 614.12`, `CR 614.16`):
/// - Enters with `0` counters when `condition` (`OpponentLostLifeThisTurn`) is `false`.
/// - Enters with `2` `+1/+1` counters when `condition` is `true`.
/// - Under `Modifier::DoubleCountersPlacedByYou`, enters with `4` `+1/+1` counters
///   (doubled once, never applied twice across `entry/` and `entering.rs`).
#[test]
fn wave7a_enter_modifier_with_counters_if_evaluates_condition_and_doubles_counters() {
    let mut engine = bench(
        2,
        vec![bloodthirst_scout(), poke_instant(), counter_doubler()],
        [
            Seat::with(&[]).holding(&[
                BLOODTHIRST_SCOUT,
                POKE_INSTANT,
                BLOODTHIRST_SCOUT,
                COUNTER_DOUBLER,
                BLOODTHIRST_SCOUT,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let scouts = objects(&engine, ZoneLocation::Hand(me()), BLOODTHIRST_SCOUT);
    let scout_dry = scouts[0];
    let scout_fed = scouts[1];
    let scout_doubled = scouts[2];
    let poke = the(&engine, ZoneLocation::Hand(me()), POKE_INSTANT);
    let doubler = the(&engine, ZoneLocation::Hand(me()), COUNTER_DOUBLER);

    // 1) Opponent has NOT lost life yet -> scout_dry enters with 0 +1/+1 counters (2/2).
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: scout_dry })
        .expect("cast scout_dry");
    drain_stack(&mut engine);
    assert_eq!(
        engine
            .state
            .object(scout_dry)
            .expect("scout_dry")
            .counters
            .get(CounterKind::P1P1),
        0,
        "condition false -> 0 counters placed"
    );

    // 2) Deal 1 damage to opponent -> scout_fed enters with 2 +1/+1 counters (4/4).
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: poke })
        .expect("cast poke");
    drain_stack(&mut engine);

    add_pool(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: scout_fed })
        .expect("cast scout_fed");
    drain_stack(&mut engine);
    assert_eq!(
        engine
            .state
            .object(scout_fed)
            .expect("scout_fed")
            .counters
            .get(CounterKind::P1P1),
        2,
        "condition true -> 2 +1/+1 counters placed"
    );
    assert_eq!(
        engine
            .state
            .object(scout_fed)
            .expect("scout_fed")
            .characteristics()
            .power,
        Some(4)
    );

    // 3) Put Counter Doubler onto battlefield -> scout_doubled enters with 4 (not 8!) +1/+1 counters.
    add_pool(&mut engine, me(), ManaColor::Green, 5);
    engine
        .apply(me(), PlayerAction::CastSpell { card: doubler })
        .expect("cast doubler");
    drain_stack(&mut engine);

    add_pool(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: scout_doubled,
            },
        )
        .expect("cast scout_doubled");
    drain_stack(&mut engine);
    assert_eq!(
        engine
            .state
            .object(scout_doubled)
            .expect("scout_doubled")
            .counters
            .get(CounterKind::P1P1),
        4,
        "CR 614.16: WithCountersIf doubled once (2 -> 4) under DoubleCountersPlacedByYou"
    );
}

/// 3. `Impending N—[cost]` (`CR 702.176a`, `CR 205.1a`, `CR 208.3`, `CR 603.4`):
/// - Offered via `CastModeKind::Impending` for `{2}{W}` (while `{4}{W}{W}` is unaffordable).
/// - Remains a Creature Enchantment spell on the stack (`CR 702.176a`).
/// - Enters the battlefield with `2` `CounterKind::Time` counters and `Rider::ImpendingPaid`.
/// - Layer 4 removes `TypeSet::CREATURE`, creature subtype `Avatar`, and `power`/`toughness`
///   while `Time >= 1` (`CREATURE_DOOM` has no legal target!).
/// - Controller's end step 1 removes 1 time counter (`2 -> 1`, still noncreature);
///   opponent's end step removes 0; controller's end step 2 removes the last time counter
///   (`1 -> 0`) so it immediately awakens as a `4/5` Enchantment Creature — Avatar!
#[test]
fn wave7a_impending_alternative_cost_time_counters_layer4_noncreature_and_end_step_awakening() {
    let mut engine = bench(
        3,
        vec![impending_avatar(), creature_doom()],
        [
            Seat::with(&[]).holding(&[IMPENDING_AVATAR]),
            Seat::with(&[]).holding(&[CREATURE_DOOM]),
        ],
    );
    to_main(&mut engine, me());

    let avatar = the(&engine, ZoneLocation::Hand(me()), IMPENDING_AVATAR);
    let doom = the(&engine, ZoneLocation::Hand(them()), CREATURE_DOOM);

    // Give {2}{W}: normal {4}{W}{W} is unaffordable, Impending {2}{W} is offered!
    add_pool(&mut engine, me(), ManaColor::White, 3);
    let (_, legal) = offer(&engine);
    assert!(
        legal
            .cast_ways
            .iter()
            .any(|(id, desc)| *id == avatar && desc.kind == CastModeKind::Impending),
        "Impending Avatar should offer CastModeKind::Impending for {{2}}{{W}}"
    );

    engine
        .apply(me(), PlayerAction::CastSpell { card: avatar })
        .expect("cast avatar via impending");
    if let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    {
        let idx = options
            .iter()
            .position(|o| o.kind == CastModeKind::Impending)
            .expect("Impending option");
        engine
            .apply(player, PlayerAction::ChooseMode(idx))
            .expect("choose Impending");
    }

    // While on the stack, CR 702.176a keeps TypeSet::CREATURE!
    let stack_chars = engine
        .state
        .object(avatar)
        .expect("avatar on stack")
        .characteristics();
    assert!(
        stack_chars.types.contains(TypeSet::CREATURE)
            && stack_chars.types.contains(TypeSet::ENCHANTMENT),
        "CR 702.176a: Impending spell is still a Creature Enchantment on the stack"
    );

    drain_stack(&mut engine);

    // Now on the battlefield with 2 time counters: NOT a creature, no Avatar subtype, no P/T!
    let bf_obj = engine.state.object(avatar).expect("avatar on battlefield");
    assert_eq!(bf_obj.zone, Zone::Battlefield);
    assert!(bf_obj.riders.contains(&Rider::ImpendingPaid));
    assert_eq!(bf_obj.counters.get(CounterKind::Time), 2);
    let bf_chars = bf_obj.characteristics();
    assert!(
        !bf_chars.types.contains(TypeSet::CREATURE)
            && bf_chars.types.contains(TypeSet::ENCHANTMENT),
        "CR 702.176a: Impending permanent with time counters is not a creature"
    );
    assert!(
        !bf_chars.subtypes.contains(subtypes::creature::AVATAR),
        "CR 205.1a: creature subtypes are removed while not a creature"
    );
    assert_eq!(
        (bf_chars.power, bf_chars.toughness),
        (None, None),
        "CR 208.3: noncreature permanent has no power or toughness"
    );

    // Opponent's CREATURE_DOOM cannot even be cast because no creature is on the battlefield!
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass to opponent");
    add_pool(&mut engine, them(), ManaColor::Black, 2);
    let (_, opp_legal) = offer(&engine);
    assert!(
        !opp_legal.castable.contains(&doom),
        "creature-only removal cannot target noncreature Impending permanent"
    );

    // Advance to me()'s end step 1 -> time counters drop 2 -> 1 (still noncreature!).
    advance_to_end_step_resolved(&mut engine, me());
    assert_eq!(
        engine
            .state
            .object(avatar)
            .expect("avatar")
            .counters
            .get(CounterKind::Time),
        1,
        "end step 1 removed 1 time counter (2 -> 1)"
    );
    assert!(
        !engine
            .state
            .object(avatar)
            .expect("avatar")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "still has 1 time counter -> still not a creature"
    );
    advance_past_current_end_step(&mut engine);

    // Advance to opponent's end step -> time counters stay at 1!
    advance_to_end_step_resolved(&mut engine, them());
    assert_eq!(
        engine
            .state
            .object(avatar)
            .expect("avatar")
            .counters
            .get(CounterKind::Time),
        1,
        "opponent's end step must not trigger Impending"
    );
    advance_past_current_end_step(&mut engine);

    // Advance to me()'s next end step -> last time counter removed (1 -> 0), awakens as 4/5 Avatar!
    advance_to_end_step_resolved(&mut engine, me());
    let awakened = engine.state.object(avatar).expect("awakened avatar");
    assert_eq!(awakened.counters.get(CounterKind::Time), 0);
    let chars = awakened.characteristics();
    assert!(
        chars.types.contains(TypeSet::CREATURE) && chars.types.contains(TypeSet::ENCHANTMENT),
        "with 0 time counters, Impending permanent becomes a Creature again"
    );
    assert!(
        chars.subtypes.contains(subtypes::creature::AVATAR),
        "Avatar creature subtype returns once it is a creature again"
    );
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(4), Some(5)),
        "printed 4/5 power and toughness return"
    );
}

/// 4. `Gift` on Instant/Sorcery & Permanent (`CR 702.174a–k`):
/// - Declining `YesNoPrompt::Gift` runs the unpromised branch and emits no `GiftGiven`.
/// - Accepting `YesNoPrompt::Gift { kind: GiftKind::Card }` in a 2-player game
///   auto-promises to the sole opponent, delivers the card draw BEFORE spell effects
///   (`CR 702.174j`), satisfies `Condition::GiftWasPromised`, binds
///   `PlayerRel::GiftRecipient`, and fires `Trigger::GaveGift` (`CR 702.174c`).
/// - Countering a promised Gift spell suppresses both the gift effect and
///   `Trigger::GaveGift` (`CR 702.174j`).
/// - Permanent with `GiftKind::TappedFish` queues an ETB trigger on entry that
///   creates a **tapped** `1/1` blue `Fish` creature token under the opponent's
///   control and fires `Trigger::GaveGift`.
#[test]
fn wave7a_gift_instant_sorcery_and_permanent_etb_delivery_condition_recipient_and_trigger() {
    let mut engine = bench(
        4,
        vec![
            gift_watcher(),
            gift_card_sorcery(),
            gift_fish_otter(),
            counter_instant(),
        ],
        [
            Seat::with(&[GIFT_WATCHER]).holding(&[
                GIFT_CARD_SORCERY,
                GIFT_CARD_SORCERY,
                GIFT_CARD_SORCERY,
                GIFT_FISH_OTTER,
            ]),
            Seat::with(&[]).holding(&[COUNTER_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    let sorceries = objects(&engine, ZoneLocation::Hand(me()), GIFT_CARD_SORCERY);
    let sorcery_declined = sorceries[0];
    let sorcery_promised = sorceries[1];
    let sorcery_countered = sorceries[2];
    let otter = the(&engine, ZoneLocation::Hand(me()), GIFT_FISH_OTTER);
    let counter = the(&engine, ZoneLocation::Hand(them()), COUNTER_INSTANT);
    let opp_hand_start = engine.state.zones.list(ZoneLocation::Hand(them())).len();

    // 1) Decline Gift -> caster gains 1 life (`otherwise`), opponent draws 0, GiftWatcher does not fire.
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: sorcery_declined,
            },
        )
        .expect("cast sorcery_declined");
    match engine.pending().clone() {
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::Gift {
                kind: GiftKind::Card,
            },
            ..
        } => {
            engine
                .apply(player, PlayerAction::YesNo(false))
                .expect("decline gift");
        }
        other => panic!("expected YesNoPrompt::Gift(Card), got {other:?}"),
    }
    drain_stack(&mut engine);
    assert_eq!(
        life(&engine, me()),
        21,
        "declined gift ran `otherwise` (+1)"
    );
    assert_eq!(life(&engine, them()), 20);
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(them())).len(),
        opp_hand_start,
        "declining gift drew 0 cards for opponent"
    );

    // 2) Accept Gift -> opponent draws 1 card, loses 2 life (`GiftWasPromised` + `GiftRecipient`),
    //    and GiftWatcher triggers (+3 life for me() -> 24)!
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: sorcery_promised,
            },
        )
        .expect("cast sorcery_promised");
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("promise gift");
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(them())).len(),
        opp_hand_start + 1,
        "CR 702.174e: opponent drew 1 card from Gift a card"
    );
    assert_eq!(
        life(&engine, them()),
        18,
        "Condition::GiftWasPromised + PlayerRel::GiftRecipient made opponent lose 2 life"
    );
    assert_eq!(
        life(&engine, me()),
        24,
        "CR 702.174c: Trigger::GaveGift fired on gift sorcery resolution (+3 life)"
    );

    // 3) Countered Gift spell (`CR 702.174j`) -> neither gift draw nor Trigger::GaveGift happens!
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: sorcery_countered,
            },
        )
        .expect("cast sorcery_countered");
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("promise gift on countered spell");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass priority to opponent");
    add_pool(&mut engine, them(), ManaColor::Blue, 1);
    engine
        .apply(them(), PlayerAction::CastSpell { card: counter })
        .expect("cast counterspell");
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![sorcery_countered],
                    players: vec![],
                },
            )
            .expect("target sorcery_countered");
    }
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(them())).len(),
        opp_hand_start,
        "CR 702.174j: countered spell did not deliver gift card (opponent spent counterspell from hand)"
    );
    assert_eq!(
        life(&engine, me()),
        24,
        "CR 702.174c/j: countered gift spell did not trigger GaveGift"
    );

    // 4) Permanent with `GiftKind::TappedFish` (`CR 702.174b`, `CR 702.174f`):
    //    ETB gift trigger creates a tapped 1/1 blue Fish under opponent's control and triggers GaveGift (+3 -> 27)!
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: otter })
        .expect("cast gift_fish_otter");
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("promise tapped Fish gift");
    drain_stack(&mut engine);

    let opp_fish: Vec<ObjectId> = engine
        .state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state.object(*id).is_some_and(|o| {
                o.controller == them()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::FISH)
            })
        })
        .collect();
    assert_eq!(opp_fish.len(), 1, "opponent received 1 Fish token");
    let fish = engine.state.object(opp_fish[0]).expect("fish token");
    assert!(
        fish.status.contains(crate::object::Status::TAPPED),
        "CR 702.174f: gifted Fish token enters tapped"
    );
    let fish_chars = fish.characteristics();
    assert_eq!((fish_chars.power, fish_chars.toughness), (Some(1), Some(1)));
    assert_eq!(fish_chars.colors, ColorSet::of(Color::Blue));
    assert_eq!(
        life(&engine, me()),
        27,
        "CR 702.174c: permanent ETB gift trigger resolving fired Trigger::GaveGift (+3 life)"
    );
}

/// 5. Remaining `GiftKind`s (`Food`, `Treasure`, `Octopus`, `ExtraTurn`) & 3-player
///    untargeted `Pending::ChoosePlayer` (`CR 702.174a`, `CR 702.174d`, `CR 702.174g–i`):
/// - In a 3-player game (`P0`, `P1`, `P2`) where `P2` has Hexproof, promising a Gift
///   prompts `Pending::ChoosePlayer` offering `[P1, P2]` (untargeted!) and lets `P0`
///   deliver `Food`, `Treasure`, `8/8` blue `Octopus`, and `ExtraTurn` to `P2`.
#[test]
fn wave7a_gift_all_remaining_kinds_and_multiplayer_opponent_choice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let p2 = PlayerId::new(2);
    let mut engine = bench(
        5,
        vec![
            hexproof_aegis(),
            gift_spell(GIFT_FOOD_SPELL, "Gift Food Spell", GiftKind::Food),
            gift_spell(
                GIFT_TREASURE_SPELL,
                "Gift Treasure Spell",
                GiftKind::Treasure,
            ),
            gift_spell(GIFT_OCTOPUS_SPELL, "Gift Octopus Spell", GiftKind::Octopus),
            gift_spell(GIFT_TURN_SPELL, "Gift Turn Spell", GiftKind::ExtraTurn),
        ],
        [
            Seat::with(&[]).holding(&[
                GIFT_FOOD_SPELL,
                GIFT_TREASURE_SPELL,
                GIFT_OCTOPUS_SPELL,
                GIFT_TURN_SPELL,
            ]),
            Seat::with(&[]),
            Seat::with(&[HEXPROOF_AEGIS]),
        ],
    );
    to_main(&mut engine, p0);

    for &spell_idx in &[
        GIFT_FOOD_SPELL,
        GIFT_TREASURE_SPELL,
        GIFT_OCTOPUS_SPELL,
        GIFT_TURN_SPELL,
    ] {
        let card = the(&engine, ZoneLocation::Hand(p0), spell_idx);
        add_pool(&mut engine, p0, ManaColor::Green, 1);
        engine
            .apply(p0, PlayerAction::CastSpell { card })
            .expect("cast multiplayer gift spell");
        engine
            .apply(p0, PlayerAction::YesNo(true))
            .expect("accept gift promise");
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0);
                assert_eq!(
                    options,
                    vec![p1, p2],
                    "Hexproof opponent P2 is still selectable for untargeted Gift choice"
                );
                engine
                    .apply(p0, PlayerAction::ChoosePlayer(p2))
                    .expect("choose P2 as gift recipient");
            }
            other => panic!("expected Pending::ChoosePlayer([P1, P2]), got {other:?}"),
        }
        drain_stack(&mut engine);
    }

    let p2_perms: Vec<&crate::object::GameObject> = engine
        .state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter_map(|id| engine.state.object(*id).filter(|o| o.controller == p2))
        .collect();

    assert!(
        p2_perms.iter().any(|o| o
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::FOOD)),
        "CR 702.174d: P2 controls a Food token"
    );
    assert!(
        p2_perms.iter().any(|o| o
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::TREASURE)),
        "CR 702.174h: P2 controls a Treasure token"
    );
    assert!(
        p2_perms.iter().any(|o| {
            let c = o.characteristics();
            c.subtypes.contains(subtypes::creature::OCTOPUS)
                && c.colors == ColorSet::of(Color::Blue)
                && (c.power, c.toughness) == (Some(8), Some(8))
        }),
        "CR 702.174i: P2 controls an 8/8 blue Octopus token"
    );
    assert_eq!(
        engine.state.extra_turns.front().copied(),
        Some(p2),
        "CR 702.174g: P2 received an extra turn after this one"
    );
}
