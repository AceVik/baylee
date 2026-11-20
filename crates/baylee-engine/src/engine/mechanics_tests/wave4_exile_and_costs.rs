//! Wave 4 engine rule tests:
//! - Impulsive draw and temporary exile play/cast permissions (`CR 609.4`,
//!   `CR 609.4b`, `CR 601.1a`, `CR 305.9`, `CR 610.3a–b`):
//!   `Effect::ExileTopWithPermission`, `Effect::ExileWithPermission`,
//!   `Effect::GrantLinkedExilePermission`, `PlayWindow::WhileSourceOnBattlefield`,
//!   `ExilePermission::{play, cast, without_paying_mana_cost, spend_mana_as_any_color}`.
//! - `Bargain` (`CR 702.166a–c`): optional sacrifice of an artifact, enchantment,
//!   or token + `Condition::Bargained` on spells and ETB triggers, independent of
//!   `Condition::Kicked`.
//! - `Emerge` (`CR 702.119a–c`): creature and quality-filtered (`Emerge::from`)
//!   alternate casting cost, generic mana reduction by sacrificed permanent's
//!   mana value (after taxes per `CR 601.2f`), pool-only fodder filtering, and
//!   distinct fodder when combined with `Bargain`.
//! - `Prototype` (`CR 702.160`, `CR 719.3`): copiable characteristics on stack,
//!   battlefield, and `Clone` copies (`Condition::Prototyped`), reversion in
//!   other zones (`CR 719.4`), and free-cast isolation.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, YesNoPrompt};
use crate::turn::Phase;
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, Emerge, ExilePermission, ExileUntil, Filter, Modifier, PlayWindow,
    PlayerRel, Prototype, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::mana::{ManaColor, ManaPool, ManaSpending};
use baylee_core::types::SupertypeSet;

const IMPULSE_TWO: u32 = 8400;
const CAST_ONLY_ONE: u32 = 8401;
const FREE_EXILE_ONE: u32 = 8402;
const HOST_IMPULSE: u32 = 8403;
const ANY_COLOR_STEAL: u32 = 8404;
const HOSTAGE_PIRATE: u32 = 8405;
const LINK_ARCHIVE: u32 = 8406;
const BLUE_BOLT: u32 = 8407;
const COLORLESS_RAY: u32 = 8408;
const BIG_BLAST: u32 = 8409;
const BLINK_INSTANT: u32 = 8410;
const KILL_INSTANT: u32 = 8411;
const BARGAIN_BURN: u32 = 8412;
const BARGAIN_KNIGHT: u32 = 8413;
const TRINKET_ROCK: u32 = 8414;
const AURA_SEAL: u32 = 8415;
const TOKEN_MAKER: u32 = 8416;
const VANILLA_BEAR: u32 = 8417;
const EMERGE_LEVIATHAN: u32 = 8418;
const EMERGE_CRAB: u32 = 8419;
const SMALL_ELF: u32 = 8420;
const FOUR_DROP_OX: u32 = 8421;
const FIVE_DROP_GOLEM: u32 = 8422;
const TAX_PILLAR: u32 = 8423;
const EMERGE_BARGAIN_WURM: u32 = 8424;
const PROTO_CONSTRUCT: u32 = 8425;
const PROTO_CLONE: u32 = 8426;

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn put_on_top(engine: &mut Bench, owner: PlayerId, card_id: ObjectId) {
    engine
        .state
        .move_object(
            card_id,
            ZoneLocation::Library(owner),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move to library top");
}

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    refresh_offer(engine);
}

