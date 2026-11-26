//! Wave 17b deterministic gameplay tests:
//! 1. `Effect::Empower` (`CR 701.71a`): dynamic loyalty counts, Jace token creation vs reuse,
//!    non-token Jace exclusion, `0`-count SBA cleanup, and token loyalty abilities.
//! 2. `Amount::DistinctPowersAmong` & `Effect::ChooseDistinctPowersOnBattlefield` (`CR 208.3`):
//!    Coven threshold across negative, zero, and duplicate powers, plus stepwise distinct-power
//!    battlefield selection (*Sigarda's Vanguard*, *Sigardian Zealot*).
//! 3. `Effect::PutAllCountersFrom` & `Effect::PutAllCountersFromIntoFound` (`CR 122.8`):
//!    full multi-counter LKI transfer on death (*Star Pupil*, *The Ozolith*), cost sacrifice
//!    (*Zack Fair*), live donor to entering `EventObject` (*Denry Klin*), and created tokens
//!    (*Ambitious Augmenter*).
//! 4. Dynamic `etbCounter` amounts & assigned counters: `CommanderCasts`,
//!    `HighestControlledCount`, `HighestCardsDrawnThisTurn`, `TotalManaValueInZone`, and
//!    `DistinctExactManaCostsInZone` (*Embodiment of Agonies*: `{2}{B}` vs `{1}{B}{B}` vs `{0}` vs
//!    costless land).
//! 5. `FaceDef::mayhem`, `CastModeKind::Mayhem`, & `Condition::MayhemCostPaid` (`CR 702.187`):
//!    discard-this-turn gating, timing rules, permanent resolution without exile, single-use
//!    graveyard incarnation on instants/sorceries (*Sandman's Quicksand*), and turn expiry.

#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{activated, loyalty, spell, triggered};
use baylee_cards_dsl::{
    Amount, AmountCmp, Condition, CounterDonor, CounterKind, Duration, EnterModifier, Filter,
    PlayerRel, TargetReq, TargetSpec, Trigger, ZoneSel, cost, counters,
};
use baylee_core::generated::subtypes;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_860;
const C1: u32 = 99_861;
const C2: u32 = 99_862;
const C3: u32 = 99_863;
const C4: u32 = 99_864;
const C5: u32 = 99_865;
const C6: u32 = 99_866;
const C7: u32 = 99_867;

fn refresh(engine: &mut Bench) {
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
    refresh(engine);
}

fn move_to_graveyard(engine: &mut Bench, owner: PlayerId, id: ObjectId) {
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("moved to graveyard");
    refresh(engine);
}

fn choose_cast_mode_kind(engine: &mut Bench, seat: PlayerId, kind: CastModeKind) {
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let pos = options
        .iter()
        .position(|opt| opt.kind == kind)
        .unwrap_or_else(|| panic!("expected {kind:?} in {options:?}"));
    engine
        .apply(seat, PlayerAction::ChooseMode(pos))
        .expect("cast mode accepted");
}

fn counters_on(engine: &Bench, id: ObjectId, kind: CounterKind) -> u16 {
    engine.state().object(id).unwrap().counters.get(kind)
}

fn cast_spell(engine: &mut Bench, seat: PlayerId, id: ObjectId) {
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(
        legal.castable.contains(&id),
        "expected {id:?} castable by {seat:?}, got {:?}",
        legal.castable
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: id })
        .expect("cast accepted");
}

