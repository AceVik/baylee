//! Beat 18 / Engine Wave 14e deterministic rule tests (ZERO WIRE):
//! 1. Sacrifice memory & LKI stats (`RememberSacrificed`):
//!    - `Effect::SacrificeAnyNumber { who, filter }` via existing
//!      `Pending::ChooseCards` (`min: 0`, `max: all`), 0 / partial / all picks,
//!      APNAP multiplayer ordering (`CR 101.4`, `CR 603.10a`),
//!    - `ThisWay::SacrificedCount` (including under Rest in Peace exile
//!      replacement!), `Amount::SacrificedTotalPower` (clamping negative power
//!      at 0 per `CR 107.1b`), `Amount::SacrificedTotalToughness`,
//!      `Amount::SacrificedTotalManaValue`, `Amount::SacrificedDistinctColors`,
//!    - `SubjectContext::found` populated on all `Sacrifice*` effects
//!      (`SacrificeAnyNumber`, `SacrificeFilter`, `PlayerMayPayCostThen` with
//!      `CostPart::Sacrifice`), preserving pre-departure LKI (`CR 608.2h`).
//! 2. Restricted inline modal choices (`Effect::ChooseOneRestricted` &
//!    `ModalRestriction::{None, OncePerTurn, OncePerPermanent}`, `CR 700.2i`):
//!    - *Demonic Pact* 4-turn lifecycle filtering used modes while preserving
//!      original `CastModeKind::Mode(orig_idx)`, out-of-range index refusal,
//!      exhaustion (0 modes left does nothing without prompting), blink reset
//!      (`CR 400.7`), in-flight departure LKI (`CR 608.2h`), `OncePerTurn`
//!      reset across turns and mid-turn blink, and `snapshot_hash` sensitivity.
//! 3. Once-per-permanent activations & Power-Up cost reduction (`PutCounter.PowerUp`):
//!    - `ActivationLimit::OncePerPermanent` (`once_per_permanent = true`) and
//!      `CostReduction::OwnManaCostIfEnteredThisTurn`
//!      (`reduce_by_own_mana_cost_if_entered_this_turn = true`):
//!    - Printed/copiable mana-cost reduction (`CR 118.7a–g`, colored + generic +
//!      excess colored spillover) on entry turn even when cheated onto the
//!      battlefield, full cost on subsequent turns, cross-turn single-use lock,
//!      independent tracking of two `OncePerPermanent` abilities on the same
//!      permanent, blink reset (`CR 400.7`), and `snapshot_hash` sensitivity.
//! 4. Created tokens bound in `SubjectContext::found` (`Token.RememberTokens`):
//!    - `CreateToken*` & `CreateTokenCopy*` (including under Doubling Season)
//!      populating `SubjectContext::found` while preserving `ThisWay` counters
//!      and outer `TargetSpec::EventObject`, enabling `UntapFound`,
//!      `RepeatEachObject { only_found: true }` (`Goad`, `AddCounterCreatedTokens`,
//!      `AtNextEndStepAboutSubject` exile), and `ChooseCardEx { zone: ChooseZone::Found }`.

#![allow(clippy::too_many_lines)] // comprehensive multi-scenario rule tests

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::object::Status;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{activated, cost, spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, ChooseMove, ChooseZone, CopyCleanup, CostPart, CounterKind, DamagedByScope, Duration,
    Effect, Filter, ModalRestriction, Modifier, PlayerRel, ReplacementRule, StepKind, TargetSpec,
    ThisWay, TokenDef, Trigger, ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::mana::{ManaColor, ManaPool};

const C0: u32 = 99_950;
const C1: u32 = 99_951;
const C2: u32 = 99_952;
const C3: u32 = 99_953;
const C4: u32 = 99_954;
const C5: u32 = 99_955;

static GOBLIN_1_1_RED: TokenDef = TokenDef {
    name: "Goblin",
    colors: ColorSet::from_slice(&[Color::Red]),
    types: TypeSet::CREATURE,
    subtypes: &[baylee_core::generated::subtypes::creature::GOBLIN],
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};

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

fn cast_hand(engine: &mut Bench, seat: PlayerId, card_id: ObjectId) {
    engine
        .apply(seat, PlayerAction::CastSpell { card: card_id })
        .expect("cast accepted");
}

fn activate_index(engine: &mut Bench, seat: PlayerId, source: ObjectId, ability_index: u32) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("activation accepted");
}

fn can_activate_index(engine: &Bench, source: ObjectId, ability_index: u32) -> bool {
    let (_, legal) = offer(engine);
    legal
        .abilities
        .iter()
        .any(|&(s, idx)| s == source && idx == ability_index)
}

fn is_goaded_by(engine: &Bench, creature: ObjectId, goader: PlayerId) -> bool {
    engine.state().object(creature).is_some_and(|obj| {
        engine.state().goaded.iter().any(|entry| {
            entry.creature.object == creature
                && entry.creature.version == obj.version
                && entry.goader == goader
        })
    })
}

fn pass_to_choice(engine: &mut Bench) {
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => return,
        }
    }
    panic!("never reached non-priority choice: {:?}", engine.pending());
}