/// Slice 1a: `Effect::ExileTopWithPermission` handles `ThroughYourNextTurn`
/// (land play this turn + spell cast next turn + expiry at next turn's cleanup),
/// `cast_only: true` refusing land plays (`CR 305.9`), and `free: true` casting
/// without paying mana cost (`CR 118.9`).
#[test]
fn exile_top_with_permission_supports_play_vs_cast_only_windows_and_free_casting() {
    static IMPULSE_ABILS: [AbilityDef; 1] = [spell!(&[Effect::ExileTopWithPermission {
        who: PlayerRel::You,
        count: Amount::Fixed(3),
        permission: ExilePermission::play(PlayWindow::ThroughYourNextTurn),
    }])];
    static CAST_ONLY_ABILS: [AbilityDef; 1] = [spell!(&[Effect::ExileTopWithPermission {
        who: PlayerRel::You,
        count: Amount::Fixed(1),
        permission: ExilePermission::cast(PlayWindow::ThisTurn),
    }])];
    static FREE_CAST_ABILS: [AbilityDef; 1] = [spell!(&[Effect::ExileTopWithPermission {
        who: PlayerRel::You,
        count: Amount::Fixed(1),
        permission: ExilePermission::cast(PlayWindow::ThisTurn).without_paying_mana_cost(),
    }])];
    static BLAST_ABILS: [AbilityDef; 1] = [spell!(&[Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(4),
    }])];

    let impulse = card(
        IMPULSE_TWO,
        face("Light the Way", "{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &IMPULSE_ABILS,
    );
    let cast_only = card(
        CAST_ONLY_ONE,
        face("Spark Glance", "{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &CAST_ONLY_ABILS,
    );
    let free_exile = card(
        FREE_EXILE_ONE,
        face("Wild Discovery", "{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &FREE_CAST_ABILS,
    );
    let blast = card(
        BIG_BLAST,
        face("Meteor Volleys", "{5}{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &BLAST_ABILS,
    );

    let mut engine = bench(
        8400,
        vec![impulse, cast_only, free_exile, blast],
        [
            Seat::default().holding(&[
                IMPULSE_TWO,
                CAST_ONLY_ONE,
                FREE_EXILE_ONE,
                BIG_BLAST,
                BIG_BLAST,
                BIG_BLAST,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let blasts = objects(&engine, ZoneLocation::Hand(me()), BIG_BLAST);
    let (blast_a, blast_b, blast_c) = (blasts[0], blasts[1], blasts[2]);
    let lib_forest = *engine
        .state
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    put_on_top(&mut engine, me(), blast_a);
    put_on_top(&mut engine, me(), blast_b);
    put_on_top(&mut engine, me(), lib_forest);

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let imp_id = the(&engine, ZoneLocation::Hand(me()), IMPULSE_TWO);
    engine
        .apply(me(), PlayerAction::CastSpell { card: imp_id })
        .expect("cast impulse");
    settle(&mut engine, me());

    let (_, legal) = offer(&engine);
    assert!(
        legal.lands.contains(&lib_forest),
        "ExilePermission::play allows playing exiled land"
    );
    engine
        .apply(me(), PlayerAction::PlayLand { card: lib_forest })
        .expect("play exiled land");
    assert_eq!(
        engine.state.object(lib_forest).unwrap().zone,
        Zone::Battlefield
    );

    // Advance to seat 0's NEXT main phase (turn 3): ThroughYourNextTurn is still active!
    walk_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == me()
            && e.state().turn.phase == Phase::FirstMain
            && e.state().zones.stack_is_empty()
    });
    add_pool(&mut engine, me(), ManaColor::Red, 6);
    let (_, legal_t3) = offer(&engine);
    assert!(
        legal_t3.castable.contains(&blast_b),
        "ThroughYourNextTurn remains playable on the caster's next turn"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: blast_b })
        .expect("cast exiled blast on next turn");
    settle(&mut engine, me());
    assert_eq!(engine.state.players[1].life, 16);

    // Advance past seat 0's turn 3 cleanup to turn 5: blast_a's ThroughYourNextTurn has expired!
    walk_until(&mut engine, |e| {
        e.state().turn.number >= 5
            && e.state().turn.active == me()
            && e.state().turn.phase == Phase::FirstMain
            && e.state().zones.stack_is_empty()
    });
    add_pool(&mut engine, me(), ManaColor::Red, 6);
    let (_, legal_t5) = offer(&engine);
    assert!(
        !legal_t5.castable.contains(&blast_a),
        "ThroughYourNextTurn must expire at cleanup of caster's next turn"
    );

    // Now test cast_only: true on a Land (lib_forest_2 on top) -> cannot play as land!
    let lib_forest_2 = *engine
        .state
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    let cast_only_id = the(&engine, ZoneLocation::Hand(me()), CAST_ONLY_ONE);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: cast_only_id })
        .expect("cast cast-only impulse");
    settle(&mut engine, me());
    let (_, legal_cast_only) = offer(&engine);
    assert!(
        !legal_cast_only.lands.contains(&lib_forest_2),
        "ExilePermission::cast (cast_only: true) must NOT allow playing an exiled land (CR 305.9)"
    );

    // Now test without_paying_mana_cost() on blast_c ({5}{R} cast with 0 mana left in pool)!
    put_on_top(&mut engine, me(), blast_c);
    engine.state.players[0].mana_pool = ManaPool::new();
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let free_id = the(&engine, ZoneLocation::Hand(me()), FREE_EXILE_ONE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: free_id })
        .expect("cast free-exile spell");
    settle(&mut engine, me());
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    let (_, legal_free) = offer(&engine);
    assert!(
        legal_free.castable.contains(&blast_c),
        "without_paying_mana_cost makes {{5}}{{R}} spell castable with 0 mana"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: blast_c })
        .expect("cast exiled spell without paying mana cost");
    settle(&mut engine, me());
    assert_eq!(engine.state.players[1].life, 12);
}