#[test]
fn wave17b_empower_dynamic_amount_token_creation_reuse_and_loyalty_abilities() {
    static YOUR_CREATURES: Amount = Amount::CountOf {
        filter: &Filter::YOUR_CREATURE,
        zone: ZoneSel::Battlefield,
    };
    static EMPOWER_SPELL_AB: &[AbilityDef] = &[spell!(&[Effect::Empower {
        token: &baylee_cards::tokens::JACE_PLANESWALKER_BLUE,
        amount: YOUR_CREATURES,
    }])];
    static REAL_JACE_SUBS: &[baylee_core::ids::SubtypeId] = &[subtypes::planeswalker::JACE];
    static REAL_JACE_AB: &[AbilityDef] = &[loyalty!(
        1,
        &[Effect::GainLife {
            amount: Amount::Fixed(1)
        }]
    )];

    let empower_sorcery = card(
        C0,
        face("Empower Rally", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        EMPOWER_SPELL_AB,
    );
    let bear = card(
        C1,
        creature_face("Scholar Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let real_jace = card(
        C2,
        FaceDef {
            subtypes: REAL_JACE_SUBS,
            loyalty: Some(3),
            ..face("Card Jace", "{0}", TypeSet::PLANESWALKER)
        },
        KeywordSet::EMPTY,
        REAL_JACE_AB,
    );

    // 1. Empower with 2 creatures and a non-token Jace on board: must ignore the non-token Jace,
    //    create a blue Jace token, and put 2 loyalty counters on that token.
    let mut engine = bench(
        17_201,
        vec![empower_sorcery, bear, real_jace],
        [
            Seat::with(&[C1, C1]).holding(&[C2, C0, C0]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let card_jace = the(&engine, ZoneLocation::Hand(me()), C2);
    cast_spell(&mut engine, me(), card_jace);
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, card_jace, CounterKind::Loyalty), 3);

    let spells = objects(&engine, ZoneLocation::Hand(me()), C0);
    cast_spell(&mut engine, me(), spells[0]);
    settle(&mut engine, me());

    assert_eq!(
        counters_on(&engine, card_jace, CounterKind::Loyalty),
        3,
        "non-token Jace must not receive Empower loyalty counters"
    );
    let jace_tokens: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| engine.state().object(id).unwrap().card.is_none())
        .collect();
    assert_eq!(jace_tokens.len(), 1);
    let jace_tok = jace_tokens[0];
    assert_eq!(counters_on(&engine, jace_tok, CounterKind::Loyalty), 2);

    // 2. Second Empower while controlling that Jace token reuses it instead of creating another.
    cast_spell(&mut engine, me(), spells[1]);
    settle(&mut engine, me());
    let jace_tokens_after: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| engine.state().object(id).unwrap().card.is_none())
        .collect();
    assert_eq!(jace_tokens_after, vec![jace_tok]);
    assert_eq!(counters_on(&engine, jace_tok, CounterKind::Loyalty), 4);

    // 3. Activate the Jace token's [-3]: Draw a card loyalty ability.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: jace_tok,
                ability_index: 1,
            },
        )
        .expect("[-3] draw ability legal at 4 loyalty");
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, jace_tok, CounterKind::Loyalty), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1
    );

    // 4. Empower 0 with no controlled Jace token creates a 0-loyalty token that dies to SBAs.
    let mut engine_zero = bench(
        17_202,
        vec![empower_sorcery],
        [Seat::default().holding(&[C0]), Seat::default()],
    );
    to_main(&mut engine_zero, me());
    let s0 = the(&engine_zero, ZoneLocation::Hand(me()), C0);
    cast_spell(&mut engine_zero, me(), s0);
    settle(&mut engine_zero, me());
    assert_eq!(
        engine_zero
            .state()
            .battlefield_seen()
            .filter(|&id| engine_zero.state().object(id).unwrap().card.is_none())
            .count(),
        0,
        "0-loyalty Jace token created by Empower 0 is cleaned up by SBAs"
    );
}

