//! Beat 18 / Engine Wave 11c deterministic rule tests:
//! 1. `Take the Initiative` (`CR 725.1–725.5`) & `Undercity` (`CR 309.4c`, `CR 701.49d`)
//!    across all 9 rooms, re-taking while holding, upkeep advance, once-per-controller
//!    combat-damage steal batching, ordinary dungeon interaction, and elimination pass-on.
//! 2. `Modifier::BecomeTypes` (0 and 2+ subtypes), `Effect::SetPTByCountFilter`,
//!    `Modifier::{SetPTSeparateToCount, SetPowerToCount, SetToughnessToCount}`, and
//!    `PtCount::{CardsInHand, InGraveyards, DistinctNamesAmong, Domain, LifeTotal,
//!    Twice, GreatestManaValueAmong, PartySize}` (`CR 700.8a–c`).
//! 3. Slice A3 CR bug fixes:
//!    - `Walking Desecration` (`EffectFilter::DslWithSubtype` dynamic `CR 611.2c` set);
//!    - `Haunted Angel` (`Effect::ExileSource` `CR 400.7e` `moving_source` check);
//!    - `Riptide Entrancer` (`PlayerMayPayCostThen` `Sacrifice(&Filter::This)` `CR 400.7` check).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::turn::Step;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    Condition, CostPart, CounterKind, Duration, Filter, Layer, Modifier, PlayerRel, PtCount,
    TargetReq, TargetSpec, Trigger, TriggerZone, spell, static_ability, triggered,
};
use baylee_core::dungeon::Dungeon;
use baylee_core::generated::subtypes::{artifact, creature, land};
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;
use baylee_core::types::SupertypeSet;

const C0: u32 = 99_810;
const C1: u32 = 99_811;
const C2: u32 = 99_812;
const C3: u32 = 99_813;
const C4: u32 = 99_814;
const C5: u32 = 99_815;
const C6: u32 = 99_816;
const C7: u32 = 99_817;
const C8: u32 = 99_818;

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

