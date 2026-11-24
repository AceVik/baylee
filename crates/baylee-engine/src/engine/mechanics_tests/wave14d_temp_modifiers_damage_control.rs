//! Beat 18 / Engine Wave 14d deterministic rule tests:
//! 1. Temporary continuous modifiers (`Effect::AddTempModifier`,
//!    `Effect::AddGlobalTempModifier`, `Duration::UntilEndOfTurn`,
//!    `Duration::UntilYourNextTurn`, `Duration::WhileSourceOnBattlefield`) and
//!    player/combat modifiers (`Modifier::PlayerCantCast`,
//!    `Modifier::PlayerCantActivateNonMana`, `Modifier::SpellsCantBeCountered`,
//!    `Modifier::MustBlock`).
//! 2. Damage halving (`DamageAmountReplacement::{HalfDown, HalfUp}`,
//!    `Modifier::ReplaceDamage`) and prevention/replacement-with-rider
//!    (`Effect::PreventNextDamageWithThen`,
//!    `Modifier::ReplaceOrPreventDamageWithThen`, `ThisWay::DamagePrevented`).
//! 3. Generalized `GainControl` (`Effect::GainControlEx`,
//!    `Effect::GainControlAllEx`, `Duration::Permanent`,
//!    `PlayerRel::EventSourceController`, `PlayerRel::EventPlayer`,
//!    `PlayerRel::DamagedPlayer`, `untap`, `haste_until_eot`, multiplayer
//!    recipient choice).

#![allow(clippy::too_many_lines)] // comprehensive multi-case rule tests

use super::*;
use crate::choice::AnswerFault;
use crate::turn::Step;
use baylee_cards_dsl::counters;
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, DamageAmountReplacement, DamageRecipients, DamageRider, Duration, Filter, Modifier,
    PlayerRel, TargetSpec, ThisWay, Trigger,
};
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_940;
const C1: u32 = 99_941;
const C2: u32 = 99_942;
const C3: u32 = 99_943;
const C4: u32 = 99_944;
const C5: u32 = 99_945;
const C6: u32 = 99_946;
const C7: u32 = 99_947;

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

fn to_attackers(engine: &mut Bench, active: PlayerId) {
    walk_until(engine, |e| {
        e.state().turn.active == active
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == active)
    });
}

fn to_blockers(engine: &mut Bench, defender: PlayerId) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == defender),
    );
}

fn next_turn_main(engine: &mut Bench, seat: PlayerId) {
    let start_turn = engine.state().turn.number;
    walk_until(engine, |e| {
        e.state().turn.number > start_turn
            && holds_priority_in(e, seat, Step::Main)
            && e.state().turn.phase == Phase::FirstMain
    });
}

fn counters_on(engine: &Bench, id: ObjectId, kind: baylee_cards_dsl::CounterKind) -> u16 {
    engine.state().object(id).unwrap().counters.get(kind)
}

fn declare_attackers(engine: &mut Bench, active: PlayerId, attackers: Vec<(ObjectId, Defender)>) {
    engine
        .apply(active, PlayerAction::DeclareAttackers { attackers })
        .expect("attackers declared");
}

fn declare_blockers(engine: &mut Bench, defender: PlayerId, blockers: Vec<(ObjectId, ObjectId)>) {
    engine
        .apply(defender, PlayerAction::DeclareBlockers { blockers })
        .expect("blockers declared");
}

#[test]
fn abeyance_locks_targeted_player_instants_sorceries_and_non_mana_abilities() {
    static ABEYANCE_EFFECTS: &[Effect] = &[
        Effect::AddTempModifier {
            target: TargetSpec::AnyPlayer,
            modifier: Modifier::PlayerCantCast {
                who: PlayerRel::EachPlayer,
                filter: &Filter::INSTANT_OR_SORCERY,
            },
            duration: Duration::UntilEndOfTurn,
        },
        Effect::AddTempModifier {
            target: TargetSpec::AnyPlayer,
            modifier: Modifier::PlayerCantActivateNonMana {
                who: PlayerRel::EachPlayer,
            },
            duration: Duration::UntilEndOfTurn,
        },
    ];
    static ABEYANCE_AB: &[AbilityDef] = &[spell!(
        ABEYANCE_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];
    static INSTANT_PING: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];
    static NON_MANA_PING: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }];
    static MANA_ABILITY: &[Effect] = &[Effect::mana(ManaColor::Green, 1)];
    static ACTIVATED_DORK_AB: &[AbilityDef] = &[
        free(NON_MANA_PING, None),
        AbilityDef::Activated {
            cost: Cost::FREE,
            effects: MANA_ABILITY,
            targets: None,
            second_targets: None,
            third_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
    ];
    static CYCLER_AB: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost::FREE,
        effects: &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Hand,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];
    static WALKER_AB: &[AbilityDef] = &[AbilityDef::Loyalty {
        cost: 1,
        effects: NON_MANA_PING,
        targets: None,
        second_targets: None,
        third_targets: None,
    }];

    let abeyance = card(
        C0,
        face("Abeyance Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ABEYANCE_AB,
    );
    let opt_instant = card(
        C1,
        face("Opt Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        INSTANT_PING,
    );
    let flash_bear = card(
        C2,
        creature_face("Flash Bear", "{0}", 2, 2),
        KeywordSet::FLASH,
        &[],
    );
    let dork = card(
        C3,
        creature_face("Dual Dork", "{0}", 1, 1),
        KeywordSet::EMPTY,
        ACTIVATED_DORK_AB,
    );
    let cycler = card(
        C4,
        face("Cycle Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CYCLER_AB,
    );
    let mut walker_face = face("Test Walker", "{0}", TypeSet::PLANESWALKER);
    walker_face.loyalty = Some(3);
    let walker = card(C5, walker_face, KeywordSet::EMPTY, WALKER_AB);

    let mut engine = bench(
        1401,
        vec![abeyance, opt_instant, flash_bear, dork, cycler, walker],
        [
            Seat::with(&[C3]).holding(&[C0, C1]),
            Seat::with(&[C3, C5]).holding(&[C1, C2, C4]),
        ],
    );
    to_main(&mut engine, me());

    // Advance to opponent's main phase so opponent can also test loyalty activation.
    next_turn_main(&mut engine, them());
    // Opponent passes priority so `me()` can cast Abeyance targeting `them()`.
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes");

    let abeyance_id = the(&engine, ZoneLocation::Hand(me()), C0);
    cast_spell(&mut engine, me(), abeyance_id);
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .expect("player target accepted");
    settle(&mut engine, them());

    // Under Abeyance, `them()` cannot cast the instant, cannot activate non-mana
    // battlefield/hand/loyalty abilities, but CAN cast the creature and CAN use mana abilities.
    let them_instant = the(&engine, ZoneLocation::Hand(them()), C1);
    let them_bear = the(&engine, ZoneLocation::Hand(them()), C2);
    let them_cycler = the(&engine, ZoneLocation::Hand(them()), C4);
    let them_dork = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == them() && o.card.is_some_and(|c| c.index.get() == C3)
            })
        })
        .expect("them dork");
    let them_walker = the(&engine, ZoneLocation::Battlefield, C5);

    let (p, legal) = offer(&engine);
    assert_eq!(p, them());
    assert!(
        !legal.castable.contains(&them_instant),
        "instant must be forbidden under Abeyance"
    );
    assert!(
        legal.castable.contains(&them_bear),
        "creature spell must still be legal under Abeyance"
    );
    assert!(
        !legal.abilities.contains(&(them_dork, 0)),
        "non-mana battlefield activation must be forbidden"
    );
    assert!(
        legal.abilities.contains(&(them_dork, 1)),
        "mana activation must remain legal"
    );
    assert!(
        !legal.abilities.contains(&(them_cycler, 0)),
        "in-hand non-mana activation (cycling) must be forbidden"
    );
    assert!(
        !legal.abilities.contains(&(them_walker, 0)),
        "planeswalker loyalty activation must be forbidden"
    );
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::ActivateAbility {
                    source: them_dork,
                    ability_index: 0,
                },
            )
            .is_err(),
        "direct non-mana activation attempt must be rejected"
    );
    // Mana ability still succeeds.
    activate(&mut engine, them(), them_dork, 1);
    assert_eq!(
        engine.state().players[usize::from(them().get())]
            .mana_pool
            .total(),
        1
    );

    // `me()` is NOT locked by Abeyance targeting `them()`.
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes");
    let me_instant = the(&engine, ZoneLocation::Hand(me()), C1);
    let me_dork = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == me() && o.card.is_some_and(|c| c.index.get() == C3)
            })
        })
        .expect("me dork");
    let (p2, legal2) = offer(&engine);
    assert_eq!(p2, me());
    assert!(legal2.castable.contains(&me_instant));
    assert!(legal2.abilities.contains(&(me_dork, 0)));

    // After cleanup, next turn on `them()`'s turn, Abeyance has expired.
    next_turn_main(&mut engine, me());
    next_turn_main(&mut engine, them());
    let (_, legal_after) = offer(&engine);
    assert!(legal_after.castable.contains(&them_instant));
    assert!(legal_after.abilities.contains(&(them_dork, 0)));
    assert!(legal_after.abilities.contains(&(them_walker, 0)));
}

