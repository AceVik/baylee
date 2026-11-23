//! Wave 15a engine rule tests — Turn-History Counts & General Amount Comparisons:
//! 1. Celebration (*Armory Mice*): `Amount::EnteredThisTurn` (nonland permanents
//!    entering battlefield under your control, tokens included, lands excluded,
//!    persists after departure, `exclude_source` CR 400.7 blink semantics, turn reset).
//! 2. *Arclight Phoenix* graveyard `CombatBegin` trigger + *April O'Neil*:
//!    `Amount::SpellsCastMatchingThisTurn` (`INSTANT_OR_SORCERY`, countered spells
//!    still count, creatures excluded) and `Amount::DistinctCardTypesCastThisTurn`
//!    (multi-type artifact creature contributes two types, duplicate types collapse).
//! 3. *Aven Heartstabber* distinct graveyard mana values:
//!    `Amount::DistinctManaValuesInZone` (land = MV 0, duplicate MVs collapse,
//!    live static reactivity on add/exile, filtered variants and battlefield zone).
//! 4. *Asmira, Holy Avenger* creatures put into your graveyard from battlefield:
//!    `Amount::EnteredThisTurn { from: Some(Battlefield), to: Graveyard, .. }`
//!    (tokens count, discards from hand excluded, exiled deaths excluded, opponent
//!    deaths excluded, turn reset).
//! 5. *Akuta, Born of Ash* hand size vs highest opponent + all 6 `AmountCmp` ops:
//!    `Amount::HighestOpponentHandSize` in 3-player game (ties, strictly greater,
//!    eliminated opponent excluded) + `Condition::AmountCompare` (`Lt`, `Le`, `Eq`,
//!    `Ne`, `Ge`, `Gt`).
//! 6. *Blitzball Stadium* distinct counter kinds:
//!    `Amount::DistinctCounterKindsOnSource` (duplicates collapse, removal drops
//!    count, LKI after sacrificing source as cost, CR 608.2h).
//! 7. *Beast* counter placed on source this turn:
//!    `Condition::CountersPlacedOnSourceThisTurn { kind, by_you }` (wrong kind
//!    ignored, opponent vs you distinction, persists after counter removal, resets
//!    on blink CR 400.7 and on turn change).
//! 8. `Amount::CreaturesAttackedThisTurn { filter, controller }` (filtered by
//!    subtype, survives attacker death via LKI, attacking tokens excluded per
//!    CR 508.4, extra combat does not double-count same incarnation, turn reset).
//! 9. `snapshot_hash()` and `loop_signature()` determinism, sensitivity when
//!    tracked, and zero perturbation when untracked.
#![allow(clippy::too_many_lines)]

use super::*;
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, AmountCmp, Condition, Cost, CostPart, CounterKind, Effect, Filter, KeywordSet,
    Modifier, PlayerRel, StepKind, TargetReq, TargetSpec, TokenDef, Trigger, TriggerZone, ZoneRef,
    ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::{Defender, SubtypeId};
use baylee_core::mana::ManaColor;

const C_BASE: u32 = 15_100;

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

fn advance_to_step(engine: &mut Bench, seat: PlayerId, step: Step) {
    for _ in 0..512 {
        if holds_priority_in(engine, seat, step) {
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
                    "unexpected pending while advancing to {step:?}: {pending:?}"
                );
            }
        }
    }
    panic!("failed to reach {step:?} for {seat:?}");
}

fn advance_to_next_main(engine: &mut Bench, seat: PlayerId) {
    let start_turn = engine.state().turn.number;
    for _ in 0..1024 {
        if engine.state().turn.number > start_turn
            && holds_priority_in(engine, seat, Step::Main)
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
                    "unexpected pending while advancing to next main: {pending:?}"
                );
            }
        }
    }
    panic!("failed to reach next turn main phase for {seat:?}");
}

// =========================================================================
// 1. Celebration (Armory Mice) + exclude_source CR 400.7
// =========================================================================

