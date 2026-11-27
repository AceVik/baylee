//! Deterministic rule & snapshot-hash tests for:
//! 1. `Modifier::UntapDuringUntapStep` (CR 502.3; Seedborn Muse, Unwinding Clock,
//!    stun counters, Stasis skipped untap, `DoesNotUntap`).
//! 2. `Modifier::{AttackManaTax, AttackPhyrexianTax, CantAttackUnlessYouReturn,
//!    CantAttackUnlessYouTap, BlockManaTax, CantBlockUnlessYouTap}` (CR 508.1d,
//!    CR 509.1d; Propaganda, Onakke Oathkeeper, Collective Restraint, Nils,
//!    Norn's Annex, Floodtide Serpent, Hollow Warrior, Cowed by Wisdom).
//! 3. `Condition::{WasForetold, WasSneaked, TreasureManaWasSpent, RememberedMatches,
//!    OpponentLifeAtMost, PlayerAttackedYouThisTurn, YouAreRememberedPlayer}` &
//!    `Amount::{Domain, TotalCountersOnSubject, PlayerCounters, RingTemptedCount,
//!    ResolutionAccumulatedNumber}` & Treasure mana provenance.
//! 4. `Effect::{ChoosePlayerEx, AddManaChoiceFor}`, `EnterModifier::{ChooseTwoPlayers,
//!    SecretlyChooseOpponent, OpponentChoosesCreatureType,
//!    NoteMostPrevalentCreatureTypeInOpponentLibrary}`, `CostPart::RevealChosenPlayer`,
//!    `PlayerRel::{SecondChosenBySource, YouAndTargetOpponent}`.
//! 5. `Effect::{ChooseNumberAtRandom, SecretChooseNumbers, SecretNumberGuess,
//!    AccumulateChosenNumber, RepeatEachPlayerPayUntilNone}`,
//!    `RepeatDomain::EachPlayerStartingWithYou`, and
//!    `EnterModifier::{ChooseNumberAtRandom, SecretlyChooseNumber}`.

#![allow(clippy::too_many_lines)] // comprehensive multi-case rule tests

use super::*;
use crate::choice::{CastModeKind, NumberPrompt, YesNoPrompt};
use crate::object::{Rider, Status};
use crate::zone::ZoneLocation;
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, AttackTaxTarget, Condition, CostPart, CounterKind, EnterModifier, Filter,
    ManaDistribution, ManaSource, Modifier, NumberChoiceKind, PlayerFilter, PlayerRel,
    RepeatDomain, SecretNumberChoice, Trigger, VariablePayKind, ZoneSel,
};
use baylee_core::generated::subtypes;
use baylee_core::ids::{Defender, SubtypeId};
use baylee_core::mana::ManaColor;

static FIVE_COLORS: &[ManaColor] = &[
    ManaColor::White,
    ManaColor::Blue,
    ManaColor::Black,
    ManaColor::Red,
    ManaColor::Green,
];

fn seat2() -> PlayerId {
    PlayerId::new(2)
}

fn is_tapped(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

fn add_pool(engine: &mut Bench, seat: PlayerId, color: ManaColor, n: u32) {
    engine.state.players[usize::from(seat.get())]
        .mana_pool
        .add(color, n);
}

fn to_attackers(engine: &mut Bench, seat: PlayerId) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
}

fn to_blockers(engine: &mut Bench, seat: PlayerId) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == seat),
    );
}

fn cast_from_hand(engine: &mut Bench, seat: PlayerId, card_id: ObjectId) {
    let (player, legal) = offer(engine);
    assert_eq!(player, seat, "caster holds priority");
    assert!(
        legal.castable.contains(&card_id),
        "expected {card_id:?} castable, got {:?}",
        legal.castable
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: card_id })
        .expect("cast accepted");
}

fn finish_mana_window(engine: &mut Bench, seat: PlayerId) {
    if matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat) {
        engine
            .apply(seat, PlayerAction::PassPriority)
            .expect("mana window finished");
    }
}

fn tap_all_lands(engine: &mut Bench, seat: PlayerId) {
    while let Pending::Priority { player, legal } = engine.pending().clone() {
        if player != seat {
            break;
        }
        let Some(&land) = legal.mana_abilities.first() else {
            break;
        };
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source: land })
            .expect("tap land for mana");
    }
}

