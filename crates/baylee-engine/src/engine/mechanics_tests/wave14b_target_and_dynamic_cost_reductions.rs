//! Beat 18 / Engine Wave 14b deterministic rule tests:
//! 1. `Modifier::ReduceCostIfTargets` (`CR 118.7a–d`, `CR 601.2f`, `CR 602.2b`):
//!    - Colored + generic target-dependent spell reduction (*Brush Off* `{1}{U}`
//!      off `{1}{U}{U}`): pre-target optimistic affordability ONLY when a legal
//!      target matching `target_filter` exists (respecting Shroud), finalized
//!      colored reduction after targeting, guaranteed `spell_reduction` when all
//!      legal targets match, and CR 734.1 reversal when choosing a non-matching
//!      target without mana to cover the unreduced cost.
//!    - Multi-target single-application cap (*This Town Ain't Big Enough*):
//!      up to two targets, reduced ONCE by `{3}` whether 1 or 2 targets match
//!      `YOUR_PERMANENT`, and unreduced when 0 targets or only opponent targets
//!      are chosen.
//!    - Two-clause and state-dependent target filters (*Banish from Edoras*
//!      tapped, *Bouncer's Beatdown* black across either target clause, *Bury in
//!      Books* attacking, *Bitter Downfall* damaged this turn) and stack-target
//!      counterspell reductions.
//!    - Activated-ability target reductions (`abilities: true`, *Bladegraft
//!      Aspirant* Equip `{1}` -> `{0}` targeting Aspirant, Shroud suppression,
//!      reversal without mana on non-matching target, CR 602.2b mana payment
//!      window when tapping a land for a non-matching target, colored ability
//!      reduction, and composition after floored `AbilitiesCostLess`).
//! 2. `Modifier::AdjustSpellCostByAmount` (`CR 601.2f`, `CR 700.5`, `CR 700.8`,
//!    `CR 107.1b`):
//!    - *Daybreak Chimera* (`Amount::Devotion` mono-white, hybrid `{W/B}`
//!      counted once for two-color devotion, nonmodal DFC back face excluded per
//!      CR 712.8e, phased-out permanent excluded).
//!    - *Deadly Alliance* (`Amount::PartySize`, CR 700.8a–c exact role subset
//!      matching).
//!    - *Delightful Discovery* (`Amount::SpellsCastThisTurn(PlayerRel::Opponent)`).
//!    - *Deem Inferior* (`Amount::CardsDrawnThisTurn(PlayerRel::You)`).
//!    - *Ghalta, Primal Hunger* (`Amount::TotalPowerAmong(&Filter::YOUR_CREATURE)`,
//!      including negative-power subtraction and 0 floor per CR 107.1b).
//!    - Board-static dynamic `RaiseCost` taxes (`increase: true`) + CR 601.2f
//!      increases-before-reductions ordering, stack self-static isolation, and
//!      cycle-safe depth guard termination.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use crate::object::Status;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{
    Amount, Effect, Filter, Modifier, PlayerRel, StaticAbility, TargetReq, TargetSpec, ZoneRef,
    ZoneSel, mana, spell, static_ability,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::mana::{ManaColor, ManaPool};

const C0: u32 = 99_950;
const C1: u32 = 99_951;
const C2: u32 = 99_952;
const C3: u32 = 99_953;
const C4: u32 = 99_954;
const C5: u32 = 99_955;
const C6: u32 = 99_956;

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

// =========================================================================
// 1. Brush Off — Colored + generic ReduceCostIfTargets ({1}{U} off {1}{U}{U})
// =========================================================================

static BRUSH_OFF_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Stack,
        layer: baylee_cards_dsl::Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReduceCostIfTargets {
            who: PlayerRel::You,
            spell_filter: &Filter::This,
            target_filter: &Filter::ControlledByYou,
            reduction: mana!("{1}{U}"),
            abilities: false,
        },
        condition: None,
    }),
    spell!(
        &[Effect::bounce(TargetSpec::Object(&Filter::NONLAND))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND)))
    ),
];

#[test]
fn brush_off_colored_target_reduction_previews_finalizes_and_reverses_on_non_matching_target() {
    let brush_a = card(
        C0,
        face("Brush Off A", "{1}{U}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BRUSH_OFF_ABILITIES,
    );
    let brush_b = card(
        C1,
        face("Brush Off B", "{1}{U}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BRUSH_OFF_ABILITIES,
    );
    let my_bear = card(
        C2,
        creature_face("My Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_bear = card(
        C3,
        creature_face("Opp Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let shroud_bear = card(
        C4,
        creature_face("Shroud Bear", "{1}{G}", 2, 2),
        KeywordSet::SHROUD,
        &[],
    );

    // Subcase A: Only opponent's permanent + our Shroud permanent (illegal target!).
    // With only {U} floating, Brush Off must NOT receive optimistic reduction.
    let mut engine = bench(
        1401,
        vec![brush_a, brush_b, my_bear, opp_bear, shroud_bear],
        [Seat::with(&[C4]).holding(&[C0, C1]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());
    let brush_a_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let opp_id = the(&engine, ZoneLocation::Battlefield, C3);

    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&brush_a_id),
        "Shroud permanent you control is not a legal target, so {{U}} cannot cast Brush Off"
    );
    assert_eq!(legal.spell_reduction(brush_a_id, CastModeKind::Normal), 0);

    // Subcase B: With full {1}{U}{U}, casting Brush Off on opponent's bear pays all 3 mana.
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: brush_a_id })
        .expect("cast Brush Off at full cost");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_id],
                players: vec![],
            },
        )
        .expect("target opp bear");
    settle(&mut engine, me());
    assert_eq!(
        pool_total(&engine, me()),
        0,
        "targeting opponent's permanent pays full {{1}}{{U}}{{U}}"
    );
    assert_eq!(
        engine.state().object(opp_id).unwrap().zone,
        crate::zone::Zone::Hand
    );

    // Subcase C: Both our targetable bear and opponent's bear on board.
    let mut engine2 = bench(
        1402,
        vec![
            card(
                C0,
                face("Brush Off A", "{1}{U}{U}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                BRUSH_OFF_ABILITIES,
            ),
            card(
                C1,
                face("Brush Off B", "{1}{U}{U}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                BRUSH_OFF_ABILITIES,
            ),
            card(
                C2,
                creature_face("My Bear", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                C3,
                creature_face("Opp Bear", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[C2]).holding(&[C0, C1]), Seat::with(&[C3])],
    );
    to_main(&mut engine2, me());
    let brush_a2 = the(&engine2, ZoneLocation::Hand(me()), C0);
    let brush_b2 = the(&engine2, ZoneLocation::Hand(me()), C1);
    let my_id = the(&engine2, ZoneLocation::Battlefield, C2);
    let opp_id2 = the(&engine2, ZoneLocation::Battlefield, C3);

    // Float ONLY {U}: castable optimistically!
    add_pool(&mut engine2, me(), ManaColor::Blue, 1);
    let (_, legal2) = offer(&engine2);
    assert!(
        legal2.castable.contains(&brush_a2),
        "Brush Off ({{1}}{{U}}{{U}}) must be castable off {{U}} when a legal permanent you control exists"
    );
    // Not guaranteed while opponent's bear is also a legal target.
    assert_eq!(legal2.spell_reduction(brush_a2, CastModeKind::Normal), 0);

    // Attempt to target OPPONENT's bear with only {U} floating -> CR 734.1 reversal!
    engine2
        .apply(me(), PlayerAction::CastSpell { card: brush_a2 })
        .expect("start casting Brush Off off {U}");
    let _ = engine2.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![opp_id2],
            players: vec![],
        },
    );
    assert_eq!(
        engine2.state().object(brush_a2).unwrap().zone,
        crate::zone::Zone::Hand,
        "targeting opponent's bear with only {{U}} must reverse and keep Brush Off in hand"
    );
    assert_eq!(
        pool_total(&engine2, me()),
        1,
        "reversed cast must leave {{U}} untouched in pool"
    );

    // Now cast Brush Off targeting MY bear off {U} -> reduces {1}{U}{U} by {1}{U} to {U}!
    engine2
        .apply(me(), PlayerAction::CastSpell { card: brush_a2 })
        .expect("cast Brush Off targeting my bear");
    engine2
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_id],
                players: vec![],
            },
        )
        .expect("choose my bear");
    settle(&mut engine2, me());
    assert_eq!(
        pool_total(&engine2, me()),
        0,
        "Brush Off targeting my bear spent exactly {{U}}"
    );
    assert_eq!(
        engine2.state().object(my_id).unwrap().zone,
        crate::zone::Zone::Hand,
        "my bear bounced to hand"
    );

    // Subcase D: ONLY my permanent on board -> guaranteed reduction = 2 (cmc of {1}{U}).
    let mut engine3 = bench(
        1403,
        vec![
            card(
                C0,
                face("Brush Off Only", "{1}{U}{U}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                BRUSH_OFF_ABILITIES,
            ),
            card(
                C2,
                creature_face("My Only Bear", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[C2]).holding(&[C0]), Seat::with(&[])],
    );
    to_main(&mut engine3, me());
    let brush3 = the(&engine3, ZoneLocation::Hand(me()), C0);
    let (_, legal3) = offer(&engine3);
    assert_eq!(
        legal3.spell_reduction(brush3, CastModeKind::Normal),
        2,
        "when every legal target matches YOUR_PERMANENT, guaranteed reduction is 2 ({{1}}{{U}})"
    );
    // Also verify leaving extra mana floating only spends {U} when targeting my bear!
    let _ = brush_b2;
    add_pool(&mut engine3, me(), ManaColor::Blue, 3);
    engine3
        .apply(me(), PlayerAction::CastSpell { card: brush3 })
        .expect("cast with excess floating");
    let my_id3 = the(&engine3, ZoneLocation::Battlefield, C2);
    engine3
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_id3],
                players: vec![],
            },
        )
        .expect("choose my bear");
    settle(&mut engine3, me());
    assert_eq!(
        pool_total(&engine3, me()),
        2,
        "must spend only {{U}} out of {{U}}{{U}}{{U}}, leaving 2 mana floating"
    );
}

