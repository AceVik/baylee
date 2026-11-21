//! Wave 8b engine rule tests:
//! - `Retrace` (`CR 702.81a`): `KeywordSet::RETRACE` graveyard cast permission
//!   requiring `CostPart::Discard(&Filter::LAND)` as an additional cost, returning
//!   to the graveyard on resolution rather than exiling itself.
//! - `Trigger::Scried` (`CR 701.22a–d`): `GameEvent::PlayerScried` emitted after
//!   scrying `>= 1` cards (including on an empty library per `CR 701.22d`;
//!   suppressed on scry 0 per `CR 701.22b`), binding `PlayerRel::EventPlayer`
//!   and `Amount::EventAmount`.
//! - `Monstrosity` (`CR 701.37a–c`, `CR 400.7`): `Effect::Monstrosity(Amount)`
//!   placing `N` `+1/+1` counters once while not already monstrous, attaching
//!   noncopiable `Rider::Monstrous` (`Condition::IsMonstrous`, `Filter::Monstrous`),
//!   firing `Trigger::BecomesMonstrous` with `Amount::EventAmount = N`, and
//!   resetting on leaving the battlefield.
//! - `Suspect` (`CR 701.60a–d`): `Effect::Suspect`, `Effect::Unsuspect`,
//!   `EnterModifier::Suspected`, `Condition::IsSuspected`, and `Filter::Suspected`
//!   granting `KeywordSet::MENACE | KeywordSet::CANT_BLOCK` in layer 6 while
//!   suspected (noncopiable).
//! - `Riot` (`CR 702.136a–b`) & `Unleash` (`CR 702.98a`): per-instance as-enters
//!   prompts (`YesNoPrompt::Riot`, `YesNoPrompt::Unleash`), `Rider::RiotHaste`
//!   granting indefinite `HASTE`, and `UNLEASH` granting `CANT_BLOCK` whenever
//!   the permanent has at least one `+1/+1` counter.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{ArrangePrompt, CastModeKind, ChoicePrompt, YesNoPrompt};
use crate::object::Rider;
use baylee_cards_dsl::prelude::{activated, cost, spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, EnterModifier, Filter, Modifier, PlayerRel, TargetSpec, Trigger,
};
use baylee_core::mana::ManaColor;

const RETRACE_BOLT: u32 = 8850;
const DUMMY_SORCERY: u32 = 8851;
const SCRY_ZERO_SPELL: u32 = 8852;
const SCRY_TWO_SPELL: u32 = 8853;
const SCRY_THREE_SPELL: u32 = 8854;
const SCRY_WATCHER: u32 = 8855;
const MONSTROUS_HYDRA: u32 = 8856;
const MONSTROUS_LORD: u32 = 8857;
const CLONE_SHIFTER: u32 = 8858;
const FLICKER_INSTANT: u32 = 8859;
const SUSPECT_SPELL: u32 = 8860;
const UNSUSPECT_SPELL: u32 = 8861;
const BORN_SUSPECTED: u32 = 8862;
const VANILLA_BEAR: u32 = 8863;
const SUSPECT_WATCHER: u32 = 8864;
const RIOT_WARRIOR: u32 = 8865;
const RIOT_ANTHEM: u32 = 8866;
const UNLEASH_DEVIL: u32 = 8867;
const GROW_INSTANT: u32 = 8868;
const SHRINK_COUNTER_INSTANT: u32 = 8869;

fn leak(id: u32, face: FaceDef) -> &'static CardDef {
    let keywords = face.keywords;
    let abilities = face.abilities;
    card(id, face, keywords, abilities)
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

fn drain_stack(engine: &mut Bench) {
    while !engine.state.zones.stack_is_empty()
        || matches!(engine.pending(), Pending::Arrange { .. })
    {
        match engine.pending().clone() {
            Pending::Arrange {
                player,
                cards,
                prompt: ArrangePrompt::TriggerOrder,
                ..
            } => {
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

fn retrace_bolt() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::Player(PlayerRel::EachOpponent),
    }])];
    leak(
        RETRACE_BOLT,
        FaceDef {
            keywords: KeywordSet::RETRACE,
            abilities: &ABILITIES,
            ..face("Retrace Bolt", "{R}", TypeSet::INSTANT)
        },
    )
}

