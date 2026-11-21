//! Wave 9b engine rule tests:
//! - `Commit a Crime` (`CR 700.13`): `Trigger::CommittedCrime` (`once_per_turn`
//!   `false` and `true`), `Condition::CommittedCrimeThisTurn`, spell/activated/
//!   triggered targeting opponent, opponent permanent, opponent spell on stack,
//!   or card in opponent's graveyard, plus non-crime negative controls.
//! - `Expend N` (`CR 700.14`): cumulative spell-mana tracking per player per
//!   turn, `Trigger::Expended` crossing threshold once per turn with
//!   `EventAmount`, `Condition::ExpendedThisTurn`,
//!   `Amount::ManaExpendedOnSpellsThisTurn`, and non-spell ability mana exclusion.
//! - `Clash` (`CR 701.30a–d`): `Effect::Clash { on_win, on_loss }`,
//!   `YesNoPrompt::ClashBottom` in APNAP order with simultaneous bottoming,
//!   strictly-highest-MV win vs tie vs empty library, `Trigger::Clashed`, and
//!   multiplayer opponent selection.
//! - Additional Combat Phases (`CR 500.8`): `Effect::AdditionalCombatPhase`
//!   with LIFO ordering and preservation of remaining normal turn phases.
//! - `Transmute` (`CR 702.53a–b`) & `Scavenge` (`CR 702.97a–b`):
//!   `KeywordSet::TRANSMUTE`, `AbilityDef::transmute`,
//!   `Filter::SameManaValueAsSource` (pre-discard LKI, sorcery timing, non-cycling),
//!   `KeywordSet::SCAVENGE`, `AbilityDef::scavenge`, and graveyard power LKI
//!   including off-battlefield `*/*` CDAs.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::YesNoPrompt;
use baylee_cards_dsl::prelude::{activated, spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, Cost, CounterKind, Effect, Filter, KeywordSet, Modifier, PlayerRel, PtCount,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::mana::ManaColor;

const CRIME_WATCHER_ALL: u32 = 9200;
const CRIME_WATCHER_ONCE: u32 = 9201;
const POKE_ANY_INSTANT: u32 = 9202;
const PUMP_CREATURE_INSTANT: u32 = 9203;
const GRAVE_PINGER: u32 = 9204;
const COUNTER_SPELL_INSTANT: u32 = 9205;
const ETB_PING_CREATURE: u32 = 9206;
const VANILLA_BEAR: u32 = 9207;
const EXPEND_WATCHER: u32 = 9208;
const TWO_MANA_SORCERY: u32 = 9209;
const THREE_MANA_SORCERY: u32 = 9210;
const MANA_SINK_ARTIFACT: u32 = 9211;
const CLASH_WATCHER: u32 = 9212;
const CLASH_SPELL: u32 = 9213;
const MV_TWO_CARD: u32 = 9214;
const MV_THREE_CARD: u32 = 9215;
const MV_FIVE_CARD: u32 = 9216;
const ADD_COMBAT_MAIN_SPELL: u32 = 9217;
const ADD_COMBAT_ONLY_INSTANT: u32 = 9218;
const TRANSMUTE_THREE_DROP: u32 = 9219;
const CYCLING_WATCHER: u32 = 9220;
const SCAVENGE_BRUTE: u32 = 9221;
const SCAVENGE_LHURGOYF: u32 = 9222;

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

fn advance_to_main(engine: &mut Bench, seat: PlayerId) {
    for _ in 0..512 {
        if holds_priority_in(engine, seat, Step::Main)
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
                    "unexpected pending while advancing to main: {pending:?}"
                );
            }
        }
    }
    panic!("failed to reach first main phase for {seat:?}");
}

