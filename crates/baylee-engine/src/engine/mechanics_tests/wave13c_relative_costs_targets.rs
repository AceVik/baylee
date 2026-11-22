//! Beat 18 / Engine Wave 13c deterministic rule tests:
//! 1. `Modifier::AdjustSpellCostByCast` (`CR 601.2f`) across:
//!    - `SpellCastMetric::DistinctTargets` (*Hinata, Dawn-Crowned*: controller
//!      reduction and opponent tax across 1 target, 2 distinct targets, shared
//!      target across two target clauses counted once, and untargeted spells),
//!    - `SpellCastMetric::CardsDiscardedThisTurn` (*Hollow One*: cycling
//!      `DiscardSelf`, effect discard, clamping at `{0}`, and next-turn reset),
//!    - `SpellCastMetric::XPaid` + `SpellCastMetric::PermanentsSacrificedThisTurn`
//!      (*Dargo, the Shipwrecker*: `SacrificeX` + earlier matching permanent
//!      sacrifices while ignoring non-matching land sacrifices, and carrying
//!      sacrificed objects into subsequent casts the same turn),
//!    - `LifeGainedThisTurn`, `LifeLostByOpponentsThisTurn`,
//!      `SpellsCastThisTurn` (*Damping Sphere*-style positive tax), and
//!      `CardsDrawnThisTurn`.
//! 2. `TargetConstraint::OnePerPlayer` (`CR 115.1`, `CR 115.7`, `CR 603.3d`,
//!    `CR 608.2b`) across:
//!    - 2-player and 3-player *Vaevictis Asmadi*-style mandatory (`min = 1`)
//!      and optional (`min = 0`) triggers, spells, activated abilities, and
//!      graveyard zone ownership in strict APNAP turn order,
//!    - `CR 115.7` retargeting (`ChooseNewTargets`) keeping each slot within its
//!      bound player group,
//!    - `CR 608.2b` partial illegality (destruction, hexproof, and mid-stack
//!      control change) leaving surviving players' targets intact and fizzling
//!      only when every target becomes illegal.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::object::Status;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    Amount, CostPart, CounterKind, Effect, Filter, Modifier, PlayerRel, SpellCastMetric,
    StaticAbility, TargetConstraint, TargetReq, TargetSpec, Trigger, TriggerZone, ZoneRef, spell,
    static_ability,
};
use baylee_core::mana::{ManaColor, ManaPool};

const C0: u32 = 99_930;
const C1: u32 = 99_931;
const C2: u32 = 99_932;
const C3: u32 = 99_933;
const C4: u32 = 99_934;
const C5: u32 = 99_935;
const C6: u32 = 99_936;
const C7: u32 = 99_937;
const C8: u32 = 99_938;
const C9: u32 = 99_939;
const C10: u32 = 99_940;

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

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    refresh_offer(engine);
}

fn clear_pool(engine: &mut Bench, player: PlayerId) {
    engine.state.players[usize::from(player.get())].mana_pool = ManaPool::default();
    refresh_offer(engine);
}

fn pool_total(engine: &Bench, player: PlayerId) -> u64 {
    engine.state().players[usize::from(player.get())]
        .mana_pool
        .total()
}

fn is_tapped(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

// =========================================================================
// 1. Hinata, Dawn-Crowned — DistinctTargets reduction & tax
// =========================================================================

static HINATA_ABILITIES: &[AbilityDef] = &[
    static_ability!(
        Filter::Any,
        Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &Filter::Any,
            delta_per_unit: -1,
            metric: SpellCastMetric::DistinctTargets,
        }
    ),
    static_ability!(
        Filter::Any,
        Modifier::AdjustSpellCostByCast {
            who: PlayerRel::Opponent,
            filter: &Filter::Any,
            delta_per_unit: 1,
            metric: SpellCastMetric::DistinctTargets,
        }
    ),
];

