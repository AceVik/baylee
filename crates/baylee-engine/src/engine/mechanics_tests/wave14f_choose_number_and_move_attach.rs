//! Beat 18 / Engine Wave 14f deterministic rule tests:
//! 1. Dynamic Number Choice & Variable Resource Payment (`Effect::ChooseNumberEx`,
//!    `NumberChoiceKind::{Choice, EnergyPayment, ManaPayment}`,
//!    `VariablePayKind::{Energy, GenericMana, Life}`, `Amount::ResolutionChosenNumber` —
//!    `CR 107.1`, `CR 107.14`, `CR 118.3`, `CR 119.4`, `CR 119.8`, `CR 608.2d`).
//! 2. Battlefield Attachment Relocation (`Effect::MoveAttachment` —
//!    `CR 301.5c`, `CR 301.6`, `CR 303.4d`, `CR 303.4j`, `CR 701.3a–b`, `CR 702.16c–d`).

#![allow(clippy::too_many_lines)] // comprehensive multi-case rule tests

use super::*;
use crate::choice::{AnswerFault, ChoicePrompt, NumberPrompt};
use crate::event::GameEvent;
use crate::object::Status;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::prelude::{mana_ability, spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, CopyCleanup, Duration, Filter, ManaUse, Modifier, NumberChoiceKind, PlayerRel,
    SpendRider, TargetSpec, TokenDef, Trigger, VariablePayKind, crew,
};
use baylee_core::color::ColorSet;
use baylee_core::generated::subtypes::{artifact, creature, enchantment};
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_960;
const C1: u32 = 99_961;
const C2: u32 = 99_962;
const C3: u32 = 99_963;
const C4: u32 = 99_964;
const C5: u32 = 99_965;
const C6: u32 = 99_966;

static AURA_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[enchantment::AURA];
static EQUIPMENT_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[artifact::EQUIPMENT];
static VEHICLE_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[artifact::VEHICLE];
static AETHERBORN_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[creature::AETHERBORN];

static AETHER_TOKEN: TokenDef = TokenDef {
    name: "Aetherborn",
    colors: ColorSet::EMPTY,
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    subtypes: AETHERBORN_SUBTYPES,
    ..TokenDef::DEFAULT
};

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    if let Pending::Priority { player: p, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player: p,
            legal: Box::new(engine.compute_legal(p)),
        };
    }
}

fn aim_object(engine: &mut Bench, seat: PlayerId, obj: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![obj],
                players: vec![],
            },
        )
        .expect("object target accepted");
}

fn aim_player(engine: &mut Bench, seat: PlayerId, target: PlayerId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .expect("player target accepted");
}

fn energy_of(engine: &Bench, seat: PlayerId) -> u16 {
    engine.state().players[usize::from(seat.get())].energy
}

#[track_caller]
fn expect_choose_number(
    engine: &Bench,
    want_player: PlayerId,
    want_min: u32,
    want_max: u32,
    want_reason: NumberPrompt,
) {
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason,
    } = engine.pending()
    else {
        panic!("expected ChooseNumber, got {:?}", engine.pending());
    };
    assert_eq!(
        (*player, *min, *max, *reason),
        (want_player, want_min, want_max, want_reason)
    );
}

