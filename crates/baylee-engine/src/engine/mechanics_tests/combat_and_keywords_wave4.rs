//! Rule tests for Wave 4 combat & keyword primitives:
//! - Ninjutsu and commander ninjutsu (`CostPart::ReturnUnblockedAttacker`,
//!   `Condition::InCombatAfterBlockers`, `ActivationZone::HandOrCommand`,
//!   `Effect::PutNinjutsuOntoBattlefield`, CR 702.49a–d, CR 508.4a/c/d, CR 903.8)
//! - Myriad (`KeywordSet::MYRIAD`, `Effect::Myriad`, CR 702.116a–b)
//! - Extort (`KeywordSet::EXTORT`, `ThisWay::LifeLost`, CR 702.101a–b)
//! - Exert (`Effect::ExertSource`, `Trigger::Exerted`, `ReflexiveEvent::ExertedThis`,
//!   CR 701.43a–d, CR 603.12)

use super::*;
use crate::choice::{ChoicePrompt, YesNoPrompt};
use crate::event::{Cause, GameEvent};
use crate::object::Status;
use crate::turn::Step;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    Amount, Condition, Cost, CostPart, Duration, Filter, Layer, ManaColor, Modifier, PlayerRel,
    ReflexiveEvent, TargetSpec, Trigger, activated, mana_ability, static_ability, triggered,
};
use baylee_core::ids::Defender;
use baylee_core::mana;
use baylee_core::types::SupertypeSet;

const CARD_A: u32 = 99_940;
const CARD_B: u32 = 99_941;
const CARD_C: u32 = 99_942;
const CARD_D: u32 = 99_943;
const CARD_E: u32 = 99_944;

fn p2() -> PlayerId {
    PlayerId::new(2)
}

fn p3() -> PlayerId {
    PlayerId::new(3)
}

fn is_tapped(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

fn declare_attackers_step(engine: &mut Bench, seat: PlayerId, pairs: Vec<(ObjectId, Defender)>) {
    walk_until(engine, |e| {
        e.state().turn.active == seat
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat)
    });
    engine
        .apply(seat, PlayerAction::DeclareAttackers { attackers: pairs })
        .expect("declare attackers");
}

fn declare_blockers_step(
    engine: &mut Bench,
    defender: PlayerId,
    blocks: Vec<(ObjectId, ObjectId)>,
) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == defender),
    );
    engine
        .apply(defender, PlayerAction::DeclareBlockers { blockers: blocks })
        .expect("declare blockers");
}

fn activate_ninjutsu(engine: &mut Bench, seat: PlayerId, ninja: ObjectId, bounce: ObjectId) {
    activate(engine, seat, ninja, 0);
    let Pending::ChooseCards {
        player,
        prompt: ChoicePrompt::CostReturn,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostReturn prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&bounce), "{bounce:?} not in {options:?}");
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![bounce],
            },
        )
        .expect("return unblocked attacker");
}

// =========================================================================
// 1. Ninjutsu & Commander Ninjutsu (CR 702.49a–d, CR 508.4a/c/d, CR 903.8)
// =========================================================================

static NINJUTSU_HAND_COST: Cost = Cost {
    mana: mana!("{1}{U}"),
    parts: &[CostPart::ReturnUnblockedAttacker],
};

static NINJUTSU_CMD_COST: Cost = Cost {
    mana: mana!("{1}{U}"),
    parts: &[CostPart::ReturnUnblockedAttacker],
};

static BLUE_ROCK_ABILITIES: &[AbilityDef] = &[mana_ability!(
    Cost::TAP,
    &[Effect::mana(ManaColor::Blue, 2)]
)];

static HAND_NINJA_ABILITIES: &[AbilityDef] = &[
    activated!(
        NINJUTSU_HAND_COST,
        &[Effect::PutNinjutsuOntoBattlefield],
        zone = ActivationZone::Hand,
        condition = Some(Condition::InCombatAfterBlockers)
    ),
    triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::Fixed(7),
        }]
    ),
    triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::draw(1)]
    ),
];

static COMMANDER_NINJA_ABILITIES: &[AbilityDef] = &[activated!(
    NINJUTSU_CMD_COST,
    &[Effect::PutNinjutsuOntoBattlefield],
    zone = ActivationZone::HandOrCommand,
    condition = Some(Condition::InCombatAfterBlockers)
)];