#[test]
fn commit_a_crime_spell_ability_trigger_graveyard_once_per_turn_and_controls() {
    static WATCHER_ALL_ABILITIES: [AbilityDef; 2] = [
        triggered!(
            Trigger::COMMITTED_CRIME,
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
        AbilityDef::ActivatedConditional {
            cost: Cost::FREE,
            effects: &[Effect::gain_life(1)],
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: Condition::CommittedCrimeThisTurn(PlayerRel::You),
            limit: ActivationLimit::PerTurn(1),
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
    ];
    static WATCHER_ONCE_ABILITIES: [AbilityDef; 1] = [triggered!(
        Trigger::COMMITTED_CRIME_ONCE_PER_TURN,
        &[Effect::gain_life(2)]
    )];
    static POKE_ABILITIES: [AbilityDef; 1] = [spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static PUMP_ABILITIES: [AbilityDef; 1] = [spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static GRAVE_PINGER_ABILITIES: [AbilityDef; 1] = [activated!(
        Cost::FREE,
        &[Effect::Exile {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::EachPlayer
        )))
    )];
    static COUNTER_ABILITIES: [AbilityDef; 1] = [spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    static ETB_PING_ABILITIES: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::TapTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let cards = vec![
        card(
            CRIME_WATCHER_ALL,
            creature_face("Outlaw Chronicler", "{1}{B}", 2, 2),
            KeywordSet::EMPTY,
            &WATCHER_ALL_ABILITIES,
        ),
        card(
            CRIME_WATCHER_ONCE,
            creature_face("Bounty Broker", "{1}{B}", 1, 3),
            KeywordSet::EMPTY,
            &WATCHER_ONCE_ABILITIES,
        ),
        card(
            POKE_ANY_INSTANT,
            face("Quick Dart", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &POKE_ABILITIES,
        ),
        card(
            PUMP_CREATURE_INSTANT,
            face("Friendly Pep", "{G}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &PUMP_ABILITIES,
        ),
        card(
            GRAVE_PINGER,
            face("Relic Vacuum", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &GRAVE_PINGER_ABILITIES,
        ),
        card(
            COUNTER_SPELL_INSTANT,
            face("Sheriff's Veto", "{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &COUNTER_ABILITIES,
        ),
        card(
            ETB_PING_CREATURE,
            creature_face("Lasso Deputy", "{1}{W}", 2, 2),
            KeywordSet::EMPTY,
            &ETB_PING_ABILITIES,
        ),
        card(
            VANILLA_BEAR,
            creature_face("Town Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        920,
        cards,
        [
            Seat::with(&[
                CRIME_WATCHER_ALL,
                CRIME_WATCHER_ONCE,
                GRAVE_PINGER,
                VANILLA_BEAR,
            ])
            .holding(&[
                PUMP_CREATURE_INSTANT,
                POKE_ANY_INSTANT,
                COUNTER_SPELL_INSTANT,
                ETB_PING_CREATURE,
                VANILLA_BEAR,
            ]),
            Seat::with(&[VANILLA_BEAR]).holding(&[PUMP_CREATURE_INSTANT, VANILLA_BEAR]),
        ],
    );
    advance_to_main(&mut engine, me());

    let my_gy_card = objects(&engine, ZoneLocation::Hand(me()), VANILLA_BEAR)[0];
    let opp_gy_card = objects(&engine, ZoneLocation::Hand(them()), VANILLA_BEAR)[0];
    engine
        .state
        .move_object(
            my_gy_card,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            opp_gy_card,
            ZoneLocation::Graveyard(them()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    let watcher_all = the(&engine, ZoneLocation::Battlefield, CRIME_WATCHER_ALL);
    let pinger = the(&engine, ZoneLocation::Battlefield, GRAVE_PINGER);
    let my_bear = objects(&engine, ZoneLocation::Battlefield, VANILLA_BEAR)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let opp_bear = objects(&engine, ZoneLocation::Battlefield, VANILLA_BEAR)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == them())
        .unwrap();

    // Control 1: Condition::CommittedCrimeThisTurn is initially false.
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|&(src, idx)| src == watcher_all && idx == 1),
        "conditional crime ability must not be legal before committing a crime"
    );

    // Control 2: Casting a spell targeting own creature does NOT commit a crime.
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    let pump = the(&engine, ZoneLocation::Hand(me()), PUMP_CREATURE_INSTANT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pump })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_bear],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(watcher_all)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(life(&engine, me()), 20);

    // Control 3: Activating an ability targeting own graveyard card does NOT commit a crime.
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: pinger,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![my_gy_card],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(watcher_all)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // Crime #1: Casting a spell targeting an opponent commits a crime!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let poke = the(&engine, ZoneLocation::Hand(me()), POKE_ANY_INSTANT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: poke })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(watcher_all)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "targeting opponent commits crime #1"
    );
    assert_eq!(
        life(&engine, me()),
        22,
        "once-per-turn crime watcher fires on first crime"
    );
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|&(src, idx)| src == watcher_all && idx == 1),
        "Condition::CommittedCrimeThisTurn(You) is now true"
    );

    // Crime #2: Activating an ability targeting a card in opponent's graveyard commits a crime,
    // while once_per_turn watcher does NOT fire a second time this turn!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: pinger,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_gy_card],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(watcher_all)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2,
        "targeting opponent's graveyard card commits crime #2"
    );
    assert_eq!(
        life(&engine, me()),
        22,
        "once-per-turn crime watcher must not fire a second time in the same turn"
    );

    // Crime #3: Putting a triggered ability on the stack targeting opponent's creature commits a crime!
    add_pool(&mut engine, me(), ManaColor::White, 2);
    let deputy = the(&engine, ZoneLocation::Hand(me()), ETB_PING_CREATURE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: deputy })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(watcher_all)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3,
        "putting triggered ability targeting opponent's permanent on stack commits crime #3"
    );

    // Crime #4 (on opponent's turn): Countering opponent's spell on the stack commits a crime,
    // and once_per_turn watcher fires again on the new turn!
    advance_to_main(&mut engine, them());
    add_pool(&mut engine, them(), ManaColor::Green, 1);
    let opp_pump = the(&engine, ZoneLocation::Hand(them()), PUMP_CREATURE_INSTANT);
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp_pump })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    let veto = the(&engine, ZoneLocation::Hand(me()), COUNTER_SPELL_INSTANT);
    let opp_spell_on_stack = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: veto })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_spell_on_stack],
                players: vec![],
            },
        )
        .unwrap();
    settle_stack(&mut engine, them());
    assert_eq!(
        engine
            .state()
            .object(watcher_all)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4,
        "targeting opponent's spell on the stack commits crime #4"
    );
    assert_eq!(
        life(&engine, me()),
        24,
        "once-per-turn crime watcher resets on the next turn"
    );
}

