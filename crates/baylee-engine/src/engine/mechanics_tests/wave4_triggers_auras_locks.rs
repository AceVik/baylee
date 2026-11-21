//! Rule tests for Engine Wave 4:
//! 1. `Trigger::ChangesZoneAll` (`CR 603.2c`, `CR 603.10a`) — simultaneous
//!    multi-token creation, multi-card graveyard departure, simultaneous
//!    board-wipe deaths with `exclude_self`, and distinct action-run boundaries.
//! 2. `Trigger::Sacrificed` (`CR 701.21a`, `CR 715.4`, `CR 603.10a`) — cost
//!    sacrifice, effect sacrifice, self-sacrifice LKI, exile-replacement
//!    sacrifice, and Saga completion SBA sacrifice, contrasted against
//!    destruction, lethal damage, zero toughness, and the legend rule.
//! 3. `Enchant Player` / player-attached Auras (`CR 303.4a`, `CR 303.4c`,
//!    `CR 303.4f`, `CR 704.5m`, `CR 702.16c`, `CR 508.3b`) — `Rider::AttachedToPlayer`,
//!    `PlayerRel::EnchantedPlayer`, `Filter::ControlledByEnchantedPlayer`,
//!    `Trigger::PlayerAttacked`, untargeted entry ignoring hexproof, and
//!    protection state-based detachment.
//! 4. `Filter::Historic` (`CR 700.6`), `Filter::HasCounters`,
//!    `Filter::AnyCountersAtLeast`, `Condition::ObjectMatches` (with CR 608.2h
//!    LKI), `Condition::AmountCompare`, resolution-bound continuous player
//!    locks (`CR 601.3`), and `Modifier::MatchingSpellCastLimitPerTurn`.
#![allow(clippy::too_many_lines)] // Positive and negative rule checks sit side by side per mechanic.

use super::*;
use baylee_cards_dsl::{
    Amount, AmountCmp, Condition, CostPart, CounterKind, Duration, Filter, Modifier, PlayerRel,
    StepKind, TargetSpec, TokenDef, Trigger, ZoneRef, ZoneSel, chapter, spell, static_ability,
    triggered,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::types::SupertypeSet;

const TOKEN_BATCH_WATCHER: u32 = 99_900;
const DRAW_BATCH_WATCHER: u32 = 99_901;
const GY_LEAVE_WATCHER: u32 = 99_902;
const DEATH_BATCH_WATCHER: u32 = 99_903;
const BATCH_DRIVER: u32 = 99_904;
const VANILLA_2_2: u32 = 99_905;
const VANILLA_ARTIFACT: u32 = 99_906;

const SAC_WATCHER: u32 = 99_910;
const SELF_SAC_PROBE: u32 = 99_911;
const SAC_DRIVER: u32 = 99_912;
const FODDER_4_2: u32 = 99_913;
const EXILE_YOUR_GY: u32 = 99_914;
const ONE_CHAPTER_SAGA: u32 = 99_915;
const LEGEND_TWIN: u32 = 99_916;

const BLACK_CURSE: u32 = 99_920;
const OPPONENT_CURSE_IN_GY: u32 = 99_921;
const REANIMATE_AURA_DRIVER: u32 = 99_922;
const GRANT_HEXPROOF_ENCHANTMENT: u32 = 99_923;
const GRANT_BLACK_PROT_SPELL: u32 = 99_924;
const ATTACKER_A: u32 = 99_925;
const ATTACKER_B: u32 = 99_926;

const HISTORIC_SWEEPER: u32 = 99_930;
const PLAIN_ENCHANTMENT: u32 = 99_931;
const COUNTER_PROBE: u32 = 99_932;
const DESTROY_IF_GOBLIN_SPELL: u32 = 99_933;
const GOBLIN_2_2: u32 = 99_934;
const ELF_2_2: u32 = 99_935;
const COMPARE_SPELL: u32 = 99_936;
const CHANT_LOCK_SPELL: u32 = 99_937;
const DEAFENING_LOCK_ENCHANTMENT: u32 = 99_938;
const FREE_CANT_TRIP_INSTANT: u32 = 99_939;
const SECOND_FREE_INSTANT: u32 = 99_940;

static SOLDIER_TOKEN: TokenDef = TokenDef {
    name: "Soldier",
    types: TypeSet::CREATURE,
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};

static TOKEN_CREATURE_FILTER: Filter = Filter::And(&[Filter::IsToken, Filter::CREATURE]);
static SAGA_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[subtypes::enchantment::SAGA];
static AURA_CURSE_SUBTYPES: &[baylee_core::ids::SubtypeId] =
    &[subtypes::enchantment::AURA, subtypes::enchantment::CURSE];
static GOBLIN_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[subtypes::creature::GOBLIN];
static ELF_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[subtypes::creature::ELF];

fn cast_from_hand(engine: &mut Bench, seat: PlayerId, idx: u32) -> ObjectId {
    let found = objects(engine, ZoneLocation::Hand(seat), idx);
    assert!(!found.is_empty(), "expected card {idx} in {seat:?}'s hand");
    let id = found[0];
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(
        legal.castable.contains(&id),
        "expected {idx} ({id:?}) to be castable, got {:?}",
        legal.castable
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: id })
        .expect("cast succeeds");
    id
}