/// Slice 1b: `PlayWindow::WhileSourceOnBattlefield` keeps the exiled card playable
/// across turns while the granting permanent remains on the battlefield, and
/// immediately revokes permission when that permanent leaves or blinks (CR 400.7).
#[test]
fn exile_top_with_while_source_on_battlefield_expires_on_source_departure_or_blink() {
    static HOST_ABILS: [AbilityDef; 1] = [free(
        &[Effect::ExileTopWithPermission {
            who: PlayerRel::You,
            count: Amount::Fixed(1),
            permission: ExilePermission::play(PlayWindow::WhileSourceOnBattlefield),
        }],
        None,
    )];
    static BLINK_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::blink_to_owner(TargetSpec::Object(
            &Filter::CREATURE
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static BOLT_ABILS: [AbilityDef; 1] = [spell!(&[Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(2),
    }])];

    let host = card(
        HOST_IMPULSE,
        creature_face("Archive Dragon", "{3}{R}", 3, 3),
        KeywordSet::EMPTY,
        &HOST_ABILS,
    );
    let blink = card(
        BLINK_INSTANT,
        face("Cloud Flicker", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &BLINK_ABILS,
    );
    let bolt = card(
        BLUE_BOLT,
        face("Mind Spark", "{U}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &BOLT_ABILS,
    );

    let mut engine = bench(
        8401,
        vec![host, blink, bolt],
        [
            Seat::with(&[HOST_IMPULSE]).holding(&[BLINK_INSTANT, BLUE_BOLT, BLUE_BOLT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let host_id = the(&engine, ZoneLocation::Battlefield, HOST_IMPULSE);
    let bolts = objects(&engine, ZoneLocation::Hand(me()), BLUE_BOLT);
    let (bolt_1, bolt_2) = (bolts[0], bolts[1]);
    put_on_top(&mut engine, me(), bolt_2);
    put_on_top(&mut engine, me(), bolt_1);

    for _ in 0..2 {
        engine
            .apply(
                me(),
                PlayerAction::ActivateAbility {
                    source: host_id,
                    ability_index: 0,
                },
            )
            .expect("activate host");
        settle(&mut engine, me());
    }

    // Advance two full turns (turn 5): host is still on battlefield, so bolt_1 is still castable!
    walk_until(&mut engine, |e| {
        e.state().turn.number >= 5
            && e.state().turn.active == me()
            && e.state().turn.phase == Phase::FirstMain
            && e.state().zones.stack_is_empty()
    });
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let (_, legal_before_blink) = offer(&engine);
    assert!(
        legal_before_blink.castable.contains(&bolt_1)
            && legal_before_blink.castable.contains(&bolt_2),
        "WhileSourceOnBattlefield persists across turns while source stays"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt_1 })
        .expect("cast bolt_1");
    settle(&mut engine, me());
    assert_eq!(engine.state.players[1].life, 18);

    // Blink the host: it returns as a new object (CR 400.7), revoking bolt_2's permission!
    add_pool(&mut engine, me(), ManaColor::White, 1);
    let blink_id = the(&engine, ZoneLocation::Hand(me()), BLINK_INSTANT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: blink_id })
        .expect("cast blink");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![host_id],
            },
        )
        .expect("target host");
    settle(&mut engine, me());

    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let (_, legal_after_blink) = offer(&engine);
    assert!(
        !legal_after_blink.castable.contains(&bolt_2),
        "blinking the granting source must revoke WhileSourceOnBattlefield (CR 400.7)"
    );
}

/// Slice 1c: `ExilePermission::spend_mana_as_any_color` (`CR 609.4b`) allows any
/// mana to pay colored requirements (`{1}{U}{U}` paid with `{R}{R}{R}`) while
/// keeping colorless `{C}` strict (`{1}{C}` rejected on `{R}{R}`, accepted on
/// `{R}{C}`), and publishes `spell_spending` on `LegalActions`.
#[test]
fn exile_permission_spend_mana_as_any_color_pays_colored_pips_but_not_colorless_cr_609_4b() {
    static STEAL_ABILS: [AbilityDef; 1] = [spell!(&[Effect::ExileTopWithPermission {
        who: PlayerRel::EachOpponent,
        count: Amount::Fixed(2),
        permission: ExilePermission::cast(PlayWindow::WhileExiled).spend_mana_as_any_color(),
    }])];
    static BLUE_ABILS: [AbilityDef; 1] = [spell!(&[Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(3),
    }])];
    static CLESS_ABILS: [AbilityDef; 1] = [spell!(&[Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(5),
    }])];

    let steal = card(
        ANY_COLOR_STEAL,
        face("Siphon Memory", "{B}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &STEAL_ABILS,
    );
    let blue_spell = card(
        BLUE_BOLT,
        face("Azure Lance", "{1}{U}{U}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &BLUE_ABILS,
    );
    let colorless_spell = card(
        COLORLESS_RAY,
        face("Null Ray", "{1}{C}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &CLESS_ABILS,
    );

    let mut engine = bench(
        8402,
        vec![steal, blue_spell, colorless_spell],
        [
            Seat::with(&[forest(), forest(), forest()]).holding(&[ANY_COLOR_STEAL]),
            Seat::default().holding(&[BLUE_BOLT, COLORLESS_RAY]),
        ],
    );
    to_main(&mut engine, me());

    let opp_blue = the(&engine, ZoneLocation::Hand(them()), BLUE_BOLT);
    let opp_cless = the(&engine, ZoneLocation::Hand(them()), COLORLESS_RAY);
    put_on_top(&mut engine, them(), opp_cless);
    put_on_top(&mut engine, them(), opp_blue);

    add_pool(&mut engine, me(), ManaColor::Black, 1);
    let steal_id = the(&engine, ZoneLocation::Hand(me()), ANY_COLOR_STEAL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: steal_id })
        .expect("cast steal");
    settle(&mut engine, me());

    let (_, legal_untapped) = offer(&engine);
    assert!(
        legal_untapped.payable.contains(&opp_blue),
        "3 untapped Forests can pay {{1}}{{U}}{{U}} under SpendManaAsAnyColor"
    );
    assert_eq!(
        legal_untapped
            .spell_spending_for(crate::choice::Spend::cast(opp_blue, CastModeKind::Normal)),
        Some(ManaSpending::ANY_COLOR),
        "LegalActions publishes ManaSpending::ANY_COLOR for planners"
    );

    add_pool(&mut engine, me(), ManaColor::Red, 3);
    let (_, legal_red3) = offer(&engine);
    assert!(
        legal_red3.castable.contains(&opp_blue),
        "{{R}}{{R}}{{R}} pays {{1}}{{U}}{{U}} under SpendManaAsAnyColor (CR 609.4b)"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: opp_blue })
        .expect("cast stolen blue spell with red mana");
    settle(&mut engine, me());
    assert_eq!(engine.state.players[1].life, 17);

    float_all(&mut engine, me());
    engine.state.players[0].mana_pool = ManaPool::new();
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    let (_, legal_red2) = offer(&engine);
    assert!(
        !legal_red2.castable.contains(&opp_cless),
        "colored mana must NOT pay {{C}} under SpendManaAsAnyColor (CR 609.4b)"
    );

    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal_with_c) = offer(&engine);
    assert!(
        legal_with_c.castable.contains(&opp_cless),
        "{{R}} + {{C}} pays {{1}}{{C}}"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: opp_cless })
        .expect("cast {1}{C} spell");
    settle(&mut engine, me());
    assert_eq!(engine.state.players[1].life, 12);
}