fn ninjutsu_duel_bench(seed: u64, opp_battlefield: &[u32]) -> Bench {
    bench(
        seed,
        vec![
            card(
                CARD_A,
                creature_face("Scout", "{0}", 1, 1),
                KeywordSet::HASTE,
                &[],
            ),
            card(
                CARD_B,
                creature_face("Mist Shinobi", "{3}{U}", 3, 2),
                KeywordSet::EMPTY,
                HAND_NINJA_ABILITIES,
            ),
            card(
                CARD_C,
                face("Sapphire Signet", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                BLUE_ROCK_ABILITIES,
            ),
            card(
                CARD_D,
                creature_face("Wall Guard", "{0}", 0, 4),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_C]).holding(&[CARD_B]),
            Seat::with(opp_battlefield),
        ],
    )
}

#[test]
fn ninjutsu_refused_before_blockers_and_when_all_attackers_are_blocked() {
    // CR 702.49a: Ninjutsu may be activated only after blockers have been declared.
    let mut engine = ninjutsu_duel_bench(401, &[CARD_D]);
    let scout = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let ninja = the(&engine, ZoneLocation::Hand(me()), CARD_B);

    declare_attackers_step(&mut engine, me(), vec![(scout, Defender::Player(them()))]);
    let (_, pre_block_legal) = offer(&engine);
    assert!(
        !pre_block_legal.abilities.contains(&(ninja, 0)),
        "ninjutsu must not be offered during DeclareAttackers"
    );

    let guard = the(&engine, ZoneLocation::Battlefield, CARD_D);
    declare_blockers_step(&mut engine, them(), vec![(guard, scout)]);
    settle(&mut engine, me());
    let (_, blocked_legal) = offer(&engine);
    assert!(
        !blocked_legal.abilities.contains(&(ninja, 0)),
        "ninjutsu must not be offered when every attacker is blocked"
    );
}

#[test]
fn ninjutsu_from_hand_swaps_unblocked_attacker_and_skips_attack_triggers() {
    // CR 702.49a–c: Reveal Ninja, bounce unblocked attacker, enter tapped and attacking.
    // CR 508.4: Entering attacking does not trigger "Whenever ~ attacks" abilities.
    let mut engine = ninjutsu_duel_bench(402, &[]);
    let scout = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let signet = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let ninja = the(&engine, ZoneLocation::Hand(me()), CARD_B);

    declare_attackers_step(&mut engine, me(), vec![(scout, Defender::Player(them()))]);
    declare_blockers_step(&mut engine, them(), vec![]);
    settle(&mut engine, me());

    let (_, pre_mana_legal) = offer(&engine);
    assert!(
        pre_mana_legal
            .unpaid_abilities
            .contains(&(ninja, 0, NINJUTSU_HAND_COST.mana)),
        "ninjutsu appears in unpaid_abilities before mana is floated"
    );
    activate(&mut engine, me(), signet, 0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate_ninjutsu(&mut engine, me(), ninja, scout);

    assert_eq!(engine.state().object(scout).unwrap().zone, Zone::Hand);
    assert!(
        engine.journal().entries().iter().any(|e| matches!(
            &e.event,
            GameEvent::Revealed { cards, .. } if cards.contains(&ninja)
        )),
        "activating ninjutsu reveals the Ninja card from hand"
    );

    settle(&mut engine, me());
    let bf_ninja = the(&engine, ZoneLocation::Battlefield, CARD_B);
    assert!(is_tapped(&engine, bf_ninja));
    assert_eq!(
        engine.state().combat.defender_of(bf_ninja),
        Some(Defender::Player(them()))
    );
    assert_eq!(life(&engine, me()), 20, "attack trigger must not fire");

    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::CombatEnd));
    assert_eq!(life(&engine, them()), 17, "3/2 Ninja dealt combat damage");
    let hand_after = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "scout returned (+1), ninja entered (-1), combat-damage draw (+1)"
    );
}