#[test]
fn expend_cumulative_spell_mana_threshold_once_per_turn_and_ability_mana_excluded() {
    static EXPEND_ABILITIES: [AbilityDef; 2] = [
        triggered!(
            Trigger::Expended {
                who: PlayerRel::You,
                threshold: 4,
            },
            &[Effect::AddCounterSource {
                kind: CounterKind::Charge,
                amount: Amount::EventAmount,
            }]
        ),
        AbilityDef::ActivatedConditional {
            cost: Cost::FREE,
            effects: &[Effect::GainLife {
                amount: Amount::ManaExpendedOnSpellsThisTurn(PlayerRel::You),
            }],
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: Condition::ExpendedThisTurn(4),
            limit: ActivationLimit::Unlimited,
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
    ];
    static SINK_ABILITIES: [AbilityDef; 1] = [activated!(
        baylee_cards_dsl::cost!("{3}"),
        &[Effect::scry(1)]
    )];
    static SORCERY_ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::gain_life(1)])];

    let cards = vec![
        card(
            EXPEND_WATCHER,
            creature_face("Raccoon Quartermaster", "{1}{G}", 2, 3),
            KeywordSet::EMPTY,
            &EXPEND_ABILITIES,
        ),
        card(
            MANA_SINK_ARTIFACT,
            face("Acorn Mill", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &SINK_ABILITIES,
        ),
        card(
            TWO_MANA_SORCERY,
            face("Forage Trail", "{1}{G}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &SORCERY_ABILITIES,
        ),
        card(
            THREE_MANA_SORCERY,
            face("Bramble Feast", "{2}{G}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &SORCERY_ABILITIES,
        ),
    ];

    let mut engine = bench(
        921,
        cards,
        [
            Seat::with(&[EXPEND_WATCHER, MANA_SINK_ARTIFACT]).holding(&[
                TWO_MANA_SORCERY,
                THREE_MANA_SORCERY,
                TWO_MANA_SORCERY,
            ]),
            Seat::default(),
        ],
    );
    advance_to_main(&mut engine, me());

    let watcher = the(&engine, ZoneLocation::Battlefield, EXPEND_WATCHER);
    let sink = the(&engine, ZoneLocation::Battlefield, MANA_SINK_ARTIFACT);

    // Control: Spending {3} mana on an activated ability does NOT count toward Expend!
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: sink,
                ability_index: 0,
            },
        )
        .unwrap();
    // Answer the scry 1 Arrange prompt inside settle_stack or manually.
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    if let Pending::Arrange { player, cards, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::Arrange {
                    piles: vec![cards, vec![]],
                },
            )
            .unwrap();
    }
    settle_stack(&mut engine, me());
    assert_eq!(engine.state().per_turn.mana_expended_by(me()), 0);
    assert_eq!(
        engine
            .state()
            .object(watcher)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );

    // Spell #1 ({1}{G} = 2 mana): cumulative 2 < 4 -> Expend 4 does NOT trigger yet.
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    let s1 = objects(&engine, ZoneLocation::Hand(me()), TWO_MANA_SORCERY)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: s1 })
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(engine.state().per_turn.mana_expended_by(me()), 2);
    assert_eq!(
        engine
            .state()
            .object(watcher)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );

    // Spell #2 ({2}{G} = 3 mana): cumulative 2 -> 5 crosses 4! Expend 4 triggers ONCE with EventAmount = 5!
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    let s2 = the(&engine, ZoneLocation::Hand(me()), THREE_MANA_SORCERY);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s2 })
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(engine.state().per_turn.mana_expended_by(me()), 5);
    assert_eq!(
        engine
            .state()
            .object(watcher)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        5,
        "Trigger::Expended fires when crossing 4 (2 -> 5) and binds EventAmount to 5"
    );

    // Spell #3 ({1}{G} = 2 mana): cumulative 5 -> 7 does NOT re-trigger Expend 4!
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    let s3 = the(&engine, ZoneLocation::Hand(me()), TWO_MANA_SORCERY);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s3 })
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(engine.state().per_turn.mana_expended_by(me()), 7);
    assert_eq!(
        engine
            .state()
            .object(watcher)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        5,
        "Expend 4 must not trigger a second time in the same turn"
    );

    // Condition::ExpendedThisTurn(4) holds, and Amount::ManaExpendedOnSpellsThisTurn reads 7!
    let life_before = life(&engine, me());
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: watcher,
                ability_index: 1,
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(life(&engine, me()), life_before + 7);

    // Next turn resets cumulative expend to 0.
    advance_to_main(&mut engine, them());
    assert_eq!(engine.state().per_turn.mana_expended_by(me()), 0);
}