// =========================================================================
// 2. This Town Ain't Big Enough — Up-to-two targets, single {3} reduction
// =========================================================================

static THIS_TOWN_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Stack,
        layer: baylee_cards_dsl::Layer::Text,
        filter: Filter::This,
        modifier: Modifier::ReduceCostIfTargets {
            who: PlayerRel::You,
            spell_filter: &Filter::This,
            target_filter: &Filter::ControlledByYou,
            reduction: mana!("{3}"),
            abilities: false,
        },
        condition: None,
    }),
    spell!(
        &[Effect::bounce(TargetSpec::Object(&Filter::NONLAND))],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::NONLAND), 2,))
    ),
];

#[test]
fn this_town_reduces_once_for_one_or_two_own_targets_and_not_for_zero_or_opp_only() {
    let town_a = card(
        C0,
        face("This Town A", "{4}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        THIS_TOWN_ABILITIES,
    );
    let town_b = card(
        C1,
        face("This Town B", "{4}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        THIS_TOWN_ABILITIES,
    );
    let town_c = card(
        C2,
        face("This Town C", "{4}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        THIS_TOWN_ABILITIES,
    );
    let my_a = card(
        C3,
        creature_face("My A", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let my_b = card(
        C4,
        creature_face("My B", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_a = card(
        C5,
        creature_face("Opp A", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_b = card(
        C6,
        creature_face("Opp B", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1404,
        vec![town_a, town_b, town_c, my_a, my_b, opp_a, opp_b],
        [
            Seat::with(&[C3, C4]).holding(&[C0, C1, C2]),
            Seat::with(&[C5, C6]),
        ],
    );
    to_main(&mut engine, me());
    let t_a = the(&engine, ZoneLocation::Hand(me()), C0);
    let t_b = the(&engine, ZoneLocation::Hand(me()), C1);
    let t_c = the(&engine, ZoneLocation::Hand(me()), C2);
    let my_a_id = the(&engine, ZoneLocation::Battlefield, C3);
    let my_b_id = the(&engine, ZoneLocation::Battlefield, C4);
    let opp_a_id = the(&engine, ZoneLocation::Battlefield, C5);
    let opp_b_id = the(&engine, ZoneLocation::Battlefield, C6);

    // 1. Target 2 permanents you control with {1}{U} -> reduces ONCE by {3} (NOT {6}!).
    //    Float {2}{U} so we can prove exactly {1}{U} (2 mana) was spent, leaving 1 floating.
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: t_a })
        .expect("cast This Town A");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_a_id, my_b_id],
                players: vec![],
            },
        )
        .expect("target 2 own creatures");
    settle(&mut engine, me());
    assert_eq!(
        pool_total(&engine, me()),
        1,
        "targeting 2 own permanents reduces {{4}}{{U}} ONCE by {{3}} to {{1}}{{U}}, leaving 1 of 3 floating"
    );

    // Re-deploy my_a to board for subtest 2.
    clear_pool(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: my_a_id })
        .expect("recast my_a");
    settle(&mut engine, me());

    // 2. Float {1}{U}, try targeting 0 targets -> unreduced {4}{U}, must reverse!
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: t_b })
        .expect("start casting This Town B");
    let _ = engine.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
    );
    assert_eq!(
        engine.state().object(t_b).unwrap().zone,
        crate::zone::Zone::Hand,
        "0 targets gets no reduction; cast must reverse with only {{1}}{{U}}"
    );
    assert_eq!(pool_total(&engine, me()), 2);

    // 3. Target 1 own + 1 opponent off {1}{U} -> succeeds and spends exactly {1}{U}!
    engine
        .apply(me(), PlayerAction::CastSpell { card: t_b })
        .expect("cast This Town B targeting 1 own + 1 opp");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_a_id, opp_a_id],
                players: vec![],
            },
        )
        .expect("choose 1 own + 1 opp");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(opp_a_id).unwrap().zone,
        crate::zone::Zone::Hand
    );

    // 4. Now NO own nonland permanents left on battlefield (only opp_b)!
    //    Float {1}{U}: This Town C must NOT be castable!
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let (_, legal_no_own) = offer(&engine);
    assert!(
        !legal_no_own.castable.contains(&t_c),
        "with no own permanent on board, {{1}}{{U}} cannot cast This Town"
    );
    // With {4}{U}, targeting opp_b succeeds and spends all 5 mana.
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: t_c })
        .expect("cast This Town C at full {4}{U}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_b_id],
                players: vec![],
            },
        )
        .expect("target opp_b");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
}

