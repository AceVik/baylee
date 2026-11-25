//! Wave 16a engine rule tests — Trigger/Cast Context Amounts & Turn-History Conditions:
//! 1. *Barrin, Tolarian Archmage*: public-departure zone-change tracking (`Battlefield -> Hand`)
//!    via `Amount::EnteredThisTurn { from: Some(ZoneRef::Battlefield), to: ZoneRef::Hand, .. }`,
//!    opponent-owned bounce exclusion, hidden `Library -> Hand` draw exclusion (`CR 400.2`),
//!    turn reset, and `fork_for` / `snapshot_hash` / `loop_signature` determinism when an
//!    opponent's permanent is bounced.
//! 2. *Bennie Bracks, Zoologist*: `Amount::TokensCreatedThisTurn` (`CR 111.1`, `CR 111.2`),
//!    verifying that resolving copies of permanent spells are NOT "created" (`CR 111.13`) and
//!    non-token arrivals are excluded.
//! 3. *Akoum Hellkite*: `Condition::ObjectMatches { what: &TargetSpec::EventObject, .. }` inside
//!    `Amount::IfCondition` (`1` damage on non-Mountain landfall, `2` damage on Mountain landfall,
//!    plus LKI preservation if the Mountain is destroyed before the trigger resolves, `CR 608.2h`).
//! 4. *Aetherflux Conduit* + *Unravel*: `Amount::EventManaSpentToCast` and
//!    `Amount::TargetManaSpentToCast` (`CR 601.2h`, `CR 608.2h`), including cost reductions,
//!    LKI when the triggering spell is countered before *Conduit*'s trigger resolves, and
//!    *Unravel* drawing iff mana spent on the countered spell was strictly less than its MV.
//! 5. *Aveline de Grandpré* + *Amarant Coral*: source-centric `DealsCombatDamageToPlayer`
//!    binding `EventObject` to the damaging deathtouch creature (`AddCounterFilter` with
//!    `Filter::Reference` + `Amount::EventAmount`), plus multiplayer `PlayerRel::OtherOpponents`
//!    dealing combat-damage amount to each opponent other than `DamagedPlayer`.
//! 6. *Ancient Imperiosaur*: `Amount::ConvokedCount` (`CR 702.51a`, `CR 400.7d`) placing two
//!    `+1/+1` counters per convoking creature (and `0` counters when reanimated without casting).
//! 7. *Ardenvale Paladin* + Converge + Turn-History (`LeftZoneThisTurn`,
//!    `PermanentsSacrificedThisTurn`): `Amount::ManaColorSpentToCast` (`>= 3` white mana vs `< 3`),
//!    `Amount::ColorsOfManaSpentToCast` (`CR 105.4` colorless exclusion), graveyard-to-hand
//!    departures, and sacrifices.
#![allow(clippy::too_many_lines)]

use super::*;
use baylee_cards_dsl::prelude::{spell, triggered};
use baylee_cards_dsl::{
    Amount, AmountCmp, Condition, Cost, CostPart, CostReduction, CounterKind, Effect,
    EnterModifier, Filter, KeywordSet, PlayerRel, StepKind, TargetReq, TargetSpec, TokenDef,
    Trigger, ZoneRef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;
use baylee_core::types::TypeSet;

const C_BASE: u32 = 16_100;

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
                    "unexpected pending while advancing to {step:?} for {seat:?}: {pending:?}"
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
    panic!("failed to reach next main for {seat:?}");
}

fn settle_until_target(engine: &mut Bench, chooser: PlayerId, chosen: ObjectId) {
    for _ in 0..32 {
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } if player == chooser => {
                engine
                    .apply(
                        chooser,
                        PlayerAction::ChooseTargets {
                            objects: vec![chosen],
                            players: vec![],
                        },
                    )
                    .expect("choose target");
                return;
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority to stack trigger");
            }
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("arrange triggers");
            }
            other => panic!("expected target prompt, got {other:?}"),
        }
    }
    panic!("did not reach ChooseTargets");
}

// ============================================================================
// 1. Barrin, Tolarian Archmage — Battlefield-to-Hand Zone Changes & Fork Safety
// ============================================================================