#[test]
fn wave17b_distinct_powers_coven_negative_zero_and_choose_distinct_powers() {
    static DISTINCT_YOUR_POWERS: Amount = Amount::DistinctPowersAmong(&Filter::YOUR_CREATURE);
    static ALL_DISTINCT_POWERS: Amount = Amount::DistinctPowersAmong(&Filter::CREATURE);
    static THREE: Amount = Amount::Fixed(3);
    static COVEN: Condition = Condition::AmountCompare {
        left: &DISTINCT_YOUR_POWERS,
        op: AmountCmp::AtLeast,
        right: &THREE,
    };
    static COVEN_DRAW_AB: &[AbilityDef] = &[spell!(&[Effect::IfCondition {
        condition: COVEN,
        then: &[Effect::DrawCards {
            amount: DISTINCT_YOUR_POWERS,
        }],
        otherwise: &[],
    }])];
    static SHRINK_AB: &[AbilityDef] = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::NegXFixed(3),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static ZEALOT_AB: &[AbilityDef] = &[spell!(&[Effect::ChooseDistinctPowersOnBattlefield {
        chooser: PlayerRel::You,
        filter: &Filter::CREATURE,
        min: 0,
        max: ALL_DISTINCT_POWERS,
        then: &[Effect::PumpEventObject {
            power: Amount::Fixed(3),
            toughness: Amount::Fixed(3),
            keywords: KeywordSet::VIGILANCE,
            duration: Duration::UntilEndOfTurn,
        }],
    }])];

    let coven_probe = card(
        C0,
        face("Coven Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        COVEN_DRAW_AB,
    );
    let shrink = card(
        C1,
        face("Shrink Ray", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        SHRINK_AB,
    );
    let bear = card(
        C2,
        creature_face("Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let wall = card(
        C3,
        creature_face("Zero Wall", "{0}", 0, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let relic = card(
        C4,
        face("Noncreature Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let zealot = card(
        C5,
        face("Zealot Rally", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        ZEALOT_AB,
    );

    // Two 2/2s + one 0/4 + noncreature artifact => only 2 distinct powers (0 and 2) -> Coven false.
    let mut engine = bench(
        17_203,
        vec![coven_probe, shrink, bear, wall, relic, zealot],
        [
            Seat::with(&[C2, C2, C3, C4]).holding(&[C0, C1, C0, C5]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let probes = objects(&engine, ZoneLocation::Hand(me()), C0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    cast_spell(&mut engine, me(), probes[0]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before - 1,
        "2 distinct powers must not satisfy Coven (>= 3)"
    );

    // Shrink one 2/2 by -3/-0 to -1/2 => powers are {-1, 0, 2} => 3 distinct powers -> draws 3!
    let bears = objects(&engine, ZoneLocation::Battlefield, C2);
    let shrink_card = the(&engine, ZoneLocation::Hand(me()), C1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: shrink_card })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bears[0]],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bears[0])
            .unwrap()
            .characteristics()
            .power,
        Some(-1)
    );

    let hand_mid = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    cast_spell(&mut engine, me(), probes[1]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_mid - 1 + 3,
        "negative (-1), zero (0), and positive (2) powers count as 3 distinct powers"
    );

    // ChooseDistinctPowersOnBattlefield: add another 2/2 so powers are {-1, 0, 2, 2};
    // picking bears[1] (power 2) must exclude the other 2/2 from the next step!
    let mut engine2 = bench(
        17_204,
        vec![bear, wall, zealot],
        [Seat::with(&[C2, C2, C3]).holding(&[C5]), Seat::default()],
    );
    to_main(&mut engine2, me());
    let bs = objects(&engine2, ZoneLocation::Battlefield, C2);
    let w = the(&engine2, ZoneLocation::Battlefield, C3);
    let z = the(&engine2, ZoneLocation::Hand(me()), C5);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: z })
        .unwrap();
    engine2.apply(me(), PlayerAction::PassPriority).unwrap();
    engine2.apply(them(), PlayerAction::PassPriority).unwrap();

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine2.pending().clone()
    else {
        panic!("expected stepwise ChooseCards, got {:?}", engine2.pending());
    };
    assert_eq!((min, max), (0, 1));
    assert_eq!(options.len(), 3);
    engine2
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bs[0]],
            },
        )
        .expect("pick first 2/2");

    let Pending::ChooseCards { options: step2, .. } = engine2.pending().clone() else {
        panic!(
            "expected second ChooseCards step, got {:?}",
            engine2.pending()
        );
    };
    assert_eq!(
        step2,
        vec![w],
        "second 2/2 must be excluded once power 2 is already chosen"
    );
    engine2
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![w] })
        .expect("pick 0/4 wall");
    settle(&mut engine2, me());

    assert_eq!(
        engine2
            .state()
            .object(bs[0])
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
    assert_eq!(
        engine2
            .state()
            .object(bs[1])
            .unwrap()
            .characteristics()
            .power,
        Some(2),
        "unchosen duplicate-power creature stays unpumped"
    );
    assert_eq!(
        engine2.state().object(w).unwrap().characteristics().power,
        Some(3)
    );
}

#[test]
fn wave17b_put_all_counters_from_lki_dies_sacrifice_denry_and_found_token() {
    static PUPIL_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::PutAllCountersFrom {
                donor: CounterDonor::Event,
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE)))
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::GainLife {
                amount: Amount::Fixed(1)
            }]
        ),
    ];
    static ZACK_AB: &[AbilityDef] = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::PutAllCountersFrom {
            donor: CounterDonor::Source,
            target: TargetSpec::Object(&Filter::YOUR_CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE)))
    )];
    static DENRY_FILTER: Filter = Filter::And(&[
        Filter::Another,
        Filter::ControlledByYou,
        Filter::CREATURE,
        Filter::Not(&Filter::IsToken),
    ]);
    static DENRY_AB: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&DENRY_FILTER),
        &[Effect::PutAllCountersFrom {
            donor: CounterDonor::Source,
            target: TargetSpec::EventObject,
        }]
    )];
    static AUGMENTER_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::Dies(&Filter::This),
            &[
                Effect::CreateToken {
                    token: &baylee_cards::tokens::CONSTRUCT_ARTIFACT_0_0,
                },
                Effect::PutAllCountersFromIntoFound {
                    donor: CounterDonor::Event,
                },
            ]
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::GainLife {
                amount: Amount::Fixed(1)
            }]
        ),
    ];

    let pupil = card(
        C0,
        creature_face("Star Pupil", "{0}", 1, 1),
        KeywordSet::EMPTY,
        PUPIL_AB,
    );
    let zack = card(
        C1,
        creature_face("Zack Probe", "{0}", 1, 1),
        KeywordSet::EMPTY,
        ZACK_AB,
    );
    let denry = card(
        C2,
        creature_face("Denry Probe", "{0}", 2, 2),
        KeywordSet::EMPTY,
        DENRY_AB,
    );
    let augmenter = card(
        C3,
        creature_face("Augmenter Probe", "{0}", 1, 1),
        KeywordSet::EMPTY,
        AUGMENTER_AB,
    );
    let ally = card(
        C4,
        creature_face("Ally Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // 1. Dies LKI transfer (Star Pupil) + SacrificeSelf Source LKI transfer (Zack).
    let mut engine = bench(
        17_205,
        vec![pupil, zack, ally],
        [Seat::with(&[C0, C1, C4]), Seat::default()],
    );
    to_main(&mut engine, me());
    let pupil_id = the(&engine, ZoneLocation::Battlefield, C0);
    let zack_id = the(&engine, ZoneLocation::Battlefield, C1);
    let ally_id = the(&engine, ZoneLocation::Battlefield, C4);
    crate::replacement::record_counters(&mut engine.state, pupil_id, CounterKind::P1P1, 3);
    crate::replacement::record_counters(&mut engine.state, pupil_id, CounterKind::Shield, 2);
    crate::replacement::record_counters(&mut engine.state, zack_id, CounterKind::Flying, 1);
    crate::replacement::record_counters(&mut engine.state, zack_id, CounterKind::P1P1, 2);
    refresh(&mut engine);

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: pupil_id,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![ally_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, ally_id, CounterKind::P1P1), 3);
    assert_eq!(counters_on(&engine, ally_id, CounterKind::Shield), 2);

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: zack_id,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![ally_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, ally_id, CounterKind::P1P1), 5);
    assert_eq!(counters_on(&engine, ally_id, CounterKind::Flying), 1);

    // 2. Denry Klin (Source -> EventObject) + Ambitious Augmenter (Event -> Found 0/0 token).
    let mut engine2 = bench(
        17_206,
        vec![denry, augmenter, ally],
        [Seat::with(&[C2, C3]).holding(&[C4]), Seat::default()],
    );
    to_main(&mut engine2, me());
    let denry_id = the(&engine2, ZoneLocation::Battlefield, C2);
    let aug_id = the(&engine2, ZoneLocation::Battlefield, C3);
    crate::replacement::record_counters(&mut engine2.state, denry_id, CounterKind::P1P1, 2);
    crate::replacement::record_counters(&mut engine2.state, denry_id, CounterKind::Vigilance, 1);
    crate::replacement::record_counters(&mut engine2.state, aug_id, CounterKind::P1P1, 3);
    refresh(&mut engine2);

    let ally_hand = the(&engine2, ZoneLocation::Hand(me()), C4);
    cast_spell(&mut engine2, me(), ally_hand);
    settle(&mut engine2, me());
    assert_eq!(counters_on(&engine2, ally_hand, CounterKind::P1P1), 2);
    assert_eq!(counters_on(&engine2, ally_hand, CounterKind::Vigilance), 1);

    engine2
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: aug_id,
                ability_index: 1,
            },
        )
        .unwrap();
    settle(&mut engine2, me());
    let tokens: Vec<ObjectId> = engine2
        .state()
        .battlefield_seen()
        .filter(|&id| engine2.state().object(id).unwrap().card.is_none())
        .collect();
    assert_eq!(
        tokens.len(),
        1,
        "0/0 token survives with copied +1/+1 counters"
    );
    assert_eq!(counters_on(&engine2, tokens[0], CounterKind::P1P1), 3);
}

