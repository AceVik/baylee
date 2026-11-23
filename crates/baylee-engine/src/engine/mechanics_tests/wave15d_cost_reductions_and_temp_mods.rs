//! Engine Wave 15d deterministic integration tests:
//! 1. `Modifier::ActivatedAbilitiesCostLess` (CR 118.7a, CR 602.2b, CR 702.177a):
//!    - *Agatha of the Vile Cauldron*: dynamic `Amount::SourcePower` generic
//!      reduction on controlled creatures' activated abilities, `min_mana: 1`
//!      floor ("can't reduce the mana in that cost to less than one mana"),
//!      dynamic power changes (pump +3/+3), noncreature & opponent exclusion,
//!      `unpaid_abilities` preview, actual activation payment, and `{X}` upper
//!      bound credit.
//!    - *Boom Scholar*: `exhaust_only: true` reduces exhaust abilities only,
//!      respects exhaust once-per-incarnation spend, and composes after floored
//!      reductions per CR 118.7a.
//! 2. `Modifier::FirstSpellEachTurnCostAdjust` & temporary until-EOT spell cost
//!    reductions (CR 601.2f, CR 118.7):
//!    - *Artificer Class*: first artifact spell discounted (even after casting a
//!      non-artifact spell earlier that turn!), second artifact spell normal,
//!      opponent unaffected, resets on next turn, and entering after an artifact
//!      was already cast does not grant a second discount.
//!    - Until-EOT temporary spell cost reduction (`Effect::ReduceNextSpellCost`
//!      with `next_only: false` and `Effect::AddGlobalTempModifier` with
//!      `Modifier::SpellsCostLess`): applies to multiple matching spells in the
//!      same turn and expires at cleanup.
//! 3. Combat & temporary effect modifiers:
//!    - *Alluring Siren* (`Modifier::MustAttackPlayer(PlayerRel::You)` via
//!      `AddTempModifier`): enforces attacking the player (rejecting empty attack
//!      AND rejecting attacking the player's planeswalker!), publishes `required`
//!      and `limits` on `Pending::ChooseAttackers`, skips tapped creatures, and
//!      expires at cleanup.
//!    - *Bone Dancer* (`Modifier::AssignsNoCombatDamage` via `AddTempModifier`):
//!      unblocked and blocked attacker assigns 0 combat damage while blocker
//!      still deals normal combat damage; expires at cleanup.
//!    - *Blitzball Stadium* (`Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)` and
//!      `Modifier::CantBeBlockedBy(&Filter::Any)` via `AddTempModifier`):
//!      prevents blocking this turn and expires at cleanup.
//! 4. `ExilePermission::for_player(PlayerRel::OwnerOfTarget)` /
//!    `PlayPermission` owner grants (CR 108.3, CR 609.4):
//!    - *Aerial Extortionist*: owner of exiled card may cast it from exile even
//!      after Aerial Extortionist leaves and even when stolen before exile;
//!      controller of Aerial Extortionist cannot cast it.
//! 5. `Filter::Saddled` (CR 702.171b):
//!    - Mount with `saddle!(2)` gains `Rider::Saddled` via `Effect::BecomeSaddled`,
//!      immediately satisfies `Filter::Saddled` in static Layer 7c buffs and
//!      targeting, changes `snapshot_hash`, and clears at cleanup.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use crate::object::Rider;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{
    Amount, Duration, Effect, ExilePermission, Filter, Modifier, PlayWindow, PlayerRel, TargetReq,
    TargetSpec, activated, cost, mana, saddle, spell, static_ability,
};
use baylee_core::generated::subtypes;
use baylee_core::ids::Defender;
use baylee_core::mana::{ManaColor, ManaPool};

const C0: u32 = 99_970;
const C1: u32 = 99_971;
const C2: u32 = 99_972;
const C3: u32 = 99_973;
const C4: u32 = 99_974;
const C5: u32 = 99_975;

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

fn planeswalker_face(name: &'static str, loyalty: u16) -> FaceDef {
    FaceDef {
        loyalty: Some(loyalty),
        ..face(name, "{3}", TypeSet::PLANESWALKER)
    }
}

fn to_attackers(engine: &mut Bench, active: PlayerId) {
    walk_until(engine, |e| {
        e.state().turn.active == active && matches!(e.pending(), Pending::ChooseAttackers { .. })
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
            && holds_priority_in(e, seat, crate::turn::Step::Main)
            && e.state().turn.phase == Phase::FirstMain
    });
}

// =========================================================================
// 1. Agatha of the Vile Cauldron: power-based reduction + 1-mana floor + X
// =========================================================================

static AGATHA_ABILITIES: &[AbilityDef] = &[
    static_ability!(
        Filter::Any,
        Modifier::ActivatedAbilitiesCostLess {
            filter: &Filter::YOUR_CREATURE,
            amount: Amount::SourcePower,
            min_mana: 1,
            exhaust_only: false,
        }
    ),
    // Free self-pump (+3/+3 until EOT) to verify dynamic power tracking.
    free(
        &[Effect::AddTempModifier {
            target: TargetSpec::ThisObject,
            modifier: Modifier::ModifyPT(3, 3),
            duration: Duration::UntilEndOfTurn,
        }],
        None,
    ),
];