// =========================================================================
// 3. Two-clause, tapped, attacking, damaged, and stack target reductions
// =========================================================================

#[test]
fn two_clause_bouncers_beatdown_and_state_filters_reduce_accurately() {
    static BLACK_PERM: Filter = Filter::HasColor(ColorSet::of(Color::Black));
    static BEATDOWN_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::ReduceCostIfTargets {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                target_filter: &BLACK_PERM,
                reduction: mana!("{1}"),
                abilities: false,
            },
            condition: None,
        }),
        AbilityDef::Spell {
            effects: &[Effect::TapTarget],
            targets: Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
            second_targets: Some(TargetReq::one(TargetSpec::Object(
                &Filter::OPPONENT_CREATURE,
            ))),
            condition: None,
        },
    ];
    static TAPPED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Tapped]);
    static BANISH_EDORAS_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::ReduceCostIfTargets {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                target_filter: &TAPPED_CREATURE,
                reduction: mana!("{2}"),
                abilities: false,
            },
            condition: None,
        }),
        spell!(
            &[Effect::exile(TargetSpec::Object(&Filter::CREATURE))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ];
    static DAMAGED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::DealtDamageThisTurn]);
    static BITTER_DOWNFALL_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::ReduceCostIfTargets {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                target_filter: &DAMAGED_CREATURE,
                reduction: mana!("{3}"),
                abilities: false,
            },
            condition: None,
        }),
        spell!(
            &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ];
    static PING_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let beatdown = card(
        C0,
        face("Beatdown Probe", "{2}{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BEATDOWN_ABILITIES,
    );
    let banish = card(
        C1,
        face("Banish Probe", "{4}{W}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BANISH_EDORAS_ABILITIES,
    );
    let downfall = card(
        C2,
        face("Downfall Probe", "{3}{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BITTER_DOWNFALL_ABILITIES,
    );
    let ping = card(
        C3,
        face("Ping Probe", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        PING_ABILITIES,
    );
    let green_bear = card(
        C4,
        creature_face("Green Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let black_knight = card(
        C5,
        creature_face("Black Knight", "{B}{B}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1405,
        vec![beatdown, banish, downfall, ping, green_bear, black_knight],
        [
            Seat::with(&[C4]).holding(&[C0, C1, C2, C3]),
            Seat::with(&[C5]),
        ],
    );
    to_main(&mut engine, me());
    let beatdown_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let banish_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let downfall_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let ping_id = the(&engine, ZoneLocation::Hand(me()), C3);
    let green_id = the(&engine, ZoneLocation::Battlefield, C4);
    let black_id = the(&engine, ZoneLocation::Battlefield, C5);

    // 1. Beatdown targets green_id (clause 1) and black_id (clause 2): black_id
    //    is black so {2}{G} reduces to {1}{G} (and guaranteed = 1 since black_id
    //    is the only opponent creature).
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_reduction(beatdown_id, CastModeKind::Normal), 1);
    assert!(legal.castable.contains(&beatdown_id));
    engine
        .apply(me(), PlayerAction::CastSpell { card: beatdown_id })
        .expect("cast Beatdown off {1}{G}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![green_id],
                players: vec![],
            },
        )
        .expect("first target green_id");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![black_id],
                players: vec![],
            },
        )
        .expect("second target black_id");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // 2. Banish from Edoras ({4}{W}, -{2} if targeting tapped creature):
    //    green_id is now TAPPED (tapped by Beatdown above!), so {2}{W} casts Banish!
    assert!(
        engine
            .state()
            .object(green_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    // First untap green_id to verify negative case: with both creatures untapped, {2}{W} cannot cast Banish.
    engine.state.set_tapped(green_id, false);
    add_pool(&mut engine, me(), ManaColor::White, 3);
    let (_, legal_untapped) = offer(&engine);
    assert!(
        !legal_untapped.castable.contains(&banish_id),
        "no tapped creature -> {{2}}{{W}} cannot cast Banish"
    );
    // Tap green_id -> now {2}{W} casts Banish targeting green_id!
    engine.state.set_tapped(green_id, true);
    refresh_offer(&mut engine);
    let (_, legal_tapped) = offer(&engine);
    assert!(legal_tapped.castable.contains(&banish_id));
    engine
        .apply(me(), PlayerAction::CastSpell { card: banish_id })
        .expect("cast Banish off {2}{W}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![green_id],
                players: vec![],
            },
        )
        .expect("target tapped green_id");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(green_id).unwrap().zone,
        crate::zone::Zone::Exile
    );

    // 3. Bitter Downfall ({3}{B}, -{3} if targeting creature dealt damage this turn):
    //    black_id is undamaged -> {B} cannot cast Downfall.
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    let (_, legal_undamaged) = offer(&engine);
    assert!(
        !legal_undamaged.castable.contains(&downfall_id),
        "undamaged creature -> {{B}} cannot cast Bitter Downfall"
    );
    // Deal 1 damage to black_id with Ping -> now {B} casts Downfall targeting black_id!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ping_id })
        .expect("cast Ping");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![black_id],
                players: vec![],
            },
        )
        .expect("target black_id with Ping");
    settle(&mut engine, me());
    let (_, legal_damaged) = offer(&engine);
    assert!(
        legal_damaged.castable.contains(&downfall_id),
        "damaged creature -> {{B}} casts Bitter Downfall"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: downfall_id })
        .expect("cast Downfall off {B}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![black_id],
                players: vec![],
            },
        )
        .expect("destroy damaged black_id");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(black_id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
}

#[test]
fn stack_targeting_spell_reduces_when_targeting_matching_spell_on_stack() {
    static COUNTER_CREATURE_DISCOUNT: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::ReduceCostIfTargets {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                target_filter: &Filter::CREATURE,
                reduction: mana!("{2}"),
                abilities: false,
            },
            condition: None,
        }),
        spell!(
            &[Effect::CounterTargetSpell],
            targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
        ),
    ];
    static GAIN_LIFE: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let counter = card(
        C0,
        face("Creature Counter Probe", "{3}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COUNTER_CREATURE_DISCOUNT,
    );
    let opp_instant = card(
        C1,
        face("Opp Instant", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        GAIN_LIFE,
    );
    let opp_creature = card(
        C2,
        creature_face("Opp Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1406,
        vec![counter, opp_instant, opp_creature],
        [
            Seat::with(&[]).holding(&[C0]),
            Seat::with(&[]).holding(&[C1, C2]),
        ],
    );
    to_main(&mut engine, them());
    let counter_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let inst_id = the(&engine, ZoneLocation::Hand(them()), C1);
    let crea_id = the(&engine, ZoneLocation::Hand(them()), C2);

    // Opponent casts noncreature instant -> with {1}{U}, counterspell is NOT castable!
    add_pool(&mut engine, them(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(them(), PlayerAction::CastSpell { card: inst_id })
        .expect("opp casts instant");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass to me");
    let (_, legal_vs_inst) = offer(&engine);
    assert!(
        !legal_vs_inst.castable.contains(&counter_id),
        "noncreature spell on stack must not trigger creature-spell target discount"
    );
    settle(&mut engine, them());

    // Opponent casts creature spell -> now {1}{U} casts and counters it!
    add_pool(&mut engine, them(), ManaColor::Green, 2);
    engine
        .apply(them(), PlayerAction::CastSpell { card: crea_id })
        .expect("opp casts creature");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass to me");
    let (_, legal_vs_crea) = offer(&engine);
    assert_eq!(
        legal_vs_crea.spell_reduction(counter_id, CastModeKind::Normal),
        2
    );
    assert!(legal_vs_crea.castable.contains(&counter_id));
    engine
        .apply(me(), PlayerAction::CastSpell { card: counter_id })
        .expect("cast discounted counterspell");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![crea_id],
                players: vec![],
            },
        )
        .expect("target creature spell on stack");
    settle(&mut engine, them());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(crea_id).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "creature spell was countered"
    );
}

// =========================================================================
// 4. Bladegraft Aspirant — Activated ability target-dependent cost reduction
// =========================================================================

static EQUIPMENT_FILTER: Filter = Filter::HasSubtype(subtypes::artifact::EQUIPMENT);

static ASPIRANT_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::Any,
    Modifier::ReduceCostIfTargets {
        who: PlayerRel::You,
        spell_filter: &EQUIPMENT_FILTER,
        target_filter: &Filter::This,
        reduction: mana!("{1}"),
        abilities: true,
    }
)];