#[test]
fn wave15a_celebration_armory_mice_and_exclude_source() {
    const MICE: u32 = C_BASE;
    const BEACON: u32 = C_BASE + 1;
    const TOKEN_MAKER: u32 = C_BASE + 2;
    const SAC_TARGET: u32 = C_BASE + 3;
    const BLINKER: u32 = C_BASE + 4;
    const LAND_CARD: u32 = C_BASE + 5;
    const BEAR: u32 = C_BASE + 6;

    static NONLAND_ENTERED_YOU: Amount = Amount::EnteredThisTurn {
        filter: &Filter::NONLAND,
        from: None,
        to: ZoneRef::Battlefield,
        owner_or_ctrl: PlayerRel::You,
        exclude_source: false,
    };
    static OTHER_NONLAND_ENTERED_YOU: Amount = Amount::EnteredThisTurn {
        filter: &Filter::NONLAND,
        from: None,
        to: ZoneRef::Battlefield,
        owner_or_ctrl: PlayerRel::You,
        exclude_source: true,
    };
    static TWO: Amount = Amount::Fixed(2);
    static ONE: Amount = Amount::Fixed(1);

    static CELEBRATION: Condition = Condition::AmountCompare {
        left: &NONLAND_ENTERED_YOU,
        op: AmountCmp::AtLeast,
        right: &TWO,
    };
    static ANOTHER_ENTERED: Condition = Condition::AmountCompare {
        left: &OTHER_NONLAND_ENTERED_YOU,
        op: AmountCmp::AtLeast,
        right: &ONE,
    };

    static MICE_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ModifyPT(2, 2),
        condition = Some(CELEBRATION)
    )];
    static BEACON_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ModifyPT(1, 1),
        condition = Some(ANOTHER_ENTERED)
    )];
    static MOUSE_TOKEN: TokenDef = TokenDef {
        name: "Mouse",
        colors: ColorSet::of(Color::White),
        types: TypeSet::CREATURE,
        power: Some(1),
        toughness: Some(1),
        ..TokenDef::DEFAULT
    };
    static TOKEN_MAKER_AB: &[AbilityDef] = &[free(
        &[Effect::CreateToken {
            token: &MOUSE_TOKEN,
        }],
        None,
    )];
    static DESTROY_AB: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static BLINK_AB: &[AbilityDef] = &[free(
        &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            MICE,
            creature_face("Armory Mice Proxy", "{1}{W}", 3, 1),
            KeywordSet::EMPTY,
            MICE_AB,
        ),
        card(
            BEACON,
            creature_face("Exclude-Self Watch", "{1}{W}", 2, 2),
            KeywordSet::EMPTY,
            BEACON_AB,
        ),
        card(
            TOKEN_MAKER,
            face("Mouse Whistle", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TOKEN_MAKER_AB,
        ),
        card(
            SAC_TARGET,
            face("Clean Removal", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DESTROY_AB,
        ),
        card(
            BLINKER,
            face("Blink Device", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BLINK_AB,
        ),
        card(LAND_CARD, land_face("Plains Proxy"), KeywordSet::EMPTY, &[]),
        card(
            BEAR,
            creature_face("Silver Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        1510,
        cards,
        [
            Seat::with(&[TOKEN_MAKER, SAC_TARGET, BLINKER]).holding(&[MICE, LAND_CARD, BEACON]),
            Seat::with(&[TOKEN_MAKER]).holding(&[BEAR]),
        ],
    );
    to_main(&mut engine, me());
    assert!(
        engine.state().per_turn.zone_changes.tracked,
        "zone_changes tracking must activate for Amount::EnteredThisTurn"
    );

    // Playing a land does NOT count as a nonland permanent entering.
    let plains = the(&engine, ZoneLocation::Hand(me()), LAND_CARD);
    engine
        .apply(me(), PlayerAction::PlayLand { card: plains })
        .unwrap();
    settle_stack(&mut engine, me());

    // Cast Armory Mice (nonland permanent #1). Celebration is still OFF (3/1).
    add_pool(&mut engine, me(), ManaColor::White, 2);
    let mice_card = the(&engine, ZoneLocation::Hand(me()), MICE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: mice_card })
        .unwrap();
    settle_stack(&mut engine, me());
    let mice = the(&engine, ZoneLocation::Battlefield, MICE);
    assert_eq!(
        (
            engine.state().object(mice).unwrap().characteristics().power,
            engine
                .state()
                .object(mice)
                .unwrap()
                .characteristics()
                .toughness
        ),
        (Some(3), Some(1)),
        "1 nonland permanent + 1 land is not enough for Celebration"
    );

    // Opponent creating a token does NOT count for your Celebration.
    let opp_maker = objects(&engine, ZoneLocation::Battlefield, TOKEN_MAKER)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == them())
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ActivateAbility {
                source: opp_maker,
                ability_index: 0,
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(mice).unwrap().characteristics().power,
        Some(3),
        "opponent's permanent entering must not trigger your Celebration"
    );

    // Create a Mouse token under YOUR control (nonland permanent #2): Celebration turns ON (5/3)!
    let my_maker = objects(&engine, ZoneLocation::Battlefield, TOKEN_MAKER)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    activate(&mut engine, me(), my_maker, 0);
    settle_stack(&mut engine, me());
    let mouse_token = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == me() && !o.is_card()
        })
        .expect("mouse token on battlefield");
    assert_eq!(
        (
            engine.state().object(mice).unwrap().characteristics().power,
            engine
                .state()
                .object(mice)
                .unwrap()
                .characteristics()
                .toughness
        ),
        (Some(5), Some(3)),
        "Celebration active after 2 nonland permanents entered under your control"
    );

    // Destroying the token does NOT turn Celebration off this turn (historical entry persists).
    let killer = the(&engine, ZoneLocation::Battlefield, SAC_TARGET);
    activate(&mut engine, me(), killer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![mouse_token],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        (
            engine.state().object(mice).unwrap().characteristics().power,
            engine
                .state()
                .object(mice)
                .unwrap()
                .characteristics()
                .toughness
        ),
        (Some(5), Some(3)),
        "Celebration persists even after an entered permanent leaves this turn"
    );

    // Advance to next turn: Celebration resets back to 3/1.
    advance_to_next_main(&mut engine, me());
    assert_eq!(
        (
            engine.state().object(mice).unwrap().characteristics().power,
            engine
                .state()
                .object(mice)
                .unwrap()
                .characteristics()
                .toughness
        ),
        (Some(3), Some(1)),
        "Celebration resets at turn boundary"
    );

    // Now test `exclude_source: true` on BEACON on this fresh turn:
    // Casting BEACON alone enters 1 nonland permanent (itself), so `exclude_source: true` sees 0 -> 2/2.
    add_pool(&mut engine, me(), ManaColor::White, 2);
    let beacon_card = the(&engine, ZoneLocation::Hand(me()), BEACON);
    engine
        .apply(me(), PlayerAction::CastSpell { card: beacon_card })
        .unwrap();
    settle_stack(&mut engine, me());
    let beacon = the(&engine, ZoneLocation::Battlefield, BEACON);
    assert_eq!(
        engine
            .state()
            .object(beacon)
            .unwrap()
            .characteristics()
            .power,
        Some(2),
        "exclude_source must exclude the current incarnation entering"
    );

    // Blinking BEACON gives it a new incarnation (CR 400.7): the earlier incarnation
    // that entered this turn is now a different object, so `exclude_source: true` sees 1 -> 3/3!
    let blinker = the(&engine, ZoneLocation::Battlefield, BLINKER);
    activate(&mut engine, me(), blinker, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![beacon],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(beacon)
            .unwrap()
            .characteristics()
            .power,
        Some(3),
        "after blink, prior incarnation counts as another entrant (CR 400.7)"
    );
}

// =========================================================================
// 2. Arclight Phoenix (SpellsCastMatchingThisTurn) + April O'Neil
//    (DistinctCardTypesCastThisTurn)
// =========================================================================

