//! Wave 8a engine rule tests:
//! - `Warp [cost]` (`CR 702.185a–c`): alternative cast cost from hand only,
//!   `Trigger::SpellWarped`, `Condition::WarpCostPaid`, `Condition::SpellWarpedThisTurn`,
//!   `Amount::SpellsWarpedThisTurn`, end-step exile (`Effect::ExileWarped` /
//!   `Rider::WarpedInExile` / `Filter::WarpedInExile`), later-turn owner recast
//!   at normal cost (not warpable from exile), blink/counter/token-copy rules.
//! - Snow Mana `{S}` (`CR 107.4h`): `ManaPool::snow_total`, `PaidRecord::snow_mana_spent`,
//!   `Rider::SnowManaSpent` preserved across `Stack -> Battlefield` and onto triggers,
//!   `Amount::SnowManaSpentToCast`, `Condition::SnowManaWasSpent`, `Filter::CmcAtMostSnowManaSpent`.
//! - `Devour N` (`CR 702.82a–c`): `EnterModifier::Devour { filter, factor }` (`0` -> `count * count`),
//!   `ChoicePrompt::Devour { factor }`, `CR 614.16` counter doublers, `Amount::EntrySacrifices`.
//! - `Spree` (`CR 702.172a–c`, `CR 700.2c–d`, `CR 608.2b–c`): 3+ target clauses across modes,
//!   player targets in non-first modes, `second_targets` within modes, partial target illegality,
//!   and `Plot` + `Spree` waiving base cost while charging chosen modes' additional costs.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, PLOT_CARD};
use crate::object::Rider;
use baylee_cards_dsl::prelude::{activated, spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, Cost, CostPart, CounterKind, Duration, EnterModifier, Filter, ModeCount,
    Modifier, PlayerRel, SpellMode, TargetReq, TargetSlot, TargetSpec, Trigger,
};
use baylee_core::mana::ManaColor;
use baylee_core::types::SupertypeSet;

const WARP_SCOUT: u32 = 8800;
const WARP_WATCHER: u32 = 8801;
const BLINK_INSTANT: u32 = 8802;
const COUNTER_INSTANT: u32 = 8803;
const COPY_INSTANT: u32 = 8804;
const SNOW_FOREST_CARD: u32 = 8805;
const FROST_ELEMENTAL: u32 = 8806;
const TINY_GOBLIN: u32 = 8807;
const TWO_DROP_BEAR: u32 = 8808;
const SAC_ALTAR: u32 = 8809;
const DEVOUR_HYDRA: u32 = 8810;
const THROMOK_WURM: u32 = 8811;
const COUNTER_DOUBLER: u32 = 8812;
const SPREE_COMMAND: u32 = 8813;
const KILL_INSTANT: u32 = 8814;
const PLOTTED_SPREE: u32 = 8815;

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

fn settle_stack(engine: &mut Bench, seat: PlayerId) {
    for _ in 0..256 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            pending => {
                assert!(
                    walk_past(engine, &pending),
                    "unexpected pending while settling stack: {pending:?}"
                );
            }
        }
    }
    panic!("failed to settle stack");
}

fn advance_to_main(engine: &mut Bench, seat: PlayerId) {
    for _ in 0..512 {
        if holds_priority_in(engine, seat, crate::turn::Step::Main)
            && engine.state().turn.phase == Phase::FirstMain
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
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
            pending => {
                assert!(
                    walk_past(engine, &pending),
                    "unexpected pending while advancing to main: {pending:?}"
                );
            }
        }
    }
    panic!("failed to reach {seat:?}'s main phase");
}

static WARP_SCOUT_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::EntersBattlefield(&Filter::This),
    &[Effect::gain_life(2)],
    condition = Some(Condition::WarpCostPaid),
)];