#[test]
fn ninjutsu_works_in_combat_end_and_enters_not_attacking_if_planeswalker_left() {
    // CR 702.49c: If the attacked planeswalker left combat before resolution, enter tapped and not attacking.
    static EXILE_TARGET: &[AbilityDef] = &[paid(
        Cost::FREE,
        &[Effect::Exile {
            target: TargetSpec::Object(&Filter::PLANESWALKER),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::PLANESWALKER))),
    )];
    let mut walker_face = face("Target Sage", "{0}", TypeSet::PLANESWALKER);
    walker_face.loyalty = Some(4);
    let mut engine = bench(
        403,
        vec![
            card(
                CARD_A,
                creature_face("Scout", "{0}", 1, 1),
                KeywordSet::HASTE,
                &[],
            ),
            card(
                CARD_B,
                creature_face("Mist Shinobi", "{3}{U}", 3, 2),
                KeywordSet::EMPTY,
                HAND_NINJA_ABILITIES,
            ),
            card(
                CARD_C,
                face("Sapphire Signet", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                BLUE_ROCK_ABILITIES,
            ),
            card(CARD_D, walker_face, KeywordSet::EMPTY, &[]),
            card(
                CARD_E,
                face("Void Relic", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                EXILE_TARGET,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_C, CARD_E]).holding(&[CARD_B]),
            Seat::with(&[CARD_D]),
        ],
    );
    let scout = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let signet = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let ninja = the(&engine, ZoneLocation::Hand(me()), CARD_B);
    let walker = the(&engine, ZoneLocation::Battlefield, CARD_D);
    let relic = the(&engine, ZoneLocation::Battlefield, CARD_E);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![(scout, Defender::Planeswalker(walker))],
    );
    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::CombatEnd));
    activate(&mut engine, me(), signet, 0);
    activate_ninjutsu(&mut engine, me(), ninja, scout);

    // Exile the attacked planeswalker in response before ninjutsu resolves.
    activate(&mut engine, me(), relic, 0);
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for relic, got {:?}",
            engine.pending()
        );
    };
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![walker],
                players: vec![],
            },
        )
        .expect("target walker");
    settle(&mut engine, me());

    let bf_ninja = the(&engine, ZoneLocation::Battlefield, CARD_B);
    assert!(is_tapped(&engine, bf_ninja));
    assert_eq!(
        engine.state().combat.defender_of(bf_ninja),
        None,
        "planeswalker left the battlefield, so Ninja enters tapped and not attacking"
    );
}

fn commander_ninjutsu_bench(seed: u64) -> Bench {
    let cmd_face = FaceDef {
        supertypes: SupertypeSet::LEGENDARY,
        ..creature_face("Yuriko Shadow", "{1}{U}{B}", 1, 3)
    };
    let cmd_def: &'static CardDef = Box::leak(Box::new(CardDef {
        index: CardIndex::new(CARD_B),
        oracle_id: "test-cmd-ninja",
        scryfall_id: "test-cmd-ninja",
        faces: Box::leak(Box::new([cmd_face])),
        color_identity: baylee_core::color::ColorSet::ALL,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::Legendary,
        partner: PartnerKind::None,
        coverage: Coverage::Implemented,
        fuse: false,
        abilities: COMMANDER_NINJA_ABILITIES,
    }));
    let cards = vec![
        card(
            CARD_A,
            creature_face("Scout", "{0}", 1, 1),
            KeywordSet::HASTE,
            &[],
        ),
        cmd_def,
        card(
            CARD_C,
            face("Sapphire Signet", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BLUE_ROCK_ABILITIES,
        ),
    ];
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let mk_seat = |cmds: Vec<u32>, bf: Vec<u32>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: cmds.into_iter().map(entry).collect(),
        starting_life: None,
        starting_hand: Some(vec![]),
        starting_battlefield: bf.into_iter().map(entry).collect(),
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
        seats: vec![
            mk_seat(vec![CARD_B], vec![CARD_A, CARD_C]),
            mk_seat(vec![], vec![]),
        ],
    };
    let mut engine = Engine::new(&preset, SyntheticLookup::new(cards)).expect("cmd ninjutsu bench");
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