fn hand_len(engine: &Bench, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

fn creature_card(
    index: u32,
    name: &'static str,
    mana_cost: &'static str,
    power: i16,
    toughness: i16,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    card(
        index,
        creature_face(name, mana_cost, power, toughness),
        KeywordSet::EMPTY,
        abilities,
    )
}

// =========================================================================
// 1. SacrificeAnyNumber + LKI totals + SubjectContext::found
// =========================================================================

#[test]
fn sacrifice_any_number_zero_and_partial_report_lki_totals_and_found() {
    static SAC_PROBE_EFFECTS: &[Effect] = &[
        Effect::SacrificeAnyNumber {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        },
        Effect::GainLife {
            amount: Amount::ThisWay(ThisWay::SacrificedCount),
        },
        Effect::LoseLife {
            target: PlayerRel::Opponent,
            amount: Amount::SacrificedTotalPower,
        },
        Effect::DrawCards {
            amount: Amount::SacrificedTotalManaValue,
        },
        Effect::AddCounterFilter {
            filter: &Filter::ARTIFACT,
            kind: CounterKind::P1P1,
            amount: Amount::SacrificedTotalToughness,
        },
        Effect::AddCounterFilter {
            filter: &Filter::ARTIFACT,
            kind: CounterKind::Charge,
            amount: Amount::SacrificedDistinctColors,
        },
        // Offer ONLY the creatures sacrificed this way via `ChooseZone::Found` and exile them.
        Effect::ChooseCardEx {
            chooser: PlayerRel::You,
            zone: ChooseZone::Found,
            filter: &Filter::Any,
            min: 0,
            max: Amount::Fixed(10),
            reveal_chosen: false,
            movement: ChooseMove::Exile,
            not_chosen_movement: ChooseMove::None,
            then: &[],
            for_unchosen: &[],
        },
    ];
    static PROBE_ABILITIES: &[AbilityDef] = &[free(SAC_PROBE_EFFECTS, None)];
    static ANTHEM_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]),
        Modifier::ModifyPT(1, 1)
    )];

    // C0: White/Blue 3/5 (MV 4) -> with +1/+1 anthem is 4/6
    // C1: Red/Green -3/2 (MV 2) -> with +1/+1 anthem is -2/3 -> negative power clamps to 0 (CR 107.1b)
    // C2: Colorless 2/1 (MV 1) -> left unsacrificed in partial test
    // C3: Pre-existing graveyard card (must NOT be exiled by `only_found: true`)
    // C4: Anthem enchantment
    // C5: Marker artifact + free probe
    let cards = vec![
        creature_card(C0, "Azorius Giant", "{2}{W}{U}", 3, 5, &[]),
        creature_card(C1, "Gruul Reversed", "{R}{G}", -3, 2, &[]),
        creature_card(C2, "Colorless Construct", "{1}", 2, 1, &[]),
        creature_card(C3, "Old Corpse", "{B}", 1, 1, &[]),
        card(
            C4,
            face("Anthem", "{2}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            ANTHEM_ABILITIES,
        ),
        card(
            C5,
            face("Sacrifice Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PROBE_ABILITIES,
        ),
    ];

    let mut engine = bench(
        14_001,
        cards,
        [
            Seat::with(&[C0, C1, C2, C4, C5]).holding(&[C3]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    // Move C3 to P0's graveyard beforehand so we verify `found` does NOT touch unrelated graveyard cards.
    let old_corpse = the(&engine, ZoneLocation::Hand(me()), C3);
    engine
        .state
        .move_object(
            old_corpse,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    let probe = the(&engine, ZoneLocation::Battlefield, C5);
    let giant = the(&engine, ZoneLocation::Battlefield, C0);
    let reversed = the(&engine, ZoneLocation::Battlefield, C1);
    let construct = the(&engine, ZoneLocation::Battlefield, C2);

    // --- Pass A: Choose 0 permanents ---
    let hand_before = hand_len(&engine, me());
    activate_index(&mut engine, me(), probe, 0);
    pass_to_choice(&mut engine);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(player, me());
    assert_eq!(min, 0);
    assert_eq!(max, 3);
    assert_eq!(options.len(), 3);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .expect("0-sacrifice choice accepted");
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 20, "0 sacrificed -> 0 life gained");
    assert_eq!(life(&engine, them()), 20, "0 sacrificed -> 0 power lost");
    assert_eq!(
        hand_len(&engine, me()),
        hand_before,
        "0 sacrificed -> 0 MV drawn"
    );
    assert_eq!(
        engine
            .state()
            .object(probe)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(
        engine
            .state()
            .object(probe)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );
    assert_eq!(
        engine.state().object(old_corpse).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "unrelated graveyard card untouched"
    );

    // --- Pass B: Choose 2 of 3 permanents (giant 4/6 W/U MV4 + reversed -2/3 R/G MV2) ---
    activate_index(&mut engine, me(), probe, 0);
    pass_to_choice(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![giant, reversed],
            },
        )
        .expect("partial sacrifice choice accepted");
    // Follow-up `ChooseCardEx { zone: ChooseZone::Found }` must offer ONLY `[giant, reversed]`.
    let Pending::ChooseCards {
        options: found_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards over SubjectContext::found, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        found_opts.len(),
        2,
        "only the 2 sacrificed cards are in found"
    );
    assert!(found_opts.contains(&giant));
    assert!(found_opts.contains(&reversed));
    assert!(!found_opts.contains(&old_corpse));
    assert!(!found_opts.contains(&construct));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: found_opts,
            },
        )
        .expect("exile found cards accepted");
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 22, "ThisWay::SacrificedCount == 2");
    assert_eq!(
        life(&engine, them()),
        18,
        "SacrificedTotalPower == 4 + (-2) = 2 (anthem LKI + signed sum per CR 107.1b)"
    );
    assert_eq!(
        hand_len(&engine, me()),
        hand_before + 6,
        "SacrificedTotalManaValue == 4 + 2 = 6"
    );
    assert_eq!(
        engine
            .state()
            .object(probe)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        9,
        "SacrificedTotalToughness == 6 + 3 = 9 (anthem LKI)"
    );
    assert_eq!(
        engine
            .state()
            .object(probe)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        4,
        "SacrificedDistinctColors == 4 (W, U, R, G)"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine.state().object(reversed).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine.state().object(construct).unwrap().zone,
        crate::zone::Zone::Battlefield,
        "unchosen creature stays on battlefield"
    );
    assert_eq!(
        engine.state().object(old_corpse).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "pre-existing graveyard card was not in SubjectContext::found"
    );
}

#[test]
fn sacrifice_any_number_apnap_simultaneous_lki_and_rest_in_peace_replacement() {
    static APNAP_SAC_EFFECTS: &[Effect] = &[
        Effect::SacrificeAnyNumber {
            who: PlayerRel::EachPlayer,
            filter: &Filter::CREATURE,
        },
        Effect::GainLife {
            amount: Amount::ThisWay(ThisWay::SacrificedCount),
        },
        Effect::LoseLife {
            target: PlayerRel::Opponent,
            amount: Amount::SacrificedTotalPower,
        },
        Effect::AddCounterFilter {
            filter: &Filter::ARTIFACT,
            kind: CounterKind::Charge,
            amount: Amount::SacrificedDistinctColors,
        },
    ];
    static APNAP_PROBE_ABILITIES: &[AbilityDef] = &[free(APNAP_SAC_EFFECTS, None)];
    static LORD_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]),
        Modifier::ModifyPT(2, 0)
    )];
    static RIP_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ExileInsteadOfGraveyard {
            filter: &Filter::Any,
            origin: None,
            owner: None,
            controller_lki: None,
            damaged_by: DamagedByScope::Any,
            link_to_source: false,
            counter: None,
        },
    )];

    // C0: White lord 2/2 (gives all your creatures +2/+0 -> itself is 4/2, buddy is 3/1)
    // C1: White buddy 1/1 (becomes 3/1 while lord is on battlefield)
    // C2: Black opponent creature 5/5
    // C3: Rest in Peace enchantment
    // C4: Probe artifact
    let cards = vec![
        creature_card(C0, "White Lord", "{1}{W}", 2, 2, LORD_ABILITIES),
        creature_card(C1, "White Buddy", "{W}", 1, 1, &[]),
        creature_card(C2, "Black Demon", "{2}{B}", 5, 5, &[]),
        card(
            C3,
            face("Rest in Peace Probe", "{1}{W}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            RIP_ABILITIES,
        ),
        card(
            C4,
            face("APNAP Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            APNAP_PROBE_ABILITIES,
        ),
    ];

    let mut engine = bench(
        14_002,
        cards,
        [Seat::with(&[C0, C1, C3, C4]), Seat::with(&[C2])],
    );
    to_main(&mut engine, me());

    let lord = the(&engine, ZoneLocation::Battlefield, C0);
    let buddy = the(&engine, ZoneLocation::Battlefield, C1);
    let demon = the(&engine, ZoneLocation::Battlefield, C2);
    let probe = the(&engine, ZoneLocation::Battlefield, C4);

    activate_index(&mut engine, me(), probe, 0);
    pass_to_choice(&mut engine);

    // APNAP: active player (P0) chooses first while ALL creatures are still on the battlefield.
    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        panic!("expected P0 ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(player, me());
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![lord, buddy],
            },
        )
        .unwrap();

    // Non-active player (P1) chooses next BEFORE P0's sacrifices happen (`CR 101.4`).
    assert_eq!(
        engine.state().object(lord).unwrap().zone,
        crate::zone::Zone::Battlefield,
        "P0's choices stay on battlefield until P1 finishes choosing"
    );
    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        panic!("expected P1 ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(player, them());
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![demon],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Under Rest in Peace, all 3 went to Exile, NOT graveyard:
    assert_eq!(
        engine.state().object(lord).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine.state().object(buddy).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine.state().object(demon).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        life(&engine, me()),
        23,
        "3 sacrificed even under Rest in Peace"
    );
    // Simultaneous LKI (`CR 603.10a` / `CR 608.2h`): lord 4/2 + buddy 3/1 + demon 5/5 = 12.
    assert_eq!(
        life(&engine, them()),
        8,
        "simultaneous LKI total power == 4 + 3 + 5 = 12"
    );
    assert_eq!(
        engine
            .state()
            .object(probe)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        2,
        "distinct colors across W + W + B == 2"
    );
}

#[test]
fn single_sacrifice_effects_record_lki_stats_and_found() {
    static DISCIPLE_EFFECTS: &[Effect] = &[
        Effect::SacrificeFilter {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        },
        Effect::GainLife {
            amount: Amount::SacrificedTotalToughness,
        },
        Effect::DrawCards {
            amount: Amount::SacrificedTotalPower,
        },
        Effect::AddCounterFilter {
            filter: &Filter::ARTIFACT,
            kind: CounterKind::Charge,
            amount: Amount::SacrificedTotalManaValue,
        },
        Effect::AddCounterFilter {
            filter: &Filter::ARTIFACT,
            kind: CounterKind::Energy,
            amount: Amount::SacrificedDistinctColors,
        },
    ];
    static MAY_SAC_THEN: &[Effect] = &[Effect::PlayerMayPayCostThen {
        player: PlayerRel::You,
        cost: &CostPart::Sacrifice(&Filter::CREATURE),
        effect: &Effect::Sequence(&[
            Effect::GainLife {
                amount: Amount::SacrificedTotalManaValue,
            },
            Effect::LoseLife {
                target: PlayerRel::Opponent,
                amount: Amount::SacrificedTotalPower,
            },
        ]),
    }];
    static DISCIPLE_PROBE_ABILITIES: &[AbilityDef] =
        &[free(DISCIPLE_EFFECTS, None), free(MAY_SAC_THEN, None)];

    let cards = vec![
        creature_card(C0, "Three-Color Beast", "{W}{U}{B}", 3, 5, &[]),
        creature_card(C1, "Negative Power Wall", "{2}{G}", -2, 4, &[]),
        card(
            C2,
            face("Disciple Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DISCIPLE_PROBE_ABILITIES,
        ),
    ];

    let mut engine = bench(14_003, cards, [Seat::with(&[C0, C1, C2]), Seat::with(&[])]);
    to_main(&mut engine, me());

    let beast = the(&engine, ZoneLocation::Battlefield, C0);
    let wall = the(&engine, ZoneLocation::Battlefield, C1);
    let probe = the(&engine, ZoneLocation::Battlefield, C2);

    // Put two +1/+1 counters on `beast` so its projected P/T is 5/7.
    engine
        .state
        .object_mut(beast)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 2);
    refresh_offer(&mut engine);

    let hand_before = hand_len(&engine, me());
    activate_index(&mut engine, me(), probe, 0);
    pass_to_choice(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![beast],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 27, "SacrificedTotalToughness == 7");
    assert_eq!(
        hand_len(&engine, me()),
        hand_before + 5,
        "SacrificedTotalPower == 5"
    );
    assert_eq!(
        engine
            .state()
            .object(probe)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        3,
        "SacrificedTotalManaValue == 3"
    );
    assert_eq!(
        engine
            .state()
            .object(probe)
            .unwrap()
            .counters
            .get(CounterKind::Energy),
        3,
        "SacrificedDistinctColors == 3"
    );

    // Test `PlayerMayPayCostThen { CostPart::Sacrifice }` on `wall` (MV 3, power -2).
    activate_index(&mut engine, me(), probe, 1);
    pass_to_choice(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        life(&engine, me()),
        30,
        "PlayerMayPayCostThen sacrifice recorded SacrificedTotalManaValue == 3"
    );
    assert_eq!(
        life(&engine, them()),
        20,
        "negative total power (-2) floored to 0 per CR 107.1b"
    );
}

// =========================================================================
// 2. Effect::ChooseOneRestricted & ModalRestriction
// =========================================================================

#[test]
fn choose_one_restricted_once_per_permanent_demonic_pact_full_lifecycle() {
    static PACT_MODES: &[Effect] = &[
        Effect::GainLife {
            amount: Amount::Fixed(4),
        },
        Effect::DrawCards {
            amount: Amount::Fixed(2),
        },
        Effect::LoseLife {
            target: PlayerRel::Opponent,
            amount: Amount::Fixed(4),
        },
        Effect::LoseGame,
    ];
    static PACT_EFFECTS: &[Effect] = &[Effect::ChooseOneRestricted {
        choices: PACT_MODES,
        restriction: ModalRestriction::OncePerPermanent,
    }];
    static PACT_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        PACT_EFFECTS
    )];

    let cards = vec![card(
        C0,
        face("Demonic Pact Probe", "{2}{B}{B}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        PACT_ABILITIES,
    )];
    let mut engine = bench(14_004, cards, [Seat::with(&[C0]), Seat::with(&[])]);

    let expect_modes = |engine: &Bench, expected: &[usize]| {
        let Pending::ChooseCastMode { options, .. } = engine.pending() else {
            panic!("expected ChooseCastMode, got {:?}", engine.pending());
        };
        let actual: Vec<usize> = options
            .iter()
            .map(|o| match o.kind {
                CastModeKind::Mode(i) => i,
                other => panic!("expected Mode, got {other:?}"),
            })
            .collect();
        assert_eq!(actual, expected, "offered original mode indices");
    };

    // Upkeep 1 (Turn 1): all 4 modes available; pick index 1 -> Mode(1) Draw 2.
    pass_to_choice(&mut engine);
    expect_modes(&engine, &[0, 1, 2, 3]);
    let hash_before = engine.state().snapshot_hash();
    let hand_before = hand_len(&engine, me());
    engine.apply(me(), PlayerAction::ChooseMode(1)).unwrap();
    settle(&mut engine, me());
    assert_eq!(hand_len(&engine, me()), hand_before + 2);
    assert_ne!(
        engine.state().snapshot_hash(),
        hash_before,
        "modal_modes_used is included in snapshot_hash"
    );

    // Advance to P0's Upkeep 2 (Turn 3): Mode(1) is omitted!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    expect_modes(&engine, &[0, 2, 3]);
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);

    // Advance to P0's Upkeep 3 (Turn 5): only Mode(2) and Mode(3) remain!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    expect_modes(&engine, &[2, 3]);
    // Out-of-range mode index must be refused cleanly without losing state.
    assert!(
        engine.apply(me(), PlayerAction::ChooseMode(2)).is_err(),
        "out-of-range mode index rejected"
    );
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 16);

    // Advance to P0's Upkeep 4 (Turn 7): ONLY Mode(3) (LoseGame) remains!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    expect_modes(&engine, &[3]);
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    assert!(
        engine.state().players[0].has_lost(),
        "choosing final LoseGame mode makes P0 lose the game"
    );
}