static EQUIP_1_ABILITIES: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost {
        mana: mana!("{1}"),
        parts: &[],
    },
    effects: &[Effect::AttachSelf {
        target: TargetSpec::Object(&Filter::YOUR_CREATURE),
    }],
    targets: Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    second_targets: None,
    timing: ActivationTiming::SorcerySpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
    limit: ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
}];

fn equipment_face(name: &'static str, mana_cost: &'static str) -> FaceDef {
    let mut f = face(name, mana_cost, TypeSet::ARTIFACT);
    f.subtypes = &[subtypes::artifact::EQUIPMENT];
    f
}

#[test]
fn bladegraft_aspirant_reduces_equip_targeting_self_and_enforces_actual_target() {
    let aspirant = card(
        C0,
        creature_face("Aspirant Probe", "{2}{R}", 2, 3),
        KeywordSet::EMPTY,
        ASPIRANT_ABILITIES,
    );
    let sword = card(
        C1,
        equipment_face("Short Sword Probe", "{1}"),
        KeywordSet::EMPTY,
        EQUIP_1_ABILITIES,
    );
    let bear = card(
        C2,
        creature_face("Bear Probe", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // Subcase A: Aspirant + Bear + Sword, 0 mana, NO lands.
    let mut engine = bench(
        1407,
        vec![aspirant, sword, bear],
        [Seat::with(&[C0, C1, C2]), Seat::with(&[])],
    );
    to_main(&mut engine, me());
    let asp_id = the(&engine, ZoneLocation::Battlefield, C0);
    let sword_id = the(&engine, ZoneLocation::Battlefield, C1);
    let bear_id = the(&engine, ZoneLocation::Battlefield, C2);

    // Pre-target offer sees Aspirant is a legal target -> Equip {1} is offered at {0}!
    let (_, legal) = offer(&engine);
    assert!(
        legal.abilities.contains(&(sword_id, 0)),
        "Equip {{1}} must be offered at {{0}} when Bladegraft Aspirant is a legal target"
    );

    // Try activating Equip targeting BEAR instead with 0 mana and no lands -> must reverse!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: sword_id,
                ability_index: 0,
            },
        )
        .expect("start equip");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_id],
                players: vec![],
            },
        )
        .expect("choose bear -> reverses cleanly");
    assert_eq!(
        engine.state().object(sword_id).unwrap().attached_to,
        None,
        "equipping Bear without mana must reverse cleanly"
    );

    // Activate Equip targeting ASPIRANT with 0 mana -> pays {0} and attaches!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: sword_id,
                ability_index: 0,
            },
        )
        .expect("start equip targeting Aspirant");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![asp_id],
                players: vec![],
            },
        )
        .expect("choose Aspirant");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sword_id).unwrap().attached_to,
        Some(asp_id),
        "Sword attached to Aspirant for {{0}}"
    );

    // Subcase B: Shroud on Aspirant prevents legal targeting -> Equip {1} NOT offered at {0}!
    let shroud_asp = card(
        C3,
        creature_face("Shroud Aspirant", "{2}{R}", 2, 3),
        KeywordSet::SHROUD,
        ASPIRANT_ABILITIES,
    );
    let mut engine_shroud = bench(
        1408,
        vec![
            shroud_asp,
            card(
                C1,
                equipment_face("Short Sword Probe", "{1}"),
                KeywordSet::EMPTY,
                EQUIP_1_ABILITIES,
            ),
            card(
                C2,
                creature_face("Bear Probe", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[C3, C1, C2]), Seat::with(&[])],
    );
    to_main(&mut engine_shroud, me());
    let sword_s = the(&engine_shroud, ZoneLocation::Battlefield, C1);
    let (_, legal_s) = offer(&engine_shroud);
    assert!(
        !legal_s.abilities.contains(&(sword_s, 0)),
        "Shroud Aspirant is not a legal target, so Equip {{1}} must NOT be free"
    );
    assert!(
        legal_s
            .unpaid_abilities
            .contains(&(sword_s, 0, mana!("{1}"))),
        "Equip {{1}} remains unpaid at full {{1}} when Aspirant has Shroud"
    );
}

#[test]
fn activated_colored_target_reduction_and_floor_composition() {
    static COLORED_AB_RED: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ReduceCostIfTargets {
            who: PlayerRel::You,
            spell_filter: &Filter::ARTIFACT,
            target_filter: &Filter::This,
            reduction: mana!("{1}{U}"),
            abilities: true,
        }
    )];
    static HEARTSTONE_LIKE: &[AbilityDef] = &[static_ability!(
        Filter::ARTIFACT,
        Modifier::AbilitiesCostLess {
            amount: 1,
            minimum: 1,
        }
    )];
    static ARTIFACT_TAP_PING: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: mana!("{2}{U}"),
            parts: &[],
        },
        effects: &[Effect::TapTarget],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];

    let helper = card(
        C0,
        creature_face("Blue Target Helper", "{U}", 1, 1),
        KeywordSet::EMPTY,
        COLORED_AB_RED,
    );
    let heartstone = card(
        C1,
        face("Floor Reducer", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        HEARTSTONE_LIKE,
    );
    let relic = card(
        C2,
        face("Relic Probe", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ARTIFACT_TAP_PING,
    );

    // Base {2}{U} - Heartstone {1} (floor 1 -> {1}{U}) - Target reduction {1}{U} -> {0}!
    let mut engine = bench(
        1409,
        vec![helper, heartstone, relic],
        [Seat::with(&[C0, C1, C2]), Seat::with(&[])],
    );
    to_main(&mut engine, me());
    let helper_id = the(&engine, ZoneLocation::Battlefield, C0);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C2);

    let (_, legal) = offer(&engine);
    assert!(
        legal.abilities.contains(&(relic_id, 0)),
        "Heartstone ({{2}}{{U}} -> {{1}}{{U}}) + colored target reduction ({{1}}{{U}}) makes activation free"
    );
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: relic_id,
                ability_index: 0,
            },
        )
        .expect("activate relic for {0}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![helper_id],
                players: vec![],
            },
        )
        .expect("target helper");
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(helper_id)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "ability resolved and tapped helper"
    );
}