#[test]
fn take_initiative_enters_undercity_retaking_advances_and_throne_completes() {
    static PROBE_ABILITIES: &[AbilityDef] = &[
        free(&[Effect::TakeInitiative(PlayerRel::You)], None),
        AbilityDef::ActivatedConditional {
            cost: Cost::FREE,
            effects: &[Effect::gain_life(1)],
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: Condition::HasInitiative(PlayerRel::You),
            limit: ActivationLimit::Unlimited,
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
        AbilityDef::ActivatedConditional {
            cost: Cost::FREE,
            effects: &[Effect::gain_life(1)],
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: Condition::CompletedADungeon(PlayerRel::You),
            limit: ActivationLimit::Unlimited,
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
    ];

    let probe = card(
        C0,
        face("Initiative Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE_ABILITIES,
    );
    let bear = card(
        C1,
        creature_face("Forge Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let zero_hydra = card(
        C2,
        creature_face("Zero Hydra", "{2}{G}", 0, 0),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1801,
        vec![probe, bear, zero_hydra],
        [Seat::with(&[C0, C1]).holding(&[C2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);
    let bear_id = the(&engine, ZoneLocation::Battlefield, C1);
    let hydra_hand_id = the(&engine, ZoneLocation::Hand(me()), C2);

    // Initially neither HasInitiative nor CompletedADungeon holds.
    assert_eq!(engine.state().initiative, None);
    let (_, legal) = offer(&engine);
    assert!(!legal.abilities.contains(&(probe_id, 1)));
    assert!(!legal.abilities.contains(&(probe_id, 2)));

    // 1. Take the initiative -> enters Undercity room 0 ("Secret Entrance"),
    //    searching library for a basic land card into hand.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let found_forest = options[0];
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![found_forest],
            },
        )
        .expect("pick basic land for Secret Entrance");
    settle(&mut engine, me());

    assert_eq!(engine.state().initiative, Some(me()));
    assert_eq!(
        engine.state().players[0].dungeon.map(|(d, _)| d),
        Some(Dungeon::Undercity)
    );
    assert_eq!(engine.state().players[0].dungeon_room, 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1
    );
    let (_, legal) = offer(&engine);
    assert!(
        legal.abilities.contains(&(probe_id, 1)),
        "Condition::HasInitiative(You) holds once initiative is taken"
    );
    assert!(!legal.abilities.contains(&(probe_id, 2)));

    // 2. Re-taking the initiative while ALREADY holding it (CR 725.3) still
    //    triggers venture into Undercity -> choose room 1 ("Forge") on bear_id.
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let forge_idx = options
        .iter()
        .find(|o| {
            matches!(
                o.kind,
                CastModeKind::DungeonRoom {
                    dungeon: Dungeon::Undercity,
                    room: 1
                }
            )
        })
        .expect("Forge offered")
        .index as usize;
    engine
        .apply(me(), PlayerAction::ChooseMode(forge_idx))
        .expect("choose Forge");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_id],
            },
        )
        .expect("target bear for Forge");
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );

    // 3. Advance to room 3 ("Trap!"): target opponent loses 5 life.
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let trap_idx = options
        .iter()
        .find(|o| {
            matches!(
                o.kind,
                CastModeKind::DungeonRoom {
                    dungeon: Dungeon::Undercity,
                    room: 3
                }
            )
        })
        .expect("Trap! offered")
        .index as usize;
    engine
        .apply(me(), PlayerAction::ChooseMode(trap_idx))
        .expect("choose Trap!");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent for Trap!");
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 15);

    // 4. Advance to room 6 ("Archives"): single outgoing edge, draws 1 card.
    let hand_before_archives = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), probe_id, 0);
    settle(&mut engine, me());
    assert_eq!(engine.state().players[0].dungeon_room, 7);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_archives + 1
    );

    // 5. Put Zero Hydra on top of our library, then advance to room 8
    //    ("Throne of the Dead Three"): it enters WITH three +1/+1 counters
    //    (surviving SBA as a 3/3) and gains hexproof until our next turn, then
    //    Undercity completes!
    let hydra_id = engine
        .state
        .move_object(
            hydra_hand_id,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![hydra_id],
            },
        )
        .expect("choose Zero Hydra from Throne of the Dead Three");
    settle(&mut engine, me());

    let hydra_obj = engine
        .state()
        .object(hydra_id)
        .expect("Zero Hydra on board");
    assert_eq!(hydra_obj.zone, Zone::Battlefield);
    assert_eq!(hydra_obj.counters.get(CounterKind::P1P1), 3);
    assert_eq!(hydra_obj.characteristics().power, Some(3));
    assert_eq!(hydra_obj.characteristics().toughness, Some(3));
    assert!(
        hydra_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::HEXPROOF),
        "Throne of the Dead Three grants hexproof until your next turn"
    );
    assert_eq!(engine.state().players[0].dungeon, None);
    assert_eq!(engine.state().players[0].dungeons_completed, 1);
    let (_, legal) = offer(&engine);
    assert!(
        legal.abilities.contains(&(probe_id, 2)),
        "Condition::CompletedADungeon(You) holds after completing Undercity"
    );
}