#[test]
fn commander_ninjutsu_ignores_commander_tax_and_works_from_command_or_hand() {
    // CR 702.49d: Commander ninjutsu functions from both the hand and the command zone.
    // CR 903.8: Commander tax applies only to casting a commander from the command zone.
    let mut engine = commander_ninjutsu_bench(404);
    let scout = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let signet = the(&engine, ZoneLocation::Battlefield, CARD_C);
    let cmd = the(&engine, ZoneLocation::Command(me()), CARD_B);
    engine.dev_state_mut(me()).unwrap().commanders[0][0].casts = 2;

    declare_attackers_step(&mut engine, me(), vec![(scout, Defender::Player(them()))]);
    declare_blockers_step(&mut engine, them(), vec![]);
    settle(&mut engine, me());

    activate(&mut engine, me(), signet, 0);
    activate_ninjutsu(&mut engine, me(), cmd, scout);
    settle(&mut engine, me());

    let bf_cmd = the(&engine, ZoneLocation::Battlefield, CARD_B);
    assert_eq!(
        engine.state().combat.defender_of(bf_cmd),
        Some(Defender::Player(them()))
    );
    assert_eq!(
        engine.state().commanders[0][0].casts,
        2,
        "commander ninjutsu is an activated ability, not a cast"
    );

    // Move commander to hand and verify HandOrCommand also activates from Hand next combat.
    engine
        .dev_state_mut(me())
        .unwrap()
        .move_object(
            bf_cmd,
            ZoneLocation::Hand(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    let hand_scout = the(&engine, ZoneLocation::Hand(me()), CARD_A);
    engine
        .apply(me(), PlayerAction::CastSpell { card: hand_scout })
        .unwrap();
    settle(&mut engine, me());
    let scout_again = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let hand_cmd = the(&engine, ZoneLocation::Hand(me()), CARD_B);
    declare_attackers_step(
        &mut engine,
        me(),
        vec![(scout_again, Defender::Player(them()))],
    );
    declare_blockers_step(&mut engine, them(), vec![]);
    settle(&mut engine, me());
    activate(&mut engine, me(), signet, 0);
    activate_ninjutsu(&mut engine, me(), hand_cmd, scout_again);
    settle(&mut engine, me());
    let bf_cmd_from_hand = the(&engine, ZoneLocation::Battlefield, CARD_B);
    assert_eq!(
        engine.state().combat.defender_of(bf_cmd_from_hand),
        Some(Defender::Player(them()))
    );
}

// =========================================================================
// 2. Myriad (CR 702.116a–b)
// =========================================================================

static MYRIAD_ETB_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::ETB,
    &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }]
)];

static GRANT_MYRIAD: &[AbilityDef] = &[static_ability!(
    Filter::YOUR_CREATURE,
    Modifier::AddKeyword(KeywordSet::MYRIAD)
)];

#[test]
fn myriad_does_not_trigger_in_two_player_duel() {
    // CR 702.116a: With no opponent other than the defending player, myriad does nothing.
    let mut engine = bench(
        405,
        vec![card(
            CARD_A,
            creature_face("Lone Herald", "{0}", 2, 2),
            KeywordSet::HASTE.union(KeywordSet::MYRIAD),
            MYRIAD_ETB_ABILITIES,
        )],
        [Seat::with(&[CARD_A]), Seat::default()],
    );
    let herald = the(&engine, ZoneLocation::Battlefield, CARD_A);
    declare_attackers_step(&mut engine, me(), vec![(herald, Defender::Player(them()))]);
    assert!(
        engine.state().zones.stack_is_empty(),
        "myriad must not queue a trigger when no other opponent exists"
    );
}

fn myriad_four_player_bench(seed: u64) -> Bench {
    let mut walker_face = face("P2 Sage", "{0}", TypeSet::PLANESWALKER);
    walker_face.loyalty = Some(5);
    bench(
        seed,
        vec![
            card(
                CARD_A,
                creature_face("Legion Vanguard", "{0}", 3, 2),
                KeywordSet::HASTE.union(KeywordSet::MYRIAD),
                MYRIAD_ETB_ABILITIES,
            ),
            card(CARD_B, walker_face, KeywordSet::EMPTY, &[]),
        ],
        [
            Seat::with(&[CARD_A]),
            Seat::default(),
            Seat::with(&[CARD_B]),
            Seat::default(),
        ],
    )
}

#[test]
fn myriad_creates_simultaneous_copies_for_other_opponents_and_exiles_at_end_of_combat() {
    // CR 702.116a: Create tapped copies attacking each other opponent or their planeswalker, exiled at end of combat.
    let mut engine = myriad_four_player_bench(406);
    let vanguard = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let walker_p2 = the(&engine, ZoneLocation::Battlefield, CARD_B);

    declare_attackers_step(
        &mut engine,
        me(),
        vec![(vanguard, Defender::Player(them()))],
    );
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
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected planeswalker choice for P2, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(options, vec![walker_p2]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![walker_p2],
            },
        )
        .unwrap();

    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());

    assert_eq!(engine.state().combat.attackers().len(), 3);
    assert_eq!(
        life(&engine, me()),
        22,
        "both token copies fired ETB +1 life"
    );
    assert!(
        engine
            .state()
            .combat
            .attackers()
            .iter()
            .any(|a| a.defending == Defender::Planeswalker(walker_p2))
    );
    assert!(
        engine
            .state()
            .combat
            .attackers()
            .iter()
            .any(|a| a.defending == Defender::Player(p3()))
    );

    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && holds_priority_in(e, me(), Step::Main)
    });
    assert_eq!(life(&engine, them()), 17);
    assert_eq!(life(&engine, p3()), 17);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|&&id| !engine.state().object(id).unwrap().is_card())
            .count(),
        0,
        "myriad tokens must be exiled at end of combat"
    );
}