#[test]
fn galvanic_discharge_clamps_energy_pays_chosen_and_zero_emits_no_counter_event() {
    static DISCHARGE_EFFECTS: &[Effect] = &[
        Effect::Energy {
            who: PlayerRel::You,
            amount: Amount::Fixed(3),
        },
        Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(0),
            max: Amount::Fixed(u32::MAX),
            prompt: NumberChoiceKind::EnergyPayment,
            pay_cost: Some(VariablePayKind::Energy),
            then: &[Effect::DealDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::ResolutionChosenNumber,
            }],
        },
    ];
    static DISCHARGE_AB: &[AbilityDef] = &[free(
        DISCHARGE_EFFECTS,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let probe = card(
        C0,
        face("Galvanic Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DISCHARGE_AB,
    );
    let wall = card(
        C1,
        creature_face("Stone Wall", "{2}", 0, 5),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        14_601,
        vec![probe, wall],
        [Seat::with(&[C0]), Seat::with(&[C1])],
    );
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);
    let wall_id = the(&engine, ZoneLocation::Battlefield, C1);

    activate(&mut engine, me(), probe_id, 0);
    aim_object(&mut engine, me(), wall_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    // After gaining 3 {E}, max is clamped from u32::MAX down to 3.
    assert_eq!(energy_of(&engine, me()), 3);
    expect_choose_number(&engine, me(), 0, 3, NumberPrompt::VariableEnergyPayment);
    assert_eq!(
        engine
            .pending()
            .answer_fault(&PlayerAction::ChooseNumber(4)),
        Some(AnswerFault::OutOfRange)
    );
    assert!(engine.apply(me(), PlayerAction::ChooseNumber(4)).is_err());
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine, me());

    assert_eq!(energy_of(&engine, me()), 1, "paid 2 {{E}} out of 3 {{E}}");
    assert_eq!(
        engine.state().object(wall_id).unwrap().damage,
        2,
        "dealt damage equal to Amount::ResolutionChosenNumber (2)"
    );

    // Second activation: gain +3 {E} -> 4 {E}; choose 0 {E}.
    activate(&mut engine, me(), probe_id, 0);
    aim_object(&mut engine, me(), wall_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    assert_eq!(energy_of(&engine, me()), 4);
    let journal_before_zero = engine.journal().entries().len();
    engine.apply(me(), PlayerAction::ChooseNumber(0)).unwrap();
    let counter_events_on_zero = engine.journal().entries()[journal_before_zero..]
        .iter()
        .filter(|e| matches!(e.event, GameEvent::PlayerCountersChanged { .. }))
        .count();
    assert_eq!(
        counter_events_on_zero, 0,
        "paying 0 {{E}} must not emit a spurious PlayerCountersChanged event"
    );
    settle(&mut engine, me());
    assert_eq!(energy_of(&engine, me()), 4);
    assert_eq!(engine.state().object(wall_id).unwrap().damage, 2);
}

#[test]
fn die_young_and_aether_spike_bind_chosen_number_across_pump_and_nested_may_pay() {
    static DIE_YOUNG_EFFECTS: &[Effect] = &[
        Effect::Energy {
            who: PlayerRel::You,
            amount: Amount::Fixed(2),
        },
        Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(0),
            max: Amount::Fixed(u32::MAX),
            prompt: NumberChoiceKind::EnergyPayment,
            pay_cost: Some(VariablePayKind::Energy),
            then: &[Effect::PumpTarget {
                power: Amount::Negated(&Amount::ResolutionChosenNumber),
                toughness: Amount::Negated(&Amount::ResolutionChosenNumber),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        },
    ];
    static SPIKE_EFFECTS: &[Effect] = &[
        Effect::Energy {
            who: PlayerRel::You,
            amount: Amount::Fixed(2),
        },
        Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(0),
            max: Amount::Fixed(u32::MAX),
            prompt: NumberChoiceKind::EnergyPayment,
            pay_cost: Some(VariablePayKind::Energy),
            then: &[Effect::PlayerMayPayOr {
                player: PlayerRel::Opponent,
                mana: Amount::ResolutionChosenNumber,
                effect: &Effect::CounterTargetSpell,
            }],
        },
    ];
    static DIE_YOUNG_AB: &[AbilityDef] = &[spell!(
        DIE_YOUNG_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static SPIKE_AB: &[AbilityDef] = &[spell!(
        SPIKE_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];

    let die_young = card(
        C0,
        face("Die Young Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        DIE_YOUNG_AB,
    );
    let spike = card(
        C1,
        face("Aether Spike Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        SPIKE_AB,
    );
    let bear = card(
        C2,
        creature_face("Runeclaw Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let ogre = card(
        C3,
        creature_face("Hill Ogre", "{0}", 3, 3),
        KeywordSet::FLASH,
        &[],
    );

    let build = || {
        let mut e = bench(
            14_602,
            vec![die_young, spike, bear, ogre],
            [
                Seat::default().holding(&[C0, C1]),
                Seat::with(&[C2]).holding(&[C3]),
            ],
        );
        to_main(&mut e, me());
        e
    };

    let mut engine = build();
    let dy_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let bear_id = the(&engine, ZoneLocation::Battlefield, C2);

    // Cast Die Young targeting 2/2 Bear, spend 2 {E} -> Bear gets -2/-2 and dies to SBA!
    engine
        .apply(me(), PlayerAction::CastSpell { card: dy_id })
        .unwrap();
    aim_object(&mut engine, me(), bear_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine, me());
    assert_eq!(energy_of(&engine, me()), 0);
    assert_eq!(
        engine.state().object(bear_id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );

    // Give me() 1 extra {E} first so Aether Spike reaches 3 {E}, then opponent flashes in Ogre
    // with only {G}{G} floating; we respond with Aether Spike paying 3 {E}.
    engine.state.players[usize::from(me().get())].energy = 1;
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    add_pool(&mut engine, them(), ManaColor::Green, 2);
    let ogre_id = the(&engine, ZoneLocation::Hand(them()), C3);
    engine
        .apply(them(), PlayerAction::CastSpell { card: ogre_id })
        .unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    let spike_id = the(&engine, ZoneLocation::Hand(me()), C1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: spike_id })
        .unwrap();
    aim_object(&mut engine, me(), ogre_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    // Pay 3 {E}: opponent is asked PayTax { mana: 3 }, cannot pay {3} with 2 mana, and Ogre is countered!
    let hash_at_prompt = engine.state().snapshot_hash();
    engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
    let hash_after_choice = engine.state().snapshot_hash();
    assert_ne!(hash_at_prompt, hash_after_choice);
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::PayTax { mana: 3 },
            ..
        } if *player == them()
    ));
    // Even attempting YesNo(true) with only 2 mana cannot pay {3}: mana stays at 2 and Ogre is countered!
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());

    assert_eq!(energy_of(&engine, me()), 0);
    assert_eq!(engine.state().players[1].mana_pool.total(), 2);
    assert_eq!(
        engine.state().object(ogre_id).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "Ogre countered when opponent had 2 mana against tax of ResolutionChosenNumber = 3"
    );

    // Replay identically up to Aether Spike, but pay only 2 {E} so opponent CAN pay {2} -> Ogre resolves!
    let mut engine2 = build();
    let dy2 = the(&engine2, ZoneLocation::Hand(me()), C0);
    let bear2 = the(&engine2, ZoneLocation::Battlefield, C2);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: dy2 })
        .unwrap();
    aim_object(&mut engine2, me(), bear2);
    engine2.apply(me(), PlayerAction::PassPriority).unwrap();
    engine2.apply(them(), PlayerAction::PassPriority).unwrap();
    engine2.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine2, me());
    engine2.state.players[usize::from(me().get())].energy = 1;
    engine2.apply(me(), PlayerAction::PassPriority).unwrap();
    add_pool(&mut engine2, them(), ManaColor::Green, 2);
    let ogre2 = the(&engine2, ZoneLocation::Hand(them()), C3);
    engine2
        .apply(them(), PlayerAction::CastSpell { card: ogre2 })
        .unwrap();
    engine2.apply(them(), PlayerAction::PassPriority).unwrap();
    let spike2 = the(&engine2, ZoneLocation::Hand(me()), C1);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: spike2 })
        .unwrap();
    aim_object(&mut engine2, me(), ogre2);
    engine2.apply(me(), PlayerAction::PassPriority).unwrap();
    engine2.apply(them(), PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine2.state().snapshot_hash(),
        hash_at_prompt,
        "deterministic snapshot_hash at ChooseNumberEx suspension"
    );

    engine2.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    // Opponent is now suspended at YesNo for paying {2}; chosen_number (2) is hashed in SubjectContext!
    assert!(matches!(engine2.pending(), Pending::YesNo { player, .. } if *player == them()));
    engine2.apply(them(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine2, me());

    assert_eq!(energy_of(&engine2, me()), 1);
    assert_eq!(
        engine2.state().object(ogre2).unwrap().zone,
        crate::zone::Zone::Battlefield,
        "Ogre resolved after opponent paid {{2}} matching ResolutionChosenNumber = 2"
    );
}

#[test]
fn aether_refinery_skips_prompt_and_then_when_min_exceeds_affordable_energy() {
    static REFINERY_AB: &[AbilityDef] = &[free(
        &[Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(1),
            max: Amount::Fixed(u32::MAX),
            prompt: NumberChoiceKind::EnergyPayment,
            pay_cost: Some(VariablePayKind::Energy),
            then: &[
                Effect::CreateTokenSized {
                    token: &AETHER_TOKEN,
                    amount: Amount::Fixed(1),
                    power: Amount::ResolutionChosenNumber,
                    toughness: Amount::ResolutionChosenNumber,
                    who: PlayerRel::You,
                    tapped: false,
                    attacking: false,
                    haste_until_eot: false,
                    cleanup: CopyCleanup::None,
                },
                Effect::GainLife {
                    amount: Amount::Fixed(2),
                },
            ],
        }],
        None,
    )];

    let refinery = card(
        C0,
        face("Refinery Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        REFINERY_AB,
    );
    let mut engine = bench(14_603, vec![refinery], [Seat::with(&[C0]), Seat::default()]);
    to_main(&mut engine, me());
    let ref_id = the(&engine, ZoneLocation::Battlefield, C0);
    let starting_life = life(&engine, me());

    // 1. With 0 {E}: min (1) > clamped max (0) -> no ChooseNumber prompt, no token created!
    activate(&mut engine, me(), ref_id, 0);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        1,
        "only Refinery on the battlefield when min (1) > held {{E}} (0)"
    );
    assert_eq!(life(&engine, me()), starting_life);

    // 2. With 4 {E}: offers 1..=4, choosing 3 pays 3 {E} and creates a 3/3 Aetherborn token!
    engine.state.players[usize::from(me().get())].energy = 4;
    activate(&mut engine, me(), ref_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    expect_choose_number(&engine, me(), 1, 4, NumberPrompt::VariableEnergyPayment);
    assert_eq!(
        engine
            .pending()
            .answer_fault(&PlayerAction::ChooseNumber(0)),
        Some(AnswerFault::OutOfRange)
    );
    assert!(engine.apply(me(), PlayerAction::ChooseNumber(0)).is_err());
    engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
    settle(&mut engine, me());

    assert_eq!(energy_of(&engine, me()), 1);
    assert_eq!(life(&engine, me()), starting_life + 2);
    let token_id = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| id != ref_id)
        .expect("3/3 token created");
    let chars = engine.state().object(token_id).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(3)));
}