/// Slice 1d: `Effect::ExileWithPermission` (with `ExileUntil::SourceLeavesBattlefield`
/// and `CR 610.3a–b` host-already-left guard) and `Effect::GrantLinkedExilePermission`.
#[test]
fn exile_with_permission_and_grant_linked_exile_permission_handle_targets_and_until_host_leaves() {
    static PIRATE_ABILS: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::ExileWithPermission {
            target: TargetSpec::Object(&Filter::OPPONENT_CREATURE),
            until: Some(ExileUntil::SourceLeavesBattlefield),
            permission: ExilePermission::cast(PlayWindow::WhileExiled).spend_mana_as_any_color(),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::OPPONENT_CREATURE
        )))
    )];
    static KILL_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static ARCHIVE_ABILS: [AbilityDef; 2] = [
        free(
            &[Effect::ExileLinked {
                target: TargetSpec::Object(&Filter::CREATURE),
                until: None,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            &[Effect::GrantLinkedExilePermission {
                permission: ExilePermission::cast(PlayWindow::ThisTurn).without_paying_mana_cost(),
            }],
            None,
        ),
    ];

    let pirate = card(
        HOSTAGE_PIRATE,
        creature_face("Hostage Corsair", "{2}{U}{B}", 2, 3),
        KeywordSet::EMPTY,
        &PIRATE_ABILS,
    );
    let kill = card(
        KILL_INSTANT,
        face("Swift End", "{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &KILL_ABILS,
    );
    let archive = card(
        LINK_ARCHIVE,
        FaceDef {
            mana_cost: ManaCost::parse("{3}"),
            types: TypeSet::ARTIFACT,
            castable_from_hand: true,
            ..land_face("Soul Vault")
        },
        KeywordSet::EMPTY,
        &ARCHIVE_ABILS,
    );
    let bear = card(
        VANILLA_BEAR,
        creature_face("Forest Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        8403,
        vec![pirate, kill, archive, bear],
        [
            Seat::with(&[LINK_ARCHIVE]).holding(&[HOSTAGE_PIRATE, HOSTAGE_PIRATE]),
            Seat::with(&[VANILLA_BEAR, VANILLA_BEAR]).holding(&[KILL_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    let opp_bears = objects(&engine, ZoneLocation::Battlefield, VANILLA_BEAR);
    let (bear_1, bear_2) = (opp_bears[0], opp_bears[1]);

    // 1) CR 610.3b check: cast first Pirate, target bear_1 on ETB, opponent kills Pirate in response -> bear_1 never moves!
    let pirates = objects(&engine, ZoneLocation::Hand(me()), HOSTAGE_PIRATE);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pirates[0] })
        .expect("cast pirate 1");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_1],
            },
        )
        .expect("target bear_1");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    add_pool(&mut engine, them(), ManaColor::Black, 1);
    let kill_id = the(&engine, ZoneLocation::Hand(them()), KILL_INSTANT);
    engine
        .apply(them(), PlayerAction::CastSpell { card: kill_id })
        .expect("cast kill");
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![pirates[0]],
            },
        )
        .expect("target pirate 1");
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(bear_1).unwrap().zone,
        Zone::Battlefield,
        "CR 610.3b: bear_1 stays on battlefield when Pirate dies before ETB resolves"
    );

    // 2) Cast second Pirate -> exiles bear_1 until Pirate leaves, and P0 casts bear_1 ({1}{G}) with {R}{R}!
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pirates[1] })
        .expect("cast pirate 2");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_1],
            },
        )
        .expect("target bear_1");
    settle(&mut engine, me());
    assert_eq!(engine.state.object(bear_1).unwrap().zone, Zone::Exile);

    add_pool(&mut engine, me(), ManaColor::Red, 2);
    let (_, legal_cast) = offer(&engine);
    assert!(
        legal_cast.castable.contains(&bear_1),
        "ExileWithPermission grants SpendManaAsAnyColor cast from exile"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: bear_1 })
        .expect("cast hostage bear_1 with red mana");
    settle(&mut engine, me());
    assert_eq!(engine.state.object(bear_1).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state.object(bear_1).unwrap().controller, me());

    // 3) GrantLinkedExilePermission on Soul Vault: ability 0 exiles bear_2 linked, ability 1 grants free cast!
    let archive_id = the(&engine, ZoneLocation::Battlefield, LINK_ARCHIVE);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: archive_id,
                ability_index: 0,
            },
        )
        .expect("exile bear_2 into linked exile");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_2],
            },
        )
        .expect("target bear_2");
    settle(&mut engine, me());
    assert!(!offer(&engine).1.castable.contains(&bear_2));

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: archive_id,
                ability_index: 1,
            },
        )
        .expect("grant linked exile permission");
    settle(&mut engine, me());
    assert!(
        offer(&engine).1.castable.contains(&bear_2),
        "GrantLinkedExilePermission opens free cast on linked exiled card"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: bear_2 })
        .expect("cast bear_2 for free");
    settle(&mut engine, me());
    assert_eq!(engine.state.object(bear_2).unwrap().controller, me());
}