// =========================================================================
// 5. AdjustSpellCostByAmount — Devotion (Daybreak Chimera), Party (Deadly
//    Alliance), Opp Spells (Delightful Discovery), Cards Drawn (Deem Inferior),
//    Total Power (Ghalta), Dynamic RaiseCost Taxes, Isolation & Depth Guard
// =========================================================================

#[test]
fn daybreak_chimera_devotion_counts_symbols_hybrid_once_and_skips_back_faces_and_phased_out() {
    static DEVOTION_WHITE: Amount = Amount::Devotion {
        who: PlayerRel::You,
        colors: ColorSet::of(Color::White),
    };
    static CHIMERA_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Stack,
        layer: baylee_cards_dsl::Layer::Text,
        filter: Filter::This,
        modifier: Modifier::AdjustSpellCostByAmount {
            who: PlayerRel::You,
            filter: &Filter::This,
            increase: false,
            amount: &DEVOTION_WHITE,
        },
        condition: None,
    })];

    let chimera = card(
        C0,
        creature_face("Daybreak Chimera Probe", "{3}{W}{W}", 3, 3),
        KeywordSet::FLYING,
        CHIMERA_ABILITIES,
    );
    let white_two = card(
        C1,
        creature_face("White Knight", "{W}{W}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let hybrid_wb = card(
        C2,
        creature_face("Hybrid Spirit", "{W/B}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_white = card(
        C3,
        creature_face("Opp Angel", "{W}{W}{W}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1410,
        vec![chimera, white_two, hybrid_wb, opp_white],
        [Seat::with(&[C1, C2]).holding(&[C0]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());
    let chimera_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let white_id = the(&engine, ZoneLocation::Battlefield, C1);
    let hybrid_id = the(&engine, ZoneLocation::Battlefield, C2);

    // Your devotion to white: {W}{W} (2) + {W/B} (1) = 3. Opponent's {W}{W}{W} doesn't count.
    let (_, legal) = offer(&engine);
    assert_eq!(legal.spell_reduction(chimera_id, CastModeKind::Normal), 3);

    // Verify CR 700.5 two-color devotion: {W/B} counts ONCE (not twice!) toward white+black!
    let wb_devotion = crate::eval::amount(
        &Amount::Devotion {
            who: PlayerRel::You,
            colors: ColorSet::of(Color::White).union(ColorSet::of(Color::Black)),
        },
        engine.state(),
        me(),
        chimera_id,
        None,
    );
    assert_eq!(
        wb_devotion, 3,
        "CR 700.5: {{W}}{{W}} (2) + {{W/B}} (1, NOT 2) = 3 devotion to white and black"
    );

    // Nonmodal DFC / disturb back face (`front_mana_value = Some(...)`) contributes 0 to devotion (CR 712.8e).
    engine
        .state
        .object_mut(white_id)
        .unwrap()
        .base_mut()
        .front_mana_value = Some(2);
    refresh_offer(&mut engine);
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(chimera_id, CastModeKind::Normal),
        1,
        "nonmodal DFC / disturb back face contributes 0 to devotion (CR 712.8e)"
    );
    engine
        .state
        .object_mut(white_id)
        .unwrap()
        .base_mut()
        .front_mana_value = None;

    // Phased-out permanent contributes 0 to devotion (CR 702.26b).
    engine
        .state
        .object_mut(hybrid_id)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    refresh_offer(&mut engine);
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(chimera_id, CastModeKind::Normal),
        2,
        "phased-out permanent contributes 0 to devotion"
    );
    engine
        .state
        .object_mut(hybrid_id)
        .unwrap()
        .status
        .remove(Status::PHASED_OUT);
    refresh_offer(&mut engine);

    // Cast Daybreak Chimera ({3}{W}{W}) off {W}{W} with devotion 3 -> pays {W}{W}!
    add_pool(&mut engine, me(), ManaColor::White, 2);
    assert!(offer(&engine).1.castable.contains(&chimera_id));
    engine
        .apply(me(), PlayerAction::CastSpell { card: chimera_id })
        .expect("cast Daybreak Chimera off {W}{W}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(chimera_id).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

#[test]
fn deadly_alliance_party_size_reduction() {
    static PARTY_YOU: Amount = Amount::PartySize(PlayerRel::You);
    static ALLIANCE_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::AdjustSpellCostByAmount {
                who: PlayerRel::You,
                filter: &Filter::This,
                increase: false,
                amount: &PARTY_YOU,
            },
            condition: None,
        }),
        spell!(
            &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ];

    let mut cr_face = creature_face("Cleric Rogue", "{1}{B}", 2, 2);
    cr_face.subtypes = &[subtypes::creature::CLERIC, subtypes::creature::ROGUE];
    let mut cl_face = creature_face("Lone Cleric", "{W}", 1, 1);
    cl_face.subtypes = &[subtypes::creature::CLERIC];
    let mut war_face = creature_face("Warrior", "{R}", 2, 1);
    war_face.subtypes = &[subtypes::creature::WARRIOR];
    let mut wiz_face = creature_face("Wizard", "{U}", 1, 2);
    wiz_face.subtypes = &[subtypes::creature::WIZARD];

    let alliance = card(
        C0,
        face("Deadly Alliance Probe", "{4}{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ALLIANCE_ABILITIES,
    );
    let cleric_rogue = card(C1, cr_face, KeywordSet::EMPTY, &[]);
    let cleric = card(C2, cl_face, KeywordSet::EMPTY, &[]);
    let warrior = card(C3, war_face, KeywordSet::EMPTY, &[]);
    let wizard = card(C4, wiz_face, KeywordSet::EMPTY, &[]);
    let opp_target = card(
        C5,
        creature_face("Opp Target", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1411,
        vec![alliance, cleric_rogue, cleric, warrior, wizard, opp_target],
        [
            Seat::with(&[C1]).holding(&[C0, C2, C3, C4]),
            Seat::with(&[C5]),
        ],
    );
    to_main(&mut engine, me());
    let alliance_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let cleric_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let warrior_id = the(&engine, ZoneLocation::Hand(me()), C3);
    let wizard_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let opp_id = the(&engine, ZoneLocation::Battlefield, C5);

    // 1 creature (Cleric Rogue) fills at most 1 party role (CR 700.8a) -> reduction = 1.
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(alliance_id, CastModeKind::Normal),
        1
    );

    // Cast Lone Cleric -> Cleric Rogue takes Rogue, Lone Cleric takes Cleric -> party = 2.
    add_pool(&mut engine, me(), ManaColor::White, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: cleric_id })
        .expect("cast Lone Cleric");
    settle(&mut engine, me());
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(alliance_id, CastModeKind::Normal),
        2
    );

    // Cast Warrior + Wizard -> full party = 4 -> Deadly Alliance ({4}{B}) casts for {B}!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: warrior_id })
        .expect("cast Warrior");
    settle(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: wizard_id })
        .expect("cast Wizard");
    settle(&mut engine, me());
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(alliance_id, CastModeKind::Normal),
        4
    );
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: alliance_id })
        .expect("cast Deadly Alliance for {B}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_id],
                players: vec![],
            },
        )
        .expect("destroy opp creature");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(opp_id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
}