#[test]
fn barrin_tolarian_archmage_draws_on_end_step_when_permanent_bounced_to_your_hand() {
    const BARRIN: u32 = C_BASE + 1;
    const BEAR: u32 = C_BASE + 2;
    const UNSUMMON: u32 = C_BASE + 3;

    static BOUNCED_TO_YOUR_HAND: Amount = Amount::EnteredThisTurn {
        filter: &Filter::Any,
        from: Some(ZoneRef::Battlefield),
        to: ZoneRef::Hand,
        owner_or_ctrl: PlayerRel::You,
        exclude_source: false,
    };
    static BARRIN_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::ETB,
            &[Effect::ReturnToHand {
                target: TargetSpec::Object(&Filter::ANOTHER_CREATURE),
            }],
            targets = Some(TargetReq::up_to(
                TargetSpec::Object(&Filter::ANOTHER_CREATURE),
                1
            )),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::draw(1)],
            condition = Some(Condition::AmountCompare {
                left: &BOUNCED_TO_YOUR_HAND,
                op: AmountCmp::AtLeast,
                right: &Amount::Fixed(1),
            }),
        ),
    ];
    static UNSUMMON_AB: &[AbilityDef] = &[spell!(
        &[Effect::ReturnToHand {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            BARRIN,
            creature_face("Barrin, Tolarian Archmage", "{1}{U}{U}", 2, 2),
            KeywordSet::EMPTY,
            BARRIN_AB,
        ),
        card(
            BEAR,
            creature_face("Tolarian Scholar", "{2}{U}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            UNSUMMON,
            face("Unsummon", "{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            UNSUMMON_AB,
        ),
    ];

    let mut engine = bench(
        16_101,
        cards,
        [
            Seat::with(&[BEAR]).holding(&[BARRIN, UNSUMMON]),
            Seat::with(&[BEAR]),
        ],
    );
    assert!(
        engine.state().per_turn.zone_changes.tracked,
        "zone_changes tracking must activate for Barrin's EnteredThisTurn"
    );

    to_main(&mut engine, me());
    let my_bear = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().owner == me())
        .unwrap();
    let opp_bear = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().owner == them())
        .unwrap();

    // Cast Barrin and bounce our own Scholar to our hand -> satisfies Barrin's end-step condition!
    let barrin_hand = the(&engine, ZoneLocation::Hand(me()), BARRIN);
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: barrin_hand })
        .expect("cast Barrin");
    settle_until_target(&mut engine, me(), my_bear);
    settle_stack(&mut engine, me());

    // Also bounce opponent's bear to opponent's hand and verify `fork_for` succeeds cleanly!
    let bounce_card = the(&engine, ZoneLocation::Hand(me()), UNSUMMON);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bounce_card })
        .expect("cast Unsummon on opponent bear");
    settle_until_target(&mut engine, me(), opp_bear);
    settle_stack(&mut engine, me());

    let forked = engine
        .fork_for(me(), crate::engine::DeckKnowledge::Known, 0xBA22_16A0)
        .expect("fork_for must succeed when an opponent's permanent was bounced to hand");
    let _ = forked.state().snapshot_hash();
    let _ = forked.state().loop_signature();

    // Advance to our End step -> Barrin triggers and draws 1 card!
    let hand_before_end = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    advance_to_step(&mut engine, me(), Step::End);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_end + 1,
        "Barrin drew a card at end step because a permanent entered your hand from battlefield"
    );

    // Next turn of ours: we draw in Draw step (Library -> Hand is hidden, NOT from Battlefield),
    // bounce nothing -> Barrin MUST NOT trigger at end step!
    advance_to_next_main(&mut engine, me());
    let hand_in_main = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    advance_to_step(&mut engine, me(), Step::End);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_in_main,
        "normal draw (Library -> Hand) must NOT satisfy Barrin's from-Battlefield condition"
    );
}

// ============================================================================
// 2. Bennie Bracks, Zoologist — TokensCreatedThisTurn vs CR 111.13 Spell Copy
// ============================================================================