#[test]
fn undercity_alternate_rooms_lost_well_arena_stash_catacombs() {
    static PROBE: &[AbilityDef] = &[free(&[Effect::VentureIntoUndercity], None)];

    let probe = card(
        C0,
        face("Undercity Compass", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE,
    );
    let foe = card(
        C1,
        creature_face("Arena Foe", "{1}{R}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1802,
        vec![probe, foe],
        [Seat::with(&[C0]), Seat::with(&[C1])],
    );
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);
    let foe_id = the(&engine, ZoneLocation::Battlefield, C1);

    // Room 0: Secret Entrance.
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let objects = match engine.pending().clone() {
        Pending::ChooseCards { options, .. } => vec![options[0]],
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects })
        .unwrap();
    settle(&mut engine, me());

    // Room 2: Lost Well ("Scry 2.").
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let well_idx = match engine.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| {
                    matches!(
                        o.kind,
                        CastModeKind::DungeonRoom {
                            dungeon: Dungeon::Undercity,
                            room: 2
                        }
                    )
                })
                .unwrap()
                .index as usize
        }
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(well_idx))
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            me(),
            PlayerAction::Arrange {
                piles: vec![cards, vec![]],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Room 4: Arena ("Goad target creature.").
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let arena_idx = match engine.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| {
                    matches!(
                        o.kind,
                        CastModeKind::DungeonRoom {
                            dungeon: Dungeon::Undercity,
                            room: 4
                        }
                    )
                })
                .unwrap()
                .index as usize
        }
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(arena_idx))
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![foe_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert!(
        !crate::combat::goad::goaders_of(engine.state(), foe_id).is_empty(),
        "Arena goads the targeted creature"
    );

    // Room 7: Catacombs ("Create a 4/1 black Skeleton creature token with menace.").
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let catacombs_idx = match engine.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| {
                    matches!(
                        o.kind,
                        CastModeKind::DungeonRoom {
                            dungeon: Dungeon::Undercity,
                            room: 7
                        }
                    )
                })
                .unwrap()
                .index as usize
        }
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(catacombs_idx))
        .unwrap();
    settle(&mut engine, me());

    let skeleton = engine
        .state()
        .battlefield_seen()
        .filter_map(|id| engine.state().object(id))
        .find(|o| !o.is_card() && o.characteristics().subtypes.contains(creature::SKELETON))
        .expect("Catacombs creates a Skeleton token");
    assert_eq!(skeleton.characteristics().power, Some(4));
    assert_eq!(skeleton.characteristics().toughness, Some(1));
    assert!(
        skeleton
            .characteristics()
            .keywords
            .contains(KeywordSet::MENACE)
    );

    // Complete Throne of the Dead Three (room 8; library is all Forests so 0 creature
    // candidates, completing immediately), then re-enter Undercity -> Secret Entrance
    // -> Lost Well -> Room 5 ("Stash") to verify Treasure creation (all 9/9 rooms!).
    activate(&mut engine, me(), probe_id, 0);
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(engine.state().players[0].dungeons_completed, 1);

    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let objects = match engine.pending().clone() {
        Pending::ChooseCards { options, .. } => vec![options[0]],
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects })
        .unwrap();
    settle(&mut engine, me());

    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let well_idx = match engine.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| {
                    matches!(
                        o.kind,
                        CastModeKind::DungeonRoom {
                            dungeon: Dungeon::Undercity,
                            room: 2
                        }
                    )
                })
                .unwrap()
                .index as usize
        }
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(well_idx))
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            me(),
            PlayerAction::Arrange {
                piles: vec![cards, vec![]],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let stash_idx = match engine.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| {
                    matches!(
                        o.kind,
                        CastModeKind::DungeonRoom {
                            dungeon: Dungeon::Undercity,
                            room: 5
                        }
                    )
                })
                .unwrap()
                .index as usize
        }
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(stash_idx))
        .unwrap();
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .battlefield_seen()
            .filter_map(|id| engine.state().object(id))
            .any(|o| !o.is_card() && o.characteristics().subtypes.contains(artifact::TREASURE)),
        "Stash creates a Treasure token"
    );
}