static WARP_WATCHER_ABILITIES: &[AbilityDef] = &[
    triggered!(
        Trigger::SpellWarped {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        },
        &[Effect::GainLife {
            amount: Amount::SpellsWarpedThisTurn(PlayerRel::You),
        }],
    ),
    AbilityDef::ActivatedConditional {
        cost: Cost::FREE,
        condition: Condition::SpellWarpedThisTurn(Some(PlayerRel::You)),
        effects: &[Effect::gain_life(1)],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::PerTurn(1),
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    },
];

fn warp_cards() -> Vec<&'static CardDef> {
    static BLINK_FX: &[AbilityDef] = &[spell!(
        &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static COUNTER_FX: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    static COPY_FX: &[AbilityDef] = &[spell!(
        &[Effect::CopyTargetSpell { mods: &[] }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    vec![
        card(
            WARP_SCOUT,
            FaceDef {
                warp: Some(Cost {
                    mana: ManaCost::parse("{1}{U}"),
                    parts: &[],
                }),
                ..creature_face("Warp Scout", "{3}{U}", 3, 2)
            },
            KeywordSet::WARP,
            WARP_SCOUT_ABILITIES,
        ),
        card(
            WARP_WATCHER,
            face("Warp Watcher", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            WARP_WATCHER_ABILITIES,
        ),
        card(
            BLINK_INSTANT,
            face("Phase Flicker", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BLINK_FX,
        ),
        card(
            COUNTER_INSTANT,
            face("Hard No", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            COUNTER_FX,
        ),
        card(
            COPY_INSTANT,
            face("Mirror Warp", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            COPY_FX,
        ),
    ]
}

#[test]
fn warp_casts_from_hand_exiles_at_end_step_and_recasts_on_later_turn() {
    let mut engine = bench(
        801,
        warp_cards(),
        [
            Seat::with(&[WARP_WATCHER]).holding(&[WARP_SCOUT]).at(20),
            Seat::with(&[]).at(20),
        ],
    );
    advance_to_main(&mut engine, me());
    let watcher = the(&engine, ZoneLocation::Battlefield, WARP_WATCHER);
    let scout = the(&engine, ZoneLocation::Hand(me()), WARP_SCOUT);
    assert!(!offer(&engine).1.abilities.contains(&(watcher, 1)));

    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let ways = engine.cast_options(me(), scout, false).expect("warp open");
    assert_eq!(ways.len(), 1);
    assert_eq!(ways[0].kind, CastModeKind::Warp);
    engine
        .apply(me(), PlayerAction::CastSpell { card: scout })
        .expect("cast with warp");
    settle_stack(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, WARP_SCOUT),
        vec![scout]
    );
    assert_eq!(
        life(&engine, me()),
        23,
        "+1 from SpellWarped watcher (SpellsWarpedThisTurn=1) + 2 from WarpCostPaid ETB"
    );
    assert!(offer(&engine).1.abilities.contains(&(watcher, 1)));

    // At the beginning of the next end step, WARP_EXILE exiles the scout with WarpedInExile.
    advance_to_main(&mut engine, them());
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), WARP_SCOUT),
        vec![scout]
    );
    assert!(
        crate::eval::matches(
            &Filter::WarpedInExile,
            engine.state(),
            engine.state().object(scout).unwrap(),
            me(),
            scout
        ),
        "exiled card matches Filter::WarpedInExile"
    );
    // Non-owner cannot cast it on their turn.
    add_pool(&mut engine, them(), ManaColor::Blue, 4);
    assert!(!offer(&engine).1.castable.contains(&scout));

    // On owner's later turn, SpellWarpedThisTurn resets and scout is castable ONLY at normal cost.
    advance_to_main(&mut engine, me());
    assert!(!offer(&engine).1.abilities.contains(&(watcher, 1)));
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    assert!(
        !offer(&engine).1.castable.contains(&scout),
        "cannot warp again from exile for {{1}}{{U}}"
    );
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let exile_ways = engine
        .cast_options(me(), scout, false)
        .expect("normal from exile");
    assert_eq!(exile_ways.len(), 1);
    assert_eq!(exile_ways[0].kind, CastModeKind::Normal);
    let life_before = life(&engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: scout })
        .expect("recast from exile");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        life_before,
        "WarpCostPaid is false on recast"
    );

    // Stays on the battlefield across the next end step.
    advance_to_main(&mut engine, them());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, WARP_SCOUT),
        vec![scout]
    );
}

