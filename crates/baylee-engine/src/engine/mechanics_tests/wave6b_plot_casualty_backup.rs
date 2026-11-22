//! Wave 6b engine rule tests:
//! - `Plot` (`CR 702.170a–f`, `CR 116.2k`, `CR 107.3b`): sorcery-speed special
//!   action from hand (`PLOT_CARD`) exiling face up (`Rider::Plotted`,
//!   `Rider::PlottedOnTurn(u32)`), firing `Trigger::Plotted`, forbidding same-turn
//!   or instant-speed casts, and offering `CastModeKind::Plot` on later turns
//!   without paying its mana cost (`X = 0`); plus `Effect::Plot` and
//!   `Effect::PlotTop` exiling cards without printed `plot` (`CR 702.170c–d`).
//! - `Casualty N` (`CR 702.153a–b`, `CR 707.10c`, `CR 608.2h`): optional
//!   additional sacrifice cost (`YesNoPrompt::Casualty { min_power: N }`)
//!   checking projected power `>= N`, preventing double-sacrifice across
//!   other sacrifice costs, and queuing a cast trigger that copies the spell
//!   with optional new targets (even if the original spell is countered).
//! - `Backup N` (`CR 702.165a–d`, `CR 611.2a`): ETB `+1/+1` counter placement
//!   that grants printed keywords, activated abilities, and triggered abilities
//!   until end of turn when targeting *another* creature (persisting even if the
//!   Backup source dies mid-turn), while placing only counters when targeting
//!   itself.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{ChoicePrompt, PLOT_CARD, YesNoPrompt, granted_slot};
use crate::object::Rider;
use baylee_cards_dsl::prelude::{activated, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, CounterKind, Duration, Filter, Modifier, PlayerRel, TargetSpec, Trigger,
};
use baylee_core::ids::Defender;
use baylee_core::mana::{ManaColor, ManaPool};

const PLOT_INSTANT: u32 = 8600;
const PLOT_X_SORCERY: u32 = 8601;
const PLOT_WATCHER: u32 = 8602;
const PLOT_INTERRUPTER: u32 = 8603;
const PLOT_TOP_RELIC: u32 = 8604;
const PLAIN_DIVINATION: u32 = 8605;
const TAX_STATUE: u32 = 8606;
const CASUALTY_BOLT: u32 = 8607;
const CASUALTY_BARGAIN_DRAIN: u32 = 8608;
const TINY_SQUIRE: u32 = 8609;
const SAVAGE_BRUTE: u32 = 8610;
const ARTIFACT_GOLEM: u32 = 8611;
const COUNTER_INSTANT: u32 = 8612;
const PUMP_RELIC: u32 = 8613;
const BACKUP_VALKYRIE: u32 = 8614;
const VANILLA_BEAR: u32 = 8615;
const KILL_INSTANT: u32 = 8616;

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

fn resolve_one(engine: &mut Bench) {
    let top_before = *engine
        .state
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("stack has an object to resolve");
    for _ in 0..32 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
                if !engine
                    .state
                    .zones
                    .list(ZoneLocation::Stack)
                    .contains(&top_before)
                {
                    return;
                }
            }
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            other => panic!("unexpected pending while resolving one stack object: {other:?}"),
        }
    }
    panic!("stack object never resolved");
}

fn drain_stack(engine: &mut Bench) {
    while !engine.state.zones.stack_is_empty()
        || matches!(engine.pending(), Pending::Arrange { .. })
    {
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            other => panic!("unexpected pending while draining stack: {other:?}"),
        }
    }
}

fn advance_to_next_main(engine: &mut Bench, target_active: PlayerId, min_turn: u32) {
    for _ in 0..256 {
        if engine.state.turn.number >= min_turn
            && engine.state.turn.active == target_active
            && matches!(engine.state.turn.phase, Phase::FirstMain)
            && engine.state.zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == target_active)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .expect("no attackers");
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .expect("no blockers");
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
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            other => panic!("unexpected pending while advancing turn: {other:?}"),
        }
    }
    panic!("failed to reach turn {min_turn} main phase for {target_active:?}");
}

