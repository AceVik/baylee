//! Wave 6a engine rule tests:
//! 1. Modal Trigger `second_targets` (`CR 603.3c–d`, `CR 115.3`, `CR 700.2d`):
//!    `AbilityDef::ModalTriggered` modes carrying `SpellMode::second_targets`,
//!    mode-availability filtering when the second target cannot be satisfied,
//!    and `DecisionContext::second_instance` / `first_targets`.
//! 2. `CostPart::DealDamageToYou(u16)` (`CR 118.3`, `CR 120.3`): self-damage
//!    costs across activated abilities, spells, and `PlayerMayPayCostOr`,
//!    including playability under `CantLoseLife` and damage prevention.
//! 3. `CostPart::{DiscardX, ExileFromGraveyardX}` + `SacrificeX` / `TapX`:
//!    `Amount::X` binding across activated abilities, spells, and
//!    `PlayerMayPayCostThen`.
//! 4. `CostPart::Forage` (`CR 701.61a`): paying with 1 Food or 3 graveyard
//!    cards across activated abilities, kicker spells, and `PlayerMayPayCostThen`,
//!    plus negative refusal with 0 Foods and 2 graveyard cards.
//! 5. `Modular N` (`CR 702.43a–b`, `CR 603.10a`, `CR 614.16`): `EnterModifier::modular`,
//!    entry counter-replacement interaction, LKI `+1/+1` counter transfer to
//!    target artifact creature on death, and multi-instance triggers.
//! 6. `Soulbond` (`CR 702.95a–e`, `CR 603.4`): self-ETB and other-ETB pairing,
//!    dual-Soulbond intervening-`if` cancellation of the second trigger,
//!    `Filter::SOULBOND_PAIR` + `Condition::IsPaired` buffs on both partners,
//!    and immediate automatic unpairing on zone exit, control change, or losing
//!    `TypeSet::CREATURE`.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, NumberPrompt, YesNoPrompt};
use baylee_cards_dsl::prelude::{cost, spell, static_ability};
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, Duration, EnterModifier, Filter, Layer, Modifier, PlayerRel,
    SpellMode, StaticAbility, TargetReq, TargetSlot, TargetSpec, Trigger, TriggerZone, ZoneRef,
};
use baylee_core::generated::subtypes::artifact::FOOD;
use baylee_core::generated::subtypes::creature::BEAR;
use baylee_core::mana::ManaColor;

const BEAR_QUEEN: u32 = 8600;
const CUB_BEAR: u32 = 8601;
const ENEMY_GOBLIN: u32 = 8602;
const PAIN_ALTAR: u32 = 8603;
const PAIN_BOLT: u32 = 8604;
const PAIN_TAX_PROBE: u32 = 8605;
const PLATINUM_LOCK: u32 = 8606;
const SELF_SHIELD: u32 = 8607;
const DISCARD_X_ENGINE: u32 = 8608;
const EXILE_GY_X_SPELL: u32 = 8609;
const SAC_X_TAX_PROBE: u32 = 8610;
const FORAGE_SQUIRREL: u32 = 8611;
const FORAGE_KICK_SPELL: u32 = 8612;
const FORAGE_MAY_PROBE: u32 = 8613;
const FOOD_ARTIFACT: u32 = 8614;
const MODULAR_WORKER: u32 = 8615;
const ARTIFACT_CONSTRUCT: u32 = 8616;
const COUNTER_DOUBLER: u32 = 8617;
const MODULAR_GRANTER: u32 = 8618;
const KILL_SPELL: u32 = 8619;
const SOULBOND_PALADIN: u32 = 8620;
const SOULBOND_MAULER: u32 = 8621;
const BLINK_INSTANT: u32 = 8622;
const STEAL_INSTANT: u32 = 8623;
const DEANIMATE_INSTANT: u32 = 8624;

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