#[test]
fn variable_generic_mana_and_life_payment_respect_spend_restrictions_and_cant_lose_life() {
    static POWERSTONE_MANA: &[AbilityDef] =
        &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)
            .restricted_for(
                &Filter::ARTIFACT,
                SpendRider::None,
                ManaUse::NonSpellPaymentsOrMatchingSpells
            )])];
    static CREATURE_SPELL_ONLY_MANA: &[AbilityDef] =
        &[mana_ability!(&[Effect::mana(ManaColor::Green, 3)
            .restricted_for(
                &Filter::CREATURE,
                SpendRider::None,
                ManaUse::Spells
            )])];
    static MANA_PAY_AB: &[AbilityDef] = &[free(
        &[Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(2),
            max: Amount::Fixed(10),
            prompt: NumberChoiceKind::ManaPayment,
            pay_cost: Some(VariablePayKind::GenericMana),
            then: &[Effect::GainLife {
                amount: Amount::ResolutionChosenNumber,
            }],
        }],
        None,
    )];
    static LIFE_PAY_MIN_ONE_AB: &[AbilityDef] = &[free(
        &[Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(1),
            max: Amount::Fixed(10),
            prompt: NumberChoiceKind::Choice,
            pay_cost: Some(VariablePayKind::Life),
            then: &[Effect::DrawCards {
                amount: Amount::ResolutionChosenNumber,
            }],
        }],
        None,
    )];
    static PLATINUM_LOCK_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::CantLoseLife {
            who: PlayerRel::You
        }
    )];

    let pstone = super::super::synthetic::land(C0, "Powerstone", POWERSTONE_MANA);
    let creature_dork =
        super::super::synthetic::land(C1, "Creature Cradle", CREATURE_SPELL_ONLY_MANA);
    let mana_sink = card(
        C2,
        face("Mana Sink", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        MANA_PAY_AB,
    );
    let life_sink = card(
        C3,
        face("Life Sink", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        LIFE_PAY_MIN_ONE_AB,
    );
    let lock_piece = card(
        C4,
        face("Aegis Lock", "{0}", TypeSet::ARTIFACT),
        KeywordSet::FLASH,
        PLATINUM_LOCK_AB,
    );

    let mut engine = bench(
        14_604,
        vec![pstone, creature_dork, mana_sink, life_sink, lock_piece],
        [
            Seat::with(&[C0, C1, C2, C3]).holding(&[C4]).at(5),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let pstone_id = the(&engine, ZoneLocation::Battlefield, C0);
    let dork_id = the(&engine, ZoneLocation::Battlefield, C1);
    let mana_sink_id = the(&engine, ZoneLocation::Battlefield, C2);
    let life_sink_id = the(&engine, ZoneLocation::Battlefield, C3);

    // Before the window, only the one red mana can pay. Float the Powerstone
    // mana from the window to meet the minimum; spell-only mana stays unspent.
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    activate(&mut engine, me(), dork_id, 0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, me(), mana_sink_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            me(),
            baylee_core::mana::ManaPayment::Variable { min: 2, max: 10 }
        )),
    );
    assert!(
        engine
            .apply(me(), PlayerAction::PayWindow { steps: Vec::new() })
            .is_err(),
        "a batched window action cannot leave the pool short of the minimum"
    );
    activate(&mut engine, me(), pstone_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    expect_choose_number(&engine, me(), 2, 2, NumberPrompt::VariableManaPayment);
    assert!(engine.apply(me(), PlayerAction::ChooseNumber(3)).is_err());
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "spent 1 unrestricted + 1 Powerstone mana, leaving all 3 creature-spell-only mana"
    );
    assert_eq!(life(&engine, me()), 7, "5 life + 2 gained from mana sink");

    // Both usable sources are now spent and the Powerstone is tapped.
    // Passing short skips the payment and its follow-up.
    activate(&mut engine, me(), mana_sink_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 7);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    // Life payment: at 7 life, max (10) clamps to 7; paying 4 life drops to 3 and draws 4 cards.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), life_sink_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    expect_choose_number(&engine, me(), 1, 7, NumberPrompt::VariableLifePayment);
    engine.apply(me(), PlayerAction::ChooseNumber(4)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 3);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 4
    );

    // Cast CantLoseLife lock (CR 119.8): payable life becomes 0, so min=1 life payment is skipped!
    let lock_id = the(&engine, ZoneLocation::Hand(me()), C4);
    engine
        .apply(me(), PlayerAction::CastSpell { card: lock_id })
        .unwrap();
    settle(&mut engine, me());

    let hand_with_lock = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), life_sink_id, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 3, "life unchanged under CantLoseLife");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_with_lock,
        "min=1 life payment skipped when CantLoseLife clamps payable life to 0"
    );
}