static PING_3R_ABILITIES: &[AbilityDef] = &[activated!(
    cost!("{3}{R}"),
    &[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::Player(PlayerRel::Opponent),
    }]
)];

static GENERIC_3_ABILITIES: &[AbilityDef] = &[
    activated!(
        cost!("{3}"),
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }]
    ),
    activated!(
        cost!("{1}"),
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }]
    ),
];

static X_ABILITY: &[AbilityDef] = &[activated!(
    cost!("{X}{2}"),
    &[Effect::GainLife { amount: Amount::X }]
)];

#[test]
fn wave15d_agatha_power_based_activation_reduction_and_min_mana_floor() {
    let agatha = card(
        C0,
        creature_face("Agatha of the Vile Cauldron", "{R}{G}", 1, 1),
        KeywordSet::EMPTY,
        AGATHA_ABILITIES,
    );
    let pinger = card(
        C1,
        creature_face("Flame Channeler", "{1}{R}", 2, 2),
        KeywordSet::EMPTY,
        PING_3R_ABILITIES,
    );
    let generic_cre = card(
        C2,
        creature_face("Generic Seer", "{2}", 2, 2),
        KeywordSet::EMPTY,
        GENERIC_3_ABILITIES,
    );
    let noncreature_art = card(
        C3,
        face("Relic", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        GENERIC_3_ABILITIES,
    );
    let opp_cre = card(
        C4,
        creature_face("Enemy Seer", "{2}", 2, 2),
        KeywordSet::EMPTY,
        GENERIC_3_ABILITIES,
    );
    let x_cre = card(
        C5,
        creature_face("X Healer", "{1}{W}", 1, 1),
        KeywordSet::EMPTY,
        X_ABILITY,
    );

    let mut engine = bench(
        15401,
        vec![agatha, pinger, generic_cre, noncreature_art, opp_cre, x_cre],
        [Seat::with(&[C0, C1, C2, C3, C5]), Seat::with(&[C4])],
    );
    to_main(&mut engine, me());

    let agatha_id = the(&engine, ZoneLocation::Battlefield, C0);
    let pinger_id = the(&engine, ZoneLocation::Battlefield, C1);
    let gen_id = the(&engine, ZoneLocation::Battlefield, C2);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C3);
    let x_id = the(&engine, ZoneLocation::Battlefield, C5);

    // At Agatha power 1: {3}{R} -> {2}{R}; {3} -> {2}; {1} -> {1} (1-mana floor);
    // noncreature artifact {3} -> still {3} (filter YOUR_CREATURE).
    let (_, legal) = offer(&engine);
    assert!(
        legal
            .unpaid_abilities
            .contains(&(pinger_id, 0, mana!("{2}{R}"))),
        "Agatha power 1 reduces {{3}}{{R}} to {{2}}{{R}}: {:?}",
        legal.unpaid_abilities
    );
    assert!(
        legal.unpaid_abilities.contains(&(gen_id, 0, mana!("{2}"))),
        "Agatha power 1 reduces {{3}} to {{2}}"
    );
    assert!(
        legal.unpaid_abilities.contains(&(gen_id, 1, mana!("{1}"))),
        "Agatha's min_mana=1 floor keeps {{1}} at {{1}}"
    );
    assert!(
        legal
            .unpaid_abilities
            .contains(&(relic_id, 0, mana!("{3}"))),
        "noncreature artifact is NOT reduced by YOUR_CREATURE Agatha"
    );

    // Pump Agatha +3/+3 -> power 4!
    activate(&mut engine, me(), agatha_id, 1);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(agatha_id)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );

    // At Agatha power 4:
    // - {3}{R} reduces by 3 (total mana floor 1 -> {R}!).
    // - {3} reduces by 2 (total mana floor 1 -> {1}!, NEVER {0}!).
    // - {1} stays {1} (cannot reduce below 1 mana!).
    let (_, legal2) = offer(&engine);
    assert!(
        legal2
            .unpaid_abilities
            .contains(&(pinger_id, 0, mana!("{R}"))),
        "Agatha power 4 reduces {{3}}{{R}} to {{R}}: {:?}",
        legal2.unpaid_abilities
    );
    assert!(
        legal2.unpaid_abilities.contains(&(gen_id, 0, mana!("{1}"))),
        "Agatha power 4 reduces {{3}} to {{1}} (min_mana=1 floor): {:?}",
        legal2.unpaid_abilities
    );
    assert!(
        !legal2.abilities.contains(&(gen_id, 0)),
        "{{3}} must NOT become free under min_mana=1 floor"
    );

    // Float {R} and activate Flame Channeler's {3}{R} ability -> spends {R}, deals 2 damage!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    activate(&mut engine, me(), pinger_id, 0);
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(life(&engine, them()), 18);

    // Float {1} and activate Generic Seer's {3} ability -> spends {1}, gains 3 life!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    activate(&mut engine, me(), gen_id, 0);
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(life(&engine, me()), 23);

    // Test X ability `{X}{2}` with Agatha power 4 and {2} floating:
    // Total raw cost for X=4 is {6}, reduced by 4 -> {2} (affordable!).
    // Total raw cost for X=5 is {7}, reduced by 4 -> {3} (unaffordable with {2}).
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    activate(&mut engine, me(), x_id, 0);
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber for X, got {:?}", engine.pending());
    };
    assert_eq!(min, 0);
    assert_eq!(
        max, 4,
        "with {{2}} floating and Agatha power 4, {{X}}{{2}} allows X up to 4 (raw {{6}} - 4 = {{2}})"
    );
    engine
        .apply(me(), PlayerAction::ChooseNumber(4))
        .expect("choose X=4");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(life(&engine, me()), 27);

    // Verify opponent's creature is NOT reduced on opponent's turn.
    to_main(&mut engine, them());
    let opp_id = the(&engine, ZoneLocation::Battlefield, C4);
    let (_, opp_legal) = offer(&engine);
    assert!(
        opp_legal
            .unpaid_abilities
            .contains(&(opp_id, 0, mana!("{3}"))),
        "opponent's creature is NOT reduced by my Agatha"
    );
}