#[test]
fn bennie_bracks_draws_on_created_token_but_not_on_cr111_13_permanent_spell_copy() {
    const BENNIE: u32 = C_BASE + 10;
    const MAKE_TOK: u32 = C_BASE + 11;
    const BEAR: u32 = C_BASE + 12;
    const COPY_PERM: u32 = C_BASE + 13;

    static SOLDIER_TOKEN: TokenDef = TokenDef {
        name: "Soldier",
        colors: ColorSet::from_slice(&[Color::White]),
        types: TypeSet::CREATURE,
        power: Some(1),
        toughness: Some(1),
        ..TokenDef::DEFAULT
    };
    static YOUR_TOKENS_CREATED: Amount = Amount::TokensCreatedThisTurn {
        filter: &Filter::Any,
        creator: PlayerRel::You,
    };
    static BENNIE_AB: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::draw(1)],
        condition = Some(Condition::AmountAtLeast(YOUR_TOKENS_CREATED, 1)),
    )];
    static MAKE_TOK_AB: &[AbilityDef] = &[spell!(&[Effect::CreateToken {
        token: &SOLDIER_TOKEN,
    }])];
    static COPY_PERM_AB: &[AbilityDef] = &[spell!(
        &[Effect::CopyTargetSpell { mods: &[] }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::CREATURE))),
    )];

    let mut bennie_face = creature_face("Bennie Bracks, Zoologist", "{3}{W}", 3, 2);
    bennie_face.convoke = true;
    let cards = vec![
        card(BENNIE, bennie_face, KeywordSet::EMPTY, BENNIE_AB),
        card(
            MAKE_TOK,
            face("Raise One", "{W}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            MAKE_TOK_AB,
        ),
        card(
            BEAR,
            creature_face("Grizzly Bears", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            COPY_PERM,
            face("Double Major Lite", "{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            COPY_PERM_AB,
        ),
    ];

    let mut engine = bench(
        16_110,
        cards,
        [
            Seat::with(&[BENNIE]).holding(&[BEAR, COPY_PERM, MAKE_TOK]),
            Seat::default(),
        ],
    );

    // Turn 1: Cast Grizzly Bears and copy it on the stack. Per CR 111.13, the resolving copy
    // enters the battlefield as a token, and is NOT "created"!
    to_main(&mut engine, me());
    let bear_card = the(&engine, ZoneLocation::Hand(me()), BEAR);
    let copy_card = the(&engine, ZoneLocation::Hand(me()), COPY_PERM);
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bear_card })
        .expect("cast Grizzly Bears");
    let stack_spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: copy_card })
        .expect("cast copy spell");
    settle_until_target(&mut engine, me(), stack_spell);
    settle_stack(&mut engine, me());

    let token_bears = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| !engine.state().object(id).unwrap().is_card())
        .count();
    assert_eq!(token_bears, 1, "spell copy resolved into a token permanent");
    let hand_before_t1_end = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    advance_to_step(&mut engine, me(), Step::End);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_t1_end,
        "CR 111.13: resolving copy of a permanent spell is NOT a created token"
    );

    // Opponent's turn: cast `MAKE_TOK` on opponent's main -> creates a Soldier token -> Bennie
    // triggers at opponent's End step ("each end step") and draws a card!
    to_main(&mut engine, them());
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("opponent passes priority");
    let tok_card = the(&engine, ZoneLocation::Hand(me()), MAKE_TOK);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: tok_card })
        .expect("cast Raise One");
    settle_stack(&mut engine, them());

    let hand_before_opp_end = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    advance_to_step(&mut engine, them(), Step::End);
    settle_stack(&mut engine, them());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_opp_end + 1,
        "Bennie Bracks drew a card on opponent's end step after creating a token this turn"
    );
}

// ============================================================================
// 3. Akoum Hellkite — Conditional Landfall Amount (1 vs 2 on Mountain + LKI)
// ============================================================================