#[test]
fn by_invitation_only_choice_of_damnations_and_sibling_effect_persistence() {
    static INVITATION_AB: &[AbilityDef] = &[free(
        &[Effect::ChooseNumberEx {
            chooser: PlayerRel::You,
            min: Amount::Fixed(0),
            max: Amount::Fixed(13),
            prompt: NumberChoiceKind::Choice,
            pay_cost: None,
            then: &[Effect::SacrificeAmountForPlayers {
                who: PlayerRel::EachPlayer,
                filter: &Filter::CREATURE,
                amount: Amount::ResolutionChosenNumber,
            }],
        }],
        None,
    )];
    // Targeted opponent chooses N in 1..=5 with empty `then`, and the SIBLING effect in the
    // outer sequence deals (6 - N) life loss to that opponent via ConstantMinus!
    static DAMNATIONS_AB: &[AbilityDef] = &[free(
        &[
            Effect::ChooseNumberEx {
                chooser: PlayerRel::Chosen,
                min: Amount::Fixed(1),
                max: Amount::Fixed(5),
                prompt: NumberChoiceKind::Choice,
                pay_cost: None,
                then: &[],
            },
            Effect::LoseLife {
                target: PlayerRel::Chosen,
                amount: Amount::ConstantMinus {
                    constant: 6,
                    subtract: &Amount::ResolutionChosenNumber,
                },
            },
        ],
        Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )];
    // Separate ability reading Amount::ResolutionChosenNumber with NO prior ChooseNumberEx:
    // verifies state.resolution_chosen_number does not leak across resolutions!
    static STALE_READ_AB: &[AbilityDef] = &[free(
        &[Effect::GainLife {
            amount: Amount::ResolutionChosenNumber,
        }],
        None,
    )];

    let invitation = card(
        C0,
        face("Invitation Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        INVITATION_AB,
    );
    let damnations = card(
        C1,
        face("Damnations Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DAMNATIONS_AB,
    );
    let reader = card(
        C2,
        face("Stale Reader", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        STALE_READ_AB,
    );
    let dork = card(
        C3,
        creature_face("Squire", "{0}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        14_605,
        vec![invitation, damnations, reader, dork],
        [Seat::with(&[C0, C1, C2, C3, C3, C3]), Seat::with(&[C3, C3])],
    );
    to_main(&mut engine, me());
    let inv_id = the(&engine, ZoneLocation::Battlefield, C0);
    let dam_id = the(&engine, ZoneLocation::Battlefield, C1);
    let reader_id = the(&engine, ZoneLocation::Battlefield, C2);

    // 1. By Invitation Only: choose 2 -> me() (3 creatures) picks 1 to keep (sacrificing 2); them() (2 creatures) sacs all 2!
    activate(&mut engine, me(), inv_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    expect_choose_number(&engine, me(), 0, 13, NumberPrompt::Choice);
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    if let Pending::ChooseCards {
        player,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::KeepCreatures,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(player, me());
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    } else {
        panic!(
            "expected KeepCreatures (1-of-3) choice for me(), got {:?}",
            engine.pending()
        );
    }
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C3).len(), 1);

    // 2. Choice of Damnations: target opponent chooses 2 -> sibling LoseLife deals 6 - 2 = 4!
    activate(&mut engine, me(), dam_id, 0);
    aim_player(&mut engine, me(), them());
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    expect_choose_number(&engine, them(), 1, 5, NumberPrompt::Choice);
    engine.apply(them(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 16, "20 - (6 - 2) = 16 life");

    // 3. Separate resolution with no ChooseNumberEx sees 0, never stale state!
    activate(&mut engine, me(), reader_id, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 20);
}

#[test]
fn aura_graft_and_crown_of_the_ages_enforce_enchant_legality_must_change_and_hexproof() {
    static YOUR_CREATURE_AURA_AB: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(2, 2)
        ),
    ];
    static AURA_FILTER: Filter = Filter::HasSubtype(enchantment::AURA);
    static AURA_GRAFT_AB: &[AbilityDef] = &[free(
        &[
            Effect::ChangeController {
                new_controller: PlayerRel::You,
            },
            Effect::MoveAttachment {
                attachment: TargetSpec::Object(&AURA_FILTER),
                recipient_filter: &Filter::Any,
                must_change_recipient: true,
                optional: false,
                then_if_moved: &[],
            },
        ],
        Some(TargetReq::one(TargetSpec::Object(&AURA_FILTER))),
    )];
    static CROWN_AB: &[AbilityDef] = &[free(
        &[Effect::MoveAttachment {
            attachment: TargetSpec::Object(&AURA_FILTER),
            recipient_filter: &Filter::CREATURE,
            must_change_recipient: true,
            optional: false,
            then_if_moved: &[],
        }],
        Some(TargetReq::one(TargetSpec::Object(&AURA_FILTER))),
    )];
    static PRO_ENCHANTMENT_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&Filter::ENCHANTMENT)
    )];
    static CANT_BECOME_ENCH_AB: &[AbilityDef] =
        &[static_ability!(Filter::This, Modifier::CantBecomeEnchanted)];

    let mut aura_face = face("Loyal Mantle", "{0}", TypeSet::ENCHANTMENT);
    aura_face.subtypes = AURA_SUBTYPES;
    aura_face.keywords = KeywordSet::FLASH;
    let loyal_aura = card(C0, aura_face, KeywordSet::FLASH, YOUR_CREATURE_AURA_AB);
    let graft_rig = card(
        C1,
        face("Graft Rig", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        AURA_GRAFT_AB,
    );
    let crown = card(
        C2,
        face("Crown Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CROWN_AB,
    );
    let hexproof_ally = card(
        C3,
        creature_face("Gladecover Ally", "{0}", 1, 1),
        KeywordSet::HEXPROOF,
        &[],
    );
    let protected_ally = card(
        C4,
        creature_face("Ward-Proof Monk", "{0}", 2, 2),
        KeywordSet::EMPTY,
        PRO_ENCHANTMENT_AB,
    );
    let unenchantable_ally = card(
        C5,
        creature_face("Slick Ranger", "{0}", 2, 2),
        KeywordSet::EMPTY,
        CANT_BECOME_ENCH_AB,
    );
    let enemy_bear = card(
        C6,
        creature_face("Enemy Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        14_606,
        vec![
            loyal_aura,
            graft_rig,
            crown,
            hexproof_ally,
            protected_ally,
            unenchantable_ally,
            enemy_bear,
        ],
        [
            Seat::with(&[C1, C2, C3, C4, C5]),
            Seat::with(&[C6, C6]).holding(&[C0]),
        ],
    );
    to_main(&mut engine, me());
    let graft_id = the(&engine, ZoneLocation::Battlefield, C1);
    let crown_id = the(&engine, ZoneLocation::Battlefield, C2);
    let hex_id = the(&engine, ZoneLocation::Battlefield, C3);
    let enemy_bears = objects(&engine, ZoneLocation::Battlefield, C6);

    // Opponent flashes Loyal Mantle ("Enchant creature you control") onto enemy_bears[0].
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let aura_id = the(&engine, ZoneLocation::Hand(them()), C0);
    engine
        .apply(them(), PlayerAction::CastSpell { card: aura_id })
        .unwrap();
    aim_object(&mut engine, them(), enemy_bears[0]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(enemy_bears[0])
    );

    // Activate Aura Graft targeting Loyal Mantle:
    // 1) ChangeController gives Loyal Mantle to me().
    // 2) MoveAttachment evaluates "Enchant creature you control" for me() (CR 303.4d):
    //    - enemy_bears[0] and enemy_bears[1] are excluded (controlled by them(), not me()!)
    //    - protected_ally (ProtectionFrom Enchantment) and unenchantable_ally (CantBecomeEnchanted) are excluded!
    //    - hexproof_ally IS offered (CR 303.4j untargeted relocation ignores hexproof)!
    activate(&mut engine, me(), graft_id, 0);
    aim_object(&mut engine, me(), aura_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        total,
    } = engine.pending()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(
        (*player, options.as_slice(), *min, *max, *prompt, total),
        (
            me(),
            &[hex_id][..],
            1,
            1,
            ChoicePrompt::NewHost { aura: aura_id },
            &None
        )
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![hex_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(engine.state().object(aura_id).unwrap().controller, me());
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(hex_id)
    );
    assert_eq!(
        engine
            .state()
            .object(hex_id)
            .unwrap()
            .characteristics()
            .power,
        Some(3),
        "hexproof ally gets +2/+2 from relocated Aura and Aura survives SBA"
    );

    // Crown of the Ages with must_change_recipient: true when NO other legal host exists:
    // hex_id is the current host, and C4/C5/enemy_bears cannot be enchanted by Loyal Mantle,
    // so MoveAttachment skips the prompt cleanly and keeps Loyal Mantle on hex_id!
    activate(&mut engine, me(), crown_id, 0);
    aim_object(&mut engine, me(), aura_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(hex_id)
    );
}

#[test]
fn breath_of_fury_runs_then_if_moved_only_when_attachment_actually_changes_host_cr_701_3b() {
    static BREATH_AB: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::AttachedToBySource),
            &[
                Effect::SacrificeEvent,
                Effect::MoveAttachment {
                    attachment: TargetSpec::ThisObject,
                    recipient_filter: &Filter::YOUR_CREATURE,
                    must_change_recipient: false,
                    optional: false,
                    then_if_moved: &[
                        Effect::UntapAll {
                            filter: &Filter::YOUR_CREATURE,
                        },
                        Effect::AdditionalCombatPhase {
                            followed_by_main_phase: false,
                        },
                    ],
                },
            ]
        ),
    ];
    static NO_SAC_LOCK_AB: &[AbilityDef] = &[static_ability!(
        Filter::ControlledByYou,
        Modifier::CantBeSacrificed {
            opponents_only: false
        }
    )];

    let mut breath_face = face("Breath Probe", "{0}", TypeSet::ENCHANTMENT);
    breath_face.subtypes = AURA_SUBTYPES;
    let breath = card(C0, breath_face, KeywordSet::EMPTY, BREATH_AB);
    let runner = card(
        C1,
        creature_face("Goblin Runner", "{0}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let reserve = card(
        C2,
        creature_face("Goblin Reserve", "{0}", 3, 3),
        KeywordSet::HASTE,
        &[],
    );
    let no_sac = card(
        C3,
        face("Assault Suit Lock", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        NO_SAC_LOCK_AB,
    );

    // Case A: Normal Breath of Fury — sacrifice runner, move Breath to tapped reserve,
    // untap reserve, and enter additional combat phase!
    {
        let mut engine = bench(
            14_607,
            vec![breath, runner, reserve],
            [Seat::with(&[C1, C2]).holding(&[C0]), Seat::default()],
        );
        to_main(&mut engine, me());
        let runner_id = the(&engine, ZoneLocation::Battlefield, C1);
        let reserve_id = the(&engine, ZoneLocation::Battlefield, C2);
        let breath_id = the(&engine, ZoneLocation::Hand(me()), C0);

        engine
            .state
            .object_mut(reserve_id)
            .unwrap()
            .status
            .insert(Status::TAPPED);

        engine
            .apply(me(), PlayerAction::CastSpell { card: breath_id })
            .unwrap();
        aim_object(&mut engine, me(), runner_id);
        settle(&mut engine, me());

        // Attack with runner_id in combat 1.
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
        );
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(runner_id, Defender::Player(them()))],
                },
            )
            .unwrap();
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == me()),
        );

        // Runner was sacrificed; reserve_id is the sole legal recipient!
        assert_eq!(
            engine.state().object(runner_id).unwrap().zone,
            crate::zone::Zone::Graveyard
        );
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![reserve_id],
                },
            )
            .unwrap();

        // Advance to the ADDITIONAL ChooseAttackers step granted by then_if_moved!
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
        );
        assert!(
            !engine
                .state()
                .object(reserve_id)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
        assert_eq!(
            engine.state().object(breath_id).unwrap().attached_to,
            Some(reserve_id)
        );
        assert_eq!(life(&engine, them()), 18, "20 - 2 from first combat");
    }

    // Case B (CR 701.3b): CantBeSacrificed keeps runner_id alive on the battlefield!
    // Re-choosing runner_id (the permanent Breath is ALREADY attached to) does nothing
    // (CR 701.3b) and must NOT run then_if_moved (reserve stays tapped, no extra combat)!
    {
        let mut engine = bench(
            14_608,
            vec![breath, runner, reserve, no_sac],
            [Seat::with(&[C1, C2, C3]).holding(&[C0]), Seat::default()],
        );
        to_main(&mut engine, me());
        let runner_id = the(&engine, ZoneLocation::Battlefield, C1);
        let reserve_id = the(&engine, ZoneLocation::Battlefield, C2);
        let breath_id = the(&engine, ZoneLocation::Hand(me()), C0);
        engine
            .state
            .object_mut(reserve_id)
            .unwrap()
            .status
            .insert(Status::TAPPED);

        engine
            .apply(me(), PlayerAction::CastSpell { card: breath_id })
            .unwrap();
        aim_object(&mut engine, me(), runner_id);
        settle(&mut engine, me());

        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
        );
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(runner_id, Defender::Player(them()))],
                },
            )
            .unwrap();
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == me()),
        );

        assert_eq!(
            engine.state().object(runner_id).unwrap().zone,
            crate::zone::Zone::Battlefield,
            "CantBeSacrificed kept runner on the battlefield"
        );
        // Re-select runner_id (same host): CR 701.3b says nothing happens, so then_if_moved skips!
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![runner_id],
                },
            )
            .unwrap();

        // Stop at either an unexpected ChooseAttackers or SecondMain: must reach SecondMain directly!
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
                || (holds_priority_in(e, me(), crate::turn::Step::Main)
                    && e.state().turn.phase == Phase::SecondMain)
        });
        assert!(
            !matches!(engine.pending(), Pending::ChooseAttackers { .. }),
            "no additional combat phase queued when attachment stayed on same permanent"
        );
        assert!(
            engine
                .state()
                .object(reserve_id)
                .unwrap()
                .status
                .contains(Status::TAPPED),
            "reserve was NOT untapped because Breath did not move to a new permanent (CR 701.3b)"
        );
    }
}

