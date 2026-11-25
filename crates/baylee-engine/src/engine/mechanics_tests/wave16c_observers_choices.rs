//! Wave 16c deterministic gameplay & Comprehensive Rules tests:
//! - `Effect::EventObjectDealsDamageEach` (`CR 120.2b`, `CR 603.7`, `CR 608.2h`; *Arcbond* LKI lifelink/deathtouch).
//! - `Modifier::CantBlock(&Filter::This)` incarnation guard (`CR 509.1b`, `CR 400.7`; *Auriok Siege Sled*).
//! - `Trigger::OneOrMoreAttack` batch `Amount::EventAmount` (`CR 508.3a`, `CR 603.2c`; *Basri Ket* `[-2]`).
//! - `Modifier::DamageCantBePrevented` (`CR 615.12`; *Skullcrack*, *Leyline of Punishment*).
//! - `ReplacementRule::{EnterTappedIf, EnterUntappedIf}` (`CR 614.1c`, `CR 616.1`; *Archelos, Lagoon Mystic*).
//! - Pre-counter entry choices (`ChooseSubtype` / `ChooseOpponent` + `WithCounters`, `PlayerRel::ChosenBySource`;
//!   `CR 614.12a`, `CR 614.16`; *Banner of Kinship*, *Canker Abomination*).
//! - `EnterModifier::{ChooseAnyPlayer, LookAtOpponentHand, BecomeDayIfNeither}` + `Trigger::DayNightChanges`
//!   (`CR 614.12a`, `CR 726.2`, `CR 730.2`; *True-Name Nemesis*, *Sorcerous Spyglass*, *Brimstone Vandal*).
//! - `Effect::ChoosePlayerAtRandom`, `Rider::MustAttackPlayerThisCombat` (`Duration::UntilEndOfCombat`), and
//!   `Filter::{CmcEqualsChosenNumber, PowerEqualsChosenNumber, ToughnessEqualsChosenNumber, PowerEqualsAmount}`
//!   (`CR 101.4`, `CR 508.1d`, `CR 607.2a`; *Ruhan of the Fomori*, *Talion, the Kindly Lord*, *Localized Destruction*).

#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::ChoicePrompt;
use crate::object::Status;
use crate::turn::{DayNight, Step};
use crate::zone::ZoneLocation;
use baylee_cards_dsl::counters::QUEST;
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, DelayedDuration, DelayedTriggerWhen, Duration, EnterModifier,
    Filter, Modifier, PlayerRel, ReplacementRule, TargetSpec, TokenDef, Trigger, TriggerRemember,
    ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;
use baylee_core::ids::{CardIndex, Defender};

const W0: u32 = 99_810;
const W1: u32 = 99_811;
const W2: u32 = 99_812;
const W3: u32 = 99_813;

static ELF_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[creature::ELF];
static SOLDIER_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[creature::SOLDIER];

static SOLDIER_TOKEN: TokenDef = TokenDef {
    name: "Soldier",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: SOLDIER_SUBTYPES,
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};

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