#[test]
fn clash_win_loss_tie_empty_library_apnap_bottoming_and_multiplayer_choice() {
    static WATCHER_ABILITIES: [AbilityDef; 2] = [
        triggered!(
            Trigger::Clashed {
                who: PlayerRel::EachPlayer,
                won_only: false,
            },
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
        triggered!(
            Trigger::Clashed {
                who: PlayerRel::You,
                won_only: true,
            },
            &[Effect::gain_life(3)]
        ),
    ];
    static CLASH_ABILITIES: [AbilityDef; 1] = [activated!(
        Cost::FREE,
        &[Effect::Clash {
            on_win: &[Effect::draw(1)],
            on_loss: &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::You,
            }],
        }]
    )];

    let cards = vec![
        card(
            CLASH_WATCHER,
            creature_face("Flamekin Herald", "{1}{R}", 2, 2),
            KeywordSet::EMPTY,
            &WATCHER_ABILITIES,
        ),
        card(
            CLASH_SPELL,
            face("Clash Gong", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &CLASH_ABILITIES,
        ),
        card(
            MV_TWO_CARD,
            creature_face("Two-Drop", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MV_THREE_CARD,
            creature_face("Three-Drop", "{2}{G}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MV_FIVE_CARD,
            creature_face("Five-Drop", "{4}{G}", 5, 5),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        922,
        cards.clone(),
        [
            Seat::with(&[CLASH_WATCHER, CLASH_SPELL]).holding(&[MV_FIVE_CARD, MV_THREE_CARD]),
            Seat::default().holding(&[MV_TWO_CARD, MV_THREE_CARD]),
        ],
    );
    advance_to_main(&mut engine, me());

    let watcher = the(&engine, ZoneLocation::Battlefield, CLASH_WATCHER);
    let gong = the(&engine, ZoneLocation::Battlefield, CLASH_SPELL);
    let my_five = the(&engine, ZoneLocation::Hand(me()), MV_FIVE_CARD);
    let my_three = the(&engine, ZoneLocation::Hand(me()), MV_THREE_CARD);
    let opp_two = the(&engine, ZoneLocation::Hand(them()), MV_TWO_CARD);
    let opp_three = the(&engine, ZoneLocation::Hand(them()), MV_THREE_CARD);

    engine
        .state
        .move_object(
            my_five,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            opp_two,
            ZoneLocation::Library(them()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    // Clash #1: MV 5 vs MV 2 -> APNAP prompts me() first, then them().
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: gong,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::ClashBottom { card },
            ..
        } if *player == me() && *card == my_five
    ));
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::ClashBottom { card },
            ..
        } if *player == them() && *card == opp_two
    ));
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(them()))
            .last(),
        Some(&opp_two)
    );
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    settle_stack(&mut engine, me());

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .contains(&my_five)
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(them()))
            .first(),
        Some(&opp_two)
    );
    assert_eq!(life(&engine, me()), 23);
    assert_eq!(
        engine
            .state()
            .object(watcher)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );

    // Clash #2: Tie (MV 3 vs MV 3) -> nobody wins, on_loss runs (me() loses 1 life), won_only does NOT fire!
    engine
        .state
        .move_object(
            my_three,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            opp_three,
            ZoneLocation::Library(them()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: gong,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        22,
        "tie does not win clash: on_loss lost 1 life and won_only trigger did not fire"
    );
    assert_eq!(
        engine
            .state()
            .object(watcher)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );

    // Clash #3: Empty library vs MV 0 Forest -> empty library reveals nothing and loses to MV 0!
    let my_lib: Vec<ObjectId> = engine.state.zones.list(ZoneLocation::Library(me())).clone();
    for c in my_lib {
        engine
            .state
            .move_object(
                c,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine
        .state
        .move_object(
            opp_three,
            ZoneLocation::Hand(them()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: gong,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::ClashBottom { .. },
            ..
        } if *player == them()
    ));
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        21,
        "empty library loses clash even against MV 0 land"
    );

    // Clash #4: 3-player game prompts controller to choose which opponent to clash with (CR 701.30a).
    let mut multi = bench(
        923,
        cards,
        [
            Seat::with(&[CLASH_SPELL]).holding(&[MV_FIVE_CARD]),
            Seat::default(),
            Seat::default(),
        ],
    );
    let active = PlayerId::new(2);
    advance_to_main(&mut multi, active);
    multi.apply(active, PlayerAction::PassPriority).unwrap();
    let m_gong = the(&multi, ZoneLocation::Battlefield, CLASH_SPELL);
    let m_five = the(&multi, ZoneLocation::Hand(me()), MV_FIVE_CARD);
    multi
        .state
        .move_object(
            m_five,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut multi);
    multi
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: m_gong,
                ability_index: 0,
            },
        )
        .unwrap();
    multi.apply(me(), PlayerAction::PassPriority).unwrap();
    multi
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();
    multi
        .apply(PlayerId::new(2), PlayerAction::PassPriority)
        .unwrap();
    assert!(matches!(
        multi.pending(),
        Pending::ChoosePlayer { player, options }
            if *player == me() && options == &vec![PlayerId::new(1), PlayerId::new(2)]
    ));
    multi
        .apply(me(), PlayerAction::ChoosePlayer(PlayerId::new(2)))
        .unwrap();
    assert!(matches!(
        multi.pending(),
        Pending::YesNo { player, prompt: YesNoPrompt::ClashBottom { .. }, .. }
            if *player == active
    ));
    multi.apply(active, PlayerAction::YesNo(true)).unwrap();
    multi.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle_stack(&mut multi, me());
    assert!(
        multi
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .contains(&m_five)
    );
}

