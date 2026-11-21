//! Rule tests for Engine Wave 10a:
//! 1. `Freerunning [cost]` (`CR 702.173a`) & `Sneak [cost]` (`CR 702.190a–b`, `CR 506.3a`)
//! 2. `Sunburst` (`CR 702.44a–d`) & `Ravenous` (`CR 702.156a`)
//! 3. `Graft N` (`CR 702.58a–b`) + AI `MoveCounterToEvent` evaluation
//! 4. `Outlast` (`CR 702.107a`), `Afterlife` (`CR 702.135a`), `Afflict` (`CR 702.130a`),
//!    `Reinforce` (`CR 702.77a`), and `Effect::Endure` (`CR 701.63a–b`)

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, YesNoPrompt};
use crate::object::Status;
use crate::turn::Step;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{
    AlternativeCost, Amount, Condition, CounterKind, Duration, EnterModifier, Filter, Modifier,
    PlayerRel, ReplacementRule, SPIRIT_1_1_WHITE_BLACK_FLYING, TargetSpec, Trigger, activated,
    cost, spell, static_ability, triggered,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;
use baylee_core::types::SupertypeSet;

const W0: u32 = 99_960;
const W1: u32 = 99_961;
const W2: u32 = 99_962;
const W3: u32 = 99_963;

fn is_tapped(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

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

fn declare_attackers_step(
    engine: &mut Bench,
    seat: PlayerId,
    attackers: Vec<(ObjectId, Defender)>,
) {
    walk_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(seat, PlayerAction::DeclareAttackers { attackers })
        .expect("declare attackers");
}

fn declare_blockers_step(
    engine: &mut Bench,
    defender: PlayerId,
    blockers: Vec<(ObjectId, ObjectId)>,
) {
    walk_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(defender, PlayerAction::DeclareBlockers { blockers })
        .expect("declare blockers");
    walk_until(engine, |e| {
        holds_priority_in(e, me(), Step::DeclareBlockers)
    });
}

fn advance_to_main2(engine: &mut Bench) {
    walk_until(engine, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
}

static FREERUN_ALTS: &[AlternativeCost] = &[AlternativeCost::freerunning(cost!("{B}"))];
static FREERUN_SPELL_AB: &[AbilityDef] = &[spell!(&[Effect::IfCondition {
    condition: Condition::AltCostPaid,
    then: &[Effect::gain_life(4)],
    otherwise: &[Effect::gain_life(1)],
}])];

fn freerunning_sorcery_face() -> FaceDef {
    FaceDef {
        alternative_costs: FREERUN_ALTS,
        ..face("Eagle Drop", "{4}{B}", TypeSet::SORCERY)
    }
}

// =========================================================================
// 1. Freerunning (CR 702.173a) & Sneak (CR 702.190a–b)
// =========================================================================

#[test]
fn freerunning_rejects_non_assassin_combat_damage_and_noncombat_assassin_damage() {
    static PING_AB: &[AbilityDef] = &[activated!(
        cost!("{0}", TapSelf),
        &[Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::EachOpponent),
            amount: Amount::Fixed(1),
        }]
    )];
    let defs = vec![
        card(
            W0,
            FaceDef {
                subtypes: &[creature::GOBLIN],
                ..creature_face("Plain Goblin", "{R}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W1,
            FaceDef {
                subtypes: &[creature::ASSASSIN],
                ..creature_face("Knife Thrower", "{B}", 1, 1)
            },
            KeywordSet::HASTE,
            PING_AB,
        ),
        card(
            W2,
            freerunning_sorcery_face(),
            KeywordSet::EMPTY,
            FREERUN_SPELL_AB,
        ),
    ];
    assert!(
        defs[2]
            .keywords_for_face(0)
            .contains(KeywordSet::FREERUNNING)
    );
    let mut engine = bench(
        10_001,
        defs,
        [Seat::with(&[W0, W1]).holding(&[W2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let goblin = the(&engine, ZoneLocation::Battlefield, W0);
    let pinger = the(&engine, ZoneLocation::Battlefield, W1);
    let drop = the(&engine, ZoneLocation::Hand(me()), W2);

    // Noncombat damage by an Assassin does NOT unlock freerunning.
    activate(&mut engine, me(), pinger, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 19);

    // Combat damage by a non-Assassin non-commander Goblin does NOT unlock freerunning.
    declare_attackers_step(&mut engine, me(), vec![(goblin, Defender::Player(them()))]);
    declare_blockers_step(&mut engine, them(), vec![]);
    advance_to_main2(&mut engine);
    assert_eq!(life(&engine, them()), 17);

    add_pool(&mut engine, me(), ManaColor::Black, 1);
    assert!(
        !offer(&engine).1.castable.contains(&drop),
        "CR 702.173a: neither noncombat Assassin damage nor Goblin combat damage unlocks freerunning"
    );
}

#[test]
fn freerunning_unlocks_from_assassin_combat_damage_even_after_assassin_dies() {
    static SAC_SELF: &[AbilityDef] = &[activated!(cost!(SacrificeSelf), &[Effect::gain_life(0)])];
    let defs = vec![
        card(
            W0,
            FaceDef {
                subtypes: &[creature::ASSASSIN],
                ..creature_face("Hidden Blade", "{B}", 2, 1)
            },
            KeywordSet::EMPTY,
            SAC_SELF,
        ),
        card(
            W1,
            freerunning_sorcery_face(),
            KeywordSet::EMPTY,
            FREERUN_SPELL_AB,
        ),
    ];
    let mut engine = bench(
        10_002,
        defs,
        [Seat::with(&[W0]).holding(&[W1]), Seat::default()],
    );
    to_main(&mut engine, me());
    let assassin = the(&engine, ZoneLocation::Battlefield, W0);
    let drop = the(&engine, ZoneLocation::Hand(me()), W1);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![(assassin, Defender::Player(them()))],
    );
    declare_blockers_step(&mut engine, them(), vec![]);
    advance_to_main2(&mut engine);
    assert_eq!(life(&engine, them()), 18);

    // Assassin leaves the battlefield before casting Freerunning spell.
    activate(&mut engine, me(), assassin, 0);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(assassin).unwrap().zone,
        Zone::Graveyard
    );

    add_pool(&mut engine, me(), ManaColor::Black, 1);
    assert!(offer(&engine).1.castable.contains(&drop));
    engine
        .apply(me(), PlayerAction::CastSpell { card: drop })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        24,
        "CR 702.173a: freerunning alternative cost was paid after Assassin combat damage"
    );
}

fn p1p1(engine: &Bench, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .unwrap()
        .counters
        .get(CounterKind::P1P1)
}

fn commander_freerun_bench() -> Bench {
    static STEAL_AB: &[AbilityDef] = &[free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::GainControl,
            Duration::Indefinitely,
        )],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let cmd_def = Box::leak(Box::new(CardDef {
        index: CardIndex::new(W0),
        oracle_id: "test-commander",
        scryfall_id: "test-commander",
        faces: Box::leak(Box::new([FaceDef {
            supertypes: SupertypeSet::LEGENDARY,
            ..creature_face("Warlord", "{0}", 3, 3)
        }])),
        color_identity: ColorSet::ALL,
        keywords: KeywordSet::HASTE,
        commander: CommanderRule::Legendary,
        partner: PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities: &[],
    }));
    let defs = vec![
        cmd_def,
        card(
            W1,
            freerunning_sorcery_face(),
            KeywordSet::EMPTY,
            FREERUN_SPELL_AB,
        ),
        card(
            W2,
            face("Thief", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            STEAL_AB,
        ),
    ];
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let mk_seat = |cmds: &[u32], hand: &[u32], bf: &[u32]| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: cmds.iter().copied().map(entry).collect(),
        starting_life: None,
        starting_hand: Some(hand.iter().copied().map(entry).collect()),
        starting_battlefield: bf.iter().copied().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 702_173,
        house_rules: HouseRules {
            starting_player: Some(0),
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk_seat(&[], &[W1], &[W2]), mk_seat(&[W0], &[], &[])],
    };
    let mut engine = Engine::new(&preset, SyntheticLookup::new(defs)).unwrap();
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

#[test]
fn freerunning_unlocks_from_commander_combat_damage_under_your_control() {
    let mut engine = commander_freerun_bench();
    to_main(&mut engine, them());
    let cmd_in_cz = the(&engine, ZoneLocation::Command(them()), W0);
    engine
        .apply(them(), PlayerAction::CastSpell { card: cmd_in_cz })
        .unwrap();
    settle(&mut engine, them());

    to_main(&mut engine, me());
    let cmd = the(&engine, ZoneLocation::Battlefield, W0);
    let drop = the(&engine, ZoneLocation::Hand(me()), W1);
    let thief = the(&engine, ZoneLocation::Battlefield, W2);
    activate(&mut engine, me(), thief, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![cmd],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    declare_attackers_step(&mut engine, me(), vec![(cmd, Defender::Player(them()))]);
    declare_blockers_step(&mut engine, them(), vec![]);
    advance_to_main2(&mut engine);

    add_pool(&mut engine, me(), ManaColor::Black, 1);
    assert!(
        offer(&engine).1.castable.contains(&drop),
        "CR 702.173a: combat damage by any commander under your control unlocks freerunning"
    );
}

#[test]
fn sneak_timing_restricted_to_declare_blockers_with_unblocked_attacker() {
    static SNEAK_AB: &[AbilityDef] = &[AbilityDef::sneak(cost!("{U}"))];
    let defs = vec![
        card(
            W0,
            creature_face("Foot Scout", "{U}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W1,
            creature_face("Turtle Ninja", "{3}{U}", 4, 4),
            KeywordSet::EMPTY,
            SNEAK_AB,
        ),
        card(
            W2,
            creature_face("Wall", "{W}", 0, 4),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    assert!(defs[1].keywords_for_face(0).contains(KeywordSet::SNEAK));
    let mut engine = bench(
        10_004,
        defs,
        [Seat::with(&[W0]).holding(&[W1]), Seat::with(&[W2])],
    );
    to_main(&mut engine, me());
    let scout = the(&engine, ZoneLocation::Battlefield, W0);
    let ninja = the(&engine, ZoneLocation::Hand(me()), W1);
    let wall = the(&engine, ZoneLocation::Battlefield, W2);

    // Main1 with {U} only: Sneak is NOT offered outside DeclareBlockers.
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    assert!(!offer(&engine).1.castable.contains(&ninja));

    // DeclareBlockers when scout IS blocked: Sneak is NOT offered.
    declare_attackers_step(&mut engine, me(), vec![(scout, Defender::Player(them()))]);
    declare_blockers_step(&mut engine, them(), vec![(wall, scout)]);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    assert!(
        !offer(&engine).1.castable.contains(&ninja),
        "CR 702.190a: blocked attacker cannot pay sneak"
    );
}

#[test]
fn sneak_returns_unblocked_attacker_and_enters_tapped_attacking_same_planeswalker() {
    static SNEAK_AB: &[AbilityDef] = &[AbilityDef::sneak(cost!("{U}"))];
    let defs = vec![
        card(
            W0,
            creature_face("Foot Scout", "{U}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W1,
            creature_face("Leonardo", "{3}{U}", 4, 4),
            KeywordSet::EMPTY,
            SNEAK_AB,
        ),
        card(
            W2,
            FaceDef {
                loyalty: Some(5),
                ..face("Enemy Walker", "{0}", TypeSet::PLANESWALKER)
            },
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine = bench(
        10_005,
        defs,
        [
            Seat::with(&[W0]).holding(&[W1]),
            Seat::default().holding(&[W2]),
        ],
    );
    to_main(&mut engine, them());
    let walker_card = the(&engine, ZoneLocation::Hand(them()), W2);
    engine
        .apply(them(), PlayerAction::CastSpell { card: walker_card })
        .unwrap();
    settle(&mut engine, them());

    to_main(&mut engine, me());
    let scout = the(&engine, ZoneLocation::Battlefield, W0);
    let leo = the(&engine, ZoneLocation::Hand(me()), W1);
    let walker = the(&engine, ZoneLocation::Battlefield, W2);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![(scout, Defender::Planeswalker(walker))],
    );
    declare_blockers_step(&mut engine, them(), vec![]);
    // Give {U}{U}{U}{U} (enough for normal {3}{U}): only Sneak is legal in DeclareBlockers!
    add_pool(&mut engine, me(), ManaColor::Blue, 4);
    let (_, legal) = offer(&engine);
    assert!(legal.castable.contains(&leo));
    let leo_ways: Vec<CastModeKind> = legal
        .cast_ways
        .iter()
        .filter(|(id, _)| *id == leo)
        .map(|(_, w)| w.kind)
        .collect();
    assert_eq!(leo_ways, vec![CastModeKind::Sneak]);

    engine
        .apply(me(), PlayerAction::CastSpell { card: leo })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::CostReturn,
            ..
        }
    ));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![scout],
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(scout).unwrap().zone, Zone::Hand);

    settle(&mut engine, me());
    assert_eq!(engine.state().object(leo).unwrap().zone, Zone::Battlefield);
    assert!(
        is_tapped(&engine, leo),
        "CR 702.190b: sneaked permanent enters tapped"
    );
    assert_eq!(
        engine.state().combat.defender_of(leo),
        Some(Defender::Planeswalker(walker)),
        "CR 702.190b: sneaked creature enters attacking the same planeswalker"
    );

    advance_to_main2(&mut engine);
    assert_eq!(
        engine
            .state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        1,
        "sneaked 4/4 dealt 4 combat damage to Enemy Walker (5 -> 1)"
    );
}

#[test]
fn sneak_noncreature_permanent_enters_tapped_not_attacking_and_sorcery_casts_in_blockers() {
    static SNEAK_U: &[AbilityDef] = &[AbilityDef::sneak(cost!("{U}"))];
    static SNEAK_SORCERY: &[AbilityDef] = &[
        AbilityDef::sneak(cost!("{U}")),
        spell!(&[Effect::gain_life(3)]),
    ];
    let defs = vec![
        card(
            W0,
            creature_face("Scout A", "{U}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W1,
            creature_face("Scout B", "{U}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W2,
            face("Party Wagon", "{4}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SNEAK_U,
        ),
        card(
            W3,
            face("Smoke Bomb", "{2}{U}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            SNEAK_SORCERY,
        ),
    ];
    let mut engine = bench(
        10_006,
        defs,
        [Seat::with(&[W0, W1]).holding(&[W2, W3]), Seat::default()],
    );
    to_main(&mut engine, me());
    let scout_a = the(&engine, ZoneLocation::Battlefield, W0);
    let scout_b = the(&engine, ZoneLocation::Battlefield, W1);
    let wagon = the(&engine, ZoneLocation::Hand(me()), W2);
    let smoke = the(&engine, ZoneLocation::Hand(me()), W3);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![
            (scout_a, Defender::Player(them())),
            (scout_b, Defender::Player(them())),
        ],
    );
    declare_blockers_step(&mut engine, them(), vec![]);
    add_pool(&mut engine, me(), ManaColor::Blue, 2);

    // Sneak noncreature artifact: enters tapped, NOT attacking (CR 702.190b, CR 506.3a).
    engine
        .apply(me(), PlayerAction::CastSpell { card: wagon })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![scout_a],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(wagon).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&engine, wagon));
    assert_eq!(engine.state().combat.defender_of(wagon), None);

    // Sneak sorcery: cast at instant speed during DeclareBlockers, resolves to graveyard.
    engine
        .apply(me(), PlayerAction::CastSpell { card: smoke })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![scout_b],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(smoke).unwrap().zone, Zone::Graveyard);
    assert_eq!(life(&engine, me()), 23);
}

// =========================================================================
// 2. Sunburst (CR 702.44a–d) & Ravenous (CR 702.156a)
// =========================================================================

#[test]
fn sunburst_places_p1p1_on_creatures_and_charge_on_animated_artifacts() {
    static SUNBURST_MOD: &[EnterModifier] = &[EnterModifier::Sunburst];
    static ANIMATE_ARTIFACTS: &[AbilityDef] = &[static_ability!(
        Filter::ARTIFACT,
        Modifier::AnimateNoncreatureArtifact
    )];
    static DOUBLER_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleCounterPlacement {
            object_filter: &Filter::ControlledByYou,
        },
    )];
    let defs = vec![
        card(
            W0,
            FaceDef {
                enter_modifiers: SUNBURST_MOD,
                ..creature_face("Suntouched Myr", "{3}", 0, 0)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W1,
            FaceDef {
                enter_modifiers: SUNBURST_MOD,
                ..face("Pentad Prism", "{2}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W2,
            face("March", "{3}{U}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            ANIMATE_ARTIFACTS,
        ),
        card(
            W3,
            face("Doubler", "{4}{G}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DOUBLER_AB,
        ),
    ];
    assert!(defs[0].keywords_for_face(0).contains(KeywordSet::SUNBURST));
    let mut engine = bench(
        10_007,
        defs,
        [Seat::with(&[W2, W3]).holding(&[W0, W1]), Seat::default()],
    );
    to_main(&mut engine, me());
    let myr = the(&engine, ZoneLocation::Hand(me()), W0);
    let prism = the(&engine, ZoneLocation::Hand(me()), W1);

    // Spend {W}{U}{B} (3 colors) on Suntouched Myr under Doubler -> 3 * 2 = 6 +1/+1 counters.
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: myr })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(myr)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        6
    );

    // Spend {R}{G} (2 colors) on Pentad Prism while March animates noncreature artifacts:
    // CR 702.44a ignores Layer 4 type-changing effects -> Charge counters (2 * 2 = 4), NOT +1/+1!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: prism })
        .unwrap();
    settle(&mut engine, me());
    let prism_obj = engine.state().object(prism).unwrap();
    assert!(
        prism_obj
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(prism_obj.counters.get(CounterKind::Charge), 4);
    assert_eq!(prism_obj.counters.get(CounterKind::P1P1), 0);
}

#[test]
fn ravenous_enters_with_x_counters_and_draws_only_when_x_is_five_or_more() {
    static RAV_MODS: &[EnterModifier] = &[EnterModifier::ravenous()];
    static RAV_AB: &[AbilityDef] = &[AbilityDef::ravenous_draw()];
    let defs = vec![
        card(
            W0,
            FaceDef {
                enter_modifiers: RAV_MODS,
                ..creature_face("Small Ravener", "{X}{G}", 1, 1)
            },
            KeywordSet::RAVENOUS,
            RAV_AB,
        ),
        card(
            W1,
            FaceDef {
                enter_modifiers: RAV_MODS,
                ..creature_face("Big Ravener", "{X}{G}", 1, 1)
            },
            KeywordSet::RAVENOUS,
            RAV_AB,
        ),
    ];
    let mut engine = bench(
        10_008,
        defs,
        [Seat::default().holding(&[W0, W1]), Seat::default()],
    );
    to_main(&mut engine, me());
    let small = the(&engine, ZoneLocation::Hand(me()), W0);
    let big = the(&engine, ZoneLocation::Hand(me()), W1);

    // X = 4 -> 4 +1/+1 counters, no card drawn.
    add_pool(&mut engine, me(), ManaColor::Green, 5);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    engine
        .apply(me(), PlayerAction::CastSpell { card: small })
        .unwrap();
    engine.apply(me(), PlayerAction::ChooseNumber(4)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(small)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before - 1
    );

    // X = 5 -> 5 +1/+1 counters and draws a card (CR 702.156a).
    add_pool(&mut engine, me(), ManaColor::Green, 6);
    engine
        .apply(me(), PlayerAction::CastSpell { card: big })
        .unwrap();
    engine.apply(me(), PlayerAction::ChooseNumber(5)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(big)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before - 1,
        "CR 702.156a: X = 5 drew a replacement card"
    );
}

// =========================================================================
// 3. Graft N (CR 702.58a–b) + AI MoveCounterToEvent
// =========================================================================

#[test]
fn graft_moves_counter_to_friendly_creature_and_ai_declines_for_hostile_creature() {
    static GRAFT_2: &[EnterModifier] = &[EnterModifier::graft(2)];
    static GRAFT_AB: &[AbilityDef] = &[AbilityDef::graft_trigger()];
    let defs = vec![
        card(
            W0,
            FaceDef {
                enter_modifiers: GRAFT_2,
                ..creature_face("Plax Frogling", "{1}{G}{U}", 0, 0)
            },
            KeywordSet::GRAFT,
            GRAFT_AB,
        ),
        card(
            W1,
            creature_face("Ally Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            W2,
            creature_face("Enemy Bear", "{0}", 2, 2),
            KeywordSet::FLASH,
            &[],
        ),
    ];
    let mut engine = bench(
        10_009,
        defs,
        [
            Seat::default().holding(&[W0, W1]),
            Seat::default().holding(&[W2]),
        ],
    );
    to_main(&mut engine, me());
    let frog = the(&engine, ZoneLocation::Hand(me()), W0);
    let ally = the(&engine, ZoneLocation::Hand(me()), W1);
    let enemy = the(&engine, ZoneLocation::Hand(them()), W2);

    add_pool(&mut engine, me(), ManaColor::Green, 2);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: frog })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(p1p1(&engine, frog), 2);

    // Friendly creature enters -> accept MayDo -> moves 1 +1/+1 counter (2 -> 1, 0 -> 1).
    engine
        .apply(me(), PlayerAction::CastSpell { card: ally })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(p1p1(&engine, frog), 1);
    assert_eq!(p1p1(&engine, ally), 1);

    // Hostile Flash creature enters -> DecisionContext carries event_object = Some(enemy).
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(them(), PlayerAction::CastSpell { card: enemy })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert_eq!(engine.decision_context().event_object, Some(enemy));
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    assert_eq!(p1p1(&engine, enemy), 0);
}

// =========================================================================
// 4. Outlast, Afterlife, Afflict, Reinforce, and Endure (CR 701.63a–b)
// =========================================================================

static OUTLAST_AFTERLIFE_AFFLICT: &[AbilityDef] = &[
    activated!(
        cost!("{W}", TapSelf),
        &[Effect::AddCounterSource {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        timing = ActivationTiming::SorcerySpeed,
    ),
    triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::CreateTokenN {
            token: &SPIRIT_1_1_WHITE_BLACK_FLYING,
            amount: Amount::Fixed(2),
        }],
    ),
    triggered!(
        Trigger::BecomesBlocked,
        &[Effect::LoseLife {
            target: PlayerRel::DefendingPlayer,
            amount: Amount::Fixed(3),
        }],
    ),
];
static REINFORCE_2: &[AbilityDef] = &[activated!(
    cost!("{G}", DiscardSelf),
    &[Effect::AddCounter {
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(2),
    }],
    target = Some(TargetSpec::Object(&Filter::CREATURE)),
    zone = ActivationZone::Hand,
)];

#[test]
fn outlast_afterlife_afflict_and_reinforce_execute_rules_and_carry_keywords() {
    let defs = vec![
        card(
            W0,
            creature_face("Quintet Veteran", "{1}{W}{B}", 2, 2),
            KeywordSet::OUTLAST
                .union(KeywordSet::AFTERLIFE)
                .union(KeywordSet::AFFLICT),
            OUTLAST_AFTERLIFE_AFFLICT,
        ),
        card(
            W1,
            creature_face("Burrenton Reinforcer", "{2}{G}", 2, 2),
            KeywordSet::REINFORCE,
            REINFORCE_2,
        ),
        card(
            W2,
            creature_face("Big Blocker", "{3}", 6, 6),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut engine = bench(
        10_010,
        defs,
        [Seat::with(&[W0]).holding(&[W1]), Seat::with(&[W2])],
    );
    to_main(&mut engine, me());
    let vet = the(&engine, ZoneLocation::Battlefield, W0);
    let reinf = the(&engine, ZoneLocation::Hand(me()), W1);
    let blocker = the(&engine, ZoneLocation::Battlefield, W2);

    // Reinforce 2 from hand puts 2 +1/+1 counters on Quintet Veteran (now 4/4).
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    activate(&mut engine, me(), reinf, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![vet],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(p1p1(&engine, vet), 2);

    // Attack -> blocked by 6/6 -> Afflict 3 makes defender lose 3 life, Dies creates 2 Spirits!
    declare_attackers_step(&mut engine, me(), vec![(vet, Defender::Player(them()))]);
    declare_blockers_step(&mut engine, them(), vec![(blocker, vet)]);
    advance_to_main2(&mut engine);
    assert_eq!(life(&engine, them()), 17, "CR 702.130a: Afflict 3");
    let spirits = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                let ch = o.characteristics();
                o.controller == me()
                    && o.token.is_some()
                    && ch.subtypes.contains(creature::SPIRIT)
                    && ch.keywords.contains(KeywordSet::FLYING)
                    && ch.colors == ColorSet::from_slice(&[Color::White, Color::Black])
            })
        })
        .count();
    assert_eq!(
        spirits, 2,
        "CR 702.135a: Afterlife 2 created two 1/1 W/B flying Spirits"
    );
}

#[test]
fn endure_prompts_for_counters_or_spirit_handles_departed_source_and_zero() {
    static ENDURE_AB: &[AbilityDef] = &[
        activated!(
            cost!("{X}"),
            &[Effect::Endure(Amount::X)],
            x_value = baylee_cards_dsl::ActivationX::Any
        ),
        activated!(cost!(SacrificeSelf), &[Effect::gain_life(0)]),
    ];
    let defs = vec![card(
        W0,
        creature_face("AbzanKinGuard", "{2}{W}", 2, 2),
        KeywordSet::FLYING,
        ENDURE_AB,
    )];
    let mut engine = bench(10_011, defs, [Seat::with(&[W0]), Seat::default()]);
    to_main(&mut engine, me());
    let guard = the(&engine, ZoneLocation::Battlefield, W0);

    // 1) Endure 0 -> nothing happens (CR 701.63b).
    activate(&mut engine, me(), guard, 0);
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(me(), PlayerAction::ChooseNumber(0)).unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(guard)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // 2) Endure 3 -> answer Yes -> 3 +1/+1 counters on guard.
    add_pool(&mut engine, me(), ManaColor::White, 3);
    activate(&mut engine, me(), guard, 0);
    engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Endure(3),
                ..
            }
        )
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Endure(3),
            source: Some(s),
            ..
        } if s.card == CardIndex::new(W0)
    ));
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(guard)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3
    );

    // 3) Endure 4, then sacrifice guard in response -> creates 4/4 white Spirit without prompting!
    add_pool(&mut engine, me(), ManaColor::White, 4);
    activate(&mut engine, me(), guard, 0);
    engine.apply(me(), PlayerAction::ChooseNumber(4)).unwrap();
    activate(&mut engine, me(), guard, 1);
    settle(&mut engine, me());
    let spirit_4_4 = engine.state().battlefield_seen().find(|&id| {
        engine.state().object(id).is_some_and(|o| {
            let ch = o.characteristics();
            o.token.is_some()
                && ch.subtypes.contains(creature::SPIRIT)
                && ch.power == Some(4)
                && ch.toughness == Some(4)
                && ch.colors == ColorSet::from_slice(&[Color::White])
        })
    });
    assert!(
        spirit_4_4.is_some(),
        "CR 701.63a: departed permanent enduring 4 creates a 4/4 white Spirit token"
    );
}

#[test]
fn endure_declined_on_live_permanent_creates_n_n_white_spirit_instead_of_counters() {
    static ENDURE_2: &[AbilityDef] = &[activated!(cost!("{0}"), &[Effect::endure(2)])];
    let defs = vec![card(
        W0,
        creature_face("Sandsteppe Outcast", "{2}{W}", 2, 1),
        KeywordSet::EMPTY,
        ENDURE_2,
    )];
    let mut engine = bench(10_012, defs, [Seat::with(&[W0]), Seat::default()]);
    to_main(&mut engine, me());
    let outcast = the(&engine, ZoneLocation::Battlefield, W0);

    activate(&mut engine, me(), outcast, 0);
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Endure(2),
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(outcast)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    let spirit_2_2 = engine.state().battlefield_seen().find(|&id| {
        engine.state().object(id).is_some_and(|o| {
            let ch = o.characteristics();
            o.token.is_some()
                && ch.subtypes.contains(creature::SPIRIT)
                && ch.power == Some(2)
                && ch.toughness == Some(2)
        })
    });
    assert!(
        spirit_2_2.is_some(),
        "CR 701.63a: declining Endure 2 creates a 2/2 white Spirit"
    );
}