#[test]
fn restricted_modal_triggers_reserve_modes_before_any_trigger_resolves() {
    use crate::zone::ZonePosition;
    use baylee_cards_dsl::ModalRestriction;
    use baylee_cards_dsl::prelude::{modal_triggered, mode};

    static MODES: &[SpellMode] = &[
        mode!(&[Effect::GainLife {
            amount: Amount::Fixed(2)
        }]),
        mode!(&[Effect::GainLife {
            amount: Amount::Fixed(3)
        }]),
    ];
    static ABILITIES: &[AbilityDef] = &[modal_triggered!(
        Trigger::EntersBattlefield(&Filter::CREATURE),
        MODES,
        restriction = ModalRestriction::OncePerTurn
    )];
    let cards = vec![
        card(
            BEAR_QUEEN,
            creature_face("Modal source", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            ABILITIES,
        ),
        card(
            CUB_BEAR,
            creature_face("Entrant", "{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut game = bench(
        603,
        cards,
        [
            Seat::with(&[BEAR_QUEEN]).holding(&[CUB_BEAR, CUB_BEAR, CUB_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut game, me());
    let entrants = objects(&game, ZoneLocation::Hand(me()), CUB_BEAR);
    assert_eq!(entrants.len(), 3);
    for entrant in entrants {
        game.state
            .move_object(
                entrant,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    game.collect_triggers();
    let Pending::ChooseCastMode { options, .. } = game.pending() else {
        panic!("first trigger must ask for its mode before it stacks");
    };
    assert_eq!(
        options.iter().map(|o| o.kind).collect::<Vec<_>>(),
        [CastModeKind::Mode(0), CastModeKind::Mode(1)]
    );
    game.apply(me(), PlayerAction::ChooseMode(1)).unwrap();
    let Pending::ChooseCastMode { options, .. } = game.pending() else {
        panic!("second trigger must ask for its mode before resolution");
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].kind, CastModeKind::Mode(0));
    assert_eq!(game.state.zones.list(ZoneLocation::Stack).len(), 1);
    game.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    assert_eq!(game.state.zones.list(ZoneLocation::Stack).len(), 2);
    drain_stack(&mut game);
    assert_eq!(life(&game, me()), 25);
}

fn drain_stack(engine: &mut Bench) {
    for _ in 0..64 {
        if engine.state.zones.stack_is_empty()
            && !matches!(engine.pending(), Pending::Arrange { .. })
        {
            return;
        }
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
    panic!("stack did not drain within 64 steps");
}

fn find_bf(engine: &Bench, owner: PlayerId, card_idx: u32) -> ObjectId {
    engine
        .state
        .battlefield_seen()
        .find(|&id| {
            engine.state.object(id).is_some_and(|o| {
                o.controller == owner && o.card.is_some_and(|c| c.index.get() == card_idx)
            })
        })
        .unwrap_or_else(|| panic!("missing battlefield card {card_idx} for {owner:?}"))
}

fn find_hand(engine: &Bench, owner: PlayerId, card_idx: u32) -> ObjectId {
    engine
        .state
        .zones
        .list(ZoneLocation::Hand(owner))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == card_idx))
        })
        .unwrap_or_else(|| panic!("missing hand card {card_idx} for {owner:?}"))
}

#[test]
fn modal_trigger_second_targets_fights_and_suppresses_untargetable_mode() {
    static YOUR_BEAR: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByYou,
        Filter::HasSubtype(BEAR),
    ]);
    static OPP_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);
    static MODES: [SpellMode; 2] = [
        SpellMode {
            effects: &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            targets: Some(TargetReq::one(TargetSpec::Object(&YOUR_BEAR))),
            second_targets: None,
            additional_cost: None,
            cost_override: None,
        },
        SpellMode {
            effects: &[
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::Fight {
                    fighter: TargetSlot::First,
                    foe: TargetSlot::Second,
                },
            ],
            targets: Some(TargetReq::one(TargetSpec::Object(&YOUR_BEAR))),
            second_targets: Some(TargetReq::one(TargetSpec::Object(&OPP_CREATURE))),
            additional_cost: None,
            cost_override: None,
        },
    ];
    static QUEEN_ABILS: [AbilityDef; 1] = [AbilityDef::ModalTriggered {
        trigger: Trigger::EntersBattlefield(&YOUR_BEAR),
        modes: &MODES,
        restriction: baylee_cards_dsl::ModalRestriction::None,
        zone: TriggerZone::Battlefield,
        once_per_turn: false,
        condition: None,
    }];

    let bear_face = |name: &'static str, cost_str: &'static str, p: i16, t: i16| FaceDef {
        subtypes: &[BEAR],
        ..creature_face(name, cost_str, p, t)
    };
    let cards = vec![
        card(
            BEAR_QUEEN,
            bear_face("Bear Queen", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &QUEEN_ABILS,
        ),
        card(
            CUB_BEAR,
            bear_face("Cub Bear", "{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ENEMY_GOBLIN,
            creature_face("Enemy Goblin", "{R}", 2, 3),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    // Part A: With no opponent creature on the battlefield, mode 1's required
    // `second_targets` has no legal target, so ONLY mode 0 may be chosen (CR 603.3c).
    let mut no_enemy = bench(
        601,
        cards.clone(),
        [
            Seat::with(&[BEAR_QUEEN]).holding(&[CUB_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut no_enemy, me());
    add_pool(&mut no_enemy, me(), ManaColor::Green, 1);
    let cub_hand = find_hand(&no_enemy, me(), CUB_BEAR);
    no_enemy
        .apply(me(), PlayerAction::CastSpell { card: cub_hand })
        .expect("cast Cub Bear");
    no_enemy
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes on Cub Bear");
    no_enemy
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes on Cub Bear");
    match no_enemy.pending().clone() {
        Pending::ChooseCastMode {
            player, options, ..
        } => {
            assert_eq!(player, me());
            let modes: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
            assert_eq!(
                modes,
                vec![CastModeKind::Mode(0)],
                "mode 1 must be excluded when its required second_targets has no legal target"
            );
        }
        other => panic!("expected ChooseCastMode with only Mode(0), got {other:?}"),
    }

    // Part B: With an opponent creature on the battlefield, both modes [0, 1] are
    // offered; choosing mode 1 asks first target then second target (`second_instance: true`)
    // and resolves the +1/+1 counter + fight!
    let mut with_enemy = bench(
        602,
        cards,
        [
            Seat::with(&[BEAR_QUEEN]).holding(&[CUB_BEAR]),
            Seat::with(&[ENEMY_GOBLIN]),
        ],
    );
    to_main(&mut with_enemy, me());
    add_pool(&mut with_enemy, me(), ManaColor::Green, 1);
    let cub_hand = find_hand(&with_enemy, me(), CUB_BEAR);
    let goblin = find_bf(&with_enemy, them(), ENEMY_GOBLIN);
    with_enemy
        .apply(me(), PlayerAction::CastSpell { card: cub_hand })
        .expect("cast Cub Bear");
    with_enemy
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes on Cub Bear");
    with_enemy
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes on Cub Bear");

    match with_enemy.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            let modes: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
            assert_eq!(modes, vec![CastModeKind::Mode(0), CastModeKind::Mode(1)]);
        }
        other => panic!("expected ChooseCastMode [Mode(0), Mode(1)], got {other:?}"),
    }
    with_enemy
        .apply(me(), PlayerAction::ChooseMode(1))
        .expect("choose fight mode");

    let cub_bf = find_bf(&with_enemy, me(), CUB_BEAR);
    assert!(
        !with_enemy.decision_context().second_instance,
        "first target prompt must have second_instance == false"
    );
    with_enemy
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![cub_bf],
                players: vec![],
            },
        )
        .expect("choose Cub Bear as first target");

    let ctx = with_enemy.decision_context();
    assert!(
        ctx.second_instance,
        "second target prompt must set second_instance == true"
    );
    assert_eq!(ctx.first_targets, &[cub_bf]);
    with_enemy
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![goblin],
                players: vec![],
            },
        )
        .expect("choose Enemy Goblin as second target");

    drain_stack(&mut with_enemy);
    // Cub Bear (2/2 + 1 counter = 3/3) fought Enemy Goblin (2/3): Goblin dies, Cub survives with 2 damage!
    assert_eq!(
        with_enemy.state.object(goblin).map(|o| o.zone),
        Some(Zone::Graveyard),
        "3/3 Cub Bear kills 2/3 Enemy Goblin via second_targets fight"
    );
    assert_eq!(
        with_enemy.state.object(cub_bf).map(|o| o.damage),
        Some(2),
        "Cub Bear took 2 fight damage from Enemy Goblin"
    );
}

#[test]
fn deal_damage_to_you_cost_payable_under_cant_lose_life_and_respects_prevention() {
    static ALTAR_ABILS: [AbilityDef; 1] = [paid(
        cost!("", DealDamageToYou(3)),
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    )];
    static BOLT_MANDATORY: [CostPart; 1] = [CostPart::DealDamageToYou(2)];
    static BOLT_ABILS: [AbilityDef; 1] = [spell!(&[Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::Fixed(4),
    }])];
    static TAX_COST: CostPart = CostPart::DealDamageToYou(2);
    static TAX_FALLBACK: Effect = Effect::LoseLife {
        target: PlayerRel::You,
        amount: Amount::Fixed(5),
    };
    static TAX_ABILS: [AbilityDef; 1] = [paid(
        cost!("", TapSelf),
        &[Effect::PlayerMayPayCostOr {
            player: PlayerRel::You,
            cost: &TAX_COST,
            effect: &TAX_FALLBACK,
        }],
        None,
    )];
    static LOCK_ABILS: [AbilityDef; 1] = [static_ability!(
        Filter::This,
        Modifier::CantLoseLife {
            who: PlayerRel::You
        }
    )];
    static SHIELD_ABILS: [AbilityDef; 1] = [static_ability!(
        Filter::This,
        Modifier::PreventDamageToYouFrom(&Filter::ARTIFACT)
    )];

    let cards = vec![
        card(
            PAIN_ALTAR,
            face("Pain Altar", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &ALTAR_ABILS,
        ),
        card(
            PAIN_BOLT,
            FaceDef {
                mandatory_additional_costs: &BOLT_MANDATORY,
                ..face("Pain Bolt", "{R}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            &BOLT_ABILS,
        ),
        card(
            PAIN_TAX_PROBE,
            face("Pain Tax Probe", "{1}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            &TAX_ABILS,
        ),
        card(
            PLATINUM_LOCK,
            face("Platinum Lock", "{4}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &LOCK_ABILS,
        ),
        card(
            SELF_SHIELD,
            face("Self Shield", "{1}{W}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            &SHIELD_ABILS,
        ),
    ];

    // 1. Under CantLoseLife, PayLife would be illegal, but DealDamageToYou IS
    // legal (CR 118.3) and deals damage without lowering life!
    let mut under_lock = bench(
        603,
        cards.clone(),
        [
            Seat::with(&[PAIN_ALTAR, PLATINUM_LOCK])
                .holding(&[PAIN_BOLT])
                .at(2),
            Seat::default().at(20),
        ],
    );
    to_main(&mut under_lock, me());
    add_pool(&mut under_lock, me(), ManaColor::Red, 1);
    let altar = find_bf(&under_lock, me(), PAIN_ALTAR);
    under_lock
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: altar,
                ability_index: 0,
            },
        )
        .expect("DealDamageToYou(3) is legal even at 2 life under CantLoseLife");
    drain_stack(&mut under_lock);
    assert_eq!(
        under_lock.state.players[0].life, 3,
        "CantLoseLife kept life at 2 during 3-damage cost, then GainLife(1) raised it to 3"
    );

    let bolt = find_hand(&under_lock, me(), PAIN_BOLT);
    under_lock
        .apply(me(), PlayerAction::CastSpell { card: bolt })
        .expect("spell with mandatory DealDamageToYou(2) is castable under CantLoseLife");
    drain_stack(&mut under_lock);
    assert_eq!(under_lock.state.players[1].life, 16);

    // 2. Prevention shield (`PreventDamageToYouFrom(&Filter::ARTIFACT)`) prevents
    // `Pain Altar`'s `DealDamageToYou(3)` cost damage, while unshielded `Pain Tax Probe`
    // deals 2 damage when accepted in `PlayerMayPayCostOr`!
    let mut shielded = bench(
        604,
        cards,
        [
            Seat::with(&[PAIN_ALTAR, SELF_SHIELD, PAIN_TAX_PROBE]).at(20),
            Seat::default(),
        ],
    );
    to_main(&mut shielded, me());
    let altar = find_bf(&shielded, me(), PAIN_ALTAR);
    shielded
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: altar,
                ability_index: 0,
            },
        )
        .expect("activate Pain Altar under prevention shield");
    drain_stack(&mut shielded);
    assert_eq!(
        shielded.state.players[0].life, 21,
        "prevention shield prevented the 3 artifact cost damage, then GainLife(1) -> 21"
    );

    let probe = find_bf(&shielded, me(), PAIN_TAX_PROBE);
    shielded
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("activate Pain Tax Probe");
    shielded
        .apply(me(), PlayerAction::PassPriority)
        .expect("me pass");
    shielded
        .apply(them(), PlayerAction::PassPriority)
        .expect("them pass");
    assert!(matches!(
        shielded.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Generic,
            ..
        }
    ));
    shielded
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept DealDamageToYou(2) in PlayerMayPayCostOr");
    assert_eq!(
        shielded.state.players[0].life, 19,
        "accepting DealDamageToYou(2) dealt 2 damage (not the 5-life fallback)"
    );
}

#[test]
fn counted_x_costs_discard_x_exile_graveyard_x_and_sacrifice_x_bind_amount_x() {
    static DISCARD_X_ABILS: [AbilityDef; 1] = [paid(
        cost!("", TapSelf, DiscardX(&Filter::Any)),
        &[Effect::GainLife { amount: Amount::X }],
        None,
    )];
    static EXILE_GY_X_MANDATORY: [CostPart; 1] = [CostPart::ExileFromGraveyardX(&Filter::CREATURE)];
    static EXILE_GY_X_ABILS: [AbilityDef; 1] = [spell!(&[Effect::LoseLife {
        target: PlayerRel::EachOpponent,
        amount: Amount::X,
    }])];
    static SAC_X_COST: CostPart = CostPart::SacrificeX(&Filter::CREATURE);
    static SAC_X_REWARD: Effect = Effect::GainLife { amount: Amount::X };
    static SAC_X_PROBE_ABILS: [AbilityDef; 1] = [paid(
        cost!("", TapSelf),
        &[Effect::PlayerMayPayCostThen {
            player: PlayerRel::You,
            cost: &SAC_X_COST,
            effect: &SAC_X_REWARD,
        }],
        None,
    )];

    let cards = vec![
        card(
            DISCARD_X_ENGINE,
            face("Discard X Engine", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &DISCARD_X_ABILS,
        ),
        card(
            EXILE_GY_X_SPELL,
            FaceDef {
                mandatory_additional_costs: &EXILE_GY_X_MANDATORY,
                ..face("Exile GY X Spell", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            &EXILE_GY_X_ABILS,
        ),
        card(
            SAC_X_TAX_PROBE,
            face("Sac X Probe", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &SAC_X_PROBE_ABILS,
        ),
        card(
            CUB_BEAR,
            creature_face("Cub Bear", "{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut game = bench(
        605,
        cards,
        [
            Seat::with(&[DISCARD_X_ENGINE, SAC_X_TAX_PROBE, CUB_BEAR, CUB_BEAR])
                .holding(&[CUB_BEAR, CUB_BEAR, EXILE_GY_X_SPELL])
                .at(20),
            Seat::default().at(20),
        ],
    );
    to_main(&mut game, me());
    // Note: `to_main` on turn 1 skips draw for player 0, so hand still has 3 cards!
    let hand_total = game.state.zones.list(ZoneLocation::Hand(me())).len() as u32;

    // 1. Activated `DiscardX(&Filter::Any)` with X = 2 discards 2 creature cards
    // and gains X = 2 life!
    let disc_engine = find_bf(&game, me(), DISCARD_X_ENGINE);
    let hand_bears: Vec<ObjectId> = game
        .state
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .filter(|&id| {
            game.state
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == CUB_BEAR))
        })
        .collect();
    assert_eq!(hand_bears.len(), 2);

    game.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: disc_engine,
            ability_index: 0,
        },
    )
    .expect("activate DiscardX ability");
    assert!(matches!(
        game.pending(),
        Pending::ChooseNumber {
            min: 0,
            max,
            reason: NumberPrompt::X,
            ..
        } if *max == hand_total
    ));
    game.apply(me(), PlayerAction::ChooseNumber(2))
        .expect("choose X = 2");
    game.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: hand_bears.clone(),
        },
    )
    .expect("discard both bears");
    drain_stack(&mut game);
    assert_eq!(game.state.players[0].life, 22, "DiscardX bound X = 2");

    // 2. Spell `ExileFromGraveyardX(&Filter::CREATURE)` exiles those 2 graveyard
    // bears (X = 2) and makes opponent lose X = 2 life!
    add_pool(&mut game, me(), ManaColor::Black, 1);
    let gy_spell = find_hand(&game, me(), EXILE_GY_X_SPELL);
    game.apply(me(), PlayerAction::CastSpell { card: gy_spell })
        .expect("cast ExileFromGraveyardX spell");
    assert!(matches!(
        game.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 2,
            reason: NumberPrompt::X,
            ..
        }
    ));
    game.apply(me(), PlayerAction::ChooseNumber(2))
        .expect("choose X = 2 for graveyard exile");
    game.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![hand_bears[0]],
        },
    )
    .expect("exile first graveyard bear");
    game.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![hand_bears[1]],
        },
    )
    .expect("exile second graveyard bear");
    drain_stack(&mut game);
    assert_eq!(
        game.state.players[1].life, 18,
        "ExileFromGraveyardX bound X = 2 on spell resolution"
    );

    // 3. `PlayerMayPayCostThen` with `SacrificeX(&Filter::CREATURE)`: directly
    // prompts `ChooseCards { min: 0, max: 2 }`, sacrifices both battlefield bears,
    // and gains X = 2 life!
    let sac_probe = find_bf(&game, me(), SAC_X_TAX_PROBE);
    let bf_bears: Vec<ObjectId> = game
        .state
        .battlefield_seen()
        .filter(|&id| {
            game.state
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == CUB_BEAR))
        })
        .collect();
    assert_eq!(bf_bears.len(), 2);
    game.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: sac_probe,
            ability_index: 0,
        },
    )
    .expect("activate SacrificeX PlayerMayPayCostThen probe");
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass");
    assert!(matches!(
        game.pending(),
        Pending::ChooseCards {
            min: 0,
            max: 2,
            prompt: ChoicePrompt::CostSacrifice,
            ..
        }
    ));
    game.apply(me(), PlayerAction::ChooseObjects { objects: bf_bears })
        .expect("sacrifice both bears");
    assert_eq!(
        game.state.players[0].life, 24,
        "PlayerMayPayCostThen SacrificeX bound res.x = 2"
    );
}