#[test]
fn combat_damage_steals_initiative_once_per_controller_and_upkeep_advances() {
    static PROBE: &[AbilityDef] = &[
        free(&[Effect::VentureIntoTheDungeon], None),
        free(&[Effect::VentureIntoUndercity], None),
    ];

    let probe = card(
        C0,
        face("Venture Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE,
    );
    let attacker = card(
        C1,
        creature_face("Scout", "{G}", 1, 1),
        KeywordSet::HASTE,
        &[],
    );

    let mut engine = bench(
        1803,
        vec![probe, attacker],
        [
            Seat::with(&[C0, C1, C1, C1]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);

    // Ordinary VentureIntoTheDungeon offers only the 3 normal dungeons, NEVER Undercity (CR 309.4c).
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 3);
    assert!(
        !options
            .iter()
            .any(|o| matches!(o.kind, CastModeKind::Dungeon(Dungeon::Undercity))),
        "Undercity cannot be entered through ordinary VentureIntoTheDungeon"
    );
    let lost_mine_idx = options
        .iter()
        .find(|o| matches!(o.kind, CastModeKind::Dungeon(Dungeon::LostMine)))
        .unwrap()
        .index as usize;
    engine
        .apply(me(), PlayerAction::ChooseMode(lost_mine_idx))
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            me(),
            PlayerAction::Arrange {
                piles: vec![cards, vec![]],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().players[0].dungeon.map(|(d, _)| d),
        Some(Dungeon::LostMine)
    );
    assert_eq!(engine.state().players[0].dungeon_room, 1);

    // Give seat 1 the initiative directly on the board, then attack seat 1 with
    // ALL THREE 1/1 creatures at once. Seat 0 must take the initiative ONCE
    // (advancing Lost Mine ONCE from room 1 to room 2, NOT three times!).
    engine.state.initiative = Some(them());
    let scouts = objects(&engine, ZoneLocation::Battlefield, C1);
    assert_eq!(scouts.len(), 3);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: scouts
                    .into_iter()
                    .map(|id| (id, Defender::Player(them())))
                    .collect(),
            },
        )
        .unwrap();

    // Seat 0's single VentureIntoUndercity while already in Lost Mine offers
    // Lost Mine's room 1 (Goblin Lair) or room 2 (Mine Tunnels) — ONCE!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let goblin_lair_idx = match engine.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            options
                .iter()
                .find(|o| {
                    matches!(
                        o.kind,
                        CastModeKind::DungeonRoom {
                            dungeon: Dungeon::LostMine,
                            room: 1
                        }
                    )
                })
                .expect("advances Lost Mine rather than starting Undercity (CR 701.49d)")
                .index as usize
        }
        _ => unreachable!(),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(goblin_lair_idx))
        .unwrap();

    // Reach postcombat main: verify initiative was taken ONCE (room == 2, not 4!).
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(engine.state().initiative, Some(me()));
    assert_eq!(
        engine.state().players[0].dungeon.map(|(d, _)| d),
        Some(Dungeon::LostMine)
    );
    assert_eq!(
        engine.state().players[0].dungeon_room,
        2,
        "three creatures dealing combat damage trigger initiative takeover once, not thrice"
    );

    // Elimination pass-on (CR 725.4): on seat 1's turn, if seat 0 concedes/leaves,
    // active player seat 1 inherits the initiative and ventures into Undercity!
    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), Step::Main) && e.state().turn.phase == Phase::FirstMain
    });
    engine.apply(me(), PlayerAction::Concede).unwrap();
    assert_eq!(
        engine.state().initiative,
        Some(them()),
        "when initiative holder leaves on another player's turn, active player takes it (CR 725.4)"
    );
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == them()),
    );
    let objects = match engine.pending().clone() {
        Pending::ChooseCards { options, .. } => vec![options[0]],
        _ => unreachable!(),
    };
    engine
        .apply(them(), PlayerAction::ChooseObjects { objects })
        .unwrap();
    settle(&mut engine, them());
    assert_eq!(
        engine.state().players[1].dungeon.map(|(d, _)| d),
        Some(Dungeon::Undercity)
    );

    // Upkeep trigger (CR 725.2): advance to seat 2's turn, then back to seat 1's
    // upkeep -> seat 1 automatically ventures into Undercity on upkeep!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCastMode { player, .. } if *player == them()),
    );
    assert_eq!(engine.state().turn.active, them());
    assert_eq!(engine.state().turn.step, Step::Upkeep);
}