#[test]
fn temp_modifier_until_your_next_turn_and_while_source_on_battlefield_cr611_2b() {
    static UNTIL_NEXT_TURN_SILENCE: &[Effect] = &[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::PlayerCantCast {
            who: PlayerRel::EachOpponent,
            filter: &Filter::Any,
        },
        duration: Duration::UntilYourNextTurn,
    }];
    static SILENCE_AB: &[AbilityDef] = &[spell!(UNTIL_NEXT_TURN_SILENCE)];
    static SOURCE_LOCK_EFFECTS: &[Effect] = &[Effect::AddTempModifier {
        target: TargetSpec::Player(PlayerRel::EachOpponent),
        modifier: Modifier::PlayerCantActivateNonMana {
            who: PlayerRel::EachPlayer,
        },
        duration: Duration::WhileSourceOnBattlefield,
    }];
    static LOCKSMITH_AB: &[AbilityDef] = &[
        free(SOURCE_LOCK_EFFECTS, None),
        free(&[Effect::SacrificeSelf], None),
    ];
    static PING_AB: &[AbilityDef] = &[free(
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    )];
    static INSTANT_GAIN_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let silence = card(
        C0,
        face("Next-Turn Silence", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        SILENCE_AB,
    );
    let instant_card = card(
        C1,
        face("Instant Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        INSTANT_GAIN_AB,
    );
    let locksmith = card(
        C2,
        creature_face("Locksmith", "{0}", 2, 2),
        KeywordSet::EMPTY,
        LOCKSMITH_AB,
    );
    let pinger = card(
        C3,
        creature_face("Pinger", "{0}", 1, 1),
        KeywordSet::EMPTY,
        PING_AB,
    );

    let mut engine = bench(
        1402,
        vec![silence, instant_card, locksmith, pinger],
        [
            Seat::with(&[C2]).holding(&[C0]),
            Seat::with(&[C3]).holding(&[C1]),
        ],
    );
    to_main(&mut engine, me());

    // 1. Cast `UntilYourNextTurn` Silence on `me()`'s turn 1.
    let silence_id = the(&engine, ZoneLocation::Hand(me()), C0);
    cast_spell(&mut engine, me(), silence_id);
    settle(&mut engine, me());

    // Opponent still cannot cast on opponent's turn 2.
    next_turn_main(&mut engine, them());
    let them_inst = the(&engine, ZoneLocation::Hand(them()), C1);
    let (_, legal_t2) = offer(&engine);
    assert!(
        !legal_t2.castable.contains(&them_inst),
        "UntilYourNextTurn must persist through opponent's turn"
    );

    // Once `me()`'s next turn begins, it expires and opponent can cast in response.
    next_turn_main(&mut engine, me());
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let (p_t3, legal_t3) = offer(&engine);
    assert_eq!(p_t3, them());
    assert!(
        legal_t3.castable.contains(&them_inst),
        "UntilYourNextTurn must expire when controller's next turn starts"
    );
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes");

    // 2. Test `WhileSourceOnBattlefield`: activate Locksmith, verify lock holds,
    //    then sacrifice Locksmith and verify immediate CR 400.7 cleanup.
    to_main(&mut engine, me());
    let locksmith_id = the(&engine, ZoneLocation::Battlefield, C2);
    let pinger_id = the(&engine, ZoneLocation::Battlefield, C3);
    activate(&mut engine, me(), locksmith_id, 0);
    settle(&mut engine, me());

    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let (_, locked_legal) = offer(&engine);
    assert!(
        !locked_legal.abilities.contains(&(pinger_id, 0)),
        "WhileSourceOnBattlefield lock must hold while source is on battlefield"
    );
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes");

    // Sacrifice Locksmith on the next main phase; lock ends immediately on LTB.
    to_main(&mut engine, me());
    activate(&mut engine, me(), locksmith_id, 1);
    settle(&mut engine, me());
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let (_, unlocked_legal) = offer(&engine);
    assert!(
        unlocked_legal.abilities.contains(&(pinger_id, 0)),
        "WhileSourceOnBattlefield lock must end immediately when source leaves"
    );

    // 3. CR 611.2b: start a fresh bench where Locksmith sacrifices itself
    //    in response to its own ability BEFORE the ability resolves.
    let mut engine2 = bench(
        1403,
        vec![locksmith, pinger],
        [Seat::with(&[C2]), Seat::with(&[C3])],
    );
    to_main(&mut engine2, me());
    let lock2 = the(&engine2, ZoneLocation::Battlefield, C2);
    let ping2 = the(&engine2, ZoneLocation::Battlefield, C3);
    activate(&mut engine2, me(), lock2, 0);
    activate(&mut engine2, me(), lock2, 1);
    settle(&mut engine2, me());

    engine2
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let (_, legal_after_cr611) = offer(&engine2);
    assert!(
        legal_after_cr611.abilities.contains(&(ping2, 0)),
        "CR 611.2b: WhileSourceOnBattlefield effect must not start if source left before resolution"
    );
}

#[test]
fn academic_dispute_must_block_enforces_at_least_one_block_without_overforcing() {
    static DISPUTE_EFFECTS: &[Effect] = &[Effect::AddTempModifier {
        target: TargetSpec::Object(&Filter::CREATURE),
        modifier: Modifier::MustBlock,
        duration: Duration::UntilEndOfTurn,
    }];
    static DISPUTE_AB: &[AbilityDef] = &[spell!(
        DISPUTE_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static EXTRA_BLOCK_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::CanBlockAdditional(1)
    )];

    let dispute = card(
        C0,
        face("Academic Dispute Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DISPUTE_AB,
    );
    let attacker_a = card(
        C1,
        creature_face("Attacker A", "{0}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let attacker_b = card(
        C2,
        creature_face("Attacker B", "{0}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let guard = card(
        C3,
        creature_face("Multi Guard", "{0}", 1, 5),
        KeywordSet::EMPTY,
        EXTRA_BLOCK_AB,
    );

    let mut engine = bench(
        1404,
        vec![dispute, attacker_a, attacker_b, guard],
        [Seat::with(&[C1, C2]).holding(&[C0]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());

    let dispute_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let atk_a = the(&engine, ZoneLocation::Battlefield, C1);
    let atk_b = the(&engine, ZoneLocation::Battlefield, C2);
    let guard_id = the(&engine, ZoneLocation::Battlefield, C3);

    cast_spell(&mut engine, me(), dispute_id);
    aim_object(&mut engine, me(), guard_id);
    settle(&mut engine, me());

    // Attack with both attackers.
    to_attackers(&mut engine, me());
    declare_attackers(
        &mut engine,
        me(),
        vec![
            (atk_a, Defender::Player(them())),
            (atk_b, Defender::Player(them())),
        ],
    );

    to_blockers(&mut engine, them());
    // Empty blocks must be rejected with AnswerFault::MustBlock.
    let err = engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect_err("must block at least one attacker");
    assert_eq!(
        err.to_string(),
        EngineError::from(AnswerFault::MustBlock).to_string()
    );

    // Blocking just ONE of the two attackers satisfies CR 509.1c even though
    // Multi Guard has CanBlockAdditional(1).
    declare_blockers(&mut engine, them(), vec![(guard_id, atk_a)]);

    // Next turn of `me()`, MustBlock has expired so declining to block is legal.
    next_turn_main(&mut engine, them());
    next_turn_main(&mut engine, me());
    to_attackers(&mut engine, me());
    declare_attackers(&mut engine, me(), vec![(atk_a, Defender::Player(them()))]);
    to_blockers(&mut engine, them());
    declare_blockers(&mut engine, them(), vec![]);
}

#[test]
fn spells_cant_be_countered_protects_matching_spells_only_not_abilities() {
    static VEIL_EFFECTS: &[Effect] = &[Effect::AddGlobalTempModifier {
        filter: &Filter::Any,
        modifier: Modifier::SpellsCantBeCountered {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        },
        duration: Duration::UntilEndOfTurn,
    }];
    static VEIL_AB: &[AbilityDef] = &[spell!(VEIL_EFFECTS)];
    static COUNTER_SPELL_AB: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    static COUNTER_EXILE_AB: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpellToExile],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    static COUNTER_ANY_AB: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpellOrAbility],
        targets = Some(TargetReq::one(TargetSpec::SpellOrAbility(&Filter::Any)))
    )];
    static SORCERY_GAIN_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(5),
    }])];
    static CREATURE_PING_AB: &[AbilityDef] = &[free(
        &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }],
        None,
    )];
    static SERPOPARD_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::SpellsCantBeCountered {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        }
    )];

    let veil = card(
        C0,
        face("Veil Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        VEIL_AB,
    );
    let bear = card(
        C1,
        creature_face("Protected Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        CREATURE_PING_AB,
    );
    let sorcery = card(
        C2,
        face("Unprotected Sorcery", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        SORCERY_GAIN_AB,
    );
    let counterspell = card(
        C3,
        face("Counterspell Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COUNTER_SPELL_AB,
    );
    let dissipate = card(
        C4,
        face("Dissipate Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COUNTER_EXILE_AB,
    );
    let disallow = card(
        C5,
        face("Disallow Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COUNTER_ANY_AB,
    );
    let serpopard = card(
        C6,
        creature_face("Serpopard Probe", "{0}", 4, 3),
        KeywordSet::EMPTY,
        SERPOPARD_AB,
    );

    let mut engine = bench(
        1405,
        vec![
            veil,
            bear,
            sorcery,
            counterspell,
            dissipate,
            disallow,
            serpopard,
        ],
        [
            Seat::default().holding(&[C0, C1, C2]),
            Seat::default().holding(&[C3, C4, C5, C3]),
        ],
    );
    to_main(&mut engine, me());

    // Resolve Veil Probe (your creature spells can't be countered this turn).
    let veil_id = the(&engine, ZoneLocation::Hand(me()), C0);
    cast_spell(&mut engine, me(), veil_id);
    settle(&mut engine, me());

    // Cast creature spell; opponent fires both CounterTargetSpell and CounterTargetSpellToExile.
    let bear_id = the(&engine, ZoneLocation::Hand(me()), C1);
    cast_spell(&mut engine, me(), bear_id);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");

    let cs_id = objects(&engine, ZoneLocation::Hand(them()), C3)[0];
    cast_spell(&mut engine, them(), cs_id);
    aim_object(&mut engine, them(), bear_id);

    let diss_id = the(&engine, ZoneLocation::Hand(them()), C4);
    cast_spell(&mut engine, them(), diss_id);
    aim_object(&mut engine, them(), bear_id);
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(bear_id).map(|o| o.zone),
        Some(Zone::Battlefield),
        "creature spell must survive CounterTargetSpell and CounterTargetSpellToExile"
    );

    // Non-creature sorcery is NOT matched by Filter::CREATURE and IS countered.
    let life_before = life(&engine, me());
    let sorcery_id = the(&engine, ZoneLocation::Hand(me()), C2);
    cast_spell(&mut engine, me(), sorcery_id);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let cs2_id = objects(&engine, ZoneLocation::Hand(them()), C3)[0];
    cast_spell(&mut engine, them(), cs2_id);
    aim_object(&mut engine, them(), sorcery_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sorcery_id).map(|o| o.zone),
        Some(Zone::Graveyard),
        "non-creature spell must still be countered"
    );
    assert_eq!(life(&engine, me()), life_before);

    // Activated ability of the creature IS an AbilityOnStack, so CounterTargetSpellOrAbility
    // MUST still counter it even though creature spells can't be countered!
    activate(&mut engine, me(), bear_id, 0);
    let ability_on_stack = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("ability on stack");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let disallow_id = the(&engine, ZoneLocation::Hand(them()), C5);
    cast_spell(&mut engine, them(), disallow_id);
    aim_object(&mut engine, them(), ability_on_stack);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        life_before,
        "SpellsCantBeCountered must NOT protect activated abilities on the stack"
    );

    // Static Serpopard protects creature spells while on battlefield.
    let mut engine2 = bench(
        1406,
        vec![bear, counterspell, serpopard],
        [
            Seat::with(&[C6]).holding(&[C1]),
            Seat::default().holding(&[C3]),
        ],
    );
    to_main(&mut engine2, me());
    let bear2 = the(&engine2, ZoneLocation::Hand(me()), C1);
    cast_spell(&mut engine2, me(), bear2);
    engine2
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let cs3 = the(&engine2, ZoneLocation::Hand(them()), C3);
    cast_spell(&mut engine2, them(), cs3);
    aim_object(&mut engine2, them(), bear2);
    settle(&mut engine2, me());
    assert_eq!(
        engine2.state().object(bear2).map(|o| o.zone),
        Some(Zone::Battlefield),
        "static SpellsCantBeCountered must protect creature spells"
    );
}

#[test]
fn ghosts_of_the_innocent_half_down_half_up_per_source_unpreventable_and_cr616_ordering() {
    static GHOSTS_DOWN_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ReplaceDamage {
            sources: &Filter::Any,
            recipients: DamageRecipients::Any,
            combat: None,
            replacement: DamageAmountReplacement::HalfDown,
        }
    )];
    static GHOSTS_UP_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ReplaceDamage {
            sources: &Filter::Any,
            recipients: DamageRecipients::Any,
            combat: None,
            replacement: DamageAmountReplacement::HalfUp,
        }
    )];
    static DOUBLER_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::MultiplyDamageFrom {
            sources: &Filter::Any,
            recipients: DamageRecipients::Any,
            factor: 2,
            combat_only: false,
            noncombat_only: false,
        }
    )];
    static BURNER_ALL_AB: &[AbilityDef] = &[
        free(
            &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(5),
            }],
            Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        free(
            &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(1),
            }],
            Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        free(
            &[Effect::DealDamageWithoutPreventionOrRedirection {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(5),
                recipients: &Filter::Any,
            }],
            Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        free(
            &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(3),
            }],
            Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ];

    let ghosts_down = card(
        C0,
        creature_face("Ghosts Down", "{0}", 4, 5),
        KeywordSet::EMPTY,
        GHOSTS_DOWN_AB,
    );
    let ghosts_up = card(
        C1,
        creature_face("Ghosts Up", "{0}", 4, 5),
        KeywordSet::EMPTY,
        GHOSTS_UP_AB,
    );
    let burner = card(
        C2,
        face("Burn Device", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        BURNER_ALL_AB,
    );
    let target_creature = card(
        C3,
        creature_face("3/3 Target", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let atk_four = card(
        C4,
        creature_face("4/4 Attacker", "{0}", 4, 4),
        KeywordSet::HASTE,
        &[],
    );
    let atk_five = card(
        C5,
        creature_face("5/5 Attacker", "{0}", 5, 5),
        KeywordSet::HASTE,
        &[],
    );
    let doubler = card(
        C6,
        face("Furnace Probe", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        DOUBLER_AB,
    );

    // 1. HalfDown on player (5 -> 2, 1 -> 0), creature (5 -> 2 survives),
    //    unpreventable (5 -> 2), and multi-source combat (4 + 5 -> 2 + 2 = 4).
    let mut engine = bench(
        1407,
        vec![ghosts_down, burner, target_creature, atk_four, atk_five],
        [
            Seat::with(&[C0, C2, C4, C5]).at(20),
            Seat::with(&[C3]).at(20),
        ],
    );
    to_main(&mut engine, me());
    let burner_id = the(&engine, ZoneLocation::Battlefield, C2);
    let creature_id = the(&engine, ZoneLocation::Battlefield, C3);
    let atk4 = the(&engine, ZoneLocation::Battlefield, C4);
    let atk5 = the(&engine, ZoneLocation::Battlefield, C5);

    // 5 damage to opponent -> floor(5/2) = 2.
    activate(&mut engine, me(), burner_id, 0);
    aim_player(&mut engine, me(), them());
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 18);

    // 1 damage to opponent -> floor(1/2) = 0.
    activate(&mut engine, me(), burner_id, 1);
    aim_player(&mut engine, me(), them());
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 18);

    // 5 damage to 3/3 creature -> 2 damage marked, creature survives.
    activate(&mut engine, me(), burner_id, 0);
    aim_object(&mut engine, me(), creature_id);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(creature_id)
            .map(|o| (o.zone, o.damage)),
        Some((Zone::Battlefield, 2))
    );

    // 5 unpreventable damage to opponent -> still halved to 2!
    activate(&mut engine, me(), burner_id, 2);
    aim_player(&mut engine, me(), them());
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 16);

    // Simultaneous 4/4 + 5/5 combat damage -> floor(4/2) + floor(5/2) = 2 + 2 = 4.
    to_attackers(&mut engine, me());
    declare_attackers(
        &mut engine,
        me(),
        vec![
            (atk4, Defender::Player(them())),
            (atk5, Defender::Player(them())),
        ],
    );
    to_blockers(&mut engine, them());
    declare_blockers(&mut engine, them(), vec![]);
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        life(&engine, them()),
        12,
        "each source halves independently: 16 - (2 + 2) = 12"
    );

    // 2. HalfUp: 5 -> 3, 1 -> 1.
    let mut engine_up = bench(
        1408,
        vec![ghosts_up, burner],
        [Seat::with(&[C1, C2]).at(20), Seat::default().at(20)],
    );
    to_main(&mut engine_up, me());
    let burner_up = the(&engine_up, ZoneLocation::Battlefield, C2);
    activate(&mut engine_up, me(), burner_up, 0);
    aim_player(&mut engine_up, me(), them());
    settle(&mut engine_up, me());
    assert_eq!(life(&engine_up, them()), 17, "HalfUp(5) == 3");
    activate(&mut engine_up, me(), burner_up, 1);
    aim_player(&mut engine_up, me(), them());
    settle(&mut engine_up, me());
    assert_eq!(life(&engine_up, them()), 16, "HalfUp(1) == 1");

    // 3. CR 616.1 ordering: HalfDown + MultiplyDamageFrom(2) on 3 damage to `them()`.
    //    `them()` chooses HalfDown first: floor(3/2)=1, *2 = 2 damage (life 18).
    let mut engine_ord = bench(
        1409,
        vec![ghosts_down, doubler, burner],
        [Seat::with(&[C0, C6, C2]).at(20), Seat::default().at(20)],
    );
    to_main(&mut engine_ord, me());
    let burner_ord = the(&engine_ord, ZoneLocation::Battlefield, C2);
    activate(&mut engine_ord, me(), burner_ord, 3);
    aim_player(&mut engine_ord, me(), them());
    walk_until(
        &mut engine_ord,
        |e| matches!(e.pending(), Pending::ChooseDamageEffect { player, .. } if *player == them()),
    );
    let Pending::ChooseDamageEffect {
        choice, options, ..
    } = engine_ord.pending().clone()
    else {
        panic!("expected ChooseDamageEffect");
    };
    let half_opt = options
        .iter()
        .find(|opt| {
            matches!(
                opt.kind,
                crate::choice::DamageEffectKind::ReplaceAmount { amount: 1 }
            )
        })
        .expect("HalfDown option (amount 1) offered");
    engine_ord
        .apply(
            them(),
            PlayerAction::ChooseDamageEffect {
                choice,
                effect: half_opt.id,
            },
        )
        .expect("chose HalfDown first");
    settle(&mut engine_ord, me());
    assert_eq!(
        life(&engine_ord, them()),
        18,
        "CR 616.1: HalfDown(3)=1 then Double(1)=2 -> 18 life"
    );
}