#[test]
fn choose_one_restricted_exhaustion_blink_reset_and_in_flight_departure_lki() {
    static TWO_MODES: &[Effect] = &[
        Effect::GainLife {
            amount: Amount::Fixed(2),
        },
        Effect::LoseLife {
            target: PlayerRel::Opponent,
            amount: Amount::Fixed(3),
        },
    ];
    static RESTRICTED_EFFECTS: &[Effect] = &[Effect::ChooseOneRestricted {
        choices: TWO_MODES,
        restriction: ModalRestriction::OncePerPermanent,
    }];
    static BLINK_EFFECTS: &[Effect] = &[Effect::Blink {
        target: TargetSpec::ThisObject,
        owner_control: true,
    }];
    static BOUNCE_EFFECTS: &[Effect] = &[Effect::ReturnToHand {
        target: TargetSpec::ThisObject,
    }];
    static RELIC_ABILITIES: &[AbilityDef] = &[
        free(RESTRICTED_EFFECTS, None),
        free(BLINK_EFFECTS, None),
        free(BOUNCE_EFFECTS, None),
    ];

    let cards = vec![card(
        C0,
        face("Modal Relic", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        RELIC_ABILITIES,
    )];

    let mut engine = bench(14_005, cards, [Seat::with(&[C0]), Seat::with(&[])]);
    to_main(&mut engine, me());
    let relic = the(&engine, ZoneLocation::Battlefield, C0);

    // Use Mode(0) then Mode(1).
    activate_index(&mut engine, me(), relic, 0);
    pass_to_choice(&mut engine);
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);

    activate_index(&mut engine, me(), relic, 0);
    pass_to_choice(&mut engine);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].kind, CastModeKind::Mode(1));
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 17);

    // Activation 3: 0 modes remain -> settles cleanly with NO prompt!
    activate_index(&mut engine, me(), relic, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);
    assert_eq!(life(&engine, them()), 17);

    // Blink the relic -> CR 400.7 new incarnation restores BOTH modes!
    activate_index(&mut engine, me(), relic, 1);
    settle(&mut engine, me());
    let relic_v2 = the(&engine, ZoneLocation::Battlefield, C0);
    activate_index(&mut engine, me(), relic_v2, 0);
    pass_to_choice(&mut engine);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode after blink");
    };
    assert_eq!(options.len(), 2, "blink resets OncePerPermanent mode mask");
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);

    // CR 608.2h: stack ability 0 of relic_v2, then bounce relic_v2 in response!
    // The resolving ability MUST still see Mode(0) was already used by relic_v2.
    activate_index(&mut engine, me(), relic_v2, 0);
    activate_index(&mut engine, me(), relic_v2, 2);
    pass_to_choice(&mut engine);
    assert_eq!(
        engine.state().object(relic_v2).unwrap().zone,
        crate::zone::Zone::Hand,
        "relic already bounced before modal ability resolves"
    );
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode from in-flight stack ability");
    };
    assert_eq!(
        options.len(),
        1,
        "in-flight ability remembers Mode(0) was used"
    );
    assert_eq!(options[0].kind, CastModeKind::Mode(1));
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 14);
}