#[test]
fn wave15a_arclight_phoenix_and_distinct_card_types_cast() {
    const PHOENIX: u32 = C_BASE + 10;
    const INSTANT_SPELL: u32 = C_BASE + 11;
    const SORCERY_SPELL: u32 = C_BASE + 12;
    const COUNTER_SPELL: u32 = C_BASE + 13;
    const ARTIFACT_CREATURE: u32 = C_BASE + 14;
    const TYPE_COUNTER_PROBE: u32 = C_BASE + 15;

    static INSTANT_OR_SORCERY_YOU: Amount = Amount::SpellsCastMatchingThisTurn {
        filter: &Filter::INSTANT_OR_SORCERY,
        caster: PlayerRel::You,
    };
    static THREE: Amount = Amount::Fixed(3);
    static PHOENIX_COND: Condition = Condition::AmountCompare {
        left: &INSTANT_OR_SORCERY_YOU,
        op: AmountCmp::AtLeast,
        right: &THREE,
    };

    static PHOENIX_AB: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::You,
        },
        &[Effect::ReturnSourceFromGraveyard {
            face: None,
            tapped: false,
            owner_control: true,
            counters: None,
        }],
        zone = TriggerZone::Graveyard,
        condition = Some(PHOENIX_COND)
    )];
    static NOOP_SPELL: &[AbilityDef] = &[spell!(&[Effect::gain_life(1)])];
    static COUNTER_AB: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    static PROBE_AB: &[AbilityDef] = &[free(
        &[Effect::GainLife {
            amount: Amount::DistinctCardTypesCastThisTurn(PlayerRel::You),
        }],
        None,
    )];

    let cards = vec![
        card(
            PHOENIX,
            creature_face("Arclight Proxy", "{3}{R}", 3, 2),
            KeywordSet::FLYING.union(KeywordSet::HASTE),
            PHOENIX_AB,
        ),
        card(
            INSTANT_SPELL,
            face("Opt Proxy", "{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            NOOP_SPELL,
        ),
        card(
            SORCERY_SPELL,
            face("Serum Proxy", "{U}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            NOOP_SPELL,
        ),
        card(
            COUNTER_SPELL,
            face("Cancel Proxy", "{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            COUNTER_AB,
        ),
        card(
            ARTIFACT_CREATURE,
            FaceDef {
                types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
                power: Some(1),
                toughness: Some(1),
                ..face("Memnite Proxy", "{0}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TYPE_COUNTER_PROBE,
            face("April Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PROBE_AB,
        ),
    ];

    let mut engine = bench(
        1511,
        cards,
        [
            Seat::with(&[TYPE_COUNTER_PROBE]).holding(&[
                PHOENIX,
                INSTANT_SPELL,
                INSTANT_SPELL,
                SORCERY_SPELL,
                ARTIFACT_CREATURE,
            ]),
            Seat::with(&[]).holding(&[COUNTER_SPELL, INSTANT_SPELL]),
        ],
    );
    to_main(&mut engine, me());
    assert!(
        engine.state().per_turn.spells_cast_log.tracked,
        "spells_cast_log tracking must activate"
    );

    // Put Arclight Phoenix into our graveyard.
    let phoenix = the(&engine, ZoneLocation::Hand(me()), PHOENIX);
    engine
        .state
        .move_object(
            phoenix,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    let probe = the(&engine, ZoneLocation::Battlefield, TYPE_COUNTER_PROBE);
    let base_life = life(&engine, me());

    // Turn 1: cast 1 Instant + 1 Sorcery + 1 Artifact Creature (only 2 instants/sorceries!).
    // Also check DistinctCardTypesCastThisTurn rises: 0 -> 1 -> 2 -> 4 (Artifact + Creature).
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    let inst1 = objects(&engine, ZoneLocation::Hand(me()), INSTANT_SPELL)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: inst1 })
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        eval::amount(
            &Amount::DistinctCardTypesCastThisTurn(PlayerRel::You),
            engine.state(),
            me(),
            probe,
            None,
        ),
        1,
        "one instant cast -> 1 distinct card type"
    );

    let sorc1 = the(&engine, ZoneLocation::Hand(me()), SORCERY_SPELL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sorc1 })
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        eval::amount(
            &Amount::DistinctCardTypesCastThisTurn(PlayerRel::You),
            engine.state(),
            me(),
            probe,
            None,
        ),
        2,
        "instant + sorcery -> 2 distinct card types"
    );

    let memnite = the(&engine, ZoneLocation::Hand(me()), ARTIFACT_CREATURE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: memnite })
        .unwrap();
    settle_stack(&mut engine, me());

    // Resolve the probe to confirm DistinctCardTypesCastThisTurn == 4 inside resolution too.
    let life_before_probe = life(&engine, me());
    activate(&mut engine, me(), probe, 0);
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()) - life_before_probe,
        4,
        "Instant + Sorcery + Artifact Creature = 4 distinct card types"
    );
    assert!(life(&engine, me()) > base_life);

    // Opponent casting an instant does NOT push OUR instant/sorcery count to 3.
    add_pool(&mut engine, them(), ManaColor::Blue, 2);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let opp_inst = the(&engine, ZoneLocation::Hand(them()), INSTANT_SPELL);
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp_inst })
        .unwrap();
    settle_stack(&mut engine, me());

    // Enter CombatBegin with only 2 of OUR instants/sorceries cast: Phoenix stays in graveyard!
    advance_to_step(&mut engine, me(), Step::CombatBegin);
    assert_eq!(
        engine.state().object(phoenix).unwrap().zone,
        Zone::Graveyard,
        "2 instants/sorceries + 1 creature + 1 opponent instant must NOT return Arclight Phoenix"
    );

    // Next turn: cast 2 instants + 1 sorcery (with the 3rd spell COUNTERED by opponent!).
    advance_to_next_main(&mut engine, me());
    assert_eq!(
        eval::amount(
            &Amount::DistinctCardTypesCastThisTurn(PlayerRel::You),
            engine.state(),
            me(),
            probe,
            None,
        ),
        0,
        "distinct card types cast resets at turn start"
    );

    // Move previously cast spells back to hand for turn 2 cast test.
    let gy_insts: Vec<ObjectId> = objects(&engine, ZoneLocation::Graveyard(me()), INSTANT_SPELL);
    let gy_sorc = the(&engine, ZoneLocation::Graveyard(me()), SORCERY_SPELL);
    for id in gy_insts.into_iter().chain(std::iter::once(gy_sorc)) {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Hand(me()),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }
    refresh_offer(&mut engine);

    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    add_pool(&mut engine, them(), ManaColor::Blue, 1);

    // Cast instant #1 (resolves).
    let i1 = objects(&engine, ZoneLocation::Hand(me()), INSTANT_SPELL)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: i1 })
        .unwrap();
    settle_stack(&mut engine, me());
    // Cast instant #2 (resolves). Two instants still give 1 distinct card type!
    let i2 = objects(&engine, ZoneLocation::Hand(me()), INSTANT_SPELL)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: i2 })
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        eval::amount(
            &Amount::DistinctCardTypesCastThisTurn(PlayerRel::You),
            engine.state(),
            me(),
            probe,
            None,
        ),
        1,
        "casting two Instants only yields 1 distinct card type"
    );

    // Cast sorcery #3, and opponent COUNTERS it — still counts as cast (CR 601.2i)!
    let s1 = the(&engine, ZoneLocation::Hand(me()), SORCERY_SPELL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s1 })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let cancel = the(&engine, ZoneLocation::Hand(them()), COUNTER_SPELL);
    engine
        .apply(them(), PlayerAction::CastSpell { card: cancel })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![s1],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());

    assert_eq!(
        eval::amount(&INSTANT_OR_SORCERY_YOU, engine.state(), me(), probe, None),
        3,
        "countered sorcery still counts as cast this turn"
    );

    // Advance to CombatBegin -> Arclight Phoenix triggers from graveyard and returns!
    advance_to_step(&mut engine, me(), Step::CombatBegin);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(phoenix).unwrap().zone,
        Zone::Battlefield,
        "Arclight Phoenix returns from graveyard at CombatBegin after 3 instants/sorceries cast"
    );
}

// =========================================================================
// 3. Aven Heartstabber (DistinctManaValuesInZone)
// =========================================================================