// =========================================================================
// 2. Boom Scholar: exhaust_only reduction & CR 118.7a composition after floor
// =========================================================================

static BOOM_SCHOLAR_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::Any,
    Modifier::ActivatedAbilitiesCostLess {
        filter: &Filter::ControlledByYou,
        amount: Amount::Fixed(1),
        min_mana: 0,
        exhaust_only: true,
    }
)];

static EXHAUST_AND_NORMAL_ABILITIES: &[AbilityDef] = &[
    // Ability 0: Exhaust — {2}: Gain 4 life.
    AbilityDef::Activated {
        cost: cost!("{2}"),
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Exhaust,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    },
    // Ability 1: Unlimited — {2}: Gain 1 life.
    activated!(
        cost!("{2}"),
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }]
    ),
    // Ability 2: Exhaust — {3}: Gain 5 life (for Agatha + Boom Scholar composition).
    AbilityDef::Activated {
        cost: cost!("{3}"),
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(5),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Exhaust,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    },
];

#[test]
fn wave15d_boom_scholar_exhaust_only_reduction_and_composition_after_floor() {
    let scholar = card(
        C0,
        creature_face("Boom Scholar", "{1}{R}{G}", 3, 3),
        KeywordSet::EMPTY,
        BOOM_SCHOLAR_ABILITIES,
    );
    let racer = card(
        C1,
        creature_face("Exhaust Racer", "{1}{R}", 2, 2),
        KeywordSet::EMPTY,
        EXHAUST_AND_NORMAL_ABILITIES,
    );
    let agatha2 = card(
        C2,
        creature_face("Agatha Two", "{R}{G}", 2, 2),
        KeywordSet::EMPTY,
        AGATHA_ABILITIES,
    );

    // Subcase A: Boom Scholar alone reduces Exhaust {2} -> {1}, leaves normal {2} at {2}.
    let mut engine = bench(
        15402,
        vec![scholar, racer],
        [Seat::with(&[C0, C1]), Seat::default()],
    );
    to_main(&mut engine, me());
    let racer_id = the(&engine, ZoneLocation::Battlefield, C1);

    let (_, legal) = offer(&engine);
    assert!(
        legal
            .unpaid_abilities
            .contains(&(racer_id, 0, mana!("{1}"))),
        "Boom Scholar reduces Exhaust {{2}} to {{1}}: {:?}",
        legal.unpaid_abilities
    );
    assert!(
        legal
            .unpaid_abilities
            .contains(&(racer_id, 1, mana!("{2}"))),
        "Boom Scholar must NOT reduce non-exhaust {{2}}: {:?}",
        legal.unpaid_abilities
    );

    // Pay {1} to activate Exhaust ability 0 -> spends {1}, gains 4 life, exhausts ability 0.
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    activate(&mut engine, me(), racer_id, 0);
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(life(&engine, me()), 24);
    let (_, after) = offer(&engine);
    assert!(
        !after.abilities.contains(&(racer_id, 0))
            && !after
                .unpaid_abilities
                .iter()
                .any(|&(s, i, _)| s == racer_id && i == 0),
        "exhaust ability cannot be activated a second time on the same incarnation"
    );

    // Subcase B: Agatha (power 2, min_mana=1) + Boom Scholar (amount 1, min_mana=0, exhaust_only)
    // on Exhaust {3} -> Agatha reduces {3} to {1} (floor 1), THEN Boom Scholar reduces {1} to {0}!
    let mut engine2 = bench(
        15403,
        vec![
            card(
                C0,
                creature_face("Boom Scholar", "{1}{R}{G}", 3, 3),
                KeywordSet::EMPTY,
                BOOM_SCHOLAR_ABILITIES,
            ),
            card(
                C1,
                creature_face("Exhaust Racer", "{1}{R}", 2, 2),
                KeywordSet::EMPTY,
                EXHAUST_AND_NORMAL_ABILITIES,
            ),
            agatha2,
        ],
        [Seat::with(&[C0, C1, C2]), Seat::default()],
    );
    to_main(&mut engine2, me());
    let racer2 = the(&engine2, ZoneLocation::Battlefield, C1);
    let (_, legal2) = offer(&engine2);
    assert!(
        legal2.abilities.contains(&(racer2, 2)),
        "CR 118.7a: Agatha (floor 1) takes {{3}} -> {{1}}, then Boom Scholar (floor 0) takes {{1}} -> {{0}}"
    );
    activate(&mut engine2, me(), racer2, 2);
    settle(&mut engine2, me());
    assert_eq!(life(&engine2, me()), 25);
}