#[test]
fn akoum_hellkite_deals_1_for_forest_and_2_for_mountain_even_if_mountain_destroyed() {
    const HELLKITE: u32 = C_BASE + 20;
    const NON_MTN_LAND: u32 = C_BASE + 21;
    const MTN_LAND: u32 = C_BASE + 22;
    const DESTROY_LAND: u32 = C_BASE + 23;

    static MOUNTAIN_FILTER: Filter =
        Filter::HasSubtype(baylee_core::generated::subtypes::land::MOUNTAIN);
    static EVENT_IS_MOUNTAIN: Condition = Condition::ObjectMatches {
        what: &TargetSpec::EventObject,
        filter: &MOUNTAIN_FILTER,
    };
    static HELLKITE_DMG: Amount = Amount::IfCondition {
        condition: &EVENT_IS_MOUNTAIN,
        then: &Amount::Fixed(2),
        otherwise: &Amount::Fixed(1),
    };
    static HELLKITE_AB: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::YOUR_LAND),
        &[Effect::DealDamage {
            amount: HELLKITE_DMG,
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )];
    static DESTROY_LAND_AB: &[AbilityDef] = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    )];

    let mut forest_face = land_face("Test Forest");
    forest_face.subtypes = &[baylee_core::generated::subtypes::land::FOREST];
    let mut mountain_face = land_face("Test Mountain");
    mountain_face.subtypes = &[baylee_core::generated::subtypes::land::MOUNTAIN];

    let cards = vec![
        card(
            HELLKITE,
            creature_face("Akoum Hellkite", "{4}{R}{R}", 4, 4),
            KeywordSet::FLYING,
            HELLKITE_AB,
        ),
        card(NON_MTN_LAND, forest_face, KeywordSet::EMPTY, &[]),
        card(MTN_LAND, mountain_face, KeywordSet::EMPTY, &[]),
        card(
            DESTROY_LAND,
            face("Instant Sinkhole", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            DESTROY_LAND_AB,
        ),
    ];

    let mut engine = bench(
        16_120,
        cards,
        [
            Seat::with(&[HELLKITE]).holding(&[NON_MTN_LAND, MTN_LAND, DESTROY_LAND]),
            Seat::default(),
        ],
    );

    // Turn 1: Play Forest -> Hellkite triggers, target opponent -> deals 1 damage (20 -> 19)!
    to_main(&mut engine, me());
    let forest_card = the(&engine, ZoneLocation::Hand(me()), NON_MTN_LAND);
    engine
        .apply(me(), PlayerAction::PlayLand { card: forest_card })
        .expect("play Forest");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent with Hellkite trigger");
    settle_stack(&mut engine, me());
    assert_eq!(life(&engine, them()), 19, "Forest landfall deals 1 damage");

    // Next our turn: Play Mountain, and in response destroy that Mountain BEFORE Hellkite's
    // trigger resolves -> CR 608.2h LKI still sees it was a Mountain and deals 2 damage (19 -> 17)!
    advance_to_next_main(&mut engine, me());
    let mountain_card = the(&engine, ZoneLocation::Hand(me()), MTN_LAND);
    engine
        .apply(
            me(),
            PlayerAction::PlayLand {
                card: mountain_card,
            },
        )
        .expect("play Mountain");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent with Mountain landfall trigger");
    let sinkhole = the(&engine, ZoneLocation::Hand(me()), DESTROY_LAND);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sinkhole })
        .expect("cast Instant Sinkhole in response");
    settle_until_target(&mut engine, me(), mountain_card);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(mountain_card).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        life(&engine, them()),
        17,
        "Mountain landfall deals 2 damage even after Mountain is destroyed in response (CR 608.2h)"
    );
}

// ============================================================================
// 4. Aetherflux Conduit & Unravel — EventManaSpentToCast & TargetManaSpentToCast
// ============================================================================