#[test]
fn hinata_reduces_controller_spells_per_distinct_target_and_counts_shared_target_once() {
    static ONE_TARGET: &[AbilityDef] = &[spell!(
        &[Effect::TapTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static TWO_CLAUSE_TARGET: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::TapTarget],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        condition: None,
    }];
    static NO_TARGET: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let hinata = card(
        C0,
        creature_face("Hinata Probe", "{1}{U}{R}{W}", 4, 4),
        KeywordSet::EMPTY,
        HINATA_ABILITIES,
    );
    let one_t = card(
        C1,
        face("Single Target Tap", "{1}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ONE_TARGET,
    );
    let two_t_a = card(
        C2,
        face("Two Clause Spell A", "{2}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        TWO_CLAUSE_TARGET,
    );
    let two_t_b = card(
        C3,
        face("Two Clause Spell B", "{2}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        TWO_CLAUSE_TARGET,
    );
    let two_t_c = card(
        C4,
        face("Two Clause Spell C", "{2}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        TWO_CLAUSE_TARGET,
    );
    let untargeted = card(
        C5,
        face("Untargeted Life", "{2}{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        NO_TARGET,
    );
    let bear_a = card(
        C6,
        creature_face("Bear A", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b = card(
        C7,
        creature_face("Bear B", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1301,
        vec![
            hinata, one_t, two_t_a, two_t_b, two_t_c, untargeted, bear_a, bear_b,
        ],
        [
            Seat::with(&[C0, C6]).holding(&[C1, C2, C3, C4, C5]),
            Seat::with(&[C7]),
        ],
    );
    to_main(&mut engine, me());

    let one_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let two_a_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let two_b_id = the(&engine, ZoneLocation::Hand(me()), C3);
    let two_c_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let untargeted_id = the(&engine, ZoneLocation::Hand(me()), C5);
    let bear_a_id = the(&engine, ZoneLocation::Battlefield, C6);
    let bear_b_id = the(&engine, ZoneLocation::Battlefield, C7);

    // Float only {R}: 1-target {1}{R} and 2-target {2}{R} are castable via
    // Hinata's preview reduction, while untargeted {2}{R} is not.
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_reduction(one_id, CastModeKind::Normal), 1);
    assert_eq!(
        legal.spell_reduction(untargeted_id, CastModeKind::Normal),
        0
    );
    assert!(
        legal.castable.contains(&one_id),
        "Hinata reduces 1-target {{1}}{{R}} spell to {{R}}"
    );
    assert!(
        legal.castable.contains(&two_a_id),
        "Hinata preview allows casting 2-target {{2}}{{R}} spell off {{R}}"
    );
    assert!(
        !legal.castable.contains(&untargeted_id),
        "untargeted {{2}}{{R}} receives no DistinctTargets reduction"
    );

    // 1. Cast 1-target {1}{R} off {R} -> pays {R}, leaves 0 floating.
    engine
        .apply(me(), PlayerAction::CastSpell { card: one_id })
        .expect("cast 1-target spell");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b_id],
                players: vec![],
            },
        )
        .expect("choose 1 target");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0, "1-target spell spent {{R}}");
    assert!(is_tapped(&engine, bear_b_id), "1-target spell resolved");

    // 2. Cast 2-clause {2}{R} targeting TWO DISTINCT creatures off {R} -> pays {R}.
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: two_a_id })
        .expect("cast 2-clause spell with 2 distinct targets");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("first target bear_a");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b_id],
                players: vec![],
            },
        )
        .expect("second target bear_b");
    settle(&mut engine, me());
    assert_eq!(
        pool_total(&engine, me()),
        0,
        "2 distinct targets reduced {{2}}{{R}} to {{R}}"
    );

    // 3. Cast 2-clause {2}{R} targeting the SAME creature twice with {1}{R} ->
    //    only 1 distinct target (-{1}), so total cost is {1}{R}, spending ALL {1}{R}!
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: two_b_id })
        .expect("cast 2-clause spell with shared target");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("first target bear_a");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("second target ALSO bear_a");
    settle(&mut engine, me());
    assert_eq!(
        pool_total(&engine, me()),
        0,
        "shared target across two target clauses counts as 1 distinct target, paying {{1}}{{R}}"
    );

    // 4. Cast 2-clause {2}{R} with ONLY {R} and pick the SAME target twice ->
    //    total cost becomes {1}{R}, which {R} cannot pay, so cast cancels cleanly!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: two_c_id })
        .expect("begin casting 2-clause spell off {R}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("first target bear_a");
    let res = engine.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![bear_a_id],
            players: vec![],
        },
    );
    assert!(
        res.is_err()
            || engine
                .state()
                .zones
                .list(ZoneLocation::Hand(me()))
                .contains(&two_c_id),
        "shared target needs {{1}}{{R}}, cannot sneak through for {{R}}: res={res:?}, pending={:?}",
        engine.pending()
    );
    assert_eq!(
        pool_total(&engine, me()),
        1,
        "failed cast refunds/preserves floating {{R}}"
    );
}