#[test]
fn choose_one_restricted_once_per_turn_resets_next_turn_and_on_blink() {
    static TURN_MODES: &[Effect] = &[
        Effect::GainLife {
            amount: Amount::Fixed(1),
        },
        Effect::LoseLife {
            target: PlayerRel::Opponent,
            amount: Amount::Fixed(1),
        },
    ];
    static TURN_EFFECTS: &[Effect] = &[Effect::ChooseOneRestricted {
        choices: TURN_MODES,
        restriction: ModalRestriction::OncePerTurn,
    }];
    static BLINK_EFFECTS: &[Effect] = &[Effect::Blink {
        target: TargetSpec::ThisObject,
        owner_control: true,
    }];
    static TURN_RELIC_ABILITIES: &[AbilityDef] =
        &[free(TURN_EFFECTS, None), free(BLINK_EFFECTS, None)];

    let cards = vec![card(
        C0,
        face("Turn Modal Relic", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TURN_RELIC_ABILITIES,
    )];
    let mut engine = bench(14_006, cards, [Seat::with(&[C0]), Seat::with(&[])]);
    to_main(&mut engine, me());
    let relic = the(&engine, ZoneLocation::Battlefield, C0);

    // Same turn: choose Mode(0), verify Mode(1) only remains, then 0 remain.
    activate_index(&mut engine, me(), relic, 0);
    pass_to_choice(&mut engine);
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 21);

    activate_index(&mut engine, me(), relic, 0);
    pass_to_choice(&mut engine);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].kind, CastModeKind::Mode(1));
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 19);

    // Third activation same turn does nothing.
    activate_index(&mut engine, me(), relic, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 21);
    assert_eq!(life(&engine, them()), 19);

    // Mid-turn blink immediately resets OncePerTurn mask (CR 400.7).
    activate_index(&mut engine, me(), relic, 1);
    settle(&mut engine, me());
    let relic_blinked = the(&engine, ZoneLocation::Battlefield, C0);
    activate_index(&mut engine, me(), relic_blinked, 0);
    pass_to_choice(&mut engine);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode after mid-turn blink");
    };
    assert_eq!(options.len(), 2, "mid-turn blink resets OncePerTurn mask");
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());

    // Advance to P1's main phase (new turn): BOTH modes are available again!
    to_main(&mut engine, them());
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    activate_index(&mut engine, me(), relic_blinked, 0);
    pass_to_choice(&mut engine);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode on next turn");
    };
    assert_eq!(options.len(), 2, "OncePerTurn resets at end of turn");
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, them());
}