fn dummy_sorcery() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];
    leak(
        DUMMY_SORCERY,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Dummy Sorcery", "{W}", TypeSet::SORCERY)
        },
    )
}

fn scry_spell(id: u32, name: &'static str, n: u32) -> &'static CardDef {
    let effects: &'static [Effect] = Box::leak(Box::new([Effect::Scry {
        amount: Amount::Fixed(n),
    }]));
    let abilities: &'static [AbilityDef] = Box::leak(Box::new([spell!(effects)]));
    leak(
        id,
        FaceDef {
            abilities,
            ..face(name, "{U}", TypeSet::INSTANT)
        },
    )
}

fn scry_watcher() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 2] = [
        triggered!(
            Trigger::Scried {
                who: PlayerRel::You
            },
            &[Effect::GainLife {
                amount: Amount::EventAmount
            }]
        ),
        triggered!(
            Trigger::Scried {
                who: PlayerRel::EachPlayer
            },
            &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::EventPlayer,
            }]
        ),
    ];
    leak(
        SCRY_WATCHER,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Scry Watcher", "{1}{U}", TypeSet::ENCHANTMENT)
        },
    )
}

fn monstrous_hydra() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 3] = [
        activated!(cost!("{X}{G}"), &[Effect::Monstrosity(Amount::X)]),
        triggered!(
            Trigger::BecomesMonstrous,
            &[Effect::GainLife {
                amount: Amount::EventAmount
            }]
        ),
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(2, 0),
            condition = Some(Condition::IsMonstrous)
        ),
    ];
    leak(
        MONSTROUS_HYDRA,
        FaceDef {
            abilities: &ABILITIES,
            ..creature_face("Monstrous Hydra", "{2}{G}", 3, 3)
        },
    )
}

fn monstrous_lord() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [static_ability!(
        Filter::Monstrous,
        Modifier::AddKeyword(KeywordSet::TRAMPLE)
    )];
    leak(
        MONSTROUS_LORD,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Monstrous Lord", "{1}{G}", TypeSet::ENCHANTMENT)
        },
    )
}

fn clone_shifter() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[],
    }];
    leak(
        CLONE_SHIFTER,
        FaceDef {
            abilities: &ABILITIES,
            ..creature_face("Clone Shifter", "{3}{U}", 0, 0)
        },
    )
}

fn flicker_instant() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    leak(
        FLICKER_INSTANT,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Flicker Instant", "{W}", TypeSet::INSTANT)
        },
    )
}

fn suspect_spell() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Suspect {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    leak(
        SUSPECT_SPELL,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Suspect Spell", "{R}", TypeSet::INSTANT)
        },
    )
}

fn unsuspect_spell() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::Unsuspect {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    leak(
        UNSUSPECT_SPELL,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Unsuspect Spell", "{W}", TypeSet::INSTANT)
        },
    )
}

fn born_suspected() -> &'static CardDef {
    static MODIFIERS: [EnterModifier; 1] = [EnterModifier::Suspected];
    static ABILITIES: [AbilityDef; 1] = [static_ability!(
        Filter::This,
        Modifier::ModifyPT(1, 0),
        condition = Some(Condition::IsSuspected)
    )];
    leak(
        BORN_SUSPECTED,
        FaceDef {
            enter_modifiers: &MODIFIERS,
            abilities: &ABILITIES,
            ..creature_face("Born Suspected", "{1}{B}", 2, 2)
        },
    )
}

fn suspect_watcher() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] =
        [static_ability!(Filter::Suspected, Modifier::ModifyPT(1, 0))];
    leak(
        SUSPECT_WATCHER,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Suspect Watcher", "{1}{R}", TypeSet::ENCHANTMENT)
        },
    )
}

fn vanilla_bear() -> &'static CardDef {
    leak(VANILLA_BEAR, creature_face("Vanilla Bear", "{1}{G}", 2, 2))
}

fn riot_warrior() -> &'static CardDef {
    leak(
        RIOT_WARRIOR,
        FaceDef {
            keywords: KeywordSet::RIOT,
            ..creature_face("Riot Warrior", "{R}{G}", 2, 2)
        },
    )
}