// =========================================================================
// 3. Artificer Class: 1st artifact spell discounted, 2nd normal, turn reset
// =========================================================================

static ARTIFICER_CLASS_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::Any,
    Modifier::FirstSpellEachTurnCostAdjust {
        who: PlayerRel::You,
        filter: &Filter::ARTIFACT,
        cost: mana!("{1}"),
        increase: false,
        colored_only: false,
    }
)];

#[test]
fn wave15d_artificer_class_first_artifact_spell_discounted_second_normal() {
    let artificer = card(
        C0,
        face("Artificer Class", "{1}{U}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ARTIFICER_CLASS_ABILITIES,
    );
    let art_a = card(
        C1,
        face("Ornithopter Mk2 A", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let art_b = card(
        C2,
        face("Ornithopter Mk2 B", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let art_c = card(
        C3,
        face("Ornithopter Mk2 C", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let non_art = card(
        C4,
        creature_face("Plain Bear", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_art = card(
        C5,
        face("Enemy Relic", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        15404,
        vec![artificer, art_a, art_b, art_c, non_art, opp_art],
        [
            Seat::with(&[C0]).holding(&[C4, C1, C2, C3]),
            Seat::default().holding(&[C5]),
        ],
    );
    to_main(&mut engine, me());

    let bear_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let art_a_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let art_b_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let art_c_id = the(&engine, ZoneLocation::Hand(me()), C3);

    // Cast a NON-artifact spell first ({1} Bear). Must pay full {1}.
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal0) = offer(&engine);
    assert_eq!(legal0.spell_reduction(bear_id, CastModeKind::Normal), 0);
    assert_eq!(legal0.spell_reduction(art_a_id, CastModeKind::Normal), 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bear_id })
        .expect("cast non-artifact bear");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // Now cast FIRST artifact spell this turn: STILL gets {1} discount ({2} -> {1})!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal1) = offer(&engine);
    assert_eq!(
        legal1.spell_reduction(art_a_id, CastModeKind::Normal),
        1,
        "casting a non-artifact earlier does not consume the first-artifact-spell discount"
    );
    assert!(legal1.castable.contains(&art_a_id));
    engine
        .apply(me(), PlayerAction::CastSpell { card: art_a_id })
        .expect("cast 1st artifact for {1}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // SECOND artifact spell this turn: NO discount ({2}, cannot cast with {1})!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal2) = offer(&engine);
    assert_eq!(
        legal2.spell_reduction(art_b_id, CastModeKind::Normal),
        0,
        "2nd artifact spell this turn must get NO discount"
    );
    assert!(!legal2.castable.contains(&art_b_id));
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: art_b_id })
        .expect("cast 2nd artifact for full {2}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // Opponent's turn: opponent's artifact spell gets NO discount from our Artificer Class.
    to_main(&mut engine, them());
    let opp_art_id = the(&engine, ZoneLocation::Hand(them()), C5);
    let (_, opp_legal) = offer(&engine);
    assert_eq!(
        opp_legal.spell_reduction(opp_art_id, CastModeKind::Normal),
        0
    );

    // Back to our next turn: per-turn cast history reset -> 3rd artifact gets {1} discount again!
    next_turn_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal3) = offer(&engine);
    assert_eq!(
        legal3.spell_reduction(art_c_id, CastModeKind::Normal),
        1,
        "first-artifact-spell discount resets on the next turn"
    );
    assert!(legal3.castable.contains(&art_c_id));
    engine
        .apply(me(), PlayerAction::CastSpell { card: art_c_id })
        .expect("cast artifact on next turn for {1}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // Subcase B (CR 611.3): If an artifact was cast BEFORE Artificer Class entered
    // the battlefield on the same turn, the next artifact is the SECOND artifact
    // cast that turn and must NOT be discounted.
    let mut engine2 = bench(
        15405,
        vec![
            card(
                C0,
                face("Artificer Class", "{1}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                ARTIFICER_CLASS_ABILITIES,
            ),
            card(
                C1,
                face("Early Relic", "{1}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                C2,
                face("Later Relic", "{2}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::default().holding(&[C0, C1, C2]), Seat::default()],
    );
    to_main(&mut engine2, me());
    let early = the(&engine2, ZoneLocation::Hand(me()), C1);
    let class = the(&engine2, ZoneLocation::Hand(me()), C0);
    let later = the(&engine2, ZoneLocation::Hand(me()), C2);
    add_pool(&mut engine2, me(), ManaColor::Colorless, 2);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: early })
        .expect("cast 1st artifact before Artificer Class");
    settle(&mut engine2, me());
    engine2
        .apply(me(), PlayerAction::CastSpell { card: class })
        .expect("cast Artificer Class");
    settle(&mut engine2, me());
    let (_, after_class) = offer(&engine2);
    assert_eq!(
        after_class.spell_reduction(later, CastModeKind::Normal),
        0,
        "artifact cast earlier this turn before Artificer Class entered already spent the first-artifact slot"
    );
}

// =========================================================================
// 4. Temporary until-EOT spell cost reductions (multiple spells + cleanup)
// =========================================================================

static TEMP_REDUCE_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::ReduceNextSpellCost {
            who: PlayerRel::You,
            filter: &Filter::INSTANT_OR_SORCERY,
            generic: Amount::Fixed(1),
            mana: ManaCost::ZERO,
            colored_only: false,
            next_only: false,
            duration: Duration::UntilEndOfTurn,
        }],
        None,
    ),
    free(
        &[Effect::AddGlobalTempModifier {
            filter: &Filter::YOUR_CREATURE,
            modifier: Modifier::SpellsCostLess(1),
            duration: Duration::UntilEndOfTurn,
        }],
        None,
    ),
];

static NOOP_SPELL: &[AbilityDef] = &[spell!(&[Effect::GainLife {
    amount: Amount::Fixed(1),
}])];

#[test]
fn wave15d_temp_spell_cost_reduction_this_turn_applies_to_multiple_spells_and_expires() {
    let reducer = card(
        C0,
        face("Temp Reducer", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TEMP_REDUCE_ABILITIES,
    );
    let sorc1 = card(
        C1,
        face("Sorc 1", "{2}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        NOOP_SPELL,
    );
    let sorc2 = card(
        C2,
        face("Sorc 2", "{2}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        NOOP_SPELL,
    );
    let sorc3 = card(
        C3,
        face("Sorc 3", "{2}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        NOOP_SPELL,
    );
    let cre1 = card(
        C4,
        creature_face("Cre 1", "{2}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        15406,
        vec![reducer, sorc1, sorc2, sorc3, cre1],
        [
            Seat::with(&[C0]).holding(&[C1, C2, C3, C4]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let red_id = the(&engine, ZoneLocation::Battlefield, C0);
    let s1 = the(&engine, ZoneLocation::Hand(me()), C1);
    let s2 = the(&engine, ZoneLocation::Hand(me()), C2);
    let s3 = the(&engine, ZoneLocation::Hand(me()), C3);
    let c1 = the(&engine, ZoneLocation::Hand(me()), C4);

    // Activate until-EOT instant/sorcery reduction (next_only: false).
    activate(&mut engine, me(), red_id, 0);
    settle(&mut engine, me());

    // Creature is NOT discounted; BOTH Sorc 1 and Sorc 2 ARE discounted to {1}!
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_reduction(c1, CastModeKind::Normal), 0);
    assert_eq!(legal.spell_reduction(s1, CastModeKind::Normal), 1);
    assert_eq!(legal.spell_reduction(s2, CastModeKind::Normal), 1);

    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s1 })
        .expect("cast 1st sorcery for {1}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s2 })
        .expect("cast 2nd sorcery for {1} under until-EOT reduction");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // Activate AddGlobalTempModifier creature reduction -> Cre 1 gets {1} off.
    activate(&mut engine, me(), red_id, 1);
    settle(&mut engine, me());
    let (_, legal_c) = offer(&engine);
    assert_eq!(legal_c.spell_reduction(c1, CastModeKind::Normal), 1);

    // On next turn after cleanup, both until-EOT reductions have expired!
    next_turn_main(&mut engine, me());
    let (_, legal_next) = offer(&engine);
    assert_eq!(legal_next.spell_reduction(s3, CastModeKind::Normal), 0);
    assert_eq!(legal_next.spell_reduction(c1, CastModeKind::Normal), 0);
}

// =========================================================================
// 5. Alluring Siren: MustAttackPlayer(PlayerRel::You) legality & expiry
// =========================================================================

static ALLURING_SIREN_ABILITIES: &[AbilityDef] = &[activated!(
    cost!(TapSelf),
    &[Effect::AddTempModifier {
        target: TargetSpec::Object(&Filter::OPPONENT_CREATURE),
        modifier: Modifier::MustAttackPlayer(PlayerRel::You),
        duration: Duration::UntilEndOfTurn,
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(
        &Filter::OPPONENT_CREATURE
    )))
)];

#[test]
fn wave15d_alluring_siren_must_attack_player_combat_legality_and_planeswalker_restriction() {
    let siren = card(
        C0,
        creature_face("Alluring Siren", "{1}{U}", 1, 1),
        KeywordSet::EMPTY,
        ALLURING_SIREN_ABILITIES,
    );
    let jace = card(C1, planeswalker_face("Jace", 4), KeywordSet::EMPTY, &[]);
    let forced_bear = card(
        C2,
        creature_face("Forced Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let free_bear = card(
        C3,
        creature_face("Free Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        15407,
        vec![siren, jace, forced_bear, free_bear],
        [Seat::with(&[C0, C1]), Seat::with(&[C2, C3])],
    );

    // Advance to P1's first main phase; P1 passes priority to P0 so P0 can activate Siren.
    to_main(&mut engine, them());
    let siren_id = the(&engine, ZoneLocation::Battlefield, C0);
    let jace_id = the(&engine, ZoneLocation::Battlefield, C1);
    let forced_id = the(&engine, ZoneLocation::Battlefield, C2);

    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("P1 passes priority in main 1");
    activate(&mut engine, me(), siren_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![forced_id],
                players: vec![],
            },
        )
        .expect("Siren targets P1's Forced Bear");
    settle(&mut engine, them());

    // Enter P1's Declare Attackers step.
    to_attackers(&mut engine, them());
    let Pending::ChooseAttackers {
        required,
        minimum_required,
        limits,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseAttackers, got {:?}", engine.pending());
    };
    assert!(defenders.contains(&Defender::Player(me())));
    assert!(defenders.contains(&Defender::Planeswalker(jace_id)));
    assert_eq!(required, vec![forced_id]);
    assert_eq!(minimum_required, Some(1));
    let forced_limit = limits
        .iter()
        .find(|l| l.creature == forced_id)
        .expect("MustAttackPlayer(You) restricts forced creature to attacking P0 (not Jace)");
    assert_eq!(forced_limit.defenders, vec![Defender::Player(me())]);

    // Declaring empty attackers is rejected!
    assert!(
        engine
            .apply(them(), PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "skipping Siren-forced attacker must be rejected"
    );
    // Declaring forced_bear attacking P0's planeswalker Jace is ALSO rejected!
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(forced_id, Defender::Planeswalker(jace_id))],
                },
            )
            .is_err(),
        "attacking P0's planeswalker does NOT satisfy 'attacks you this turn if able'"
    );
    // Declaring forced_bear attacking Defender::Player(me()) is accepted!
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(forced_id, Defender::Player(me()))],
            },
        )
        .expect("attacking P0 directly satisfies MustAttackPlayer(You)");

    // Next turn of P1: Alluring Siren's until-EOT effect expired, so empty attack is legal!
    next_turn_main(&mut engine, them());
    to_attackers(&mut engine, them());
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(required.is_empty(), "MustAttackPlayer must expire at EOT");
    engine
        .apply(them(), PlayerAction::DeclareAttackers { attackers: vec![] })
        .expect("empty attack legal on next turn");
}

// =========================================================================
// 6. Bone Dancer (AssignsNoCombatDamage) & Blitzball Stadium (Unblockable)
// =========================================================================

static BONE_DANCER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::AddTempModifier {
        target: TargetSpec::ThisObject,
        modifier: Modifier::AssignsNoCombatDamage,
        duration: Duration::UntilEndOfTurn,
    }],
    None,
)];

static BLITZBALL_STADIUM_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::AddTempModifier {
            target: TargetSpec::Object(&Filter::CREATURE),
            modifier: Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
            duration: Duration::UntilEndOfTurn,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
    free(
        &[Effect::AddTempModifier {
            target: TargetSpec::Object(&Filter::CREATURE),
            modifier: Modifier::CantBeBlockedBy(&Filter::Any),
            duration: Duration::UntilEndOfTurn,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
];

#[test]
fn wave15d_bone_dancer_assigns_no_combat_damage_and_blitzball_stadium_cant_be_blocked() {
    let dancer = card(
        C0,
        creature_face("Bone Dancer", "{1}{B}{B}", 2, 2),
        KeywordSet::EMPTY,
        BONE_DANCER_ABILITIES,
    );
    let stadium = card(
        C1,
        face("Blitzball Stadium", "{0}", TypeSet::LAND),
        KeywordSet::EMPTY,
        BLITZBALL_STADIUM_ABILITIES,
    );
    let striker = card(
        C2,
        creature_face("Striker", "{2}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let blocker = card(
        C3,
        creature_face("Blocker", "{1}{W}", 1, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        15408,
        vec![dancer, stadium, striker, blocker],
        [Seat::with(&[C0, C1, C2]), Seat::with(&[C3])],
    );
    // Walk to P0's turn 3 (so creatures have no summoning sickness).
    next_turn_main(&mut engine, me());
    let dancer_id = the(&engine, ZoneLocation::Battlefield, C0);
    let stadium_id = the(&engine, ZoneLocation::Battlefield, C1);
    let striker_id = the(&engine, ZoneLocation::Battlefield, C2);
    let blocker_id = the(&engine, ZoneLocation::Battlefield, C3);

    // Grant AssignsNoCombatDamage to Bone Dancer AND Unblockable (via AddTempModifier) to Striker!
    activate(&mut engine, me(), dancer_id, 0);
    settle(&mut engine, me());
    activate(&mut engine, me(), stadium_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![striker_id],
                players: vec![],
            },
        )
        .expect("Blitzball Stadium targets Striker");
    settle(&mut engine, me());

    // Attack with both Bone Dancer (2/2, assigns no combat damage) and Striker (3/3, unblockable).
    to_attackers(&mut engine, me());
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (dancer_id, Defender::Player(them())),
                    (striker_id, Defender::Player(them())),
                ],
            },
        )
        .expect("declare both attackers");

    to_blockers(&mut engine, them());
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers");
    };
    let blk_opt = blockers
        .iter()
        .find(|o| o.blocker == blocker_id)
        .expect("blocker can still block Bone Dancer");
    assert!(
        !blk_opt.attackers.contains(&striker_id),
        "Blitzball Stadium's AddTempModifier(UNBLOCKABLE) prevents blocking Striker"
    );
    assert!(blk_opt.attackers.contains(&dancer_id));

    // Block Bone Dancer with 1/3 Blocker.
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker_id, dancer_id)],
            },
        )
        .expect("block Bone Dancer");
    // Resolve combat through second main.
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && holds_priority_in(e, me(), crate::turn::Step::Main)
    });

    // Striker dealt 3 unblocked damage -> opponent life is 17.
    assert_eq!(life(&engine, them()), 17);
    // Bone Dancer assigned NO combat damage -> 1/3 blocker has 0 damage marked!
    assert_eq!(
        engine.state().object(blocker_id).unwrap().damage,
        0,
        "Bone Dancer assigns no combat damage to its blocker"
    );
    // Blocker still dealt 1 normal combat damage to Bone Dancer!
    assert_eq!(
        engine.state().object(dancer_id).unwrap().damage,
        1,
        "blocker still deals normal combat damage to Bone Dancer"
    );

    // Next turn of P0: both until-EOT effects expired! Now test CantBeBlockedBy(&Filter::Any) too.
    next_turn_main(&mut engine, me());
    activate(&mut engine, me(), stadium_id, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![striker_id],
                players: vec![],
            },
        )
        .expect("CantBeBlockedBy(&Filter::Any) targets Striker");
    settle(&mut engine, me());

    to_attackers(&mut engine, me());
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (dancer_id, Defender::Player(them())),
                    (striker_id, Defender::Player(them())),
                ],
            },
        )
        .expect("attack again on next turn");
    to_blockers(&mut engine, them());
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers");
    };
    let blk_opt2 = blockers
        .iter()
        .find(|o| o.blocker == blocker_id)
        .expect("blocker can block Bone Dancer");
    assert!(
        !blk_opt2.attackers.contains(&striker_id),
        "AddTempModifier(CantBeBlockedBy(&Filter::Any)) also prevents blocking Striker"
    );
    // Don't block -> both Bone Dancer (2) and Striker (3) deal combat damage = 5 -> life 12!
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && holds_priority_in(e, me(), crate::turn::Step::Main)
    });
    assert_eq!(
        life(&engine, them()),
        12,
        "Bone Dancer deals normal 2 combat damage after AssignsNoCombatDamage expires"
    );
}

