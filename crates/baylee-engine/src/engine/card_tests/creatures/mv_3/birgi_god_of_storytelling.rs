//! `cards/creatures/mv_3/birgi_god_of_storytelling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The front adds red mana for a spell; only the back has the discard ability.
#[test]
fn birgi_adds_red_mana_on_spell_cast_and_does_not_have_the_back_faces_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[birgi_god_of_storytelling(), forest(), forest()])
        .hand(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let birgi =
        on_battlefield(&engine, p0, birgi_god_of_storytelling()).expect("birgi on battlefield");
    assert_eq!(pt(&engine, birgi), (3, 3));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0
    );

    let my_forests = all_on_battlefield(&engine, p0, forest());
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: my_forests[0],
            },
        )
        .unwrap();

    cast_with_floating(&mut engine, p0, young_wolf());

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_some(),
        "young wolf resolved"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Birgi added {{R}} on spell cast"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == birgi),
        "the front face has no activated ability"
    );
}

#[test]
fn birgi_mana_survives_phases_and_source_departure_but_ends_in_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[birgi_god_of_storytelling(), forest()])
        .hand(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, young_wolf());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Stack).len(),
        2,
        "Birgi's trigger uses the stack"
    );
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 1);
    let birgi = on_battlefield(&e, p0, birgi_god_of_storytelling()).unwrap();
    bury(&mut e, &[birgi]);
    e.state.players[0].mana_pool.add(ManaColor::Green, 2);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Green),
        0
    );
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 1);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn harnfel_discards_as_cost_exiles_two_and_requires_normal_spell_payment() {
    let p0 = PlayerId::new(0);
    let mut e = harnfel_table();
    let fodder = in_hand(&e, p0, forest()).unwrap();
    let library = e.state().zones.list(ZoneLocation::Library(p0)).clone();
    activate(&mut e, p0, birgi_god_of_storytelling(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![fodder],
        },
    )
    .unwrap();
    assert_eq!(e.state().object(fodder).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Exile(p0)).len(),
        0,
        "discard precedes resolution"
    );
    pass_until(&mut e, stack_is_empty);
    let exiled = e.state().zones.list(ZoneLocation::Exile(p0)).clone();
    assert_eq!(exiled.len(), 2);
    assert!(
        exiled
            .iter()
            .all(|id| library[library.len() - 2..].contains(id))
    );
    assert!(
        exiled
            .iter()
            .all(|id| !priority_offer(&e).castable.contains(id))
    );
    e.state.players[0].mana_pool.add(ManaColor::Green, 2);
    e.refresh_offer();
    assert!(
        exiled
            .iter()
            .all(|id| priority_offer(&e).castable.contains(id))
    );
    cast_object_and_resolve(&mut e, p0, exiled[0]);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        0,
        "only the front produces spell mana"
    );
    reach_their_main_phase(&mut e, PlayerId::new(1));
    assert!(crate::casting::play_permission(e.state(), p0, exiled[1]).is_none());
}

fn give_boast(e: &mut Engine<RegistryLookup>, creature: ObjectId) {
    use baylee_cards_dsl::{ActivationLimit, Effect, activated, cost};
    static ABILITIES: &[AbilityDef] = &[activated!(
        cost!("{0}"),
        &[Effect::gain_life(1)],
        limit = ActivationLimit::Boast
    )];
    e.state
        .object_mut(creature)
        .unwrap()
        .take_abilities(crate::object::AbilityList::from_static(
            ABILITIES, None, None,
        ));
    e.refresh_offer();
}