#[test]
fn become_types_and_computed_pt_counts_cover_all_new_variants() {
    static YOUR_LANDS: Filter = Filter::YOUR_LAND;
    static TWICE_HAND: PtCount = PtCount::Twice(&PtCount::CardsInHand(PlayerRel::You));

    static ANIMATE_PROBE: &[AbilityDef] = &[
        // 0: target permanent becomes a pure Enchantment (0 subtypes), keeping supertypes.
        free(
            &[Effect::CreateContinuousEffect {
                layer: Layer::Type,
                filter: &Filter::This,
                modifier: Modifier::BecomeTypes {
                    types: TypeSet::ENCHANTMENT,
                    subtypes: &[],
                },
                duration: Duration::UntilEndOfTurn,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        ),
        // 1: target permanent becomes a Phyrexian Imp Creature (2 subtypes) with locked
        //    SetPTByCountFilter (Twice(CardsInHand(You)) / PartySize(You)).
        free(
            &[
                Effect::CreateContinuousEffect {
                    layer: Layer::Type,
                    filter: &Filter::This,
                    modifier: Modifier::BecomeTypes {
                        types: TypeSet::CREATURE,
                        subtypes: &[creature::PHYREXIAN, creature::IMP],
                    },
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::SetPTByCountFilter {
                    filter: &Filter::This,
                    power: TWICE_HAND,
                    toughness: PtCount::PartySize(PlayerRel::You),
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        ),
    ];

    static DOMAIN_POWER: PtCount = PtCount::Domain(PlayerRel::You);
    static DISTINCT_LAND_NAMES: PtCount = PtCount::DistinctNamesAmong(&YOUR_LANDS);
    static STAT_CREATURE_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::SetPTSeparateToCount {
            power: DOMAIN_POWER,
            toughness: DISTINCT_LAND_NAMES,
        }
    )];

    static AVATAR_ABILITIES: &[AbilityDef] = &[
        static_ability!(Filter::Any, Modifier::PlayersCantLose),
        static_ability!(
            Filter::This,
            Modifier::SetPowerToCount(PtCount::LifeTotal(PlayerRel::You))
        ),
        static_ability!(
            Filter::This,
            Modifier::SetToughnessToCount(PtCount::GreatestManaValueAmong(
                &Filter::ControlledByYou
            ))
        ),
    ];

    static GY_LORD_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::SetPTToCount(PtCount::InGraveyards {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        })
    )];

    let animate_probe = card(
        C0,
        face("Animator", "{6}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ANIMATE_PROBE,
    );
    let dual_party = card(
        C1,
        FaceDef {
            subtypes: &[creature::CLERIC, creature::ROGUE],
            supertypes: SupertypeSet::LEGENDARY,
            ..creature_face("Dual Party", "{1}{W}", 2, 2)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let cleric_only = card(
        C2,
        FaceDef {
            subtypes: &[creature::CLERIC],
            ..creature_face("Lone Cleric", "{W}", 1, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let relic = card(
        C3,
        FaceDef {
            subtypes: &[artifact::TREASURE],
            ..face("Relic Box", "{2}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let tri_land = card(
        C4,
        FaceDef {
            subtypes: &[land::PLAINS, land::ISLAND, land::SWAMP],
            ..land_face("Triome Alpha")
        },
        KeywordSet::EMPTY,
        &[],
    );
    let stat_creature = card(
        C5,
        creature_face("Domain Scout", "{2}{G}", 1, 1),
        KeywordSet::EMPTY,
        STAT_CREATURE_ABILITIES,
    );
    let avatar = card(
        C6,
        creature_face("Life Avatar", "{4}{W}", 1, 1),
        KeywordSet::EMPTY,
        AVATAR_ABILITIES,
    );
    let gy_lord = card(
        C7,
        creature_face("Boneyard Wurm", "{1}{G}", 1, 1),
        KeywordSet::EMPTY,
        GY_LORD_ABILITIES,
    );

    let mut engine = bench(
        1804,
        vec![
            animate_probe,
            dual_party,
            cleric_only,
            relic,
            tri_land,
            stat_creature,
            avatar,
            gy_lord,
        ],
        [
            Seat::with(&[C0, C1, C3, C4, forest(), forest(), C5, C6, C7])
                .holding(&[C2, C1, C7])
                .at(7),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);
    let dual_id = the(&engine, ZoneLocation::Battlefield, C1);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C3);
    let scout_id = the(&engine, ZoneLocation::Battlefield, C5);
    let avatar_id = the(&engine, ZoneLocation::Battlefield, C6);
    // Initial Boneyard Wurm started at 0/0 with an empty graveyard and died to SBA.
    let dead_gy_lord_id = the(&engine, ZoneLocation::Graveyard(me()), C7);
    assert_eq!(
        engine.state().object(dead_gy_lord_id).unwrap().zone,
        Zone::Graveyard
    );

    // Domain(You) = 4 (Plains, Island, Swamp on Triome Alpha + Forest);
    // DistinctNamesAmong(YOUR_LANDS) = 2 ("Triome Alpha" and "Forest", even with two Forests!).
    assert_eq!(
        engine
            .state()
            .object(scout_id)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(scout_id)
            .unwrap()
            .characteristics()
            .toughness,
        Some(2)
    );

    // Avatar: power = LifeTotal(You) = 7, toughness = GreatestManaValueAmong(ControlledByYou) = 6 (Animator "{6}").
    assert_eq!(
        engine
            .state()
            .object(avatar_id)
            .unwrap()
            .characteristics()
            .power,
        Some(7)
    );
    assert_eq!(
        engine
            .state()
            .object(avatar_id)
            .unwrap()
            .characteristics()
            .toughness,
        Some(6)
    );
    // Negative life total under PlayersCantLose (CR 208.4):
    engine.state.players[0].life = -3;
    engine.state.effects.generation += 1;
    engine.state.refresh_characteristics();
    assert_eq!(
        engine
            .state()
            .object(avatar_id)
            .unwrap()
            .characteristics()
            .power,
        Some(-3)
    );
    engine.state.players[0].life = 7;
    engine.state.effects.generation += 1;
    engine.state.refresh_characteristics();

    // PartySize (CR 700.8): a single Cleric Rogue fills at most 1 party role!
    // Hand size = 3, so Twice(CardsInHand(You)) = 6, PartySize(You) = 1.
    // Animate relic_id into a Phyrexian Imp Creature with SetPTByCountFilter -> 6/1!
    activate(&mut engine, me(), probe_id, 1);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![relic_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let relic_chars = engine
        .state()
        .object(relic_id)
        .unwrap()
        .characteristics()
        .clone();
    assert_eq!(relic_chars.types, TypeSet::CREATURE);
    assert!(relic_chars.subtypes.contains(creature::PHYREXIAN));
    assert!(relic_chars.subtypes.contains(creature::IMP));
    assert!(!relic_chars.subtypes.contains(artifact::TREASURE));
    assert_eq!(relic_chars.power, Some(6));
    assert_eq!(relic_chars.toughness, Some(1));

    // Cast Lone Cleric from hand: hand size drops to 2 and PartySize rises to 2
    // (Lone Cleric takes Cleric, Dual Party takes Rogue!), yet relic_id's
    // SetPTByCountFilter stays locked at 6/1 per CR 608.2h / CR 611.2c!
    add_pool(&mut engine, me(), ManaColor::White, 1);
    let cleric_hand = the(&engine, ZoneLocation::Hand(me()), C2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: cleric_hand })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(relic_id)
            .unwrap()
            .characteristics()
            .power,
        Some(6)
    );
    assert_eq!(
        engine
            .state()
            .object(relic_id)
            .unwrap()
            .characteristics()
            .toughness,
        Some(1)
    );

    // Re-animating relic_id now locks Twice(2) = 4 and PartySize = 2!
    activate(&mut engine, me(), probe_id, 1);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![relic_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(relic_id)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(relic_id)
            .unwrap()
            .characteristics()
            .toughness,
        Some(2)
    );

    // BecomeTypes with 0 subtypes: Dual Party becomes a Legendary Enchantment (no subtypes!).
    activate(&mut engine, me(), probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![dual_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let dual_chars = engine
        .state()
        .object(dual_id)
        .unwrap()
        .characteristics()
        .clone();
    assert_eq!(dual_chars.types, TypeSet::ENCHANTMENT);
    assert!(dual_chars.supertypes.contains(SupertypeSet::LEGENDARY));
    assert!(dual_chars.subtypes.is_empty());

    // Cast second Boneyard Wurm from hand: with dead_gy_lord_id in graveyard,
    // PtCount::InGraveyards { who: You, filter: CREATURE } = 1, so it enters and survives as 1/1!
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    let second_gy_lord = the(&engine, ZoneLocation::Hand(me()), C7);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: second_gy_lord,
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let live_gy_lord = engine.state().object(second_gy_lord).unwrap();
    assert_eq!(live_gy_lord.zone, Zone::Battlefield);
    assert_eq!(live_gy_lord.characteristics().power, Some(1));
    assert_eq!(live_gy_lord.characteristics().toughness, Some(1));
}

#[test]
fn slice_a3_cr_fixes_walking_desecration_haunted_angel_and_riptide_entrancer() {
    static CHOSEN_TYPE_CREATURES: Filter =
        Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);

    // 1. Walking Desecration probe: choose a creature type; creatures of that type
    //    attack this turn if able (including creatures entering AFTER resolution!).
    static DESECRATION_ABILITIES: &[AbilityDef] = &[free(
        &[
            Effect::ChooseCreatureType { exclude: None },
            Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &CHOSEN_TYPE_CREATURES,
                modifier: Modifier::AttacksEachCombat,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        None,
    )];

    // 2. Haunted Angel probe: "When this creature dies, exile it and gain 3 life."
    static HAUNTED_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::ExileSource, Effect::gain_life(3)]
    )];

    // 3. Riptide Entrancer probe: "Whenever this creature deals combat damage to a
    //    player, you may sacrifice it. If you do, gain control of target creature."
    static GAIN_TARGET: Effect = Effect::CreateContinuousEffect {
        layer: Layer::Control,
        filter: &Filter::This,
        modifier: Modifier::GainControl,
        duration: Duration::Indefinitely,
    };
    static ENTRANCER_ABILITIES: &[AbilityDef] = &[AbilityDef::Triggered {
        trigger: Trigger::DealsCombatDamageToPlayer(&Filter::This),
        zone: TriggerZone::Battlefield,
        effects: &[Effect::PlayerMayPayCostThen {
            player: PlayerRel::You,
            cost: &CostPart::Sacrifice(&Filter::This),
            effect: &GAIN_TARGET,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        once_per_turn: false,
        condition: None,
        resolves_if_targets_illegal: false,
    }];

    // Instant-speed tools: destroy creature, return graveyard creature to hand, blink creature.
    static KILL_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static RAISE_DEAD: &[AbilityDef] = &[spell!(
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        )))
    )];
    static BLINK_INSTANT: &[AbilityDef] = &[spell!(
        &[Effect::blink_to_owner(TargetSpec::Object(
            &Filter::CREATURE
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let desecration = card(
        C0,
        face("Desecration Totem", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DESECRATION_ABILITIES,
    );
    let elf = card(
        C1,
        FaceDef {
            subtypes: &[creature::ELF],
            ..creature_face("Test Elf", "{G}", 1, 1)
        },
        KeywordSet::HASTE,
        &[],
    );
    let goblin = card(
        C2,
        FaceDef {
            subtypes: &[creature::GOBLIN],
            ..creature_face("Test Goblin", "{R}", 1, 1)
        },
        KeywordSet::HASTE,
        &[],
    );
    let haunted = card(
        C3,
        creature_face("Haunted Probe", "{2}{W}", 2, 2),
        KeywordSet::EMPTY,
        HAUNTED_ABILITIES,
    );
    let entrancer = card(
        C4,
        creature_face("Entrancer Probe", "{1}{U}{U}", 1, 1),
        KeywordSet::HASTE,
        ENTRANCER_ABILITIES,
    );
    let kill = card(
        C5,
        face("Instant Doom", "{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        KILL_SPELL,
    );
    let raise = card(
        C6,
        face("Instant Raise", "{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        RAISE_DEAD,
    );
    let blink = card(
        C7,
        face("Instant Flicker", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BLINK_INSTANT,
    );
    let prize = card(
        C8,
        creature_face("Prize Beast", "{3}{G}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1805,
        vec![
            desecration,
            elf,
            goblin,
            haunted,
            entrancer,
            kill,
            raise,
            blink,
            prize,
        ],
        [
            Seat::with(&[C0, C1, C2, C3, C3, C4]).holding(&[C1, C5, C5, C6, C7]),
            Seat::with(&[C8]),
        ],
    );
    to_main(&mut engine, me());
    let totem_id = the(&engine, ZoneLocation::Battlefield, C0);
    let elf_1 = the(&engine, ZoneLocation::Battlefield, C1);
    let goblin_id = the(&engine, ZoneLocation::Battlefield, C2);
    let haunted_ids = objects(&engine, ZoneLocation::Battlefield, C3);
    let entrancer_id = the(&engine, ZoneLocation::Battlefield, C4);
    let prize_id = the(&engine, ZoneLocation::Battlefield, C8);

    // --- Part A: Haunted Angel (CR 400.7e) ---
    // A1: Uninterrupted death exiles Haunted Probe from graveyard.
    let kill_cards = objects(&engine, ZoneLocation::Hand(me()), C5);
    add_pool(&mut engine, me(), ManaColor::Black, 3);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: kill_cards[0],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![haunted_ids[0]],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(haunted_ids[0]).unwrap().zone,
        Zone::Exile
    );

    // A2: Destroy second Haunted Probe, then return it from graveyard to hand in
    //     response to its dies trigger -> CR 400.7e keeps it in hand (NOT exiled!).
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: kill_cards[1],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![haunted_ids[1]],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        e.state().object(haunted_ids[1]).unwrap().zone == Zone::Graveyard
            && !e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    let raise_id = the(&engine, ZoneLocation::Hand(me()), C6);
    engine
        .apply(me(), PlayerAction::CastSpell { card: raise_id })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![haunted_ids[1]],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(haunted_ids[1]).unwrap().zone,
        Zone::Hand,
        "Haunted Probe returned to hand before its dies trigger resolved is a new object (CR 400.7e) and is NOT exiled"
    );

    // --- Part B: Walking Desecration (DslWithSubtype + CR 611.2c) ---
    activate(&mut engine, me(), totem_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseSubtype(creature::ELF))
        .unwrap();
    settle(&mut engine, me());

    // Cast a SECOND Elf AFTER Walking Desecration's effect has already resolved!
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    let elf_2 = the(&engine, ZoneLocation::Hand(me()), C1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: elf_2 })
        .unwrap();
    settle(&mut engine, me());

    // Move to DeclareAttackers: BOTH elf_1 and elf_2 MUST be in `required`, while goblin_id is NOT!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(required.contains(&elf_1), "existing Elf must attack");
    assert!(
        required.contains(&elf_2),
        "Elf entering after resolution must also attack (CR 611.2c)"
    );
    assert!(
        !required.contains(&goblin_id),
        "Goblin does not match chosen subtype Elf"
    );

    // --- Part C: Riptide Entrancer (CR 400.7 on blinked self-sacrifice) ---
    // Attack with elf_1, elf_2, and entrancer_id; opponent takes the hit!
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (elf_1, Defender::Player(them())),
                    (elf_2, Defender::Player(them())),
                    (entrancer_id, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![prize_id],
            },
        )
        .unwrap();

    // While Entrancer's combat-damage trigger is on the stack, blink Entrancer!
    walk_until(&mut engine, |e| {
        !e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    add_pool(&mut engine, me(), ManaColor::White, 1);
    let blink_id = the(&engine, ZoneLocation::Hand(me()), C7);
    engine
        .apply(me(), PlayerAction::CastSpell { card: blink_id })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![entrancer_id],
            },
        )
        .unwrap();

    // Let Blink and Entrancer's trigger both finish resolving: blinked Entrancer
    // CANNOT be sacrificed as "it" (CR 400.7), so Entrancer remains on the
    // battlefield and opponent retains control of Prize Beast!
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(
        engine.state().object(entrancer_id).unwrap().zone,
        Zone::Battlefield,
        "blinked Entrancer is a new object and was not sacrificed"
    );
    assert_eq!(
        engine.state().object(prize_id).unwrap().controller,
        them(),
        "opponent keeps Prize Beast when blinked Entrancer cannot pay 'sacrifice it'"
    );

    // --- Part D: Riptide Entrancer positive control (unblinked sacrifice steals target) ---
    to_main(&mut engine, me());
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(entrancer_id, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![prize_id],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![entrancer_id],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(
        engine.state().object(entrancer_id).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(prize_id).unwrap().controller,
        me(),
        "unblinked Entrancer sacrifices itself and steals Prize Beast"
    );
}