/// Slice 2a: `Bargain` (`CR 702.166a–c`) prompts `YesNoPrompt::Bargain` only when
/// an artifact, enchantment, or token is controlled, sacrifices the chosen fodder,
/// sets `Condition::Bargained` on the spell, and never sets `Condition::Kicked`.
#[test]
fn bargain_sacrifices_artifact_enchantment_or_token_and_sets_bargained_without_kicking() {
    static BURN_EFFECT_BASE: [Effect; 1] = [Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(2),
    }];
    static BURN_BONUS_BARGAIN: [Effect; 1] = [Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(3),
    }];
    static BURN_BONUS_KICK: [Effect; 1] = [Effect::GainLife {
        amount: Amount::Fixed(9),
    }];
    static BURN_ABILS: [AbilityDef; 1] = [spell!(&[
        BURN_EFFECT_BASE[0],
        Effect::IfCondition {
            condition: Condition::Bargained,
            then: &BURN_BONUS_BARGAIN,
            otherwise: &[],
        },
        Effect::IfKicked {
            then: &BURN_BONUS_KICK,
            otherwise: &[],
        },
    ])];
    static SOLDIER_TOKEN: TokenDef = TokenDef {
        scryfall_id: "test",
        name: "Soldier",
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
        defense: None,
        colors: ColorSet::from_slice(&[Color::White]),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        keywords: KeywordSet::EMPTY,
        abilities: &[],
    };
    static MAKE_TOKEN_ABILS: [AbilityDef; 1] = [free(
        &[Effect::CreateToken {
            token: &SOLDIER_TOKEN,
        }],
        None,
    )];

    let burn = card(
        BARGAIN_BURN,
        FaceDef {
            bargain: true,
            ..face("Torch the Tower", "{R}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        &BURN_ABILS,
    );
    let rock = card(
        TRINKET_ROCK,
        FaceDef {
            mana_cost: ManaCost::parse("{1}"),
            types: TypeSet::ARTIFACT,
            castable_from_hand: true,
            ..land_face("Copper Trinket")
        },
        KeywordSet::EMPTY,
        &[],
    );
    let seal = card(
        AURA_SEAL,
        FaceDef {
            mana_cost: ManaCost::parse("{1}{W}"),
            types: TypeSet::ENCHANTMENT,
            castable_from_hand: true,
            ..land_face("Oath Seal")
        },
        KeywordSet::EMPTY,
        &[],
    );
    let maker = card(
        TOKEN_MAKER,
        creature_face("Banner Squire", "{1}{W}", 2, 2),
        KeywordSet::EMPTY,
        &MAKE_TOKEN_ABILS,
    );

    let mut engine = bench(
        8404,
        vec![burn, rock, seal, maker],
        [
            Seat::with(&[TOKEN_MAKER]).holding(&[
                BARGAIN_BURN,
                BARGAIN_BURN,
                BARGAIN_BURN,
                TRINKET_ROCK,
                AURA_SEAL,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let burns = objects(&engine, ZoneLocation::Hand(me()), BARGAIN_BURN);

    // 1) Only nontoken creature TOKEN_MAKER on battlefield -> YesNoPrompt::Bargain is NOT asked!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: burns[0] })
        .expect("cast burn without bargain fodder");
    assert!(
        !matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Bargain,
                ..
            }
        ),
        "Bargain must be skipped when no artifact, enchantment, or token can be sacrificed"
    );
    settle(&mut engine, me());
    assert_eq!(engine.state.players[1].life, 18, "base 2 life loss only");

    // 2) Create a 1/1 creature token -> Bargain IS asked; decline it first -> 2 life loss, token kept.
    let maker_id = the(&engine, ZoneLocation::Battlefield, TOKEN_MAKER);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: maker_id,
                ability_index: 0,
            },
        )
        .expect("create token");
    settle(&mut engine, me());
    let token_id = engine
        .state
        .battlefield_view()
        .iter()
        .copied()
        .find(|&id| !engine.state.object(id).unwrap().is_card())
        .expect("soldier token on battlefield");

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: burns[1] })
        .expect("cast second burn");
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Bargain,
                ..
            }
        ),
        "Bargain prompt appears when a token is controlled"
    );
    engine
        .apply(me(), PlayerAction::YesNo(false))
        .expect("decline bargain");
    settle(&mut engine, me());
    assert_eq!(engine.state.players[1].life, 16, "unbargained deals 2");
    assert!(engine.state.battlefield_view().contains(&token_id));

    // 3) Cast rock & seal, then cast third burn accepting Bargain -> offers token, rock, and seal (NOT maker_id)!
    add_pool(&mut engine, me(), ManaColor::White, 3);
    let rock_id = the(&engine, ZoneLocation::Hand(me()), TRINKET_ROCK);
    let seal_id = the(&engine, ZoneLocation::Hand(me()), AURA_SEAL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: rock_id })
        .unwrap();
    settle(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: seal_id })
        .unwrap();
    settle(&mut engine, me());

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: burns[2] })
        .expect("cast third burn");
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept bargain");
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards sacrifice prompt, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(options.contains(&token_id));
    assert!(options.contains(&rock_id));
    assert!(options.contains(&seal_id));
    assert!(
        !options.contains(&maker_id),
        "nontoken non-artifact non-enchantment creature must NOT be offered for Bargain"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![seal_id],
            },
        )
        .expect("sacrifice enchantment for bargain");
    settle(&mut engine, me());
    assert_eq!(engine.state.object(seal_id).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state.players[1].life, 11,
        "bargained burn deals 2 + 3 = 5"
    );
    assert_eq!(
        engine.state.players[0].life, 20,
        "Bargain must NOT trigger IfKicked (still 20 life, not 29)"
    );
}