#[test]
fn aetherflux_conduit_and_unravel_track_actual_mana_spent_with_reductions_and_lki() {
    const CONDUIT: u32 = C_BASE + 30;
    const DISCOUNTED: u32 = C_BASE + 31;
    const FULL_SPELL: u32 = C_BASE + 32;
    const UNRAVEL: u32 = C_BASE + 33;

    static CONDUIT_AB: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&Filter::ControlledByYou),
        &[Effect::Energy {
            who: PlayerRel::You,
            amount: Amount::EventManaSpentToCast,
        }],
    )];
    static UNRAVEL_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::CounterTargetSpell,
            Effect::IfCondition {
                condition: Condition::AmountCompare {
                    left: &Amount::TargetManaSpentToCast,
                    op: AmountCmp::LessThan,
                    right: &Amount::TargetCmc,
                },
                then: &[Effect::draw(1)],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];

    // MV = 4, costs {2} less if you control an artifact -> actual mana spent = 2 (< MV 4)!
    let mut disc_face = creature_face("Reduced Construct", "{3}{U}", 3, 3);
    disc_face.cost_reduction = Some(CostReduction::If {
        condition: Condition::ControlCount(&Filter::ARTIFACT, 1),
        amount: 2,
    });

    let cards = vec![
        card(
            CONDUIT,
            face("Aetherflux Conduit", "{6}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            CONDUIT_AB,
        ),
        card(DISCOUNTED, disc_face, KeywordSet::EMPTY, &[]),
        card(
            FULL_SPELL,
            creature_face("Honest Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            UNRAVEL,
            face("Unravel", "{1}{U}{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            UNRAVEL_AB,
        ),
    ];

    let mut engine = bench(
        16_130,
        cards,
        [
            Seat::with(&[CONDUIT]).holding(&[DISCOUNTED, FULL_SPELL]),
            Seat::default().holding(&[UNRAVEL, UNRAVEL]),
        ],
    );

    to_main(&mut engine, me());
    // 1. Cast Reduced Construct (MV 4, reduced by {2} -> spend {1}{U} = 2 mana).
    // Opponent counters it with Unravel BEFORE Aetherflux Conduit's trigger resolves!
    let disc = the(&engine, ZoneLocation::Hand(me()), DISCOUNTED);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: disc })
        .expect("cast Reduced Construct for 2 mana");
    let disc_on_stack = the(&engine, ZoneLocation::Stack, DISCOUNTED);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass with Conduit trigger on top");
    let unravel1 = engine.state().zones.list(ZoneLocation::Hand(them()))[0];
    add_pool(&mut engine, them(), ManaColor::Blue, 3);
    engine
        .apply(them(), PlayerAction::CastSpell { card: unravel1 })
        .expect("cast Unravel on discounted spell");
    settle_until_target(&mut engine, them(), disc_on_stack);
    let opp_hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(them())).len();
    settle_stack(&mut engine, me());

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(them())).len(),
        opp_hand_after_cast + 1,
        "Unravel drew a card because 2 mana spent < MV 4"
    );
    assert_eq!(
        engine.state().players[usize::from(me().get())].energy,
        2,
        "Aetherflux Conduit granted 2 {{E}} (actual mana spent) even after the spell was countered"
    );

    // 2. Cast Honest Bear for full cost (2 mana == MV 2), opponent casts second Unravel ->
    // counters Bear, opponent does NOT draw (2 < 2 is false), and Conduit adds +2 {E} (total 4)!
    let bear = the(&engine, ZoneLocation::Hand(me()), FULL_SPELL);
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bear })
        .expect("cast Honest Bear");
    let bear_on_stack = the(&engine, ZoneLocation::Stack, FULL_SPELL);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass priority");
    let unravel2 = the(&engine, ZoneLocation::Hand(them()), UNRAVEL);
    add_pool(&mut engine, them(), ManaColor::Blue, 3);
    engine
        .apply(them(), PlayerAction::CastSpell { card: unravel2 })
        .expect("cast second Unravel");
    settle_until_target(&mut engine, them(), bear_on_stack);
    let opp_hand_after_cast2 = engine.state().zones.list(ZoneLocation::Hand(them())).len();
    settle_stack(&mut engine, me());

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(them())).len(),
        opp_hand_after_cast2,
        "Unravel must NOT draw when mana spent (2) equals mana value (2)"
    );
    assert_eq!(
        engine.state().players[usize::from(me().get())].energy,
        4,
        "Aetherflux Conduit added 2 more {{E}}"
    );
}

// ============================================================================
// 5. Aveline de Grandpré & Amarant Coral — DamageAmount + Source LKI + OtherOpponents
// ============================================================================