fn wave6b_cards() -> Vec<&'static CardDef> {
    static PLOT_INSTANT_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static PLOT_X_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::GainLife {
            amount: Amount::Plus {
                base: &Amount::X,
                offset: 2,
            },
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static PLOT_WATCHER_ABILS: [AbilityDef; 1] = [triggered!(
        Trigger::Plotted {
            who: PlayerRel::You,
            filter: &Filter::Any,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }]
    )];
    static INTERRUPTER_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Plot {
            target: TargetSpec::Spell(&Filter::Any),
        }],
        targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static PLOT_TOP_ABILS: [AbilityDef; 1] = [free(
        &[Effect::PlotTop {
            who: PlayerRel::You,
            count: Amount::Fixed(1),
        }],
        None,
    )];
    static DIVINATION_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(6),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static TAX_ABILS: [AbilityDef; 1] = [static_ability!(Filter::Any, Modifier::SpellsCostMore(1))];
    static CASUALTY_BOLT_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::PlayerOrObject(&Filter::CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::PlayerOrObject(
            &Filter::CREATURE,
        ))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static CASUALTY_BARGAIN_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static COUNTER_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::CounterTargetSpell],
        targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static PUMP_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::PumpTarget {
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static BACKUP_GRANTED_ABILS: [AbilityDef; 2] = [
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::GainLife {
                amount: Amount::Fixed(3),
            }]
        ),
        activated!(
            Cost::TAP,
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }]
        ),
    ];
    static BACKUP_VALKYRIE_ABILS: [AbilityDef; 3] = [
        AbilityDef::Triggered {
            trigger: Trigger::ETB,
            effects: &[Effect::Backup {
                count: 2,
                keywords: KeywordSet::FLYING.union(KeywordSet::HASTE),
                grant_abilities: &BACKUP_GRANTED_ABILS,
            }],
            targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            second_targets: None,
            third_targets: None,
            zone: baylee_cards_dsl::TriggerZone::Battlefield,
            once_per_turn: false,
            condition: None,
            resolves_if_targets_illegal: false,
        },
        BACKUP_GRANTED_ABILS[0],
        BACKUP_GRANTED_ABILS[1],
    ];
    static KILL_ABILS: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    vec![
        card(
            PLOT_INSTANT,
            FaceDef {
                plot: Some(ManaCost::parse("{1}{U}")),
                abilities: &PLOT_INSTANT_ABILS,
                ..face("Slickshot Plan", "{2}{U}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &PLOT_INSTANT_ABILS,
        ),
        card(
            PLOT_X_SORCERY,
            FaceDef {
                plot: Some(ManaCost::parse("{1}{R}")),
                abilities: &PLOT_X_ABILS,
                ..face("Plotted Blaze", "{X}{R}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            &PLOT_X_ABILS,
        ),
        card(
            PLOT_WATCHER,
            FaceDef {
                abilities: &PLOT_WATCHER_ABILS,
                ..face("Plot Chronicler", "{1}{G}", TypeSet::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            &PLOT_WATCHER_ABILS,
        ),
        card(
            PLOT_INTERRUPTER,
            FaceDef {
                abilities: &INTERRUPTER_ABILS,
                ..face("Plot Snatcher", "{1}{W}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &INTERRUPTER_ABILS,
        ),
        card(
            PLOT_TOP_RELIC,
            FaceDef {
                abilities: &PLOT_TOP_ABILS,
                ..face("Plot Lens", "{0}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &PLOT_TOP_ABILS,
        ),
        card(
            PLAIN_DIVINATION,
            FaceDef {
                abilities: &DIVINATION_ABILS,
                ..face("Plain Counsel", "{3}{U}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            &DIVINATION_ABILS,
        ),
        card(
            TAX_STATUE,
            FaceDef {
                abilities: &TAX_ABILS,
                ..face("Tax Statue", "{2}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &TAX_ABILS,
        ),
        card(
            CASUALTY_BOLT,
            FaceDef {
                casualty: Some(2),
                abilities: &CASUALTY_BOLT_ABILS,
                ..face("Casualty Bolt", "{1}{R}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &CASUALTY_BOLT_ABILS,
        ),
        card(
            CASUALTY_BARGAIN_DRAIN,
            FaceDef {
                casualty: Some(2),
                bargain: true,
                abilities: &CASUALTY_BARGAIN_ABILS,
                ..face("Double Tithe", "{1}{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            &CASUALTY_BARGAIN_ABILS,
        ),
        card(
            TINY_SQUIRE,
            creature_face("Tiny Squire", "{W}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SAVAGE_BRUTE,
            creature_face("Savage Brute", "{1}{R}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ARTIFACT_GOLEM,
            FaceDef {
                types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
                ..creature_face("Scrap Golem", "{2}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            COUNTER_INSTANT,
            FaceDef {
                abilities: &COUNTER_ABILS,
                ..face("Hard Counter", "{U}{U}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &COUNTER_ABILS,
        ),
        card(
            PUMP_RELIC,
            FaceDef {
                abilities: &PUMP_ABILS,
                ..face("Pump Surge", "{0}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &PUMP_ABILS,
        ),
        card(
            BACKUP_VALKYRIE,
            FaceDef {
                abilities: &BACKUP_VALKYRIE_ABILS,
                keywords: KeywordSet::FLYING.union(KeywordSet::HASTE),
                ..creature_face("Backup Valkyrie", "{2}{W}", 2, 2)
            },
            KeywordSet::FLYING.union(KeywordSet::HASTE),
            &BACKUP_VALKYRIE_ABILS,
        ),
        card(
            VANILLA_BEAR,
            creature_face("Runeclaw Trainee", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            KILL_INSTANT,
            FaceDef {
                abilities: &KILL_ABILS,
                ..face("Instant Doom", "{B}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            &KILL_ABILS,
        ),
    ]
}

/// `Plot` special action & cast rules (`CR 702.170a–d`, `CR 116.2k`, `CR 107.3b`):
/// - Offered in `unpaid_abilities` with untapped lands and in `abilities` with
///   floating mana during own main phase with an empty stack, never while the
///   stack is non-empty or during an opponent's turn.
/// - Exiles face up with `Rider::Plotted` + `Trigger::Plotted`, forbids casting
///   on the same turn or at instant speed on an opponent's turn (even for an
///   Instant!), and casts on a later main phase via `CastModeKind::Plot`
///   without paying its mana cost (`X = 0` per `CR 107.3b`, while still paying
///   static cost increases like `Modifier::SpellsCostMore(1)` per `CR 118.9d`).
#[test]
fn wave6b_plot_special_action_timing_turn_delay_sorcery_speed_and_trigger() {
    let mut engine = bench(
        86_001,
        wave6b_cards(),
        [
            Seat::with(&[PLOT_WATCHER, forest(), forest()]).holding(&[
                PLOT_INSTANT,
                PLOT_X_SORCERY,
                TAX_STATUE,
            ]),
            Seat::with(&[]).holding(&[PLOT_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    let plan = the(&engine, ZoneLocation::Hand(me()), PLOT_INSTANT);
    let x_spell = the(&engine, ZoneLocation::Hand(me()), PLOT_X_SORCERY);
    let statue = the(&engine, ZoneLocation::Hand(me()), TAX_STATUE);
    let opp_plan = the(&engine, ZoneLocation::Hand(them()), PLOT_INSTANT);

    // Untapped lands hint `(plan, PLOT_CARD, {1}{U})` in unpaid_abilities.
    let (_, legal) = offer(&engine);
    assert!(
        legal
            .unpaid_abilities
            .contains(&(plan, PLOT_CARD, ManaCost::parse("{1}{U}"))),
        "unpaid_abilities should advertise PLOT_CARD with plot cost"
    );

    // Floating {1}{U} + {1}{R} moves both to legal.abilities!
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    let (_, legal) = offer(&engine);
    assert!(legal.abilities.contains(&(plan, PLOT_CARD)));
    assert!(legal.abilities.contains(&(x_spell, PLOT_CARD)));

    // Plot `plan` (`{1}{U}`): exiles face up, fires Trigger::Plotted (+2 life)!
    engine.state.players[0].mana_pool = ManaPool::new();
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    // Keep `{1}{R}` separate after `{1}{U}` so generic `{1}` doesn't spend `{R}`:
    engine.state.players[0].mana_pool = ManaPool::new();
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: plan,
                ability_index: PLOT_CARD,
            },
        )
        .expect("plot instant");
    assert_eq!(engine.state.object(plan).unwrap().zone, Zone::Exile);
    assert!(
        engine
            .state
            .object(plan)
            .unwrap()
            .riders
            .contains(&Rider::Plotted)
    );
    // While Trigger::Plotted is on the stack, `x_spell` CANNOT be plotted (`CR 702.170a`: stack must be empty),
    // even when `{1}{R}` is floating!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal_while_stack_busy) = offer(&engine);
    assert!(
        !legal_while_stack_busy
            .abilities
            .contains(&(x_spell, PLOT_CARD)),
        "PLOT_CARD requires an empty stack (CR 702.170a)"
    );
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.players[0].life, 22,
        "Trigger::Plotted fired for PLOT_CARD"
    );

    // Plot `x_spell` (`{1}{R}`) once stack is empty again:
    refresh_offer(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: x_spell,
                ability_index: PLOT_CARD,
            },
        )
        .expect("plot X sorcery");
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[0].life, 24);

    // Same turn (`turn == plotted_turn`): NEITHER plotted card may be cast yet (`CR 702.170d`)!
    let (_, legal_same_turn) = offer(&engine);
    assert!(
        !legal_same_turn.castable.contains(&plan) && !legal_same_turn.payable.contains(&plan),
        "plotted card cannot be cast on the turn it became plotted"
    );

    // Opponent's main phase (later turn!): plotted INSTANT still cannot be cast (`CR 702.170d` requires YOUR main phase),
    // and opponent cannot plot `opp_plan` during me()'s priority!
    let plot_turn = engine.state.turn.number;
    advance_to_next_main(&mut engine, them(), plot_turn + 1);
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes priority");
    let (_, me_on_opp_turn) = offer(&engine);
    assert!(
        !me_on_opp_turn.castable.contains(&plan) && !me_on_opp_turn.payable.contains(&plan),
        "plotted Instant still cannot be cast during opponent's turn (CR 702.170d)"
    );
    assert!(
        !me_on_opp_turn.abilities.contains(&(opp_plan, PLOT_CARD)),
        "cannot plot opponent's card"
    );

    // Advance to me()'s next main phase (`turn > plot_turn`): BOTH plotted cards are castable for free!
    advance_to_next_main(&mut engine, me(), plot_turn + 2);
    engine.state.players[0].mana_pool = ManaPool::new();
    refresh_offer(&mut engine);
    let (_, legal_later) = offer(&engine);
    assert!(
        legal_later.castable.contains(&plan),
        "plotted card is castable for {{0}} on a later main phase"
    );
    assert!(
        legal_later.castable.contains(&x_spell),
        "plotted {{X}} sorcery is castable for {{0}} with X=0 on a later main phase"
    );

    // Cast `x_spell` via Plot with empty mana pool -> X is forced to 0 (`CR 107.3b`), gains 0 + 2 = 2 life!
    engine
        .apply(me(), PlayerAction::CastSpell { card: x_spell })
        .expect("cast plotted X sorcery without paying mana cost");
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[0].life, 26);
    assert_eq!(engine.state.object(x_spell).unwrap().zone, Zone::Graveyard);

    // Put Tax Statue (`SpellsCostMore(1)`) onto battlefield: `plan` now costs `{1}` via `CastModeKind::Plot` (`CR 118.9d`)!
    engine
        .state
        .move_object(
            statue,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("put tax statue onto battlefield");
    engine.sync_static_effects();
    refresh_offer(&mut engine);
    let (_, taxed_offer) = offer(&engine);
    assert!(
        !taxed_offer.castable.contains(&plan),
        "SpellsCostMore(1) taxes free plotted cast by {{1}} when mana pool is empty (CR 118.9d)"
    );
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: plan })
        .expect("cast plotted instant paying {1} tax");
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[0].life, 30);
}

/// `Effect::Plot` & `Effect::PlotTop` (`CR 702.170c–d`):
/// - Cards exiled and plotted by an effect become plotted even when they do NOT
///   print `FaceDef::plot`, fire `Trigger::Plotted`, cannot be cast that turn,
///   and can be cast by their owner on a later main phase without paying their
///   mana cost!
#[test]
fn wave6b_effect_plot_and_plot_top_exile_face_up_and_enable_later_turn_cast() {
    let mut engine = bench(
        86_002,
        wave6b_cards(),
        [
            Seat::with(&[PLOT_WATCHER, PLOT_TOP_RELIC]).holding(&[
                PLAIN_DIVINATION,
                PLAIN_DIVINATION,
                PLOT_INTERRUPTER,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let top_card = objects(&engine, ZoneLocation::Hand(me()), PLAIN_DIVINATION)[0];
    let stack_card = objects(&engine, ZoneLocation::Hand(me()), PLAIN_DIVINATION)[1];
    let interrupter = the(&engine, ZoneLocation::Hand(me()), PLOT_INTERRUPTER);
    let relic = the(&engine, ZoneLocation::Battlefield, PLOT_TOP_RELIC);
    put_on_top(&mut engine, me(), top_card);
    refresh_offer(&mut engine);

    // 1) `Effect::PlotTop` exiles `top_card` (which has NO `FaceDef::plot`) as plotted!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: relic,
                ability_index: 0,
            },
        )
        .expect("activate PlotTop(1)");
    drain_stack(&mut engine);
    assert_eq!(engine.state.object(top_card).unwrap().zone, Zone::Exile);
    assert!(
        engine
            .state
            .object(top_card)
            .unwrap()
            .riders
            .contains(&Rider::Plotted)
    );
    assert_eq!(engine.state.players[0].life, 22);

    // 2) `Effect::Plot { target: Spell }` exiles `stack_card` off the stack as plotted!
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: stack_card })
        .expect("cast divination");
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: interrupter })
        .expect("cast plot interrupter");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![stack_card],
                players: vec![],
            },
        )
        .expect("target divination on stack");
    drain_stack(&mut engine);
    assert_eq!(engine.state.object(stack_card).unwrap().zone, Zone::Exile);
    assert!(
        engine
            .state
            .object(stack_card)
            .unwrap()
            .riders
            .contains(&Rider::Plotted)
    );
    assert_eq!(
        engine.state.players[0].life, 24,
        "PLAIN_DIVINATION did not resolve (was plotted off stack); Trigger::Plotted fired (+2)"
    );

    // Neither can be cast this turn; both can be cast for free on me()'s next main phase!
    let plot_turn = engine.state.turn.number;
    advance_to_next_main(&mut engine, me(), plot_turn + 2);
    engine.state.players[0].mana_pool = ManaPool::new();
    refresh_offer(&mut engine);
    for id in [top_card, stack_card] {
        engine
            .apply(me(), PlayerAction::CastSpell { card: id })
            .expect("cast effect-plotted card without printed plot");
        drain_stack(&mut engine);
    }
    assert_eq!(engine.state.players[0].life, 36);
}

/// `Casualty N` (`CR 702.153a–b`, `CR 707.10c`, `CR 608.2h`):
/// - Power `< 2` (`1/1`) skips `YesNoPrompt::Casualty { min_power: 2 }`;
///   pumping that `1/1` to `2/2` enables `YesNoPrompt::Casualty { min_power: 2 }`.
/// - Accepting sacrifices a `power >= 2` creature, queues the cast trigger that
///   copies the spell and offers `may_choose_new_targets = true` (`CR 707.10c`),
///   and resolves the copy even if the original spell is countered (`CR 608.2h`).
/// - On a spell with both `casualty: Some(2)` and `bargain: true`, a single
///   `2/2` artifact creature cannot satisfy both sacrifices!
#[test]
fn wave6b_casualty_sacrifices_power_at_least_n_copies_spell_with_retargeting_and_prevents_double_sacrifice()
 {
    let mut engine = bench(
        86_003,
        wave6b_cards(),
        [
            Seat::with(&[TINY_SQUIRE]).holding(&[
                PUMP_RELIC,
                CASUALTY_BOLT,
                CASUALTY_BOLT,
                CASUALTY_BARGAIN_DRAIN,
                ARTIFACT_GOLEM,
            ]),
            Seat::with(&[]).holding(&[COUNTER_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    let squire = the(&engine, ZoneLocation::Battlefield, TINY_SQUIRE);
    let pump = the(&engine, ZoneLocation::Hand(me()), PUMP_RELIC);
    let bolt_0 = objects(&engine, ZoneLocation::Hand(me()), CASUALTY_BOLT)[0];
    let bolt_1 = objects(&engine, ZoneLocation::Hand(me()), CASUALTY_BOLT)[1];

    // 1) With only a 1/1 (`TINY_SQUIRE`), Casualty 2 is NOT offered!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt_0 })
        .expect("cast bolt 0");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "1/1 creature must not prompt Casualty 2"
    );
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[1].life, 17);

    // 2) Pump `squire` to 2/2 -> Casualty 2 IS offered! Accept, choose `me()` as new target for copy,
    // AND counter original `bolt_1` in response!
    refresh_offer(&mut engine);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pump })
        .expect("cast pump");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![squire],
                players: vec![],
            },
        )
        .expect("pump squire to 2/2");
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.object(squire).unwrap().characteristics().power,
        Some(2)
    );

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt_1 })
        .expect("cast bolt 1");
    // CR 601.2b precedes CR 601.2c: optional Casualty is announced before targets!
    match engine.pending() {
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::Casualty { min_power: 2 },
            ..
        } if *player == me() => {}
        other => panic!("expected YesNoPrompt::Casualty(2), got {other:?}"),
    }
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept casualty 2");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent with original bolt");
    // Only one legal 2/2 creature (`squire`): sacrificed at CR 601.2h!
    if matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::CostSacrifice,
            ..
        }
    ) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![squire],
                },
            )
            .expect("choose squire to sacrifice");
    }
    assert_eq!(
        engine.state.object(squire).unwrap().zone,
        Zone::Graveyard,
        "squire sacrificed for Casualty 2"
    );
    // Casualty cast trigger is now on top of the stack above `bolt_1`!
    // Pass me() priority so them() counters `bolt_1` before Casualty trigger resolves (`CR 608.2h`)!
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass priority with casualty trigger on stack");
    let counter = the(&engine, ZoneLocation::Hand(them()), COUNTER_INSTANT);
    add_pool(&mut engine, them(), ManaColor::Blue, 2);
    engine
        .apply(them(), PlayerAction::CastSpell { card: counter })
        .expect("cast counterspell");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bolt_1],
                players: vec![],
            },
        )
        .expect("target original casualty bolt");
    resolve_one(&mut engine); // resolves counterspell -> `bolt_1` is countered to graveyard!
    assert_eq!(engine.state.object(bolt_1).unwrap().zone, Zone::Graveyard);

    // Resolve Casualty cast trigger: copies `bolt_1` via LKI and offers `may_choose_new_targets`!
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            Pending::YesNo { player, .. } => {
                engine
                    .apply(player, PlayerAction::YesNo(true))
                    .expect("choose new targets for casualty copy");
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        me(),
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![me()],
                        },
                    )
                    .expect("retarget copy at me()");
                break;
            }
            other => panic!("unexpected pending resolving casualty copy trigger: {other:?}"),
        }
    }
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.players[0].life, 17,
        "retargeted Casualty copy dealt 3 damage to me()"
    );
    assert_eq!(
        engine.state.players[1].life, 17,
        "countered original dealt 0 damage to them()"
    );

    // 3) Put ONE 2/2 artifact creature (`ARTIFACT_GOLEM`) on battlefield and cast `CASUALTY_BARGAIN_DRAIN`:
    // accepting Bargain leaves 0 creatures for Casualty 2 (cannot sacrifice the same permanent twice)!
    let golem = the(&engine, ZoneLocation::Hand(me()), ARTIFACT_GOLEM);
    engine
        .state
        .move_object(
            golem,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("put golem onto battlefield");
    let drain = the(&engine, ZoneLocation::Hand(me()), CASUALTY_BARGAIN_DRAIN);
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: drain })
        .expect("cast casualty+bargain sorcery");
    let mut accepted_first = false;
    let mut prompted_second = false;
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        match engine.pending().clone() {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Bargain | YesNoPrompt::Casualty { .. },
                ..
            } => {
                if accepted_first {
                    prompted_second = true;
                    engine
                        .apply(player, PlayerAction::YesNo(false))
                        .expect("decline second");
                } else {
                    accepted_first = true;
                    engine
                        .apply(player, PlayerAction::YesNo(true))
                        .expect("accept first sacrifice cost");
                }
            }
            Pending::ChooseCards {
                player,
                options,
                prompt: ChoicePrompt::CostSacrifice,
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .expect("sacrifice golem");
            }
            other => panic!("unexpected pending in single-golem casualty+bargain: {other:?}"),
        }
    }
    assert!(accepted_first, "first optional sacrifice cost was offered");
    assert!(
        !prompted_second,
        "second optional sacrifice cost must NOT be offered when only one permanent could pay both"
    );
}