/// Slice 2b: `Condition::Bargained` works on permanent ETB triggers (including
/// intervening-if) and resets when the permanent blinks (`CR 702.166b`, `CR 400.7`).
#[test]
fn bargain_permanent_etb_intervening_if_and_blink_reset_cr_702_166() {
    static KNIGHT_ABILS: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }],
        condition = Some(Condition::Bargained)
    )];
    static BLINK_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::blink_to_owner(TargetSpec::Object(
            &Filter::CREATURE
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let knight = card(
        BARGAIN_KNIGHT,
        FaceDef {
            bargain: true,
            ..creature_face("Fae Cavalier", "{2}{W}", 3, 2)
        },
        KeywordSet::EMPTY,
        &KNIGHT_ABILS,
    );
    let rock = card(
        TRINKET_ROCK,
        FaceDef {
            mana_cost: ManaCost::parse("{1}"),
            types: TypeSet::ARTIFACT,
            castable_from_hand: true,
            ..land_face("IronShard")
        },
        KeywordSet::EMPTY,
        &[],
    );
    let blink = card(
        BLINK_INSTANT,
        face("Step Away", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &BLINK_ABILS,
    );

    let mut engine = bench(
        8405,
        vec![knight, rock, blink],
        [
            Seat::with(&[TRINKET_ROCK]).holding(&[BARGAIN_KNIGHT, BLINK_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let rock_id = the(&engine, ZoneLocation::Battlefield, TRINKET_ROCK);
    let knight_id = the(&engine, ZoneLocation::Hand(me()), BARGAIN_KNIGHT);
    add_pool(&mut engine, me(), ManaColor::White, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: knight_id })
        .expect("cast bargain knight");
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("bargain");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![rock_id],
            },
        )
        .expect("sacrifice rock");
    settle(&mut engine, me());
    assert_eq!(
        engine.state.players[0].life, 24,
        "ETB intervening-if Condition::Bargained fired (+4 life)"
    );

    // Blink the knight -> enters unbargained, life stays 24!
    add_pool(&mut engine, me(), ManaColor::White, 1);
    let blink_id = the(&engine, ZoneLocation::Hand(me()), BLINK_INSTANT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: blink_id })
        .expect("cast blink");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![knight_id],
            },
        )
        .expect("target knight");
    settle(&mut engine, me());
    assert_eq!(
        engine.state.players[0].life, 24,
        "blinked permanent was not bargained (CR 400.7)"
    );
}

/// Slice 2c: `Emerge` (`CR 702.119a–c`) reduces generic mana by sacrificed
/// permanent's MV (including Thalia taxes per `CR 601.2f`), filters out
/// insufficient-MV fodder when paying from pool alone, supports `Emerge::from`
/// (`CR 702.119b`), and requires distinct permanents when combined with `Bargain`.
#[test]
fn emerge_reduces_generic_cost_by_sacrificed_mv_pays_taxes_and_filters_pool_fodder() {
    static TAX_ABILS: [AbilityDef; 1] = [static_ability!(Filter::Any, Modifier::SpellsCostMore(1))];

    let leviathan = card(
        EMERGE_LEVIATHAN,
        FaceDef {
            emerge: Some(Emerge::creature(ManaCost::parse("{5}{U}{U}"))),
            ..creature_face("Trench Fiend", "{8}", 5, 6)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let crab = card(
        EMERGE_CRAB,
        FaceDef {
            emerge: Some(Emerge::from(ManaCost::parse("{4}{B}"), &Filter::ARTIFACT)),
            ..creature_face("Scrap Shell", "{6}", 4, 4)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let wurm = card(
        EMERGE_BARGAIN_WURM,
        FaceDef {
            bargain: true,
            emerge: Some(Emerge::creature(ManaCost::parse("{3}{G}"))),
            ..creature_face("Greedy Wurm", "{7}", 5, 5)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let elf = card(
        SMALL_ELF,
        creature_face("Tiny Scout", "{G}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let ox = card(
        FOUR_DROP_OX,
        creature_face("Stout Ox", "{3}{G}", 3, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let golem = card(
        FIVE_DROP_GOLEM,
        FaceDef {
            types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            ..creature_face("Iron Colossus", "{5}", 4, 5)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let pillar = card(
        TAX_PILLAR,
        FaceDef {
            mana_cost: ManaCost::parse("{2}"),
            types: TypeSet::ARTIFACT,
            castable_from_hand: true,
            ..land_face("Sphere of Duty")
        },
        KeywordSet::EMPTY,
        &TAX_ABILS,
    );

    let mut engine = bench(
        8406,
        vec![leviathan, crab, wurm, elf, ox, golem, pillar],
        [
            Seat::with(&[SMALL_ELF, FOUR_DROP_OX]).holding(&[
                EMERGE_LEVIATHAN,
                EMERGE_LEVIATHAN,
                EMERGE_CRAB,
                EMERGE_BARGAIN_WURM,
                FIVE_DROP_GOLEM,
                TAX_PILLAR,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let elf_id = the(&engine, ZoneLocation::Battlefield, SMALL_ELF);
    let ox_id = the(&engine, ZoneLocation::Battlefield, FOUR_DROP_OX);
    let leviathans = objects(&engine, ZoneLocation::Hand(me()), EMERGE_LEVIATHAN);

    // 1) Pool has {1}{U}{U} (3 mana), 0 lands: SMALL_ELF (MV 1 -> needs {4}{U}{U}) cannot pay,
    //    FOUR_DROP_OX (MV 4 -> needs {1}{U}{U}) CAN pay!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&leviathans[0]),
        "Emerge {{5}}{{U}}{{U}} reduced by MV 4 Ox is {{1}}{{U}}{{U}}, castable off 3 mana"
    );
    assert!(
        legal.cast_ways.iter().any(|(id, w)| *id == leviathans[0]
            && w.kind == CastModeKind::Emerge
            && w.cost == ManaCost::parse("{1}{U}{U}")),
        "cast_ways reports cheapest Emerge price {{1}}{{U}}{{U}}"
    );
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: leviathans[0],
            },
        )
        .expect("cast leviathan via emerge");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected emerge sacrifice choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        options,
        vec![ox_id],
        "pool-only emerge must filter out MV-1 elf that leaves {{4}}{{U}}{{U}} unpaid"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![ox_id],
            },
        )
        .expect("choose ox for emerge");
    settle(&mut engine, me());
    assert_eq!(engine.state.object(ox_id).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state.object(leviathans[0]).unwrap().zone,
        Zone::Battlefield
    );
    assert!(engine.state.object(leviathans[0]).unwrap().alt_cast);
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);

    // 2) Put FIVE_DROP_GOLEM (MV 5 artifact creature) and TAX_PILLAR (+{1} tax) onto battlefield:
    //    Emerge {5}{U}{U} + {1} tax - MV 5 = {1}{U}{U} (CR 601.2f)!
    add_pool(&mut engine, me(), ManaColor::Colorless, 7);
    let golem_id = the(&engine, ZoneLocation::Hand(me()), FIVE_DROP_GOLEM);
    let pillar_id = the(&engine, ZoneLocation::Hand(me()), TAX_PILLAR);
    engine
        .apply(me(), PlayerAction::CastSpell { card: golem_id })
        .unwrap();
    settle(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: pillar_id })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);

    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: leviathans[1],
            },
        )
        .expect("cast second leviathan under +1 tax using MV-5 golem");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![golem_id],
            },
        )
        .expect("sacrifice MV-5 golem");
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(leviathans[1]).unwrap().zone,
        Zone::Battlefield
    );

    // 3) Emerge from artifact (`CR 702.119b`): EMERGE_CRAB (`{4}{B}` + `{1}` tax = `{5}{B}`)
    //    sacrifices TAX_PILLAR (MV 2 artifact, NOT a creature!), paying `{3}{B}`!
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    let crab_id = the(&engine, ZoneLocation::Hand(me()), EMERGE_CRAB);
    engine
        .apply(me(), PlayerAction::CastSpell { card: crab_id })
        .expect("cast crab via emerge from artifact");
    let Pending::ChooseCards {
        options: crab_opts, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected emerge artifact choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        crab_opts,
        vec![pillar_id],
        "Emerge from artifact offers non-creature artifact and excludes non-artifact creatures"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![pillar_id],
            },
        )
        .expect("sacrifice artifact pillar");
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(crab_id).unwrap().zone,
        Zone::Battlefield
    );

    // 4) Emerge + Bargain on EMERGE_BARGAIN_WURM: no artifacts/enchantments/tokens remain on board ->
    //    after choosing a creature for Emerge, YesNoPrompt::Bargain is NOT asked!
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    let wurm_id = the(&engine, ZoneLocation::Hand(me()), EMERGE_BARGAIN_WURM);
    engine
        .apply(me(), PlayerAction::CastSpell { card: wurm_id })
        .expect("cast wurm via emerge");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![leviathans[0]],
            },
        )
        .expect("sacrifice MV-8 leviathan");
    assert!(
        !matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Bargain,
                ..
            }
        ),
        "Bargain is skipped when no distinct artifact/enchantment/token exists"
    );
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(wurm_id).unwrap().zone,
        Zone::Battlefield
    );
    let _ = elf_id;
}