fn basic_land_card(index: u32, name: &'static str, subtype: SubtypeId) -> &'static CardDef {
    let mut f = land_face(name);
    f.subtypes = Box::leak(Box::new([subtype]));
    card(index, f, KeywordSet::EMPTY, &[])
}

fn creature_with_subtype(index: u32, name: &'static str, subtype: SubtypeId) -> &'static CardDef {
    let mut f = creature_face(name, "{1}", 2, 2);
    f.subtypes = Box::leak(Box::new([subtype]));
    card(index, f, KeywordSet::EMPTY, &[])
}

fn face_with_enters(mut f: FaceDef, enters: &'static [EnterModifier]) -> FaceDef {
    f.enter_modifiers = enters;
    f
}

// =========================================================================
// 1. UntapOtherPlayer (CR 502.3)
// =========================================================================

#[test]
fn untap_during_other_players_untap_step_stun_does_not_untap_and_stasis() {
    const SEEDBORN: u32 = 910_001;
    const UNWINDING: u32 = 910_002;
    const BEAR: u32 = 910_003;
    const ROCK: u32 = 910_004;
    const FROZEN: u32 = 910_005;
    const STUNNED: u32 = 910_006;
    const STASIS: u32 = 910_007;

    static UNWINDING_FILTER: Filter = Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou]);
    static SEEDBORN_ABS: &[AbilityDef] = &[static_ability!(
        Filter::ControlledByYou,
        Modifier::UntapDuringUntapStep {
            who: PlayerRel::EachOtherPlayer,
        }
    )];
    static UNWINDING_ABS: &[AbilityDef] = &[static_ability!(
        UNWINDING_FILTER,
        Modifier::UntapDuringUntapStep {
            who: PlayerRel::EachOtherPlayer,
        }
    )];
    static FROZEN_ABS: &[AbilityDef] = &[static_ability!(Filter::This, Modifier::DoesNotUntap)];
    static STASIS_ABS: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::SkipUntapStep {
            who: PlayerRel::EachPlayer,
        }
    )];

    // Part A: Seedborn Muse untaps creature, artifact, and DoesNotUntap during
    // opponent's untap step; removes stun counter instead of untapping;
    // DoesNotUntap still stays tapped on controller's own untap step!
    let cards = vec![
        card(
            SEEDBORN,
            creature_face("Muse", "{3}{G}{G}", 2, 4),
            KeywordSet::EMPTY,
            SEEDBORN_ABS,
        ),
        card(
            BEAR,
            creature_face("Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ROCK,
            face("Rock", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            FROZEN,
            creature_face("Frozen", "{2}", 3, 3),
            KeywordSet::EMPTY,
            FROZEN_ABS,
        ),
        card(
            STUNNED,
            creature_face("Stunned", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine = bench(
        9101,
        cards,
        [
            Seat::with(&[SEEDBORN, BEAR, ROCK, FROZEN, STUNNED]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let bear = the(&engine, ZoneLocation::Battlefield, BEAR);
    let rock = the(&engine, ZoneLocation::Battlefield, ROCK);
    let frozen = the(&engine, ZoneLocation::Battlefield, FROZEN);
    let stunned = the(&engine, ZoneLocation::Battlefield, STUNNED);
    for id in [bear, rock, frozen, stunned] {
        engine
            .state
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }
    engine
        .state
        .object_mut(stunned)
        .unwrap()
        .counters
        .add(CounterKind::Stun, 1);

    // Advance to opponent's main phase (passes through opponent's untap step).
    to_main(&mut engine, them());
    assert!(!is_tapped(&engine, bear), "bear untaps on opponent's untap");
    assert!(!is_tapped(&engine, rock), "rock untaps on opponent's untap");
    assert!(
        !is_tapped(&engine, frozen),
        "CR 502.3: DoesNotUntap only blocks controller's own untap step, not opponent's"
    );
    assert!(
        is_tapped(&engine, stunned),
        "stun counter replaces untapping during opponent's untap step"
    );
    assert_eq!(
        engine
            .state()
            .object(stunned)
            .unwrap()
            .counters
            .get(CounterKind::Stun),
        0,
        "stun counter was removed instead of untapping"
    );

    // Tap Frozen again during opponent's turn; on our own untap step it must stay tapped!
    engine
        .state
        .object_mut(frozen)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    to_main(&mut engine, me());
    assert!(
        is_tapped(&engine, frozen),
        "DoesNotUntap still keeps permanent tapped during controller's own untap step"
    );
    assert!(
        !is_tapped(&engine, stunned),
        "with stun counter gone, stunned creature untaps normally on our turn"
    );

    // Part B: Unwinding Clock filter scoping in 3-player game (only controller's artifacts untap).
    let cards_b = vec![
        card(
            UNWINDING,
            face("Clock", "{4}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            UNWINDING_ABS,
        ),
        card(
            BEAR,
            creature_face("Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ROCK,
            face("Rock", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine_b = bench(
        9102,
        cards_b,
        [
            Seat::with(&[UNWINDING, BEAR, ROCK]),
            Seat::default(),
            Seat::with(&[ROCK]),
        ],
    );
    to_main(&mut engine_b, me());
    let my_bear = the(&engine_b, ZoneLocation::Battlefield, BEAR);
    let rocks = objects(&engine_b, ZoneLocation::Battlefield, ROCK);
    let my_rock = rocks
        .iter()
        .copied()
        .find(|&id| engine_b.state().object(id).unwrap().controller == me())
        .unwrap();
    let seat2_rock = rocks
        .iter()
        .copied()
        .find(|&id| engine_b.state().object(id).unwrap().controller == seat2())
        .unwrap();
    for id in [my_bear, my_rock, seat2_rock] {
        engine_b
            .state
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }
    to_main(&mut engine_b, them());
    assert!(
        !is_tapped(&engine_b, my_rock),
        "controller's artifact untaps"
    );
    assert!(is_tapped(&engine_b, my_bear), "non-artifact stays tapped");
    assert!(
        is_tapped(&engine_b, seat2_rock),
        "third player's artifact stays tapped during seat 1's untap"
    );

    // Part C: Stasis (`SkipUntapStep`) prevents untapping during the skipped untap step.
    let cards_c = vec![
        card(
            SEEDBORN,
            creature_face("Muse", "{3}{G}{G}", 2, 4),
            KeywordSet::EMPTY,
            SEEDBORN_ABS,
        ),
        card(
            BEAR,
            creature_face("Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            STASIS,
            face("Stasis", "{1}{U}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            STASIS_ABS,
        ),
    ];
    let mut engine_c = bench(
        9103,
        cards_c,
        [Seat::with(&[SEEDBORN, BEAR]), Seat::with(&[STASIS])],
    );
    to_main(&mut engine_c, me());
    let c_bear = the(&engine_c, ZoneLocation::Battlefield, BEAR);
    engine_c
        .state
        .object_mut(c_bear)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    to_main(&mut engine_c, them());
    assert!(
        is_tapped(&engine_c, c_bear),
        "skipped untap step must not untap Seedborn Muse's permanents"
    );
}

// =========================================================================
// 2. CantAttackUnless & CantBlockUnless (Mana, Phyrexian, Bounce, Tap)
// =========================================================================

#[test]
fn attack_and_block_mana_and_phyrexian_taxes_by_defender_scope() {
    const PROPAGANDA: u32 = 910_020;
    const OATHKEEPER: u32 = 910_021;
    const RESTRAINT: u32 = 910_022;
    const NILS: u32 = 910_023;
    const ANNEX: u32 = 910_024;
    const COWED: u32 = 910_025;
    const WALKER: u32 = 910_026;
    const ATTACKER: u32 = 910_027;
    const MUST_ATTACKER: u32 = 910_028;
    const PLAINS: u32 = 910_029;
    const ISLAND: u32 = 910_030;
    const SWAMP: u32 = 910_031;

    static PROP_ABS: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::AttackManaTax {
            target: AttackTaxTarget::You,
            amount: Amount::Fixed(2),
        }
    )];
    static OATH_ABS: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::AttackManaTax {
            target: AttackTaxTarget::PlaneswalkersYouControl,
            amount: Amount::Fixed(1),
        }
    )];
    static RESTRAINT_ABS: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::AttackManaTax {
            target: AttackTaxTarget::You,
            amount: Amount::Domain(PlayerRel::You),
        }
    )];
    static NILS_ABS: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::AttackManaTax {
            target: AttackTaxTarget::You,
            amount: Amount::TotalCountersOnSubject,
        }
    )];
    static ANNEX_ABS: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::AttackPhyrexianTax {
            target: AttackTaxTarget::YouOrYourPlaneswalkers,
            color: ManaColor::White,
        }
    )];
    static HAND_COUNT: Amount = Amount::CountOf {
        filter: &Filter::Any,
        zone: ZoneSel::HandOf(PlayerRel::You),
    };
    static COWED_ABS: &[AbilityDef] = &[
        static_ability!(
            Filter::CREATURE,
            Modifier::AttackManaTax {
                target: AttackTaxTarget::AnyDefender,
                amount: HAND_COUNT,
            }
        ),
        static_ability!(
            Filter::CREATURE,
            Modifier::BlockManaTax { amount: HAND_COUNT }
        ),
    ];
    static MUST_ABS: &[AbilityDef] = &[static_ability!(Filter::This, Modifier::AttacksEachCombat)];

    let pw_face = || {
        let mut f = face("Walker", "{3}", TypeSet::PLANESWALKER);
        f.loyalty = Some(4);
        f
    };

    // Part A: Propaganda taxes player ONLY (not planeswalker).
    {
        let cards = vec![
            card(
                PROPAGANDA,
                face("Propaganda", "{2}{U}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                PROP_ABS,
            ),
            card(WALKER, pw_face(), KeywordSet::EMPTY, &[]),
            card(
                ATTACKER,
                creature_face("Attacker", "{2}", 2, 2),
                KeywordSet::HASTE,
                &[],
            ),
        ];
        let mut engine = bench(
            9110,
            cards,
            [Seat::with(&[ATTACKER]), Seat::with(&[PROPAGANDA, WALKER])],
        );
        to_attackers(&mut engine, me());
        let atk = the(&engine, ZoneLocation::Battlefield, ATTACKER);
        let pw = the(&engine, ZoneLocation::Battlefield, WALKER);

        assert!(
            engine
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(atk, Defender::Player(them()))],
                    },
                )
                .is_err(),
            "Propaganda rejects unpaid attack against defending player"
        );
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(atk, Defender::Planeswalker(pw))],
                },
            )
            .expect("Propaganda does not tax attacking a planeswalker");
    }

    // Part B: Onakke Oathkeeper taxes planeswalker ONLY (player is untaxed).
    {
        let cards = vec![
            card(
                OATHKEEPER,
                creature_face("Oathkeeper", "{1}{W}", 1, 4),
                KeywordSet::EMPTY,
                OATH_ABS,
            ),
            card(WALKER, pw_face(), KeywordSet::EMPTY, &[]),
            card(
                ATTACKER,
                creature_face("Attacker", "{2}", 2, 2),
                KeywordSet::HASTE,
                &[],
            ),
        ];
        let mut engine = bench(
            9111,
            cards,
            [Seat::with(&[ATTACKER]), Seat::with(&[OATHKEEPER, WALKER])],
        );
        to_attackers(&mut engine, me());
        let atk = the(&engine, ZoneLocation::Battlefield, ATTACKER);
        let pw = the(&engine, ZoneLocation::Battlefield, WALKER);
        assert!(
            engine
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(atk, Defender::Planeswalker(pw))],
                    },
                )
                .is_err(),
            "Oathkeeper rejects unpaid attack against planeswalker"
        );
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(atk, Defender::Player(them()))],
                },
            )
            .expect("Oathkeeper does not tax attacking the player");
    }

    // Part C: Collective Restraint (`Amount::Domain`) + Nils (`Amount::TotalCountersOnSubject`).
    {
        let cards = vec![
            card(
                RESTRAINT,
                face("Restraint", "{3}{U}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                RESTRAINT_ABS,
            ),
            card(
                NILS,
                creature_face("Nils", "{2}{W}", 2, 2),
                KeywordSet::EMPTY,
                NILS_ABS,
            ),
            card(
                ATTACKER,
                creature_face("Attacker", "{2}", 2, 2),
                KeywordSet::HASTE,
                &[],
            ),
            basic_land_card(PLAINS, "Plains", subtypes::land::PLAINS),
            basic_land_card(ISLAND, "Island", subtypes::land::ISLAND),
            basic_land_card(SWAMP, "Swamp", subtypes::land::SWAMP),
        ];
        // 4 Forests < Domain(3) + Counters(2) = {5} -> attack rejected!
        let mut engine_short = bench(
            9112,
            cards.clone(),
            [
                Seat::with(&[ATTACKER, forest(), forest(), forest(), forest()]),
                Seat::with(&[RESTRAINT, NILS, PLAINS, ISLAND, SWAMP]),
            ],
        );
        to_main(&mut engine_short, me());
        let atk_s = the(&engine_short, ZoneLocation::Battlefield, ATTACKER);
        engine_short
            .state
            .object_mut(atk_s)
            .unwrap()
            .counters
            .add(CounterKind::P1P1, 2);
        to_attackers(&mut engine_short, me());
        assert!(
            engine_short
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(atk_s, Defender::Player(them()))],
                    },
                )
                .is_err(),
            "4 mana is short of Domain(3) + Counters(2) = 5"
        );

        // 5 Forests == Domain(3) + Counters(2) = {5} -> opens mana window, tap 5 lands, succeeds!
        let mut engine = bench(
            9119,
            cards,
            [
                Seat::with(&[ATTACKER, forest(), forest(), forest(), forest(), forest()]),
                Seat::with(&[RESTRAINT, NILS, PLAINS, ISLAND, SWAMP]),
            ],
        );
        to_main(&mut engine, me());
        let atk = the(&engine, ZoneLocation::Battlefield, ATTACKER);
        engine
            .state
            .object_mut(atk)
            .unwrap()
            .counters
            .add(CounterKind::P1P1, 2);
        to_attackers(&mut engine, me());
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(atk, Defender::Player(them()))],
                },
            )
            .expect("5 mana reach opens payment window for Domain(3) + Counters(2)");
        tap_all_lands(&mut engine, me());
        finish_mana_window(&mut engine, me());
        assert_eq!(engine.state().combat.attackers().len(), 1);
    }

    // Part D: Norn's Annex (`AttackPhyrexianTax { White }`): mixed {W} + 2 life,
    // refusal at 1 life with no {W}, and CR 508.1d AttacksEachCombat not forcing payment.
    {
        let cards = vec![
            card(
                ANNEX,
                face("Annex", "{3}{W}{W}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                ANNEX_ABS,
            ),
            card(
                ATTACKER,
                creature_face("Attacker", "{2}", 2, 2),
                KeywordSet::HASTE,
                &[],
            ),
            card(
                MUST_ATTACKER,
                creature_face("Forced", "{2}", 2, 2),
                KeywordSet::HASTE,
                MUST_ABS,
            ),
        ];
        let mut engine = bench(
            9113,
            cards.clone(),
            [Seat::with(&[ATTACKER, MUST_ATTACKER]), Seat::with(&[ANNEX])],
        );
        to_attackers(&mut engine, me());
        add_pool(&mut engine, me(), ManaColor::White, 1);
        let atk1 = the(&engine, ZoneLocation::Battlefield, ATTACKER);
        let atk2 = the(&engine, ZoneLocation::Battlefield, MUST_ATTACKER);
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![
                        (atk1, Defender::Player(them())),
                        (atk2, Defender::Player(them())),
                    ],
                },
            )
            .expect("attackers with mixed life + {W} accepted");
        assert!(matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayLife { amount: 2, .. },
                ..
            } if *player == me()
        ));
        engine
            .apply(me(), PlayerAction::YesNo(true))
            .expect("pay 2 life");
        assert!(matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayLife { amount: 2, .. },
                ..
            } if *player == me()
        ));
        engine
            .apply(me(), PlayerAction::YesNo(false))
            .expect("choose {W}");
        finish_mana_window(&mut engine, me());
        assert_eq!(life(&engine, me()), 18, "paid 2 life for first attacker");
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "spent {{W}} for second attacker"
        );
        assert_eq!(engine.state().combat.attackers().len(), 2);

        // CR 508.1d: AttacksEachCombat under Annex with 0 mana does NOT force paying 2 life!
        let mut engine_must = bench(
            9114,
            cards.clone(),
            [Seat::with(&[MUST_ATTACKER]).at(20), Seat::with(&[ANNEX])],
        );
        to_attackers(&mut engine_must, me());
        engine_must
            .apply(me(), PlayerAction::DeclareAttackers { attackers: vec![] })
            .expect("CR 508.1d: AttacksEachCombat never forces paying a Phyrexian attack tax");

        // At 1 life with 0 mana, cannot pay {W} or 2 life -> attack rejected.
        let mut engine_low = bench(
            9115,
            cards,
            [Seat::with(&[ATTACKER]).at(1), Seat::with(&[ANNEX])],
        );
        to_attackers(&mut engine_low, me());
        let low_atk = the(&engine_low, ZoneLocation::Battlefield, ATTACKER);
        assert!(
            engine_low
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(low_atk, Defender::Player(them()))],
                    },
                )
                .is_err(),
            "at 1 life with no {{W}}, Phyrexian attack tax cannot be paid"
        );
    }

    // Part E: BlockManaTax (Cowed by Wisdom): requires mana equal to hand size to block.
    {
        let cards = vec![
            card(
                COWED,
                face("Cowed", "{W}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                COWED_ABS,
            ),
            card(
                ATTACKER,
                creature_face("Fighter", "{2}", 2, 2),
                KeywordSet::HASTE,
                &[],
            ),
        ];
        // Unpaid block rejected when defender has 0 mana reach:
        let mut engine_no = bench(
            9116,
            cards.clone(),
            [
                Seat::with(&[ATTACKER, COWED, forest(), forest()]).holding(&[ATTACKER, ATTACKER]),
                Seat::with(&[ATTACKER]),
            ],
        );
        to_attackers(&mut engine_no, me());
        let my_atk_n = objects(&engine_no, ZoneLocation::Battlefield, ATTACKER)
            .into_iter()
            .find(|&id| engine_no.state().object(id).unwrap().controller == me())
            .unwrap();
        let their_blk_n = objects(&engine_no, ZoneLocation::Battlefield, ATTACKER)
            .into_iter()
            .find(|&id| engine_no.state().object(id).unwrap().controller == them())
            .unwrap();
        engine_no
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(my_atk_n, Defender::Player(them()))],
                },
            )
            .expect("attack tax window opens");
        tap_all_lands(&mut engine_no, me());
        finish_mana_window(&mut engine_no, me());
        to_blockers(&mut engine_no, them());
        assert!(
            engine_no
                .apply(
                    them(),
                    PlayerAction::DeclareBlockers {
                        blockers: vec![(their_blk_n, my_atk_n)],
                    },
                )
                .is_err(),
            "BlockManaTax rejects unpaid block"
        );

        // Paid block succeeds when defender has 2 Forests:
        let mut engine = bench(
            9117,
            cards,
            [
                Seat::with(&[ATTACKER, COWED, forest(), forest()]).holding(&[ATTACKER, ATTACKER]),
                Seat::with(&[ATTACKER, forest(), forest()]),
            ],
        );
        to_attackers(&mut engine, me());
        let my_atk = objects(&engine, ZoneLocation::Battlefield, ATTACKER)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == me())
            .unwrap();
        let their_blk = objects(&engine, ZoneLocation::Battlefield, ATTACKER)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == them())
            .unwrap();
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(my_atk, Defender::Player(them()))],
                },
            )
            .expect("attack tax window opens");
        tap_all_lands(&mut engine, me());
        finish_mana_window(&mut engine, me());
        to_blockers(&mut engine, them());
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(their_blk, my_atk)],
                },
            )
            .expect("BlockManaTax accepts block when {2} is available");
        tap_all_lands(&mut engine, them());
        finish_mana_window(&mut engine, them());
        assert!(
            engine
                .state()
                .combat
                .blockers
                .iter()
                .any(|b| b.blocker == their_blk)
        );
    }
}