fn riot_anthem() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::AddKeyword(KeywordSet::RIOT)
    )];
    leak(
        RIOT_ANTHEM,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Riot Anthem", "{1}{R}{G}", TypeSet::ENCHANTMENT)
        },
    )
}

fn unleash_devil() -> &'static CardDef {
    leak(
        UNLEASH_DEVIL,
        FaceDef {
            keywords: KeywordSet::UNLEASH,
            ..creature_face("Unleash Devil", "{R}", 1, 1)
        },
    )
}

fn grow_instant() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    leak(
        GROW_INSTANT,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Grow Instant", "{G}", TypeSet::INSTANT)
        },
    )
}

fn shrink_counter_instant() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::Spell {
        effects: &[Effect::AddCounter {
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    leak(
        SHRINK_COUNTER_INSTANT,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Shrink Counter Instant", "{B}", TypeSet::INSTANT)
        },
    )
}

/// 1. `Retrace` (`CR 702.81a`):
/// - Graveyard card with `KeywordSet::RETRACE` cannot be cast when holding only nonlands.
/// - Castable from owner's graveyard when holding a land (`CastModeKind::Retrace`),
///   prompts `ChoicePrompt::CostDiscard` over land cards only, resolves, and returns
///   to the graveyard (NOT exiled), allowing a second Retrace cast with another land!
#[test]
fn wave8b_retrace_casts_from_graveyard_discarding_land_and_returns_to_graveyard() {
    let mut engine = bench(
        2,
        vec![retrace_bolt(), dummy_sorcery()],
        [
            Seat::with(&[]).holding(&[DUMMY_SORCERY, RETRACE_BOLT]),
            Seat::with(&[]).holding(&[RETRACE_BOLT]),
        ],
    );
    to_main(&mut engine, me());

    let my_bolt = the(&engine, ZoneLocation::Hand(me()), RETRACE_BOLT);
    let opp_bolt = the(&engine, ZoneLocation::Hand(them()), RETRACE_BOLT);
    let _ = engine.state.move_object(
        my_bolt,
        ZoneLocation::Graveyard(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    let _ = engine.state.move_object(
        opp_bolt,
        ZoneLocation::Graveyard(them()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );

    // 1) With only a nonland card in hand, `my_bolt` is NOT castable via Retrace.
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    let legal = engine.compute_legal(me());
    assert!(
        !legal.castable.contains(&my_bolt),
        "CR 702.81a: Retrace requires a land card in hand to discard"
    );

    // 2) Move two Forests from library into hand -> `my_bolt` offers `CastModeKind::Retrace`,
    //    while `opp_bolt` (in opponent's graveyard) is never offered to `me()`.
    let land1 = *engine
        .state
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    let _ = engine.state.move_object(
        land1,
        ZoneLocation::Hand(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    let land2 = *engine
        .state
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    let _ = engine.state.move_object(
        land2,
        ZoneLocation::Hand(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    refresh_offer(&mut engine);
    let legal = engine.compute_legal(me());
    assert!(
        legal
            .cast_ways
            .iter()
            .any(|(id, w)| *id == my_bolt && w.kind == CastModeKind::Retrace),
        "CR 702.81a: graveyard card with RETRACE offers CastModeKind::Retrace"
    );
    assert!(
        !legal.castable.contains(&opp_bolt),
        "opponent's Retrace card in opponent's graveyard is not castable by me"
    );

    // Cast #1 via Retrace -> discards `land1`, deals 2 damage, returns to graveyard!
    engine
        .apply(me(), PlayerAction::CastSpell { card: my_bolt })
        .expect("cast my_bolt via Retrace #1");
    match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            options,
            prompt: ChoicePrompt::CostDiscard,
            ..
        } => {
            assert_eq!(player, me());
            assert_eq!(
                options,
                vec![land1, land2],
                "only land cards in hand are offered for Retrace discard"
            );
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseObjects {
                        objects: vec![land1],
                    },
                )
                .expect("discard land1");
        }
        other => panic!("expected ChoicePrompt::CostDiscard for Retrace, got {other:?}"),
    }
    drain_stack(&mut engine);
    assert_eq!(life(&engine, them()), 18);
    assert_eq!(
        engine.state.object(my_bolt).expect("my_bolt").zone,
        Zone::Graveyard,
        "CR 702.81a: Retrace spell returns to graveyard on resolution instead of exiling"
    );

    // Cast #2 via Retrace in the very same turn discarding `land2`!
    engine
        .apply(me(), PlayerAction::CastSpell { card: my_bolt })
        .expect("cast my_bolt via Retrace #2");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![land2],
            },
        )
        .expect("discard land2");
    drain_stack(&mut engine);
    assert_eq!(life(&engine, them()), 16);
    assert_eq!(
        engine.state.object(my_bolt).expect("my_bolt").zone,
        Zone::Graveyard
    );
}

/// 2. `Trigger::Scried` (`CR 701.22a–d`):
/// - Scry 0 (`CR 701.22b`) emits no event and triggers nothing.
/// - Scry 2 with library cards suspends `Arrange`, triggers only AFTER completion
///   (`CR 701.22d`), and binds `Amount::EventAmount = 2` and `EventPlayer = me()`.
/// - Scry 3 on an empty library (`CR 701.22d`) still triggers `Trigger::Scried`
///   with `Amount::EventAmount = 3`!
#[test]
fn wave8b_trigger_scried_fires_with_event_player_and_amount_including_empty_library_and_skips_scry_zero()
 {
    let mut engine = bench(
        2,
        vec![
            scry_spell(SCRY_ZERO_SPELL, "Scry Zero", 0),
            scry_spell(SCRY_TWO_SPELL, "Scry Two", 2),
            scry_spell(SCRY_THREE_SPELL, "Scry Three", 3),
            scry_watcher(),
        ],
        [
            Seat::with(&[SCRY_WATCHER]).holding(&[
                SCRY_ZERO_SPELL,
                SCRY_TWO_SPELL,
                SCRY_THREE_SPELL,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let scry0 = the(&engine, ZoneLocation::Hand(me()), SCRY_ZERO_SPELL);
    let scry2 = the(&engine, ZoneLocation::Hand(me()), SCRY_TWO_SPELL);
    let scry3 = the(&engine, ZoneLocation::Hand(me()), SCRY_THREE_SPELL);

    // Ensure exactly 2 cards in me()'s library first.
    while engine.state.zones.list(ZoneLocation::Library(me())).len() > 2 {
        let top = *engine
            .state
            .zones
            .list(ZoneLocation::Library(me()))
            .last()
            .unwrap();
        let _ = engine.state.move_object(
            top,
            ZoneLocation::Exile(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
    }
    let lib_cards = engine.state.zones.list(ZoneLocation::Library(me())).clone();
    let (lib_a, lib_b) = (lib_cards[0], lib_cards[1]);

    // 1) Scry 0 -> no `PlayerScried` event, life stays 20!
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: scry0 })
        .expect("cast scry0");
    drain_stack(&mut engine);
    assert_eq!(
        life(&engine, me()),
        20,
        "CR 701.22b: scry 0 does not trigger Scried"
    );

    // 2) Scry 2 -> Arrange prompt; triggers fire AFTER Arrange with EventAmount = 2 (+2 - 1 = +1 -> 21)!
    engine
        .apply(me(), PlayerAction::CastSpell { card: scry2 })
        .expect("cast scry2");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass me");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass them");
    match engine.pending().clone() {
        Pending::Arrange {
            player,
            prompt: ArrangePrompt::Scry,
            ..
        } => {
            assert_eq!(player, me());
            assert!(
                engine.trigger_queue.is_empty(),
                "CR 701.22d: Scried does not trigger before scry arrangement finishes"
            );
            engine
                .apply(
                    me(),
                    PlayerAction::Arrange {
                        piles: vec![vec![lib_a], vec![lib_b]],
                    },
                )
                .expect("finish scry 2");
        }
        other => panic!("expected ArrangePrompt::Scry, got {other:?}"),
    }
    drain_stack(&mut engine);
    assert_eq!(
        life(&engine, me()),
        21,
        "CR 701.22d: Scry 2 gained 2 life (EventAmount) and lost 1 life (EventPlayer)"
    );

    // 3) Empty `me()`'s library completely, then Scry 3 -> still triggers with EventAmount = 3 (+3 - 1 = +2 -> 23)!
    for card in [lib_a, lib_b] {
        let _ = engine.state.move_object(
            card,
            ZoneLocation::Exile(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
    }
    assert!(
        engine
            .state
            .zones
            .list(ZoneLocation::Library(me()))
            .is_empty()
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: scry3 })
        .expect("cast scry3 on empty library");
    drain_stack(&mut engine);
    assert_eq!(
        life(&engine, me()),
        23,
        "CR 701.22d: Scry 3 on empty library still fires Trigger::Scried with EventAmount = 3"
    );
}

/// 3. `Monstrosity` (`CR 701.37a–c`, `CR 400.7`):
/// - `Effect::Monstrosity(Amount::X)` puts `X` `+1/+1` counters, sets `Rider::Monstrous`
///   (`Condition::IsMonstrous`, `Filter::Monstrous`), and fires `Trigger::BecomesMonstrous`
///   with `EventAmount = X`.
/// - Activating a second time while already monstrous is a no-op (`CR 701.37a`).
/// - Noncopiable (`CR 701.37b`) and resets when flickering (`CR 400.7`).
#[test]
fn wave8b_monstrosity_places_counters_sets_designation_once_fires_becomes_monstrous_with_x_and_clears_on_ltb()
 {
    let mut engine = bench(
        2,
        vec![
            monstrous_hydra(),
            monstrous_lord(),
            clone_shifter(),
            flicker_instant(),
        ],
        [
            Seat::with(&[MONSTROUS_HYDRA, MONSTROUS_LORD])
                .holding(&[CLONE_SHIFTER, FLICKER_INSTANT]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let hydra = the(&engine, ZoneLocation::Battlefield, MONSTROUS_HYDRA);
    let clone = the(&engine, ZoneLocation::Hand(me()), CLONE_SHIFTER);
    let flicker = the(&engine, ZoneLocation::Hand(me()), FLICKER_INSTANT);

    assert_eq!(
        engine.state.object(hydra).unwrap().characteristics().power,
        Some(3)
    );
    assert!(
        !engine
            .state
            .object(hydra)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );

    // 1) Activate `{X}{G}` with X = 3 -> 3 +1/+1 counters, +2/+0 from IsMonstrous (8/6),
    //    TRAMPLE from Filter::Monstrous, +3 life from Trigger::BecomesMonstrous!
    add_pool(&mut engine, me(), ManaColor::Green, 4);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: hydra,
                ability_index: 0,
            },
        )
        .expect("activate monstrosity X");
    engine
        .apply(me(), PlayerAction::ChooseNumber(3))
        .expect("choose X = 3");
    drain_stack(&mut engine);

    let hydra_obj = engine.state.object(hydra).unwrap();
    assert!(hydra_obj.riders.contains(&Rider::Monstrous));
    assert_eq!(hydra_obj.counters.get(CounterKind::P1P1), 3);
    assert_eq!(hydra_obj.characteristics().power, Some(8));
    assert_eq!(hydra_obj.characteristics().toughness, Some(6));
    assert!(
        hydra_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );
    assert_eq!(
        life(&engine, me()),
        23,
        "CR 701.37c: BecomesMonstrous read EventAmount = 3"
    );

    // 2) Activate Monstrosity again with X = 2 -> already monstrous, so 0 new counters and no trigger!
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: hydra,
                ability_index: 0,
            },
        )
        .expect("activate monstrosity second time");
    engine
        .apply(me(), PlayerAction::ChooseNumber(2))
        .expect("choose X = 2");
    drain_stack(&mut engine);
    assert_eq!(
        engine
            .state
            .object(hydra)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3
    );
    assert_eq!(life(&engine, me()), 23);

    // 3) Clone copying `hydra` does NOT copy `Monstrous` (`CR 701.37b`)!
    add_pool(&mut engine, me(), ManaColor::Blue, 4);
    engine
        .apply(me(), PlayerAction::CastSpell { card: clone })
        .expect("cast clone");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass me");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass them");
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![hydra],
                    players: vec![],
                },
            )
            .expect("copy hydra");
    }
    drain_stack(&mut engine);
    assert!(
        !engine
            .state
            .object(clone)
            .unwrap()
            .riders
            .contains(&Rider::Monstrous),
        "CR 701.37b: monstrous designation is not copiable"
    );

    // 4) Flicker `hydra` -> new incarnation is NOT monstrous and CAN become monstrous again!
    add_pool(&mut engine, me(), ManaColor::White, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: flicker })
        .expect("cast flicker");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![hydra],
                players: vec![],
            },
        )
        .expect("target hydra");
    drain_stack(&mut engine);
    assert!(
        !engine
            .state
            .object(hydra)
            .unwrap()
            .riders
            .contains(&Rider::Monstrous)
    );

    add_pool(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: hydra,
                ability_index: 0,
            },
        )
        .expect("activate monstrosity after flicker");
    engine
        .apply(me(), PlayerAction::ChooseNumber(1))
        .expect("choose X = 1");
    drain_stack(&mut engine);
    assert!(
        engine
            .state
            .object(hydra)
            .unwrap()
            .riders
            .contains(&Rider::Monstrous)
    );
    assert_eq!(life(&engine, me()), 24);
}