#[test]
fn changes_zone_all_batches_simultaneous_tokens_and_separates_distinct_runs() {
    static TOKEN_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::ChangesZoneAll {
            origin: None,
            destination: Some(ZoneRef::Battlefield),
            filter: &TOKEN_CREATURE_FILTER,
            exclude_self: true,
        },
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }]
    )];
    static DRAW_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::ChangesZoneAll {
            origin: Some(ZoneRef::Library),
            destination: Some(ZoneRef::Hand),
            filter: &Filter::Any,
            exclude_self: false,
        },
        &[Effect::LoseLife {
            amount: Amount::EventAmount,
            target: PlayerRel::Opponent,
        }]
    )];
    static DRIVER_ABILITIES: &[AbilityDef] = &[
        // 0: create 3 tokens simultaneously -> 1 trigger with EventAmount = 3.
        free(
            &[Effect::CreateTokenN {
                token: &SOLDIER_TOKEN,
                amount: Amount::Fixed(3),
            }],
            None,
        ),
        // 1: draw 3 cards in one draw instruction -> 1 trigger with EventAmount = 3.
        free(
            &[Effect::DrawCards {
                amount: Amount::Fixed(3),
            }],
            None,
        ),
        // 2: draw 1, then draw 1 -> 2 distinct action runs -> 2 triggers, 1 each.
        free(
            &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
            ],
            None,
        ),
    ];

    let cards = vec![
        card(
            TOKEN_BATCH_WATCHER,
            face("Token Batch Watcher", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            TOKEN_WATCHER_ABILITIES,
        ),
        card(
            DRAW_BATCH_WATCHER,
            face("Draw Batch Watcher", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DRAW_WATCHER_ABILITIES,
        ),
        card(
            BATCH_DRIVER,
            face("Batch Driver", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DRIVER_ABILITIES,
        ),
    ];

    let mut engine = bench(
        401,
        cards,
        [
            Seat::with(&[TOKEN_BATCH_WATCHER, DRAW_BATCH_WATCHER, BATCH_DRIVER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let token_watcher = the(&engine, ZoneLocation::Battlefield, TOKEN_BATCH_WATCHER);
    let draw_watcher = the(&engine, ZoneLocation::Battlefield, DRAW_BATCH_WATCHER);
    let driver = the(&engine, ZoneLocation::Battlefield, BATCH_DRIVER);

    activate(&mut engine, me(), driver, 0);
    settle(&mut engine, me());
    assert_eq!(
        triggered(&engine, token_watcher),
        1,
        "three simultaneous tokens fire ChangesZoneAll exactly once (CR 603.2c)"
    );
    assert_eq!(
        life(&engine, me()),
        23,
        "EventAmount binds to the matched batch count (3)"
    );

    let before_draws = triggered(&engine, draw_watcher);
    activate(&mut engine, me(), driver, 1);
    settle(&mut engine, me());
    assert_eq!(triggered(&engine, draw_watcher) - before_draws, 1);
    assert_eq!(life(&engine, them()), 17);

    let before_split = triggered(&engine, draw_watcher);
    activate(&mut engine, me(), driver, 2);
    settle(&mut engine, me());
    assert_eq!(
        triggered(&engine, draw_watcher) - before_split,
        2,
        "two separate DrawCards instructions break into two action runs"
    );
    assert_eq!(life(&engine, them()), 15);
}

#[test]
fn changes_zone_all_looks_back_on_graveyard_departures_and_simultaneous_deaths() {
    static GY_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::ChangesZoneAll {
            origin: Some(ZoneRef::Graveyard),
            destination: None,
            filter: &Filter::CREATURE,
            exclude_self: false,
        },
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }]
    )];
    static DEATH_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::ChangesZoneAll {
            origin: Some(ZoneRef::Battlefield),
            destination: Some(ZoneRef::Graveyard),
            filter: &Filter::CREATURE,
            exclude_self: true,
        },
        &[Effect::LoseLife {
            amount: Amount::EventAmount,
            target: PlayerRel::Opponent,
        }]
    )];
    static DRIVER_ABILITIES: &[AbilityDef] = &[
        // 0: destroy all creatures and artifacts -> puts 2 bears + death watcher + 1 artifact in graveyard.
        free(
            &[Effect::DestroyAll {
                filter: &Filter::ARTIFACT_OR_CREATURE,
                no_regen: false,
            }],
            None,
        ),
        // 1: exile graveyard -> 3 creature cards + 1 artifact leave graveyard simultaneously.
        free(
            &[Effect::ExileGraveyard {
                player: PlayerRel::You,
            }],
            None,
        ),
    ];

    let cards = vec![
        card(
            GY_LEAVE_WATCHER,
            face("Roots Watcher", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            GY_WATCHER_ABILITIES,
        ),
        card(
            DEATH_BATCH_WATCHER,
            creature_face("Death Batch Watcher", "{0}", 2, 2),
            KeywordSet::EMPTY,
            DEATH_WATCHER_ABILITIES,
        ),
        card(
            BATCH_DRIVER,
            face("Sweep Driver", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DRIVER_ABILITIES,
        ),
        card(
            VANILLA_2_2,
            creature_face("Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            VANILLA_ARTIFACT,
            face("Relic", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        402,
        cards,
        [
            Seat::with(&[
                GY_LEAVE_WATCHER,
                DEATH_BATCH_WATCHER,
                BATCH_DRIVER,
                VANILLA_2_2,
                VANILLA_2_2,
                VANILLA_ARTIFACT,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let gy_watcher = the(&engine, ZoneLocation::Battlefield, GY_LEAVE_WATCHER);
    let death_watcher = the(&engine, ZoneLocation::Battlefield, DEATH_BATCH_WATCHER);
    let driver = the(&engine, ZoneLocation::Battlefield, BATCH_DRIVER);

    // Simultaneous wrath kills DEATH_BATCH_WATCHER + 2 VANILLA_2_2 (+ 1 artifact).
    // With `exclude_self: true` and CR 603.10a look-back, DEATH_BATCH_WATCHER
    // sees the 2 other dying creatures in one batch!
    activate(&mut engine, me(), driver, 0);
    settle(&mut engine, me());
    assert_eq!(
        triggered(&engine, death_watcher),
        1,
        "dying observer looks back (CR 603.10a) and batches the 2 co-dying creatures once"
    );
    assert_eq!(life(&engine, them()), 18);

    // Now graveyard holds 3 creature cards + 1 artifact card. Exiling all of
    // them fires GY_LEAVE_WATCHER once with EventAmount == 3 (ignoring the artifact).
    activate(&mut engine, me(), driver, 1);
    settle(&mut engine, me());
    assert_eq!(triggered(&engine, gy_watcher), 1);
    assert_eq!(life(&engine, me()), 23);
}

#[test]
fn sacrificed_trigger_fires_on_cost_effect_saga_and_exile_replacement_never_destroy_or_sba() {
    static WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::Sacrificed {
            who: PlayerRel::You,
            filter: &Filter::Any,
        },
        &[
            Effect::GainLifeFor {
                who: PlayerRel::EventPlayer,
                amount: Amount::Fixed(1),
            },
            Effect::LoseLife {
                amount: Amount::EventPower,
                target: PlayerRel::Opponent,
            },
        ]
    )];
    static SELF_SAC_ABILITIES: &[AbilityDef] = &[
        triggered!(
            Trigger::Sacrificed {
                who: PlayerRel::You,
                filter: &Filter::This,
            },
            &[Effect::GainLife {
                amount: Amount::Fixed(5),
            }]
        ),
        paid(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::SacrificeSelf],
            },
            &[],
            None,
        ),
    ];
    static DRIVER_ABILITIES: &[AbilityDef] = &[
        // 0: cost sacrifice of a creature.
        paid(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::Sacrifice(&Filter::CREATURE)],
            },
            &[],
            None,
        ),
        // 1: destroy target creature (NOT a sacrifice).
        free(
            &[Effect::Destroy {
                target: TargetSpec::Object(&Filter::CREATURE),
                no_regen: false,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        // 2: deal 5 damage to target creature -> lethal damage SBA (NOT a sacrifice).
        free(
            &[Effect::DealDamage {
                amount: Amount::Fixed(5),
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        // 3: -0/-4 to target creature -> 0-toughness SBA (NOT a sacrifice).
        free(
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::NegXFixed(4),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];
    static EXILE_GY_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ExileInsteadOfYourGraveyard
    )];
    static SAGA_ABILITIES: &[AbilityDef] = &[chapter!(
        1,
        &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }]
    )];

    let saga_face = FaceDef {
        subtypes: SAGA_SUBTYPES,
        ..face("Brief Saga", "{0}", TypeSet::ENCHANTMENT)
    };
    let legend_face = FaceDef {
        supertypes: SupertypeSet::LEGENDARY,
        ..creature_face("Twin Hero", "{0}", 3, 3)
    };

    let cards = vec![
        card(
            SAC_WATCHER,
            face("Sacrifice Watcher", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            WATCHER_ABILITIES,
        ),
        card(
            SELF_SAC_PROBE,
            face("Bauble Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SELF_SAC_ABILITIES,
        ),
        card(
            SAC_DRIVER,
            face("Sacrifice Driver", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DRIVER_ABILITIES,
        ),
        card(
            FODDER_4_2,
            creature_face("Scrapper", "{0}", 4, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            EXILE_YOUR_GY,
            face("Void Clause", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            EXILE_GY_ABILITIES,
        ),
        card(
            ONE_CHAPTER_SAGA,
            saga_face,
            KeywordSet::EMPTY,
            SAGA_ABILITIES,
        ),
        card(LEGEND_TWIN, legend_face, KeywordSet::EMPTY, &[]),
    ];

    let mut engine = bench(
        403,
        cards,
        [
            Seat::with(&[
                SAC_WATCHER,
                SELF_SAC_PROBE,
                SAC_DRIVER,
                FODDER_4_2,
                FODDER_4_2,
                FODDER_4_2,
                FODDER_4_2,
            ])
            .holding(&[EXILE_YOUR_GY, ONE_CHAPTER_SAGA, LEGEND_TWIN, LEGEND_TWIN]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let watcher = the(&engine, ZoneLocation::Battlefield, SAC_WATCHER);
    let bauble = the(&engine, ZoneLocation::Battlefield, SELF_SAC_PROBE);
    let driver = the(&engine, ZoneLocation::Battlefield, SAC_DRIVER);

    // Negative 1: Destroy is NOT a sacrifice.
    let victim1 = objects(&engine, ZoneLocation::Battlefield, FODDER_4_2)[0];
    activate(&mut engine, me(), driver, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![victim1],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        triggered(&engine, watcher),
        0,
        "Destroy must not fire Sacrificed"
    );

    // Negative 2: Lethal damage SBA is NOT a sacrifice.
    let victim2 = objects(&engine, ZoneLocation::Battlefield, FODDER_4_2)[0];
    activate(&mut engine, me(), driver, 2);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![victim2],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        triggered(&engine, watcher),
        0,
        "lethal damage SBA must not fire Sacrificed"
    );

    // Negative 3: 0-toughness SBA is NOT a sacrifice.
    let victim3 = objects(&engine, ZoneLocation::Battlefield, FODDER_4_2)[0];
    activate(&mut engine, me(), driver, 3);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![victim3],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        triggered(&engine, watcher),
        0,
        "zero-toughness SBA must not fire Sacrificed"
    );

    // Negative 4: Legend rule SBA (CR 704.5j) is NOT a sacrifice.
    cast_from_hand(&mut engine, me(), LEGEND_TWIN);
    settle(&mut engine, me());
    cast_from_hand(&mut engine, me(), LEGEND_TWIN);
    while !matches!(engine.pending(), Pending::Priority { player, .. } if *player == me() && engine.state().zones.stack_is_empty())
    {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::LegendChoice { player, options }
            | Pending::ChooseCards {
                player, options, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected legend-rule prompt: {other:?}"),
        }
    }
    assert_eq!(
        triggered(&engine, watcher),
        0,
        "legend rule SBA is not a sacrifice (CR 704.5j)"
    );

    // Positive 1 + 3: Put Void Clause on board (your graveyard -> exile), then
    // sacrifice the remaining 4/2 Scrapper as a cost -> goes to Exile, STILL
    // triggers SAC_WATCHER with EventPlayer = me() and EventPower = 4!
    cast_from_hand(&mut engine, me(), EXILE_YOUR_GY);
    settle(&mut engine, me());
    let scrapper = the(&engine, ZoneLocation::Battlefield, FODDER_4_2);
    activate(&mut engine, me(), driver, 0);
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        assert_eq!(player, me());
        assert!(options.contains(&scrapper));
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![scrapper],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(scrapper).map(|o| o.zone),
        Some(Zone::Exile),
        "replacement sent the sacrificed creature to exile"
    );
    assert_eq!(
        triggered(&engine, watcher),
        1,
        "sacrifice still emits Sacrificed when destination is replaced (CR 701.21a)"
    );
    assert_eq!(life(&engine, me()), 21);
    assert_eq!(
        life(&engine, them()),
        16,
        "EventPower captured pre-sacrifice battlefield power (4)"
    );

    // Positive 2: Bauble sacrificing itself fires BOTH its own self-Sacrificed
    // look-back trigger (CR 603.10a) AND SAC_WATCHER!
    activate(&mut engine, me(), bauble, 1);
    settle(&mut engine, me());
    assert_eq!(triggered(&engine, bauble), 1);
    assert_eq!(triggered(&engine, watcher), 2);
    assert_eq!(life(&engine, me()), 27);

    // Positive 4: Completed Saga SBA is a sacrifice (CR 715.4)!
    let saga = cast_from_hand(&mut engine, me(), ONE_CHAPTER_SAGA);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(saga).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert_eq!(
        triggered(&engine, watcher),
        3,
        "completed Saga SBA sacrifices the Saga (CR 715.4)"
    );
}

#[test]
fn curse_attaches_to_player_drives_enchanted_player_triggers_and_statics_and_falls_off_to_protection()
 {
    static CURSE_CREATURES: Filter =
        Filter::And(&[Filter::CREATURE, Filter::ControlledByEnchantedPlayer]);
    static BLACK_FILTER: Filter = Filter::HasColor(ColorSet::of(Color::Black));
    static CURSE_ABILITIES: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::AnyPlayer,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
        ),
        static_ability!(CURSE_CREATURES, Modifier::ModifyPT(-1, -1)),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EnchantedPlayer,
            },
            &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::EnchantedPlayer,
            }]
        ),
        triggered!(
            Trigger::PlayerAttacked(PlayerRel::EnchantedPlayer),
            &[Effect::GainLifeFor {
                who: PlayerRel::EventPlayer,
                amount: Amount::EventAmount,
            }]
        ),
    ];
    static PROT_BLACK_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::CreateContinuousEffect {
            layer: baylee_cards_dsl::Layer::Text,
            filter: &Filter::Any,
            modifier: Modifier::PlayerProtectionFrom {
                who: PlayerRel::Chosen,
                from: &BLACK_FILTER,
            },
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];

    let curse_face = FaceDef {
        color_indicator: ColorSet::of(Color::Black),
        subtypes: AURA_CURSE_SUBTYPES,
        ..face("Curse of Dread", "{0}", TypeSet::ENCHANTMENT)
    };

    let cards = vec![
        card(BLACK_CURSE, curse_face, KeywordSet::EMPTY, CURSE_ABILITIES),
        card(
            GRANT_BLACK_PROT_SPELL,
            face("Shield from Darkness", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            PROT_BLACK_ABILITIES,
        ),
        card(
            ATTACKER_A,
            creature_face("Raider A", "{0}", 2, 2),
            KeywordSet::HASTE,
            &[],
        ),
        card(
            ATTACKER_B,
            creature_face("Raider B", "{0}", 2, 2),
            KeywordSet::HASTE,
            &[],
        ),
    ];

    let mut engine = bench(
        404,
        cards,
        [
            Seat::with(&[ATTACKER_A, ATTACKER_B]).holding(&[BLACK_CURSE, GRANT_BLACK_PROT_SPELL]),
            Seat::with(&[ATTACKER_A]),
        ],
    );
    to_main(&mut engine, me());
    let my_raider = objects(&engine, ZoneLocation::Battlefield, ATTACKER_A)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let their_raider = objects(&engine, ZoneLocation::Battlefield, ATTACKER_A)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == them())
        .unwrap();
    let my_raider_b = the(&engine, ZoneLocation::Battlefield, ATTACKER_B);

    // Cast Curse targeting `them`.
    let curse = cast_from_hand(&mut engine, me(), BLACK_CURSE);
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        engine
            .state()
            .object(curse)
            .and_then(crate::object::GameObject::attached_player),
        Some(them()),
        "Curse enters attached to targeted player (CR 303.4b)"
    );
    assert_eq!(
        engine
            .state()
            .object(my_raider)
            .unwrap()
            .characteristics()
            .power,
        Some(2),
        "controller's own creature is unaffected"
    );
    assert_eq!(
        engine
            .state()
            .object(their_raider)
            .unwrap()
            .characteristics()
            .power,
        Some(1),
        "enchanted player's creature gets -1/-1"
    );

    // Declare BOTH Raider A and Raider B attacking `them` -> CR 508.3b fires
    // `PlayerAttacked(EnchantedPlayer)` ONCE with EventAmount = 2, EventPlayer = me()!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (my_raider, baylee_core::ids::Defender::Player(them())),
                    (my_raider_b, baylee_core::ids::Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(
        life(&engine, me()),
        22,
        "PlayerAttacked fired once with EventAmount == 2 for the attacking player"
    );

    // Walk to `them`'s main phase -> `them`'s upkeep triggered the Curse's -2 life!
    let life_after_combat = life(&engine, them());
    to_main(&mut engine, them());
    assert_eq!(
        life(&engine, them()),
        life_after_combat - 2,
        "enchanted player's upkeep triggered Curse's -2 life"
    );

    // Walk back to `me()`'s turn -> `me()`'s upkeep does NOT trigger the Curse!
    let their_life_before_my_turn = life(&engine, them());
    to_main(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        their_life_before_my_turn,
        "controller's upkeep must not trigger EnchantedPlayer step trigger"
    );

    // Grant `them` protection from black -> SBA puts BLACK_CURSE into graveyard (CR 704.5m, CR 702.16c)!
    cast_from_hand(&mut engine, me(), GRANT_BLACK_PROT_SPELL);
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(curse).map(|o| o.zone),
        Some(Zone::Graveyard),
        "protection from black unattaches and bins the black Curse via SBA (CR 704.5m, CR 702.16c)"
    );
}

#[test]
fn untargeted_player_aura_entry_chooses_player_ignoring_hexproof_per_cr_303_4f() {
    static OPP_CURSE_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::AnyOpponent,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )];
    static HEXPROOF_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::PlayerHexproof {
            who: PlayerRel::You,
        }
    )];
    static REANIMATE_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::YourGraveyardToBattlefield {
            filter: &Filter::ENCHANTMENT,
            tapped: false,
        }],
        None,
    )];
    static DISCARD_SELF_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::DiscardForPlayers {
            who: PlayerRel::You,
            count: 1,
        }],
        None,
    )];

    let opp_curse_face = FaceDef {
        subtypes: AURA_CURSE_SUBTYPES,
        ..face("Curse of Silence", "{0}", TypeSet::ENCHANTMENT)
    };

    let cards = vec![
        card(
            OPPONENT_CURSE_IN_GY,
            opp_curse_face,
            KeywordSet::EMPTY,
            OPP_CURSE_ABILITIES,
        ),
        card(
            GRANT_HEXPROOF_ENCHANTMENT,
            face("Aegis", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            HEXPROOF_ABILITIES,
        ),
        card(
            REANIMATE_AURA_DRIVER,
            face("Replenisher", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            REANIMATE_ABILITIES,
        ),
        card(
            SAC_DRIVER,
            face("Mill Self", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DISCARD_SELF_ABILITIES,
        ),
    ];

    let mut engine = bench(
        405,
        cards,
        [
            Seat::with(&[REANIMATE_AURA_DRIVER, SAC_DRIVER]).holding(&[OPPONENT_CURSE_IN_GY]),
            Seat::with(&[GRANT_HEXPROOF_ENCHANTMENT]),
        ],
    );
    to_main(&mut engine, me());
    let hand_curse = the(&engine, ZoneLocation::Hand(me()), OPPONENT_CURSE_IN_GY);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&hand_curse),
        "hexproof opponent cannot be targeted by an Aura spell"
    );

    // Discard the Curse to graveyard, then return it onto the battlefield untargeted!
    let discarder = the(&engine, ZoneLocation::Battlefield, SAC_DRIVER);
    activate(&mut engine, me(), discarder, 0);
    while !matches!(engine.pending(), Pending::Priority { player, .. } if *player == me() && engine.state().zones.stack_is_empty())
    {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                assert_eq!(player, me());
                assert!(options.contains(&hand_curse));
                engine
                    .apply(
                        me(),
                        PlayerAction::ChooseObjects {
                            objects: vec![hand_curse],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected discard prompt: {other:?}"),
        }
    }
    assert_eq!(
        engine.state().object(hand_curse).map(|o| o.zone),
        Some(Zone::Graveyard)
    );

    let reanimator = the(&engine, ZoneLocation::Battlefield, REANIMATE_AURA_DRIVER);
    activate(&mut engine, me(), reanimator, 0);
    while !matches!(engine.pending(), Pending::Priority { player, .. } if *player == me() && engine.state().zones.stack_is_empty())
    {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, me());
                assert_eq!(options, vec![them()]);
                engine
                    .apply(me(), PlayerAction::ChoosePlayer(them()))
                    .unwrap();
            }
            other => panic!("unexpected entry prompt: {other:?}"),
        }
    }
    assert_eq!(
        engine.state().object(hand_curse).map(|o| o.zone),
        Some(Zone::Battlefield),
        "untargeted Aura entry attaches to hexproof opponent per CR 303.4f"
    );
    assert_eq!(
        engine
            .state()
            .object(hand_curse)
            .and_then(crate::object::GameObject::attached_player),
        Some(them())
    );
}