#[test]
fn attack_return_to_hand_and_attack_block_tap_taxes() {
    const SERPENT: u32 = 910_040;
    const ENCH: u32 = 910_041;
    const WARRIOR: u32 = 910_042;
    const HELPER: u32 = 910_043;
    const MUST_WARRIOR: u32 = 910_044;

    static SERPENT_ABS: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::CantAttackUnlessYouReturn {
            count: 1,
            filter: &Filter::ENCHANTMENT,
        }
    )];
    static WARRIOR_ABS: &[AbilityDef] = &[
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessYouTap {
                count: 1,
                filter: &Filter::CREATURE,
            }
        ),
        static_ability!(
            Filter::This,
            Modifier::CantBlockUnlessYouTap {
                count: 1,
                filter: &Filter::CREATURE,
            }
        ),
    ];
    static MUST_WARRIOR_ABS: &[AbilityDef] = &[
        static_ability!(
            Filter::This,
            Modifier::CantBlockUnlessYouTap {
                count: 1,
                filter: &Filter::CREATURE,
            }
        ),
        static_ability!(Filter::This, Modifier::MustBlock),
    ];

    // Part A: Floodtide Serpent (`CantAttackUnlessYouReturn`).
    {
        let cards = vec![
            card(
                SERPENT,
                creature_face("Serpent", "{4}{U}", 4, 4),
                KeywordSet::HASTE,
                SERPENT_ABS,
            ),
            card(
                ENCH,
                face("Ench", "{1}{W}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                &[],
            ),
        ];
        let mut engine_no = bench(
            9120,
            cards.clone(),
            [Seat::with(&[SERPENT]), Seat::default()],
        );
        to_attackers(&mut engine_no, me());
        let s_no = the(&engine_no, ZoneLocation::Battlefield, SERPENT);
        assert!(
            engine_no
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(s_no, Defender::Player(them()))],
                    },
                )
                .is_err(),
            "Serpent cannot attack without an enchantment to bounce"
        );

        let mut engine = bench(9121, cards, [Seat::with(&[SERPENT, ENCH]), Seat::default()]);
        to_attackers(&mut engine, me());
        let serp = the(&engine, ZoneLocation::Battlefield, SERPENT);
        let ench = the(&engine, ZoneLocation::Battlefield, ENCH);
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(serp, Defender::Player(them()))],
                },
            )
            .expect("Serpent attack begins bounce cost");
        assert!(matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == me()));
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![ench],
                },
            )
            .expect("choose enchantment to return");
        assert_eq!(
            engine.state().object(ench).unwrap().zone,
            crate::zone::Zone::Hand
        );
        assert_eq!(engine.state().combat.attackers().len(), 1);
    }

    // Part B: Hollow Warrior (`CantAttackUnlessYouTap` + `CantBlockUnlessYouTap`).
    {
        let cards = vec![
            card(
                WARRIOR,
                creature_face("Warrior", "{4}", 2, 2),
                KeywordSet::HASTE,
                WARRIOR_ABS,
            ),
            card(
                HELPER,
                creature_face("Helper", "{1}", 1, 1),
                KeywordSet::HASTE,
                &[],
            ),
            card(
                MUST_WARRIOR,
                creature_face("MustWarrior", "{4}", 2, 2),
                KeywordSet::HASTE,
                MUST_WARRIOR_ABS,
            ),
        ];

        let mut engine_alone = bench(
            9122,
            cards.clone(),
            [Seat::with(&[WARRIOR]), Seat::default()],
        );
        to_attackers(&mut engine_alone, me());
        let w = the(&engine_alone, ZoneLocation::Battlefield, WARRIOR);
        assert!(
            engine_alone
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(w, Defender::Player(them()))],
                    },
                )
                .is_err(),
            "Hollow Warrior alone cannot pay CantAttackUnlessYouTap"
        );

        let mut engine = bench(
            9123,
            cards.clone(),
            [
                Seat::with(&[WARRIOR, HELPER]),
                Seat::with(&[WARRIOR, HELPER]),
            ],
        );
        to_attackers(&mut engine, me());
        let my_w = objects(&engine, ZoneLocation::Battlefield, WARRIOR)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == me())
            .unwrap();
        let my_h = objects(&engine, ZoneLocation::Battlefield, HELPER)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == me())
            .unwrap();
        let their_w = objects(&engine, ZoneLocation::Battlefield, WARRIOR)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == them())
            .unwrap();
        let their_h = objects(&engine, ZoneLocation::Battlefield, HELPER)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == them())
            .unwrap();

        assert!(
            engine
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![
                            (my_w, Defender::Player(them())),
                            (my_h, Defender::Player(them())),
                        ],
                    },
                )
                .is_err(),
            "cannot tap an attacking creature to pay CantAttackUnlessYouTap"
        );
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(my_w, Defender::Player(them()))],
                },
            )
            .expect("attack with Warrior");
        assert!(matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == me()));
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![my_h],
                },
            )
            .expect("tap helper for attack");
        assert!(is_tapped(&engine, my_h));
        assert_eq!(engine.state().combat.attackers().len(), 1);

        to_blockers(&mut engine, them());
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(their_w, my_w)],
                },
            )
            .expect("block with Warrior");
        assert!(
            matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == them())
        );
        engine
            .apply(
                them(),
                PlayerAction::ChooseObjects {
                    objects: vec![their_h],
                },
            )
            .expect("tap helper for block");
        assert!(is_tapped(&engine, their_h));
        assert!(
            engine
                .state()
                .combat
                .blockers
                .iter()
                .any(|b| b.blocker == their_w)
        );

        let mut engine_mb = bench(
            9124,
            cards,
            [Seat::with(&[HELPER]), Seat::with(&[MUST_WARRIOR, HELPER])],
        );
        to_attackers(&mut engine_mb, me());
        let atk_h = objects(&engine_mb, ZoneLocation::Battlefield, HELPER)
            .into_iter()
            .find(|&id| engine_mb.state().object(id).unwrap().controller == me())
            .unwrap();
        engine_mb
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(atk_h, Defender::Player(them()))],
                },
            )
            .expect("helper attacks");
        to_blockers(&mut engine_mb, them());
        engine_mb
            .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
            .expect("CR 509.1c: MustBlock never forces paying CantBlockUnlessYouTap");
    }
}

// =========================================================================
// 3. Branch Conditions, New Amounts, and Treasure Mana Provenance
// =========================================================================