#[test]
fn arcbond_event_object_deals_damage_each_uses_lki_lifelink_and_deathtouch() {
    static ARCBOND_EFFECTS: &[Effect] = &[Effect::RegisterDelayedTrigger {
        when: DelayedTriggerWhen::OnTrigger {
            trigger: &Trigger::DealtDamage(&Filter::This),
            watch: TriggerRemember::FirstTarget,
            duration: DelayedDuration::ThisTurn,
        },
        condition: None,
        targets: None,
        remember: TriggerRemember::None,
        effects: &[Effect::EventObjectDealsDamageEach {
            amount: Amount::EventAmount,
            filter: &Filter::CREATURE,
            players: Some(PlayerRel::EachPlayer),
            exclude_event_object: true,
        }],
    }];
    static ARCBOND_AB: &[AbilityDef] = &[spell!(
        ARCBOND_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static BOLT_FOUR: &[Effect] = &[Effect::DealDamage {
        amount: Amount::Fixed(4),
        target: TargetSpec::Object(&Filter::CREATURE),
    }];
    static BOLT_AB: &[AbilityDef] = &[spell!(
        BOLT_FOUR,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let arcbond = card(
        W0,
        face("Arcbond Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        ARCBOND_AB,
    );
    let bolt = card(
        W1,
        face("Four Bolt", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        BOLT_AB,
    );
    let host = card(
        W2,
        creature_face("Vampire Conduit", "{0}", 2, 2),
        KeywordSet::LIFELINK.union(KeywordSet::DEATHTOUCH),
        &[],
    );
    let colossus = card(
        W3,
        creature_face("Stone Colossus", "{0}", 0, 10),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        16_301,
        vec![arcbond, bolt, host, colossus],
        [
            Seat::with(&[W2]).holding(&[W0, W1]).at(20),
            Seat::with(&[W3]).at(20),
        ],
    );
    to_main(&mut engine, me());
    let host_id = the(&engine, ZoneLocation::Battlefield, W2);
    let colossus_id = the(&engine, ZoneLocation::Battlefield, W3);
    let arcbond_id = the(&engine, ZoneLocation::Hand(me()), W0);
    let bolt_id = the(&engine, ZoneLocation::Hand(me()), W1);

    // Cast Arcbond targeting our 2/2 lifelink+deathtouch creature.
    engine
        .apply(me(), PlayerAction::CastSpell { card: arcbond_id })
        .unwrap();
    aim_object(&mut engine, me(), host_id);
    settle(&mut engine, me());

    // Deal 4 lethal damage to the 2/2: it dies before the delayed trigger resolves,
    // so EventObjectDealsDamageEach must use its departed LKI characteristics!
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt_id })
        .unwrap();
    aim_object(&mut engine, me(), host_id);
    settle(&mut engine, me());

    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), W2),
        vec![host_id],
        "host died from the 4 damage"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(them()), W3),
        vec![colossus_id],
        "0/10 Stone Colossus died to 4 deathtouch damage from departed LKI source"
    );
    // Lifelink gained 4 (from Colossus) + 4 (from me) + 4 (from them) = +12 life,
    // minus 4 damage dealt to me = 28 life; opponent took 4 = 16 life.
    assert_eq!(life(&engine, me()), 28);
    assert_eq!(life(&engine, them()), 16);
}