#[test]
fn wave15a_aven_heartstabber_distinct_graveyard_mana_values() {
    const AVEN: u32 = C_BASE + 20;
    const MV0_LAND: u32 = C_BASE + 21;
    const MV1_CARD: u32 = C_BASE + 22;
    const MV2_CARD: u32 = C_BASE + 23;
    const MV3_CARD: u32 = C_BASE + 24;
    const MV4_CARD: u32 = C_BASE + 25;

    static GY_DISTINCT_MV: Amount = Amount::DistinctManaValuesInZone {
        zone: ZoneSel::GraveyardYou,
        filter: &Filter::Any,
    };
    static GY_CREATURE_DISTINCT_MV: Amount = Amount::DistinctManaValuesInZone {
        zone: ZoneSel::GraveyardYou,
        filter: &Filter::CREATURE,
    };
    static FIVE: Amount = Amount::Fixed(5);
    static AVEN_COND: Condition = Condition::AmountCompare {
        left: &GY_DISTINCT_MV,
        op: AmountCmp::AtLeast,
        right: &FIVE,
    };

    static AVEN_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(2, 2),
            condition = Some(AVEN_COND)
        ),
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::DEATHTOUCH),
            condition = Some(AVEN_COND)
        ),
    ];

    let cards = vec![
        card(
            AVEN,
            creature_face("Aven Heartstabber Proxy", "{U}{B}", 1, 1),
            KeywordSet::FLYING,
            AVEN_AB,
        ),
        card(MV0_LAND, land_face("Island Proxy"), KeywordSet::EMPTY, &[]),
        card(
            MV1_CARD,
            creature_face("One-Drop", "{U}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MV2_CARD,
            creature_face("Two-Drop", "{1}{B}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MV3_CARD,
            face("Three-Drop Instant", "{2}{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MV4_CARD,
            creature_face("Four-Drop", "{3}{B}", 4, 4),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        1512,
        cards,
        [
            Seat::with(&[AVEN])
                .holding(&[MV0_LAND, MV1_CARD, MV2_CARD, MV2_CARD, MV3_CARD, MV4_CARD]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let aven = the(&engine, ZoneLocation::Battlefield, AVEN);
    let hand_mv0 = the(&engine, ZoneLocation::Hand(me()), MV0_LAND);
    let hand_mv1 = the(&engine, ZoneLocation::Hand(me()), MV1_CARD);
    let hand_mv2s = objects(&engine, ZoneLocation::Hand(me()), MV2_CARD);
    let hand_mv3 = the(&engine, ZoneLocation::Hand(me()), MV3_CARD);
    let hand_mv4 = the(&engine, ZoneLocation::Hand(me()), MV4_CARD);

    // Put MV 0 (land), MV 1, TWO MV 2 cards, and MV 3 into graveyard:
    // 5 cards total, but only 4 distinct mana values {0, 1, 2, 3}.
    for id in [hand_mv0, hand_mv1, hand_mv2s[0], hand_mv2s[1], hand_mv3] {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Graveyard(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine.sync_static_effects();
    engine.state.refresh_characteristics();

    assert_eq!(
        eval::amount(&GY_DISTINCT_MV, engine.state(), me(), aven, None),
        4,
        "duplicate MV 2 must collapse to 4 distinct MVs: {{0, 1, 2, 3}}"
    );
    assert_eq!(
        eval::amount(&GY_CREATURE_DISTINCT_MV, engine.state(), me(), aven, None),
        2,
        "only creature cards in graveyard contribute to filtered count: {{1, 2}}"
    );
    assert_eq!(
        (
            engine.state().object(aven).unwrap().characteristics().power,
            engine
                .state()
                .object(aven)
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::DEATHTOUCH),
        ),
        (Some(1), false),
        "4 distinct MVs does not enable Aven Heartstabber's bonus"
    );

    // Add MV 4 card -> 5 distinct mana values {0, 1, 2, 3, 4}: +2/+2 and deathtouch turn ON!
    engine
        .state
        .move_object(
            hand_mv4,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    assert_eq!(
        (
            engine.state().object(aven).unwrap().characteristics().power,
            engine
                .state()
                .object(aven)
                .unwrap()
                .characteristics()
                .toughness,
            engine
                .state()
                .object(aven)
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::DEATHTOUCH),
        ),
        (Some(3), Some(3), true),
        "5 distinct graveyard MVs enables +2/+2 and deathtouch"
    );

    // Exile the land (unique MV 0) -> drops back to 4 distinct MVs -> bonus turns OFF!
    engine
        .state
        .move_object(
            hand_mv0,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    assert_eq!(
        (
            engine.state().object(aven).unwrap().characteristics().power,
            engine
                .state()
                .object(aven)
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::DEATHTOUCH),
        ),
        (Some(1), false),
        "exiling unique MV 0 drops count to 4 and turns off Aven bonus"
    );
}

// =========================================================================
// 4. Asmira, Holy Avenger (creatures put into your graveyard from battlefield)
// =========================================================================

#[test]
fn wave15a_asmira_creatures_died_into_your_graveyard() {
    const ASMIRA: u32 = C_BASE + 30;
    const BEAR: u32 = C_BASE + 31;
    const TOKEN_MAKER: u32 = C_BASE + 32;
    const KILLER: u32 = C_BASE + 33;

    static YOUR_CREATURES_DIED: Amount = Amount::EnteredThisTurn {
        filter: &Filter::CREATURE,
        from: Some(ZoneRef::Battlefield),
        to: ZoneRef::Graveyard,
        owner_or_ctrl: PlayerRel::You,
        exclude_source: false,
    };

    static ASMIRA_AB: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::AddCounterSource {
            kind: CounterKind::P1P1,
            amount: YOUR_CREATURES_DIED,
        }]
    )];
    static SAPROLING: TokenDef = TokenDef {
        name: "Saproling",
        colors: ColorSet::of(Color::Green),
        types: TypeSet::CREATURE,
        power: Some(1),
        toughness: Some(1),
        ..TokenDef::DEFAULT
    };
    static TOKEN_MAKER_AB: &[AbilityDef] =
        &[free(&[Effect::CreateToken { token: &SAPROLING }], None)];
    static KILLER_AB: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            ASMIRA,
            creature_face("Asmira Proxy", "{2}{G}{W}", 2, 3),
            KeywordSet::FLYING,
            ASMIRA_AB,
        ),
        card(
            BEAR,
            creature_face("Forest Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_MAKER,
            face("Saproling Vat", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TOKEN_MAKER_AB,
        ),
        card(
            KILLER,
            face("Doom Device", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            KILLER_AB,
        ),
    ];

    let mut engine = bench(
        1513,
        cards,
        [
            Seat::with(&[ASMIRA, BEAR, BEAR, TOKEN_MAKER, KILLER]).holding(&[BEAR]),
            Seat::with(&[BEAR]),
        ],
    );
    to_main(&mut engine, me());

    let asmira = the(&engine, ZoneLocation::Battlefield, ASMIRA);
    let killer = the(&engine, ZoneLocation::Battlefield, KILLER);
    let maker = the(&engine, ZoneLocation::Battlefield, TOKEN_MAKER);
    let my_bears: Vec<ObjectId> = objects(&engine, ZoneLocation::Battlefield, BEAR)
        .into_iter()
        .filter(|&id| engine.state().object(id).unwrap().controller == me())
        .collect();
    let opp_bear = objects(&engine, ZoneLocation::Battlefield, BEAR)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == them())
        .unwrap();
    let hand_bear = the(&engine, ZoneLocation::Hand(me()), BEAR);

    // 1. Discard a creature from hand -> Graveyard (from: Hand, NOT Battlefield) -> must NOT count.
    engine
        .state
        .move_object(
            hand_bear,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    // 2. Exile our second bear from battlefield -> Exile (to: Exile, NOT Graveyard) -> must NOT count.
    engine
        .state
        .move_object(
            my_bears[1],
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    // 3. Destroy opponent's bear (enters opponent's graveyard) -> must NOT count for PlayerRel::You.
    activate(&mut engine, me(), killer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());

    // 4. Create a Saproling token, then destroy BOTH our bear #0 and our Saproling token -> 2 count!
    activate(&mut engine, me(), maker, 0);
    settle_stack(&mut engine, me());
    let saproling = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == me() && !o.is_card()
        })
        .expect("saproling token");

    for victim in [my_bears[0], saproling] {
        activate(&mut engine, me(), killer, 0);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![victim],
                    players: vec![],
                },
            )
            .unwrap();
        settle_stack(&mut engine, me());
    }

    // Advance to End Step: Asmira triggers and receives exactly 2 +1/+1 counters.
    advance_to_step(&mut engine, me(), Step::End);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(asmira)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2,
        "Asmira counts exactly 1 creature card + 1 creature token put into your graveyard from battlefield"
    );

    // Advance through opponent's end step with no deaths: still 2 counters.
    advance_to_step(&mut engine, them(), Step::End);
    settle_stack(&mut engine, them());
    assert_eq!(
        engine
            .state()
            .object(asmira)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2,
        "no creatures died on opponent's turn -> Asmira stays at 2 counters"
    );
}

// =========================================================================
// 5. Akuta, Born of Ash (HighestOpponentHandSize + all 6 AmountCmp ops)
// =========================================================================

#[test]
fn wave15a_akuta_highest_opponent_hand_size_and_all_cmp_ops() {
    const AKUTA: u32 = C_BASE + 40;
    const FILLER: u32 = C_BASE + 41;

    static YOUR_HAND: Amount = Amount::HandSize(PlayerRel::You);
    static HIGHEST_OPP_HAND: Amount = Amount::HighestOpponentHandSize;
    static AKUTA_COND: Condition = Condition::AmountCompare {
        left: &YOUR_HAND,
        op: AmountCmp::GreaterThan,
        right: &HIGHEST_OPP_HAND,
    };

    static AKUTA_AB: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::ReturnSourceFromGraveyard {
            face: None,
            tapped: false,
            owner_control: true,
            counters: None,
        }],
        zone = TriggerZone::Graveyard,
        condition = Some(AKUTA_COND)
    )];

    let cards = vec![
        card(
            AKUTA,
            creature_face("Akuta Proxy", "{2}{B}{B}", 3, 2),
            KeywordSet::HASTE,
            AKUTA_AB,
        ),
        card(
            FILLER,
            creature_face("Filler Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    // 3-player game (CR 103.8a: starting player draws on turn 1 in multiplayer!):
    // Seat 0 (you): 4 starting cards + 1 turn-1 draw = 5, minus Akuta -> 4 cards in hand.
    // Seat 1 (opp1): 2 cards in hand.
    // Seat 2 (opp2): 4 cards in hand -> HighestOpponentHandSize == 4!
    let mut engine = bench(
        1514,
        cards,
        [
            Seat::with(&[]).holding(&[AKUTA, FILLER, FILLER, FILLER]),
            Seat::with(&[]).holding(&[FILLER, FILLER]),
            Seat::with(&[]).holding(&[FILLER, FILLER, FILLER, FILLER]),
        ],
    );
    to_main(&mut engine, me());

    let akuta = the(&engine, ZoneLocation::Hand(me()), AKUTA);
    engine
        .state
        .move_object(
            akuta,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    // You now have 4 cards, Opp1 has 2, Opp2 has 4.
    assert_eq!(
        eval::amount(&HIGHEST_OPP_HAND, engine.state(), me(), akuta, None),
        4,
        "highest opponent hand size is max(2, 4) = 4"
    );

    // Verify all 6 AmountCmp operators when left == 4, right == 4:
    for (op, expected) in [
        (AmountCmp::LessThan, false),
        (AmountCmp::AtMost, true),
        (AmountCmp::Equal, true),
        (AmountCmp::NotEqual, false),
        (AmountCmp::AtLeast, true),
        (AmountCmp::GreaterThan, false),
    ] {
        let cond = Condition::AmountCompare {
            left: &YOUR_HAND,
            op,
            right: &HIGHEST_OPP_HAND,
        };
        assert_eq!(
            eval::condition_holds(engine.state(), me(), akuta, cond),
            expected,
            "4 {op:?} 4 should be {expected}"
        );
    }

    // Advance to our next upkeep (remember Opp1 and Opp2 each draw 1 card on their turns,
    // so Opp2 will have 5 cards while we still have 4 -> tied/less, Akuta does NOT return!).
    advance_to_step(&mut engine, me(), Step::Upkeep);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(akuta).unwrap().zone,
        Zone::Graveyard,
        "Akuta must NOT return when you don't strictly exceed highest opponent's hand"
    );

    // Now have Opp2 lose the game: HighestOpponentHandSize drops to Opp1's hand (3 cards)!
    // Meanwhile our hand is 4 cards (4 > 3), and on our next upkeep we strictly beat Opp1!
    let opp2 = PlayerId::new(2);
    engine.state.players[usize::from(opp2.get())].life = 0;
    // Also draw 2 extra cards for ourselves so even after Opp1's next draw step we lead strictly.
    engine.state.draw_cards(me(), 2);
    advance_to_step(&mut engine, me(), Step::Main);
    assert!(
        engine.state().has_left(opp2),
        "eliminated opponent 2 must have left the game"
    );
    let opp1_hand = engine.state().zones.list(ZoneLocation::Hand(them())).len() as u32;
    assert_eq!(
        eval::amount(&HIGHEST_OPP_HAND, engine.state(), me(), akuta, None),
        opp1_hand,
        "eliminated opponent is excluded from HighestOpponentHandSize"
    );

    // Verify all 6 AmountCmp operators when left > right (my_hand > opp1_hand):
    for (op, expected) in [
        (AmountCmp::LessThan, false),
        (AmountCmp::AtMost, false),
        (AmountCmp::Equal, false),
        (AmountCmp::NotEqual, true),
        (AmountCmp::AtLeast, true),
        (AmountCmp::GreaterThan, true),
    ] {
        let cond = Condition::AmountCompare {
            left: &YOUR_HAND,
            op,
            right: &HIGHEST_OPP_HAND,
        };
        assert_eq!(
            eval::condition_holds(engine.state(), me(), akuta, cond),
            expected,
            "strictly greater {op:?} should be {expected}"
        );
    }

    // Next upkeep: you strictly exceed Opp1's hand -> Akuta triggers and returns!
    advance_to_step(&mut engine, me(), Step::Upkeep);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(akuta).unwrap().zone,
        Zone::Battlefield,
        "Akuta returns at upkeep when your hand exceeds highest active opponent's hand"
    );
}

// =========================================================================
// 6. Blitzball Stadium (DistinctCounterKindsOnSource, including LKI)
// =========================================================================

#[test]
fn wave15a_blitzball_stadium_distinct_counter_kinds_and_lki() {
    const STADIUM: u32 = C_BASE + 50;

    static SAC_PARTS: [CostPart; 1] = [CostPart::SacrificeSelf];
    static SAC_COST: Cost = Cost {
        mana: ManaCost::ZERO,
        parts: &SAC_PARTS,
    };

    static STADIUM_AB: &[AbilityDef] = &[
        free(
            &[Effect::GainLife {
                amount: Amount::DistinctCounterKindsOnSource,
            }],
            None,
        ),
        paid(
            SAC_COST,
            &[Effect::GainLife {
                amount: Amount::DistinctCounterKindsOnSource,
            }],
            None,
        ),
    ];

    let cards = vec![card(
        STADIUM,
        face("Blitzball Stadium Proxy", "{3}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        STADIUM_AB,
    )];

    let mut engine = bench(1515, cards, [Seat::with(&[STADIUM]), Seat::with(&[])]);
    to_main(&mut engine, me());

    let stadium = the(&engine, ZoneLocation::Battlefield, STADIUM);

    // 0 counters -> 0 distinct kinds.
    assert_eq!(
        eval::amount(
            &Amount::DistinctCounterKindsOnSource,
            engine.state(),
            me(),
            stadium,
            None,
        ),
        0
    );

    // 3 +1/+1 counters -> still only 1 distinct kind!
    crate::replacement::put_counters_by(&mut engine.state, me(), stadium, CounterKind::P1P1, 3);
    assert_eq!(
        eval::amount(
            &Amount::DistinctCounterKindsOnSource,
            engine.state(),
            me(),
            stadium,
            None,
        ),
        1,
        "multiple counters of the same kind count as 1 distinct kind"
    );

    // Add 1 Charge + 2 Shield counters -> 3 distinct kinds!
    crate::replacement::put_counters_by(&mut engine.state, me(), stadium, CounterKind::Charge, 1);
    crate::replacement::put_counters_by(&mut engine.state, me(), stadium, CounterKind::Shield, 2);
    refresh_offer(&mut engine);

    let before = life(&engine, me());
    activate(&mut engine, me(), stadium, 0);
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()) - before,
        3,
        "P1P1 + Charge + Shield = 3 distinct counter kinds"
    );

    // Remove the lone Charge counter -> drops to 2 distinct kinds!
    let removed =
        crate::replacement::remove_counters(&mut engine.state, stadium, CounterKind::Charge, 1);
    assert_eq!(removed, 1);
    assert_eq!(
        eval::amount(
            &Amount::DistinctCounterKindsOnSource,
            engine.state(),
            me(),
            stadium,
            None,
        ),
        2,
        "removing all Charge counters leaves 2 distinct kinds (P1P1, Shield)"
    );

    // Sacrifice stadium to pay ability 1: LKI preserves the 2 distinct counter kinds (CR 608.2h)!
    refresh_offer(&mut engine);
    let before_sac = life(&engine, me());
    activate(&mut engine, me(), stadium, 1);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(stadium).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        life(&engine, me()) - before_sac,
        2,
        "sacrificed source still reports 2 distinct counter kinds via LKI (CR 608.2h)"
    );
}