#[test]
fn branch_conditions_amounts_and_treasure_mana_provenance() {
    const TREASURE: u32 = 910_060;
    const TREASURE_SPELL: u32 = 910_061;
    const TREASURE_PERM: u32 = 910_062;
    const FORETOLD_PERM: u32 = 910_063;
    const SNEAKED_PERM: u32 = 910_064;
    const COND_PROBE: u32 = 910_065;
    const AMOUNT_PROBE: u32 = 910_066;
    const WALKER: u32 = 910_067;
    const ATTACKER: u32 = 910_068;

    static TREASURE_ABS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
        },
        effects: &[Effect::AddMana {
            source: ManaSource::Choice(FIVE_COLORS),
            amount: Amount::Fixed(1),
            distribution: ManaDistribution::OneColor,
            restriction: None,
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];
    static TREASURE_SPELL_ABS: &[AbilityDef] = &[spell!(&[Effect::IfCondition {
        condition: Condition::TreasureManaWasSpent,
        then: &[Effect::gain_life(5)],
        otherwise: &[Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::You,
        }],
    }])];
    static TREASURE_PERM_ABS: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[Effect::IfCondition {
            condition: Condition::TreasureManaWasSpent,
            then: &[Effect::gain_life(4)],
            otherwise: &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::You,
            }],
        }]
    )];
    static FORETOLD_ABS: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[Effect::IfCondition {
            condition: Condition::WasForetold,
            then: &[Effect::gain_life(3)],
            otherwise: &[],
        }]
    )];
    static SNEAKED_ABS: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[Effect::IfCondition {
            condition: Condition::WasSneaked,
            then: &[Effect::gain_life(6)],
            otherwise: &[],
        }]
    )];

    static COND_PROBE_ABS: &[AbilityDef] = &[
        free(
            &[Effect::IfCondition {
                condition: Condition::OpponentLifeAtMost(0),
                then: &[Effect::gain_life(10)],
                otherwise: &[Effect::gain_life(1)],
            }],
            None,
        ),
        free(
            &[Effect::IfCondition {
                condition: Condition::PlayerAttackedYouThisTurn(PlayerRel::ActivePlayer),
                then: &[Effect::gain_life(7)],
                otherwise: &[Effect::gain_life(2)],
            }],
            None,
        ),
        free(
            &[Effect::CaptureAffected {
                first: &[Effect::ExileAll {
                    filter: &Filter::And(&[
                        Filter::CREATURE,
                        Filter::ControlledBy(PlayerRel::Opponent),
                    ]),
                    linked: false,
                }],
                then: &[
                    Effect::IfCondition {
                        condition: Condition::RememberedMatches(&Filter::CREATURE),
                        then: &[Effect::gain_life(3)],
                        otherwise: &[],
                    },
                    Effect::IfCondition {
                        condition: Condition::RememberedMatches(&Filter::ARTIFACT),
                        then: &[Effect::gain_life(50)],
                        otherwise: &[],
                    },
                ],
            }],
            None,
        ),
    ];

    static AMOUNT_PROBE_ABS: &[AbilityDef] = &[
        free(
            &[Effect::GainLife {
                amount: Amount::PlayerCounters {
                    player: PlayerRel::Opponent,
                    kind: CounterKind::Poison,
                },
            }],
            None,
        ),
        free(
            &[Effect::GainLife {
                amount: Amount::RingTemptedCount(PlayerRel::You),
            }],
            None,
        ),
    ];

    let mut treasure_face = face("Treasure", "{0}", TypeSet::ARTIFACT);
    treasure_face.subtypes = Box::leak(Box::new([subtypes::artifact::TREASURE]));

    // Part A: Treasure mana provenance on both spell and ETB permanent!
    {
        const SWAMP_LAND: u32 = 910_069;
        let cards = vec![
            card(TREASURE, treasure_face, KeywordSet::EMPTY, TREASURE_ABS),
            card(
                TREASURE_SPELL,
                face("Devour", "{B}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                TREASURE_SPELL_ABS,
            ),
            card(
                TREASURE_PERM,
                creature_face("Fixer", "{G}", 2, 2),
                KeywordSet::EMPTY,
                TREASURE_PERM_ABS,
            ),
            basic_land_card(SWAMP_LAND, "Swamp", subtypes::land::SWAMP),
        ];
        let mut engine = bench(
            9130,
            cards,
            [
                Seat::with(&[TREASURE, TREASURE, SWAMP_LAND, forest()]).holding(&[
                    TREASURE_SPELL,
                    TREASURE_SPELL,
                    TREASURE_PERM,
                    TREASURE_PERM,
                ]),
                Seat::default(),
            ],
        );
        to_main(&mut engine, me());

        let swamp = the(&engine, ZoneLocation::Battlefield, SWAMP_LAND);
        engine
            .apply(me(), PlayerAction::ActivateManaAbility { source: swamp })
            .expect("tap swamp");
        let spell1 = objects(&engine, ZoneLocation::Hand(me()), TREASURE_SPELL)[0];
        cast_from_hand(&mut engine, me(), spell1);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, me()),
            18,
            "normal mana takes false TreasureManaWasSpent branch"
        );

        let t1 = objects(&engine, ZoneLocation::Battlefield, TREASURE)[0];
        activate(&mut engine, me(), t1, 0);
        engine
            .apply(me(), PlayerAction::ChooseColor(ManaColor::Black))
            .expect("choose {B} from treasure");
        let spell2 = objects(&engine, ZoneLocation::Hand(me()), TREASURE_SPELL)[0];
        cast_from_hand(&mut engine, me(), spell2);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, me()),
            23,
            "Treasure mana takes true TreasureManaWasSpent branch"
        );

        let for_land = the(&engine, ZoneLocation::Battlefield, forest());
        engine
            .apply(me(), PlayerAction::ActivateManaAbility { source: for_land })
            .expect("tap forest");
        let perm1 = objects(&engine, ZoneLocation::Hand(me()), TREASURE_PERM)[0];
        cast_from_hand(&mut engine, me(), perm1);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, me()),
            22,
            "permanent cast with normal mana lacks TreasureManaSpent"
        );

        let t2 = objects(&engine, ZoneLocation::Battlefield, TREASURE)[0];
        activate(&mut engine, me(), t2, 0);
        engine
            .apply(me(), PlayerAction::ChooseColor(ManaColor::Green))
            .expect("choose {G} from treasure");
        let perm2 = objects(&engine, ZoneLocation::Hand(me()), TREASURE_PERM)[0];
        cast_from_hand(&mut engine, me(), perm2);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, me()),
            26,
            "Rider::TreasureManaSpent transfers from stack spell to permanent for ETB check"
        );
    }

    // Part B: WasForetold, WasSneaked, OpponentLifeAtMost, RememberedMatches,
    // PlayerAttackedYouThisTurn (player vs planeswalker), PlayerCounters, RingTemptedCount.
    {
        let mut pw_face = face("Walker", "{3}", TypeSet::PLANESWALKER);
        pw_face.loyalty = Some(5);
        let cards = vec![
            card(
                FORETOLD_PERM,
                creature_face("Foretold", "{1}", 2, 2),
                KeywordSet::EMPTY,
                FORETOLD_ABS,
            ),
            card(
                SNEAKED_PERM,
                creature_face("Sneaked", "{1}", 2, 2),
                KeywordSet::EMPTY,
                SNEAKED_ABS,
            ),
            card(
                COND_PROBE,
                face("CondProbe", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                COND_PROBE_ABS,
            ),
            card(
                AMOUNT_PROBE,
                face("AmtProbe", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                AMOUNT_PROBE_ABS,
            ),
            card(WALKER, pw_face, KeywordSet::EMPTY, &[]),
            card(
                ATTACKER,
                creature_face("Attacker", "{1}", 1, 1),
                KeywordSet::HASTE,
                &[],
            ),
        ];
        let mut engine = bench(
            9131,
            cards,
            [
                Seat::with(&[ATTACKER, AMOUNT_PROBE, forest(), forest()])
                    .holding(&[FORETOLD_PERM, SNEAKED_PERM]),
                Seat::with(&[COND_PROBE, WALKER, ATTACKER]).at(1),
            ],
        );
        to_main(&mut engine, me());

        engine.state.players[usize::from(them().get())].poison = 4;
        engine.state.players[usize::from(me().get())].ring_temptations = 3;
        let amt_probe = the(&engine, ZoneLocation::Battlefield, AMOUNT_PROBE);
        activate(&mut engine, me(), amt_probe, 0);
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 24, "PlayerCounters(Poison) = 4");
        activate(&mut engine, me(), amt_probe, 1);
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 27, "RingTemptedCount = 3");

        tap_all_lands(&mut engine, me());
        let fp = the(&engine, ZoneLocation::Hand(me()), FORETOLD_PERM);
        engine
            .state
            .object_mut(fp)
            .unwrap()
            .riders
            .push(Rider::Foretold { turn: 1 });
        cast_from_hand(&mut engine, me(), fp);
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 30, "WasForetold ETB gained 3 life");

        let sp = the(&engine, ZoneLocation::Hand(me()), SNEAKED_PERM);
        cast_from_hand(&mut engine, me(), sp);
        engine
            .state
            .object_mut(sp)
            .unwrap()
            .riders
            .push(Rider::Sneaked);
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 36, "WasSneaked ETB gained 6 life");

        // Attack opponent's PLANESWALKER -> PlayerAttackedYouThisTurn must remain false!
        to_attackers(&mut engine, me());
        let my_atk = objects(&engine, ZoneLocation::Battlefield, ATTACKER)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == me())
            .unwrap();
        let pw = the(&engine, ZoneLocation::Battlefield, WALKER);
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(my_atk, Defender::Planeswalker(pw))],
                },
            )
            .expect("attack planeswalker");
        walk_until(&mut engine, |e| {
            e.state().turn.phase == Phase::SecondMain
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == them())
        });
        let cond_probe = the(&engine, ZoneLocation::Battlefield, COND_PROBE);
        let opp_before = life(&engine, them());
        activate(&mut engine, them(), cond_probe, 1);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, them()),
            opp_before + 2,
            "attacking a planeswalker does NOT satisfy PlayerAttackedYouThisTurn"
        );

        let opp_mid = life(&engine, them());
        engine.apply(me(), PlayerAction::PassPriority).unwrap();
        activate(&mut engine, them(), cond_probe, 2);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, them()),
            opp_mid + 3,
            "RememberedMatches(CREATURE) true (+3), RememberedMatches(ARTIFACT) false (+0)"
        );

        let opp_before_life = life(&engine, them());
        engine.apply(me(), PlayerAction::PassPriority).unwrap();
        activate(&mut engine, them(), cond_probe, 0);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, them()),
            opp_before_life + 1,
            "opponent life > 0 -> false"
        );

        to_attackers(&mut engine, them());
        let their_atk = objects(&engine, ZoneLocation::Battlefield, ATTACKER)
            .into_iter()
            .find(|&id| engine.state().object(id).unwrap().controller == them())
            .unwrap();
        engine
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(their_atk, Defender::Player(me()))],
                },
            )
            .expect("attack seat 0 directly");
        assert!(
            engine
                .state()
                .per_turn
                .attacked_players
                .items
                .contains(&(them(), me())),
            "direct player attack recorded in per_turn.attacked_players"
        );
    }
}

// =========================================================================
// 4. ChoosePlayerEx, Entry Choices, RevealChosenPlayer, AddManaChoiceFor
// =========================================================================