/// `Backup N` targeting itself (`CR 702.165a`):
/// - Places `N` `+1/+1` counters on itself, and does NOT grant duplicate copies
///   of its printed abilities to itself (attacking fires its attack trigger once,
///   not twice!).
#[test]
fn wave6b_backup_self_target_places_counters_only_without_duplicate_ability_grants() {
    let mut engine = bench(
        86_004,
        wave6b_cards(),
        [Seat::with(&[]).holding(&[BACKUP_VALKYRIE]), Seat::default()],
    );
    to_main(&mut engine, me());

    let valk = the(&engine, ZoneLocation::Hand(me()), BACKUP_VALKYRIE);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: valk })
        .expect("cast backup valkyrie");
    resolve_one(&mut engine); // Valkyrie enters -> Backup ETB trigger asks for target!
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![valk],
                players: vec![],
            },
        )
        .expect("target valkyrie itself with Backup 2");
    drain_stack(&mut engine);

    assert_eq!(
        engine
            .state
            .object(valk)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(
        engine.state.object(valk).unwrap().characteristics().power,
        Some(4)
    );

    // Move to declare attackers and attack with `valk` (printed Haste): GainLife(3) must fire ONCE!
    for _ in 0..16 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass to combat");
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::DeclareAttackers {
                            attackers: vec![(valk, Defender::Player(them()))],
                        },
                    )
                    .expect("attack with valkyrie");
                break;
            }
            other => panic!("unexpected pending before combat: {other:?}"),
        }
    }
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.players[0].life, 23,
        "self-targeted Backup must NOT duplicate its printed attack trigger"
    );
}

