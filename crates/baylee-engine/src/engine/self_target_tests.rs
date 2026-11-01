//! Explicit self-targets obey targeting restrictions; implicit self-effects do not.

use super::synthetic::{self, SyntheticLookup};
use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use baylee_cards_dsl::{AbilityDef, Amount, Effect, Modifier, PlayerRel, TargetReq, TargetSpec};

const SELF: TargetSpec = TargetSpec::Player(PlayerRel::You);
const INDEX: u32 = 98_011;

fn probe() -> Engine<SyntheticLookup> {
    static ABILITIES: &[AbilityDef] = &[
        baylee_cards_dsl::activated!(
            baylee_cards_dsl::cost!("", TapSelf),
            &[Effect::GainLife {
                amount: Amount::Fixed(3)
            }],
            targets = Some(TargetReq::one(SELF))
        ),
        baylee_cards_dsl::activated!(
            baylee_cards_dsl::cost!("", TapSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: SELF
            }]
        ),
    ];
    let definition = synthetic::land(INDEX, "Self target probe", ABILITIES);
    let mut engine = Engine::new(
        &synthetic::preset(81, &[INDEX]),
        SyntheticLookup::new(vec![definition]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if player.get() == 0),
    );
    engine
}

fn walk_until(
    engine: &mut Engine<SyntheticLookup>,
    ready: impl Fn(&Engine<SyntheticLookup>) -> bool,
) {
    for _ in 0..50 {
        if ready(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected choice: {pending:?}"
        );
    }
    panic!("condition not reached: {:?}", engine.pending());
}

fn protect(engine: &mut Engine<SyntheticLookup>, modifier: Modifier) {
    use baylee_cards_dsl::{Duration, Filter, Layer};
    let timestamp = engine.state.next_timestamp();
    engine.state.effects.register(ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: PlayerId::new(0),
        origin: EffectOrigin::Resolution,
        layer: Layer::Text,
        timestamp,
        duration: Duration::Indefinitely,
        filter: EffectFilter::Dsl(&Filter::Any),
        modifier,
    });
    engine.refresh_offer();
}

fn offered(engine: &Engine<SyntheticLookup>, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    legal
        .abilities
        .contains(&(synthetic::permanents(engine, INDEX)[0], index))
}

fn activate(engine: &mut Engine<SyntheticLookup>, index: u32) {
    assert!(offered(engine, index), "the ability must be offered");
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateAbility {
                source: synthetic::permanents(engine, INDEX)[0],
                ability_index: index,
            },
        )
        .unwrap();
}

fn choose_self(engine: &mut Engine<SyntheticLookup>) {
    let Pending::ChooseTargets {
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending()
    else {
        panic!("expected self target choice, got {:?}", engine.pending());
    };
    assert!(options.is_empty());
    assert_eq!(player_options, &[PlayerId::new(0)]);
    assert_eq!((*min, *max), (1, 1));
    assert!(
        engine
            .apply(
                PlayerId::new(0),
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![PlayerId::new(1)],
                }
            )
            .is_err(),
        "an opponent cannot satisfy a self-target"
    );
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![PlayerId::new(0)],
            },
        )
        .unwrap();
}

#[test]
fn explicit_self_target_is_offered_and_resolves_through_own_hexproof() {
    let mut engine = probe();
    protect(
        &mut engine,
        Modifier::PlayerHexproof {
            who: PlayerRel::You,
        },
    );
    activate(&mut engine, 0);
    choose_self(&mut engine);
    walk_until(&mut engine, |e| e.state.zones.stack_is_empty());
    assert_eq!(engine.state.players[0].life, 23);
    assert_eq!(engine.state.players[1].life, 20);
}

#[test]
fn protection_withholds_explicit_self_target_from_ability_and_spell_offers() {
    let mut engine = probe();
    protect(
        &mut engine,
        Modifier::PlayerProtectionFromEverything {
            who: PlayerRel::You,
        },
    );
    assert!(!offered(&engine, 0));
    assert!(!crate::casting::requirement_is_reachable_for(
        Some(TargetReq::one(SELF)),
        &engine.state,
        PlayerId::new(0),
        synthetic::permanents(&engine, INDEX)[0],
        None,
        false,
    ));
}

#[test]
fn filtered_player_protection_reads_the_granting_sources_chosen_card_type() {
    for (chosen, targetable) in [
        (baylee_core::types::TypeSet::LAND, false),
        (baylee_core::types::TypeSet::INSTANT, true),
    ] {
        let mut engine = probe();
        let source = synthetic::permanents(&engine, INDEX)[0];
        engine
            .state
            .object_mut(source)
            .expect("the synthetic source is on the battlefield")
            .set_chosen_card_type(chosen);
        protect(
            &mut engine,
            Modifier::PlayerProtectionFrom {
                who: PlayerRel::You,
                from: &baylee_cards_dsl::Filter::OfSourceChosenCardType,
            },
        );
        assert_eq!(
            offered(&engine, 0),
            targetable,
            "only a source of the chosen type is protected against"
        );
    }
}

#[test]
fn explicit_self_target_is_rechecked_on_resolution() {
    let mut engine = probe();
    activate(&mut engine, 0);
    choose_self(&mut engine);
    protect(
        &mut engine,
        Modifier::PlayerProtectionFromEverything {
            who: PlayerRel::You,
        },
    );
    walk_until(&mut engine, |e| e.state.zones.stack_is_empty());
    assert_eq!(
        engine.state.players[0].life, 20,
        "illegal self-target must not gain life"
    );
}

#[test]
fn implicit_self_effect_needs_no_target_choice_despite_player_protection() {
    for (modifier, life) in [
        (
            Modifier::PlayerHexproof {
                who: PlayerRel::You,
            },
            17,
        ),
        (
            Modifier::PlayerProtectionFromEverything {
                who: PlayerRel::You,
            },
            20,
        ),
    ] {
        let mut engine = probe();
        protect(&mut engine, modifier);
        activate(&mut engine, 1);
        assert!(matches!(engine.pending(), Pending::Priority { .. }));
        walk_until(&mut engine, |e| e.state.zones.stack_is_empty());
        assert_eq!(engine.state.players[0].life, life);
    }
}