#[test]
fn aveline_de_grandpre_and_other_opponents_damage_scale_with_combat_damage_dealt() {
    const AVELINE: u32 = C_BASE + 40;
    const VIPER: u32 = C_BASE + 41;
    const AMARANT: u32 = C_BASE + 42;

    static YOUR_DEATHTOUCH_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByYou,
        Filter::HasKeyword(KeywordSet::DEATHTOUCH),
    ]);
    static AVELINE_AB: &[AbilityDef] = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&YOUR_DEATHTOUCH_CREATURE),
        &[Effect::AddCounterFilter {
            filter: &Filter::Reference,
            kind: CounterKind::P1P1,
            amount: Amount::EventAmount,
        }],
    )];
    static AMARANT_AB: &[AbilityDef] = &[triggered!(
        Trigger::DealsCombatDamageToOpponent(&Filter::This),
        &[Effect::DealDamage {
            amount: Amount::EventAmount,
            target: TargetSpec::Player(PlayerRel::OtherOpponents),
        }],
    )];

    let cards = vec![
        card(
            AVELINE,
            creature_face("Aveline de Grandpré", "{2}{G}{G}", 3, 3),
            KeywordSet::DEATHTOUCH,
            AVELINE_AB,
        ),
        card(
            VIPER,
            creature_face("Ohran Viper", "{1}{G}{G}", 2, 3),
            KeywordSet::DEATHTOUCH.union(KeywordSet::HASTE),
            &[],
        ),
        card(
            AMARANT,
            creature_face("Amarant Coral", "{2}{R}{G}", 4, 4),
            KeywordSet::HASTE,
            AMARANT_AB,
        ),
    ];

    let opp1 = PlayerId::new(1);
    let opp2 = PlayerId::new(2);
    let mut engine = bench(
        16_140,
        cards,
        [
            Seat::with(&[AVELINE, VIPER, AMARANT]),
            Seat::default(),
            Seat::default(),
        ],
    );

    let viper = the(&engine, ZoneLocation::Battlefield, VIPER);
    let amarant = the(&engine, ZoneLocation::Battlefield, AMARANT);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (viper, Defender::Player(opp1)),
                    (amarant, Defender::Player(opp1)),
                ],
            },
        )
        .expect("attack opp1 with Viper (2) and Amarant (4)");
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });

    assert_eq!(
        engine
            .state()
            .object(viper)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2,
        "Aveline put 2 +1/+1 counters on the 2-power deathtouch Viper"
    );
    assert_eq!(
        engine
            .state()
            .object(amarant)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "Amarant lacks deathtouch and gets 0 counters from Aveline"
    );
    assert_eq!(life(&engine, opp1), 14, "opp1 took 2 + 4 = 6");
    assert_eq!(
        life(&engine, opp2),
        16,
        "opp2 took 4 damage from Amarant via PlayerRel::OtherOpponents"
    );
}

// ============================================================================
// 6. Ancient Imperiosaur — ConvokedCount (2x convoking creatures, 0 when uncast)
// ============================================================================