#[test]
fn additional_combat_phases_lifo_order_and_normal_phase_preservation() {
    static WITH_MAIN: [AbilityDef; 1] = [activated!(
        Cost::FREE,
        &[Effect::AdditionalCombatPhase {
            followed_by_main_phase: true,
        }]
    )];
    static COMBAT_ONLY: [AbilityDef; 1] = [activated!(
        Cost::FREE,
        &[Effect::AdditionalCombatPhase {
            followed_by_main_phase: false,
        }]
    )];

    let cards = vec![
        card(
            ADD_COMBAT_MAIN_SPELL,
            face("Banner of Assault", "{2}{R}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &WITH_MAIN,
        ),
        card(
            ADD_COMBAT_ONLY_INSTANT,
            face("Warhorn of Fury", "{1}{R}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &COMBAT_ONLY,
        ),
    ];

    let mut engine = bench(
        924,
        cards,
        [
            Seat::with(&[ADD_COMBAT_MAIN_SPELL, ADD_COMBAT_ONLY_INSTANT]),
            Seat::default(),
        ],
    );
    advance_to_main(&mut engine, me());
    let with_main = the(&engine, ZoneLocation::Battlefield, ADD_COMBAT_MAIN_SPELL);
    let combat_only = the(&engine, ZoneLocation::Battlefield, ADD_COMBAT_ONLY_INSTANT);

    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: with_main,
                ability_index: 0,
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());

    let mut phases_seen = Vec::new();
    let start_turn = engine.state().turn.number;
    let mut last = (engine.state().turn.phase, engine.state().turn.step);
    for _ in 0..256 {
        if engine.state().turn.number != start_turn {
            break;
        }
        let now = (engine.state().turn.phase, engine.state().turn.step);
        if now != last {
            if matches!(now.1, Step::CombatBegin | Step::Main | Step::End) {
                phases_seen.push(now);
            }
            last = now;
        }
        if phases_seen.len() == 4
            && now == (Phase::SecondMain, Step::Main)
            && holds_priority_in(&engine, me(), Step::Main)
            && engine.state().per_turn.extra_phases_created.0 == 1
        {
            engine
                .apply(
                    me(),
                    PlayerAction::ActivateAbility {
                        source: with_main,
                        ability_index: 0,
                    },
                )
                .unwrap();
            settle_stack(&mut engine, me());
            engine
                .apply(
                    me(),
                    PlayerAction::ActivateAbility {
                        source: combat_only,
                        ability_index: 0,
                    },
                )
                .unwrap();
            settle_stack(&mut engine, me());
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending));
    }

    assert_eq!(
        phases_seen,
        vec![
            (Phase::Combat, Step::CombatBegin),
            (Phase::SecondMain, Step::Main),
            (Phase::Combat, Step::CombatBegin),
            (Phase::SecondMain, Step::Main),
            (Phase::Combat, Step::CombatBegin),
            (Phase::Combat, Step::CombatBegin),
            (Phase::SecondMain, Step::Main),
            (Phase::Ending, Step::End),
        ],
        "CR 500.8 LIFO extra combat & main phase order must preserve normal phases and run most-recent first"
    );
}