// =========================================================================
// 3. OncePerPermanent & Power-Up Cost Reduction
// =========================================================================

#[test]
fn power_up_entry_turn_discount_next_turn_full_cost_and_once_per_permanent() {
    static POWER_UP_ABILITIES: &[AbilityDef] = &[activated!(
        cost!("{3}{W}{U}{R}"),
        &[Effect::AddCounterSource {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(2),
        }],
        once_per_permanent = true,
        reduce_by_own_mana_cost_if_entered_this_turn = true
    )];

    // Printed mana cost: {1}{W}{U}.
    // Power-up cost: {3}{W}{U}{R}.
    // Discounted cost on entry turn: {2}{R}.
    let cards = vec![card(
        C0,
        creature_face("Heroic Power-Up", "{1}{W}{U}", 2, 2),
        KeywordSet::EMPTY,
        POWER_UP_ABILITIES,
    )];

    let mut engine = bench(
        14_007,
        cards,
        [Seat::with(&[]).holding(&[C0, C0]), Seat::with(&[])],
    );
    to_main(&mut engine, me());

    // Cast first copy on Turn 1.
    let copy1_hand = objects(&engine, ZoneLocation::Hand(me()), C0)[0];
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    cast_hand(&mut engine, me(), copy1_hand);
    settle(&mut engine, me());
    let copy1 = the(&engine, ZoneLocation::Battlefield, C0);

    // 1R is NOT enough for 2R.
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    assert!(
        !can_activate_index(&engine, copy1, 0),
        "1R is insufficient for discounted 2R"
    );

    // Adding 1 -> 2R IS enough on the turn it entered!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    assert!(
        can_activate_index(&engine, copy1, 0),
        "2R pays Power-Up on entry turn"
    );
    let hash_before = engine.state().snapshot_hash();
    activate_index(&mut engine, me(), copy1, 0);
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0, "consumed exactly 2R");
    assert_eq!(
        engine
            .state()
            .object(copy1)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_ne!(
        engine.state().snapshot_hash(),
        hash_before,
        "OncePerPermanent activation mask changes snapshot_hash"
    );

    // Even with plenty of mana, copy1 cannot activate again this turn OR next turn.
    add_pool(&mut engine, me(), ManaColor::White, 3);
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    add_pool(&mut engine, me(), ManaColor::Red, 3);
    assert!(
        !can_activate_index(&engine, copy1, 0),
        "OncePerPermanent blocks second activation same turn"
    );
    clear_pool(&mut engine, me());

    // Cast second copy on Turn 1, do NOT activate it this turn, then advance to Turn 3.
    let copy2_hand = objects(&engine, ZoneLocation::Hand(me()), C0)[0];
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    cast_hand(&mut engine, me(), copy2_hand);
    settle(&mut engine, me());

    to_main(&mut engine, them());
    to_main(&mut engine, me());

    let all_copies = objects(&engine, ZoneLocation::Battlefield, C0);
    let copy2 = all_copies.into_iter().find(|&id| id != copy1).unwrap();

    // Turn 3: copy1 is STILL permanently locked even with full 3WUR.
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    assert!(
        !can_activate_index(&engine, copy1, 0),
        "OncePerPermanent persists across turns"
    );

    // Turn 3: copy2 did NOT enter this turn, so 2R is NOT enough; full 3WUR IS required.
    clear_pool(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    assert!(
        !can_activate_index(&engine, copy2, 0),
        "no Power-Up discount after the turn it entered"
    );
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    assert!(
        can_activate_index(&engine, copy2, 0),
        "full 3WUR activates copy2 on a later turn"
    );
    activate_index(&mut engine, me(), copy2, 0);
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0, "consumed full 3WUR");
    assert_eq!(
        engine
            .state()
            .object(copy2)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
}

#[test]
fn power_up_colored_spillover_cheated_entry_two_abilities_and_blink_reset() {
    static DUAL_POWER_UP: &[AbilityDef] = &[
        // Ability 0: {4}{W}, own mana cost {W}{W} -> on entry turn reduces by {W} + {1} spillover = {3}!
        activated!(
            cost!("{4}{W}"),
            &[Effect::GainLife {
                amount: Amount::Fixed(3),
            }],
            once_per_permanent = true,
            reduce_by_own_mana_cost_if_entered_this_turn = true
        ),
        // Ability 1: independent OncePerPermanent ability ({0}: draw 1).
        activated!(
            Cost::FREE,
            &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            once_per_permanent = true
        ),
        // Ability 2: free blink to test CR 400.7 reset + re-enabled entry-turn discount.
        free(
            &[Effect::Blink {
                target: TargetSpec::ThisObject,
                owner_control: true,
            }],
            None,
        ),
    ];

    let cards = vec![card(
        C0,
        creature_face("Dual Power-Up Angel", "{W}{W}", 2, 2),
        KeywordSet::EMPTY,
        DUAL_POWER_UP,
    )];
    let mut engine = bench(
        14_008,
        cards,
        [Seat::with(&[]).holding(&[C0]), Seat::with(&[])],
    );
    to_main(&mut engine, me());

    // Cheat onto battlefield directly without casting (0 mana paid) — discount still uses printed WW!
    let angel_hand = the(&engine, ZoneLocation::Hand(me()), C0);
    engine
        .state
        .move_object(
            angel_hand,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);
    let angel = the(&engine, ZoneLocation::Battlefield, C0);

    // 2 is insufficient; 3 (colorless only!) pays 4W reduced by WW (CR 118.7a–g spillover).
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    assert!(!can_activate_index(&engine, angel, 0));
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    assert!(
        can_activate_index(&engine, angel, 0),
        "WW spills excess W into generic, reducing 4W to 3"
    );

    activate_index(&mut engine, me(), angel, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23);
    assert_eq!(pool_total(&engine, me()), 0);

    // Ability 0 is now locked, while independent Ability 1 is still available!
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    assert!(!can_activate_index(&engine, angel, 0));
    assert!(
        can_activate_index(&engine, angel, 1),
        "second OncePerPermanent ability has its own bit in the mask"
    );
    let hand_before = hand_len(&engine, me());
    activate_index(&mut engine, me(), angel, 1);
    settle(&mut engine, me());
    assert_eq!(hand_len(&engine, me()), hand_before + 1);
    assert!(!can_activate_index(&engine, angel, 1));
    clear_pool(&mut engine, me());

    // Advance to Turn 3, then blink the angel: CR 400.7 resets BOTH bits AND re-enables entry-turn discount!
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    activate_index(&mut engine, me(), angel, 2);
    settle(&mut engine, me());
    let angel_blinked = the(&engine, ZoneLocation::Battlefield, C0);

    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    assert!(
        can_activate_index(&engine, angel_blinked, 0),
        "blink restores OncePerPermanent ability 0 AND entry-turn discount (3)"
    );
    assert!(
        can_activate_index(&engine, angel_blinked, 1),
        "blink restores OncePerPermanent ability 1"
    );
    activate_index(&mut engine, me(), angel_blinked, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 26);
}

// =========================================================================
// 4. Token.RememberTokens — Created tokens bound in SubjectContext::found
// =========================================================================

#[test]
fn created_tokens_populate_found_for_untap_goad_exile_without_clobbering_event() {
    static YOUR_NONTOKEN_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByYou,
        Filter::Not(&Filter::IsToken),
    ]);
    static ETB_TOKEN_EFFECTS: &[Effect] = &[
        // Creates 2 tapped 1/1 Goblin tokens -> binds BOTH into `SubjectContext::found`.
        Effect::CreateTokenModified {
            token: &GOBLIN_1_1_RED,
            amount: Amount::Fixed(2),
            tapped: true,
            haste: false,
        },
        // Existing `AddCounterCreatedTokens` (`ThisWay` journal) still works alongside `found`!
        Effect::AddCounterCreatedTokens {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        },
        // `UntapFound` untaps ONLY the newly created tokens (not a pre-existing tapped creature!).
        Effect::UntapFound,
        // `RepeatEachObject { only_found: true }` goads ONLY the created tokens.
        Effect::RepeatEachObject {
            zone: ZoneSel::Battlefield,
            filter: &Filter::Any,
            only_found: true,
            then: &[Effect::Goad {
                target: TargetSpec::EventObject,
            }],
        },
        // Existing `AtNextEndStepForCreatedTokens` schedules delayed exile for ONLY them.
        Effect::AtNextEndStepForCreatedTokens {
            effects: &[Effect::Exile {
                target: TargetSpec::EventObject,
            }],
        },
        // Outer trigger's `TargetSpec::EventObject` MUST NOT have been clobbered by token creation!
        Effect::PumpEventObject {
            power: Amount::Fixed(3),
            toughness: Amount::Fixed(3),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        },
    ];
    static TRIGGER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&YOUR_NONTOKEN_CREATURE),
        ETB_TOKEN_EFFECTS
    )];

    let cards = vec![
        card(
            C0,
            face("Balefang Trigger Probe", "{2}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            TRIGGER_ABILITIES,
        ),
        card(
            C1,
            creature_face("Triggering Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            C2,
            creature_face("Bystander Tapped Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        14_009,
        cards,
        [Seat::with(&[C0, C2]).holding(&[C1]), Seat::with(&[])],
    );
    to_main(&mut engine, me());

    let bystander = the(&engine, ZoneLocation::Battlefield, C2);
    engine.state.set_tapped(bystander, true);
    refresh_offer(&mut engine);

    let bear_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    cast_hand(&mut engine, me(), bear_hand);
    settle(&mut engine, me());

    let bear = the(&engine, ZoneLocation::Battlefield, C1);
    let goblins: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| engine.state().object(id).is_some_and(|o| o.card.is_none()))
        .collect();
    assert_eq!(goblins.len(), 2, "two Goblin tokens created");

    for &goblin in &goblins {
        let obj = engine.state().object(goblin).unwrap();
        assert_eq!(
            obj.counters.get(CounterKind::P1P1),
            1,
            "AddCounterCreatedTokens preserved"
        );
        assert!(
            !obj.status.contains(Status::TAPPED),
            "UntapFound untapped created token"
        );
        assert!(
            is_goaded_by(&engine, goblin, me()),
            "RepeatEachObject(only_found) goaded created token"
        );
    }
    assert!(
        engine
            .state()
            .object(bystander)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "pre-existing tapped creature was NOT untapped by UntapFound"
    );
    assert!(
        !is_goaded_by(&engine, bystander, me()),
        "pre-existing creature was NOT goaded"
    );
    assert!(
        !is_goaded_by(&engine, bear, me()),
        "triggering bear was NOT goaded"
    );
    // Outer `EventObject` survived token creation + `RepeatEachObject` -> triggering bear got +3/+3 (5/5)!
    assert_eq!(
        engine.state().object(bear).unwrap().characteristics().power,
        Some(5),
        "outer trigger EventObject was NOT clobbered by CreateToken or RepeatEachObject"
    );

    // Advance to P1's main phase (after P0's end step): BOTH created tokens exiled
    // (ceasing to exist per CR 704.5d), while bear & bystander stay on the battlefield!
    to_main(&mut engine, them());
    for &goblin in &goblins {
        assert!(
            engine.state().object(goblin).is_none(),
            "created token exiled at end step and ceased to exist (CR 704.5d)"
        );
    }
    assert_eq!(
        engine.state().object(bear).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(bystander).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

#[test]
fn create_token_copy_under_doubling_season_binds_all_copies_in_found() {
    static DOUBLER_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleTokenCreation {
            controller_filter: &Filter::ControlledByYou,
        },
    )];
    static COPY_AND_PICK_EFFECTS: &[Effect] = &[
        Effect::CreateTokenCopy {
            target: TargetSpec::Object(&Filter::CREATURE),
            controller: PlayerRel::You,
            amount: Amount::Fixed(1),
            mods: &[],
            tapped: false,
            attacking: false,
            haste_until_eot: false,
            cleanup: CopyCleanup::None,
        },
        // Choose 1 of the created token copies from `ChooseZone::Found` and exile it;
        // verify BOTH doubled copies were offered in `ChooseZone::Found`!
        Effect::ChooseCardEx {
            chooser: PlayerRel::You,
            zone: ChooseZone::Found,
            filter: &Filter::Any,
            min: 1,
            max: Amount::Fixed(1),
            reveal_chosen: false,
            movement: ChooseMove::Exile,
            not_chosen_movement: ChooseMove::None,
            then: &[],
            for_unchosen: &[],
        },
    ];
    static SPELL_ABILITIES: &[AbilityDef] = &[spell!(
        COPY_AND_PICK_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let cards = vec![
        card(
            C0,
            face("Doubler", "{4}{G}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DOUBLER_ABILITIES,
        ),
        card(
            C1,
            creature_face("Clone Template", "{1}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            C2,
            face("Copy and Pick", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            SPELL_ABILITIES,
        ),
    ];

    let mut engine = bench(
        14_010,
        cards,
        [Seat::with(&[C0, C1]).holding(&[C2]), Seat::with(&[])],
    );
    to_main(&mut engine, me());

    let template = the(&engine, ZoneLocation::Battlefield, C1);
    let spell_card = the(&engine, ZoneLocation::Hand(me()), C2);
    cast_hand(&mut engine, me(), spell_card);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![template],
                players: vec![],
            },
        )
        .unwrap();
    pass_to_choice(&mut engine);

    // Doubling Season doubled 1 copy -> 2 token copies, BOTH bound in `SubjectContext::found`!
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseCards from ChooseZone::Found, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        options.len(),
        2,
        "both doubled token copies are bound in found"
    );
    assert!(
        !options.contains(&template),
        "original creature is not in found"
    );

    let chosen_copy = options[0];
    let kept_copy = options[1];
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![chosen_copy],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert!(
        engine.state().object(chosen_copy).is_none(),
        "exiled token copy ceased to exist per CR 704.5d"
    );
    assert_eq!(
        engine.state().object(kept_copy).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(template).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}
