//! Animate Artifact: actual Aura casts, combat, mana, and attachment lifetime.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::ColorSet;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn setup(card: CardIndex, seat: u8) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(1120, forest())
        .battlefield(usize::from(seat), &[card])
        .hand(
            0,
            &[
                animate_artifact(),
                animate_artifact(),
                index::DISENCHANT,
                index::GIANT_GROWTH,
                sol_ring(),
                copy_artifact(),
                index::SWIFT_RECONFIGURATION,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let object = on_battlefield(&e, PlayerId::new(seat), card).unwrap();
    (e, object)
}

fn cast(e: &mut Engine<RegistryLookup>, card: CardIndex, target: Option<ObjectId>) -> ObjectId {
    pass_until(e, |e| at_rest(e, P0));
    for color in [ManaColor::Blue, ManaColor::White, ManaColor::Green] {
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(color, 4);
    }
    e.refresh_offer();
    let source = in_hand(e, P0, card).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: source })
        .unwrap();
    if let Some(target) = target {
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("target choice")
        };
        assert!(options.contains(&target));
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    }
    pass_until(e, stack_is_empty);
    source
}

fn animate(e: &mut Engine<RegistryLookup>, target: ObjectId) -> ObjectId {
    cast(e, animate_artifact(), Some(target))
}

fn is_creature(e: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    e.state()
        .object(object)
        .unwrap()
        .characteristics()
        .types
        .contains(TypeSet::CREATURE)
}

#[test]
fn animate_artifact_sets_mana_value_and_keeps_old_mana_ability() {
    let (mut e, ring) = setup(sol_ring(), 0);
    animate(&mut e, ring);
    assert!(is_creature(&e, ring));
    assert_eq!(pt(&e, ring), (1, 1));
    assert_eq!(
        e.state().object(ring).unwrap().characteristics().colors,
        ColorSet::EMPTY
    );
    pass_until(&mut e, |e| at_rest(e, P0));
    assert!(priority_offer(&e).abilities.contains(&(ring, 0)));
    let before = e.state().players[0]
        .mana_pool
        .available(ManaColor::Colorless);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: ring,
            ability_index: 0,
        },
    )
    .unwrap();
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        before + 2
    );
    assert!(is_tapped(&e, ring));
    assert!(stack_is_empty(&e));
}

#[test]
fn animate_artifact_can_enchant_an_opponents_artifact() {
    let (mut e, ring) = setup(sol_ring(), 1);
    let aura = animate(&mut e, ring);
    assert_eq!(pt(&e, ring), (1, 1));
    assert_eq!(e.state().object(ring).unwrap().controller, P1);
    assert_eq!(e.state().object(aura).unwrap().attached_to, Some(ring));
}

#[test]
fn animate_artifact_leaves_printed_artifact_creature_power_unchanged() {
    let (mut e, juggernaut) = setup(index::JUGGERNAUT, 0);
    animate(&mut e, juggernaut);
    assert_eq!(
        pt(&e, juggernaut),
        (5, 3),
        "mana value is four, but already a creature"
    );
}

#[test]
fn animate_artifact_zero_mana_value_dies_and_its_aura_follows() {
    for card in [index::BLACK_LOTUS, index::DARKSTEEL_CITADEL] {
        let (mut e, artifact) = setup(card, 0);
        let aura = animate(&mut e, artifact);
        assert_eq!(e.state().object(artifact).unwrap().zone, Zone::Graveyard);
        assert_eq!(e.state().object(aura).unwrap().zone, Zone::Graveyard);
    }
}

#[test]
fn animate_artifact_keeps_land_type_and_indestructible_with_a_counter() {
    let (mut e, land) = setup(index::DARKSTEEL_CITADEL, 0);
    crate::replacement::put_counters(e.dev_state_mut(P0).unwrap(), land, CounterKind::P1P1, 1);
    animate(&mut e, land);
    let c = e.state().object(land).unwrap().characteristics();
    assert!(
        c.types.contains(
            TypeSet::ARTIFACT
                .union(TypeSet::LAND)
                .union(TypeSet::CREATURE)
        )
    );
    assert!(c.keywords.contains(KeywordSet::INDESTRUCTIBLE));
    assert_eq!(pt(&e, land), (1, 1));
}

#[test]
fn animate_artifact_base_power_precedes_counters_and_temporary_pumps() {
    let (mut e, ring) = setup(sol_ring(), 0);
    crate::replacement::put_counters(e.dev_state_mut(P0).unwrap(), ring, CounterKind::P1P1, 1);
    animate(&mut e, ring);
    assert_eq!(pt(&e, ring), (2, 2));
    cast(&mut e, index::GIANT_GROWTH, Some(ring));
    assert_eq!(pt(&e, ring), (5, 5));
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&e, ring), (2, 2));
}

#[test]
fn animate_artifact_removal_restores_noncreature_without_losing_counters() {
    let (mut e, ring) = setup(sol_ring(), 0);
    crate::replacement::put_counters(e.dev_state_mut(P0).unwrap(), ring, CounterKind::P1P1, 1);
    let aura = animate(&mut e, ring);
    cast(&mut e, index::DISENCHANT, Some(aura));
    assert!(!is_creature(&e, ring));
    let object = e.state().object(ring).unwrap();
    assert_eq!(object.characteristics().power, None);
    assert_eq!(object.characteristics().toughness, None);
    assert_eq!(object.counters.get(CounterKind::P1P1), 1);
    assert_eq!(object.zone, Zone::Battlefield);
}