#[test]
fn forage_cost_pays_with_one_food_or_three_graveyard_cards_and_blocks_at_two() {
    static FORAGE_ACT_ABILS: [AbilityDef; 1] = [paid(
        cost!("", Forage),
        &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }],
        None,
    )];
    static FORAGE_KICK_PARTS: [CostPart; 1] = [CostPart::Forage];
    static FORAGE_KICK_COSTS: [Cost; 1] = [Cost {
        mana: ManaCost::ZERO,
        parts: &FORAGE_KICK_PARTS,
    }];
    static FORAGE_SPELL_ABILS: [AbilityDef; 1] = [spell!(&[Effect::IfKicked {
        then: &[Effect::GainLife {
            amount: Amount::Fixed(5),
        }],
        otherwise: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
    }])];
    static FORAGE_COST: CostPart = CostPart::Forage;
    static FORAGE_REWARD: Effect = Effect::GainLife {
        amount: Amount::Fixed(3),
    };
    static FORAGE_MAY_ABILS: [AbilityDef; 1] = [paid(
        cost!("", TapSelf),
        &[Effect::PlayerMayPayCostThen {
            player: PlayerRel::You,
            cost: &FORAGE_COST,
            effect: &FORAGE_REWARD,
        }],
        None,
    )];

    let cards = vec![
        card(
            FORAGE_SQUIRREL,
            creature_face("Forage Squirrel", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &FORAGE_ACT_ABILS,
        ),
        card(
            FORAGE_KICK_SPELL,
            FaceDef {
                additional_costs: &FORAGE_KICK_COSTS,
                ..face("Forage Kick Spell", "{G}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            &FORAGE_SPELL_ABILS,
        ),
        card(
            FORAGE_MAY_PROBE,
            face("Forage May Probe", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &FORAGE_MAY_ABILS,
        ),
        card(
            FOOD_ARTIFACT,
            FaceDef {
                subtypes: &[FOOD],
                ..face("Test Food", "{2}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CUB_BEAR,
            creature_face("Cub Bear", "{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    // Start with 0 Foods and 2 cards moved to graveyard -> Forage NOT payable!
    let mut game = bench(
        606,
        cards,
        [
            Seat::with(&[FORAGE_SQUIRREL, FORAGE_MAY_PROBE])
                .holding(&[
                    CUB_BEAR,
                    CUB_BEAR,
                    CUB_BEAR,
                    FOOD_ARTIFACT,
                    FORAGE_KICK_SPELL,
                ])
                .at(20),
            Seat::default(),
        ],
    );
    to_main(&mut game, me());
    let squirrel = find_bf(&game, me(), FORAGE_SQUIRREL);
    let hand_bears: Vec<ObjectId> = game
        .state
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .filter(|&id| {
            game.state
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == CUB_BEAR))
        })
        .collect();
    for &b in &hand_bears[..2] {
        game.state
            .move_object(
                b,
                ZoneLocation::Graveyard(me()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("seed 2 graveyard cards");
    }
    refresh_offer(&mut game);
    let Pending::Priority { legal, .. } = game.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(squirrel, 0)),
        "Forage must NOT be legal with 0 Foods and only 2 graveyard cards"
    );

    // Move the 3rd Bear into graveyard: now Forage IS legal via 3 graveyard cards!
    game.state
        .move_object(
            hand_bears[2],
            ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("seed 3rd graveyard card");
    refresh_offer(&mut game);
    game.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: squirrel,
            ability_index: 0,
        },
    )
    .expect("Forage legal with 3 graveyard cards");
    game.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![hand_bears[0]],
        },
    )
    .expect("choose 1st graveyard card for Forage");
    assert!(matches!(
        game.pending(),
        Pending::ChooseCards {
            min: 2,
            max: 2,
            prompt: ChoicePrompt::CostExile,
            ..
        }
    ));
    game.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![hand_bears[1], hand_bears[2]],
        },
    )
    .expect("choose remaining 2 graveyard cards for Forage");
    drain_stack(&mut game);
    assert_eq!(game.state.players[0].life, 22);
    assert!(
        game.state
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .is_empty(),
        "all 3 graveyard cards were exiled to forage"
    );

    // Put Food onto battlefield (with 0 graveyard cards): cast Forage kicker spell
    // paying with the 1 Food!
    add_pool(&mut game, me(), ManaColor::Colorless, 2);
    let food_hand = find_hand(&game, me(), FOOD_ARTIFACT);
    game.apply(me(), PlayerAction::CastSpell { card: food_hand })
        .expect("cast Food");
    drain_stack(&mut game);
    let food_bf = find_bf(&game, me(), FOOD_ARTIFACT);

    add_pool(&mut game, me(), ManaColor::Green, 1);
    let kick_spell = find_hand(&game, me(), FORAGE_KICK_SPELL);
    game.apply(me(), PlayerAction::CastSpell { card: kick_spell })
        .expect("cast Forage Kick Spell");
    game.apply(me(), PlayerAction::YesNo(true))
        .expect("accept Forage optional additional cost");
    game.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![food_bf],
        },
    )
    .expect("sacrifice Food to forage");
    drain_stack(&mut game);
    assert_eq!(
        game.state.players[0].life, 27,
        "kicked Forage spell gained 5 life (22 + 5 = 27)"
    );
    assert_eq!(
        game.state.object(food_bf).map(|o| o.zone),
        Some(Zone::Graveyard),
        "Food was sacrificed to forage"
    );
}