#[test]
fn auriok_siege_sled_cant_block_this_resets_when_source_reenters_as_new_object() {
    static SLED_AB: &[AbilityDef] = &[free(
        &[Effect::ModifyTargets {
            modifier: Modifier::CantBlock(&Filter::This),
            duration: Duration::UntilEndOfTurn,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static BLINK_EFFECTS: &[Effect] = &[Effect::blink_to_owner(TargetSpec::Object(
        &Filter::CREATURE,
    ))];
    static BLINK_AB: &[AbilityDef] = &[spell!(
        BLINK_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let sled = card(
        W0,
        creature_face("Siege Sled", "{0}", 3, 5),
        KeywordSet::HASTE,
        SLED_AB,
    );
    let blocker = card(
        W1,
        creature_face("Iron Wall", "{0}", 0, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let blink = card(
        W2,
        face("Cloudshift Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::FLASH,
        BLINK_AB,
    );

    // Case A: Without blinking, Iron Wall CANNOT block Siege Sled.
    let mut g1 = bench(
        16_302,
        vec![sled, blocker, blink],
        [Seat::with(&[W0]), Seat::with(&[W1])],
    );
    to_main(&mut g1, me());
    let sled_id = the(&g1, ZoneLocation::Battlefield, W0);
    let wall_id = the(&g1, ZoneLocation::Battlefield, W1);
    activate(&mut g1, me(), sled_id, 0);
    aim_object(&mut g1, me(), wall_id);
    settle(&mut g1, me());

    walk_until(&mut g1, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    g1.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(sled_id, Defender::Player(them()))],
        },
    )
    .unwrap();
    walk_until(&mut g1, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || (e.state().turn.phase == Phase::SecondMain && holds_priority_in(e, me(), Step::Main))
    });
    if matches!(g1.pending(), Pending::ChooseBlockers { .. }) {
        assert!(
            g1.apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(wall_id, sled_id)],
                },
            )
            .is_err(),
            "Iron Wall must not be allowed to block Siege Sled"
        );
    } else {
        assert_eq!(life(&g1, them()), 17, "Siege Sled attacked unblocked for 3");
    }

    // Case B: Blinking Siege Sled creates a new incarnation (CR 400.7) with a newer
    // timestamp than the CantBlock effect, so Iron Wall CAN block the new incarnation!
    let mut g2 = bench(
        16_303,
        vec![sled, blocker, blink],
        [Seat::with(&[W0]).holding(&[W2]), Seat::with(&[W1])],
    );
    to_main(&mut g2, me());
    let sled_old = the(&g2, ZoneLocation::Battlefield, W0);
    let wall_id2 = the(&g2, ZoneLocation::Battlefield, W1);
    let blink_id = the(&g2, ZoneLocation::Hand(me()), W2);

    activate(&mut g2, me(), sled_old, 0);
    aim_object(&mut g2, me(), wall_id2);
    settle(&mut g2, me());

    g2.apply(me(), PlayerAction::CastSpell { card: blink_id })
        .unwrap();
    aim_object(&mut g2, me(), sled_old);
    settle(&mut g2, me());

    let sled_new = the(&g2, ZoneLocation::Battlefield, W0);
    walk_until(&mut g2, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    g2.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(sled_new, Defender::Player(them()))],
        },
    )
    .unwrap();
    walk_until(&mut g2, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    g2.apply(
        them(),
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall_id2, sled_new)],
        },
    )
    .expect("CR 400.7: Iron Wall can legally block the blinked new incarnation");
}