#[test]
fn myriad_multiple_instances_trigger_separately() {
    // CR 702.116b: Multiple instances of myriad on a creature each trigger separately.
    let mut engine = bench(
        407,
        vec![
            card(
                CARD_A,
                creature_face("Twin Striker", "{0}", 2, 2),
                KeywordSet::HASTE.union(KeywordSet::MYRIAD),
                &[],
            ),
            card(
                CARD_B,
                face("Myriad Banner", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GRANT_MYRIAD,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B]),
            Seat::default(),
            Seat::default(),
        ],
    );
    let striker = the(&engine, ZoneLocation::Battlefield, CARD_A);
    declare_attackers_step(&mut engine, me(), vec![(striker, Defender::Player(them()))]);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        2,
        "two instances of myriad put two triggers on the stack"
    );
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
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .combat
            .attackers()
            .iter()
            .filter(|a| a.defending == Defender::Player(p2()))
            .count(),
        1,
        "accepted first myriad copy and declined second"
    );
}

// =========================================================================
// 3. Extort (CR 702.101a–b)
// =========================================================================

static SWAMP_ROCK_ABILITIES: &[AbilityDef] = &[mana_ability!(
    Cost::TAP,
    &[Effect::mana(ManaColor::Black, 1)]
)];

static GRANT_EXTORT: &[AbilityDef] = &[static_ability!(
    Filter::YOUR_CREATURE,
    Modifier::AddKeyword(KeywordSet::EXTORT)
)];

static CANT_LOSE_LIFE_SELF: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::CantLoseLife {
        who: PlayerRel::You,
    }
)];

#[test]
fn extort_triggers_per_instance_and_gains_only_actual_life_lost() {
    // CR 702.101a–b: Each extort instance triggers on your spell casts and gains life equal to actual life lost.
    let mut engine = bench(
        408,
        vec![
            card(
                CARD_A,
                creature_face("Tithe Deacon", "{0}", 1, 4),
                KeywordSet::EXTORT,
                &[],
            ),
            card(
                CARD_B,
                face("Pontiff Seal", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GRANT_EXTORT,
            ),
            card(
                CARD_C,
                face("Jet Medallion", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                SWAMP_ROCK_ABILITIES,
            ),
            card(
                CARD_D,
                face("Cantrip Bauble", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CARD_E,
                face("Platinum Halo", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                CANT_LOSE_LIFE_SELF,
            ),
        ],
        [
            Seat::with(&[CARD_A, CARD_B, CARD_C]).holding(&[CARD_D]),
            Seat::with(&[CARD_E]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let medallion = the(&engine, ZoneLocation::Battlefield, CARD_C);
    activate(&mut engine, me(), medallion, 0);
    let bauble = the(&engine, ZoneLocation::Hand(me()), CARD_D);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bauble })
        .expect("cast bauble");

    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 3);

    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();

    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());

    assert_eq!(life(&engine, them()), 20, "P1 cannot lose life");
    assert_eq!(life(&engine, p2()), 19, "P2 lost 1 life");
    assert_eq!(
        life(&engine, me()),
        21,
        "extort controller gained exact total life lost (1, not 2)"
    );
}

// =========================================================================
// 4. Exert, Trigger::Exerted & ReflexiveEvent::ExertedThis (CR 701.43a–d)
// =========================================================================

static EXERT_ATTACKER_ABILITIES: &[AbilityDef] = &[
    triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::ExertSource],
        }]
    ),
    triggered!(
        Trigger::EXERTED_THIS,
        &[Effect::CreateContinuousEffect {
            layer: Layer::PtModify,
            filter: &Filter::This,
            modifier: Modifier::ModifyPT(2, 0),
            duration: Duration::UntilEndOfTurn,
        }]
    ),
];

static EXERT_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Exerted(&Filter::YOUR_CREATURE),
    &[Effect::GainLife {
        amount: Amount::Fixed(2),
    }]
)];