#[test]
fn hinata_taxes_opponent_spells_per_distinct_target_including_shared_target() {
    static ONE_TARGET: &[AbilityDef] = &[spell!(
        &[Effect::TapTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static TWO_CLAUSE_TARGET: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::TapTarget],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        condition: None,
    }];
    static NO_TARGET: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let hinata = card(
        C0,
        creature_face("Hinata Probe", "{1}{U}{R}{W}", 4, 4),
        KeywordSet::EMPTY,
        HINATA_ABILITIES,
    );
    let untargeted = card(
        C1,
        face("Opponent Untargeted", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        NO_TARGET,
    );
    let one_t = card(
        C2,
        face("Opponent Bolt", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ONE_TARGET,
    );
    let two_distinct = card(
        C3,
        face("Opponent Twin A", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        TWO_CLAUSE_TARGET,
    );
    let two_shared = card(
        C4,
        face("Opponent Twin B", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        TWO_CLAUSE_TARGET,
    );
    let bear_a = card(
        C5,
        creature_face("Bear A", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b = card(
        C6,
        creature_face("Bear B", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1302,
        vec![
            hinata,
            untargeted,
            one_t,
            two_distinct,
            two_shared,
            bear_a,
            bear_b,
        ],
        [
            Seat::with(&[C0, C5]),
            Seat::with(&[C6]).holding(&[C1, C2, C3, C4]),
        ],
    );
    to_main(&mut engine, them());

    let untargeted_id = the(&engine, ZoneLocation::Hand(them()), C1);
    let one_id = the(&engine, ZoneLocation::Hand(them()), C2);
    let two_distinct_id = the(&engine, ZoneLocation::Hand(them()), C3);
    let two_shared_id = the(&engine, ZoneLocation::Hand(them()), C4);
    let bear_a_id = the(&engine, ZoneLocation::Battlefield, C5);
    let bear_b_id = the(&engine, ZoneLocation::Battlefield, C6);

    // With only {R}: untargeted {R} is castable (+0 tax), 1-target {R} is NOT (+1 tax).
    add_pool(&mut engine, them(), ManaColor::Red, 1);
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_increase(untargeted_id, CastModeKind::Normal), 0);
    assert_eq!(legal.spell_increase(one_id, CastModeKind::Normal), 1);
    assert!(legal.castable.contains(&untargeted_id));
    assert!(
        !legal.castable.contains(&one_id),
        "Hinata taxes opponent's 1-target {{R}} spell to {{1}}{{R}}"
    );

    // 1. Untargeted {R} spends exactly {R}.
    engine
        .apply(
            them(),
            PlayerAction::CastSpell {
                card: untargeted_id,
            },
        )
        .expect("cast untargeted spell without tax");
    settle(&mut engine, them());
    assert_eq!(pool_total(&engine, them()), 0);

    // 2. 1-target {R} with {1}{R} pays {1}{R}.
    add_pool(&mut engine, them(), ManaColor::Red, 2);
    engine
        .apply(them(), PlayerAction::CastSpell { card: one_id })
        .expect("cast 1-target spell paying +1 tax");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("choose 1 target");
    settle(&mut engine, them());
    assert_eq!(pool_total(&engine, them()), 0, "paid {{1}}{{R}}");

    // 3. 2-clause {R} targeting 2 distinct creatures with {2}{R} pays {2}{R} (+2 tax).
    add_pool(&mut engine, them(), ManaColor::Red, 3);
    engine
        .apply(
            them(),
            PlayerAction::CastSpell {
                card: two_distinct_id,
            },
        )
        .expect("cast 2-clause spell with 2 distinct targets");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("first target");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b_id],
                players: vec![],
            },
        )
        .expect("second distinct target");
    settle(&mut engine, them());
    assert_eq!(
        pool_total(&engine, them()),
        0,
        "2 distinct targets taxed {{R}} by +{{2}} -> {{2}}{{R}}"
    );

    // 4. 2-clause {R} targeting the SAME creature twice with {2}{R} pays ONLY {1}{R} (+1 tax),
    //    leaving {R} floating!
    add_pool(&mut engine, them(), ManaColor::Red, 3);
    engine
        .apply(
            them(),
            PlayerAction::CastSpell {
                card: two_shared_id,
            },
        )
        .expect("cast 2-clause spell with shared target");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("first target bear_a");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("second target ALSO bear_a");
    settle(&mut engine, them());
    assert_eq!(
        pool_total(&engine, them()),
        1,
        "shared target across two clauses taxed by +{{1}} only -> paid {{1}}{{R}} out of {{2}}{{R}}"
    );
}

// =========================================================================
// 2. Hollow One — CardsDiscardedThisTurn (cycling + effect discard + reset)
// =========================================================================

#[test]
fn hollow_one_reduction_tracks_cycling_and_effect_discards_and_resets_next_turn() {
    static HOLLOW_ONE_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &Filter::This,
            delta_per_unit: -2,
            metric: SpellCastMetric::CardsDiscardedThisTurn,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static CYCLING_ONE: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{1}"),
            parts: &[CostPart::DiscardSelf],
        },
        effects: &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Hand,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];
    static DISCARD_PROBE: &[AbilityDef] = &[free(
        &[Effect::DiscardCount {
            who: PlayerRel::You,
            count: Amount::Fixed(1),
        }],
        None,
    )];

    let hollow_a = card(
        C0,
        FaceDef {
            power: Some(4),
            toughness: Some(4),
            ..face(
                "Hollow One A",
                "{5}",
                TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            )
        },
        KeywordSet::EMPTY,
        HOLLOW_ONE_ABILITIES,
    );
    let hollow_b = card(
        C1,
        FaceDef {
            power: Some(4),
            toughness: Some(4),
            ..face(
                "Hollow One B",
                "{5}",
                TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            )
        },
        KeywordSet::EMPTY,
        HOLLOW_ONE_ABILITIES,
    );
    let cycler = card(
        C2,
        face("Cycler Probe", "{3}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CYCLING_ONE,
    );
    let discarder = card(
        C3,
        face("Discard Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DISCARD_PROBE,
    );
    let fodder_a = card(
        C4,
        creature_face("Fodder A", "{1}{G}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let fodder_b = card(
        C5,
        creature_face("Fodder B", "{1}{G}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1303,
        vec![hollow_a, hollow_b, cycler, discarder, fodder_a, fodder_b],
        [
            Seat::with(&[C3]).holding(&[C0, C1, C2, C4, C5]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let hollow_a_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let hollow_b_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let cycler_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let discarder_id = the(&engine, ZoneLocation::Battlefield, C3);
    let fodder_a_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let fodder_b_id = the(&engine, ZoneLocation::Hand(me()), C5);

    // 0 discards -> reduction 0; {3} is NOT enough for {5}.
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    let (_, legal0) = offer(&engine);
    assert_eq!(legal0.spell_reduction(hollow_a_id, CastModeKind::Normal), 0);
    assert!(!legal0.castable.contains(&hollow_a_id));

    // 1. Cycle Cycler Probe ({1} + DiscardSelf) -> 1 discard -> reduction {2} -> {3} left!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    activate(&mut engine, me(), cycler_id, 0);
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 3);
    let (_, legal1) = offer(&engine);
    assert_eq!(
        legal1.spell_reduction(hollow_a_id, CastModeKind::Normal),
        2,
        "cycling (DiscardSelf) counts as a discard this turn"
    );
    assert!(
        legal1.castable.contains(&hollow_a_id),
        "Hollow One costs {{3}} after 1 cycle"
    );

    // 2. Discard 2 more cards via effect -> 3 discards total -> reduction {6} clamps {5} to {0}!
    clear_pool(&mut engine, me());
    for fodder in [fodder_a_id, fodder_b_id] {
        activate(&mut engine, me(), discarder_id, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .expect("choose card to discard");
        settle(&mut engine, me());
    }
    let (_, legal3) = offer(&engine);
    assert_eq!(legal3.spell_reduction(hollow_a_id, CastModeKind::Normal), 6);
    assert!(
        legal3.castable.contains(&hollow_a_id),
        "Hollow One is free ({{0}}) after 3 discards"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: hollow_a_id })
        .expect("cast Hollow One A for {0}");
    settle(&mut engine, me());
    let _ = the(&engine, ZoneLocation::Battlefield, C0);

    // 3. Advance to next turn -> per-turn discard count resets to 0 for Hollow One B!
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    let (_, next_turn_legal) = offer(&engine);
    assert_eq!(
        next_turn_legal.spell_reduction(hollow_b_id, CastModeKind::Normal),
        0,
        "CardsDiscardedThisTurn resets on the next turn"
    );
    assert!(!next_turn_legal.castable.contains(&hollow_b_id));
}

// =========================================================================
// 3. Dargo, the Shipwrecker — XPaid + PermanentsSacrificedThisTurn
// =========================================================================

#[test]
fn dargo_combines_sacrifice_x_and_earlier_matching_sacrifices_ignoring_nonmatching() {
    static DARGO_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            filter: Filter::This,
            layer: baylee_cards_dsl::Layer::Text,
            modifier: Modifier::AdjustSpellCostByCast {
                who: PlayerRel::You,
                filter: &Filter::This,
                delta_per_unit: -2,
                metric: SpellCastMetric::XPaid,
            },
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
        AbilityDef::Static(StaticAbility {
            filter: Filter::This,
            layer: baylee_cards_dsl::Layer::Text,
            modifier: Modifier::AdjustSpellCostByCast {
                who: PlayerRel::You,
                filter: &Filter::This,
                delta_per_unit: -2,
                metric: SpellCastMetric::PermanentsSacrificedThisTurn(
                    &Filter::ARTIFACT_OR_CREATURE,
                ),
            },
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
    ];
    static SAC_ANY_PROBE: &[AbilityDef] = &[free(
        &[Effect::SacrificeFilter {
            who: PlayerRel::You,
            filter: &Filter::Any,
        }],
        None,
    )];
    static SAC_X_ART_OR_CREATURE: &[CostPart] =
        &[CostPart::SacrificeX(&Filter::ARTIFACT_OR_CREATURE)];

    let dargo_a = card(
        C0,
        FaceDef {
            mandatory_additional_costs: SAC_X_ART_OR_CREATURE,
            ..creature_face("Dargo A", "{6}{R}", 7, 5)
        },
        KeywordSet::EMPTY,
        DARGO_ABILITIES,
    );
    let dargo_b = card(
        C1,
        FaceDef {
            mandatory_additional_costs: SAC_X_ART_OR_CREATURE,
            ..creature_face("Dargo B", "{6}{R}", 7, 5)
        },
        KeywordSet::EMPTY,
        DARGO_ABILITIES,
    );
    let relic_1 = card(
        C2,
        face("Relic 1", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let relic_2 = card(
        C3,
        face("Relic 2", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let relic_3 = card(
        C4,
        face("Relic 3", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let sac_probe = card(
        C5,
        face("Sacrifice Probe", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SAC_ANY_PROBE,
    );

    let mut engine = bench(
        1304,
        vec![dargo_a, dargo_b, relic_1, relic_2, relic_3, sac_probe],
        [
            Seat::with(&[forest(), C2, C3, C4, C5]).holding(&[C0, C1]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let dargo_a_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let dargo_b_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let land_id = the(&engine, ZoneLocation::Battlefield, forest());
    let relic_1_id = the(&engine, ZoneLocation::Battlefield, C2);
    let relic_2_id = the(&engine, ZoneLocation::Battlefield, C3);
    let relic_3_id = the(&engine, ZoneLocation::Battlefield, C4);
    let sac_probe_id = the(&engine, ZoneLocation::Battlefield, C5);

    // 1. Sacrifice the Forest (a land, NOT an artifact or creature) -> reduction stays 0.
    activate(&mut engine, me(), sac_probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![land_id],
            },
        )
        .expect("sacrifice land");
    settle(&mut engine, me());
    let (_, legal_after_land) = offer(&engine);
    assert_eq!(
        legal_after_land.spell_reduction(dargo_a_id, CastModeKind::Normal),
        0,
        "sacrificing a land must not reduce Dargo's ARTIFACT_OR_CREATURE metric"
    );

    // 2. Sacrifice Relic 1 (artifact) -> earlier-sacrifice reduction becomes {2}.
    activate(&mut engine, me(), sac_probe_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![relic_1_id],
            },
        )
        .expect("sacrifice relic 1");
    settle(&mut engine, me());

    // 3. Float ONLY {R} (no lands left!). Preview sees 2 remaining artifacts for
    //    SacrificeX (-{4}) + 1 earlier artifact (-{2}) = -{6} -> Dargo A castable for {R}!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let (_, legal_dargo) = offer(&engine);
    assert_eq!(
        legal_dargo.spell_reduction(dargo_a_id, CastModeKind::Normal),
        2
    );
    assert!(
        legal_dargo.castable.contains(&dargo_a_id),
        "Dargo A ({{6}}{{R}}) must be castable off {{R}} via 1 earlier sac (-2) + SacrificeX=2 (-4)"
    );

    engine
        .apply(me(), PlayerAction::CastSpell { card: dargo_a_id })
        .expect("cast Dargo A");
    let Pending::ChooseNumber {
        min: x_min,
        max: x_max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseNumber for SacrificeX, got {:?}",
            engine.pending()
        );
    };
    assert_eq!((x_min, x_max), (0, 2));
    engine
        .apply(me(), PlayerAction::ChooseNumber(2))
        .expect("announce X = 2");
    for relic in [relic_2_id, relic_3_id] {
        let Pending::ChooseCards {
            min: sac_min,
            max: sac_max,
            ..
        } = engine.pending().clone()
        else {
            panic!(
                "expected ChooseCards for SacrificeX, got {:?}",
                engine.pending()
            );
        };
        assert_eq!((sac_min, sac_max), (1, 1));
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![relic],
                },
            )
            .expect("sacrifice relic for SacrificeX");
    }
    settle(&mut engine, me());
    let _ = the(&engine, ZoneLocation::Battlefield, C0);
    assert_eq!(
        pool_total(&engine, me()),
        0,
        "Dargo A spent exactly {{R}} after -6 total reduction"
    );

    // 4. Now 3 artifacts have been sacrificed this turn (Relic 1, 2, 3) ->
    //    Dargo B has static reduction {6} and casts with X = 0 for {R}!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let (_, legal_b) = offer(&engine);
    assert_eq!(
        legal_b.spell_reduction(dargo_b_id, CastModeKind::Normal),
        6,
        "all 3 sacrificed artifacts this turn reduce subsequent Dargo by {{6}}"
    );
    assert!(legal_b.castable.contains(&dargo_b_id));
    engine
        .apply(me(), PlayerAction::CastSpell { card: dargo_b_id })
        .expect("cast Dargo B");
    engine
        .apply(me(), PlayerAction::ChooseNumber(0))
        .expect("announce X = 0 (no sacrifices needed)");
    settle(&mut engine, me());
    let _ = the(&engine, ZoneLocation::Battlefield, C1);
    assert_eq!(pool_total(&engine, me()), 0);
}

// =========================================================================
// 4. Remaining SpellCastMetric variants
// =========================================================================

#[test]
fn remaining_spell_cast_metrics_life_gained_opponent_life_lost_spells_cast_and_cards_drawn() {
    static GAIN_LIFE_REDUCER: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &Filter::This,
            delta_per_unit: -1,
            metric: SpellCastMetric::LifeGainedThisTurn,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static OPP_LOSS_REDUCER: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &Filter::This,
            delta_per_unit: -1,
            metric: SpellCastMetric::LifeLostByOpponentsThisTurn,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static CARDS_DRAWN_REDUCER: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &Filter::This,
            delta_per_unit: -1,
            metric: SpellCastMetric::CardsDrawnThisTurn,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static DAMPING_SPHERE_TAX: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::AdjustSpellCostByCast {
            who: PlayerRel::EachPlayer,
            filter: &Filter::Any,
            delta_per_unit: 1,
            metric: SpellCastMetric::SpellsCastThisTurn,
        }
    )];
    static PROBE_ABILITIES: &[AbilityDef] = &[
        free(
            &[Effect::GainLife {
                amount: Amount::Fixed(3),
            }],
            None,
        ),
        free(
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::Opponent),
            }],
            None,
        ),
        free(
            &[Effect::DrawCards {
                amount: Amount::Fixed(2),
            }],
            None,
        ),
    ];
    static CHEAP_SPELL: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(0),
    }])];

    let gain_spell = card(
        C0,
        creature_face("Life Gain Creature", "{4}{W}", 3, 3),
        KeywordSet::EMPTY,
        GAIN_LIFE_REDUCER,
    );
    let opp_loss_spell = card(
        C1,
        creature_face("Rakdos Creature", "{4}{B}", 3, 3),
        KeywordSet::EMPTY,
        OPP_LOSS_REDUCER,
    );
    let draw_spell = card(
        C2,
        creature_face("Drawn Creature", "{3}{U}", 3, 3),
        KeywordSet::EMPTY,
        CARDS_DRAWN_REDUCER,
    );
    let sphere = card(
        C3,
        face("Damping Probe", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DAMPING_SPHERE_TAX,
    );
    let probe = card(
        C4,
        face("Metric Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE_ABILITIES,
    );
    let ping_1 = card(
        C5,
        face("Cheap Spell 1", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CHEAP_SPELL,
    );
    let ping_2 = card(
        C6,
        face("Cheap Spell 2", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CHEAP_SPELL,
    );
    let ping_3 = card(
        C7,
        face("Cheap Spell 3", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CHEAP_SPELL,
    );

    // Part A: LifeGainedThisTurn, LifeLostByOpponentsThisTurn, CardsDrawnThisTurn
    let mut engine = bench(
        1305,
        vec![gain_spell, opp_loss_spell, draw_spell, probe],
        [Seat::with(&[C4]).holding(&[C0, C1, C2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let gain_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let loss_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let draw_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let probe_id = the(&engine, ZoneLocation::Battlefield, C4);

    // Turn 1 active player skipped draw step -> all 3 reductions start at 0.
    let (_, initial) = offer(&engine);
    assert_eq!(initial.spell_reduction(gain_id, CastModeKind::Normal), 0);
    assert_eq!(initial.spell_reduction(loss_id, CastModeKind::Normal), 0);
    assert_eq!(initial.spell_reduction(draw_id, CastModeKind::Normal), 0);

    // Gain 3 life, deal 3 damage to opponent, draw 2 cards.
    for idx in 0..3 {
        activate(&mut engine, me(), probe_id, idx);
        settle(&mut engine, me());
    }
    let (_, after) = offer(&engine);
    assert_eq!(after.spell_reduction(gain_id, CastModeKind::Normal), 3);
    assert_eq!(after.spell_reduction(loss_id, CastModeKind::Normal), 3);
    assert_eq!(after.spell_reduction(draw_id, CastModeKind::Normal), 2);

    // Cast each spell at its exact reduced mana cost ({1}{W}, {1}{B}, {1}{U}).
    add_pool(&mut engine, me(), ManaColor::White, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: gain_id })
        .expect("cast {4}{W} for {1}{W}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: loss_id })
        .expect("cast {4}{B} for {1}{B}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: draw_id })
        .expect("cast {3}{U} for {1}{U}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // Part B: SpellsCastThisTurn positive tax (Damping Sphere style: +0, +1, +2).
    let mut sphere_engine = bench(
        1306,
        vec![sphere, ping_1, ping_2, ping_3],
        [Seat::with(&[C3]).holding(&[C5, C6, C7]), Seat::default()],
    );
    to_main(&mut sphere_engine, me());
    let p1 = the(&sphere_engine, ZoneLocation::Hand(me()), C5);
    let p2 = the(&sphere_engine, ZoneLocation::Hand(me()), C6);
    let p3 = the(&sphere_engine, ZoneLocation::Hand(me()), C7);

    // Spell 1: +0 tax -> pays {R}.
    add_pool(&mut sphere_engine, me(), ManaColor::Red, 1);
    assert_eq!(
        offer(&sphere_engine)
            .1
            .spell_increase(p1, CastModeKind::Normal),
        0
    );
    sphere_engine
        .apply(me(), PlayerAction::CastSpell { card: p1 })
        .expect("cast 1st spell for {R}");
    settle(&mut sphere_engine, me());
    assert_eq!(pool_total(&sphere_engine, me()), 0);

    // Spell 2: +1 tax -> refused with {R}, pays {1}{R}.
    add_pool(&mut sphere_engine, me(), ManaColor::Red, 1);
    let (_, l2) = offer(&sphere_engine);
    assert_eq!(l2.spell_increase(p2, CastModeKind::Normal), 1);
    assert!(!l2.castable.contains(&p2));
    add_pool(&mut sphere_engine, me(), ManaColor::Red, 1);
    sphere_engine
        .apply(me(), PlayerAction::CastSpell { card: p2 })
        .expect("cast 2nd spell for {1}{R}");
    settle(&mut sphere_engine, me());
    assert_eq!(pool_total(&sphere_engine, me()), 0);

    // Spell 3: +2 tax -> pays {2}{R}.
    add_pool(&mut sphere_engine, me(), ManaColor::Red, 3);
    assert_eq!(
        offer(&sphere_engine)
            .1
            .spell_increase(p3, CastModeKind::Normal),
        2
    );
    sphere_engine
        .apply(me(), PlayerAction::CastSpell { card: p3 })
        .expect("cast 3rd spell for {2}{R}");
    settle(&mut sphere_engine, me());
    assert_eq!(pool_total(&sphere_engine, me()), 0);
}

// =========================================================================
// 5. TargetConstraint::OnePerPlayer — 2p & 3p mandatory/optional, APNAP, GY
// =========================================================================

#[test]
fn one_per_player_vaevictis_trigger_spell_and_ability_in_3p_mandatory_vs_optional() {
    const MANDATORY_NONLAND: TargetReq = TargetReq::one(TargetSpec::Object(&Filter::NONLAND))
        .with_constraint(TargetConstraint::OnePerPlayer);
    const OPTIONAL_NONLAND: TargetReq = TargetReq::up_to(TargetSpec::Object(&Filter::NONLAND), 1)
        .with_constraint(TargetConstraint::OnePerPlayer);
    const MANDATORY_GY_CREATURE: TargetReq = TargetReq::one(TargetSpec::CardInGraveyard(
        &Filter::CREATURE,
        PlayerRel::EachPlayer,
    ))
    .with_constraint(TargetConstraint::OnePerPlayer);

    static VAEVICTIS_ETB: &[AbilityDef] = &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::NONLAND),
            no_regen: false,
        }],
        targets: Some(MANDATORY_NONLAND),
        second_targets: None,
        condition: None,
        zone: TriggerZone::Battlefield,
        once_per_turn: false,
        resolves_if_targets_illegal: false,
    }];
    static MANDATORY_SPELL_AB: &[AbilityDef] = &[spell!(
        &[Effect::TapTarget],
        targets = Some(MANDATORY_NONLAND)
    )];
    static OPTIONAL_SPELL_AB: &[AbilityDef] = &[spell!(
        &[Effect::TapTarget],
        targets = Some(OPTIONAL_NONLAND)
    )];
    static MANDATORY_ACT_AB: &[AbilityDef] = &[free(&[Effect::TapTarget], Some(MANDATORY_NONLAND))];
    static GY_EXILE_AB: &[AbilityDef] = &[free(
        &[Effect::Exile {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer),
        }],
        Some(MANDATORY_GY_CREATURE),
    )];

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let p2 = PlayerId::new(2);

    let vaevictis = card(
        C0,
        creature_face("Vaevictis Probe", "{0}", 6, 6),
        KeywordSet::EMPTY,
        VAEVICTIS_ETB,
    );
    let mand_spell = card(
        C1,
        face("Mandatory OnePerPlayer", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        MANDATORY_SPELL_AB,
    );
    let opt_spell = card(
        C2,
        face("Optional OnePerPlayer", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        OPTIONAL_SPELL_AB,
    );
    let mand_act = card(
        C3,
        face("Mandatory Activator", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        MANDATORY_ACT_AB,
    );
    let gy_act = card(
        C4,
        face("GY Activator", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        GY_EXILE_AB,
    );
    let bear_0 = card(
        C5,
        creature_face("Bear P0", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_1 = card(
        C6,
        creature_face("Bear P1", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_2 = card(
        C7,
        creature_face("Bear P2", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let hexproof_2 = card(
        C8,
        creature_face("Hexproof P2", "{1}{G}", 2, 2),
        KeywordSet::HEXPROOF,
        &[],
    );

    // 1. 3-player board: P0, P1, P2 each control a nonland permanent.
    //    Cast Vaevictis Probe on P0's turn -> ETB trigger asks in turn order
    //    P0 -> P1 -> P2, offering ONLY each player's own nonland permanents!
    let mut engine = bench(
        1307,
        vec![
            vaevictis, mand_spell, opt_spell, mand_act, gy_act, bear_0, bear_1, bear_2, hexproof_2,
        ],
        [
            Seat::with(&[C3, C4, C5]).holding(&[C0, C1, C2]),
            Seat::with(&[C6]),
            Seat::with(&[forest(), C7, C8]),
        ],
    );
    to_main(&mut engine, p0);

    let vaev_hand = the(&engine, ZoneLocation::Hand(p0), C0);
    let mand_spell_id = the(&engine, ZoneLocation::Hand(p0), C1);
    let opt_spell_id = the(&engine, ZoneLocation::Hand(p0), C2);
    let mand_act_id = the(&engine, ZoneLocation::Battlefield, C3);
    let gy_act_id = the(&engine, ZoneLocation::Battlefield, C4);
    let bear_0_id = the(&engine, ZoneLocation::Battlefield, C5);
    let bear_1_id = the(&engine, ZoneLocation::Battlefield, C6);
    let bear_2_id = the(&engine, ZoneLocation::Battlefield, C7);
    let hex_2_id = the(&engine, ZoneLocation::Battlefield, C8);
    let forest_2_id = the(&engine, ZoneLocation::Battlefield, forest());

    engine
        .apply(p0, PlayerAction::CastSpell { card: vaev_hand })
        .expect("cast Vaevictis Probe");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let vaev_bf = the(&engine, ZoneLocation::Battlefield, C0);

    // Prompt 1: P0's nonland permanents only.
    let Pending::ChooseTargets {
        options: opts0,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (1, 1));
    assert!(opts0.contains(&bear_0_id) && opts0.contains(&vaev_bf));
    assert!(!opts0.contains(&bear_1_id) && !opts0.contains(&bear_2_id));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_0_id],
                players: vec![],
            },
        )
        .expect("choose P0 target");

    // Prompt 2: P1's nonland permanents only.
    let Pending::ChooseTargets { options: opts1, .. } = engine.pending().clone() else {
        panic!("expected P1 target prompt, got {:?}", engine.pending());
    };
    assert_eq!(opts1, vec![bear_1_id]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_1_id],
                players: vec![],
            },
        )
        .expect("choose P1 target");

    // Prompt 3: P2's legal nonland permanents only (excludes land AND hexproof!).
    let Pending::ChooseTargets { options: opts2, .. } = engine.pending().clone() else {
        panic!("expected P2 target prompt, got {:?}", engine.pending());
    };
    assert_eq!(
        opts2,
        vec![bear_2_id],
        "P2 prompt must exclude P2's land ({forest_2_id:?}) and hexproof creature ({hex_2_id:?})"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_2_id],
                players: vec![],
            },
        )
        .expect("choose P2 target");

    // Resolve trigger: all 3 chosen bears are destroyed -> moved to their owners' graveyards!
    settle(&mut engine, p0);
    let gy_0 = the(&engine, ZoneLocation::Graveyard(p0), C5);
    let gy_1 = the(&engine, ZoneLocation::Graveyard(p1), C6);
    let gy_2 = the(&engine, ZoneLocation::Graveyard(p2), C7);

    // 2. Now P2 controls ONLY a Forest and a Hexproof creature -> NO legal
    //    nonland target for P2!
    //    - Mandatory OnePerPlayer spell & activated ability are NOT offered!
    //    - Optional OnePerPlayer spell IS offered and skips P2!
    let (_, legal_missing_p2) = offer(&engine);
    assert!(
        !legal_missing_p2.castable.contains(&mand_spell_id),
        "mandatory OnePerPlayer spell cannot be cast when P2 has no legal target"
    );
    assert!(
        !legal_missing_p2.abilities.contains(&(mand_act_id, 0)),
        "mandatory OnePerPlayer activation cannot be activated when P2 has no legal target"
    );
    assert!(
        legal_missing_p2.castable.contains(&opt_spell_id),
        "optional (min=0) OnePerPlayer spell skips players with no legal target"
    );

    // Cast optional OnePerPlayer spell: P0 has permanents, P1 has NONE (bear_1 died),
    // P2 has NONE legal -> prompts ONLY for P0 (min=0, max=1) and skips P1 & P2!
    engine
        .apply(p0, PlayerAction::CastSpell { card: opt_spell_id })
        .expect("cast optional OnePerPlayer spell");
    let Pending::ChooseTargets {
        options: opt_opts,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected single prompt for P0, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 1));
    assert!(opt_opts.contains(&vaev_bf));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![vaev_bf],
                players: vec![],
            },
        )
        .expect("choose vaev_bf for P0");
    settle(&mut engine, p0);
    assert!(
        is_tapped(&engine, vaev_bf),
        "optional OnePerPlayer skipped P1 & P2 and tapped P0's chosen target"
    );

    // 3. Non-battlefield graveyard zone ownership: each player now has 1 creature
    //    card in their graveyard (gy_0, gy_1, gy_2)! Activate gy_act -> prompts in
    //    APNAP order P0 -> P1 -> P2 and exiles all 3!
    activate(&mut engine, p0, gy_act_id, 0);
    for (expected_gy, label) in [(gy_0, "P0 GY"), (gy_1, "P1 GY"), (gy_2, "P2 GY")] {
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("expected {label} target prompt, got {:?}", engine.pending());
        };
        assert_eq!(options, vec![expected_gy], "{label} prompt");
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![expected_gy],
                    players: vec![],
                },
            )
            .expect("choose graveyard target");
    }
    settle(&mut engine, p0);
    let exiled_total = engine.state().zones.list(ZoneLocation::Exile(p0)).len()
        + engine.state().zones.list(ZoneLocation::Exile(p1)).len()
        + engine.state().zones.list(ZoneLocation::Exile(p2)).len();
    assert_eq!(
        exiled_total, 3,
        "OnePerPlayer graveyard ability exiled 1 creature from each of the 3 graveyards"
    );

    // 4. Mandatory OnePerPlayer trigger when a player has no legal target (CR 603.3d):
    //    bounce Vaevictis to hand and recast while P1 & P2 have no legal nonland target ->
    //    ETB trigger must be removed from stack without prompting or destroying anything!
    engine
        .state
        .move_object(
            vaev_bf,
            ZoneLocation::Hand(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);
    let vaev_hand2 = the(&engine, ZoneLocation::Hand(p0), C0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: vaev_hand2 })
        .expect("recast Vaevictis when P1/P2 have no legal targets");
    settle(&mut engine, p0);
    let _ = the(&engine, ZoneLocation::Battlefield, C0);
    let _ = the(&engine, ZoneLocation::Battlefield, C3);

    // 5. Turn-order rotation check: advance to P1's main phase with a creature on
    //    each player's board and verify OnePerPlayer prompts P1 -> P2 -> P0!
    let mut rot = bench(
        1308,
        vec![mand_spell, bear_0, bear_1, bear_2],
        [
            Seat::with(&[C5]),
            Seat::with(&[C6]).holding(&[C1]),
            Seat::with(&[C7]),
        ],
    );
    to_main(&mut rot, p1);
    let spell_p1 = the(&rot, ZoneLocation::Hand(p1), C1);
    let b0 = the(&rot, ZoneLocation::Battlefield, C5);
    let b1 = the(&rot, ZoneLocation::Battlefield, C6);
    let b2 = the(&rot, ZoneLocation::Battlefield, C7);

    rot.apply(p1, PlayerAction::CastSpell { card: spell_p1 })
        .expect("P1 casts mandatory OnePerPlayer spell");
    for (expected, label) in [(b1, "active P1"), (b2, "next P2"), (b0, "last P0")] {
        let Pending::ChooseTargets { options, .. } = rot.pending().clone() else {
            panic!("expected {label} prompt, got {:?}", rot.pending());
        };
        assert_eq!(options, vec![expected], "APNAP order must ask {label}");
        rot.apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![expected],
                players: vec![],
            },
        )
        .expect("choose turn-order target");
    }
    settle(&mut rot, p1);
    assert!(is_tapped(&rot, b0));
    assert!(is_tapped(&rot, b1));
    assert!(is_tapped(&rot, b2));
}

// =========================================================================
// 6. TargetConstraint::OnePerPlayer — CR 115.7 retargeting & CR 608.2b
// =========================================================================

#[test]
fn one_per_player_retargeting_and_cr608_2b_partial_illegality_and_control_change() {
    const ONE_CREATURE_PER_PLAYER: TargetReq =
        TargetReq::one(TargetSpec::Object(&Filter::CREATURE))
            .with_constraint(TargetConstraint::OnePerPlayer);

    static ONE_PER_PUMP: &[AbilityDef] = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets = Some(ONE_CREATURE_PER_PLAYER)
    )];
    static RETARGET_PROBE: &[AbilityDef] = &[free(
        &[Effect::ChooseNewTargets],
        Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    static DESTROY_PROBE: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static STEAL_PROBE: &[AbilityDef] = &[free(
        &[Effect::ChangeController {
            new_controller: PlayerRel::You,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let spell_a = card(
        C0,
        face("OnePerPlayer Pump A", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ONE_PER_PUMP,
    );
    let spell_b = card(
        C1,
        face("OnePerPlayer Pump B", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ONE_PER_PUMP,
    );
    let spell_c = card(
        C2,
        face("OnePerPlayer Pump C", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ONE_PER_PUMP,
    );
    let bear_a1 = card(
        C3,
        creature_face("Bear A1", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_a2 = card(
        C4,
        creature_face("Bear A2", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b1 = card(
        C5,
        creature_face("Bear B1", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b2 = card(
        C6,
        creature_face("Bear B2", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let retargeter = card(
        C7,
        face("Retarget Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        RETARGET_PROBE,
    );
    let destroyer = card(
        C8,
        face("Destroy Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DESTROY_PROBE,
    );
    let stealer = card(
        C9,
        face("Steal Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        STEAL_PROBE,
    );
    let bear_b3 = card(
        C10,
        creature_face("Bear B3", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1309,
        vec![
            spell_a, spell_b, spell_c, bear_a1, bear_a2, bear_b1, bear_b2, bear_b3, retargeter,
            destroyer, stealer,
        ],
        [
            Seat::with(&[C3, C4, C7, C8, C9]).holding(&[C0, C1, C2]),
            Seat::with(&[C5, C6, C10]),
        ],
    );
    to_main(&mut engine, me());

    let spell_a_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let spell_b_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let spell_c_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let a1 = the(&engine, ZoneLocation::Battlefield, C3);
    let a2 = the(&engine, ZoneLocation::Battlefield, C4);
    let b1 = the(&engine, ZoneLocation::Battlefield, C5);
    let b2 = the(&engine, ZoneLocation::Battlefield, C6);
    let b3 = the(&engine, ZoneLocation::Battlefield, C10);
    let retarget_id = the(&engine, ZoneLocation::Battlefield, C7);
    let destroy_id = the(&engine, ZoneLocation::Battlefield, C8);
    let steal_id = the(&engine, ZoneLocation::Battlefield, C9);

    // 1. Cast spell_a targeting [a1 (me()), b1 (them())].
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_a_id })
        .expect("cast spell_a");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![a1],
                players: vec![],
            },
        )
        .expect("target a1 for me()");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![b1],
                players: vec![],
            },
        )
        .expect("target b1 for them()");
    let spell_a_stack = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();

    // 2. CR 115.7 retarget check: slot 0 (bound to me()) can redirect ONLY to a2,
    //    never cross-player to b1 or b2; slot 1 (bound to them()) can redirect ONLY to b2!
    activate(&mut engine, me(), retarget_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![spell_a_stack],
                players: vec![],
            },
        )
        .expect("target spell_a on stack");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options: slot0_opts,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert!(slot0_opts.contains(&a2));
    assert!(
        !slot0_opts.contains(&b1) && !slot0_opts.contains(&b2),
        "slot 0 bound to me() must reject them()'s creatures: {slot0_opts:?}"
    );
    assert!(
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![b2],
                    players: vec![],
                },
            )
            .is_err(),
        "retargeting slot 0 to another player's creature must fail"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![a2],
                players: vec![],
            },
        )
        .expect("redirect slot 0 from a1 to a2");

    let Pending::ChooseTargets {
        options: slot1_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected slot 1 retarget prompt, got {:?}",
            engine.pending()
        );
    };
    assert!(slot1_opts.contains(&b2));
    assert!(
        !slot1_opts.contains(&a1) && !slot1_opts.contains(&a2),
        "slot 1 bound to them() must reject me()'s creatures: {slot1_opts:?}"
    );
    // Keep slot 1 unchanged on b1.
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("keep slot 1 on b1");

    // 3. CR 608.2b partial illegality via destruction: destroy b1 in response ->
    //    spell_a still resolves and puts a +1/+1 counter on a2 (and NOT a1)!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    activate(&mut engine, me(), destroy_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![b1],
                players: vec![],
            },
        )
        .expect("destroy b1 before spell_a resolves");
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(a2)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "a2 received +1/+1 counter despite b1 dying (CR 608.2b partial illegality)"
    );
    assert_eq!(
        engine
            .state()
            .object(a1)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "a1 was retargeted away from"
    );

    // 4. CR 608.2b partial illegality via mid-stack control change:
    //    cast spell_b targeting [a1 (me()), b2 (them())], then steal b2 to me()
    //    in response -> b2 is no longer controlled by its bound player them(),
    //    so only a1 gets the +1/+1 counter!
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_b_id })
        .expect("cast spell_b");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![a1],
                players: vec![],
            },
        )
        .expect("target a1");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![b2],
                players: vec![],
            },
        )
        .expect("target b2");
    activate(&mut engine, me(), steal_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![b2],
                players: vec![],
            },
        )
        .expect("steal b2 in response");
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(a1)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "a1 stays legal and receives +1/+1 counter"
    );
    assert_eq!(
        engine
            .state()
            .object(b2)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "b2 changed controller to me() so it is illegal for them()'s bound slot"
    );

    // 5. CR 608.2b total fizzle: cast spell_c targeting [a1 (me()), b3 (them())],
    //    destroy both in response -> spell_c fizzles cleanly!
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_c_id })
        .expect("cast spell_c");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![a1],
                players: vec![],
            },
        )
        .expect("target a1");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![b3],
                players: vec![],
            },
        )
        .expect("target b3");
    for victim in [a1, b3] {
        activate(&mut engine, me(), destroy_id, 0);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![victim],
                    players: vec![],
                },
            )
            .expect("destroy target in response");
    }
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .contains(&spell_c_id),
        "spell_c fizzled cleanly into graveyard when all targets became illegal"
    );
}