/// `Backup N` targeting another creature (`CR 702.165a–d`, `CR 611.2a`):
/// - Places `2` `+1/+1` counters on `VANILLA_BEAR` (`4/4`) AND grants
///   `FLYING | HASTE`, the printed attack trigger (`+3` life), and the printed
///   activated ability (`{T}: Gain 1 life`) until end of turn.
/// - Even after `BACKUP_VALKYRIE` is destroyed, `VANILLA_BEAR` retains those
///   granted abilities for the rest of the turn, and loses them at cleanup!
#[test]
fn wave6b_backup_other_creature_grants_keywords_activated_and_triggered_until_eot_even_after_source_dies()
 {
    let mut engine = bench(
        86_005,
        wave6b_cards(),
        [
            Seat::with(&[]).holding(&[VANILLA_BEAR, BACKUP_VALKYRIE, KILL_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    // Cast `VANILLA_BEAR` THIS turn so it is summoning-sick unless Backup grants HASTE!
    let bear = the(&engine, ZoneLocation::Hand(me()), VANILLA_BEAR);
    let valk = the(&engine, ZoneLocation::Hand(me()), BACKUP_VALKYRIE);
    let kill = the(&engine, ZoneLocation::Hand(me()), KILL_INSTANT);

    add_pool(&mut engine, me(), ManaColor::Green, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bear })
        .expect("cast bear");
    drain_stack(&mut engine);

    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: valk })
        .expect("cast backup valkyrie");
    resolve_one(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .expect("target bear with Backup 2");
    drain_stack(&mut engine);

    // Destroy `valk` right now so we prove granted abilities persist after source dies (`CR 611.2a`)!
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: kill })
        .expect("cast kill on backup source");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![valk],
                players: vec![],
            },
        )
        .expect("target valkyrie");
    drain_stack(&mut engine);
    assert_eq!(engine.state.object(valk).unwrap().zone, Zone::Graveyard);

    let bear_char = engine.state.object(bear).unwrap().characteristics();
    assert_eq!(bear_char.power, Some(4));
    assert!(bear_char.keywords.contains(KeywordSet::FLYING));
    assert!(bear_char.keywords.contains(KeywordSet::HASTE));

    // Activate granted `{T}: Gain 1 life` on summoning-sick `bear` (only legal because HASTE was granted!):
    let (_, legal) = offer(&engine);
    let granted_idx = legal
        .abilities
        .iter()
        .find_map(|&(src, idx)| (src == bear && granted_slot(idx).is_some()).then_some(idx))
        .expect("bear has Backup-granted activated ability");
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: granted_idx,
            },
        )
        .expect("activate granted ability on bear");
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[0].life, 21);

    // Untap `bear` and attack with it in combat -> granted attack trigger (`+3` life) fires!
    engine.state.set_tapped(bear, false);
    for _ in 0..16 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass to combat");
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::DeclareAttackers {
                            attackers: vec![(bear, Defender::Player(them()))],
                        },
                    )
                    .expect("attack with bear");
                break;
            }
            other => panic!("unexpected pending before combat: {other:?}"),
        }
    }
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[0].life, 24);

    // Advance to next turn: counters stay (`4/4`), granted keywords and abilities expire at EOT!
    let turn_now = engine.state.turn.number;
    advance_to_next_main(&mut engine, them(), turn_now + 1);
    let bear_after = engine.state.object(bear).unwrap().characteristics();
    assert_eq!(bear_after.power, Some(4), "+1/+1 counters persist");
    assert!(
        !bear_after.keywords.contains(KeywordSet::FLYING)
            && !bear_after.keywords.contains(KeywordSet::HASTE),
        "Backup keyword grants expire at cleanup"
    );
}