/// 4. `Suspect` (`CR 701.60a–d`):
/// - `Effect::Suspect` marks target creature `Rider::Suspected`, granting `MENACE`
///   and `CANT_BLOCK` in layer 6 and matching `Filter::Suspected` / `Condition::IsSuspected`.
/// - `Effect::Unsuspect` clears `Rider::Suspected`, restoring ability to block.
/// - `EnterModifier::Suspected` enters suspected immediately; noncopiable (`CR 701.60b`).
#[test]
fn wave8b_suspect_grants_menace_and_cant_block_unsuspects_and_supports_enter_modifier() {
    let mut engine = bench(
        2,
        vec![
            suspect_spell(),
            unsuspect_spell(),
            born_suspected(),
            vanilla_bear(),
            suspect_watcher(),
            clone_shifter(),
        ],
        [
            Seat::with(&[VANILLA_BEAR, SUSPECT_WATCHER]).holding(&[
                SUSPECT_SPELL,
                UNSUSPECT_SPELL,
                BORN_SUSPECTED,
                CLONE_SHIFTER,
            ]),
            Seat::with(&[VANILLA_BEAR]),
        ],
    );
    to_main(&mut engine, me());

    let my_bear = engine
        .state
        .battlefield_seen()
        .find(|&id| {
            let o = engine.state.object(id).unwrap();
            o.controller == me() && o.card.is_some_and(|c| c.index.get() == VANILLA_BEAR)
        })
        .unwrap();
    let opp_bear = engine
        .state
        .battlefield_seen()
        .find(|&id| {
            let o = engine.state.object(id).unwrap();
            o.controller == them() && o.card.is_some_and(|c| c.index.get() == VANILLA_BEAR)
        })
        .unwrap();
    let suspect_card = the(&engine, ZoneLocation::Hand(me()), SUSPECT_SPELL);
    let unsuspect_card = the(&engine, ZoneLocation::Hand(me()), UNSUSPECT_SPELL);
    let born = the(&engine, ZoneLocation::Hand(me()), BORN_SUSPECTED);
    let clone = the(&engine, ZoneLocation::Hand(me()), CLONE_SHIFTER);

    // 1) Suspect `opp_bear` -> gains MENACE + CANT_BLOCK + +1/+0 from `Filter::Suspected`!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: suspect_card })
        .expect("cast suspect_spell");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .expect("target opp_bear");
    drain_stack(&mut engine);

    let opp_obj = engine.state.object(opp_bear).unwrap();
    assert!(opp_obj.riders.contains(&Rider::Suspected));
    assert!(
        opp_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::MENACE)
    );
    assert!(
        opp_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::CANT_BLOCK)
    );
    assert_eq!(opp_obj.characteristics().power, Some(3));
    assert!(
        !crate::combat::can_block(&engine.state, them(), opp_bear, my_bear),
        "CR 701.60c: suspected creature cannot block"
    );

    // 2) Cast `BORN_SUSPECTED` (`EnterModifier::Suspected` + `Condition::IsSuspected`)!
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: born })
        .expect("cast born_suspected");
    drain_stack(&mut engine);
    let born_obj = engine.state.object(born).unwrap();
    assert!(born_obj.riders.contains(&Rider::Suspected));
    assert_eq!(
        born_obj.characteristics().power,
        Some(4),
        "2 base + 1 Condition::IsSuspected + 1 Filter::Suspected"
    );

    // 3) Clone copying `born` is NOT suspected (`CR 701.60b`), unless its copied EnterModifier::Suspected applies!
    //    Copy `opp_bear` (vanilla bear that became suspected via spell) -> Clone is NOT suspected!
    add_pool(&mut engine, me(), ManaColor::Blue, 4);
    engine
        .apply(me(), PlayerAction::CastSpell { card: clone })
        .expect("cast clone");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass me");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass them");
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![opp_bear],
                    players: vec![],
                },
            )
            .expect("copy opp_bear");
    }
    drain_stack(&mut engine);
    assert!(
        !engine
            .state
            .object(clone)
            .unwrap()
            .riders
            .contains(&Rider::Suspected),
        "CR 701.60b: suspected designation is not copiable"
    );

    // 4) Unsuspect `opp_bear` -> loses Suspected, MENACE, and CANT_BLOCK, so it can block again!
    add_pool(&mut engine, me(), ManaColor::White, 1);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: unsuspect_card,
            },
        )
        .expect("cast unsuspect_spell");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .expect("target opp_bear");
    drain_stack(&mut engine);
    let opp_obj = engine.state.object(opp_bear).unwrap();
    assert!(!opp_obj.riders.contains(&Rider::Suspected));
    assert!(
        !opp_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::MENACE)
    );
    assert!(
        !opp_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::CANT_BLOCK)
    );
    assert!(crate::combat::can_block(
        &engine.state,
        them(),
        opp_bear,
        my_bear
    ));
}