#[test]
fn basri_ket_one_or_more_attack_binds_event_amount_and_skullcrack_stops_all_prevention() {
    static NONTOKEN_CREATURE: Filter =
        Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)]);
    static BASRI_AB: &[AbilityDef] = &[free(
        &[Effect::RegisterDelayedTrigger {
            when: DelayedTriggerWhen::OnTrigger {
                trigger: &Trigger::OneOrMoreAttack(&NONTOKEN_CREATURE),
                watch: TriggerRemember::None,
                duration: DelayedDuration::ThisTurn,
            },
            condition: None,
            targets: None,
            remember: TriggerRemember::None,
            effects: &[Effect::CreateTokenAttacking {
                token: &SOLDIER_TOKEN,
                amount: Amount::EventAmount,
                sacrifice_at_next_end_step: false,
            }],
        }],
        None,
    )];
    static MAKE_TOKEN_AB: &[AbilityDef] = &[free(
        &[Effect::CreateTokenN {
            token: &SOLDIER_TOKEN,
            amount: Amount::Fixed(1),
        }],
        None,
    )];
    static HASTE_AB: &[AbilityDef] = &[static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::AddKeyword(KeywordSet::HASTE)
    )];
    static PRO_RED_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Red])))
    )];
    static SKULLCRACK_AB: &[AbilityDef] = &[spell!(&[
        Effect::continuous(
            &Filter::Any,
            Modifier::DamageCantBePrevented,
            Duration::UntilEndOfTurn,
        ),
        Effect::damage_each(3, &Filter::CREATURE),
    ])];

    let basri = card(
        W0,
        face("Basri Emblemizer", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        BASRI_AB,
    );
    let token_maker = card(
        W1,
        face("Soldier Horn", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        MAKE_TOKEN_AB,
    );
    let vanguard = card(
        W2,
        creature_face("Vanguard", "{0}", 2, 2),
        KeywordSet::HASTE,
        &[],
    );
    let fervor = card(
        W3,
        face("Haste Banner", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        HASTE_AB,
    );

    let mut engine = bench(
        16_304,
        vec![basri, token_maker, vanguard, fervor],
        [Seat::with(&[W0, W1, W2, W2, W3]), Seat::default().at(20)],
    );
    to_main(&mut engine, me());
    let basri_id = the(&engine, ZoneLocation::Battlefield, W0);
    let horn_id = the(&engine, ZoneLocation::Battlefield, W1);
    activate(&mut engine, me(), basri_id, 0);
    settle(&mut engine, me());
    activate(&mut engine, me(), horn_id, 0);
    settle(&mut engine, me());

    let vanguards = objects(&engine, ZoneLocation::Battlefield, W2);
    assert_eq!(vanguards.len(), 2);
    let pre_token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| engine.state().object(*id).unwrap().card.is_none())
        .expect("token created");

    // Attack with 2 nontoken Vanguards + 1 Soldier token -> EventAmount must be 2 (not 3)!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (vanguards[0], Defender::Player(them())),
                    (vanguards[1], Defender::Player(them())),
                    (pre_token, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    let total_tokens = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| engine.state().object(**id).unwrap().card.is_none())
        .count();
    assert_eq!(
        total_tokens, 3,
        "1 pre-existing token + 2 tapped-and-attacking tokens from 2 nontoken attackers"
    );
    assert_eq!(life(&engine, them()), 13);

    // Skullcrack / Leyline of Punishment: DamageCantBePrevented bypasses ProtectionFrom!
    let pro_red_knight = card(
        W0,
        creature_face("Silver Knight", "{0}", 2, 2),
        KeywordSet::EMPTY,
        PRO_RED_AB,
    );
    let skullcrack = card(
        W1,
        FaceDef {
            color_indicator: ColorSet::from_slice(&[Color::Red]),
            ..face("Skullcrack Sweep", "{0}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        SKULLCRACK_AB,
    );
    let mut g_prev = bench(
        16_305,
        vec![pro_red_knight, skullcrack],
        [Seat::default().holding(&[W1]), Seat::with(&[W0])],
    );
    to_main(&mut g_prev, me());
    let sc_id = the(&g_prev, ZoneLocation::Hand(me()), W1);
    g_prev
        .apply(me(), PlayerAction::CastSpell { card: sc_id })
        .unwrap();
    settle(&mut g_prev, me());
    assert!(
        objects(&g_prev, ZoneLocation::Battlefield, W0).is_empty(),
        "CR 615.12: red sweep killed protection-from-red knight because damage can't be prevented"
    );
}

#[test]
fn archelos_enter_tapped_if_and_enter_untapped_if_track_source_tap_state() {
    static ARCHELOS_AB: &[AbilityDef] = &[
        AbilityDef::Replacement(ReplacementRule::EnterTappedIf {
            filter: &Filter::Any,
            condition: Condition::SourceMatches(&Filter::Tapped),
        }),
        AbilityDef::Replacement(ReplacementRule::EnterUntappedIf {
            filter: &Filter::Any,
            condition: Condition::SourceMatches(&Filter::Untapped),
        }),
        paid(Cost::TAP, &[Effect::gain_life(1)], None),
    ];

    let archelos = card(
        W0,
        creature_face("Archelos Probe", "{0}", 2, 4),
        KeywordSet::HASTE,
        ARCHELOS_AB,
    );
    let tap_artifact = card(
        W1,
        FaceDef {
            enter_modifiers: &[EnterModifier::Tapped],
            ..face("Worn Diamond", "{0}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let bear = card(
        W2,
        creature_face("Fresh Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        16_306,
        vec![archelos, tap_artifact, bear],
        [Seat::with(&[W0]).holding(&[W1, W2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let arch_id = the(&engine, ZoneLocation::Battlefield, W0);
    let diamond_id = the(&engine, ZoneLocation::Hand(me()), W1);
    let bear_id = the(&engine, ZoneLocation::Hand(me()), W2);

    // 1. While Archelos is UNTAPPED: casting Worn Diamond (EnterModifier::Tapped)
    //    competes with Archelos's EnterUntappedIf (CR 616.1). Apply self-tapped first,
    //    then Archelos's EnterUntappedIf overrides it so Worn Diamond enters UNTAPPED!
    engine
        .apply(me(), PlayerAction::CastSpell { card: diamond_id })
        .unwrap();
    for _ in 0..20 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me())
        {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                prompt: ChoicePrompt::EntryReplacement,
                ..
            } => {
                // Choose own self-tapped clause first if present, leaving Archelos's
                // EnterUntappedIf to apply second and leave the permanent untapped!
                let pick = if options.contains(&diamond_id) {
                    diamond_id
                } else {
                    arch_id
                };
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pick],
                        },
                    )
                    .unwrap();
            }
            p => assert!(walk_past(&mut engine, &p), "unexpected: {p:?}"),
        }
    }
    assert!(
        !engine
            .state()
            .object(diamond_id)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "untapped Archelos caused tap-artifact to enter untapped"
    );

    // 2. Tap Archelos, then cast Fresh Bear -> Fresh Bear MUST enter TAPPED!
    activate(&mut engine, me(), arch_id, 2);
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(arch_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    engine
        .apply(me(), PlayerAction::CastSpell { card: bear_id })
        .unwrap();
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(bear_id)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "tapped Archelos caused Fresh Bear to enter tapped"
    );
}

#[test]
fn banner_of_kinship_and_canker_abomination_resolve_entry_choice_before_counters() {
    static CHOSEN_YOUR_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::MatchesChosenTypeOfSource,
        Filter::ControlledByYou,
    ]);
    static CHOSEN_OPP_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledBy(PlayerRel::ChosenBySource),
    ]);
    static BOOSTER_AB: &[AbilityDef] = &[static_ability!(
        Filter::ControlledByYou,
        Modifier::AdditionalCounterPlacement { n: 1 }
    )];

    static BANNER_MODS: &[EnterModifier] = &[
        EnterModifier::ChooseSubtype,
        EnterModifier::WithCounters {
            kind: QUEST,
            amount: Amount::CountOf {
                filter: &CHOSEN_YOUR_CREATURE,
                zone: ZoneSel::Battlefield,
            },
        },
    ];
    static CANKER_MODS: &[EnterModifier] = &[
        EnterModifier::ChooseOpponent,
        EnterModifier::WithCounters {
            kind: CounterKind::M1M1,
            amount: Amount::CountOf {
                filter: &CHOSEN_OPP_CREATURE,
                zone: ZoneSel::Battlefield,
            },
        },
    ];

    let banner = card(
        W0,
        FaceDef {
            enter_modifiers: BANNER_MODS,
            ..face("Banner Probe", "{0}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let elf = card(
        W1,
        FaceDef {
            subtypes: ELF_SUBTYPES,
            ..creature_face("Llanowar Scout", "{0}", 1, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let canker = card(
        W2,
        FaceDef {
            enter_modifiers: CANKER_MODS,
            ..creature_face("Canker Probe", "{0}", 6, 6)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let booster = card(
        W3,
        face("Counter Amplifier", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        BOOSTER_AB,
    );

    let mut engine = bench(
        16_307,
        vec![banner, elf, canker, booster],
        [
            Seat::with(&[W1, W1, W3]).holding(&[W0, W2]),
            Seat::with(&[W1, W1, W1]),
        ],
    );
    to_main(&mut engine, me());
    let banner_id = the(&engine, ZoneLocation::Hand(me()), W0);
    let canker_id = the(&engine, ZoneLocation::Hand(me()), W2);

    // Cast Banner of Kinship -> choose Elf -> 2 Elves + 1 amplifier = 3 counters!
    engine
        .apply(me(), PlayerAction::CastSpell { card: banner_id })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseSubtype(creature::ELF))
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(banner_id)
            .unwrap()
            .counters
            .get(QUEST),
        3,
        "2 Elves controlled + 1 AdditionalCounterPlacement = 3 counters"
    );

    // Cast Canker Abomination -> choose opponent -> 3 opponent creatures + 1 amplifier = 4 -1/-1 counters -> 2/2!
    engine
        .apply(me(), PlayerAction::CastSpell { card: canker_id })
        .unwrap();
    for _ in 0..20 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me())
        {
            break;
        }
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChoosePlayer(them()))
                    .unwrap();
            }
            p => assert!(walk_past(&mut engine, &p), "unexpected: {p:?}"),
        }
    }
    let canker_obj = engine.state().object(canker_id).unwrap();
    assert_eq!(canker_obj.counters.get(CounterKind::M1M1), 4);
    assert_eq!(
        (
            canker_obj.characteristics().power,
            canker_obj.characteristics().toughness
        ),
        (Some(2), Some(2))
    );
}

#[test]
fn choose_any_player_look_at_opponent_hand_and_day_night_entry_mechanics() {
    static TNN_AB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::ChosenBySource),
        }]
    )];
    static VANDAL_AB: &[AbilityDef] = &[triggered!(
        Trigger::DayNightChanges,
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Player(PlayerRel::EachOpponent),
        }]
    )];

    let tnn = card(
        W0,
        FaceDef {
            enter_modifiers: &[EnterModifier::ChooseAnyPlayer],
            ..creature_face("True-Name Probe", "{0}", 3, 1)
        },
        KeywordSet::EMPTY,
        TNN_AB,
    );
    let spyglass = card(
        W1,
        FaceDef {
            enter_modifiers: &[
                EnterModifier::LookAtOpponentHand,
                EnterModifier::ChooseCardName,
            ],
            ..face("Spyglass Probe", "{0}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let vandal = card(
        W2,
        FaceDef {
            enter_modifiers: &[EnterModifier::BecomeDayIfNeither],
            ..creature_face("Day Vandal", "{0}", 2, 3)
        },
        KeywordSet::EMPTY,
        VANDAL_AB,
    );

    let mut engine = bench(
        16_308,
        vec![tnn, spyglass, vandal],
        [
            Seat::default().holding(&[W0, W1, W2]).at(20),
            Seat::default().holding(&[W0]).at(20),
        ],
    );
    to_main(&mut engine, me());
    let tnn_id = the(&engine, ZoneLocation::Hand(me()), W0);
    let spy_id = the(&engine, ZoneLocation::Hand(me()), W1);
    let vandal_id = the(&engine, ZoneLocation::Hand(me()), W2);
    let opp_card_id = the(&engine, ZoneLocation::Hand(them()), W0);

    // 1. ChooseAnyPlayer allows choosing SELF (me()) as well as opponent!
    engine
        .apply(me(), PlayerAction::CastSpell { card: tnn_id })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    if let Pending::ChoosePlayer { options, .. } = engine.pending() {
        assert_eq!(options, &[me(), them()], "both players are eligible");
    }
    engine
        .apply(me(), PlayerAction::ChoosePlayer(me()))
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        18,
        "PlayerRel::ChosenBySource resolved to self (me())"
    );

    // 2. Spyglass: LookAtOpponentHand shows opponent's hand via LookAtHand prompt,
    //    then prompts ChooseCardName!
    engine
        .apply(me(), PlayerAction::CastSpell { card: spy_id })
        .unwrap();
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChoosePlayer(them()))
                    .unwrap();
            }
            p => assert!(walk_past(&mut engine, &p), "unexpected: {p:?}"),
        }
    }
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            prompt: ChoicePrompt::LookAtHand,
            ..
        } => {
            assert_eq!(*player, me());
            assert_eq!(options, &[opp_card_id], "controller inspects opponent hand");
        }
        other => panic!("expected LookAtHand prompt, got {other:?}"),
    }
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseCardName { .. }));
    engine
        .apply(
            me(),
            PlayerAction::ChooseCardName {
                card: CardIndex::new(W0),
                face: 0,
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // 3. BecomeDayIfNeither + DayNightChanges:
    //    Game starts with day_night == None -> Vandal makes it Day WITHOUT triggering
    //    DayNightChanges (CR 730.2); then flipping Day -> Night DOES trigger it!
    assert_eq!(engine.state().day_night, None);
    engine
        .apply(me(), PlayerAction::CastSpell { card: vandal_id })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().day_night, Some(DayNight::Day));
    assert_eq!(
        life(&engine, them()),
        20,
        "CR 730.2: becoming day from neither must NOT trigger DayNightChanges"
    );

    // Seat 0 cast 3 spells this turn, so seat 1's turn stays Day; pass seat 1's turn
    // with 0 spells so seat 0's next turn flips Day -> Night!
    walk_until(&mut engine, |e| {
        e.state().turn.active == them() && holds_priority_in(e, them(), Step::Main)
    });
    walk_until(&mut engine, |e| {
        e.state().turn.active == me() && holds_priority_in(e, me(), Step::Main)
    });
    assert_eq!(engine.state().day_night, Some(DayNight::Night));
    assert_eq!(
        life(&engine, them()),
        17,
        "flipping Day -> Night triggered DayNightChanges for 3 damage"
    );
}