static EXERT_DORK_ABILITIES: &[AbilityDef] = &[activated!(
    Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::ExertSelf],
    },
    &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }]
)];

#[test]
fn exert_effect_and_cost_fire_exerted_triggers_and_skip_one_untap_step() {
    // CR 701.43a–d: Exerting skips the controller's next untap step and triggers Trigger::Exerted.
    let mut engine = bench(
        409,
        vec![
            card(
                CARD_A,
                creature_face("Suncrest Runner", "{0}", 2, 2),
                KeywordSet::HASTE,
                EXERT_ATTACKER_ABILITIES,
            ),
            card(
                CARD_B,
                creature_face("Crop Steward", "{0}", 1, 3),
                KeywordSet::EMPTY,
                EXERT_WATCHER_ABILITIES,
            ),
            card(
                CARD_C,
                creature_face("Oasis Initiate", "{0}", 1, 2),
                KeywordSet::HASTE,
                EXERT_DORK_ABILITIES,
            ),
        ],
        [Seat::with(&[CARD_A, CARD_B, CARD_C]), Seat::default()],
    );
    let runner = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let dork = the(&engine, ZoneLocation::Battlefield, CARD_C);

    to_main(&mut engine, me());
    activate(&mut engine, me(), dork, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23, "dork +1 life and watcher +2 life");

    declare_attackers_step(&mut engine, me(), vec![(runner, Defender::Player(them()))]);
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

    assert_eq!(
        life(&engine, me()),
        25,
        "watcher fired again on attack exert"
    );
    assert_eq!(
        engine
            .state()
            .object(runner)
            .unwrap()
            .characteristics()
            .power,
        Some(4),
        "Trigger::EXERTED_THIS granted +2/+0"
    );

    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert!(
        is_tapped(&engine, runner) && is_tapped(&engine, dork),
        "exerted permanents do not untap during controller's next untap step"
    );

    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert!(
        !is_tapped(&engine, runner) && !is_tapped(&engine, dork),
        "exerted permanents untap normally on subsequent turns"
    );
}

static REFLEXIVE_EXERT_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Attacks(&Filter::This),
    &[
        Effect::MayDo {
            effects: &[Effect::ExertSource],
        },
        Effect::Reflexive {
            when: ReflexiveEvent::ExertedThis,
            effects: &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(3),
            }],
            target: Some(TargetSpec::AnyTarget),
        },
    ]
)];

#[test]
fn exert_reflexive_when_you_do_fires_only_when_exerted_on_battlefield() {
    // CR 701.43c, CR 603.12: Reflexive "When you do" fires when exerted, not when declined or dead.
    static SAC_CREATURE: &[AbilityDef] = &[paid(
        Cost::FREE,
        &[Effect::SacrificeFilter {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        }],
        None,
    )];
    let mut engine = bench(
        410,
        vec![
            card(
                CARD_A,
                creature_face("Flame Drake", "{0}", 2, 2),
                KeywordSet::HASTE.union(KeywordSet::VIGILANCE),
                REFLEXIVE_EXERT_ABILITIES,
            ),
            card(
                CARD_B,
                face("Altar", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                SAC_CREATURE,
            ),
        ],
        [Seat::with(&[CARD_A, CARD_B]), Seat::default()],
    );
    let drake = the(&engine, ZoneLocation::Battlefield, CARD_A);
    let altar = the(&engine, ZoneLocation::Battlefield, CARD_B);

    // Combat 1: decline exert -> reflexive trigger does NOT fire (only 2 combat damage).
    declare_attackers_step(&mut engine, me(), vec![(drake, Defender::Player(them()))]);
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::CombatEnd));
    assert_eq!(life(&engine, them()), 18);

    // Combat 2: accept exert on vigilant drake -> reflexive deals 3 damage + 2 combat = 13.
    to_main(&mut engine, them());
    declare_attackers_step(&mut engine, me(), vec![(drake, Defender::Player(them()))]);
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
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![them()],
                },
            )
            .unwrap();
    }
    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::CombatEnd));
    assert_eq!(life(&engine, them()), 13);

    // Combat 3: sacrifice drake in response before attack trigger resolves -> cannot exert!
    to_main(&mut engine, them());
    declare_attackers_step(&mut engine, me(), vec![(drake, Defender::Player(them()))]);
    activate(&mut engine, me(), altar, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![drake],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        13,
        "departed source cannot be exerted, so reflexive trigger never fires"
    );
}