#[test]
fn warp_blink_counter_and_spell_copy_follow_cr_702_185() {
    // 1. Blinking before end step creates a new object that is NOT exiled at end step.
    let mut engine = bench(
        802,
        warp_cards(),
        [
            Seat::with(&[]).holding(&[WARP_SCOUT, BLINK_INSTANT]).at(20),
            Seat::with(&[]).at(20),
        ],
    );
    advance_to_main(&mut engine, me());
    let scout = the(&engine, ZoneLocation::Hand(me()), WARP_SCOUT);
    let blink = the(&engine, ZoneLocation::Hand(me()), BLINK_INSTANT);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: scout })
        .unwrap();
    settle_stack(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: blink })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![scout],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    advance_to_main(&mut engine, them());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, WARP_SCOUT),
        vec![scout],
        "blinked permanent is a new incarnation and stays on the battlefield"
    );

    // 2. Countered warped spell goes to graveyard without WarpedInExile.
    let mut countered = bench(
        803,
        warp_cards(),
        [
            Seat::with(&[])
                .holding(&[WARP_SCOUT, COUNTER_INSTANT])
                .at(20),
            Seat::with(&[]).at(20),
        ],
    );
    advance_to_main(&mut countered, me());
    let scout2 = the(&countered, ZoneLocation::Hand(me()), WARP_SCOUT);
    let counter = the(&countered, ZoneLocation::Hand(me()), COUNTER_INSTANT);
    add_pool(&mut countered, me(), ManaColor::Blue, 2);
    countered
        .apply(me(), PlayerAction::CastSpell { card: scout2 })
        .unwrap();
    countered
        .apply(me(), PlayerAction::CastSpell { card: counter })
        .unwrap();
    countered
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![scout2],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut countered, me());
    assert_eq!(
        objects(&countered, ZoneLocation::Graveyard(me()), WARP_SCOUT),
        vec![scout2]
    );
    assert!(
        !countered
            .state()
            .object(scout2)
            .unwrap()
            .riders
            .iter()
            .any(|r| matches!(r, Rider::WarpedInExile(_)))
    );

    // 3. Copied warped permanent spell resolves as a token that exiles at end step and ceases.
    let mut copied = bench(
        804,
        warp_cards(),
        [
            Seat::with(&[]).holding(&[WARP_SCOUT, COPY_INSTANT]).at(20),
            Seat::with(&[]).at(20),
        ],
    );
    advance_to_main(&mut copied, me());
    let scout3 = the(&copied, ZoneLocation::Hand(me()), WARP_SCOUT);
    let copy_spell = the(&copied, ZoneLocation::Hand(me()), COPY_INSTANT);
    add_pool(&mut copied, me(), ManaColor::Blue, 2);
    copied
        .apply(me(), PlayerAction::CastSpell { card: scout3 })
        .unwrap();
    copied
        .apply(me(), PlayerAction::CastSpell { card: copy_spell })
        .unwrap();
    copied
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![scout3],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut copied, me());
    assert_eq!(
        copied.state().zones.list(ZoneLocation::Battlefield).len(),
        2
    );
    advance_to_main(&mut copied, them());
    assert!(
        copied
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .is_empty()
    );
    assert_eq!(
        copied.state().zones.list(ZoneLocation::Exile(me())).len(),
        1
    );
}