#[test]
fn choose_player_ex_entry_choices_and_reveal_chosen_player() {
    const BOREAS: u32 = 910_080;
    const LIFE_PICK: u32 = 910_081;
    const GLUNTCH: u32 = 910_082;
    const HELLKITE: u32 = 910_083;
    const SEARCHLIGHT: u32 = 910_084;
    const BITTER_FEUD: u32 = 910_085;
    const ARCHON: u32 = 910_086;
    const OPPRESSOR: u32 = 910_087;
    const FAVORED: u32 = 910_088;
    const ELF: u32 = 910_089;
    const GOBLIN: u32 = 910_090;
    const PLAINS: u32 = 910_091;

    static LAND_DIFF: Amount = Amount::Difference {
        left: &Amount::CountOf {
            filter: &Filter::And(&[Filter::LAND, Filter::ControlledBy(PlayerRel::Chosen)]),
            zone: ZoneSel::Battlefield,
        },
        right: &Amount::CountOf {
            filter: &Filter::And(&[Filter::LAND, Filter::ControlledByYou]),
            zone: ZoneSel::Battlefield,
        },
    };
    static BOREAS_ABS: &[AbilityDef] = &[free(
        &[Effect::ChoosePlayerEx {
            chooser: PlayerRel::You,
            filter: PlayerFilter::OpponentControlsMoreThanYou(&Filter::LAND),
            random: false,
            remember: false,
            if_none: &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::You,
            }],
            then: &[Effect::GainLife { amount: LAND_DIFF }],
        }],
        None,
    )];
    static LIFE_PICK_ABS: &[AbilityDef] = &[
        free(
            &[Effect::ChoosePlayerEx {
                chooser: PlayerRel::You,
                filter: PlayerFilter::OpponentWithGreatestLife,
                random: false,
                remember: false,
                if_none: &[],
                then: &[Effect::LoseLife {
                    target: PlayerRel::Chosen,
                    amount: Amount::Fixed(3),
                }],
            }],
            None,
        ),
        free(
            &[Effect::GainLifeFor {
                who: PlayerRel::YouAndTargetOpponent,
                amount: Amount::Fixed(2),
            }],
            Some(TargetReq::one(baylee_cards_dsl::TargetSpec::AnyOpponent)),
        ),
    ];
    static GLUNTCH_ABS: &[AbilityDef] = &[free(
        &[
            Effect::ChoosePlayerEx {
                chooser: PlayerRel::You,
                filter: PlayerFilter::NotYetRemembered(PlayerRel::EachPlayer),
                random: false,
                remember: true,
                if_none: &[],
                then: &[Effect::GainLifeFor {
                    who: PlayerRel::Chosen,
                    amount: Amount::Fixed(1),
                }],
            },
            Effect::ChoosePlayerEx {
                chooser: PlayerRel::You,
                filter: PlayerFilter::NotYetRemembered(PlayerRel::EachPlayer),
                random: false,
                remember: true,
                if_none: &[],
                then: &[Effect::GainLifeFor {
                    who: PlayerRel::Chosen,
                    amount: Amount::Fixed(2),
                }],
            },
            Effect::ChoosePlayerEx {
                chooser: PlayerRel::You,
                filter: PlayerFilter::NotYetRemembered(PlayerRel::EachPlayer),
                random: false,
                remember: true,
                if_none: &[],
                then: &[Effect::GainLifeFor {
                    who: PlayerRel::Chosen,
                    amount: Amount::Fixed(4),
                }],
            },
            Effect::ChoosePlayerEx {
                chooser: PlayerRel::You,
                filter: PlayerFilter::NotYetRemembered(PlayerRel::EachPlayer),
                random: false,
                remember: true,
                if_none: &[Effect::LoseLife {
                    amount: Amount::Fixed(5),
                    target: PlayerRel::You,
                }],
                then: &[],
            },
        ],
        None,
    )];
    static HELLKITE_ABS: &[AbilityDef] = &[free(
        &[Effect::ChoosePlayerEx {
            chooser: PlayerRel::You,
            filter: PlayerFilter::OpponentNotAttackedBySourceLastCombat,
            random: false,
            remember: false,
            if_none: &[Effect::TapAll {
                filter: &Filter::This,
            }],
            then: &[Effect::LoseLife {
                target: PlayerRel::Chosen,
                amount: Amount::Fixed(2),
            }],
        }],
        None,
    )];
    static SEARCHLIGHT_ABS: &[AbilityDef] = &[free(
        &[Effect::ChoosePlayerEx {
            chooser: PlayerRel::You,
            filter: PlayerFilter::Among(PlayerRel::EachPlayer),
            random: false,
            remember: false,
            if_none: &[],
            then: &[Effect::AddManaChoiceFor {
                who: PlayerRel::Chosen,
                source: ManaSource::Choice(FIVE_COLORS),
                amount: Amount::Fixed(1),
                distribution: ManaDistribution::OneColor,
                restriction: None,
            }],
        }],
        None,
    )];
    static BITTER_FEUD_ENTERS: &[EnterModifier] = &[EnterModifier::ChooseTwoPlayers];
    static BITTER_FEUD_ABS: &[AbilityDef] = &[free(
        &[
            Effect::LoseLife {
                target: PlayerRel::ChosenBySource,
                amount: Amount::Fixed(2),
            },
            Effect::LoseLife {
                target: PlayerRel::SecondChosenBySource,
                amount: Amount::Fixed(5),
            },
        ],
        None,
    )];
    static ARCHON_ENTERS: &[EnterModifier] = &[EnterModifier::SecretlyChooseOpponent];
    static ARCHON_ABS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::RevealChosenPlayer],
        },
        effects: &[Effect::LoseLife {
            target: PlayerRel::ChosenBySource,
            amount: Amount::Fixed(4),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];
    static OPPRESSOR_ENTERS: &[EnterModifier] = &[EnterModifier::OpponentChoosesCreatureType];
    static FAVORED_ENTERS: &[EnterModifier] =
        &[EnterModifier::NoteMostPrevalentCreatureTypeInOpponentLibrary];

    // Part A: Boreas Charger, OpponentWithGreatestLife tie, Gluntch, Spectral Searchlight (3P).
    {
        let cards = vec![
            card(
                BOREAS,
                face("Boreas", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                BOREAS_ABS,
            ),
            card(
                LIFE_PICK,
                face("LifePick", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                LIFE_PICK_ABS,
            ),
            card(
                GLUNTCH,
                face("Gluntch", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GLUNTCH_ABS,
            ),
            card(
                SEARCHLIGHT,
                face("Searchlight", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                SEARCHLIGHT_ABS,
            ),
            basic_land_card(PLAINS, "Plains", subtypes::land::PLAINS),
        ];
        let mut engine = bench(
            9140,
            cards,
            [
                Seat::with(&[BOREAS, LIFE_PICK, GLUNTCH, SEARCHLIGHT, PLAINS]),
                Seat::with(&[PLAINS]),
                Seat::with(&[PLAINS, PLAINS, PLAINS, PLAINS]),
            ],
        );
        to_main(&mut engine, me());

        let boreas = the(&engine, ZoneLocation::Battlefield, BOREAS);
        activate(&mut engine, me(), boreas, 0);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, me()),
            23,
            "Boreas auto-picked seat2 and gained 4-1=3"
        );

        let lp = the(&engine, ZoneLocation::Battlefield, LIFE_PICK);
        activate(&mut engine, me(), lp, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
            unreachable!();
        };
        assert_eq!(player, me());
        assert_eq!(options.len(), 2);
        engine
            .apply(me(), PlayerAction::ChoosePlayer(seat2()))
            .unwrap();
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, seat2()),
            17,
            "chosen greatest-life opponent lost 3"
        );

        let gluntch = the(&engine, ZoneLocation::Battlefield, GLUNTCH);
        activate(&mut engine, me(), gluntch, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        engine
            .apply(me(), PlayerAction::ChoosePlayer(me()))
            .unwrap();
        let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
            panic!("expected 2nd player choice, got {:?}", engine.pending());
        };
        assert!(
            !options.contains(&me()),
            "remembered player excluded from NotYetRemembered"
        );
        engine
            .apply(me(), PlayerAction::ChoosePlayer(them()))
            .unwrap();
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 23 + 1 - 5);
        assert_eq!(life(&engine, them()), 20 + 2);
        assert_eq!(life(&engine, seat2()), 17 + 4);

        let sl = the(&engine, ZoneLocation::Battlefield, SEARCHLIGHT);
        activate(&mut engine, me(), sl, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        engine
            .apply(me(), PlayerAction::ChoosePlayer(them()))
            .unwrap();
        assert!(
            matches!(engine.pending(), Pending::ChooseColor { player, .. } if *player == them()),
            "recipient seat chooses mana color on AddManaChoiceFor, got {:?}",
            engine.pending()
        );
        engine
            .apply(them(), PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        settle(&mut engine, me());
        let pool = &engine.state().players[usize::from(them().get())].mana_pool;
        assert_eq!(pool.total(), 1);
        assert!(crate::mana_pay::can_pay(pool, &ManaCost::parse("{U}")));

        // PlayerRel::YouAndTargetOpponent gains 2 life for me() and targeted seat2(), NOT them()!
        let me_pre = life(&engine, me());
        let them_pre = life(&engine, them());
        let seat2_pre = life(&engine, seat2());
        activate(&mut engine, me(), lp, 1);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![seat2()],
                },
            )
            .expect("target seat2");
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, me()),
            me_pre + 2,
            "YouAndTargetOpponent includes you"
        );
        assert_eq!(
            life(&engine, seat2()),
            seat2_pre + 2,
            "YouAndTargetOpponent includes target opponent"
        );
        assert_eq!(
            life(&engine, them()),
            them_pre,
            "YouAndTargetOpponent excludes untargeted opponent"
        );
    }

    // Part B: Territorial Hellkite (`OpponentNotAttackedBySourceLastCombat`) in 2P.
    {
        let cards = vec![card(
            HELLKITE,
            creature_face("Hellkite", "{2}{R}{R}", 6, 5),
            KeywordSet::HASTE,
            HELLKITE_ABS,
        )];
        let mut engine = bench(9141, cards, [Seat::with(&[HELLKITE]), Seat::default()]);
        to_attackers(&mut engine, me());
        let hk = the(&engine, ZoneLocation::Battlefield, HELLKITE);
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(hk, Defender::Player(them()))],
                },
            )
            .expect("Hellkite attacks seat 1 in combat 1");
        to_main(&mut engine, me());
        let hk_life_before = life(&engine, them());
        activate(&mut engine, me(), hk, 0);
        settle(&mut engine, me());
        assert!(
            is_tapped(&engine, hk),
            "in 2P after attacking them() last combat, no eligible opponent -> if_none taps Hellkite"
        );
        assert_eq!(life(&engine, them()), hk_life_before);

        // Advance through Turn 3's combat (no attackers declared -> clears last_combat_attacked!)
        to_main(&mut engine, them());
        to_main(&mut engine, me());
        activate(&mut engine, me(), hk, 0);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, them()),
            hk_life_before - 2,
            "after not attacking in previous combat, them() is eligible again"
        );
    }

    // Part C: Entry choices (Bitter Feud, Guardian Archon + RevealChosenPlayer, Callous Oppressor, Favored Enemy).
    {
        let cards = vec![
            card(
                BITTER_FEUD,
                face_with_enters(
                    face("BitterFeud", "{0}", TypeSet::ENCHANTMENT),
                    BITTER_FEUD_ENTERS,
                ),
                KeywordSet::EMPTY,
                BITTER_FEUD_ABS,
            ),
            card(
                ARCHON,
                face_with_enters(creature_face("Archon", "{0}", 5, 5), ARCHON_ENTERS),
                KeywordSet::EMPTY,
                ARCHON_ABS,
            ),
            card(
                OPPRESSOR,
                face_with_enters(creature_face("Oppressor", "{0}", 1, 2), OPPRESSOR_ENTERS),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                FAVORED,
                face_with_enters(face("Favored", "{0}", TypeSet::ENCHANTMENT), FAVORED_ENTERS),
                KeywordSet::EMPTY,
                &[],
            ),
            creature_with_subtype(ELF, "Elf", subtypes::creature::ELF),
            creature_with_subtype(GOBLIN, "Goblin", subtypes::creature::GOBLIN),
        ];
        // Put 3 Elves + 1 Goblin in seat 1's library so Favored Enemy notes Elf!
        let mut engine = bench(
            9142,
            cards,
            [
                Seat::default().holding(&[BITTER_FEUD, ARCHON, OPPRESSOR, FAVORED]),
                Seat::default(),
                Seat::default(),
            ],
        );
        to_main(&mut engine, me());

        let bf = the(&engine, ZoneLocation::Hand(me()), BITTER_FEUD);
        cast_from_hand(&mut engine, me(), bf);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        engine
            .apply(me(), PlayerAction::ChoosePlayer(them()))
            .unwrap();
        let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
            panic!("expected 2nd player choice for Bitter Feud");
        };
        assert!(
            !options.contains(&them()),
            "second player must be distinct from first"
        );
        engine
            .apply(me(), PlayerAction::ChoosePlayer(seat2()))
            .unwrap();
        settle(&mut engine, me());
        let bf_bf = the(&engine, ZoneLocation::Battlefield, BITTER_FEUD);
        activate(&mut engine, me(), bf_bf, 0);
        settle(&mut engine, me());
        assert_eq!(life(&engine, them()), 18, "ChosenBySource lost 2");
        assert_eq!(life(&engine, seat2()), 15, "SecondChosenBySource lost 5");

        let arch = the(&engine, ZoneLocation::Hand(me()), ARCHON);
        cast_from_hand(&mut engine, me(), arch);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        engine
            .apply(me(), PlayerAction::ChoosePlayer(seat2()))
            .unwrap();
        settle(&mut engine, me());
        let arch_bf = the(&engine, ZoneLocation::Battlefield, ARCHON);
        let arch_obj = engine.state().object(arch_bf).unwrap();
        assert!(arch_obj.has_unrevealed_secret_chosen_player());
        assert_eq!(
            arch_obj.chosen_opponent(),
            None,
            "unrevealed secret chosen player must stay hidden from PublicObject"
        );
        assert_eq!(
            arch_obj.chosen_opponent_for_rules(),
            Some(seat2()),
            "rules-internal accessor still reads unrevealed secret chosen player"
        );
        activate(&mut engine, me(), arch_bf, 0);
        settle(&mut engine, me());
        let arch_obj = engine.state().object(arch_bf).unwrap();
        assert!(!arch_obj.has_unrevealed_secret_chosen_player());
        assert_eq!(
            arch_obj.chosen_opponent(),
            Some(seat2()),
            "revealed secret chosen player becomes visible to PublicObject"
        );
        assert_eq!(
            life(&engine, seat2()),
            11,
            "RevealChosenPlayer fired ability against secret opponent"
        );
        let (_, legal_after) = offer(&engine);
        assert!(
            !legal_after.abilities.contains(&(arch_bf, 0)),
            "RevealChosenPlayer is no longer legal once revealed"
        );

        let opp_card = the(&engine, ZoneLocation::Hand(me()), OPPRESSOR);
        cast_from_hand(&mut engine, me(), opp_card);
        walk_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChoosePlayer { .. } | Pending::ChooseSubtype { .. }
            )
        });
        if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
            engine
                .apply(me(), PlayerAction::ChoosePlayer(them()))
                .unwrap();
        }
        assert!(
            matches!(engine.pending(), Pending::ChooseSubtype { player, .. } if *player == them())
        );
        engine
            .apply(them(), PlayerAction::ChooseSubtype(subtypes::creature::ELF))
            .unwrap();
        settle(&mut engine, me());
        let opp_bf = the(&engine, ZoneLocation::Battlefield, OPPRESSOR);
        assert_eq!(
            engine.state().object(opp_bf).unwrap().chosen_subtype,
            Some(subtypes::creature::ELF)
        );

        // Give 3 cards in them()'s library Elf subtype and 1 Goblin subtype for Favored Enemy!
        let lib_ids: Vec<ObjectId> = engine
            .state()
            .zones
            .list(ZoneLocation::Library(them()))
            .iter()
            .take(4)
            .copied()
            .collect();
        for (i, id) in lib_ids.into_iter().enumerate() {
            let sub = if i < 3 {
                subtypes::creature::ELF
            } else {
                subtypes::creature::GOBLIN
            };
            let obj = engine.state.object_mut(id).unwrap();
            let base = std::sync::Arc::make_mut(&mut obj.base);
            base.types = TypeSet::CREATURE;
            base.subtypes = baylee_core::types::SubtypeSet::from_slice(&[sub]);
        }
        let fav = the(&engine, ZoneLocation::Hand(me()), FAVORED);
        cast_from_hand(&mut engine, me(), fav);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        engine
            .apply(me(), PlayerAction::ChoosePlayer(them()))
            .unwrap();
        settle(&mut engine, me());
        let fav_bf = the(&engine, ZoneLocation::Battlefield, FAVORED);
        assert_eq!(
            engine.state().object(fav_bf).unwrap().chosen_subtype,
            Some(subtypes::creature::ELF),
            "unique most prevalent creature type in opponent's library auto-noted"
        );
    }
}