#[test]
fn awe_strike_prevent_next_damage_with_then_gains_life_once_and_hashes_shield() {
    static AWE_STRIKE_EFFECTS: &[Effect] = &[Effect::PreventNextDamageWithThen {
        sources: TargetSpec::Object(&Filter::CREATURE),
        recipients: TargetSpec::AnyTarget,
        combat_only: false,
        then: DamageRider::new(&[Effect::GainLife {
            amount: Amount::ThisWay(ThisWay::DamagePrevented),
        }]),
    }];
    static AWE_STRIKE_AB: &[AbilityDef] = &[spell!(
        AWE_STRIKE_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static PING_2_AB: &[AbilityDef] = &[
        free(
            &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EachOpponent),
                amount: Amount::Fixed(2),
            }],
            None,
        ),
        free(
            &[Effect::DealDamageWithoutPreventionOrRedirection {
                target: TargetSpec::Player(PlayerRel::EachOpponent),
                amount: Amount::Fixed(3),
                recipients: &Filter::Any,
            }],
            None,
        ),
    ];
    static NO_PREV_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::CombatDamageCantBePrevented
    )];

    let awe_strike = card(
        C0,
        face("Awe Strike Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        AWE_STRIKE_AB,
    );
    let bruiser = card(
        C1,
        creature_face("Bruiser", "{0}", 4, 4),
        KeywordSet::HASTE,
        PING_2_AB,
    );

    let mut engine = bench(
        1410,
        vec![awe_strike, bruiser],
        [
            Seat::default().holding(&[C0]).at(20),
            Seat::with(&[C1]).at(20),
        ],
    );
    to_main(&mut engine, me());
    next_turn_main(&mut engine, them());

    let bruiser_id = the(&engine, ZoneLocation::Battlefield, C1);
    let awe_id = the(&engine, ZoneLocation::Hand(me()), C0);

    // Opponent attacks with 4/4 Bruiser; in declare-attackers priority, `me()` casts Awe Strike.
    to_attackers(&mut engine, them());
    declare_attackers(
        &mut engine,
        them(),
        vec![(bruiser_id, Defender::Player(me()))],
    );
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes");

    let hash_before = engine.state().snapshot_hash();
    cast_spell(&mut engine, me(), awe_id);
    aim_object(&mut engine, me(), bruiser_id);
    settle(&mut engine, them());
    let hash_with_shield = engine.state().snapshot_hash();
    assert_ne!(
        hash_before, hash_with_shield,
        "ShieldKind::NextWithThen must contribute to snapshot_hash"
    );

    // Let combat damage happen unblocked: 4 prevented -> 0 damage taken, +4 life!
    to_blockers(&mut engine, me());
    declare_blockers(&mut engine, me(), vec![]);
    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        life(&engine, me()),
        24,
        "4 combat damage prevented -> 0 lost and +4 life gained"
    );

    // Second damage event from Bruiser is NOT prevented (shield was consumed).
    activate(&mut engine, them(), bruiser_id, 0);
    settle(&mut engine, them());
    assert_eq!(
        life(&engine, me()),
        22,
        "subsequent 2 damage must not be prevented after one-shot shield spent"
    );

    // Unpreventable combat damage ignores PreventNextDamageWithThen and grants 0 life.
    let no_prev = card(
        C2,
        face("Questing Curse", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        NO_PREV_AB,
    );
    let mut engine2 = bench(
        1411,
        vec![awe_strike, bruiser, no_prev],
        [
            Seat::default().holding(&[C0]).at(20),
            Seat::with(&[C1, C2]).at(20),
        ],
    );
    to_main(&mut engine2, me());
    next_turn_main(&mut engine2, them());
    let awe2 = the(&engine2, ZoneLocation::Hand(me()), C0);
    let bruiser2 = the(&engine2, ZoneLocation::Battlefield, C1);
    to_attackers(&mut engine2, them());
    declare_attackers(
        &mut engine2,
        them(),
        vec![(bruiser2, Defender::Player(me()))],
    );
    engine2
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes");
    cast_spell(&mut engine2, me(), awe2);
    aim_object(&mut engine2, me(), bruiser2);
    settle(&mut engine2, them());
    to_blockers(&mut engine2, me());
    declare_blockers(&mut engine2, me(), vec![]);
    walk_until(&mut engine2, |e| {
        holds_priority_in(e, them(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        life(&engine2, me()),
        16,
        "unpreventable 4 combat damage must bypass prevention shield and gain 0 life"
    );
}

#[test]
fn gloom_surgeon_and_force_bubble_prevention_vs_replacement_with_then() {
    static GLOOM_THEN: DamageRider = DamageRider::new(&[Effect::ExileTop {
        who: PlayerRel::You,
        count: Amount::ThisWay(ThisWay::DamagePrevented),
    }]);
    static GLOOM_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ReplaceOrPreventDamageWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Permanents(&Filter::This),
            combat: Some(true),
            is_replacement: false,
            then: GLOOM_THEN,
        }
    )];
    static FORCE_THEN: DamageRider = DamageRider::new(&[Effect::AddCounterSource {
        kind: counters::DEPLETION,
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
    }]);
    static FORCE_BUBBLE_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::Any,
            Modifier::ReplaceOrPreventDamageWithThen {
                sources: &Filter::Any,
                recipients: DamageRecipients::Players(PlayerRel::You),
                combat: None,
                is_replacement: true,
                then: FORCE_THEN,
            }
        ),
        triggered!(
            Trigger::CountersReach {
                kind: counters::DEPLETION,
                n: 4,
            },
            &[Effect::SacrificeSelf]
        ),
    ];
    static PING_1_AB: &[AbilityDef] = &[
        free(
            &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(1),
            }],
            Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        free(
            &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(3),
            }],
            Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        free(
            &[Effect::DealDamageWithoutPreventionOrRedirection {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(2),
                recipients: &Filter::Any,
            }],
            Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ];
    static CANT_PREVENT_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::CombatDamageCantBePrevented
    )];

    let gloom = card(
        C0,
        creature_face("Gloom Surgeon Probe", "{0}", 2, 1),
        KeywordSet::HASTE,
        GLOOM_AB,
    );
    let blocker = card(
        C1,
        creature_face("3/3 Blocker", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let force_bubble = card(
        C2,
        face("Force Bubble Probe", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        FORCE_BUBBLE_AB,
    );
    let pinger = card(
        C3,
        face("Ping Device", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PING_1_AB,
    );
    let skullcrack_enchant = card(
        C4,
        face("No Prevention", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        CANT_PREVENT_AB,
    );

    // 1. Gloom Surgeon: combat damage prevented + top 3 exiled; noncombat ping kills it.
    let mut engine = bench(
        1412,
        vec![gloom, blocker, pinger],
        [Seat::with(&[C0]), Seat::with(&[C1, C3])],
    );
    to_main(&mut engine, me());
    let gloom_id = the(&engine, ZoneLocation::Battlefield, C0);
    let blocker_id = the(&engine, ZoneLocation::Battlefield, C1);
    let pinger_id = the(&engine, ZoneLocation::Battlefield, C3);
    let exile_before = engine.state().zones.list(ZoneLocation::Exile(me())).len();

    to_attackers(&mut engine, me());
    declare_attackers(
        &mut engine,
        me(),
        vec![(gloom_id, Defender::Player(them()))],
    );
    to_blockers(&mut engine, them());
    declare_blockers(&mut engine, them(), vec![(blocker_id, gloom_id)]);
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        engine.state().object(gloom_id).map(|o| (o.zone, o.damage)),
        Some((Zone::Battlefield, 0)),
        "Gloom Surgeon prevents 3 combat damage and survives"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(me())).len(),
        exile_before + 3,
        "Gloom Surgeon exiles top 3 cards equal to prevented combat damage"
    );

    // Noncombat 1 damage kills Gloom Surgeon without exiling any cards.
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    activate(&mut engine, them(), pinger_id, 0);
    aim_object(&mut engine, them(), gloom_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(gloom_id).map(|o| o.zone),
        Some(Zone::Graveyard),
        "noncombat damage is not prevented by Gloom Surgeon"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(me())).len(),
        exile_before + 3
    );

    // Under CombatDamageCantBePrevented, Gloom Surgeon (is_replacement: false) cannot prevent combat damage.
    let mut engine_noprev = bench(
        1413,
        vec![gloom, blocker, skullcrack_enchant],
        [Seat::with(&[C0]), Seat::with(&[C1, C4])],
    );
    to_main(&mut engine_noprev, me());
    let gloom_np = the(&engine_noprev, ZoneLocation::Battlefield, C0);
    let blocker_np = the(&engine_noprev, ZoneLocation::Battlefield, C1);
    let exile_np_before = engine_noprev
        .state()
        .zones
        .list(ZoneLocation::Exile(me()))
        .len();
    to_attackers(&mut engine_noprev, me());
    declare_attackers(
        &mut engine_noprev,
        me(),
        vec![(gloom_np, Defender::Player(them()))],
    );
    to_blockers(&mut engine_noprev, them());
    declare_blockers(&mut engine_noprev, them(), vec![(blocker_np, gloom_np)]);
    walk_until(&mut engine_noprev, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        engine_noprev.state().object(gloom_np).map(|o| o.zone),
        Some(Zone::Graveyard),
        "prevention-mode Gloom Surgeon fails under CombatDamageCantBePrevented"
    );
    assert_eq!(
        engine_noprev
            .state()
            .zones
            .list(ZoneLocation::Exile(me()))
            .len(),
        exile_np_before
    );

    // 2. Force Bubble (is_replacement: true): replaces 3 normal damage AND 2 unpreventable
    //    combat damage -> reaches 5 depletion counters -> sacrifices via Trigger::CountersReach(4).
    let two_attacker = card(
        C5,
        creature_face("2/2 Attacker", "{0}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let mut engine_fb = bench(
        1414,
        vec![force_bubble, pinger, skullcrack_enchant, two_attacker],
        [Seat::with(&[C2]).at(20), Seat::with(&[C3, C4, C5]).at(20)],
    );
    to_main(&mut engine_fb, me());
    let fb_id = the(&engine_fb, ZoneLocation::Battlefield, C2);
    let pinger_fb = the(&engine_fb, ZoneLocation::Battlefield, C3);
    let atk_fb = the(&engine_fb, ZoneLocation::Battlefield, C5);

    engine_fb
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    activate(&mut engine_fb, them(), pinger_fb, 1); // 3 normal damage to me
    aim_player(&mut engine_fb, them(), me());
    settle(&mut engine_fb, me());
    assert_eq!(life(&engine_fb, me()), 20);
    assert_eq!(counters_on(&engine_fb, fb_id, counters::DEPLETION), 3);
    assert_eq!(
        engine_fb.state().object(fb_id).map(|o| o.zone),
        Some(Zone::Battlefield)
    );

    // Now deal 2 UNPREVENTABLE combat damage to `me()` (with CombatDamageCantBePrevented on board):
    // Force Bubble is a replacement effect (`is_replacement: true`), so it STILL replaces the
    // damage and triggers sacrifice at 5 >= 4!
    next_turn_main(&mut engine_fb, them());
    to_attackers(&mut engine_fb, them());
    declare_attackers(
        &mut engine_fb,
        them(),
        vec![(atk_fb, Defender::Player(me()))],
    );
    to_blockers(&mut engine_fb, me());
    declare_blockers(&mut engine_fb, me(), vec![]);
    walk_until(&mut engine_fb, |e| {
        holds_priority_in(e, them(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        life(&engine_fb, me()),
        20,
        "Force Bubble replacement must replace even unpreventable combat damage"
    );
    assert_eq!(
        engine_fb.state().object(fb_id).map(|o| o.zone),
        Some(Zone::Graveyard),
        "Force Bubble sacrifices itself at 4+ depletion counters"
    );
}

#[test]
fn gain_control_ex_charisma_flayer_contested_war_zone_fractured_loyalty_and_blim() {
    // 1. Charisma: WhileSourceOnBattlefield + immediate LTB return + CR 611.2b pre-resolution LTB.
    static CHARISMA_AB: &[AbilityDef] = &[
        free(
            &[Effect::GainControlEx {
                target: TargetSpec::Object(&Filter::CREATURE),
                to: PlayerRel::You,
                duration: Duration::WhileSourceOnBattlefield,
                untap: false,
                haste_until_eot: false,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(&[Effect::SacrificeSelf], None),
    ];
    // 2. Flayer of Loyalties: UntilEndOfTurn + untap + haste_until_eot.
    static FLAYER_AB: &[AbilityDef] = &[spell!(
        &[Effect::GainControlEx {
            target: TargetSpec::Object(&Filter::CREATURE),
            to: PlayerRel::You,
            duration: Duration::UntilEndOfTurn,
            untap: true,
            haste_until_eot: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    // 3. Contested War Zone: whenever a creature deals combat damage to you,
    //    that creature's controller permanently gains control of this.
    static CONTESTED_AB: &[AbilityDef] = &[triggered!(
        Trigger::DealsCombatDamageToYou(&Filter::CREATURE),
        &[Effect::GainControlEx {
            target: TargetSpec::ThisObject,
            to: PlayerRel::EventSourceController,
            duration: Duration::Permanent,
            untap: false,
            haste_until_eot: false,
        }]
    )];
    // 4. Blim, Comedic Genius: deals combat damage to a player -> that player
    //    permanently gains control of target permanent you control.
    static BLIM_AB: &[AbilityDef] = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::GainControlEx {
            target: TargetSpec::Object(&Filter::ControlledByYou),
            to: PlayerRel::DamagedPlayer,
            duration: Duration::Permanent,
            untap: false,
            haste_until_eot: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ControlledByYou)))
    )];
    // 5. Fractured Loyalty: whenever this becomes the target of a spell or ability,
    //    that spell or ability's controller gains control of it permanently.
    static FRACTURED_AB: &[AbilityDef] = &[triggered!(
        Trigger::BecomesTarget,
        &[Effect::GainControlEx {
            target: TargetSpec::ThisObject,
            to: PlayerRel::EventPlayer,
            duration: Duration::Permanent,
            untap: false,
            haste_until_eot: false,
        }]
    )];
    static TARGET_PROBE_AB: &[AbilityDef] = &[free(
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let charisma = card(
        C0,
        face("Charisma Probe", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        CHARISMA_AB,
    );
    let victim = card(
        C1,
        creature_face("Victim Bear", "{0}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let flayer = card(
        C2,
        face("Flayer Spell", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        FLAYER_AB,
    );
    let war_zone = card(
        C3,
        face("Contested War Zone Probe", "{0}", TypeSet::LAND),
        KeywordSet::EMPTY,
        CONTESTED_AB,
    );
    let blim = card(
        C4,
        creature_face("Blim Probe", "{0}", 4, 3),
        KeywordSet::HASTE,
        BLIM_AB,
    );
    let gift = card(
        C5,
        face("Cursed Gift", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let fractured = card(
        C6,
        creature_face("Fractured Creature", "{0}", 2, 2),
        KeywordSet::EMPTY,
        FRACTURED_AB,
    );
    let targeter = card(
        C7,
        face("Target Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TARGET_PROBE_AB,
    );

    // 1. Charisma steals victim while on battlefield; sacrificing Charisma returns it;
    //    sacrificing Charisma before resolution starts no control change.
    let mut eng1 = bench(
        1415,
        vec![charisma, victim],
        [Seat::with(&[C0]), Seat::with(&[C1])],
    );
    to_main(&mut eng1, me());
    let ch_id = the(&eng1, ZoneLocation::Battlefield, C0);
    let vic_id = the(&eng1, ZoneLocation::Battlefield, C1);
    activate(&mut eng1, me(), ch_id, 0);
    aim_object(&mut eng1, me(), vic_id);
    settle(&mut eng1, me());
    assert_eq!(eng1.state().object(vic_id).unwrap().controller, me());

    activate(&mut eng1, me(), ch_id, 1);
    settle(&mut eng1, me());
    assert_eq!(
        eng1.state().object(vic_id).unwrap().controller,
        them(),
        "destroying Charisma must immediately return control"
    );

    // CR 611.2b: sacrifice Charisma in response to its own theft ability.
    let mut eng1b = bench(
        1416,
        vec![charisma, victim],
        [Seat::with(&[C0]), Seat::with(&[C1])],
    );
    to_main(&mut eng1b, me());
    let ch2 = the(&eng1b, ZoneLocation::Battlefield, C0);
    let vic2 = the(&eng1b, ZoneLocation::Battlefield, C1);
    activate(&mut eng1b, me(), ch2, 0);
    aim_object(&mut eng1b, me(), vic2);
    activate(&mut eng1b, me(), ch2, 1);
    settle(&mut eng1b, me());
    assert_eq!(
        eng1b.state().object(vic2).unwrap().controller,
        them(),
        "CR 611.2b: Charisma leaving before resolution must never transfer control"
    );

    // 2. Flayer: steals tapped creature, untaps it, grants haste so it can attack, returns at EOT.
    let mut eng2 = bench(
        1417,
        vec![flayer, victim],
        [Seat::default().holding(&[C2]), Seat::with(&[C1])],
    );
    to_main(&mut eng2, me());
    let vic3 = the(&eng2, ZoneLocation::Battlefield, C1);
    let flayer_id = the(&eng2, ZoneLocation::Hand(me()), C2);
    eng2.state.set_tapped(vic3, true);
    assert!(
        eng2.state()
            .object(vic3)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );

    cast_spell(&mut eng2, me(), flayer_id);
    aim_object(&mut eng2, me(), vic3);
    settle(&mut eng2, me());
    let stolen = eng2.state().object(vic3).unwrap();
    assert_eq!(stolen.controller, me());
    assert!(!stolen.status.contains(crate::object::Status::TAPPED));
    assert!(
        stolen
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE)
    );

    to_attackers(&mut eng2, me());
    declare_attackers(&mut eng2, me(), vec![(vic3, Defender::Player(them()))]);
    to_blockers(&mut eng2, them());
    declare_blockers(&mut eng2, them(), vec![]);
    next_turn_main(&mut eng2, them());
    let returned = eng2.state().object(vic3).unwrap();
    assert_eq!(returned.controller, them(), "control returns at EOT");
    assert!(
        !returned
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE)
    );

    // 3. Contested War Zone (EventSourceController + Permanent) + Blim (DamagedPlayer + Permanent).
    let mut eng3 = bench(
        1418,
        vec![war_zone, blim, gift],
        [Seat::with(&[C4, C5]), Seat::with(&[C3])],
    );
    to_main(&mut eng3, me());
    let blim_id = the(&eng3, ZoneLocation::Battlefield, C4);
    let gift_id = the(&eng3, ZoneLocation::Battlefield, C5);
    let wz_id = the(&eng3, ZoneLocation::Battlefield, C3);

    to_attackers(&mut eng3, me());
    declare_attackers(&mut eng3, me(), vec![(blim_id, Defender::Player(them()))]);
    to_blockers(&mut eng3, them());
    declare_blockers(&mut eng3, them(), vec![]);
    // Blim's trigger asks `me()` for target permanent you control -> choose Cursed Gift.
    walk_until(
        &mut eng3,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == me()),
    );
    aim_object(&mut eng3, me(), gift_id);
    walk_until(&mut eng3, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        eng3.state().object(wz_id).unwrap().controller,
        me(),
        "Contested War Zone transfers to attacker's controller (EventSourceController)"
    );
    assert_eq!(
        eng3.state().object(gift_id).unwrap().controller,
        them(),
        "Blim transfers Cursed Gift to DamagedPlayer"
    );
    // Permanent duration persists onto next turn.
    next_turn_main(&mut eng3, them());
    assert_eq!(eng3.state().object(wz_id).unwrap().controller, me());
    assert_eq!(eng3.state().object(gift_id).unwrap().controller, them());

    // 4. Fractured Loyalty (EventPlayer + Permanent).
    let mut eng4 = bench(
        1419,
        vec![fractured, targeter],
        [Seat::with(&[C6]), Seat::with(&[C7])],
    );
    to_main(&mut eng4, me());
    let frac_id = the(&eng4, ZoneLocation::Battlefield, C6);
    let targ_id = the(&eng4, ZoneLocation::Battlefield, C7);
    eng4.apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    activate(&mut eng4, them(), targ_id, 0);
    aim_object(&mut eng4, them(), frac_id);
    settle(&mut eng4, me());
    assert_eq!(
        eng4.state().object(frac_id).unwrap().controller,
        them(),
        "Fractured Loyalty transfers permanent control to targeting EventPlayer"
    );
    next_turn_main(&mut eng4, them());
    assert_eq!(eng4.state().object(frac_id).unwrap().controller, them());
}

#[test]
fn gain_control_all_ex_temporary_permanent_guard_and_multiplayer_recipient() {
    static BROADCAST_AB: &[AbilityDef] = &[spell!(&[Effect::GainControlAllEx {
        filter: &Filter::CREATURE,
        to: PlayerRel::You,
        duration: Duration::UntilEndOfTurn,
        untap: true,
        haste_until_eot: true,
    }])];
    static ASHIOK_AB: &[AbilityDef] = &[spell!(&[Effect::GainControlAllEx {
        filter: &Filter::CREATURE,
        to: PlayerRel::You,
        duration: Duration::Permanent,
        untap: false,
        haste_until_eot: false,
    }])];
    static HOMEWARD_GUARD_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::OtherPlayersCantGainControl
    )];
    static DONATE_OPPONENT_AB: &[AbilityDef] = &[spell!(
        &[Effect::GainControlEx {
            target: TargetSpec::Object(&Filter::CREATURE),
            to: PlayerRel::Opponent,
            duration: Duration::Permanent,
            untap: false,
            haste_until_eot: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let broadcast = card(
        C0,
        face("Broadcast Takeover Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BROADCAST_AB,
    );
    let ashiok = card(
        C1,
        face("Ashiok Ultimate Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        ASHIOK_AB,
    );
    let bear_a = card(
        C2,
        creature_face("Opponent Bear A", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let guarded_bear = card(
        C3,
        creature_face("Guarded Bear", "{0}", 3, 3),
        KeywordSet::EMPTY,
        HOMEWARD_GUARD_AB,
    );
    let donate = card(
        C4,
        face("Multiplayer Donate", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        DONATE_OPPONENT_AB,
    );

    // 1. Broadcast Takeover: steals unguarded tapped creature, untaps + haste, respects guard,
    //    and returns control at cleanup.
    let mut eng1 = bench(
        1420,
        vec![broadcast, bear_a, guarded_bear],
        [Seat::default().holding(&[C0]), Seat::with(&[C2, C3])],
    );
    to_main(&mut eng1, me());
    let bc_id = the(&eng1, ZoneLocation::Hand(me()), C0);
    let bear_id = the(&eng1, ZoneLocation::Battlefield, C2);
    let guard_id = the(&eng1, ZoneLocation::Battlefield, C3);
    eng1.state.set_tapped(bear_id, true);

    cast_spell(&mut eng1, me(), bc_id);
    settle(&mut eng1, me());

    assert_eq!(eng1.state().object(bear_id).unwrap().controller, me());
    assert!(
        !eng1
            .state()
            .object(bear_id)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert!(
        eng1.state()
            .object(bear_id)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE)
    );
    assert_eq!(
        eng1.state().object(guard_id).unwrap().controller,
        them(),
        "OtherPlayersCantGainControl must keep Guarded Bear under opponent's control"
    );

    next_turn_main(&mut eng1, them());
    assert_eq!(
        eng1.state().object(bear_id).unwrap().controller,
        them(),
        "UntilEndOfTurn GainControlAllEx must return control at cleanup"
    );

    // 2. Ashiok Permanent GainControlAllEx persists across turns.
    let mut eng2 = bench(
        1421,
        vec![ashiok, bear_a],
        [Seat::default().holding(&[C1]), Seat::with(&[C2])],
    );
    to_main(&mut eng2, me());
    let ash_id = the(&eng2, ZoneLocation::Hand(me()), C1);
    let bear2 = the(&eng2, ZoneLocation::Battlefield, C2);
    cast_spell(&mut eng2, me(), ash_id);
    settle(&mut eng2, me());
    assert_eq!(eng2.state().object(bear2).unwrap().controller, me());
    next_turn_main(&mut eng2, them());
    assert_eq!(
        eng2.state().object(bear2).unwrap().controller,
        me(),
        "Permanent GainControlAllEx must persist onto next turn"
    );

    // 3. 3-player game: `to: PlayerRel::Opponent` pauses on Pending::ChoosePlayer
    //    via AwaitingOp::GainControlExRecipient and transfers to the chosen opponent.
    let seat2 = PlayerId::new(2);
    let mut eng3 = bench(
        1422,
        vec![donate, bear_a],
        [
            Seat::with(&[C2]).holding(&[C4]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut eng3, me());
    let donate_id = the(&eng3, ZoneLocation::Hand(me()), C4);
    let bear3 = the(&eng3, ZoneLocation::Battlefield, C2);
    cast_spell(&mut eng3, me(), donate_id);
    aim_object(&mut eng3, me(), bear3);
    walk_until(
        &mut eng3,
        |e| matches!(e.pending(), Pending::ChoosePlayer { player, .. } if *player == me()),
    );
    let Pending::ChoosePlayer { options, .. } = eng3.pending().clone() else {
        panic!("expected ChoosePlayer");
    };
    assert_eq!(options, vec![them(), seat2]);
    eng3.apply(me(), PlayerAction::ChoosePlayer(seat2))
        .expect("recipient chosen");
    settle(&mut eng3, me());
    assert_eq!(
        eng3.state().object(bear3).unwrap().controller,
        seat2,
        "multiplayer GainControlEx must transfer control to chosen opponent"
    );
}