#[test]
fn snow_mana_tracks_spent_across_stack_battlefield_and_triggers() {
    static FROST_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::EntersBattlefield(&Filter::This),
            &[
                Effect::GainLife {
                    amount: Amount::SnowManaSpentToCast,
                },
                Effect::DestroyAll {
                    filter: &Filter::And(&[
                        Filter::CREATURE,
                        Filter::ControlledByOpponent,
                        Filter::CmcAtMostSnowManaSpent,
                    ]),
                    no_regen: false,
                },
            ],
            condition = Some(Condition::SnowManaWasSpent),
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::GainLife {
                amount: Amount::SnowManaSpentToCast,
            }],
        ),
    ];
    static ALTAR_AB: &[AbilityDef] = &[activated!(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::Sacrifice(&Filter::CREATURE)],
        },
        &[Effect::gain_life(1)],
    )];
    let defs = vec![
        card(
            SNOW_FOREST_CARD,
            FaceDef {
                supertypes: SupertypeSet::BASIC.union(SupertypeSet::SNOW),
                subtypes: &[baylee_core::generated::subtypes::land::FOREST],
                ..land_face("Snow-Covered Forest")
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            FROST_ELEMENTAL,
            creature_face("Frost Elemental", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            FROST_AB,
        ),
        card(
            TINY_GOBLIN,
            creature_face("Tiny Goblin", "{R}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TWO_DROP_BEAR,
            creature_face("Two-Drop Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SAC_ALTAR,
            face("Sac Altar", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            ALTAR_AB,
        ),
    ];

    // Control: paid with 0 snow mana -> Condition::SnowManaWasSpent is false.
    let mut plain_game = bench(
        805,
        defs.clone(),
        [
            Seat::with(&[forest(), forest()])
                .holding(&[FROST_ELEMENTAL])
                .at(20),
            Seat::with(&[TINY_GOBLIN]).at(20),
        ],
    );
    advance_to_main(&mut plain_game, me());
    float_all(&mut plain_game, me());
    assert_eq!(plain_game.state().players[0].mana_pool.snow_total(), 0);
    let plain_elem = the(&plain_game, ZoneLocation::Hand(me()), FROST_ELEMENTAL);
    plain_game
        .apply(me(), PlayerAction::CastSpell { card: plain_elem })
        .unwrap();
    settle_stack(&mut plain_game, me());
    assert_eq!(life(&plain_game, me()), 20);
    assert_eq!(
        objects(&plain_game, ZoneLocation::Battlefield, TINY_GOBLIN).len(),
        1
    );

    // Positive: 1 snow mana + 1 plain mana -> SnowManaSpent(1) on permanent, ETB, and Dies.
    let mut snow_game = bench(
        806,
        defs,
        [
            Seat::with(&[SNOW_FOREST_CARD, forest(), SAC_ALTAR])
                .holding(&[FROST_ELEMENTAL])
                .at(20),
            Seat::with(&[TINY_GOBLIN, TWO_DROP_BEAR]).at(20),
        ],
    );
    advance_to_main(&mut snow_game, me());
    float_all(&mut snow_game, me());
    assert_eq!(snow_game.state().players[0].mana_pool.snow_total(), 1);
    let elem = the(&snow_game, ZoneLocation::Hand(me()), FROST_ELEMENTAL);
    snow_game
        .apply(me(), PlayerAction::CastSpell { card: elem })
        .unwrap();
    settle_stack(&mut snow_game, me());

    assert_eq!(
        life(&snow_game, me()),
        21,
        "gained 1 life for 1 snow mana spent"
    );
    assert!(
        objects(&snow_game, ZoneLocation::Battlefield, TINY_GOBLIN).is_empty(),
        "MV 1 <= 1 snow spent destroyed"
    );
    assert_eq!(
        objects(&snow_game, ZoneLocation::Battlefield, TWO_DROP_BEAR).len(),
        1,
        "MV 2 > 1 snow spent survives"
    );

    let altar = the(&snow_game, ZoneLocation::Battlefield, SAC_ALTAR);
    activate(&mut snow_game, me(), altar, 0);
    snow_game
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![elem],
            },
        )
        .unwrap();
    settle_stack(&mut snow_game, me());
    assert_eq!(
        life(&snow_game, me()),
        23,
        "+1 from altar + 1 from Dies trigger reading preserved SnowManaSpentToCast"
    );
}

#[test]
fn devour_places_counters_supports_thromok_x_doublers_and_entry_sacrifices() {
    static DEVOUR_2: &[EnterModifier] = &[EnterModifier::devour(2)];
    static DEVOUR_X: &[EnterModifier] = &[EnterModifier::Devour {
        filter: &Filter::CREATURE,
        factor: 0,
    }];
    static DEVOUR_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::EntersBattlefield(&Filter::This),
            &[Effect::GainLife {
                amount: Amount::EntrySacrifices,
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::GainLife {
                amount: Amount::EntrySacrifices,
            }],
        ),
    ];
    static DOUBLER_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::DoubleCountersPlacedByYou,
    )];
    static ALTAR_AB: &[AbilityDef] = &[activated!(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::Sacrifice(&Filter::CREATURE)],
        },
        &[Effect::gain_life(1)],
    )];
    let defs = vec![
        card(
            DEVOUR_HYDRA,
            FaceDef {
                enter_modifiers: DEVOUR_2,
                ..creature_face("Devour Hydra", "{0}", 0, 0)
            },
            KeywordSet::DEVOUR,
            DEVOUR_AB,
        ),
        card(
            THROMOK_WURM,
            FaceDef {
                enter_modifiers: DEVOUR_X,
                ..creature_face("Thromok Wurm", "{0}", 0, 0)
            },
            KeywordSet::DEVOUR,
            &[],
        ),
        card(
            TINY_GOBLIN,
            creature_face("Fodder Goblin", "{R}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            COUNTER_DOUBLER,
            face("Doubling Relic", "{1}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DOUBLER_AB,
        ),
        card(
            SAC_ALTAR,
            face("Sac Altar", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            ALTAR_AB,
        ),
    ];

    // 1. Devour 2 on 0/0 eating 2 goblins + counter doubler -> 8 counters, survives SBAs, EntrySacrifices=2.
    let mut game = bench(
        807,
        defs.clone(),
        [
            Seat::with(&[TINY_GOBLIN, TINY_GOBLIN, COUNTER_DOUBLER, SAC_ALTAR])
                .holding(&[DEVOUR_HYDRA])
                .at(20),
            Seat::with(&[]).at(20),
        ],
    );
    advance_to_main(&mut game, me());
    let goblins = objects(&game, ZoneLocation::Battlefield, TINY_GOBLIN);
    let hydra = the(&game, ZoneLocation::Hand(me()), DEVOUR_HYDRA);
    game.apply(me(), PlayerAction::CastSpell { card: hydra })
        .unwrap();
    walk_until(&mut game, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Devour { factor: 2 },
                ..
            }
        )
    });
    game.apply(me(), PlayerAction::ChooseObjects { objects: goblins })
        .unwrap();
    settle_stack(&mut game, me());
    assert_eq!(
        game.state()
            .object(hydra)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        8,
        "2 devoured * factor 2 * doubler 2 = 8 +1/+1 counters"
    );
    assert_eq!(life(&game, me()), 22, "ETB gained EntrySacrifices=2 life");

    let altar = the(&game, ZoneLocation::Battlefield, SAC_ALTAR);
    activate(&mut game, me(), altar, 0);
    game.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![hydra],
        },
    )
    .unwrap();
    settle_stack(&mut game, me());
    assert_eq!(
        life(&game, me()),
        25,
        "+1 altar + 2 from Dies reading EntrySacrifices"
    );

    // 2. Thromok Devour X (`factor: 0`) eating 3 goblins -> 3 * 3 = 9 +1/+1 counters.
    let mut thromok_game = bench(
        808,
        defs,
        [
            Seat::with(&[TINY_GOBLIN, TINY_GOBLIN, TINY_GOBLIN])
                .holding(&[THROMOK_WURM])
                .at(20),
            Seat::with(&[]).at(20),
        ],
    );
    advance_to_main(&mut thromok_game, me());
    let three = objects(&thromok_game, ZoneLocation::Battlefield, TINY_GOBLIN);
    let wurm = the(&thromok_game, ZoneLocation::Hand(me()), THROMOK_WURM);
    thromok_game
        .apply(me(), PlayerAction::CastSpell { card: wurm })
        .unwrap();
    walk_until(&mut thromok_game, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Devour { factor: 0 },
                ..
            }
        )
    });
    thromok_game
        .apply(me(), PlayerAction::ChooseObjects { objects: three })
        .unwrap();
    settle_stack(&mut thromok_game, me());
    assert_eq!(
        thromok_game
            .state()
            .object(wurm)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        9
    );
}