// =========================================================================
// 7. Aerial Extortionist: owner casts exiled card, controller cannot
// =========================================================================

static AERIAL_EXTORTIONIST_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::ExileWithPermission {
            target: TargetSpec::Object(&Filter::NONLAND),
            until: None,
            permission: ExilePermission::cast(PlayWindow::WhileExiled)
                .for_player(PlayerRel::OwnerOfTarget),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND))),
    ),
    // Free self-sacrifice to verify WhileExiled persists after Aerial Extortionist leaves.
    activated!(cost!(SacrificeSelf), &[]),
    // Free GainControl to test stealing an opponent's creature before exiling it!
    free(
        &[Effect::GainControlEx {
            target: TargetSpec::Object(&Filter::CREATURE),
            to: PlayerRel::You,
            duration: Duration::Indefinitely,
            untap: false,
            haste_until_eot: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
];

#[test]
fn wave15d_aerial_extortionist_owner_casts_exiled_card_controller_cannot() {
    let extortionist = card(
        C0,
        creature_face("Aerial Extortionist", "{3}{W}{W}", 4, 3),
        KeywordSet::FLYING,
        AERIAL_EXTORTIONIST_ABILITIES,
    );
    let victim = card(
        C1,
        creature_face("Exiled Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        15409,
        vec![extortionist, victim],
        [Seat::with(&[C0]), Seat::with(&[C1])],
    );
    to_main(&mut engine, me());

    let ext_id = the(&engine, ZoneLocation::Battlefield, C0);
    let victim_id = the(&engine, ZoneLocation::Battlefield, C1);

    // First steal P1's Exiled Bear so P0 controls it while P1 STILL owns it (CR 108.3)!
    activate(&mut engine, me(), ext_id, 2);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![victim_id],
                players: vec![],
            },
        )
        .expect("steal P1's bear");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(victim_id).unwrap().controller, me());
    assert_eq!(engine.state().object(victim_id).unwrap().owner, them());

    // Now exile it with Aerial Extortionist ("its owner may cast it while exiled").
    activate(&mut engine, me(), ext_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![victim_id],
                players: vec![],
            },
        )
        .expect("exile stolen bear");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(victim_id).unwrap().zone, Zone::Exile);

    // P0 (controller of Aerial Extortionist AND prior controller of the bear) has plenty of mana:
    // P0 must NOT be able to cast the exiled card!
    add_pool(&mut engine, me(), ManaColor::Green, 4);
    let (_, p0_legal) = offer(&engine);
    assert!(
        !p0_legal.castable.contains(&victim_id) && !p0_legal.payable.contains(&victim_id),
        "controller of Aerial Extortionist must NOT be offered opponent-owned exiled card"
    );
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card: victim_id })
            .is_err(),
        "controller cannot cast opponent-owned card exiled by Aerial Extortionist"
    );

    // Sacrifice Aerial Extortionist: PlayWindow::WhileExiled still keeps owner's permission alive!
    activate(&mut engine, me(), ext_id, 1);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(ext_id).unwrap().zone, Zone::Graveyard);
    clear_pool(&mut engine, me());

    // Advance to P1's main phase: P1 (owner) CAN cast Exiled Bear from exile!
    to_main(&mut engine, them());
    add_pool(&mut engine, them(), ManaColor::Green, 2);
    let (_, p1_legal) = offer(&engine);
    assert!(
        p1_legal.castable.contains(&victim_id),
        "owner must be able to cast their card exiled by Aerial Extortionist"
    );
    engine
        .apply(them(), PlayerAction::CastSpell { card: victim_id })
        .expect("owner casts exiled bear");
    settle(&mut engine, them());
    assert_eq!(
        engine.state().object(victim_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(victim_id).unwrap().controller, them());
    assert_eq!(pool_total(&engine, them()), 0);
}

