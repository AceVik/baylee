//! Rule tests for Wave 3 Command-Zone / Eminence & Commander primitives
//! (`TriggerZone::Command`, `TriggerZone::BattlefieldOrCommand`,
//! `ZoneRef::BattlefieldOrCommand`, `ActivationZone::BattlefieldOrCommand`,
//! `Condition::ControlYourCommander`, `Condition::SourceInZone`,
//! `Filter::YOUR_COMMANDER`, `Filter::COMMANDER_YOU_CONTROL`),
//! `Goad` (`Effect::Goad`, `Effect::GoadAll`, `Modifier::Goaded`, CR 701.15a–d,
//! CR 508.1d, CR 800.4m, CR 400.7), and combat attack keywords
//! (`BATTLE_CRY` CR 702.91, `DETHRONE` CR 702.105, `MELEE` CR 702.121,
//! `MENTOR` CR 702.134, `TRAINING` CR 702.149).

use super::*;
use crate::choice::AnswerFault;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, Duration, Filter, Modifier, PlayerRel, StepKind, TargetSpec,
    Trigger, TriggerZone, ZoneRef, activated, spell, static_ability, triggered,
};
use baylee_core::generated::subtypes::enchantment;
use baylee_core::ids::Defender;
use baylee_core::types::SupertypeSet;

const CARD_A: u32 = 99_910;
const CARD_B: u32 = 99_911;
const CARD_C: u32 = 99_912;
const CARD_D: u32 = 99_913;
const CARD_E: u32 = 99_914;

fn p2() -> PlayerId {
    PlayerId::new(2)
}

fn legendary_creature_face(
    name: &'static str,
    cost: &'static str,
    power: i16,
    toughness: i16,
) -> FaceDef {
    FaceDef {
        supertypes: SupertypeSet::LEGENDARY,
        ..creature_face(name, cost, power, toughness)
    }
}

fn commander_card(
    index: u32,
    face: FaceDef,
    keywords: KeywordSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    assert_ne!(index, forest());
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test-commander",
        scryfall_id: "test-commander",
        faces: Box::leak(Box::new([face])),
        color_identity: baylee_core::color::ColorSet::ALL,
        keywords,
        commander: CommanderRule::Legendary,
        partner: PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

#[derive(Default, Clone)]
struct CommanderSeat {
    commanders: Vec<u32>,
    battlefield: Vec<u32>,
    hand: Vec<u32>,
    life: Option<i32>,
}

fn commander_bench<const N: usize>(
    seed: u64,
    cards: Vec<&'static CardDef>,
    seats: [CommanderSeat; N],
) -> Bench {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let build_seat = |s: CommanderSeat| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: s.commanders.into_iter().map(entry).collect(),
        starting_life: s.life,
        starting_hand: Some(s.hand.into_iter().map(entry).collect()),
        starting_battlefield: s.battlefield.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed,
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
        seats: seats.into_iter().map(build_seat).collect(),
    };
    let mut engine = Engine::new(&preset, SyntheticLookup::new(cards)).expect("commander bench");
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine
            .apply(player, PlayerAction::MulliganKeep)
            .expect("keep mulligan");
    }
    engine
}

// =========================================================================
// 1. Command-Zone / Eminence & Commander Primitives
// =========================================================================

static OLORO_UPKEEP: &[AbilityDef] = &[triggered!(
    Trigger::StepBegin {
        step: StepKind::Upkeep,
        whose: PlayerRel::You,
    },
    &[Effect::GainLife {
        amount: Amount::Fixed(2),
    }],
    zone = TriggerZone::Command,
    condition = Some(Condition::SourceInZone(ZoneRef::Command))
)];