#[test]
fn armed_and_armored_equipment_move_attachment_checks_creature_type_and_optional_decline() {
    static EQUIP_FILTER: Filter = Filter::HasSubtype(artifact::EQUIPMENT);
    static ARMED_AB: &[AbilityDef] = &[free(
        &[Effect::MoveAttachment {
            attachment: TargetSpec::Object(&EQUIP_FILTER),
            recipient_filter: &Filter::ControlledByYou,
            must_change_recipient: false,
            optional: true,
            then_if_moved: &[Effect::GainLife {
                amount: Amount::Fixed(4),
            }],
        }],
        Some(TargetReq::one(TargetSpec::Object(&EQUIP_FILTER))),
    )];
    static EQUIP_STATIC_AB: &[AbilityDef] = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
        Modifier::ModifyPT(3, 1)
    )];
    static CREW_ZERO_AB: &[AbilityDef] = &[crew!(0)];
    static PRO_ARTIFACT_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&Filter::ARTIFACT)
    )];

    let rig = card(
        C0,
        face("Armory Rig", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ARMED_AB,
    );
    let mut blade_face = face("War Blade", "{0}", TypeSet::ARTIFACT);
    blade_face.subtypes = EQUIPMENT_SUBTYPES;
    let blade = card(C1, blade_face, KeywordSet::EMPTY, EQUIP_STATIC_AB);

    let mut vehicle_a = creature_face("Idle Skiff", "{0}", 4, 4);
    vehicle_a.types = TypeSet::ARTIFACT;
    vehicle_a.subtypes = VEHICLE_SUBTYPES;
    let idle_vehicle = card(C2, vehicle_a, KeywordSet::EMPTY, CREW_ZERO_AB);

    let mut vehicle_b = creature_face("Crewed Skiff", "{0}", 4, 4);
    vehicle_b.types = TypeSet::ARTIFACT;
    vehicle_b.subtypes = VEHICLE_SUBTYPES;
    let active_vehicle = card(C3, vehicle_b, KeywordSet::EMPTY, CREW_ZERO_AB);

    let pro_art_ally = card(
        C4,
        creature_face("Relic Warder", "{0}", 2, 2),
        KeywordSet::EMPTY,
        PRO_ARTIFACT_AB,
    );

    let mut engine = bench(
        14_609,
        vec![rig, blade, idle_vehicle, active_vehicle, pro_art_ally],
        [Seat::with(&[C0, C1, C2, C3, C4]), Seat::default()],
    );
    to_main(&mut engine, me());
    let rig_id = the(&engine, ZoneLocation::Battlefield, C0);
    let blade_id = the(&engine, ZoneLocation::Battlefield, C1);
    let active_veh_id = the(&engine, ZoneLocation::Battlefield, C3);

    // Crew only active_vehicle (idle_vehicle remains non-creature artifact; pro_art_ally has pro-artifact).
    activate(&mut engine, me(), active_veh_id, 0);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    settle(&mut engine, me());

    // 1. Activate Armory Rig, then DECLINE optional MoveAttachment (vec![]):
    //    blade stays unattached and then_if_moved (GainLife 4) does NOT run!
    activate(&mut engine, me(), rig_id, 0);
    aim_object(&mut engine, me(), blade_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        total,
    } = engine.pending()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(
        (*player, options.as_slice(), *min, *max, *prompt, total),
        (
            me(),
            &[active_veh_id][..],
            0,
            1,
            ChoicePrompt::Generic,
            &None
        ),
        "only the crewed Vehicle is a legal non-protected creature recipient (CR 301.5c)"
    );
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(blade_id).unwrap().attached_to, None);
    assert_eq!(life(&engine, me()), 20, "declining skips then_if_moved");

    // 2. Activate Armory Rig again and choose active_veh_id: attaches +3/+1 and gains 4 life!
    activate(&mut engine, me(), rig_id, 0);
    aim_object(&mut engine, me(), blade_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![active_veh_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(blade_id).unwrap().attached_to,
        Some(active_veh_id)
    );
    assert_eq!(
        engine
            .state()
            .object(active_veh_id)
            .unwrap()
            .characteristics()
            .power,
        Some(7)
    );
    assert_eq!(life(&engine, me()), 24);
}