#[test]
fn birgi_allows_two_boasts_only_after_attacking_and_the_second_depends_on_her_current_presence() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for remove_birgi in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[birgi_god_of_storytelling(), grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        give_boast(&mut e, bear);
        assert!(!priority_offer(&e).abilities.contains(&(bear, 0)));
        attack_and_collect_blocks(&mut e, bear, p1);
        pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
        let life = e.state().players[0].life;
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: 0,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, life + 1);
        if remove_birgi {
            let birgi = on_battlefield(&e, p0, birgi_god_of_storytelling()).unwrap();
            bury(&mut e, &[birgi]);
            let Pending::Priority { player, .. } = e.pending() else {
                panic!("priority")
            };
            e.apply(*player, PlayerAction::PassPriority).unwrap();
            e.apply(p1, PlayerAction::PassPriority).unwrap();
            assert!(!priority_offer(&e).abilities.contains(&(bear, 0)));
        } else {
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: bear,
                    ability_index: 0,
                },
            )
            .unwrap();
            assert!(
                !priority_offer(&e).abilities.contains(&(bear, 0)),
                "announcing spends the second boast before resolution"
            );
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: bear,
                        ability_index: 0
                    }
                )
                .is_err()
            );
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().players[0].life, life + 2);
        }
        reach_their_main_phase(&mut e, p1);
        reach_their_main_phase(&mut e, p0);
        assert!(
            !priority_offer(&e).abilities.contains(&(bear, 0)),
            "a new turn needs a new attack"
        );
    }
}

#[test]
fn birgi_does_not_double_boast_on_an_opponents_turn_after_control_changes() {
    for used_before_transfer in [false, true] {
        let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[birgi_god_of_storytelling()])
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        reach_their_main_phase(&mut e, p1);
        let bear = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        give_boast(&mut e, bear);
        attack_and_collect_blocks(&mut e, bear, p0);
        pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
        if used_before_transfer {
            e.apply(
                p1,
                PlayerAction::ActivateAbility {
                    source: bear,
                    ability_index: 0,
                },
            )
            .unwrap();
            pass_until(&mut e, stack_is_empty);
        }
        e.state.object_mut(bear).unwrap().set_controller(p0);
        e.apply(p1, PlayerAction::PassPriority).unwrap();
        if used_before_transfer {
            assert!(
                !priority_offer(&e).abilities.contains(&(bear, 0)),
                "control changes do not reset the activation count"
            );
            continue;
        }
        assert!(
            priority_offer(&e).abilities.contains(&(bear, 0)),
            "the exact object attacked before changing controller"
        );
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bear,
                ability_index: 0,
            },
        )
        .unwrap();
        assert!(
            !priority_offer(&e).abilities.contains(&(bear, 0)),
            "Birgi only doubles during your turns"
        );
    }
}

fn harnfel_table() -> Engine<RegistryLookup> {
    use crate::choice::CastModeKind;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, grizzly_bears())
        .hand(0, &[birgi_god_of_storytelling(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let harnfel = in_hand(&e, p0, birgi_god_of_storytelling()).unwrap();
    e.state.players[0].mana_pool.add(ManaColor::Red, 5);
    e.refresh_offer();
    e.apply(p0, PlayerAction::CastSpell { card: harnfel })
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    let mode = options
        .iter()
        .position(|o| o.kind == CastModeKind::Face(1))
        .unwrap();
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(harnfel).unwrap().face_index, 1);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        0,
        "Harnfel has no Birgi trigger"
    );
    e
}

#[test]
fn harnfel_uses_the_available_library_cards_without_drawing_from_an_empty_library() {
    let p0 = PlayerId::new(0);
    for remaining in [0, 1] {
        let mut e = harnfel_table();
        let library = e.state().zones.list(ZoneLocation::Library(p0)).clone();
        for card in &library[..library.len() - remaining] {
            e.state
                .move_object(
                    *card,
                    ZoneLocation::Graveyard(p0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        e.refresh_offer();
        let fodder = in_hand(&e, p0, forest()).unwrap();
        activate(&mut e, p0, birgi_god_of_storytelling(), 0);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().zones.list(ZoneLocation::Exile(p0)).len(),
            remaining
        );
        assert!(!e.state().players[0].tried_empty_draw);
        assert!(!e.state().players[0].has_lost());
    }
}