static SPREE_MODES: &[SpellMode] = &[
    SpellMode {
        effects: &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        cost_override: None,
        additional_cost: Some(ManaCost::parse("{1}")),
    },
    SpellMode {
        effects: &[Effect::GainLifeFor {
            amount: Amount::Fixed(5),
            who: PlayerRel::Chosen,
        }],
        targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets: None,
        cost_override: None,
        additional_cost: Some(ManaCost::parse("{1}")),
    },
    SpellMode {
        effects: &[
            Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(2, 2),
                Duration::UntilEndOfTurn,
            ),
            Effect::Fight {
                fighter: TargetSlot::First,
                foe: TargetSlot::Second,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        second_targets: Some(TargetReq::one(TargetSpec::Object(
            &Filter::OPPONENT_CREATURE,
        ))),
        cost_override: None,
        additional_cost: Some(ManaCost::parse("{G}")),
    },
];

static SPREE_AB: &[AbilityDef] = &[AbilityDef::ModalSpell {
    modes: SPREE_MODES,
    choose: ModeCount::ONE_OR_MORE,
}];

fn spree_cards() -> Vec<&'static CardDef> {
    static KILL_AB: &[AbilityDef] = &[spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    vec![
        card(
            SPREE_COMMAND,
            face("Spree Command", "{R}", TypeSet::INSTANT),
            KeywordSet::SPREE,
            SPREE_AB,
        ),
        card(
            TINY_GOBLIN,
            creature_face("Scout Goblin", "{R}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TWO_DROP_BEAR,
            creature_face("Rival Bear", "{1}{G}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            KILL_INSTANT,
            face("Quick Slay", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            KILL_AB,
        ),
    ]
}

#[test]
fn spree_resolves_four_target_clauses_including_player_and_within_mode_second_targets() {
    let mut engine = bench(
        809,
        spree_cards(),
        [
            Seat::with(&[TINY_GOBLIN]).holding(&[SPREE_COMMAND]).at(20),
            Seat::with(&[TINY_GOBLIN, TWO_DROP_BEAR]).at(20),
        ],
    );
    let my_goblin = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let their_goblin = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == them() && o.card.unwrap().index.get() == TINY_GOBLIN
        })
        .unwrap();
    let their_bear = the(&engine, ZoneLocation::Battlefield, TWO_DROP_BEAR);
    let cmd = the(&engine, ZoneLocation::Hand(me()), SPREE_COMMAND);

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);

    engine
        .apply(me(), PlayerAction::CastSpell { card: cmd })
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    let all_three = options
        .iter()
        .position(|o| o.kind == CastModeKind::Modes(0b111))
        .expect("Modes(0b111) offered");
    engine
        .apply(me(), PlayerAction::ChooseMode(all_three))
        .unwrap();

    // Clause 0: Mode 0 creature target (their_goblin).
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![their_goblin],
                players: vec![],
            },
        )
        .unwrap();
    // Clause 1: Mode 1 AnyPlayer target (me()).
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    // Clause 2: Mode 2 first target (my_goblin).
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_goblin],
                players: vec![],
            },
        )
        .unwrap();
    // Clause 3: Mode 2 second target (their_bear).
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![their_bear],
                players: vec![],
            },
        )
        .unwrap();

    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        25,
        "Mode 1 player target gained 5 life"
    );
    assert_eq!(
        engine.state().object(their_goblin).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "Mode 0 dealt 2 lethal damage to their_goblin"
    );
    assert_eq!(
        engine.state().object(their_bear).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "Mode 2 pumped our 2/2 to 4/4 and fought their 3/3 bear"
    );
    assert_eq!(
        engine.state().object(my_goblin).unwrap().zone,
        crate::zone::Zone::Battlefield,
        "our 4/4 survived fighting their 3/3"
    );
}