#[test]
fn delightful_discovery_opp_spells_and_deem_inferior_cards_drawn_this_turn() {
    static OPP_SPELLS: Amount = Amount::SpellsCastThisTurn(PlayerRel::Opponent);
    static DRAWS_YOU: Amount = Amount::CardsDrawnThisTurn(PlayerRel::You);
    static DISCOVERY_AB: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::AdjustSpellCostByAmount {
                who: PlayerRel::You,
                filter: &Filter::This,
                increase: false,
                amount: &OPP_SPELLS,
            },
            condition: None,
        }),
        spell!(&[Effect::GainLife {
            amount: Amount::Fixed(2),
        }]),
    ];
    static DEEM_AB: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::AdjustSpellCostByAmount {
                who: PlayerRel::You,
                filter: &Filter::This,
                increase: false,
                amount: &DRAWS_YOU,
            },
            condition: None,
        }),
        spell!(&[Effect::GainLife {
            amount: Amount::Fixed(1),
        }]),
    ];
    static DRAW_TWO: &[AbilityDef] = &[spell!(&[Effect::DrawCards {
        amount: Amount::Fixed(2),
    }])];
    static OPP_PING: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let discovery = card(
        C0,
        face("Delightful Discovery Probe", "{4}{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DISCOVERY_AB,
    );
    let deem = card(
        C1,
        face("Deem Inferior Probe", "{3}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DEEM_AB,
    );
    let draw2 = card(
        C2,
        face("Divination Probe", "{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DRAW_TWO,
    );
    let opp_s1 = card(
        C3,
        face("Opp Spell 1", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        OPP_PING,
    );
    let opp_s2 = card(
        C4,
        face("Opp Spell 2", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        OPP_PING,
    );

    let mut engine = bench(
        1412,
        vec![discovery, deem, draw2, opp_s1, opp_s2],
        [
            Seat::with(&[]).holding(&[C0, C1, C2]),
            Seat::with(&[]).holding(&[C3, C4]),
        ],
    );
    to_main(&mut engine, them());
    let disc_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let deem_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let draw2_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let opp1_id = the(&engine, ZoneLocation::Hand(them()), C3);
    let opp2_id = the(&engine, ZoneLocation::Hand(them()), C4);

    // Opponent casts 2 spells on their turn.
    add_pool(&mut engine, them(), ManaColor::White, 2);
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp1_id })
        .expect("opp spell 1");
    settle(&mut engine, them());
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp2_id })
        .expect("opp spell 2");
    settle(&mut engine, them());

    // Pass to me: Discovery has reduction 2 (opp cast 2); Deem Inferior has 0 (I drew 0 on opp's turn).
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass to me");
    let (_, legal_me) = offer(&engine);
    assert_eq!(legal_me.spell_reduction(disc_id, CastModeKind::Normal), 2);
    assert_eq!(legal_me.spell_reduction(deem_id, CastModeKind::Normal), 0);

    // Cast Divination Probe (I draw 2 cards): Discovery STILL has reduction 2 (my spell didn't bump
    // opponent's spells cast!), while Deem Inferior rises from 0 to 2!
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: draw2_id })
        .expect("cast draw 2");
    settle(&mut engine, them());
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass back to me");
    let (_, legal_after_draw) = offer(&engine);
    assert_eq!(
        legal_after_draw.spell_reduction(disc_id, CastModeKind::Normal),
        2,
        "my own spell cast must not increase opponents' spells cast count"
    );
    assert_eq!(
        legal_after_draw.spell_reduction(deem_id, CastModeKind::Normal),
        2,
        "drawing 2 cards reduces Deem Inferior by {{2}}"
    );

    // Cast Discovery ({4}{W} - 2 = {2}{W}) and Deem Inferior ({3}{U} - 2 = {1}{U}).
    add_pool(&mut engine, me(), ManaColor::White, 3);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: disc_id })
        .expect("cast Discovery for {2}{W}");
    settle(&mut engine, them());
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass to me");
    engine
        .apply(me(), PlayerAction::CastSpell { card: deem_id })
        .expect("cast Deem Inferior for {1}{U}");
    settle(&mut engine, them());
    assert_eq!(
        pool_total(&engine, me()),
        0,
        "both spells spent exact dynamically reduced costs"
    );
}

#[test]
fn ghalta_total_power_handles_positive_negative_and_zero_floor() {
    static TOTAL_POWER_YOUR_CREATURES: Amount = Amount::TotalPowerAmong(&Filter::YOUR_CREATURE);
    static GHALTA_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Stack,
        layer: baylee_cards_dsl::Layer::Text,
        filter: Filter::This,
        modifier: Modifier::AdjustSpellCostByAmount {
            who: PlayerRel::You,
            filter: &Filter::This,
            increase: false,
            amount: &TOTAL_POWER_YOUR_CREATURES,
        },
        condition: None,
    })];
    static NEG_PUMP: &[AbilityDef] = &[static_ability!(Filter::This, Modifier::ModifyPT(-4, 0))];

    let ghalta = card(
        C0,
        creature_face("Ghalta Probe", "{10}{G}{G}", 12, 12),
        KeywordSet::TRAMPLE,
        GHALTA_ABILITIES,
    );
    let beast = card(
        C1,
        creature_face("Beast", "{3}{G}", 5, 5),
        KeywordSet::EMPTY,
        &[],
    );
    let brute = card(
        C2,
        creature_face("Brute", "{3}{G}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let shrunken = card(
        C3,
        creature_face("Shrunken", "{1}{B}", 1, 4),
        KeywordSet::EMPTY,
        NEG_PUMP,
    );
    let opp_giant = card(
        C4,
        creature_face("Opp Giant", "{5}{R}", 8, 8),
        KeywordSet::EMPTY,
        &[],
    );

    // 1. Beast (5) + Brute (4) + Shrunken (1 - 4 = -3) => total power = 6 (opp's 8/8 ignored!).
    let mut engine = bench(
        1413,
        vec![ghalta, beast, brute, shrunken, opp_giant],
        [Seat::with(&[C1, C2, C3]).holding(&[C0]), Seat::with(&[C4])],
    );
    to_main(&mut engine, me());
    let ghalta_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let shrunken_id = the(&engine, ZoneLocation::Battlefield, C3);
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(ghalta_id, CastModeKind::Normal),
        6,
        "CR 107.1b: 5 + 4 + (-3) = 6 total power"
    );

    // Move Shrunken to graveyard -> total power becomes 5 + 4 = 9 -> Ghalta costs {1}{G}{G}!
    engine
        .state
        .move_object(
            shrunken_id,
            ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move shrunken creature to graveyard");
    refresh_offer(&mut engine);
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(ghalta_id, CastModeKind::Normal),
        9
    );
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ghalta_id })
        .expect("cast Ghalta off {1}{G}{G}");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);

    // 2. Net negative total power (-3) floors at 0 (never increases cost!).
    let mut engine_neg = bench(
        1414,
        vec![
            card(
                C0,
                creature_face("Ghalta Probe", "{10}{G}{G}", 12, 12),
                KeywordSet::TRAMPLE,
                GHALTA_ABILITIES,
            ),
            card(
                C3,
                creature_face("Shrunken", "{1}{B}", 1, 4),
                KeywordSet::EMPTY,
                NEG_PUMP,
            ),
        ],
        [Seat::with(&[C3]).holding(&[C0]), Seat::with(&[])],
    );
    to_main(&mut engine_neg, me());
    let ghalta_neg = the(&engine_neg, ZoneLocation::Hand(me()), C0);
    assert_eq!(
        offer(&engine_neg)
            .1
            .spell_reduction(ghalta_neg, CastModeKind::Normal),
        0,
        "net negative total power floors at 0"
    );
}

