//! Independent Personal Incarnation review: CR 107.1b, 400.7, 602.2,
//! 608.2h, and 614.9. Oracle/rulings: saved 2026-10-02 Scryfall data.

#[allow(clippy::wildcard_imports)] // Shared behavioral card-test vocabulary.
use super::*;
use baylee_core::generated::index;

const OWNER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: holder, .. } if *holder == player),
    );
}

fn setup(life: i32) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .life(0, life)
        .battlefield(0, &[index::PERSONAL_INCARNATION, plains(), plains()])
        .hand(0, &[ephemerate(), index::HEALING_SALVE])
        .battlefield(
            1,
            &[
                island(),
                island(),
                island(),
                island(),
                swamp(),
                plains(),
                mountain(),
                mountain(),
            ],
        )
        .hand(
            1,
            &[
                index::CONTROL_MAGIC,
                index::TERROR,
                swords_to_plowshares(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let creature = on_battlefield(&engine, OWNER, index::PERSONAL_INCARNATION).unwrap();
    (engine, creature)
}

fn activate_owner(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    priority(engine, OWNER);
    engine
        .apply(
            OWNER,
            PlayerAction::ActivateAbility {
                source: creature,
                ability_index: 0,
            },
        )
        .unwrap();
}

fn steal(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    reach_their_main_phase(engine, OTHER);
    // Pay the Aura entirely with Islands; reserve the Plains for Swords.
    for source in all_on_battlefield(engine, OTHER, island()) {
        engine
            .apply(OTHER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(engine, OTHER, index::CONTROL_MAGIC);
    aim(engine, vec![creature], vec![]);
    pass_until(engine, stack_is_empty);
    assert_eq!(engine.state().object(creature).unwrap().controller, OTHER);
    assert_eq!(engine.state().object(creature).unwrap().owner, OWNER);
}

fn bolt(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    priority(engine, OTHER);
    cast_from_hand(engine, OTHER, lightning_bolt());
    aim(engine, vec![creature], vec![]);
    for _ in 0..40 {
        if stack_is_empty(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } => {
                assert_eq!(
                    player,
                    engine.state().object(creature).unwrap().controller,
                    "the damaged creature's controller chooses, even when its owner created the shield"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseDamageEffect {
                            choice,
                            effect: options[0].id,
                        },
                    )
                    .unwrap();
            }
            Pending::AllocatePrevention {
                player,
                choice,
                damage,
                total,
                ..
            } => {
                assert_eq!(
                    player,
                    engine.state().object(creature).unwrap().controller,
                    "the affected creature's controller allocates the redirected point"
                );
                assert_eq!(
                    total, 1,
                    "each Personal Incarnation activation redirects exactly one"
                );
                assert_eq!(damage.len(), 1, "one real Bolt damage event");
                engine
                    .apply(
                        player,
                        PlayerAction::AllocatePrevention {
                            choice,
                            allocation: vec![(damage[0].id, 1)],
                        },
                    )
                    .unwrap();
            }
            pending => panic!("unexpected Personal Incarnation damage choice: {pending:?}"),
        }
    }
    panic!("Bolt did not finish");
}

fn terror_to_trigger(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    priority(engine, OTHER);
    cast_from_hand(engine, OTHER, index::TERROR);
    aim(engine, vec![creature], vec![]);
    pass_until(engine, |e| {
        e.state().object(creature).unwrap().zone == Zone::Graveyard
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

#[test]
fn personal_review_casts_its_printed_body_and_offers_its_owner_ability() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 6])
        .hand(0, &[index::PERSONAL_INCARNATION])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    cast_from_hand(&mut engine, OWNER, index::PERSONAL_INCARNATION);
    pass_until(&mut engine, stack_is_empty);
    let creature = on_battlefield(&engine, OWNER, index::PERSONAL_INCARNATION).unwrap();
    assert_eq!(pt(&engine, creature), (6, 6));
    assert!(!keywords(&engine, creature).contains(KeywordSet::FLYING));
    priority(&mut engine, OWNER);
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!("owner has priority");
    };
    assert!(
        legal.abilities.contains(&(creature, 0)),
        "the completed card offers its owner-only redirection ability"
    );
}

#[test]
fn personal_review_stolen_creature_owner_can_activate_and_controller_cannot() {
    let (mut engine, creature) = setup(20);
    steal(&mut engine, creature);
    priority(&mut engine, OTHER);
    let hash = engine.fingerprint();
    let journal = engine.state().journal.len();
    let pending = serde_json::to_value(engine.pending()).unwrap();
    assert!(
        engine
            .apply(
                OTHER,
                PlayerAction::ActivateAbility {
                    source: creature,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(engine.fingerprint(), hash);
    assert_eq!(engine.state().journal.len(), journal);
    assert_eq!(serde_json::to_value(engine.pending()).unwrap(), pending);
    activate_owner(&mut engine, creature);
    pass_until(&mut engine, stack_is_empty);
    bolt(&mut engine, creature);
    assert_eq!(engine.state().object(creature).unwrap().damage, 2);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the owner receives the redirected point"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn personal_review_each_activation_redirects_only_one_point() {
    for shields in [1, 2] {
        let (mut engine, creature) = setup(20);
        for _ in 0..shields {
            activate_owner(&mut engine, creature);
        }
        pass_until(&mut engine, stack_is_empty);
        bolt(&mut engine, creature);
        assert_eq!(engine.state().object(creature).unwrap().damage, 3 - shields);
        assert_eq!(engine.state().players[0].life, 20 - i32::from(shields));
        if shields == 1 {
            bolt(&mut engine, creature);
            assert_eq!(
                engine.state().object(creature).unwrap().damage,
                5,
                "the spent shield cannot redirect a later Bolt"
            );
            assert_eq!(engine.state().players[0].life, 19);
        }
        assert_eq!(engine.state().players[1].life, 20);
    }
}

#[test]
fn personal_review_waiting_or_resolved_shield_does_not_follow_real_blink() {
    for resolve_first in [false, true] {
        let (mut engine, creature) = setup(20);
        activate_owner(&mut engine, creature);
        if resolve_first {
            pass_until(&mut engine, stack_is_empty);
        }
        priority(&mut engine, OWNER);
        let old = engine.state().object(creature).unwrap().version;
        cast_from_hand(&mut engine, OWNER, ephemerate());
        aim(&mut engine, vec![creature], vec![]);
        pass_until(&mut engine, stack_is_empty);
        assert_ne!(engine.state().object(creature).unwrap().version, old);
        bolt(&mut engine, creature);
        assert_eq!(engine.state().object(creature).unwrap().damage, 3);
        assert_eq!(engine.state().players[0].life, 20);
    }
}

#[test]
fn personal_review_unused_shield_expires_at_cleanup() {
    let (mut engine, creature) = setup(20);
    activate_owner(&mut engine, creature);
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, OTHER);
    bolt(&mut engine, creature);
    assert_eq!(engine.state().object(creature).unwrap().damage, 3);
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn personal_review_death_loses_half_owner_life_rounded_up() {
    for (before, after) in [(20, 10), (21, 10), (1, 0)] {
        let (mut engine, creature) = setup(before);
        terror_to_trigger(&mut engine, creature);
        assert_eq!(
            engine.state().players[0].life,
            before,
            "loss waits for the trigger to resolve"
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[0].life, after);
        assert_eq!(engine.state().players[1].life, 20);
        if after == 0 {
            assert!(
                engine.state().has_left(OWNER),
                "the owner loses at zero life"
            );
            assert!(
                engine.state().object(creature).is_none(),
                "the eliminated owner's objects leave the game"
            );
        } else {
            assert_eq!(
                engine.state().object(creature).unwrap().zone,
                Zone::Graveyard
            );
        }
    }
}

#[test]
fn personal_review_stolen_creature_death_still_charges_its_owner() {
    let (mut engine, creature) = setup(21);
    steal(&mut engine, creature);
    terror_to_trigger(&mut engine, creature);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 10);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn personal_review_death_trigger_reads_owner_life_after_real_healing_salve() {
    let (mut engine, creature) = setup(21);
    terror_to_trigger(&mut engine, creature);
    priority(&mut engine, OWNER);
    cast_from_hand(&mut engine, OWNER, index::HEALING_SALVE);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("Healing Salve mode must be chosen");
    };
    let gain_life = options
        .iter()
        .position(|option| option.kind == CastModeKind::Mode(0))
        .unwrap();
    engine
        .apply(OWNER, PlayerAction::ChooseMode(gain_life))
        .unwrap();
    engine
        .apply(OWNER, PlayerAction::ChoosePlayer(OWNER))
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().players[0].life == 24 && matches!(e.pending(), Pending::Priority { .. })
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        12,
        "half the current 24, not the old 21"
    );
}

#[test]
fn personal_review_negative_owner_life_is_not_halved_or_raised() {
    let (mut engine, creature) = setup(20);
    // Isolate CR 107.1b arithmetic. This fixture prevents losing the game,
    // but permits losing life; Everybody Lives would mask the instruction.
    let timestamp = engine.state.next_timestamp();
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: OWNER,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Ability,
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
            modifier: baylee_cards_dsl::Modifier::PlayersCantLose,
        });
    engine.state.players[0].life = -10;
    engine.state.refresh_characteristics();
    terror_to_trigger(&mut engine, creature);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        -10,
        "negative calculated loss is zero"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn personal_review_swords_exiles_without_death_loss_and_pays_the_controller() {
    for stolen in [false, true] {
        let (mut engine, creature) = setup(20);
        if stolen {
            steal(&mut engine, creature);
        }
        priority(&mut engine, OTHER);
        cast_from_hand(&mut engine, OTHER, swords_to_plowshares());
        aim(&mut engine, vec![creature], vec![]);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().object(creature).unwrap().zone, Zone::Exile);
        assert_eq!(engine.state().players[0].life, if stolen { 20 } else { 26 });
        assert_eq!(engine.state().players[1].life, if stolen { 26 } else { 20 });
        assert!(in_graveyard(&engine, OWNER, index::PERSONAL_INCARNATION).is_none());
    }
}