// =========================================================================
// 5. ChooseNumber: Random, SecretChooseNumbers, SecretNumberGuess, Join Forces, Plague of Vermin
// =========================================================================

#[test]
fn choose_number_secret_guess_join_forces_and_plague_of_vermin() {
    const HAKTOS: u32 = 910_100;
    const LOCKBOX: u32 = 910_101;
    const HAPATO: u32 = 910_102;
    const WHEEL: u32 = 910_103;
    const GOBLIN_GAME: u32 = 910_104;
    const SAFE: u32 = 910_105;
    const TRAP: u32 = 910_106;
    const JOIN_FORCES: u32 = 910_107;
    const PLAGUE: u32 = 910_108;

    static HAKTOS_ENTERS: &[EnterModifier] =
        &[EnterModifier::ChooseNumberAtRandom { min: 2, max: 4 }];
    static LOCKBOX_ENTERS: &[EnterModifier] =
        &[EnterModifier::SecretlyChooseNumber { min: 1, max: 5 }];
    static HAPATO_ABS: &[AbilityDef] = &[free(
        &[Effect::ChooseNumberAtRandom {
            min: Amount::Fixed(2),
            max: Amount::Fixed(5),
            then: &[Effect::GainLife {
                amount: Amount::ResolutionChosenNumber,
            }],
        }],
        None,
    )];
    static WHEEL_ABS: &[AbilityDef] = &[free(
        &[
            Effect::SecretChooseNumbers(SecretNumberChoice {
                players: PlayerRel::EachPlayer,
                min: Amount::Fixed(0),
                max: Amount::Fixed(20),
                remember_highest: true,
                on_each: &[],
                on_highest: &[Effect::LoseLife {
                    target: PlayerRel::EventPlayer,
                    amount: Amount::ResolutionChosenNumber,
                }],
                on_lowest: &[],
                on_not_lowest: &[Effect::GainLifeFor {
                    who: PlayerRel::EventPlayer,
                    amount: Amount::Fixed(10),
                }],
                on_matched: &[],
                on_unmatched: &[],
            }),
            Effect::IfCondition {
                condition: Condition::YouAreRememberedPlayer,
                then: &[Effect::gain_life(1)],
                otherwise: &[],
            },
        ],
        None,
    )];
    static GOBLIN_GAME_ABS: &[AbilityDef] = &[free(
        &[Effect::SecretChooseNumbers(SecretNumberChoice {
            players: PlayerRel::EachPlayer,
            min: Amount::Fixed(1),
            max: Amount::Fixed(20),
            remember_highest: false,
            on_each: &[Effect::LoseLife {
                target: PlayerRel::EventPlayer,
                amount: Amount::ResolutionChosenNumber,
            }],
            on_highest: &[],
            on_lowest: &[Effect::LoseHalfLife {
                player: PlayerRel::EventPlayer,
            }],
            on_not_lowest: &[],
            on_matched: &[],
            on_unmatched: &[],
        })],
        None,
    )];
    static SAFE_ABS: &[AbilityDef] = &[free(
        &[Effect::SecretChooseNumbers(SecretNumberChoice {
            players: PlayerRel::EachPlayer,
            min: Amount::Fixed(1),
            max: Amount::Fixed(5),
            remember_highest: false,
            on_each: &[],
            on_highest: &[],
            on_lowest: &[],
            on_not_lowest: &[],
            on_matched: &[Effect::gain_life(8)],
            on_unmatched: &[Effect::LoseLife {
                amount: Amount::Fixed(3),
                target: PlayerRel::You,
            }],
        })],
        None,
    )];
    static TRAP_ABS: &[AbilityDef] = &[free(
        &[Effect::SecretNumberGuess {
            min: 1,
            max: 3,
            track_used_on_source: true,
            guesser: PlayerRel::Opponent,
            on_correct: &[Effect::gain_life(10)],
            on_wrong: &[Effect::LoseLife {
                target: PlayerRel::Chosen,
                amount: Amount::Fixed(4),
            }],
        }],
        None,
    )];
    static JOIN_FORCES_ABS: &[AbilityDef] = &[free(
        &[
            Effect::RepeatEach {
                domain: RepeatDomain::EachPlayerStartingWithYou,
                effects: &[Effect::ChooseNumberEx {
                    chooser: PlayerRel::EventPlayer,
                    min: Amount::Fixed(0),
                    max: Amount::Fixed(10),
                    prompt: NumberChoiceKind::ManaPayment,
                    pay_cost: Some(VariablePayKind::GenericMana),
                    then: &[Effect::AccumulateChosenNumber],
                }],
            },
            Effect::GainLife {
                amount: Amount::ResolutionAccumulatedNumber,
            },
        ],
        None,
    )];
    static PLAGUE_ABS: &[AbilityDef] = &[free(
        &[Effect::RepeatEachPlayerPayUntilNone {
            kind: VariablePayKind::Life,
            max_per_player: Amount::Fixed(20),
            then_per_player: &[Effect::GainLifeFor {
                who: PlayerRel::EventPlayer,
                amount: Amount::ResolutionChosenNumber,
            }],
        }],
        None,
    )];

    // Part A: Haktos (random entry number), Lockbox (secret entry number), Hapato (random effect number).
    {
        let cards = vec![
            card(
                HAKTOS,
                face_with_enters(creature_face("Haktos", "{0}", 6, 1), HAKTOS_ENTERS),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                LOCKBOX,
                face_with_enters(face("Lockbox", "{0}", TypeSet::ARTIFACT), LOCKBOX_ENTERS),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                HAPATO,
                face("Hapato", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                HAPATO_ABS,
            ),
        ];
        let mut engine = bench(
            9150,
            cards,
            [
                Seat::with(&[HAPATO]).holding(&[HAKTOS, LOCKBOX]),
                Seat::default(),
            ],
        );
        to_main(&mut engine, me());

        let hk = the(&engine, ZoneLocation::Hand(me()), HAKTOS);
        cast_from_hand(&mut engine, me(), hk);
        settle(&mut engine, me());
        let hk_bf = the(&engine, ZoneLocation::Battlefield, HAKTOS);
        let n = engine
            .state()
            .object(hk_bf)
            .unwrap()
            .chosen_number()
            .expect("random number chosen");
        assert!((2..=4).contains(&n), "Haktos chose {n} in 2..=4");

        let lb = the(&engine, ZoneLocation::Hand(me()), LOCKBOX);
        cast_from_hand(&mut engine, me(), lb);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        engine.apply(me(), PlayerAction::ChooseNumber(4)).unwrap();
        settle(&mut engine, me());
        let lb_bf = the(&engine, ZoneLocation::Battlefield, LOCKBOX);
        let lb_obj = engine.state().object(lb_bf).unwrap();
        assert_eq!(
            lb_obj.chosen_number(),
            None,
            "unrevealed secret chosen number must stay hidden from PublicObject"
        );
        assert_eq!(
            lb_obj.chosen_number_for_rules(),
            Some(4),
            "rules-internal accessor still reads unrevealed secret chosen number"
        );

        let hp = the(&engine, ZoneLocation::Battlefield, HAPATO);
        activate(&mut engine, me(), hp, 0);
        settle(&mut engine, me());
        let gained = life(&engine, me()) - 20;
        assert!(
            (2..=5).contains(&gained),
            "Hapato gained random {gained} in 2..=5"
        );
    }

    // Part B: Wheel of Misfortune (3P distinct + 2P tie), Goblin Game, Expert-Level Safe.
    {
        let cards = vec![
            card(
                WHEEL,
                face("Wheel", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                WHEEL_ABS,
            ),
            card(
                GOBLIN_GAME,
                face("GoblinGame", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GOBLIN_GAME_ABS,
            ),
            card(
                SAFE,
                face("Safe", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                SAFE_ABS,
            ),
        ];
        let mut e3 = bench(
            9151,
            cards.clone(),
            [Seat::with(&[WHEEL]), Seat::default(), Seat::default()],
        );
        to_main(&mut e3, me());
        let wh = the(&e3, ZoneLocation::Battlefield, WHEEL);
        activate(&mut e3, me(), wh, 0);
        walk_until(&mut e3, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        e3.apply(me(), PlayerAction::ChooseNumber(5)).unwrap();
        e3.apply(them(), PlayerAction::ChooseNumber(3)).unwrap();
        e3.apply(seat2(), PlayerAction::ChooseNumber(1)).unwrap();
        settle(&mut e3, me());
        assert_eq!(life(&e3, me()), 26);
        assert_eq!(life(&e3, them()), 30);
        assert_eq!(life(&e3, seat2()), 20);

        let mut e2 = bench(
            9152,
            cards,
            [Seat::with(&[WHEEL, GOBLIN_GAME, SAFE]), Seat::default()],
        );
        to_main(&mut e2, me());
        let wh2 = the(&e2, ZoneLocation::Battlefield, WHEEL);
        activate(&mut e2, me(), wh2, 0);
        walk_until(&mut e2, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        e2.apply(me(), PlayerAction::ChooseNumber(4)).unwrap();
        e2.apply(them(), PlayerAction::ChooseNumber(4)).unwrap();
        settle(&mut e2, me());
        assert_eq!(
            life(&e2, me()),
            17,
            "20 - 4 (highest) + 0 (tied lowest) + 1 (remembered)"
        );
        assert_eq!(life(&e2, them()), 16, "20 - 4 (highest) + 0 (tied lowest)");

        let gg = the(&e2, ZoneLocation::Battlefield, GOBLIN_GAME);
        activate(&mut e2, me(), gg, 0);
        walk_until(&mut e2, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        e2.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
        e2.apply(them(), PlayerAction::ChooseNumber(1)).unwrap();
        settle(&mut e2, me());
        assert_eq!(life(&e2, me()), 14);
        assert_eq!(life(&e2, them()), 7);

        let safe = the(&e2, ZoneLocation::Battlefield, SAFE);
        activate(&mut e2, me(), safe, 0);
        walk_until(&mut e2, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        e2.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
        e2.apply(them(), PlayerAction::ChooseNumber(2)).unwrap();
        settle(&mut e2, me());
        assert_eq!(life(&e2, me()), 22, "matched branch gained 8");

        activate(&mut e2, me(), safe, 0);
        walk_until(&mut e2, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        e2.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
        e2.apply(them(), PlayerAction::ChooseNumber(5)).unwrap();
        settle(&mut e2, me());
        assert_eq!(life(&e2, me()), 19, "unmatched branch lost 3");
    }

    // Part C: The Toymaker's Trap (`SecretNumberGuess` with used-number tracking).
    {
        let cards = vec![card(
            TRAP,
            face("Trap", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TRAP_ABS,
        )];
        let mut engine = bench(9153, cards, [Seat::with(&[TRAP]), Seat::default()]);
        to_main(&mut engine, me());
        let trap = the(&engine, ZoneLocation::Battlefield, TRAP);

        activate(&mut engine, me(), trap, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        let Pending::ChooseCastMode {
            player, options, ..
        } = engine.pending().clone()
        else {
            unreachable!();
        };
        assert_eq!(player, me());
        let kinds: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
        assert_eq!(
            kinds,
            vec![
                CastModeKind::Mode(1),
                CastModeKind::Mode(2),
                CastModeKind::Mode(3)
            ]
        );
        // Pick index 1 -> secret number 2.
        engine.apply(me(), PlayerAction::ChooseMode(1)).unwrap();
        engine.apply(them(), PlayerAction::ChooseNumber(1)).unwrap();
        settle(&mut engine, me());
        assert_eq!(life(&engine, them()), 16);

        // Second activation: number 2 is already used, so ONLY [1, 3] are offered!
        activate(&mut engine, me(), trap, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
            unreachable!();
        };
        let kinds: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
        assert_eq!(
            kinds,
            vec![CastModeKind::Mode(1), CastModeKind::Mode(3)],
            "already-used number 2 must not be offered"
        );
        assert!(
            engine.apply(me(), PlayerAction::ChooseMode(2)).is_err(),
            "out-of-range mode index rejected"
        );
        // Pick index 1 -> secret number 3.
        engine.apply(me(), PlayerAction::ChooseMode(1)).unwrap();
        engine.apply(them(), PlayerAction::ChooseNumber(3)).unwrap();
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 30);

        // Third activation: ONLY [1] remains.
        activate(&mut engine, me(), trap, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
            unreachable!();
        };
        let kinds: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
        assert_eq!(kinds, vec![CastModeKind::Mode(1)]);
        engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
        engine.apply(them(), PlayerAction::ChooseNumber(2)).unwrap();
        settle(&mut engine, me());
        assert_eq!(life(&engine, them()), 12);

        // Fourth activation: all numbers 1..=3 used -> does nothing.
        activate(&mut engine, me(), trap, 0);
        settle(&mut engine, me());
        assert_eq!(life(&engine, them()), 12);
    }

    // Part D: Join Forces (`EachPlayerStartingWithYou` + `AccumulateChosenNumber` + `ResolutionAccumulatedNumber`).
    {
        let cards = vec![card(
            JOIN_FORCES,
            face("JoinForces", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            JOIN_FORCES_ABS,
        )];
        let mut engine = bench(
            9154,
            cards,
            [Seat::with(&[JOIN_FORCES]), Seat::default(), Seat::default()],
        );
        to_main(&mut engine, them());
        add_pool(&mut engine, me(), ManaColor::Colorless, 3);
        add_pool(&mut engine, them(), ManaColor::Colorless, 2);
        add_pool(&mut engine, seat2(), ManaColor::Colorless, 4);
        engine.apply(them(), PlayerAction::PassPriority).unwrap();
        engine.apply(seat2(), PlayerAction::PassPriority).unwrap();
        let jf = the(&engine, ZoneLocation::Battlefield, JOIN_FORCES);
        activate(&mut engine, me(), jf, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });

        assert!(matches!(
            engine.pending(),
            Pending::ChooseNumber { player, reason: NumberPrompt::VariableManaPayment, .. } if *player == me()
        ));
        engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
        // Close them()'s ManaForNumber window, then assert them() gets ChooseNumber!
        assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == them()));
        engine.apply(them(), PlayerAction::PassPriority).unwrap();
        assert!(matches!(
            engine.pending(),
            Pending::ChooseNumber { player, reason: NumberPrompt::VariableManaPayment, .. } if *player == them()
        ));
        engine.apply(them(), PlayerAction::ChooseNumber(1)).unwrap();
        // Close seat2()'s ManaForNumber window, then assert seat2() gets ChooseNumber!
        assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat2()));
        engine.apply(seat2(), PlayerAction::PassPriority).unwrap();
        assert!(matches!(
            engine.pending(),
            Pending::ChooseNumber { player, reason: NumberPrompt::VariableManaPayment, .. } if *player == seat2()
        ));
        engine
            .apply(seat2(), PlayerAction::ChooseNumber(3))
            .unwrap();
        settle(&mut engine, them());
        assert_eq!(
            life(&engine, me()),
            26,
            "ResolutionAccumulatedNumber = 2+1+3 = 6"
        );
    }

    // Part E: Plague of Vermin (`RepeatEachPlayerPayUntilNone`).
    {
        let cards = vec![card(
            PLAGUE,
            face("Plague", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PLAGUE_ABS,
        )];
        let mut engine = bench(9155, cards, [Seat::with(&[PLAGUE]), Seat::default()]);
        to_main(&mut engine, me());
        let pl = the(&engine, ZoneLocation::Battlefield, PLAGUE);
        activate(&mut engine, me(), pl, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });

        engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
        engine.apply(them(), PlayerAction::ChooseNumber(2)).unwrap();
        engine.apply(me(), PlayerAction::ChooseNumber(1)).unwrap();
        engine.apply(them(), PlayerAction::ChooseNumber(0)).unwrap();
        engine.apply(me(), PlayerAction::ChooseNumber(0)).unwrap();
        engine.apply(them(), PlayerAction::ChooseNumber(0)).unwrap();
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 20, "me() total paid = 3+1 = 4");
        assert_eq!(life(&engine, them()), 20, "them() total paid = 2");
        assert_eq!(
            engine.state().per_turn.life_lost_amount[0],
            4,
            "me() paid 4 total life across two rounds"
        );
        assert_eq!(
            engine.state().per_turn.life_lost_amount[1],
            2,
            "them() paid 2 total life"
        );
    }
}

// =========================================================================
// 6. Snapshot Hash Sensitivity & Checkpoint Determinism
// =========================================================================

#[test]
fn snapshot_hash_and_checkpoint_cover_new_riders_and_continuations() {
    const TRAP: u32 = 910_120;
    static TRAP_ABS: &[AbilityDef] = &[free(
        &[Effect::SecretNumberGuess {
            min: 1,
            max: 3,
            track_used_on_source: true,
            guesser: PlayerRel::Opponent,
            on_correct: &[Effect::gain_life(5)],
            on_wrong: &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            }],
        }],
        None,
    )];

    let cards = vec![card(
        TRAP,
        face("Trap", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TRAP_ABS,
    )];
    let mut e1 = bench(9160, cards.clone(), [Seat::with(&[TRAP]), Seat::default()]);
    let mut e1_copy = bench(9160, cards.clone(), [Seat::with(&[TRAP]), Seat::default()]);
    let mut e2 = bench(9160, cards, [Seat::with(&[TRAP]), Seat::default()]);
    to_main(&mut e1_copy, me());
    to_main(&mut e1, me());
    to_main(&mut e2, me());

    let t1 = the(&e1, ZoneLocation::Battlefield, TRAP);
    let base_hash = e1.state().snapshot_hash();

    e1.state
        .object_mut(t1)
        .unwrap()
        .riders
        .push(Rider::TreasureManaSpent);
    let h_treasure = e1.state().snapshot_hash();
    assert_ne!(
        base_hash, h_treasure,
        "Rider::TreasureManaSpent changes hash"
    );

    e1.state
        .object_mut(t1)
        .unwrap()
        .riders
        .push(Rider::SecondChosenPlayer(them()));
    let h_second = e1.state().snapshot_hash();
    assert_ne!(
        h_treasure, h_second,
        "Rider::SecondChosenPlayer changes hash"
    );

    e1.state
        .object_mut(t1)
        .unwrap()
        .set_secret_chosen_player(them());
    let h_secret_unrev = e1.state().snapshot_hash();
    assert_ne!(
        h_second, h_secret_unrev,
        "Rider::SecretChosenPlayer(unrevealed) changes hash"
    );

    e1.state
        .object_mut(t1)
        .unwrap()
        .reveal_secret_chosen_player();
    let h_secret_rev = e1.state().snapshot_hash();
    assert_ne!(
        h_secret_unrev, h_secret_rev,
        "revealing SecretChosenPlayer changes hash"
    );

    e1.state.object_mut(t1).unwrap().set_secret_chosen_number(3);
    let h_sec_num = e1.state().snapshot_hash();
    assert_ne!(
        h_secret_rev, h_sec_num,
        "Rider::SecretChosenNumber changes hash"
    );

    e1.state.object_mut(t1).unwrap().mark_used_chosen_number(2);
    let h_used = e1.state().snapshot_hash();
    assert_ne!(h_sec_num, h_used, "Rider::UsedChosenNumbers changes hash");

    e1.state.object_mut(t1).unwrap().riders.clear();
    assert_eq!(e1.state().snapshot_hash(), base_hash);

    let t2 = the(&e2, ZoneLocation::Battlefield, TRAP);
    activate(&mut e1, me(), t1, 0);
    activate(&mut e2, me(), t2, 0);
    walk_until(&mut e1, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    walk_until(&mut e2, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });

    e1.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    e2.apply(me(), PlayerAction::ChooseMode(1)).unwrap();
    assert_ne!(
        e1.snapshot_hash(),
        e2.snapshot_hash(),
        "AwaitingOp::SecretGuessAnswer hashes the secret number (tag 550)"
    );

    // Identical replays at SecretGuessAnswer produce the exact same hash and outcome,
    // while different secrets diverge cleanly.
    let t1_copy = the(&e1_copy, ZoneLocation::Battlefield, TRAP);
    activate(&mut e1_copy, me(), t1_copy, 0);
    walk_until(&mut e1_copy, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e1_copy.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    assert_eq!(e1.snapshot_hash(), e1_copy.snapshot_hash());
    e1.apply(them(), PlayerAction::ChooseNumber(1)).unwrap();
    e1_copy
        .apply(them(), PlayerAction::ChooseNumber(1))
        .unwrap();
    settle(&mut e1, me());
    settle(&mut e1_copy, me());
    assert_eq!(e1.snapshot_hash(), e1_copy.snapshot_hash());
    assert_eq!(life(&e1, me()), 25, "correct guess ran on_correct (+5)");

    e2.apply(them(), PlayerAction::ChooseNumber(1)).unwrap();
    settle(&mut e2, me());
    assert_eq!(life(&e2, me()), 18, "wrong guess ran on_wrong (-2)");
}

// =========================================================================
// 7. Review regressions: EventPlayer scoping, multi-recipient AddManaChoiceFor,
//    CR 400.7 SecretNumberGuess, removed-from-combat last_combat_attacked,
//    and state/reads.rs walkers.
// =========================================================================

#[test]
fn secret_choose_numbers_preserves_outer_chosen_player_and_chosen_number() {
    const ARTIFACT: u32 = 910_130;
    static ABS: &[AbilityDef] = &[free(
        &[
            Effect::ChoosePlayerEx {
                chooser: PlayerRel::You,
                filter: PlayerFilter::Among(PlayerRel::Opponent),
                random: false,
                remember: false,
                if_none: &[],
                then: &[],
            },
            Effect::ChooseNumberAtRandom {
                min: Amount::Fixed(7),
                max: Amount::Fixed(7),
                then: &[],
            },
            Effect::SecretChooseNumbers(SecretNumberChoice {
                players: PlayerRel::EachPlayer,
                min: Amount::Fixed(1),
                max: Amount::Fixed(5),
                remember_highest: false,
                on_each: &[Effect::LoseLife {
                    target: PlayerRel::EventPlayer,
                    amount: Amount::ResolutionChosenNumber,
                }],
                on_highest: &[],
                on_lowest: &[],
                on_not_lowest: &[],
                on_matched: &[],
                on_unmatched: &[],
            }),
            // Outer `res.chosen_player` (seat2) and `res.subject.chosen_number` (7)
            // must BOTH be intact after SecretChooseNumbers finishes!
            Effect::LoseLife {
                target: PlayerRel::Chosen,
                amount: Amount::ResolutionChosenNumber,
            },
        ],
        None,
    )];

    let cards = vec![card(
        ARTIFACT,
        face("Preserve", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ABS,
    )];
    let mut engine = bench(
        9170,
        cards,
        [Seat::with(&[ARTIFACT]), Seat::default(), Seat::default()],
    );
    to_main(&mut engine, me());
    let art = the(&engine, ZoneLocation::Battlefield, ARTIFACT);
    activate(&mut engine, me(), art, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine
        .apply(me(), PlayerAction::ChoosePlayer(seat2()))
        .unwrap();
    // SecretChooseNumbers asks me(), them(), seat2() in APNAP order.
    engine.apply(me(), PlayerAction::ChooseNumber(1)).unwrap();
    engine.apply(them(), PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(seat2(), PlayerAction::ChooseNumber(3))
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 19, "me() lost 1 via EventPlayer");
    assert_eq!(life(&engine, them()), 18, "them() lost 2 via EventPlayer");
    assert_eq!(
        life(&engine, seat2()),
        20 - 3 - 7,
        "seat2() lost 3 via EventPlayer AND 7 via preserved outer Chosen + ResolutionChosenNumber"
    );
}

#[test]
fn add_mana_choice_for_each_player_prompts_all_seats_in_apnap_order_and_hashes() {
    const MANA_FLARE: u32 = 910_131;
    static ABS: &[AbilityDef] = &[free(
        &[Effect::AddManaChoiceFor {
            who: PlayerRel::EachPlayer,
            source: ManaSource::Choice(FIVE_COLORS),
            amount: Amount::Fixed(1),
            distribution: ManaDistribution::OneColor,
            restriction: None,
        }],
        None,
    )];

    let cards = vec![card(
        MANA_FLARE,
        face("GroupMana", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ABS,
    )];
    let mut e1 = bench(
        9171,
        cards.clone(),
        [Seat::with(&[MANA_FLARE]), Seat::default(), Seat::default()],
    );
    let mut e2 = bench(
        9171,
        cards,
        [Seat::with(&[MANA_FLARE]), Seat::default(), Seat::default()],
    );
    to_main(&mut e1, me());
    to_main(&mut e2, me());
    let m1 = the(&e1, ZoneLocation::Battlefield, MANA_FLARE);
    let m2 = the(&e2, ZoneLocation::Battlefield, MANA_FLARE);
    activate(&mut e1, me(), m1, 0);
    activate(&mut e2, me(), m2, 0);
    walk_until(&mut e1, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    walk_until(&mut e2, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });

    assert!(matches!(e1.pending(), Pending::ChooseColor { player, .. } if *player == me()));
    e1.apply(me(), PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    e2.apply(me(), PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert!(matches!(e1.pending(), Pending::ChooseColor { player, .. } if *player == them()));
    assert_ne!(
        e1.snapshot_hash(),
        e2.snapshot_hash(),
        "suspended AwaitingOp::ManaChoiceFor preserves divergent state hashes"
    );

    e1.apply(them(), PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert!(matches!(e1.pending(), Pending::ChooseColor { player, .. } if *player == seat2()));
    e1.apply(seat2(), PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    settle(&mut e1, me());

    assert!(crate::mana_pay::can_pay(
        &e1.state().players[0].mana_pool,
        &ManaCost::parse("{R}")
    ));
    assert!(crate::mana_pay::can_pay(
        &e1.state().players[1].mana_pool,
        &ManaCost::parse("{U}")
    ));
    assert!(crate::mana_pay::can_pay(
        &e1.state().players[2].mana_pool,
        &ManaCost::parse("{G}")
    ));
}

#[test]
fn secret_number_guess_enforces_cr_400_7_when_source_is_blinked() {
    const TRAP: u32 = 910_132;
    static TRAP_ABS: &[AbilityDef] = &[free(
        &[Effect::SecretNumberGuess {
            min: 1,
            max: 3,
            track_used_on_source: true,
            guesser: PlayerRel::Opponent,
            on_correct: &[Effect::gain_life(5)],
            on_wrong: &[Effect::LoseLife {
                target: PlayerRel::Chosen,
                amount: Amount::Fixed(2),
            }],
        }],
        None,
    )];

    let cards = vec![card(
        TRAP,
        face("Trap", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TRAP_ABS,
    )];
    let mut engine = bench(9172, cards, [Seat::with(&[TRAP]), Seat::default()]);
    to_main(&mut engine, me());
    let trap = the(&engine, ZoneLocation::Battlefield, TRAP);

    // Put the ability on the stack referencing incarnation v1, then blink Trap
    // (exile + return -> new incarnation) and mark `2` used on the NEW object.
    activate(&mut engine, me(), trap, 0);
    let _ = engine.state.move_object(
        trap,
        ZoneLocation::Exile(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    let _ = engine.state.move_object(
        trap,
        ZoneLocation::Battlefield,
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    engine
        .state
        .object_mut(trap)
        .unwrap()
        .mark_used_chosen_number(2);

    // Resolve the v1 ability: per CR 400.7, it must NOT read the new object's
    // used bitmask (so 1, 2, 3 are all offered) and must NOT mark `1` on the new object!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let kinds: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
    assert_eq!(
        kinds,
        vec![
            CastModeKind::Mode(1),
            CastModeKind::Mode(2),
            CastModeKind::Mode(3)
        ],
        "CR 400.7: blinked source's used numbers must not leak into old incarnation's ability"
    );
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    engine.apply(them(), PlayerAction::ChooseNumber(1)).unwrap();
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(trap).unwrap().used_chosen_numbers(),
        1_u32 << 2,
        "CR 400.7: old incarnation's resolution must not mutate the new incarnation's used mask"
    );
}

#[test]
fn last_combat_attacked_keeps_removed_attackers_and_ignores_put_into_play_attacking() {
    const HELLKITE: u32 = 910_133;
    const UNDO_COMBAT: u32 = 910_134;
    static HELLKITE_ABS: &[AbilityDef] = &[free(
        &[Effect::ChoosePlayerEx {
            chooser: PlayerRel::You,
            filter: PlayerFilter::OpponentNotAttackedBySourceLastCombat,
            random: false,
            remember: false,
            if_none: &[Effect::TapAll {
                filter: &Filter::This,
            }],
            then: &[Effect::LoseLife {
                target: PlayerRel::Chosen,
                amount: Amount::Fixed(2),
            }],
        }],
        None,
    )];
    static REMOVE_ABS: &[AbilityDef] = &[free(
        &[Effect::RemoveTargetFromCombat { unblock: false }],
        Some(TargetReq::one(baylee_cards_dsl::TargetSpec::Object(
            &Filter::CREATURE,
        ))),
    )];

    let cards = vec![
        card(
            HELLKITE,
            creature_face("Hellkite", "{2}{R}{R}", 6, 5),
            KeywordSet::HASTE,
            HELLKITE_ABS,
        ),
        card(
            UNDO_COMBAT,
            face("Maze", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            REMOVE_ABS,
        ),
    ];
    let mut engine = bench(
        9173,
        cards,
        [Seat::with(&[HELLKITE, UNDO_COMBAT]), Seat::default()],
    );
    to_attackers(&mut engine, me());
    let hk = the(&engine, ZoneLocation::Battlefield, HELLKITE);
    let maze = the(&engine, ZoneLocation::Battlefield, UNDO_COMBAT);

    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(hk, Defender::Player(them()))],
            },
        )
        .expect("Hellkite declared attacking them()");

    // Verify tag 555 subtag 3 hashes mid-combat declared_against_players when tracked.
    let hash_with_declared = engine.state().snapshot_hash();
    let saved = engine.state.combat.declared_against_players.clone();
    engine.state.combat.declared_against_players.clear();
    assert_ne!(
        hash_with_declared,
        engine.state().snapshot_hash(),
        "CombatState::declared_against_players must affect snapshot_hash when tracked"
    );
    engine.state.combat.declared_against_players = saved;

    // Remove Hellkite from combat BEFORE combat ends!
    activate(&mut engine, me(), maze, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![hk],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        e.state().combat.attackers().is_empty() && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(
        engine.state().combat.attackers().is_empty(),
        "Hellkite was removed from combat"
    );

    // Finish combat and advance to our next main phase: them() must STILL count
    // as attacked by Hellkite last combat even though Hellkite was removed from combat!
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    let life_before = life(&engine, them());
    activate(&mut engine, me(), hk, 0);
    settle(&mut engine, me());
    assert!(
        is_tapped(&engine, hk),
        "removed-from-combat declared attacker still excludes them() on next combat"
    );
    assert_eq!(life(&engine, them()), life_before);

    // Put-into-play-attacking (CR 508.4) adds to combat.attackers without going through
    // declare_attackers -> must NOT populate last_combat_attacked!
    to_attackers(&mut engine, me());
    engine
        .state
        .combat
        .declare_attackers(std::iter::once(crate::combat::AttackerInfo {
            creature: hk,
            defending: Defender::Player(them()),
            blocked: false,
            band: None,
        }));
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert!(
        engine.state().players[0].last_combat_attacked.is_empty(),
        "CR 508.4: non-declared attacking creature must not populate last_combat_attacked"
    );
}

#[test]
fn state_reads_detects_reflexive_triggers_and_granted_abilities() {
    use baylee_cards_dsl::Cost;
    use baylee_core::mana::ManaCost;

    static REFLEXIVE_CARD_ABS: &[AbilityDef] = &[free(
        &[Effect::ReflexiveTrigger {
            when: baylee_cards_dsl::ReflexiveWhen::Always,
            once: true,
            condition: Some(&Condition::PlayerAttackedYouThisTurn(PlayerRel::Opponent)),
            targets: None,
            remember: baylee_cards_dsl::TriggerRemember::None,
            effects: &[Effect::gain_life(1)],
        }],
        None,
    )];
    static GRANTED_ATTACKED_MOD: Modifier = Modifier::GrantActivated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[],
        },
        effects: &[Effect::IfCondition {
            condition: Condition::PlayerAttackedYouThisTurn(PlayerRel::Opponent),
            then: &[Effect::gain_life(1)],
            otherwise: &[],
        }],
        mana_ability: false,
    };
    static GRANTED_ATTACKED_ABS: &[AbilityDef] =
        &[static_ability!(Filter::CREATURE, GRANTED_ATTACKED_MOD)];
    static GRANTED_LAST_COMBAT_MOD: Modifier = Modifier::GrantTriggered {
        trigger: Trigger::StepBegin {
            step: baylee_cards_dsl::StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        effects: &[Effect::ChoosePlayerEx {
            chooser: PlayerRel::You,
            filter: PlayerFilter::OpponentNotAttackedBySourceLastCombat,
            random: true,
            remember: false,
            if_none: &[],
            then: &[],
        }],
        target: None,
    };
    static GRANTED_LAST_COMBAT_ABS: &[AbilityDef] =
        &[static_ability!(Filter::CREATURE, GRANTED_LAST_COMBAT_MOD)];

    let c1 = card(
        910_140,
        face("Reflexive", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        REFLEXIVE_CARD_ABS,
    );
    let c2 = card(
        910_141,
        face("GrantAttacked", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        GRANTED_ATTACKED_ABS,
    );
    let c3 = card(
        910_142,
        face("GrantLastCombat", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        GRANTED_LAST_COMBAT_ABS,
    );

    assert!(
        crate::state::reads::card_reads_attacked_players(c1),
        "ReflexiveTrigger condition must activate attacked_players tracking"
    );
    assert!(
        crate::state::reads::card_reads_attacked_players(c2),
        "GrantActivated effect condition must activate attacked_players tracking"
    );
    assert!(
        crate::state::reads::card_reads_last_combat_attacked(c3),
        "GrantTriggered effect must activate last_combat_attacked tracking"
    );
}