/// Slice 2d: `Prototype` (`CR 702.160`, `CR 719.3`) sets copiable cost, color,
/// and P/T on stack and battlefield, propagates `Condition::Prototyped` to
/// `Clone` copies (`CR 719.3d`), reverts outside stack/battlefield (`CR 719.4`),
/// and cannot be combined with a `without paying its mana cost` permission.
#[test]
fn prototype_characteristics_clone_copy_and_free_cast_isolation_cr_719() {
    static PROTO_ABILS: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        condition = Some(Condition::Prototyped)
    )];
    static CLONE_ABILS: [AbilityDef; 1] = [AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[],
    }];
    static FREE_EXILE_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::ExileWithPermission {
            target: TargetSpec::Object(&Filter::CREATURE),
            until: None,
            permission: ExilePermission::cast(PlayWindow::ThisTurn).without_paying_mana_cost(),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let proto = card(
        PROTO_CONSTRUCT,
        FaceDef {
            prototype: Some(Prototype::new(ManaCost::parse("{1}{B}{B}"), 3, 3)),
            types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            ..creature_face("Dusk Automaton", "{7}", 7, 7)
        },
        KeywordSet::EMPTY,
        &PROTO_ABILS,
    );
    let clone_card = card(
        PROTO_CLONE,
        creature_face("Mirror Twin", "{3}{U}", 0, 0),
        KeywordSet::EMPTY,
        &CLONE_ABILS,
    );
    let exile_recast = card(
        FREE_EXILE_ONE,
        face("Phase Recast", "{W}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        &FREE_EXILE_ABILS,
    );

    let mut engine = bench(
        8407,
        vec![proto, clone_card, exile_recast],
        [
            Seat::default().holding(&[PROTO_CONSTRUCT, PROTO_CLONE, FREE_EXILE_ONE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    // 1) Cast PROTO_CONSTRUCT for {1}{B}{B} -> 3/3 Black MV 3, Condition::Prototyped fires (+3 life -> 23)!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    let proto_id = the(&engine, ZoneLocation::Hand(me()), PROTO_CONSTRUCT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: proto_id })
        .expect("cast prototyped");
    settle(&mut engine, me());

    let proto_obj = engine.state.object(proto_id).unwrap();
    assert!(proto_obj.prototyped);
    assert_eq!(proto_obj.characteristics().power, Some(3));
    assert_eq!(proto_obj.characteristics().toughness, Some(3));
    assert_eq!(proto_obj.characteristics().mana_value(), 3);
    assert_eq!(
        proto_obj.characteristics().colors,
        ColorSet::from_slice(&[Color::Black])
    );
    assert_eq!(
        engine.state.players[0].life, 23,
        "Condition::Prototyped ETB trigger gained 3 life"
    );

    // 2) Cast PROTO_CLONE copying proto_id -> enters as 3/3 Black MV 3 AND Condition::Prototyped fires (+3 life -> 26, CR 719.3d)!
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    let clone_id = the(&engine, ZoneLocation::Hand(me()), PROTO_CLONE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: clone_id })
        .expect("cast clone");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![proto_id],
                players: vec![],
            },
        )
        .expect("copy prototyped construct");
    settle(&mut engine, me());

    let cloned = engine.state.object(clone_id).unwrap();
    assert!(
        cloned.prototyped,
        "CR 719.3d: copy of prototyped permanent is prototyped"
    );
    assert_eq!(cloned.characteristics().power, Some(3));
    assert_eq!(cloned.characteristics().toughness, Some(3));
    assert_eq!(cloned.characteristics().mana_value(), 3);
    assert_eq!(
        engine.state.players[0].life, 26,
        "CR 719.3d: Clone's copied Condition::Prototyped ETB trigger also fired (+3 life)"
    );

    // 3) Exile proto_id with without_paying_mana_cost() -> reverts to 7/7 colorless in exile (CR 719.4),
    //    and free cast puts it onto battlefield as unprototyped 7/7 colorless (life stays 26)!
    add_pool(&mut engine, me(), ManaColor::White, 1);
    let recast_id = the(&engine, ZoneLocation::Hand(me()), FREE_EXILE_ONE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: recast_id })
        .expect("cast exile recast");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![proto_id],
            },
        )
        .expect("target proto_id");
    settle(&mut engine, me());

    let in_exile = engine.state.object(proto_id).unwrap();
    assert!(
        !in_exile.prototyped,
        "CR 719.4: normal characteristics in exile"
    );
    assert_eq!(in_exile.characteristics().power, Some(7));

    engine
        .apply(me(), PlayerAction::CastSpell { card: proto_id })
        .expect("cast from exile without paying mana cost");
    settle(&mut engine, me());
    let back_on_bf = engine.state.object(proto_id).unwrap();
    assert!(!back_on_bf.prototyped, "free cast cannot choose prototype");
    assert_eq!(back_on_bf.characteristics().power, Some(7));
    assert!(back_on_bf.characteristics().colors.is_colorless());
    assert_eq!(
        engine.state.players[0].life, 26,
        "unprototyped ETB did not fire"
    );
}