#[test]
fn command_zone_trigger_fires_in_command_not_on_battlefield_and_checks_incarnation() {
    let mut engine = commander_bench(
        9_301,
        vec![commander_card(
            CARD_A,
            legendary_creature_face("Ascetic Sage", "{0}", 4, 5),
            KeywordSet::EMPTY,
            OLORO_UPKEEP,
        )],
        [
            CommanderSeat {
                commanders: vec![CARD_A],
                ..CommanderSeat::default()
            },
            CommanderSeat::default(),
        ],
    );
    let cmd = the(&engine, ZoneLocation::Command(me()), CARD_A);

    // Turn 1 has no upkeep? Wait: Turn 1 DOES have an Upkeep step! Walk to FirstMain
    // and confirm Oloro triggered once in Turn 1's upkeep for +2 life.
    to_main(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);

    // Cast the commander onto the battlefield for {0}.
    engine
        .apply(me(), PlayerAction::CastSpell { card: cmd })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(cmd).unwrap().zone, Zone::Battlefield);

    // Walk to opponent's main, then back to our next FirstMain: while on the
    // battlefield, `TriggerZone::Command` must NOT fire during upkeep.
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        22,
        "TriggerZone::Command must stay silent while the commander is on the battlefield"
    );

    // Move commander back to Command zone, walk to opponent's main, then stop in
    // our next upkeep while the trigger is on the stack!
    engine
        .dev_state_mut(me())
        .unwrap()
        .move_object(
            cmd,
            ZoneLocation::Command(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    to_main(&mut engine, them());
    walk_until(&mut engine, |e| {
        e.state().turn.active == me()
            && e.state().turn.step == crate::turn::Step::Upkeep
            && !e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });

    // Bounce the commander to Battlefield and back to Command before the trigger
    // resolves: it is now a new incarnation (CR 400.7), so CR 603.4's
    // `Condition::SourceInZone(ZoneRef::Command)` must fail on resolution!
    engine
        .dev_state_mut(me())
        .unwrap()
        .move_object(
            cmd,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .dev_state_mut(me())
        .unwrap()
        .move_object(
            cmd,
            ZoneLocation::Command(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    to_main(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        22,
        "a new incarnation in the command zone must not satisfy the old trigger's SourceInZone check"
    );
}

static EMINENCE_COMMANDER: &[AbilityDef] = &[
    static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::ModifyPT(1, 1),
        zone = ZoneRef::BattlefieldOrCommand
    ),
    triggered!(
        Trigger::SpellCast(&Filter::CREATURE),
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        zone = TriggerZone::BattlefieldOrCommand,
        condition = Some(Condition::SourceInZone(ZoneRef::BattlefieldOrCommand))
    ),
    triggered!(
        Trigger::Draws(PlayerRel::You),
        &[Effect::GainLife {
            amount: Amount::Fixed(10),
        }],
        zone = TriggerZone::BattlefieldOrCommand
    ),
    activated!(
        Cost::FREE,
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        zone = ActivationZone::BattlefieldOrCommand,
        limit = ActivationLimit::PerTurn(1)
    ),
];

#[test]
fn eminence_static_trigger_and_activation_work_in_command_and_battlefield_not_on_stack() {
    let mut engine = commander_bench(
        9_302,
        vec![
            commander_card(
                CARD_A,
                legendary_creature_face("Eminence Lord", "{0}", 3, 3),
                KeywordSet::EMPTY,
                EMINENCE_COMMANDER,
            ),
            card(
                CARD_B,
                creature_face("Squire", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Recruit", "{0}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            CommanderSeat {
                commanders: vec![CARD_A],
                battlefield: vec![CARD_B],
                hand: vec![CARD_C],
                life: None,
            },
            CommanderSeat::default(),
        ],
    );
    let cmd = the(&engine, ZoneLocation::Command(me()), CARD_A);
    let squire = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let recruit = the(&engine, ZoneLocation::Hand(me()), CARD_C);

    to_main(&mut engine, me());
    // Opening-hand draw before Turn 1 must NOT have fired the command-zone draw trigger!
    // (Turn 1 draw step is skipped for starting player in 1v1.)
    assert_eq!(life(&engine, me()), 20);

    // 1) In Command zone: anthem pumps Squire from 2/2 -> 3/3.
    let chars = engine.state().object(squire).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(3)));

    // 2) Activated ability with `ActivationZone::BattlefieldOrCommand` is offered & fires from Command.
    activate(&mut engine, me(), cmd, 3);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 21);

    // 3) Casting Recruit triggers Eminence from Command (+3 life -> 24).
    engine
        .apply(me(), PlayerAction::CastSpell { card: recruit })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);

    // 4) Cast the commander itself: while on the Stack (neither Command nor Battlefield),
    //    its Eminence anthem does NOT apply (Squire drops back to 2/2), nor did casting
    //    itself see it in Command or Battlefield!
    engine
        .apply(me(), PlayerAction::CastSpell { card: cmd })
        .unwrap();
    assert_eq!(engine.state().object(cmd).unwrap().zone, Zone::Stack);
    let mid_cast = engine.state().object(squire).unwrap().characteristics();
    assert_eq!(
        (mid_cast.power, mid_cast.toughness),
        (Some(2), Some(2)),
        "Eminence static must turn off while the commander is on the stack"
    );
    settle(&mut engine, me());
    assert_eq!(engine.state().object(cmd).unwrap().zone, Zone::Battlefield);

    // 5) Once on the Battlefield: anthem applies again (Squire 3/3, Commander 4/4),
    //    and on the next turn the activated ability works from Battlefield too.
    let post = engine.state().object(squire).unwrap().characteristics();
    assert_eq!((post.power, post.toughness), (Some(3), Some(3)));
    to_main(&mut engine, them());
    // Our turn 2 draw fires the Eminence draw trigger on the battlefield (+10 life -> 34).
    to_main(&mut engine, me());
    assert_eq!(life(&engine, me()), 34);
    activate(&mut engine, me(), cmd, 3);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 35);
}