#[test]
fn historic_and_counter_filters_and_if_condition_object_and_amount_comparisons() {
    static ANY_COUNTERED: Filter = Filter::HasCounters(None);
    static P1P1_COUNTERED: Filter = Filter::HasCounters(Some(CounterKind::P1P1));
    static THREE_COUNTERS: Filter = Filter::AnyCountersAtLeast(3);
    static IS_GOBLIN: Filter = Filter::HasSubtype(subtypes::creature::GOBLIN);

    static HISTORIC_SWEEPER_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::GainLife {
            amount: Amount::CountOf {
                filter: &Filter::Historic,
                zone: ZoneSel::Battlefield,
            },
        }],
        None,
    )];
    static COUNTER_PROBE_ABILITIES: &[AbilityDef] = &[
        // 0: put 1 charge counter on target permanent.
        free(
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(1),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        ),
        // 1: put 2 +1/+1 counters on target permanent.
        free(
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        ),
        // 2: count HasCounters(None), HasCounters(Some(P1P1)), AnyCountersAtLeast(3).
        free(
            &[
                Effect::GainLife {
                    amount: Amount::CountOf {
                        filter: &ANY_COUNTERED,
                        zone: ZoneSel::Battlefield,
                    },
                },
                Effect::LoseLife {
                    amount: Amount::CountOf {
                        filter: &P1P1_COUNTERED,
                        zone: ZoneSel::Battlefield,
                    },
                    target: PlayerRel::Opponent,
                },
                Effect::IfCondition {
                    condition: Condition::AmountCompare {
                        left: &Amount::CountOf {
                            filter: &THREE_COUNTERS,
                            zone: ZoneSel::Battlefield,
                        },
                        op: AmountCmp::AtLeast,
                        right: &Amount::Fixed(1),
                    },
                    then: &[Effect::GainLife {
                        amount: Amount::Fixed(10),
                    }],
                    otherwise: &[],
                },
            ],
            None,
        ),
    ];
    static DESTROY_GOBLIN_BONUS: &[AbilityDef] = &[spell!(
        &[
            Effect::Destroy {
                target: TargetSpec::Object(&Filter::CREATURE),
                no_regen: false,
            },
            Effect::IfCondition {
                condition: Condition::ObjectMatches {
                    what: &TargetSpec::Object(&Filter::CREATURE),
                    filter: &IS_GOBLIN,
                },
                then: &[Effect::GainLife {
                    amount: Amount::Fixed(4),
                }],
                otherwise: &[Effect::LoseLife {
                    amount: Amount::Fixed(1),
                    target: PlayerRel::You,
                }],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static COMPARE_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::IfCondition {
        condition: Condition::AmountCompare {
            left: &Amount::LifeTotal(PlayerRel::You),
            op: AmountCmp::GreaterThan,
            right: &Amount::LifeTotal(PlayerRel::Opponent),
        },
        then: &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        otherwise: &[],
    }])];
    static SAGA_STAY_ABILITIES: &[AbilityDef] = &[
        chapter!(
            1,
            &[Effect::GainLife {
                amount: Amount::Fixed(0),
            }]
        ),
        chapter!(
            2,
            &[Effect::GainLife {
                amount: Amount::Fixed(0),
            }]
        ),
    ];

    let saga_face = FaceDef {
        subtypes: SAGA_SUBTYPES,
        ..face("Staying Saga", "{0}", TypeSet::ENCHANTMENT)
    };
    let legend_face = FaceDef {
        supertypes: SupertypeSet::LEGENDARY,
        ..creature_face("Historic Hero", "{0}", 2, 2)
    };
    let goblin_face = FaceDef {
        subtypes: GOBLIN_SUBTYPES,
        ..creature_face("Test Goblin", "{0}", 2, 2)
    };
    let elf_face = FaceDef {
        subtypes: ELF_SUBTYPES,
        ..creature_face("Test Elf", "{0}", 2, 2)
    };

    let cards = vec![
        card(
            HISTORIC_SWEEPER,
            face("Historic Census", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            HISTORIC_SWEEPER_ABILITIES,
        ),
        card(
            PLAIN_ENCHANTMENT,
            face("Plain Aura-less", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            COUNTER_PROBE,
            face("Counter Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            COUNTER_PROBE_ABILITIES,
        ),
        card(
            ONE_CHAPTER_SAGA,
            saga_face,
            KeywordSet::EMPTY,
            SAGA_STAY_ABILITIES,
        ),
        card(LEGEND_TWIN, legend_face, KeywordSet::EMPTY, &[]),
        card(GOBLIN_2_2, goblin_face, KeywordSet::EMPTY, &[]),
        card(ELF_2_2, elf_face, KeywordSet::EMPTY, &[]),
        card(
            DESTROY_IF_GOBLIN_SPELL,
            face("Goblin Check", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            DESTROY_GOBLIN_BONUS,
        ),
        card(
            COMPARE_SPELL,
            face("Life Check", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            COMPARE_ABILITIES,
        ),
    ];

    let mut engine = bench(
        406,
        cards,
        [
            Seat::with(&[
                HISTORIC_SWEEPER,
                PLAIN_ENCHANTMENT,
                COUNTER_PROBE,
                LEGEND_TWIN,
                GOBLIN_2_2,
                ELF_2_2,
            ])
            .holding(&[
                ONE_CHAPTER_SAGA,
                DESTROY_IF_GOBLIN_SPELL,
                DESTROY_IF_GOBLIN_SPELL,
                COMPARE_SPELL,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    // Cast Saga so battlefield has: COUNTER_PROBE (Artifact), LEGEND_TWIN (Legendary),
    // ONE_CHAPTER_SAGA (Saga, with 1 Lore counter!) -> 3 historic permanents;
    // HISTORIC_SWEEPER, PLAIN_ENCHANTMENT, GOBLIN_2_2, ELF_2_2 are non-historic!
    cast_from_hand(&mut engine, me(), ONE_CHAPTER_SAGA);
    settle(&mut engine, me());

    let census = the(&engine, ZoneLocation::Battlefield, HISTORIC_SWEEPER);
    activate(&mut engine, me(), census, 0);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        23,
        "Filter::Historic matches Artifact + Legendary + Saga = 3 permanents (CR 700.6)"
    );

    // Put 1 charge counter + 2 +1/+1 counters on ELF_2_2 (= 3 counters total).
    // Note ONE_CHAPTER_SAGA already has 1 Lore counter!
    let probe = the(&engine, ZoneLocation::Battlefield, COUNTER_PROBE);
    let elf = the(&engine, ZoneLocation::Battlefield, ELF_2_2);
    activate(&mut engine, me(), probe, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    activate(&mut engine, me(), probe, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // HasCounters(None) matches Saga + Elf (2 -> +2 life = 25);
    // HasCounters(Some(P1P1)) matches Elf only (1 -> them loses 1 = 19);
    // AnyCountersAtLeast(3) matches Elf (1 >= 1 -> +10 life = 35)!
    activate(&mut engine, me(), probe, 2);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 35);
    assert_eq!(life(&engine, them()), 19);

    // Destroy Goblin -> Condition::ObjectMatches checks LKI after Destroy -> +4 life = 39!
    let goblin = the(&engine, ZoneLocation::Battlefield, GOBLIN_2_2);
    cast_from_hand(&mut engine, me(), DESTROY_IF_GOBLIN_SPELL);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![goblin],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        39,
        "Condition::ObjectMatches reads destroyed target's LKI subtype (CR 608.2h)"
    );

    // Destroy Elf -> Condition::ObjectMatches false -> otherwise branch (-1 life = 38)!
    cast_from_hand(&mut engine, me(), DESTROY_IF_GOBLIN_SPELL);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 38);

    // AmountCompare: my life (38) > their life (19) -> draws 1 card!
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    cast_from_hand(&mut engine, me(), COMPARE_SPELL);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before, // spent COMPARE_SPELL (-1) and drew +1!
        "Condition::AmountCompare Gt ran then-branch"
    );
}

#[test]
fn resolution_bound_player_locks_and_matching_spell_cast_limit_per_turn() {
    static CHANT_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::CreateContinuousEffect {
            layer: baylee_cards_dsl::Layer::Text,
            filter: &Filter::Any,
            modifier: Modifier::PlayersCantCast {
                who: PlayerRel::Chosen,
                filter: &Filter::Any,
            },
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];
    static DEAFENING_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::MatchingSpellCastLimitPerTurn {
            who: PlayerRel::EachPlayer,
            filter: &Filter::NONCREATURE,
            limit: 1,
        }
    )];
    static CANT_TRIP_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let cards = vec![
        card(
            CHANT_LOCK_SPELL,
            face("Silent Chant", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            CHANT_ABILITIES,
        ),
        card(
            DEAFENING_LOCK_ENCHANTMENT,
            face("Quiet Rule", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DEAFENING_ABILITIES,
        ),
        card(
            FREE_CANT_TRIP_INSTANT,
            face("Quick Spark", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            CANT_TRIP_ABILITIES,
        ),
        card(
            SECOND_FREE_INSTANT,
            face("Second Spark", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            CANT_TRIP_ABILITIES,
        ),
        card(
            VANILLA_2_2,
            creature_face("Quiet Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        407,
        cards,
        [
            Seat::with(&[DEAFENING_LOCK_ENCHANTMENT]).holding(&[
                VANILLA_2_2,
                CHANT_LOCK_SPELL,
                SECOND_FREE_INSTANT,
                VANILLA_2_2,
            ]),
            Seat::default().holding(&[FREE_CANT_TRIP_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    // Under Deafening Silence (`limit: 1` noncreature spell per turn):
    // 1) Casting a creature does NOT spend the noncreature allowance.
    cast_from_hand(&mut engine, me(), VANILLA_2_2);
    settle(&mut engine, me());

    // 2) Casting CHANT_LOCK_SPELL (1st noncreature spell) targeting `them` is legal!
    cast_from_hand(&mut engine, me(), CHANT_LOCK_SPELL);
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .unwrap();
    settle(&mut engine, me());

    // 3) Now `me()` has cast 1 noncreature spell this turn -> SECOND_FREE_INSTANT
    //    is blocked, while the second VANILLA_2_2 creature is STILL castable!
    let second_instant = the(&engine, ZoneLocation::Hand(me()), SECOND_FREE_INSTANT);
    let second_bear = the(&engine, ZoneLocation::Hand(me()), VANILLA_2_2);
    let (_, my_legal) = offer(&engine);
    assert!(
        !my_legal.castable.contains(&second_instant),
        "MatchingSpellCastLimitPerTurn blocks a second noncreature spell"
    );
    assert!(
        my_legal.castable.contains(&second_bear),
        "MatchingSpellCastLimitPerTurn still allows creature spells"
    );

    // 4) Pass priority to `them` -> CHANT_LOCK_SPELL bound `PlayerRel::Chosen`
    //    to `them`, so `them` cannot cast FREE_CANT_TRIP_INSTANT this turn!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let their_instant = the(&engine, ZoneLocation::Hand(them()), FREE_CANT_TRIP_INSTANT);
    let (prio, their_legal) = offer(&engine);
    assert_eq!(prio, them());
    assert!(
        !their_legal.castable.contains(&their_instant),
        "resolution-bound PlayersCantCast(Chosen) locks the targeted player for the turn"
    );
}