/// Slice 2e: Copying a spell on the stack (`CR 707.10`) preserves `Condition::Bargained`
/// (`CR 702.166c`) and `Condition::Prototyped` (`CR 719.3c`).
#[test]
fn spell_copies_preserve_bargained_and_prototyped_state_cr_702_166c_and_cr_719_3c() {
    const SPELL_COPIER: u32 = 8427;
    static BURN_ABILS: [AbilityDef; 1] = [spell!(&[Effect::IfCondition {
        condition: Condition::Bargained,
        then: &[Effect::LoseLife {
            target: PlayerRel::EachOpponent,
            amount: Amount::Fixed(4),
        }],
        otherwise: &[Effect::LoseLife {
            target: PlayerRel::EachOpponent,
            amount: Amount::Fixed(1),
        }],
    }])];
    static PROTO_ABILS: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        condition = Some(Condition::Prototyped)
    )];
    static COPY_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::CopyTargetSpell { mods: &[] }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];

    let burn = card(
        BARGAIN_BURN,
        FaceDef {
            bargain: true,
            ..face("Bargained Bolt", "{R}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        &BURN_ABILS,
    );
    let proto = card(
        PROTO_CONSTRUCT,
        FaceDef {
            prototype: Some(Prototype::new(ManaCost::parse("{1}{B}{B}"), 3, 3)),
            types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            ..creature_face("Dusk Automaton", "{7}", 7, 7)
        },
        KeywordSet::EMPTY,
        &PROTO_ABILS,
    );
    let rock = card(
        TRINKET_ROCK,
        FaceDef {
            mana_cost: ManaCost::parse("{1}"),
            types: TypeSet::ARTIFACT,
            castable_from_hand: true,
            ..land_face("Scrap Bauble")
        },
        KeywordSet::EMPTY,
        &[],
    );
    let copier = card(
        SPELL_COPIER,
        face("Echo Fork", "{U}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &COPY_ABILS,
    );

    let mut engine = bench(
        8408,
        vec![burn, proto, rock, copier],
        [
            Seat::with(&[TRINKET_ROCK]).holding(&[
                BARGAIN_BURN,
                PROTO_CONSTRUCT,
                SPELL_COPIER,
                SPELL_COPIER,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let rock_id = the(&engine, ZoneLocation::Battlefield, TRINKET_ROCK);
    let burn_id = the(&engine, ZoneLocation::Hand(me()), BARGAIN_BURN);
    let copiers = objects(&engine, ZoneLocation::Hand(me()), SPELL_COPIER);

    // 1) Cast BARGAIN_BURN with Bargain, then copy it on the stack -> BOTH deal 4 (total 8)!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: burn_id })
        .unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![rock_id],
            },
        )
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: copiers[0] })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![burn_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state.players[1].life, 12,
        "CR 702.166c: copy of bargained spell is also bargained (4 + 4 = 8 life lost)"
    );

    // 2) Cast PROTO_CONSTRUCT for prototype {1}{B}{B}, copy it on the stack ->
    //    BOTH resolve as 3/3 prototyped constructs and BOTH fire Condition::Prototyped (+6 life)!
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    let proto_id = the(&engine, ZoneLocation::Hand(me()), PROTO_CONSTRUCT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: proto_id })
        .unwrap();
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: copiers[1] })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![proto_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state.players[0].life, 26,
        "CR 719.3c: permanent spell copy is also prototyped (+3 + +3 = 26 life)"
    );
}