#[test]
fn transmute_same_mana_value_lki_not_cycling_and_scavenge_graveyard_power_including_cda() {
    static TRANSMUTE_ABILITIES: [AbilityDef; 1] =
        [AbilityDef::transmute(ManaCost::parse("{1}{U}{U}"))];
    static CYCLING_WATCH_ABILITIES: [AbilityDef; 1] = [triggered!(
        Trigger::Cycled {
            by: PlayerRel::You,
            filter: &Filter::Any,
        },
        &[Effect::gain_life(5)]
    )];
    static SCAVENGE_BRUTE_ABILITIES: [AbilityDef; 1] =
        [AbilityDef::scavenge(ManaCost::parse("{2}{G}"))];
    static SCAVENGE_LHURGOYF_ABILITIES: [AbilityDef; 2] = [
        static_ability!(
            Filter::This,
            Modifier::CharacteristicPT {
                count: PtCount::InAllGraveyards(&Filter::CREATURE),
                toughness_plus: 1,
            }
        ),
        AbilityDef::scavenge(ManaCost::parse("{1}{B}")),
    ];

    let cards = vec![
        card(
            TRANSMUTE_THREE_DROP,
            face("Dimir Courier", "{1}{U}{B}", TypeSet::CREATURE),
            KeywordSet::TRANSMUTE,
            &TRANSMUTE_ABILITIES,
        ),
        card(
            CYCLING_WATCHER,
            creature_face("Rift Watcher", "{1}{W}", 2, 2),
            KeywordSet::EMPTY,
            &CYCLING_WATCH_ABILITIES,
        ),
        card(
            MV_TWO_CARD,
            creature_face("Silverblade", "{1}{W}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MV_THREE_CARD,
            face("Three-Mana Tutor Target", "{2}{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            MV_FIVE_CARD,
            creature_face("Goliath", "{4}{G}", 5, 5),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SCAVENGE_BRUTE,
            creature_face("Dreg Mangler", "{1}{B}{G}", 4, 2),
            KeywordSet::SCAVENGE,
            &SCAVENGE_BRUTE_ABILITIES,
        ),
        card(
            SCAVENGE_LHURGOYF,
            creature_face("Grave Boneyard", "{2}{B}{G}", 0, 1),
            KeywordSet::SCAVENGE,
            &SCAVENGE_LHURGOYF_ABILITIES,
        ),
    ];

    let mut engine = bench(
        925,
        cards,
        [
            Seat::with(&[CYCLING_WATCHER, MV_TWO_CARD]).holding(&[
                TRANSMUTE_THREE_DROP,
                MV_TWO_CARD,
                MV_THREE_CARD,
                MV_FIVE_CARD,
                SCAVENGE_BRUTE,
                SCAVENGE_LHURGOYF,
            ]),
            Seat::default(),
        ],
    );
    advance_to_main(&mut engine, me());

    let bear = the(&engine, ZoneLocation::Battlefield, MV_TWO_CARD);
    let transmuter = the(&engine, ZoneLocation::Hand(me()), TRANSMUTE_THREE_DROP);
    let lib_two = the(&engine, ZoneLocation::Hand(me()), MV_TWO_CARD);
    let lib_three = the(&engine, ZoneLocation::Hand(me()), MV_THREE_CARD);
    let lib_five = the(&engine, ZoneLocation::Hand(me()), MV_FIVE_CARD);
    let brute = the(&engine, ZoneLocation::Hand(me()), SCAVENGE_BRUTE);
    let lhurgoyf = the(&engine, ZoneLocation::Hand(me()), SCAVENGE_LHURGOYF);

    for c in [lib_two, lib_three, lib_five] {
        engine
            .state
            .move_object(
                c,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    for c in [brute, lhurgoyf] {
        engine
            .state
            .move_object(
                c,
                ZoneLocation::Graveyard(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine.state.refresh_characteristics();
    refresh_offer(&mut engine);

    // Control 1: Neither Transmute nor Scavenge may be activated on opponent's turn (sorcery speed)!
    advance_to_main(&mut engine, them());
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority on opponent's turn")
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|&(id, _)| id == transmuter || id == brute || id == lhurgoyf),
        "Transmute and Scavenge must be restricted to sorcery timing"
    );

    // Return to me()'s main phase.
    advance_to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: transmuter,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(transmuter).unwrap().zone,
        Zone::Graveyard
    );
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected library search choice, got {:?}", engine.pending());
    };
    assert_eq!(min, 0);
    assert_eq!(max, 1);
    assert_eq!(
        options,
        vec![lib_three],
        "Filter::SameManaValueAsSource must match only MV 3 cards using pre-discard LKI"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![lib_three],
            },
        )
        .unwrap();
    settle_stack(&mut engine, me());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .contains(&lib_three)
    );
    assert_eq!(
        life(&engine, me()),
        20,
        "Transmute is not cycling and must not trigger Trigger::Cycled"
    );

    // Now graveyard has 3 creature cards: transmuter, brute (4/2), and lhurgoyf (*/*+1 = 3/4)!
    engine.state.refresh_characteristics();
    assert_eq!(
        engine
            .state()
            .object(lhurgoyf)
            .unwrap()
            .characteristics()
            .power,
        Some(3),
        "*/*+1 graveyard CDA reflects 3 creature cards in graveyards before scavenge"
    );

    // Scavenge lhurgoyf ({1}{B}) targeting 2/2 bear -> exiles lhurgoyf and puts 3 +1/+1 counters on bear!
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: lhurgoyf,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(lhurgoyf).unwrap().zone, Zone::Exile);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3,
        "Scavenge on */*+1 creature card puts +1/+1 counters equal to its graveyard LKI power (3)"
    );

    // Scavenge brute ({2}{G}, printed 4/2) targeting bear -> exiles brute and adds 4 +1/+1 counters (total 7)!
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: brute,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(brute).unwrap().zone, Zone::Exile);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        7,
        "Scavenge on 4/2 creature card adds 4 +1/+1 counters"
    );
}