static LIEUTENANT_ABILITIES: &[AbilityDef] = &[
    static_ability!(
        Filter::This,
        Modifier::ModifyPT(2, 2),
        condition = Some(Condition::ControlYourCommander)
    ),
    static_ability!(Filter::YOUR_COMMANDER, Modifier::ModifyPT(1, 0)),
    static_ability!(Filter::COMMANDER_YOU_CONTROL, Modifier::ModifyPT(0, 1)),
];

static PHASE_OUT_TARGET: &[AbilityDef] = &[
    free(
        &[Effect::PhaseOut {
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
    free(
        &[Effect::ChangeController {
            new_controller: PlayerRel::You,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
];

fn target_obj(engine: &mut Bench, seat: PlayerId, obj: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![obj],
                players: vec![],
            },
        )
        .unwrap();
}

#[test]
fn control_your_commander_distinguishes_own_stolen_and_phased_out_commanders() {
    let mut engine = commander_bench(
        9_303,
        vec![
            commander_card(
                CARD_A,
                legendary_creature_face("My General", "{0}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
            commander_card(
                CARD_B,
                legendary_creature_face("Rival General", "{0}", 4, 4),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Loyal Lieutenant", "{0}", 2, 2),
                KeywordSet::EMPTY,
                LIEUTENANT_ABILITIES,
            ),
            card(
                CARD_D,
                face("Phaser", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                PHASE_OUT_TARGET,
            ),
        ],
        [
            CommanderSeat {
                commanders: vec![CARD_A],
                battlefield: vec![CARD_C, CARD_D],
                ..CommanderSeat::default()
            },
            CommanderSeat {
                commanders: vec![CARD_B],
                ..CommanderSeat::default()
            },
        ],
    );
    let my_cmd = the(&engine, ZoneLocation::Command(me()), CARD_A);
    let their_cmd = the(&engine, ZoneLocation::Command(them()), CARD_B);
    let lt = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let phaser = the(&engine, ZoneLocation::Battlefield, CARD_D);

    to_main(&mut engine, me());
    assert_eq!(
        engine.state().object(lt).unwrap().characteristics().power,
        Some(2)
    );

    // Opponent casts their commander, then we steal it onto our side of the battlefield:
    // `COMMANDER_YOU_CONTROL` gives it +0/+1 (4/5), `YOUR_COMMANDER` does NOT (+0/+0),
    // and `Condition::ControlYourCommander` stays false (Lieutenant stays 2/2).
    to_main(&mut engine, them());
    engine
        .apply(them(), PlayerAction::CastSpell { card: their_cmd })
        .unwrap();
    settle(&mut engine, them());
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, me(), phaser, 1);
    target_obj(&mut engine, me(), their_cmd);
    to_main(&mut engine, me());
    assert_eq!(
        engine.state().object(lt).unwrap().characteristics().power,
        Some(2),
        "controlling an opponent's commander does not satisfy ControlYourCommander"
    );
    let rival_chars = engine.state().object(their_cmd).unwrap().characteristics();
    assert_eq!(
        (rival_chars.power, rival_chars.toughness),
        (Some(4), Some(5))
    );

    // Cast our own commander onto the battlefield: Lieutenant becomes 4/4, and our
    // commander gets BOTH `YOUR_COMMANDER` (+1/+0) and `COMMANDER_YOU_CONTROL` (+0/+1) -> 4/4.
    engine
        .apply(me(), PlayerAction::CastSpell { card: my_cmd })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(lt).unwrap().characteristics().power,
        Some(4)
    );
    let my_chars = engine.state().object(my_cmd).unwrap().characteristics();
    assert_eq!((my_chars.power, my_chars.toughness), (Some(4), Some(4)));

    // Phase out our commander: treated as though it doesn't exist (CR 702.26b).
    activate(&mut engine, me(), phaser, 0);
    target_obj(&mut engine, me(), my_cmd);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(lt).unwrap().characteristics().power,
        Some(2),
        "phased-out commander does not count as controlled"
    );
}

// =========================================================================
// 2. Goad (CR 701.15a–d, CR 508.1d, CR 800.4m, CR 400.7)
// =========================================================================

static GOAD_TARGET: &[AbilityDef] = &[free(
    &[Effect::goad_target()],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

static GOAD_ALL_OPPONENTS: &[AbilityDef] = &[free(
    &[Effect::GoadAll {
        filter: &Filter::CREATURE,
        controlled_by: Some(PlayerRel::Opponent),
    }],
    None,
)];

static BLINK_TARGET: &[AbilityDef] = &[free(
    &[Effect::Blink {
        target: TargetSpec::Object(&Filter::CREATURE),
        owner_control: true,
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

static PRISON_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::CREATURE,
    Modifier::AttackTaxMana { mana: 2 }
)];

static ARBITER_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::CREATURE,
    Modifier::MaxAttackersPerCombat(1)
)];

static MUST_ATTACK_ABILITIES: &[AbilityDef] =
    &[static_ability!(Filter::This, Modifier::AttacksEachCombat)];

static GOAD_AURA_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Spell {
        effects: &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    },
    static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(2, 2)),
    static_ability!(ENCHANTED_CREATURE, Modifier::Goaded),
];

#[test]
fn goad_in_duel_forces_attack_and_expires_at_goaders_next_turn() {
    let mut engine = bench(
        9_310,
        vec![
            card(
                CARD_A,
                face("Taunt Rod", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GOAD_TARGET,
            ),
            card(
                CARD_B,
                creature_face("Brawler", "{0}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[CARD_A]), Seat::with(&[CARD_B])],
    );
    let rod = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let brawler = the(&engine, ZoneLocation::Battlefield, CARD_B);

    to_main(&mut engine, me());
    activate(&mut engine, me(), rod, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![brawler],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Advance to `them()`'s declare-attackers step: staying home is illegal!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    let err = engine
        .apply(them(), PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap_err();
    assert!(matches!(
        err,
        crate::engine::EngineError::IllegalAction(msg) if msg == AnswerFault::MustAttack.reason()
    ));
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(brawler, Defender::Player(me()))],
            },
        )
        .unwrap();

    // Walk to `me()`'s next main phase (expiring `me()`'s goad per CR 701.15a),
    // then to `them()`'s next declare-attackers step: now `[]` is legal!
    to_main(&mut engine, me());
    assert!(engine.state().goaded.is_empty());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    engine
        .apply(them(), PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
}

fn multiplayer_goad_bench(seed: u64) -> Bench {
    let mut walker_face = face("Bystander Walker", "{0}", TypeSet::PLANESWALKER);
    walker_face.loyalty = Some(4);
    bench(
        seed,
        vec![
            card(
                CARD_A,
                face("Mass Taunt", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GOAD_ALL_OPPONENTS,
            ),
            card(
                CARD_B,
                creature_face("Brute", "{0}", 4, 4),
                KeywordSet::EMPTY,
                &[],
            ),
            card(CARD_C, walker_face, KeywordSet::EMPTY, &[]),
            card(
                CARD_D,
                face("Second Taunt", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GOAD_TARGET,
            ),
        ],
        [
            Seat::with(&[CARD_A]),
            Seat::with(&[CARD_B]),
            Seat::with(&[CARD_C, CARD_D]),
        ],
    )
}

#[test]
fn goad_in_multiplayer_forces_non_goader_player_and_handles_multiple_goaders() {
    let mut engine = multiplayer_goad_bench(9_311);
    let taunt_all = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let brute = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let walker = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let second_taunt = the(&engine, ZoneLocation::Battlefield, CARD_D);

    // P0 goads all opposing creatures twice (CR 701.15d: idempotent per goader).
    to_main(&mut engine, me());
    activate(&mut engine, me(), taunt_all, 0);
    settle(&mut engine, me());
    activate(&mut engine, me(), taunt_all, 0);
    settle(&mut engine, me());
    assert_eq!(
        crate::combat::goad::goaders_of(engine.state(), brute).len(),
        1
    );

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    assert!(
        engine
            .apply(them(), PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(brute, Defender::Player(me()))],
                },
            )
            .is_err(),
        "goaded creature cannot attack its goader when another player can be attacked"
    );
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(brute, Defender::Planeswalker(walker))],
                },
            )
            .is_err(),
        "CR 701.15b requires attacking a player other than the goader, not a planeswalker"
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(brute, Defender::Player(p2()))],
            },
        )
        .unwrap();

    // Next round: BOTH P0 and P2 goad Brute (CR 701.15c). Either player obeys 3
    // requirements while P2's planeswalker obeys only 2.
    to_main(&mut engine, me());
    activate(&mut engine, me(), taunt_all, 0);
    settle(&mut engine, me());
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, p2(), second_taunt, 0);
    target_obj(&mut engine, p2(), brute);
    settle(&mut engine, me());
    assert_eq!(
        crate::combat::goad::goaders_of(engine.state(), brute).len(),
        2
    );

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(brute, Defender::Planeswalker(walker))],
                },
            )
            .is_err(),
        "planeswalker obeys 2 requirements while either player obeys 3"
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(brute, Defender::Player(me()))],
            },
        )
        .unwrap();
}