#[test]
fn spree_partial_target_illegality_keeps_remaining_modes_and_plot_waives_only_base_cost() {
    // 1. Partial illegality (CR 608.2b): Mode 0's creature dies in response, Mode 1's player resolves.
    let mut engine = bench(
        810,
        spree_cards(),
        [
            Seat::with(&[])
                .holding(&[SPREE_COMMAND, KILL_INSTANT])
                .at(20),
            Seat::with(&[TWO_DROP_BEAR]).at(20),
        ],
    );
    let bear = the(&engine, ZoneLocation::Battlefield, TWO_DROP_BEAR);
    let cmd = the(&engine, ZoneLocation::Hand(me()), SPREE_COMMAND);
    let kill = the(&engine, ZoneLocation::Hand(me()), KILL_INSTANT);

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: cmd })
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    let two_modes = options
        .iter()
        .position(|o| o.kind == CastModeKind::Modes(0b011))
        .expect("Modes(0b011) offered");
    engine
        .apply(me(), PlayerAction::ChooseMode(two_modes))
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
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
    // Kill bear in response so Mode 0's target is illegal, while Mode 1's player target stays legal.
    engine
        .apply(me(), PlayerAction::CastSpell { card: kill })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        25,
        "Spree spell did not fizzle because Mode 1's player target remained legal"
    );

    // 2. Plot + Spree (CR 702.170d + CR 702.172a + CR 118.9d): base {2}{R} waived, mode +{1} paid.
    let plot_defs = vec![card(
        PLOTTED_SPREE,
        FaceDef {
            plot: Some(ManaCost::parse("{1}{R}")),
            ..face("Plotted Spree", "{2}{R}", TypeSet::SORCERY)
        },
        KeywordSet::SPREE,
        PLOT_SPREE_AB,
    )];
    let mut plot_game = bench(
        811,
        plot_defs,
        [
            Seat::with(&[forest()]).holding(&[PLOTTED_SPREE]).at(20),
            Seat::with(&[]).at(20),
        ],
    );
    advance_to_main(&mut plot_game, me());
    let spree_card = the(&plot_game, ZoneLocation::Hand(me()), PLOTTED_SPREE);
    add_pool(&mut plot_game, me(), ManaColor::Red, 2);
    plot_game
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: spree_card,
                ability_index: PLOT_CARD,
            },
        )
        .expect("plot from hand");

    advance_to_main(&mut plot_game, them());
    advance_to_main(&mut plot_game, me());
    // With 0 mana floating and 1 untapped Forest, plotted Spree is NOT free (`!castable`)
    // because every mode adds >= {1}, and IS `payable` because 1 land pays `{1}` while `{2}{R}` is waived!
    assert!(!offer(&plot_game).1.castable.contains(&spree_card));
    assert!(offer(&plot_game).1.payable.contains(&spree_card));

    // Add just {1} colorless (waiving {2}{R} base cost!) -> Mode 0 is now castable and charges {1}.
    add_pool(&mut plot_game, me(), ManaColor::Colorless, 1);
    assert!(offer(&plot_game).1.castable.contains(&spree_card));
    plot_game
        .apply(me(), PlayerAction::CastSpell { card: spree_card })
        .expect("cast plotted spree for {1}");
    settle_stack(&mut plot_game, me());
    assert_eq!(
        plot_game.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );
    assert_eq!(life(&plot_game, me()), 24);
}