// =========================================================================
// 8. Saddle + Filter::Saddled (Layer 7c static, targeting, hash, cleanup)
// =========================================================================

static MOUNT_ABILITIES: &[AbilityDef] = &[
    saddle!(2),
    static_ability!(Filter::Saddled, Modifier::ModifyPT(2, 2)),
];

static SADDLED_PING_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::GainLife {
        amount: Amount::Fixed(3),
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::Saddled))),
)];

#[test]
fn wave15d_saddle_and_filter_saddled_static_targeting_hash_and_cleanup() {
    let mut mount_f = creature_face("Desert Steed", "{2}{W}", 2, 2);
    mount_f.subtypes = &[subtypes::creature::MOUNT];
    let mount = card(C0, mount_f, KeywordSet::EMPTY, MOUNT_ABILITIES);
    let rider = card(
        C1,
        creature_face("Rider", "{1}{W}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let probe = card(
        C2,
        face("Saddle Probe", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SADDLED_PING_ABILITIES,
    );

    let mut engine = bench(
        15410,
        vec![mount, rider, probe],
        [Seat::with(&[C0, C1, C2]), Seat::default()],
    );
    to_main(&mut engine, me());

    let mount_id = the(&engine, ZoneLocation::Battlefield, C0);
    let rider_id = the(&engine, ZoneLocation::Battlefield, C1);
    let probe_id = the(&engine, ZoneLocation::Battlefield, C2);

    // Before saddling: Mount is 2/2 and Filter::Saddled probe has no legal target.
    assert_eq!(
        engine
            .state()
            .object(mount_id)
            .unwrap()
            .characteristics()
            .power,
        Some(2)
    );
    let (_, before_legal) = offer(&engine);
    assert!(
        !before_legal.abilities.contains(&(probe_id, 0)),
        "Filter::Saddled probe must not be offered before Mount is saddled"
    );
    let hash_before = engine.state().snapshot_hash();

    // Activate Saddle 2 tapping Rider.
    activate(&mut engine, me(), mount_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![rider_id],
            },
        )
        .expect("tap Rider to pay Saddle 2");
    settle(&mut engine, me());

    // Mount now has Rider::Saddled -> Filter::Saddled static buffs it to 4/4 AND probe is offered!
    assert!(
        engine
            .state()
            .object(mount_id)
            .unwrap()
            .riders
            .contains(&Rider::Saddled)
    );
    assert_eq!(
        engine
            .state()
            .object(mount_id)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(mount_id)
            .unwrap()
            .characteristics()
            .toughness,
        Some(4)
    );
    assert_ne!(engine.state().snapshot_hash(), hash_before);

    let (_, saddled_legal) = offer(&engine);
    assert!(
        saddled_legal.abilities.contains(&(probe_id, 0)),
        "Filter::Saddled probe must be offered once Mount is saddled"
    );
    activate(&mut engine, me(), probe_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![mount_id],
                players: vec![],
            },
        )
        .expect("target saddled Mount");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23);

    // Advance past cleanup onto opponent's turn: Rider::Saddled clears, Mount returns to 2/2!
    to_main(&mut engine, them());
    assert!(
        !engine
            .state()
            .object(mount_id)
            .unwrap()
            .riders
            .contains(&Rider::Saddled)
    );
    assert_eq!(
        engine
            .state()
            .object(mount_id)
            .unwrap()
            .characteristics()
            .power,
        Some(2)
    );
}