#[test]
fn ruhan_random_must_attack_until_end_of_combat_and_talion_chosen_number_filters() {
    static RUHAN_AB: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: baylee_cards_dsl::StepKind::CombatBegin,
            whose: PlayerRel::You,
        },
        &[
            Effect::ChoosePlayerAtRandom {
                among: PlayerRel::EachOpponent,
            },
            Effect::AddTempModifier {
                target: TargetSpec::ThisObject,
                modifier: Modifier::MustAttackPlayer(PlayerRel::Chosen),
                duration: Duration::UntilEndOfCombat,
            },
        ]
    )];
    static TALION_AB: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&Filter::And(&[
            Filter::ControlledByOpponent,
            Filter::Or(&[
                Filter::CmcEqualsChosenNumber,
                Filter::PowerEqualsChosenNumber,
                Filter::ToughnessEqualsChosenNumber,
            ]),
        ])),
        &[Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::EventPlayer,
        }]
    )];
    static LOCALIZED_SWEEP_AB: &[AbilityDef] = &[spell!(&[Effect::destroy_all(&Filter::And(&[
        Filter::CREATURE,
        Filter::Not(&Filter::PowerEqualsAmount(&Amount::GreatestPowerAmong(
            &Filter::YOUR_CREATURE,
        ))),
    ]))])];

    let ruhan = card(
        W0,
        creature_face("Ruhan Probe", "{0}", 7, 7),
        KeywordSet::HASTE,
        RUHAN_AB,
    );
    let talion = card(
        W1,
        FaceDef {
            enter_modifiers: &[EnterModifier::ChooseNumber { min: 1, max: 10 }],
            ..creature_face("Talion Probe", "{0}", 3, 4)
        },
        KeywordSet::EMPTY,
        TALION_AB,
    );
    let opp_two_three = card(
        W2,
        creature_face("Two Three", "{1}", 2, 3),
        KeywordSet::FLASH,
        &[],
    );
    let sweep = card(
        W3,
        face("Localized Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        LOCALIZED_SWEEP_AB,
    );

    let run_ruhan = || {
        let mut engine = bench(
            16_309,
            vec![ruhan, talion, opp_two_three, sweep],
            [
                Seat::with(&[W0]).holding(&[W1, W3]),
                Seat::default().holding(&[W2]).at(20),
                Seat::default().at(20),
            ],
        );
        to_main(&mut engine, me());
        let ruhan_id = the(&engine, ZoneLocation::Battlefield, W0);
        let talion_id = the(&engine, ZoneLocation::Hand(me()), W1);
        let sweep_id = the(&engine, ZoneLocation::Hand(me()), W3);

        // Cast Talion choosing 3.
        engine
            .apply(me(), PlayerAction::CastSpell { card: talion_id })
            .unwrap();
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
        settle(&mut engine, me());

        // Move to ChooseAttackers: Ruhan's CombatBegin trigger picked one opponent at random
        // and installed Rider::MustAttackPlayerThisCombat!
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let forced_opp = engine
            .state()
            .object(ruhan_id)
            .unwrap()
            .riders
            .iter()
            .find_map(|r| match r {
                crate::object::Rider::MustAttackPlayerThisCombat(p) => Some(*p),
                _ => None,
            })
            .expect("MustAttackPlayerThisCombat rider installed");
        let other_opp = if forced_opp == PlayerId::new(1) {
            PlayerId::new(2)
        } else {
            PlayerId::new(1)
        };

        // Declaring no attackers or attacking the wrong opponent must fail CR 508.1d!
        assert!(
            engine
                .apply(me(), PlayerAction::DeclareAttackers { attackers: vec![] })
                .is_err(),
            "skipping mandatory attack must be rejected"
        );
        assert!(
            engine
                .apply(
                    me(),
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(ruhan_id, Defender::Player(other_opp))],
                    },
                )
                .is_err(),
            "attacking non-chosen opponent must be rejected"
        );
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(ruhan_id, Defender::Player(forced_opp))],
                },
            )
            .expect("attacking chosen opponent succeeds");

        walk_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
        assert!(
            !engine
                .state()
                .object(ruhan_id)
                .unwrap()
                .riders
                .iter()
                .any(|r| matches!(r, crate::object::Rider::MustAttackPlayerThisCombat(_))),
            "MustAttackPlayerThisCombat rider cleared at end of combat"
        );

        // Verify Talion triggers when seat 1 casts a {1} 2/3 flash creature (toughness == 3)!
        engine.state.players[1]
            .mana_pool
            .add(baylee_core::mana::ManaColor::Colorless, 1);
        engine.apply(me(), PlayerAction::PassPriority).unwrap();
        let flash_c = the(&engine, ZoneLocation::Hand(PlayerId::new(1)), W2);
        let life_before = life(&engine, PlayerId::new(1));
        engine
            .apply(PlayerId::new(1), PlayerAction::CastSpell { card: flash_c })
            .unwrap();
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, PlayerId::new(1)),
            life_before - 2,
            "Talion triggered on toughness == chosen number (3)"
        );

        // Cast Localized Probe: destroys creatures whose power != MaxPower(YOUR_CREATURE) (7),
        // sparing 7/7 Ruhan while destroying 3/4 Talion and 2/3 flash creature!
        engine
            .apply(me(), PlayerAction::CastSpell { card: sweep_id })
            .unwrap();
        settle(&mut engine, me());
        assert_eq!(
            objects(&engine, ZoneLocation::Battlefield, W0),
            vec![ruhan_id]
        );
        assert!(objects(&engine, ZoneLocation::Battlefield, W1).is_empty());
        assert!(objects(&engine, ZoneLocation::Battlefield, W2).is_empty());

        engine.state().snapshot_hash()
    };

    assert_eq!(
        run_ruhan(),
        run_ruhan(),
        "ChoosePlayerAtRandom + MustAttackPlayerThisCombat + ChosenNumber/PowerEqualsAmount filters are deterministic"
    );
}