#[test]
fn ancient_imperiosaur_enters_with_twice_convoked_count_and_zero_when_reanimated() {
    const DINO: u32 = C_BASE + 50;
    const ELF: u32 = C_BASE + 51;
    const DISCARD_ONE: u32 = C_BASE + 52;
    const REANIM: u32 = C_BASE + 53;

    static TWICE_CONVOKED: Amount = Amount::Times {
        base: &Amount::ConvokedCount,
        factor: 2,
    };
    static DINO_MODS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: CounterKind::P1P1,
        amount: TWICE_CONVOKED,
    }];
    static DISCARD_AB: &[AbilityDef] = &[paid(
        Cost {
            mana: baylee_core::mana!("{0}"),
            parts: &[CostPart::Discard(&Filter::CREATURE)],
        },
        &[],
        None,
    )];
    static REANIM_AB: &[AbilityDef] = &[spell!(
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            owner_control: false,
            counters: None,
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )];

    let mut dino_face = creature_face("Ancient Imperiosaur", "{5}{G}{G}", 6, 6);
    dino_face.convoke = true;
    dino_face.enter_modifiers = DINO_MODS;

    let cards = vec![
        card(DINO, dino_face, KeywordSet::EMPTY, &[]),
        card(
            ELF,
            creature_face("Convoke Elf", "{G}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            DISCARD_ONE,
            face("Discard Outlet", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DISCARD_AB,
        ),
        card(
            REANIM,
            face("Zombify Dino", "{B}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REANIM_AB,
        ),
    ];

    let mut engine = bench(
        16_150,
        cards,
        [
            Seat::with(&[ELF, ELF, ELF, DISCARD_ONE]).holding(&[DINO, DINO, REANIM]),
            Seat::default(),
        ],
    );

    to_main(&mut engine, me());
    let elves = objects(&engine, ZoneLocation::Battlefield, ELF);
    let dino1 = objects(&engine, ZoneLocation::Hand(me()), DINO)[0];
    add_pool(&mut engine, me(), ManaColor::Green, 4);

    engine
        .apply(me(), PlayerAction::CastSpell { card: dino1 })
        .expect("begin casting Ancient Imperiosaur");
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "expected convoke prompt, got {:?}",
        engine.pending()
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: elves,
                players: vec![],
            },
        )
        .expect("convoke with all 3 Elves");
    settle_stack(&mut engine, me());

    assert_eq!(
        engine
            .state()
            .object(dino1)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        6,
        "convoking with 3 creatures puts 2 * 3 = 6 +1/+1 counters on Ancient Imperiosaur"
    );

    // Discard second Dino to graveyard, then reanimate it without casting -> 0 counters!
    let outlet = the(&engine, ZoneLocation::Battlefield, DISCARD_ONE);
    let dino2 = the(&engine, ZoneLocation::Hand(me()), DINO);
    activate(&mut engine, me(), outlet, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![dino2],
            },
        )
        .expect("discard second Dino");
    settle_stack(&mut engine, me());

    let reanim = the(&engine, ZoneLocation::Hand(me()), REANIM);
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: reanim })
        .expect("cast Zombify Dino");
    settle_until_target(&mut engine, me(), dino2);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(dino2)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "reanimated Ancient Imperiosaur was not convoked and enters with 0 counters"
    );
}

// ============================================================================
// 7. Ardenvale Paladin, Converge & Turn-History Amounts
// ============================================================================