#[test]
fn animate_artifact_second_aura_takes_over_after_first_is_destroyed() {
    let (mut e, ring) = setup(sol_ring(), 0);
    let first = animate(&mut e, ring);
    let second = animate(&mut e, ring);
    assert_eq!(pt(&e, ring), (1, 1));
    cast(&mut e, index::DISENCHANT, Some(first));
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Battlefield);
    assert!(is_creature(&e, ring));
    assert_eq!(pt(&e, ring), (1, 1));
}

#[test]
fn animate_artifact_new_permanent_cannot_tap_or_attack_until_its_next_turn() {
    let (mut e, _) = setup(sol_ring(), 0);
    let new_ring = cast(&mut e, sol_ring(), None);
    pass_until(&mut e, |e| at_rest(e, P0));
    assert!(priority_offer(&e).abilities.contains(&(new_ring, 0)));
    animate(&mut e, new_ring);
    pass_until(&mut e, |e| at_rest(e, P0));
    assert!(!priority_offer(&e).abilities.contains(&(new_ring, 0)));
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: new_ring,
                ability_index: 0
            }
        )
        .is_err()
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!attackers.contains(&new_ring));
    e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| e.state().turn.active == P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain && at_rest(e, P0)
    });
    assert!(priority_offer(&e).abilities.contains(&(new_ring, 0)));
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(attackers.contains(&new_ring));
}

#[test]
fn animate_artifact_old_permanent_attacks_and_deals_its_new_power() {
    let (mut e, ring) = setup(sol_ring(), 0);
    animate(&mut e, ring);
    let life = e.state().players[1].life;
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(attackers.contains(&ring));
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(ring, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().players[1].life, life - 1);
}

#[test]
fn animate_artifact_uses_copy_mana_value_and_keeps_enchantment_type() {
    let (mut e, ring) = setup(sol_ring(), 0);
    e.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 2);
    e.refresh_offer();
    let copy = in_hand(&e, P0, copy_artifact()).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: copy }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![ring],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    animate(&mut e, copy);
    assert_eq!(
        pt(&e, copy),
        (1, 1),
        "copies Sol Ring's mana value, not Copy Artifact's two"
    );
    let c = e.state().object(copy).unwrap().characteristics();
    assert!(
        c.types.contains(
            TypeSet::ARTIFACT
                .union(TypeSet::ENCHANTMENT)
                .union(TypeSet::CREATURE)
        )
    );
    assert_eq!(c.colors, ColorSet::EMPTY);
}

#[test]
fn animate_artifact_continues_when_its_source_loses_abilities_in_layer_six() {
    let (mut e, ring) = setup(sol_ring(), 0);
    let aura = animate(&mut e, ring);
    pass_until(&mut e, |e| at_rest(e, P0));
    // Explicit layer-six fixture; advance the engine to exercise static sync,
    // rather than only projecting an already populated effect table.
    let state = e.dev_state_mut(P0).unwrap();
    let filter = crate::effects::EffectFilter::object(state, aura);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier: baylee_cards_dsl::Modifier::LoseAllAbilities,
    });
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(
        e.state()
            .object(aura)
            .unwrap()
            .characteristics()
            .abilities_lost
            .is_some()
    );
    assert!(is_creature(&e, ring));
    assert_eq!(pt(&e, ring), (1, 1));
    pass_until(&mut e, |e| at_rest(e, P0));
    assert_eq!(pt(&e, ring), (1, 1));
}

#[test]
fn animate_artifact_before_swift_on_a_printed_noncreature_ends_noncreature() {
    let (mut e, ring) = setup(sol_ring(), 0);
    let aura = animate(&mut e, ring);
    cast(&mut e, index::SWIFT_RECONFIGURATION, Some(ring));
    assert!(
        !is_creature(&e, ring),
        "a no-op type removal cannot force an earlier animation to wait"
    );
    assert_eq!(e.state().object(aura).unwrap().zone, Zone::Battlefield);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!attackers.contains(&ring));
}

#[test]
fn animate_artifact_waits_for_swift_removing_a_printed_creatures_type() {
    let (mut e, juggernaut) = setup(index::JUGGERNAUT, 0);
    cast(&mut e, index::SWIFT_RECONFIGURATION, Some(juggernaut));
    assert!(!is_creature(&e, juggernaut));
    animate(&mut e, juggernaut);
    assert!(is_creature(&e, juggernaut));
    assert_eq!(pt(&e, juggernaut), (4, 4));
}

#[test]
fn animate_artifact_two_auras_with_swift_between_continue_the_correct_setter() {
    let (mut e, ring) = setup(sol_ring(), 0);
    animate(&mut e, ring);
    cast(&mut e, index::SWIFT_RECONFIGURATION, Some(ring));
    // An independent P/T setter between the two Aura timestamps makes
    // which animation started observable; final creature type alone does not.
    let state = e.dev_state_mut(P0).unwrap();
    let filter = crate::effects::EffectFilter::object(state, ring);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::PtSet,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier: baylee_cards_dsl::Modifier::SetPT(2, 4),
    });
    animate(&mut e, ring);
    assert!(is_creature(&e, ring));
    assert_eq!(
        pt(&e, ring),
        (1, 1),
        "the later Aura starts after Swift and wins over the intervening setter"
    );
}