#[test]
fn board_dynamic_raise_cost_tax_orders_before_reduction_and_stack_static_stays_isolated() {
    static YOUR_ARTIFACTS: Amount = Amount::CountOf {
        filter: &Filter::YOUR_ARTIFACT,
        zone: ZoneSel::Battlefield,
    };
    static TAX_ENCHANTMENT_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::AdjustSpellCostByAmount {
            who: PlayerRel::Opponent,
            filter: &Filter::Any,
            increase: true,
            amount: &YOUR_ARTIFACTS,
        }
    )];
    static TWO_GENERIC: Amount = Amount::Fixed(2);
    static SELF_REDUCE_2: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::AdjustSpellCostByAmount {
                who: PlayerRel::You,
                filter: &Filter::This,
                increase: false,
                amount: &TWO_GENERIC,
            },
            condition: None,
        }),
        spell!(&[Effect::GainLife {
            amount: Amount::Fixed(1),
        }]),
    ];
    static PLAIN_INSTANT: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let taxer = card(
        C0,
        face("Artifact Taxer", "{2}{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        TAX_ENCHANTMENT_AB,
    );
    let art1 = card(
        C1,
        face("Relic 1", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let art2 = card(
        C2,
        face("Relic 2", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let art3 = card(
        C3,
        face("Relic 3", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_reduced = card(
        C4,
        face("Opp Discount Spell", "{1}{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        SELF_REDUCE_2,
    );
    let opp_plain = card(
        C5,
        face("Opp Plain Instant", "{1}{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        PLAIN_INSTANT,
    );
    let my_spell = card(
        C6,
        face("My Instant", "{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        PLAIN_INSTANT,
    );

    let mut engine = bench(
        1415,
        vec![taxer, art1, art2, art3, opp_reduced, opp_plain, my_spell],
        [
            Seat::with(&[C0, C1, C2, C3]).holding(&[C6]),
            Seat::with(&[]).holding(&[C4, C5]),
        ],
    );
    to_main(&mut engine, me());
    let my_spell_id = the(&engine, ZoneLocation::Hand(me()), C6);
    // Taxer targets Opponent only, so my own spell has 0 increase!
    assert_eq!(
        offer(&engine)
            .1
            .spell_increase(my_spell_id, CastModeKind::Normal),
        0
    );

    // Move to opponent's main phase.
    to_main(&mut engine, them());
    let opp_red_id = the(&engine, ZoneLocation::Hand(them()), C4);
    let opp_plain_id = the(&engine, ZoneLocation::Hand(them()), C5);
    let (_, legal_them) = offer(&engine);
    assert_eq!(
        legal_them.spell_increase(opp_red_id, CastModeKind::Normal),
        3,
        "3 artifacts under controller tax opponent's spells by {{3}}"
    );
    assert_eq!(
        legal_them.spell_reduction(opp_red_id, CastModeKind::Normal),
        2
    );
    // CR 601.2f: {1}{G} + tax {3} = {4}{G}, then - reduction {2} = {2}{G} (3 mana total!).
    add_pool(&mut engine, them(), ManaColor::Green, 2);
    assert!(
        !offer(&engine).1.castable.contains(&opp_red_id),
        "{{2}} mana cannot pay {{2}}{{G}}"
    );
    add_pool(&mut engine, them(), ManaColor::Green, 1);
    assert!(offer(&engine).1.castable.contains(&opp_red_id));
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp_red_id })
        .expect("cast opp_reduced for {2}{G}");
    assert_eq!(pool_total(&engine, them()), 0);

    // While opp_reduced is on the stack, its ZoneRef::Stack self-static MUST NOT leak
    // onto opp_plain cast in response!
    let (_, legal_stack) = offer(&engine);
    assert_eq!(
        legal_stack.spell_reduction(opp_plain_id, CastModeKind::Normal),
        0,
        "ZoneRef::Stack self-static on stack must not leak onto another spell"
    );
    assert_eq!(
        legal_stack.spell_increase(opp_plain_id, CastModeKind::Normal),
        3
    );
}

#[test]
fn deeply_nested_amount_terminates_via_cycle_safe_depth_guard() {
    static A0: Amount = Amount::Fixed(1);
    static A1: Amount = Amount::Plus {
        base: &A0,
        offset: 1,
    };
    static A2: Amount = Amount::Plus {
        base: &A1,
        offset: 1,
    };
    static A3: Amount = Amount::Plus {
        base: &A2,
        offset: 1,
    };
    static A4: Amount = Amount::Plus {
        base: &A3,
        offset: 1,
    };
    static A5: Amount = Amount::Plus {
        base: &A4,
        offset: 1,
    };
    static A6: Amount = Amount::Plus {
        base: &A5,
        offset: 1,
    };
    static A7: Amount = Amount::Plus {
        base: &A6,
        offset: 1,
    };
    static A8: Amount = Amount::Plus {
        base: &A7,
        offset: 1,
    };
    static A9: Amount = Amount::Plus {
        base: &A8,
        offset: 1,
    };
    static A10: Amount = Amount::Plus {
        base: &A9,
        offset: 1,
    };
    static A11: Amount = Amount::Plus {
        base: &A10,
        offset: 1,
    };
    static A12: Amount = Amount::Plus {
        base: &A11,
        offset: 1,
    };
    static A13: Amount = Amount::Plus {
        base: &A12,
        offset: 1,
    };
    static A14: Amount = Amount::Plus {
        base: &A13,
        offset: 1,
    };
    static A15: Amount = Amount::Plus {
        base: &A14,
        offset: 1,
    };
    static A16: Amount = Amount::Plus {
        base: &A15,
        offset: 1,
    };
    static A17: Amount = Amount::Plus {
        base: &A16,
        offset: 1,
    };
    static A18: Amount = Amount::Plus {
        base: &A17,
        offset: 1,
    };
    static A19: Amount = Amount::Plus {
        base: &A18,
        offset: 1,
    };
    static DEEP_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Stack,
        layer: baylee_cards_dsl::Layer::Text,
        filter: Filter::This,
        modifier: Modifier::AdjustSpellCostByAmount {
            who: PlayerRel::You,
            filter: &Filter::This,
            increase: false,
            amount: &A19,
        },
        condition: None,
    })];

    let deep = card(
        C0,
        creature_face("Deep Probe", "{20}{G}", 2, 2),
        KeywordSet::EMPTY,
        DEEP_AB,
    );
    let mut engine = bench(
        1416,
        vec![deep],
        [Seat::with(&[]).holding(&[C0]), Seat::with(&[])],
    );
    to_main(&mut engine, me());
    let deep_id = the(&engine, ZoneLocation::Hand(me()), C0);
    // Terminates cleanly at the depth guard cap (16 frames -> 15 increments + 0 at cap = 15, < 20).
    let red = offer(&engine)
        .1
        .spell_reduction(deep_id, CastModeKind::Normal);
    assert!(
        red > 0 && red < 20,
        "depth guard capped evaluation cleanly at {red}"
    );
}

#[test]
fn bladegraft_aspirant_opens_payment_window_when_targeting_non_matching_with_untapped_land() {
    let aspirant = card(
        C0,
        creature_face("Bladegraft Aspirant Probe", "{2}{R}", 2, 3),
        KeywordSet::EMPTY,
        ASPIRANT_ABILITIES,
    );
    let sword = card(
        C1,
        equipment_face("Short Sword Probe", "{1}"),
        KeywordSet::EMPTY,
        EQUIP_1_ABILITIES,
    );
    let bear = card(
        C2,
        creature_face("Bear Probe", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut engine = bench(
        1417,
        vec![aspirant, sword, bear],
        [Seat::with(&[C0, C1, C2, forest()]), Seat::with(&[])],
    );
    to_main(&mut engine, me());
    let sword_id = the(&engine, ZoneLocation::Battlefield, C1);
    let bear_id = the(&engine, ZoneLocation::Battlefield, C2);
    let forest_id = the(&engine, ZoneLocation::Battlefield, forest());

    // Equip {1} is offered at {0} optimistically because Aspirant is a legal target.
    assert!(offer(&engine).1.abilities.contains(&(sword_id, 0)));

    // Activate Equip and choose Bear (non-matching) with 0 floating mana but 1 untapped Forest:
    // CR 602.2b payment window opens instead of reversing immediately.
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: sword_id,
                ability_index: 0,
            },
        )
        .expect("start equip");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_id],
                players: vec![],
            },
        )
        .expect("choose bear opens payment window");
    let (payer, legal) = offer(&engine);
    assert_eq!(payer, me());
    assert!(
        legal.abilities.contains(&(forest_id, 0)),
        "CR 602.2b payment window must offer untapped Forest to cover unreduced {{1}}"
    );
    // Tap Forest for {G}, then pass priority to finalize the payment and put Equip on stack.
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: forest_id,
                ability_index: 0,
            },
        )
        .expect("tap forest during payment window");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("finish activation payment");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sword_id).unwrap().attached_to,
        Some(bear_id),
        "Equip paid via CR 602.2b payment window must resolve and attach to Bear"
    );
    assert!(
        engine
            .state()
            .object(forest_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(pool_total(&engine, me()), 0);
}

#[test]
fn bury_in_books_reduces_only_for_attacking_creature_during_combat() {
    static BURY_ABILITIES: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Stack,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::ReduceCostIfTargets {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                target_filter: &Filter::ATTACKING_CREATURE,
                reduction: mana!("{2}"),
                abilities: false,
            },
            condition: None,
        }),
        spell!(
            &[Effect::bounce(TargetSpec::Object(&Filter::CREATURE))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ];

    let bury = card(
        C0,
        face("Bury in Books Probe", "{4}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BURY_ABILITIES,
    );
    let attacker = card(
        C1,
        creature_face("Attacker Probe", "{1}{G}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let bystander = card(
        C2,
        creature_face("Bystander Probe", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1418,
        vec![bury, attacker, bystander],
        [Seat::with(&[]).holding(&[C0]), Seat::with(&[C1, C2])],
    );
    // Start on opponent's turn main phase: no creature is attacking yet.
    to_main(&mut engine, them());
    let bury_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let attacker_id = the(&engine, ZoneLocation::Battlefield, C1);
    let bystander_id = the(&engine, ZoneLocation::Battlefield, C2);

    // Pass priority to me during their main phase -> {2}{U} cannot cast Bury (no attacker).
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them pass in main");
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    assert!(
        !offer(&engine).1.castable.contains(&bury_id),
        "outside combat with no attacking creature, {{2}}{{U}} cannot cast Bury in Books"
    );

    // Advance into combat; opponent declares Attacker Probe as attacker and leaves Bystander back.
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker_id, baylee_core::ids::Defender::Player(me()))],
            },
        )
        .expect("declare attacker");
    // Wait until me() gets priority in DeclareAttackers.
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );

    // Pool emptied across step change; add {2}{U}. Now Bury IS castable optimistically!
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    assert!(
        offer(&engine).1.castable.contains(&bury_id),
        "with an attacking creature on board, {{2}}{{U}} must make Bury in Books castable"
    );
    assert_eq!(
        offer(&engine)
            .1
            .spell_reduction(bury_id, CastModeKind::Normal),
        0,
        "Bystander is also a legal non-attacking target, so guaranteed reduction stays 0"
    );

    // Choosing non-attacking Bystander with only {2}{U} reverses cleanly.
    engine
        .apply(me(), PlayerAction::CastSpell { card: bury_id })
        .expect("cast Bury");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bystander_id],
                players: vec![],
            },
        )
        .expect("target non-attacking bystander -> reverses");
    assert_eq!(
        engine.state().object(bury_id).unwrap().zone,
        crate::zone::Zone::Hand
    );

    // Choosing Attacking creature succeeds for {2}{U}!
    engine
        .apply(me(), PlayerAction::CastSpell { card: bury_id })
        .expect("cast Bury targeting attacker");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![attacker_id],
                players: vec![],
            },
        )
        .expect("target attacking creature");
    settle(&mut engine, them());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(attacker_id).unwrap().zone,
        crate::zone::Zone::Hand
    );
}