#[test]
fn modular_enters_with_doubled_counters_transfers_ltb_p1p1_to_artifact_creature_and_stacks_per_instance()
 {
    static MODULAR_2: [EnterModifier; 1] = [EnterModifier::modular(2)];
    static DOUBLER_ABILS: [AbilityDef; 1] = [static_ability!(
        Filter::Any,
        Modifier::DoubleCountersPlacedByYou
    )];
    static GRANTER_ABILS: [AbilityDef; 1] = [static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::AddKeyword(KeywordSet::MODULAR)
    )];
    static KILL_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let cards = vec![
        card(
            MODULAR_WORKER,
            FaceDef {
                enter_modifiers: &MODULAR_2,
                types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
                power: Some(0),
                toughness: Some(0),
                mana_cost: ManaCost::parse("{2}"),
                castable_from_hand: true,
                ..land_face("Modular Worker")
            },
            KeywordSet::MODULAR,
            &[],
        ),
        card(
            ARTIFACT_CONSTRUCT,
            FaceDef {
                types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
                power: Some(1),
                toughness: Some(1),
                mana_cost: ManaCost::parse("{1}"),
                castable_from_hand: true,
                ..land_face("Artifact Construct")
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CUB_BEAR,
            creature_face("Nonartifact Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            COUNTER_DOUBLER,
            face("Counter Doubler", "{4}{G}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            &DOUBLER_ABILS,
        ),
        card(
            MODULAR_GRANTER,
            face("Modular Granter", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &GRANTER_ABILS,
        ),
        card(
            KILL_SPELL,
            face("Kill Spell", "{B}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &KILL_ABILS,
        ),
    ];

    let mut game = bench(
        607,
        cards,
        [
            Seat::with(&[
                COUNTER_DOUBLER,
                MODULAR_GRANTER,
                ARTIFACT_CONSTRUCT,
                CUB_BEAR,
            ])
            .holding(&[MODULAR_WORKER, KILL_SPELL]),
            Seat::default(),
        ],
    );
    to_main(&mut game, me());
    let construct = find_bf(&game, me(), ARTIFACT_CONSTRUCT);
    let nonartifact_bear = find_bf(&game, me(), CUB_BEAR);

    // Cast Modular 2 Worker under Counter Doubler -> enters with 4 +1/+1 counters (CR 614.16)!
    add_pool(&mut game, me(), ManaColor::Colorless, 2);
    let worker_hand = find_hand(&game, me(), MODULAR_WORKER);
    game.apply(me(), PlayerAction::CastSpell { card: worker_hand })
        .expect("cast Modular Worker");
    drain_stack(&mut game);
    let worker_bf = find_bf(&game, me(), MODULAR_WORKER);
    assert_eq!(
        game.state
            .object(worker_bf)
            .map(|o| o.counters.get(CounterKind::P1P1)),
        Some(4),
        "Modular 2 entered with 4 +1/+1 counters under DoubleCountersPlacedByYou"
    );

    // Destroy Modular Worker (which has printed MODULAR + granted MODULAR = 2 instances!).
    add_pool(&mut game, me(), ManaColor::Black, 1);
    let kill = find_hand(&game, me(), KILL_SPELL);
    game.apply(me(), PlayerAction::CastSpell { card: kill })
        .expect("cast Kill Spell");
    game.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![worker_bf],
            players: vec![],
        },
    )
    .expect("target Modular Worker");
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass on Kill Spell");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass on Kill Spell");

    // Both Modular death triggers prompt for an ARTIFACT CREATURE target (excluding Nonartifact Bear!).
    for instance in 0..2 {
        match game.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                assert!(
                    options.contains(&construct),
                    "instance {instance}: Artifact Construct must be a legal Modular target"
                );
                assert!(
                    !options.contains(&nonartifact_bear),
                    "instance {instance}: non-artifact creature must NOT be a legal Modular target"
                );
            }
            other => panic!("expected Modular ChooseTargets #{instance}, got {other:?}"),
        }
        game.apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![construct],
                players: vec![],
            },
        )
        .expect("target Artifact Construct");
    }

    // Resolve both Modular triggers (accepting `MayDo` on the first and declining on the second
    // to prove `MayDo` optionality AND exact LKI +1/+1 counter transfer!).
    for accept in [true, false] {
        game.apply(me(), PlayerAction::PassPriority)
            .expect("me pass");
        game.apply(them(), PlayerAction::PassPriority)
            .expect("them pass");
        assert!(matches!(
            game.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        ));
        game.apply(me(), PlayerAction::YesNo(accept))
            .expect("answer Modular MayDo");
    }
    // Accepted 1st trigger put 4 LKI counters, doubled to 8 by Counter Doubler; declined 2nd put 0!
    assert_eq!(
        game.state
            .object(construct)
            .map(|o| o.counters.get(CounterKind::P1P1)),
        Some(8),
        "accepted Modular trigger transferred 4 LKI +1/+1 counters (doubled to 8), declined 2nd added 0"
    );
}