#[test]
fn goad_respects_attack_taxes() {
    let mut taxed = bench(
        9_312,
        vec![
            card(
                CARD_A,
                face("Taunt Rod", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GOAD_TARGET,
            ),
            card(
                CARD_B,
                creature_face("Brute", "{0}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_C,
                face("Tax Ward", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                PRISON_ABILITIES,
            ),
        ],
        [
            Seat::with(&[CARD_A]),
            Seat::with(&[CARD_B]),
            Seat::with(&[CARD_C]),
        ],
    );
    let rod = the(&taxed, ZoneLocation::Battlefield, CARD_A);
    let brute = the(&taxed, ZoneLocation::Battlefield, CARD_B);
    to_main(&mut taxed, me());
    activate(&mut taxed, me(), rod, 0);
    target_obj(&mut taxed, me(), brute);
    settle(&mut taxed, me());
    walk_until(
        &mut taxed,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    assert!(
        taxed
            .apply(them(), PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
    taxed
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(brute, Defender::Player(me()))],
            },
        )
        .unwrap();
}

#[test]
fn goad_beats_plain_must_attack_under_attacker_cap() {
    let mut capped = bench(
        9_313,
        vec![
            card(
                CARD_A,
                face("Taunt Rod", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GOAD_TARGET,
            ),
            card(
                CARD_B,
                creature_face("Goaded One", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Berserker", "{0}", 2, 2),
                KeywordSet::EMPTY,
                MUST_ATTACK_ABILITIES,
            ),
            card(
                CARD_D,
                face("Single File", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                ARBITER_ABILITIES,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_D]),
            Seat::with(&[CARD_B, CARD_C]),
            Seat::with(&[]),
        ],
    );
    let rod = the(&capped, ZoneLocation::Battlefield, CARD_A);
    let goaded_one = the(&capped, ZoneLocation::Battlefield, CARD_B);
    let berserker = the(&capped, ZoneLocation::Battlefield, CARD_C);
    to_main(&mut capped, me());
    activate(&mut capped, me(), rod, 0);
    target_obj(&mut capped, me(), goaded_one);
    settle(&mut capped, me());
    walk_until(
        &mut capped,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    assert!(
        capped
            .apply(
                them(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(berserker, Defender::Player(p2()))],
                },
            )
            .is_err(),
        "Berserker obeys 1 requirement while Goaded One obeys 2"
    );
    capped
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(goaded_one, Defender::Player(p2()))],
            },
        )
        .unwrap();
}

#[test]
fn goad_resets_on_blink_and_persists_past_eliminated_goader() {
    let mut elim = bench(
        9_314,
        vec![
            card(
                CARD_A,
                face("Taunt Rod", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GOAD_TARGET,
            ),
            card(
                CARD_B,
                face("Blinker", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                BLINK_TARGET,
            ),
            card(
                CARD_C,
                creature_face("Survivor", "{0}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[CARD_B]),
            Seat::with(&[CARD_A]),
            Seat::with(&[CARD_C]),
        ],
    );
    let blinker = the(&elim, ZoneLocation::Battlefield, CARD_B);
    let rod_p1 = the(&elim, ZoneLocation::Battlefield, CARD_A);
    let survivor = the(&elim, ZoneLocation::Battlefield, CARD_C);

    to_main(&mut elim, them());
    activate(&mut elim, them(), rod_p1, 0);
    target_obj(&mut elim, them(), survivor);
    settle(&mut elim, them());
    assert_eq!(
        crate::combat::goad::goaders_of(elim.state(), survivor).len(),
        1
    );

    elim.apply(them(), PlayerAction::PassPriority).unwrap();
    elim.apply(p2(), PlayerAction::PassPriority).unwrap();
    activate(&mut elim, me(), blinker, 0);
    target_obj(&mut elim, me(), survivor);
    settle(&mut elim, them());
    assert!(crate::combat::goad::goaders_of(elim.state(), survivor).is_empty());

    activate(&mut elim, them(), rod_p1, 0);
    target_obj(&mut elim, them(), survivor);
    settle(&mut elim, them());
    elim.dev_state_mut(me()).unwrap().players[1].life = 0;
    elim.apply(them(), PlayerAction::PassPriority).unwrap();

    walk_until(
        &mut elim,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p2()),
    );
    assert!(
        elim.apply(p2(), PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "CR 800.4m: eliminated goader's goad persists until their next turn would have begun"
    );
    elim.apply(
        p2(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(survivor, Defender::Player(me()))],
        },
    )
    .unwrap();

    to_main(&mut elim, me());
    walk_until(
        &mut elim,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p2()),
    );
    assert!(elim.state().goaded.is_empty());
    elim.apply(p2(), PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
}

#[test]
fn goad_static_aura_is_harmful_and_forces_attack() {
    assert_eq!(
        crate::target_side::aura_side(GOAD_AURA_ABILITIES),
        Some(baylee_core::types::TargetSide::Harmful)
    );
    let mut aura_face = face("Shiny Vow", "{0}", TypeSet::ENCHANTMENT);
    aura_face.subtypes = Box::leak(Box::new([enchantment::AURA]));
    let mut engine = bench(
        9_315,
        vec![
            card(CARD_A, aura_face, KeywordSet::EMPTY, GOAD_AURA_ABILITIES),
            card(
                CARD_B,
                creature_face("Survivor", "{0}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::default().holding(&[CARD_A]), Seat::with(&[CARD_B])],
    );
    let aura = the(&engine, ZoneLocation::Hand(me()), CARD_A);
    let survivor = the(&engine, ZoneLocation::Battlefield, CARD_B);
    to_main(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: aura })
        .unwrap();
    target_obj(&mut engine, me(), survivor);
    settle(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    assert!(
        engine
            .apply(them(), PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
}

// =========================================================================
// 3. Combat Keywords (Battle cry, Dethrone, Melee, Mentor, Training)
// =========================================================================

static GRANT_SECOND_BATTLE_CRY: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::AddKeyword(KeywordSet::BATTLE_CRY)
)];

#[test]
fn battle_cry_pumps_other_attackers_only_and_stacks_per_instance() {
    let mut engine = bench(
        9_320,
        vec![
            card(
                CARD_A,
                creature_face("Warchief", "{0}", 2, 2),
                KeywordSet::BATTLE_CRY,
                GRANT_SECOND_BATTLE_CRY,
            ),
            card(
                CARD_B,
                creature_face("Raider", "{0}", 1, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Watcher", "{0}", 1, 3),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[CARD_A, CARD_B, CARD_C]), Seat::with(&[])],
    );
    let warchief = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let raider = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let watcher = the(&engine, ZoneLocation::Battlefield, CARD_C);

    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (warchief, Defender::Player(them())),
                    (raider, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Two instances of battle cry give Raider +2/+0 (3/2), Warchief +0/+0 (2/2),
    // and non-attacking Watcher +0/+0 (1/3).
    assert_eq!(
        engine
            .state()
            .object(raider)
            .unwrap()
            .characteristics()
            .power,
        Some(3)
    );
    assert_eq!(
        engine
            .state()
            .object(warchief)
            .unwrap()
            .characteristics()
            .power,
        Some(2)
    );
    assert_eq!(
        engine
            .state()
            .object(watcher)
            .unwrap()
            .characteristics()
            .power,
        Some(1)
    );
}

#[test]
fn dethrone_triggers_on_most_or_tied_life_player_not_lower_life_or_planeswalker() {
    let mut walker_face = face("Crown Walker", "{0}", TypeSet::PLANESWALKER);
    walker_face.loyalty = Some(5);
    let mut engine = bench(
        9_321,
        vec![
            card(
                CARD_A,
                creature_face("Usurper", "{0}", 2, 2),
                KeywordSet::DETHRONE.union(KeywordSet::VIGILANCE),
                &[],
            ),
            card(CARD_B, walker_face, KeywordSet::EMPTY, &[]),
        ],
        [
            Seat::with(&[CARD_A]).at(20),
            Seat::with(&[CARD_B]).at(20),
            Seat::with(&[]).at(15),
        ],
    );
    let usurper = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let walker = the(&engine, ZoneLocation::Battlefield, CARD_B);

    // Round 1: P0 (20) and P1 (20) are tied for most life, P2 has 15.
    // Attacking P2 (15 < 20) does NOT trigger dethrone!
    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(usurper, Defender::Player(p2()))],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(usurper)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // Round 2: Attacking P1's planeswalker does NOT trigger dethrone (CR 702.105a)!
    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(usurper, Defender::Planeswalker(walker))],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(usurper)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // Round 3: Attacking P1 directly (tied for most life at 20 == 20) DOES trigger dethrone!
    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(usurper, Defender::Player(them()))],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(usurper)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
}

#[test]
fn melee_counts_distinct_attacked_opponents_ignores_walkers_and_persists_until_eot() {
    let mut walker_face = face("Side Walker", "{0}", TypeSet::PLANESWALKER);
    walker_face.loyalty = Some(5);
    let mut engine = bench(
        9_322,
        vec![
            card(
                CARD_A,
                creature_face("Duelist", "{0}", 2, 2),
                KeywordSet::MELEE,
                &[],
            ),
            card(
                CARD_B,
                creature_face("Scout", "{0}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Flanker", "{0}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(CARD_D, walker_face, KeywordSet::EMPTY, &[]),
        ],
        [
            Seat::with(&[CARD_A, CARD_B, CARD_C]),
            Seat::with(&[]),
            Seat::with(&[CARD_D]),
        ],
    );
    let duelist = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let scout = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let flanker = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let walker = the(&engine, ZoneLocation::Battlefield, CARD_D);

    // Duelist + Scout both attack Player(P1), while Flanker attacks P2's planeswalker:
    // CR 702.121a counts distinct *opponents* attacked (not walkers, and not duplicate
    // creatures on the same opponent), so count == 1 -> Duelist becomes 3/3!
    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (duelist, Defender::Player(them())),
                    (scout, Defender::Player(them())),
                    (flanker, Defender::Planeswalker(walker)),
                ],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let chars = engine.state().object(duelist).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(3)));

    // Walk to SecondMain: Melee's +1/+1 is `UntilEndOfTurn`, so it persists after combat!
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Main)
            && e.state().turn.phase == Phase::SecondMain
    });
    let chars_main2 = engine.state().object(duelist).unwrap().characteristics();
    assert_eq!(
        (chars_main2.power, chars_main2.toughness),
        (Some(3), Some(3))
    );

    // Next round: Duelist attacks P1 and Scout attacks P2 -> 2 opponents -> +2/+2 (4/4)!
    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (duelist, Defender::Player(them())),
                    (scout, Defender::Player(p2())),
                ],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let chars_two = engine.state().object(duelist).unwrap().characteristics();
    assert_eq!((chars_two.power, chars_two.toughness), (Some(4), Some(4)));
}

static PUMP_TARGET_PLUS_TWO: &[AbilityDef] = &[spell!(
    &[Effect::PumpTarget {
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(2),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
)];

static DESTROY_TARGET: &[AbilityDef] = &[free(
    &[Effect::Destroy {
        target: TargetSpec::Object(&Filter::CREATURE),
        no_regen: false,
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

fn mentor_bench(seed: u64) -> Bench {
    bench(
        seed,
        vec![
            card(
                CARD_A,
                creature_face("Veteran Mentor", "{0}", 2, 2),
                KeywordSet::MENTOR.union(KeywordSet::VIGILANCE),
                &[],
            ),
            card(
                CARD_B,
                creature_face("Pupil", "{0}", 1, 1),
                KeywordSet::VIGILANCE,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Senior Pupil", "{0}", 3, 3),
                KeywordSet::VIGILANCE,
                &[],
            ),
            card(
                CARD_D,
                face("Battle Lesson", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                PUMP_TARGET_PLUS_TWO,
            ),
            card(
                CARD_E,
                face("Sniper", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                DESTROY_TARGET,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B, CARD_C]).holding(&[CARD_D]),
            Seat::with(&[CARD_E]),
        ],
    )
}

#[test]
fn mentor_fizzles_when_target_grows_before_resolution() {
    let mut engine = mentor_bench(9_323);
    let mentor = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let pupil = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let pump = the(&engine, ZoneLocation::Hand(me()), CARD_D);

    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (mentor, Defender::Player(them())),
                    (pupil, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    target_obj(&mut engine, me(), pupil);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pump })
        .unwrap();
    target_obj(&mut engine, me(), pupil);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(pupil)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "Mentor must fizzle when target's power rises to >= Mentor's power before resolution"
    );
}

#[test]
fn mentor_uses_source_lki_power_when_mentor_dies_in_response() {
    let mut engine = mentor_bench(9_324);
    let mentor = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let senior = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let sniper = the(&engine, ZoneLocation::Battlefield, CARD_E);
    let pump = the(&engine, ZoneLocation::Hand(me()), CARD_D);

    // Pump Mentor (2/2 -> 4/4) BEFORE combat, attack with Mentor (4/4) and Senior Pupil
    // (3/3) targeting Senior Pupil, then opponent destroys Mentor in response.
    // In the graveyard Mentor's printed power is 2 (<= 3), so only last-known battlefield
    // power (4 > 3, CR 608.2h) lets Mentor resolve and place the counter.
    to_main(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: pump })
        .unwrap();
    target_obj(&mut engine, me(), mentor);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(mentor)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (mentor, Defender::Player(them())),
                    (senior, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    target_obj(&mut engine, me(), senior);

    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), sniper, 0);
    target_obj(&mut engine, them(), mentor);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(mentor).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine
            .state()
            .object(senior)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "CR 608.2h: Mentor that died as a 4/4 still mentors a 3/3 despite printed 2 power in graveyard"
    );
}

static GRANT_SECOND_TRAINING: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::AddKeyword(KeywordSet::TRAINING)
)];

#[test]
fn training_triggers_only_with_strictly_greater_power_co_attacker_and_stacks() {
    let mut engine = bench(
        9_324,
        vec![
            card(
                CARD_A,
                creature_face("Initiate", "{0}", 2, 2),
                KeywordSet::TRAINING.union(KeywordSet::VIGILANCE),
                GRANT_SECOND_TRAINING,
            ),
            card(
                CARD_B,
                creature_face("Peer", "{0}", 2, 2),
                KeywordSet::VIGILANCE,
                &[],
            ),
            card(
                CARD_C,
                creature_face("Champion", "{0}", 3, 3),
                KeywordSet::VIGILANCE,
                &[],
            ),
        ],
        [Seat::with(&[CARD_A, CARD_B, CARD_C]), Seat::with(&[])],
    );
    let initiate = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let peer = the(&engine, ZoneLocation::Battlefield, CARD_B);
    let champion = the(&engine, ZoneLocation::Battlefield, CARD_C);

    // Round 1: Attacking alongside equal-power Peer (2 == 2) does NOT trigger Training.
    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (initiate, Defender::Player(them())),
                    (peer, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(initiate)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // Round 2: Attacking alongside Champion (3 > 2) triggers BOTH instances of Training -> 2 counters!
    to_main(&mut engine, me());
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (initiate, Defender::Player(them())),
                    (champion, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(initiate)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
}