static PLOT_SPREE_MODES: &[SpellMode] = &[
    SpellMode {
        effects: &[Effect::gain_life(4)],
        targets: None,
        second_targets: None,
        cost_override: None,
        additional_cost: Some(ManaCost::parse("{1}")),
    },
    SpellMode {
        effects: &[Effect::gain_life(6)],
        targets: None,
        second_targets: None,
        cost_override: None,
        additional_cost: Some(ManaCost::parse("{2}")),
    },
];

static PLOT_SPREE_AB: &[AbilityDef] = &[AbilityDef::ModalSpell {
    modes: PLOT_SPREE_MODES,
    choose: ModeCount::ONE_OR_MORE,
}];

#[test]
fn shares_creature_type_with_other_target_keeps_first_target_on_resolution() {
    use baylee_core::generated::subtypes::creature::{BEAR, WOLF};

    const PAIR_SPELL: u32 = 8816;
    const WOLF_A: u32 = 8817;
    const WOLF_B: u32 = 8818;
    const SOLO_BEAR: u32 = 8819;
    static PAIR_FILTER: Filter =
        Filter::And(&[Filter::CREATURE, Filter::SharesCreatureTypeWithOtherTarget]);
    static PAIR_AB: &[AbilityDef] = &[spell!(
        &[Effect::Fight {
            fighter: TargetSlot::First,
            foe: TargetSlot::Second,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&PAIR_FILTER))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&PAIR_FILTER))),
    )];

    let wolf_face = |name| FaceDef {
        subtypes: &[WOLF],
        ..creature_face(name, "{1}{G}", 2, 2)
    };
    let bear_face = FaceDef {
        subtypes: &[BEAR],
        ..creature_face("Solo Bear", "{1}{G}", 2, 2)
    };
    let defs = vec![
        card(
            PAIR_SPELL,
            face("Pack Clash", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            PAIR_AB,
        ),
        card(WOLF_A, wolf_face("Alpha Wolf"), KeywordSet::EMPTY, &[]),
        card(WOLF_B, wolf_face("Beta Wolf"), KeywordSet::EMPTY, &[]),
        card(SOLO_BEAR, bear_face, KeywordSet::EMPTY, &[]),
    ];

    let mut engine = bench(
        812,
        defs,
        [
            Seat::with(&[WOLF_A, SOLO_BEAR])
                .holding(&[PAIR_SPELL])
                .at(20),
            Seat::with(&[WOLF_B]).at(20),
        ],
    );
    let wolf_a = the(&engine, ZoneLocation::Battlefield, WOLF_A);
    let wolf_b = the(&engine, ZoneLocation::Battlefield, WOLF_B);
    let bear = the(&engine, ZoneLocation::Battlefield, SOLO_BEAR);
    let clash = the(&engine, ZoneLocation::Hand(me()), PAIR_SPELL);

    engine
        .apply(me(), PlayerAction::CastSpell { card: clash })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected first ChooseTargets");
    };
    assert!(options.contains(&wolf_a));
    assert!(
        !options.contains(&bear),
        "bear has no same-type partner on board"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![wolf_a],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![wolf_b],
                players: vec![],
            },
        )
        .unwrap();

    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(wolf_a).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "first target must stay legal (`first_ref == Slot::Second(0)`) so the fight resolves"
    );
    assert_eq!(
        engine.state().object(wolf_b).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "second target must fight and die too"
    );
}