#[test]
fn soulbond_pairs_both_directions_cancels_redundant_trigger_and_unpairs_on_leave_control_or_type_loss()
 {
    static PALADIN_ABILS: [AbilityDef; 2] = [
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::SOULBOND_PAIR,
            modifier: Modifier::AddKeyword(KeywordSet::DOUBLE_STRIKE),
            condition: Some(Condition::IsPaired),
            source_zone: ZoneRef::Battlefield,
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::PtModify,
            filter: Filter::SOULBOND_PAIR,
            modifier: Modifier::ModifyPT(2, 2),
            condition: Some(Condition::IsPaired),
            source_zone: ZoneRef::Battlefield,
        }),
    ];
    static MAULER_ABILS: [AbilityDef; 1] = [AbilityDef::Static(StaticAbility {
        layer: Layer::Ability,
        filter: Filter::SOULBOND_PAIR,
        modifier: Modifier::AddKeyword(KeywordSet::HASTE),
        condition: Some(Condition::IsPaired),
        source_zone: ZoneRef::Battlefield,
    })];
    static BLINK_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static STEAL_ABILS: [AbilityDef; 1] = [spell!(
        &[Effect::ChangeController {
            new_controller: PlayerRel::You,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static DEANIMATE_ABILS: [AbilityDef; 1] = [spell!(
        &[
            Effect::CreateContinuousEffect {
                layer: Layer::Type,
                filter: &Filter::This,
                modifier: Modifier::AddType(TypeSet::ARTIFACT),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Type,
                filter: &Filter::This,
                modifier: Modifier::RemoveType(TypeSet::CREATURE),
                duration: Duration::UntilEndOfTurn,
            }
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let cards = vec![
        card(
            SOULBOND_PALADIN,
            creature_face("Soulbond Paladin", "{1}{W}{W}", 2, 2),
            KeywordSet::SOULBOND,
            &PALADIN_ABILS,
        ),
        card(
            SOULBOND_MAULER,
            creature_face("Soulbond Mauler", "{1}{R}", 2, 1),
            KeywordSet::SOULBOND,
            &MAULER_ABILS,
        ),
        card(
            CUB_BEAR,
            creature_face("Cub Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            BLINK_INSTANT,
            face("Blink Instant", "{W}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &BLINK_ABILS,
        ),
        card(
            STEAL_INSTANT,
            face("Steal Instant", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &STEAL_ABILS,
        ),
        card(
            DEANIMATE_INSTANT,
            face("Deanimate Instant", "{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &DEANIMATE_ABILS,
        ),
    ];

    // 1. Dual-Soulbond ETB (`CR 702.95a` + `CR 603.4`): Paladin is on the battlefield
    // unpaired; Mauler enters with Soulbond -> BOTH Soulbond triggers stack, accepting
    // the top one pairs them, and the second trigger is cancelled at resolution by CR 603.4!
    let mut game = bench(
        608,
        cards,
        [
            Seat::with(&[SOULBOND_PALADIN]).holding(&[
                SOULBOND_MAULER,
                CUB_BEAR,
                BLINK_INSTANT,
                DEANIMATE_INSTANT,
            ]),
            Seat::default().holding(&[STEAL_INSTANT]),
        ],
    );
    to_main(&mut game, me());
    let paladin = find_bf(&game, me(), SOULBOND_PALADIN);
    assert_eq!(game.state.paired_partner(paladin), None);

    add_pool(&mut game, me(), ManaColor::Red, 2);
    let mauler_hand = find_hand(&game, me(), SOULBOND_MAULER);
    game.apply(me(), PlayerAction::CastSpell { card: mauler_hand })
        .expect("cast Soulbond Mauler");
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass on Mauler");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass on Mauler");

    let mauler = find_bf(&game, me(), SOULBOND_MAULER);
    assert_eq!(
        game.state.zones.list(ZoneLocation::Stack).len(),
        2,
        "both Paladin's other-ETB and Mauler's self-ETB Soulbond abilities triggered"
    );

    // Resolve top trigger on the stack!
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass on 1st Soulbond trigger");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass on 1st Soulbond trigger");
    match game.pending().clone() {
        Pending::ChooseCards { options, .. } => {
            assert_eq!(options, vec![paladin]);
            game.apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![paladin],
                },
            )
            .expect("choose Paladin to pair with Mauler");
        }
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        } => {
            game.apply(me(), PlayerAction::YesNo(true))
                .expect("confirm pairing Paladin with Mauler");
        }
        other => panic!("expected Soulbond pairing choice, got {other:?}"),
    }

    // Both creatures are now reciprocally paired and share +2/+2, DOUBLE_STRIKE, and HASTE!
    assert_eq!(game.state.paired_partner(paladin), Some(mauler));
    assert_eq!(game.state.paired_partner(mauler), Some(paladin));
    let pal_chars = game.state.object(paladin).unwrap().characteristics();
    let maul_chars = game.state.object(mauler).unwrap().characteristics();
    assert_eq!((pal_chars.power, pal_chars.toughness), (Some(4), Some(4)));
    assert_eq!((maul_chars.power, maul_chars.toughness), (Some(4), Some(3)));
    assert!(
        pal_chars
            .keywords
            .contains(KeywordSet::DOUBLE_STRIKE.union(KeywordSet::HASTE))
    );
    assert!(
        maul_chars
            .keywords
            .contains(KeywordSet::DOUBLE_STRIKE.union(KeywordSet::HASTE))
    );

    // Passing priority on the 2nd Soulbond trigger MUST NOT prompt again (`CR 603.4`)!
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass on 2nd Soulbond trigger");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass on 2nd Soulbond trigger");
    assert!(
        matches!(game.pending(), Pending::Priority { .. }) && game.state.zones.stack_is_empty(),
        "second Soulbond trigger was removed on resolution by CR 603.4 without asking"
    );

    // 2. Zone-leave unpairing (`CR 702.95e`): Blink Mauler -> Paladin immediately
    // unpairs and returns to 2/2 before re-entry triggers resolve!
    add_pool(&mut game, me(), ManaColor::White, 1);
    let blink = find_hand(&game, me(), BLINK_INSTANT);
    game.apply(me(), PlayerAction::CastSpell { card: blink })
        .expect("cast Blink Instant");
    game.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![mauler],
            players: vec![],
        },
    )
    .expect("target Mauler");
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass on Blink");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass on Blink");
    assert_eq!(game.state.paired_partner(paladin), None);
    assert_eq!(
        game.state.object(paladin).unwrap().characteristics().power,
        Some(2),
        "Paladin lost its +2/+2 paired bonus as soon as Mauler left the battlefield"
    );

    // Re-pair Paladin and Mauler on the top re-entry trigger, then test losing CREATURE type (`CR 702.95e`)!
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass");
    match game.pending().clone() {
        Pending::ChooseCards { options, .. } => {
            game.apply(me(), PlayerAction::ChooseObjects { objects: options })
                .expect("re-pair");
        }
        Pending::YesNo { .. } => {
            game.apply(me(), PlayerAction::YesNo(true))
                .expect("re-pair");
        }
        other => panic!("expected Soulbond re-pair prompt, got {other:?}"),
    }
    drain_stack(&mut game);
    assert_eq!(game.state.paired_partner(paladin), Some(mauler));

    // Turn Mauler into a noncreature Artifact -> immediately unpairs both (`CR 702.95e`)!
    add_pool(&mut game, me(), ManaColor::Blue, 1);
    let deanimate = find_hand(&game, me(), DEANIMATE_INSTANT);
    game.apply(me(), PlayerAction::CastSpell { card: deanimate })
        .expect("cast Deanimate Instant");
    game.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![mauler],
            players: vec![],
        },
    )
    .expect("target Mauler");
    drain_stack(&mut game);
    assert_eq!(
        game.state.paired_partner(paladin),
        None,
        "stopping being a creature immediately unpairs both permanents (CR 702.95e)"
    );
    assert_eq!(
        game.state.object(paladin).unwrap().characteristics().power,
        Some(2)
    );

    // 3. Cast Cub Bear -> Paladin's other-ETB Soulbond triggers and pairs Paladin + Cub Bear;
    // then opponent steals Cub Bear (`Effect::ChangeController`) -> immediately unpairs both (`CR 702.95e`)!
    add_pool(&mut game, me(), ManaColor::Green, 2);
    let cub_hand = find_hand(&game, me(), CUB_BEAR);
    game.apply(me(), PlayerAction::CastSpell { card: cub_hand })
        .expect("cast Cub Bear");
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass");
    let cub_bf = find_bf(&game, me(), CUB_BEAR);
    game.apply(me(), PlayerAction::PassPriority)
        .expect("me pass");
    game.apply(them(), PlayerAction::PassPriority)
        .expect("them pass");
    game.apply(me(), PlayerAction::YesNo(true))
        .expect("confirm pairing Paladin with Cub Bear");
    assert_eq!(game.state.paired_partner(paladin), Some(cub_bf));

    add_pool(&mut game, them(), ManaColor::Red, 1);
    game.apply(me(), PlayerAction::PassPriority)
        .expect("pass priority to opponent");
    let steal = find_hand(&game, them(), STEAL_INSTANT);
    game.apply(them(), PlayerAction::CastSpell { card: steal })
        .expect("opponent casts Steal Instant");
    game.apply(
        them(),
        PlayerAction::ChooseTargets {
            objects: vec![cub_bf],
            players: vec![],
        },
    )
    .expect("opponent targets Cub Bear");
    drain_stack(&mut game);
    assert_eq!(
        game.state.paired_partner(paladin),
        None,
        "another player gaining control of a paired creature immediately unpairs both (CR 702.95e)"
    );
    assert_eq!(
        game.state.paired_partner(cub_bf),
        None,
        "stolen Cub Bear is also unpaired"
    );
}