#[test]
fn wave17b_etb_counter_dynamic_amounts_and_exact_mana_costs() {
    static NONLAND: Filter = Filter::Not(&Filter::HasType(TypeSet::LAND));
    static INSTANT_OR_SORCERY: Filter = Filter::Or(&[
        Filter::HasType(TypeSet::INSTANT),
        Filter::HasType(TypeSet::SORCERY),
    ]);
    static JOURNAL_MODS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: counters::SUSPECT,
        amount: Amount::HighestControlledCount {
            among: PlayerRel::EachPlayer,
            filter: &Filter::CREATURE,
        },
    }];
    static SPONGE_MODS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: CounterKind::P1P1,
        amount: Amount::HighestCardsDrawnThisTurn(PlayerRel::EachOpponent),
    }];
    static INFERNO_MODS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: CounterKind::P1P1,
        amount: Amount::TotalManaValueInZone {
            zone: ZoneSel::GraveyardYou,
            filter: &INSTANT_OR_SORCERY,
        },
    }];
    static EMBODIMENT_MODS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: CounterKind::P1P1,
        amount: Amount::DistinctExactManaCostsInZone {
            zone: ZoneSel::GraveyardYou,
            filter: &NONLAND,
        },
    }];
    static SWARMLORD_BASE: Amount = Amount::CommanderCasts(PlayerRel::You);
    static SWARMLORD_MODS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: CounterKind::P1P1,
        amount: Amount::Times {
            base: &SWARMLORD_BASE,
            factor: 2,
        },
    }];

    let journal = card(
        C0,
        FaceDef {
            enter_modifiers: JOURNAL_MODS,
            ..face("Journal Probe", "{0}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let sponge = card(
        C1,
        FaceDef {
            enter_modifiers: SPONGE_MODS,
            ..creature_face("Sponge Probe", "{0}", 1, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let inferno = card(
        C2,
        FaceDef {
            enter_modifiers: INFERNO_MODS,
            ..creature_face("Inferno Probe", "{0}", 0, 0)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let embodiment = card(
        C3,
        FaceDef {
            enter_modifiers: EMBODIMENT_MODS,
            ..creature_face("Embodiment Probe", "{0}", 0, 0)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let swarmlord = card(
        C4,
        FaceDef {
            enter_modifiers: SWARMLORD_MODS,
            ..creature_face("Swarmlord Probe", "{0}", 5, 5)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let spell_2b = card(
        C5,
        face("Spell TwoB", "{2}{B}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &[],
    );
    let spell_1bb = card(
        C6,
        face("Spell OneBB", "{1}{B}{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &[],
    );
    let costless_suspend = card(
        C7,
        FaceDef {
            mana_cost: ManaCost::ZERO,
            types: TypeSet::SORCERY,
            castable_from_hand: false,
            ..land_face("No Mana Cost Spell")
        },
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        17_207,
        vec![
            journal,
            sponge,
            inferno,
            embodiment,
            swarmlord,
            spell_2b,
            spell_1bb,
            costless_suspend,
        ],
        [
            Seat::default().holding(&[C0, C1, C2, C3, C4, C5, C5, C6, C7]),
            Seat::with(&[C1, C1, C1]),
        ],
    );
    to_main(&mut engine, me());

    // Put two {2}{B} sorceries, one {1}{B}{B} instant, and one costless sorcery in our graveyard.
    for &idx in &[C5, C6, C7] {
        for id in objects(&engine, ZoneLocation::Hand(me()), idx) {
            move_to_graveyard(&mut engine, me(), id);
        }
    }
    engine.state.per_turn.draws[usize::from(them().get())] = 4;
    engine.state.commander_casts[usize::from(me().get())] = 3;
    refresh(&mut engine);

    let j_id = the(&engine, ZoneLocation::Hand(me()), C0);
    cast_spell(&mut engine, me(), j_id);
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, j_id, counters::SUSPECT), 3);

    let sp_id = the(&engine, ZoneLocation::Hand(me()), C1);
    cast_spell(&mut engine, me(), sp_id);
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, sp_id, CounterKind::P1P1), 4);

    // Total MV in graveyard = 3 + 3 + 3 + 0 = 9.
    let inf_id = the(&engine, ZoneLocation::Hand(me()), C2);
    cast_spell(&mut engine, me(), inf_id);
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, inf_id, CounterKind::P1P1), 9);

    // Distinct exact mana costs = {{2}{B}, {1}{B}{B}} => 2 (duplicate {2}{B} collapses; costless ignored!).
    let emb_id = the(&engine, ZoneLocation::Hand(me()), C3);
    cast_spell(&mut engine, me(), emb_id);
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, emb_id, CounterKind::P1P1), 2);

    // Swarmlord: 2 * 3 commander casts = 6 +1/+1 counters.
    let sw_id = the(&engine, ZoneLocation::Hand(me()), C4);
    cast_spell(&mut engine, me(), sw_id);
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, sw_id, CounterKind::P1P1), 6);
}

#[test]
fn wave17b_mayhem_graveyard_casting_timing_incarnation_and_mayhem_cost_paid() {
    static QUICKSAND_AB: &[AbilityDef] = &[spell!(&[Effect::IfCondition {
        condition: Condition::MayhemCostPaid,
        then: &[Effect::PumpFilter {
            filter: &Filter::CREATURE,
            controlled_by: Some(PlayerRel::Opponent),
            power: Amount::NegXFixed(2),
            toughness: Amount::NegXFixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        otherwise: &[Effect::PumpFilter {
            filter: &Filter::CREATURE,
            controlled_by: None,
            power: Amount::NegXFixed(2),
            toughness: Amount::NegXFixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    }])];
    static DISCARD_ONE_AB: &[AbilityDef] = &[spell!(&[Effect::DiscardCount {
        who: PlayerRel::You,
        count: Amount::Fixed(1),
    }])];

    let goblinoid = card(
        C0,
        FaceDef {
            mayhem: Some(ManaCost::parse("{1}{R}")),
            ..creature_face("Mayhem Raider", "{4}{R}", 5, 4)
        },
        KeywordSet::MAYHEM,
        &[],
    );
    let quicksand = card(
        C1,
        FaceDef {
            mayhem: Some(ManaCost::parse("{1}{B}")),
            ..face("Quicksand Probe", "{2}{B}", TypeSet::SORCERY)
        },
        KeywordSet::MAYHEM,
        QUICKSAND_AB,
    );
    let discard_spell = card(
        C2,
        face("Self Rummage", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        DISCARD_ONE_AB,
    );
    let bear = card(
        C3,
        creature_face("Sturdy Bear", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        17_208,
        vec![goblinoid, quicksand, discard_spell, bear],
        [
            Seat::with(&[C3]).holding(&[C0, C0, C1, C2, C2]),
            Seat::with(&[C3]),
        ],
    );
    to_main(&mut engine, me());
    let my_bear = engine
        .state()
        .battlefield_seen()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let opp_bear = engine
        .state()
        .battlefield_seen()
        .find(|&id| id != my_bear)
        .unwrap();

    // 1. A card put directly into the graveyard without being discarded is NOT castable via Mayhem.
    let goblins = objects(&engine, ZoneLocation::Hand(me()), C0);
    move_to_graveyard(&mut engine, me(), goblins[0]);
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card: goblins[0] })
            .is_err(),
        "milled/moved card was not discarded this turn and cannot use Mayhem"
    );

    // 2. Discard the second copy this turn -> castable from graveyard for {1}{R}, enters battlefield!
    let discards = objects(&engine, ZoneLocation::Hand(me()), C2);
    cast_spell(&mut engine, me(), discards[0]);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![goblins[1]],
            },
        )
        .expect("discard goblinoid");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(goblins[1]).unwrap().zone,
        Zone::Graveyard
    );

    engine
        .apply(me(), PlayerAction::CastSpell { card: goblins[1] })
        .expect("discarded Mayhem creature is castable for {1}{R}");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::Mayhem);
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(goblins[1]).unwrap().zone,
        Zone::Battlefield,
        "Mayhem permanent resolves onto the battlefield rather than exiling"
    );

    // 3. Discard Quicksand and cast via Mayhem for {1}{B}: Condition::MayhemCostPaid holds, so ONLY
    //    opponent's bear shrinks; afterward Quicksand is in graveyard as a new incarnation and
    //    cannot be cast via Mayhem a second time!
    let qs_id = the(&engine, ZoneLocation::Hand(me()), C1);
    cast_spell(&mut engine, me(), discards[1]);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![qs_id],
            },
        )
        .expect("discard quicksand");
    settle(&mut engine, me());

    add_pool(&mut engine, me(), ManaColor::Black, 4);
    engine
        .apply(me(), PlayerAction::CastSpell { card: qs_id })
        .expect("discarded Quicksand castable via Mayhem");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::Mayhem);
    }
    settle(&mut engine, me());

    assert_eq!(
        engine
            .state()
            .object(my_bear)
            .unwrap()
            .characteristics()
            .power,
        Some(3),
        "your creature is spared when MayhemCostPaid is true"
    );
    assert_eq!(
        engine
            .state()
            .object(opp_bear)
            .unwrap()
            .characteristics()
            .power,
        Some(1),
        "opponent's creature gets -2/-2 when MayhemCostPaid is true"
    );
    assert_eq!(
        engine.state().object(qs_id).unwrap().zone,
        Zone::Graveyard,
        "Mayhem sorcery goes to graveyard (not exile) on resolution"
    );
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card: qs_id })
            .is_err(),
        "new graveyard incarnation after resolving was not discarded and cannot use Mayhem again"
    );
}