#[test]
fn board_static_target_dependent_spell_reduction_applies_to_matching_spells() {
    static BOARD_REDUCER_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ReduceCostIfTargets {
            who: PlayerRel::You,
            spell_filter: &Filter::INSTANT,
            target_filter: &Filter::ControlledByYou,
            reduction: mana!("{1}{U}"),
            abilities: false,
        }
    )];
    static PLAIN_BOUNCE_AB: &[AbilityDef] = &[spell!(
        &[Effect::bounce(TargetSpec::Object(&Filter::NONLAND))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND)))
    )];

    let reducer = card(
        C0,
        face("Board Spell Target Reducer", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        BOARD_REDUCER_AB,
    );
    let instant = card(
        C1,
        face("Plain Instant Bounce", "{1}{U}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        PLAIN_BOUNCE_AB,
    );
    let sorcery = card(
        C2,
        face("Plain Sorcery Bounce", "{1}{U}{U}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        PLAIN_BOUNCE_AB,
    );
    let opp_bear = card(
        C3,
        creature_face("Opp Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1419,
        vec![reducer, instant, sorcery, opp_bear],
        [Seat::with(&[C0]).holding(&[C1, C2]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());
    let reducer_id = the(&engine, ZoneLocation::Battlefield, C0);
    let instant_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let sorcery_id = the(&engine, ZoneLocation::Hand(me()), C2);

    // Float only {U}: Instant matches `spell_filter: INSTANT` and reducer_id is a legal
    // ControlledByYou target -> castable for {U}! Sorcery does NOT match -> not castable.
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&instant_id),
        "board-static ReduceCostIfTargets must make matching Instant castable for {{U}}"
    );
    assert!(
        !legal.castable.contains(&sorcery_id),
        "board-static ReduceCostIfTargets with spell_filter INSTANT must NOT reduce Sorcery"
    );

    // Cast Instant targeting our own reducer enchantment -> pays {U} and bounces it!
    engine
        .apply(me(), PlayerAction::CastSpell { card: instant_id })
        .expect("cast instant off board-static target reduction");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![reducer_id],
                players: vec![],
            },
        )
        .expect("target own enchantment");
    settle(&mut engine, me());
    assert_eq!(pool_total(&engine, me()), 0);
    assert_eq!(
        engine.state().object(reducer_id).unwrap().zone,
        crate::zone::Zone::Hand
    );
}