/// 5. `Riot` (`CR 702.136a–b`) & `Unleash` (`CR 702.98a`):
/// - Riot prompts `YesNoPrompt::Riot` (`true` -> `+1/+1` counter; `false` -> `Rider::RiotHaste` + `HASTE`);
///   with `RIOT_ANTHEM` (`2` instances of Riot per `CR 702.136b`), prompts twice so choosing
///   `true` then `false` grants BOTH `+1/+1` counter AND `HASTE`!
/// - Unleash prompts `YesNoPrompt::Unleash` (`true` -> `+1/+1` counter, `CANT_BLOCK`);
///   annihilating the `+1/+1` counter with `-1/-1` removes `CANT_BLOCK`, while placing a
///   `+1/+1` counter externally on an unleashed-declined devil grants `CANT_BLOCK`!
#[test]
fn wave8b_riot_and_unleash_prompt_on_entry_stack_multiple_riot_instances_and_enforce_unleash_cant_block()
 {
    let mut engine = bench(
        2,
        vec![
            riot_warrior(),
            riot_anthem(),
            unleash_devil(),
            grow_instant(),
            shrink_counter_instant(),
            vanilla_bear(),
        ],
        [
            Seat::with(&[]).holding(&[
                RIOT_WARRIOR,
                RIOT_ANTHEM,
                RIOT_WARRIOR,
                UNLEASH_DEVIL,
                UNLEASH_DEVIL,
                GROW_INSTANT,
                SHRINK_COUNTER_INSTANT,
            ]),
            Seat::with(&[VANILLA_BEAR]),
        ],
    );
    to_main(&mut engine, me());

    let opp_bear = the(&engine, ZoneLocation::Battlefield, VANILLA_BEAR);
    let warriors = objects(&engine, ZoneLocation::Hand(me()), RIOT_WARRIOR);
    let riot_one = warriors[0];
    let riot_two = warriors[1];
    let anthem = the(&engine, ZoneLocation::Hand(me()), RIOT_ANTHEM);
    let devils = objects(&engine, ZoneLocation::Hand(me()), UNLEASH_DEVIL);
    let devil_unleashed = devils[0];
    let devil_calm = devils[1];
    let grow = the(&engine, ZoneLocation::Hand(me()), GROW_INSTANT);
    let shrink = the(&engine, ZoneLocation::Hand(me()), SHRINK_COUNTER_INSTANT);

    // 1) Cast `riot_one` (1 Riot instance), choose `false` (Haste) -> 2/2 with Haste!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: riot_one })
        .expect("cast riot_one");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass me");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass them");
    match engine.pending().clone() {
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::Riot { permanent },
            ..
        } => {
            assert_eq!(player, me());
            assert_eq!(permanent, riot_one);
            engine
                .apply(me(), PlayerAction::YesNo(false))
                .expect("choose haste for riot_one");
        }
        other => panic!("expected YesNoPrompt::Riot, got {other:?}"),
    }
    drain_stack(&mut engine);
    let r1 = engine.state.object(riot_one).unwrap();
    assert_eq!(r1.counters.get(CounterKind::P1P1), 0);
    assert!(r1.riders.contains(&Rider::RiotHaste));
    assert!(r1.characteristics().keywords.contains(KeywordSet::HASTE));

    // 2) Cast `RIOT_ANTHEM`, then `riot_two` (2 Riot instances!) -> answer `true` (+1/+1) then `false` (Haste)!
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: anthem })
        .expect("cast riot_anthem");
    drain_stack(&mut engine);

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: riot_two })
        .expect("cast riot_two");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass me");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass them");
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Riot { .. },
            ..
        }
    ));
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("first Riot instance -> +1/+1 counter");
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Riot { .. },
                ..
            }
        ),
        "CR 702.136b: second Riot instance prompts separately"
    );
    engine
        .apply(me(), PlayerAction::YesNo(false))
        .expect("second Riot instance -> haste");
    drain_stack(&mut engine);
    let r2 = engine.state.object(riot_two).unwrap();
    assert_eq!(r2.counters.get(CounterKind::P1P1), 1);
    assert!(r2.characteristics().keywords.contains(KeywordSet::HASTE));

    // 3) Cast `devil_unleashed` (choose `true`) -> enters with 1 +1/+1 counter and CANT_BLOCK!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: devil_unleashed,
            },
        )
        .expect("cast devil_unleashed");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass me");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass them");
    // Note: `RIOT_ANTHEM` is on the battlefield too, so `devil_unleashed` gets Riot first or Unleash — decline Riot (`false`) and accept Unleash (`true`)!
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::YesNo {
                prompt: YesNoPrompt::Riot { .. },
                ..
            } => {
                engine
                    .apply(me(), PlayerAction::YesNo(false))
                    .expect("decline riot counter");
            }
            Pending::YesNo {
                prompt: YesNoPrompt::Unleash { .. },
                ..
            } => {
                engine
                    .apply(me(), PlayerAction::YesNo(true))
                    .expect("accept unleash counter");
            }
            other => panic!("expected Riot/Unleash prompt, got {other:?}"),
        }
    }
    drain_stack(&mut engine);
    assert_eq!(
        engine
            .state
            .object(devil_unleashed)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert!(!crate::combat::can_block(
        &engine.state,
        me(),
        devil_unleashed,
        opp_bear
    ));

    // Put a -1/-1 counter on `devil_unleashed` -> SBA annihilates +1/+1 & -1/-1, so it CAN block again!
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: shrink })
        .expect("cast shrink");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![devil_unleashed],
                players: vec![],
            },
        )
        .expect("target devil_unleashed");
    drain_stack(&mut engine);
    assert_eq!(
        engine
            .state
            .object(devil_unleashed)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert!(crate::combat::can_block(
        &engine.state,
        me(),
        devil_unleashed,
        opp_bear
    ));

    // 4) Cast `devil_calm` declining Unleash (`false`) -> can block; then `grow` puts +1/+1 counter -> CANT_BLOCK!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: devil_calm })
        .expect("cast devil_calm");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass me");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass them");
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::YesNo {
                prompt: YesNoPrompt::Riot { .. } | YesNoPrompt::Unleash { .. },
                ..
            } => {
                engine
                    .apply(me(), PlayerAction::YesNo(false))
                    .expect("decline counter");
            }
            other => panic!("expected Riot/Unleash prompt, got {other:?}"),
        }
    }
    drain_stack(&mut engine);
    assert!(crate::combat::can_block(
        &engine.state,
        me(),
        devil_calm,
        opp_bear
    ));

    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: grow })
        .expect("cast grow");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![devil_calm],
                players: vec![],
            },
        )
        .expect("target devil_calm");
    drain_stack(&mut engine);
    assert!(
        !crate::combat::can_block(&engine.state, me(), devil_calm, opp_bear),
        "CR 702.98a: Unleash permanent cannot block whenever it has any +1/+1 counter"
    );
}