// =========================================================================
// 7. Beast counter placed on source this turn (by_you: false vs true)
// =========================================================================

#[test]
fn wave15a_beast_counters_placed_on_source_this_turn() {
    const BEAST: u32 = C_BASE + 60;
    const OPP_COUNTERER: u32 = C_BASE + 61;
    const MY_COUNTERER: u32 = C_BASE + 62;
    const CHARGE_COUNTERER: u32 = C_BASE + 63;
    const BLINKER: u32 = C_BASE + 64;

    static BEAST_AB: &[AbilityDef] = &[
        // Ability 0: static +2/+0 if ANYONE placed a +1/+1 counter on this this turn.
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(2, 0),
            condition = Some(Condition::CountersPlacedOnSourceThisTurn {
                kind: CounterKind::P1P1,
                by_you: false,
            })
        ),
        // Ability 1: static +0/+3 if YOU placed a +1/+1 counter on this this turn.
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(0, 3),
            condition = Some(Condition::CountersPlacedOnSourceThisTurn {
                kind: CounterKind::P1P1,
                by_you: true,
            })
        ),
    ];
    static PUT_P1P1: &[AbilityDef] = &[free(
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static PUT_CHARGE: &[AbilityDef] = &[free(
        &[Effect::AddCounter {
            kind: CounterKind::Charge,
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static BLINK_AB: &[AbilityDef] = &[free(
        &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            BEAST,
            creature_face("Counter Beast Proxy", "{2}{G}", 2, 2),
            KeywordSet::EMPTY,
            BEAST_AB,
        ),
        card(
            OPP_COUNTERER,
            face("Opp Boost", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PUT_P1P1,
        ),
        card(
            MY_COUNTERER,
            face("My Boost", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PUT_P1P1,
        ),
        card(
            CHARGE_COUNTERER,
            face("Charge Boost", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PUT_CHARGE,
        ),
        card(
            BLINKER,
            face("Blink Ring", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BLINK_AB,
        ),
    ];

    let mut engine = bench(
        1516,
        cards,
        [
            Seat::with(&[BEAST, MY_COUNTERER, CHARGE_COUNTERER, BLINKER]),
            Seat::with(&[OPP_COUNTERER]),
        ],
    );
    to_main(&mut engine, me());
    assert!(
        engine.state().per_turn.counter_placers.tracked,
        "counter_placers tracking must activate for by_you: true"
    );

    let beast = the(&engine, ZoneLocation::Battlefield, BEAST);
    let my_boost = the(&engine, ZoneLocation::Battlefield, MY_COUNTERER);
    let opp_boost = the(&engine, ZoneLocation::Battlefield, OPP_COUNTERER);
    let charge_boost = the(&engine, ZoneLocation::Battlefield, CHARGE_COUNTERER);
    let blinker = the(&engine, ZoneLocation::Battlefield, BLINKER);

    // Initially neither condition holds: Beast is 2/2.
    assert_eq!(
        (
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .toughness,
        ),
        (Some(2), Some(2))
    );

    // Placing a Charge counter (wrong kind) leaves both P1P1 conditions FALSE: still 2/2.
    activate(&mut engine, me(), charge_boost, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        (
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .toughness,
        ),
        (Some(2), Some(2)),
        "wrong counter kind must not satisfy P1P1 placed condition"
    );

    // Opponent puts a +1/+1 counter on Beast:
    // base 2/2 + 1/1 counter + 2/0 (by_you: false) = 5/3; `by_you: true` (+0/+3) stays FALSE!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ActivateAbility {
                source: opp_boost,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        (
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .toughness,
        ),
        (Some(5), Some(3)),
        "opponent's +1/+1 counter triggers by_you:false (+2/+0) but NOT by_you:true (+0/+3)"
    );

    // Now YOU put a +1/+1 counter on Beast:
    // base 2/2 + 2/2 counters + 2/0 (by_you: false) + 0/3 (by_you: true) = 6/7!
    activate(&mut engine, me(), my_boost, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        (
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .toughness,
        ),
        (Some(6), Some(7)),
        "your +1/+1 counter enables by_you:true (+0/+3) as well"
    );

    // Removing all +1/+1 counters still keeps BOTH historical conditions TRUE this turn:
    // base 2/2 + 2/0 + 0/3 = 4/5!
    crate::replacement::remove_counters(&mut engine.state, beast, CounterKind::P1P1, 2);
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    assert_eq!(
        (
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .toughness,
        ),
        (Some(4), Some(5)),
        "removing counters later in the same turn preserves historical placement fact"
    );

    // Blinking Beast makes it a new object (CR 400.7): both conditions reset to FALSE (2/2)!
    refresh_offer(&mut engine);
    activate(&mut engine, me(), blinker, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        (
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .toughness,
        ),
        (Some(2), Some(2)),
        "blinked permanent is a new object with no counters placed this turn (CR 400.7)"
    );

    // Put a counter again, then advance to next turn: resets to FALSE (base 2/2 + 1/1 counter = 3/3)!
    activate(&mut engine, me(), my_boost, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(beast)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
    advance_to_next_main(&mut engine, me());
    assert_eq!(
        (
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(beast)
                .unwrap()
                .characteristics()
                .toughness,
        ),
        (Some(3), Some(3)),
        "placement flags reset on next turn while counter itself remains (2+1 / 2+1)"
    );
}

// =========================================================================
// 8. CreaturesAttackedThisTurn (filter, LKI after death, extra combat dedup)
// =========================================================================

#[test]
fn wave15a_creatures_attacked_this_turn_filter_lki_and_dedup() {
    const GOBLIN: u32 = C_BASE + 70;
    const BEAR: u32 = C_BASE + 71;
    const BLOCKER: u32 = C_BASE + 72;
    const EXTRA_COMBAT: u32 = C_BASE + 73;

    static GOBLIN_SUBS: &[SubtypeId] = &[baylee_core::generated::subtypes::creature::GOBLIN];
    static FILTER_GOBLIN: Filter =
        Filter::HasSubtype(baylee_core::generated::subtypes::creature::GOBLIN);

    static EXTRA_COMBAT_AB: &[AbilityDef] = &[free(
        &[
            Effect::UntapAll {
                filter: &Filter::YOUR_CREATURE,
            },
            Effect::AdditionalCombatPhase {
                followed_by_main_phase: false,
            },
        ],
        None,
    )];

    let cards = vec![
        card(
            GOBLIN,
            FaceDef {
                subtypes: GOBLIN_SUBS,
                ..creature_face("Goblin Raider", "{R}", 1, 1)
            },
            KeywordSet::HASTE,
            &[],
        ),
        card(
            BEAR,
            creature_face("Sturdy Bear", "{1}{G}", 2, 2),
            KeywordSet::HASTE.union(KeywordSet::VIGILANCE),
            &[],
        ),
        card(
            BLOCKER,
            creature_face("Wall Crusher", "{2}{W}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            EXTRA_COMBAT,
            face("Relentless Horn", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            EXTRA_COMBAT_AB,
        ),
    ];

    let mut engine = bench(
        1517,
        cards,
        [
            Seat::with(&[GOBLIN, BEAR, EXTRA_COMBAT]),
            Seat::with(&[BLOCKER]),
        ],
    );
    to_main(&mut engine, me());

    let goblin = the(&engine, ZoneLocation::Battlefield, GOBLIN);
    let bear = the(&engine, ZoneLocation::Battlefield, BEAR);
    let blocker = the(&engine, ZoneLocation::Battlefield, BLOCKER);
    let horn = the(&engine, ZoneLocation::Battlefield, EXTRA_COMBAT);

    // Queue an extra combat phase during first main.
    activate(&mut engine, me(), horn, 0);
    settle_stack(&mut engine, me());

    // Combat #1: declare Goblin and Bear attacking; opponent blocks and kills Goblin!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (goblin, Defender::Player(them())),
                    (bear, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == them()),
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, goblin)],
            },
        )
        .unwrap();

    // Let combat #1 finish and enter combat #2's DeclareAttackers.
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    assert_eq!(
        engine.state().object(goblin).unwrap().zone,
        Zone::Graveyard,
        "Goblin died in combat #1"
    );

    // Bear has vigilance so it can attack a SECOND time in combat #2!
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(them()))],
            },
        )
        .unwrap();

    // Advance to postcombat main.
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });

    let all_attacked = Amount::CreaturesAttackedThisTurn {
        filter: &Filter::CREATURE,
        controller: PlayerRel::You,
    };
    let goblins_attacked = Amount::CreaturesAttackedThisTurn {
        filter: &FILTER_GOBLIN,
        controller: PlayerRel::You,
    };
    let opp_attacked = Amount::CreaturesAttackedThisTurn {
        filter: &Filter::CREATURE,
        controller: PlayerRel::Opponent,
    };

    assert_eq!(
        eval::amount(&all_attacked, engine.state(), me(), horn, None),
        2,
        "Bear attacking twice counts once, dead Goblin still counts via LKI -> total 2"
    );
    assert_eq!(
        eval::amount(&goblins_attacked, engine.state(), me(), horn, None),
        1,
        "dead Goblin still matches Goblin subtype filter via LKI"
    );
    assert_eq!(
        eval::amount(&opp_attacked, engine.state(), me(), horn, None),
        0,
        "opponent declared no attackers"
    );

    // Next turn resets cleanly to 0.
    advance_to_next_main(&mut engine, me());
    assert_eq!(
        eval::amount(&all_attacked, engine.state(), me(), horn, None),
        0
    );
}

// =========================================================================
// 9. snapshot_hash() & loop_signature() determinism and tracking isolation
// =========================================================================

#[test]
fn wave15a_snapshot_hash_and_loop_signature_determinism() {
    const TRACKER_CARD: u32 = C_BASE + 80;
    const INSTANT_CARD: u32 = C_BASE + 81;
    const VANILLA_CARD: u32 = C_BASE + 82;

    static ZONE_AMT: Amount = Amount::EnteredThisTurn {
        filter: &Filter::NONLAND,
        from: None,
        to: ZoneRef::Battlefield,
        owner_or_ctrl: PlayerRel::You,
        exclude_source: false,
    };
    static CAST_AMT: Amount = Amount::SpellsCastMatchingThisTurn {
        filter: &Filter::INSTANT,
        caster: PlayerRel::You,
    };
    static ZERO: Amount = Amount::Fixed(0);
    static TRACKER_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(1, 0),
            condition = Some(Condition::AmountCompare {
                left: &ZONE_AMT,
                op: AmountCmp::GreaterThan,
                right: &ZERO,
            })
        ),
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(0, 1),
            condition = Some(Condition::AmountCompare {
                left: &CAST_AMT,
                op: AmountCmp::GreaterThan,
                right: &ZERO,
            })
        ),
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::VIGILANCE),
            condition = Some(Condition::CountersPlacedOnSourceThisTurn {
                kind: CounterKind::P1P1,
                by_you: true,
            })
        ),
    ];
    static NOOP_SPELL: &[AbilityDef] = &[spell!(&[Effect::gain_life(1)])];

    let run_once = || -> (u64, u64) {
        let cards = vec![
            card(
                TRACKER_CARD,
                creature_face("Tracker", "{1}{W}", 2, 2),
                KeywordSet::EMPTY,
                TRACKER_AB,
            ),
            card(
                INSTANT_CARD,
                face("Spark", "{R}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                NOOP_SPELL,
            ),
            card(
                VANILLA_CARD,
                creature_face("Bear", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ];
        let mut engine = bench(
            1518,
            cards,
            [
                Seat::with(&[TRACKER_CARD]).holding(&[INSTANT_CARD, VANILLA_CARD]),
                Seat::with(&[]),
            ],
        );
        to_main(&mut engine, me());
        add_pool(&mut engine, me(), ManaColor::Red, 1);
        add_pool(&mut engine, me(), ManaColor::Green, 2);
        let spark = the(&engine, ZoneLocation::Hand(me()), INSTANT_CARD);
        engine
            .apply(me(), PlayerAction::CastSpell { card: spark })
            .unwrap();
        settle_stack(&mut engine, me());
        let bear = the(&engine, ZoneLocation::Hand(me()), VANILLA_CARD);
        engine
            .apply(me(), PlayerAction::CastSpell { card: bear })
            .unwrap();
        settle_stack(&mut engine, me());
        let tracker = the(&engine, ZoneLocation::Battlefield, TRACKER_CARD);
        crate::replacement::put_counters_by(&mut engine.state, me(), tracker, CounterKind::P1P1, 1);
        (
            engine.state().snapshot_hash(),
            engine.state().loop_signature(),
        )
    };

    // Determinism: two independent runs produce bit-identical hashes.
    let first = run_once();
    let second = run_once();
    assert_eq!(
        first, second,
        "snapshot_hash and loop_signature must be deterministic"
    );

    // Sensitivity when tracked vs zero perturbation when untracked.
    let cards = vec![
        card(
            VANILLA_CARD,
            creature_face("Plain Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            INSTANT_CARD,
            face("Plain Spark", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            NOOP_SPELL,
        ),
    ];
    let mut engine = bench(
        1519,
        cards,
        [
            Seat::with(&[VANILLA_CARD]).holding(&[INSTANT_CARD]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    assert!(!engine.state().per_turn.zone_changes.tracked);
    assert!(!engine.state().per_turn.spells_cast_log.tracked);
    assert!(!engine.state().per_turn.counter_placers.tracked);

    let bear = the(&engine, ZoneLocation::Battlefield, VANILLA_CARD);
    let base_snap = engine.state().snapshot_hash();
    let base_loop = engine.state().loop_signature();

    // Mutating untracked PerTurnTracked vectors must NOT move either digest.
    let mut untracked = engine.state.clone();
    let obj_arc = crate::state::freeze_object(untracked.object(bear).unwrap());
    let src_ref = untracked.source_identity(bear).unwrap();
    untracked
        .per_turn
        .zone_changes
        .items
        .push(crate::state::ZoneChangeRecord {
            object: src_ref,
            from: Some(Zone::Stack),
            to: Zone::Battlefield,
            owner: me(),
            controller: me(),
            snapshot: Some(std::sync::Arc::clone(&obj_arc)),
        });
    untracked
        .per_turn
        .spells_cast_log
        .items
        .push(crate::state::SpellCastRecord {
            spell: src_ref,
            caster: me(),
            snapshot: std::sync::Arc::clone(&obj_arc),
        });
    untracked
        .per_turn
        .counter_placers
        .items
        .push(crate::state::CounterPlacerRecord {
            target: src_ref,
            kind: CounterKind::P1P1,
            by: me(),
        });
    assert_eq!(
        untracked.snapshot_hash(),
        base_snap,
        "untracked PerTurnTracked fields must not perturb snapshot_hash"
    );
    assert_eq!(
        untracked.loop_signature(),
        base_loop,
        "untracked PerTurnTracked fields must not perturb loop_signature"
    );

    // Turning tracking ON makes each field participate in BOTH snapshot_hash and loop_signature.
    let mut tracked_zone = untracked.clone();
    tracked_zone.per_turn.zone_changes.tracked = true;
    assert_ne!(tracked_zone.snapshot_hash(), base_snap);
    assert_ne!(tracked_zone.loop_signature(), base_loop);

    let mut tracked_cast = untracked.clone();
    tracked_cast.per_turn.spells_cast_log.tracked = true;
    assert_ne!(tracked_cast.snapshot_hash(), base_snap);
    assert_ne!(tracked_cast.loop_signature(), base_loop);

    let mut tracked_placer = untracked.clone();
    tracked_placer.per_turn.counter_placers.tracked = true;
    assert_ne!(tracked_placer.snapshot_hash(), base_snap);
    assert_ne!(tracked_placer.loop_signature(), base_loop);
}