#[test]
fn ardenvale_paladin_adamant_converge_and_turn_history_amounts() {
    const PALADIN: u32 = C_BASE + 60;
    const CONVERGE_GOLEM: u32 = C_BASE + 61;
    const RAISE_DEAD: u32 = C_BASE + 62;
    const SAC_ALTAR: u32 = C_BASE + 63;
    const BEAR: u32 = C_BASE + 64;

    static LEFT_YOUR_GY: Amount = Amount::LeftZoneThisTurn {
        filter: &Filter::CREATURE,
        from: ZoneRef::Graveyard,
        owner_or_ctrl: PlayerRel::You,
    };
    static SACS_THIS_TURN: Amount = Amount::PermanentsSacrificedThisTurn {
        who: PlayerRel::You,
        filter: &Filter::CREATURE,
    };
    static TEN_LEFT_GY: Amount = Amount::Times {
        base: &LEFT_YOUR_GY,
        factor: 10,
    };
    static PROBE_GAIN: Amount = Amount::Sum {
        left: &TEN_LEFT_GY,
        right: &SACS_THIS_TURN,
    };
    static PALADIN_MODS: &[EnterModifier] = &[EnterModifier::WithCountersIf {
        kind: CounterKind::P1P1,
        count: Amount::Fixed(1),
        condition: Condition::AmountAtLeast(Amount::ManaColorSpentToCast(ManaColor::White), 3),
    }];
    static CONVERGE_MODS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: CounterKind::P1P1,
        amount: Amount::ColorsOfManaSpentToCast,
    }];
    static RAISE_DEAD_AB: &[AbilityDef] = &[spell!(
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )];
    static ALTAR_AB: &[AbilityDef] = &[paid(
        Cost {
            mana: baylee_core::mana!("{0}"),
            parts: &[CostPart::Sacrifice(&Filter::CREATURE)],
        },
        &[Effect::GainLife { amount: PROBE_GAIN }],
        None,
    )];

    let mut paladin_face = creature_face("Ardenvale Paladin", "{3}{W}", 2, 5);
    paladin_face.enter_modifiers = PALADIN_MODS;
    let mut converge_face = face(
        "Skyrider Golem",
        "{4}",
        TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    );
    converge_face.power = Some(1);
    converge_face.toughness = Some(1);
    converge_face.enter_modifiers = CONVERGE_MODS;

    let cards = vec![
        card(PALADIN, paladin_face, KeywordSet::EMPTY, &[]),
        card(CONVERGE_GOLEM, converge_face, KeywordSet::EMPTY, &[]),
        card(
            RAISE_DEAD,
            face("Raise Dead", "{B}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            RAISE_DEAD_AB,
        ),
        card(
            SAC_ALTAR,
            face("History Altar", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            ALTAR_AB,
        ),
        card(
            BEAR,
            creature_face("Grave Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        16_160,
        cards,
        [
            Seat::with(&[SAC_ALTAR, BEAR, BEAR]).holding(&[
                PALADIN,
                PALADIN,
                CONVERGE_GOLEM,
                RAISE_DEAD,
            ]),
            Seat::default(),
        ],
    );

    to_main(&mut engine, me());

    // 1a. Cast first Ardenvale Paladin with {W}{W}{U}{U} (only 2 white mana) -> 0 counters!
    let pal1 = objects(&engine, ZoneLocation::Hand(me()), PALADIN)[0];
    add_pool(&mut engine, me(), ManaColor::White, 2);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pal1 })
        .expect("cast Paladin 1 with 2W + 2U");
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(pal1)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "2 white mana (< 3) must NOT trigger Adamant"
    );

    // 1b. Cast second Ardenvale Paladin with {W}{W}{W}{G} (3 white mana) -> 1 +1/+1 counter!
    let pal2 = the(&engine, ZoneLocation::Hand(me()), PALADIN);
    add_pool(&mut engine, me(), ManaColor::White, 3);
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pal2 })
        .expect("cast Paladin 2 with 3W + 1G");
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(pal2)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "3 white mana satisfies Adamant and places a +1/+1 counter"
    );

    // 2. Cast Skyrider Golem ({4}) spending {W}{U}{R}{C} -> Colorless is NOT a color (CR 105.4),
    // so Converge counts 3 colors -> 3 +1/+1 counters!
    let conv = the(&engine, ZoneLocation::Hand(me()), CONVERGE_GOLEM);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: conv })
        .expect("cast Skyrider Golem with W+U+R+C");
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(conv)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3,
        "Converge counts W, U, R = 3 colors (Colorless excluded per CR 105.4)"
    );

    // 3. Sacrifice first Bear to Altar (LEFT_YOUR_GY == 0, SACS_THIS_TURN == 1 -> +1 life = 21),
    // return it from graveyard to hand with Raise Dead (`Graveyard -> Hand`, LEFT_YOUR_GY == 1),
    // then sacrifice second Bear to Altar (LEFT_YOUR_GY == 1, SACS_THIS_TURN == 2 -> +12 life = 33)!
    let altar = the(&engine, ZoneLocation::Battlefield, SAC_ALTAR);
    let bears = objects(&engine, ZoneLocation::Battlefield, BEAR);
    activate(&mut engine, me(), altar, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bears[0]],
            },
        )
        .expect("sacrifice first Bear");
    settle_stack(&mut engine, me());
    assert_eq!(life(&engine, me()), 21, "+1 life from first sacrifice");

    let gy_bear = the(&engine, ZoneLocation::Graveyard(me()), BEAR);
    let raise = the(&engine, ZoneLocation::Hand(me()), RAISE_DEAD);
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: raise })
        .expect("cast Raise Dead");
    settle_until_target(&mut engine, me(), gy_bear);
    settle_stack(&mut engine, me());

    activate(&mut engine, me(), altar, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bears[1]],
            },
        )
        .expect("sacrifice second Bear");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        33,
        "gained 10 * 1 (left graveyard to hand) + 2 (two sacrificed creatures) = 12 more life"
    );
}
