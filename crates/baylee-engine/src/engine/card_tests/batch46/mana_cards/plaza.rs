#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch46_plaza_checks_two_gates_on_entry_and_again_on_resolution() {
    for (gate_count, remove_gate) in [(0, false), (1, false), (2, false), (2, true)] {
        let gates = vec![index::AZORIUS_GUILDGATE; gate_count];
        let mut e = prepared(&[index::PLAZA_OF_HARMONY], &gates, &[]);
        let before = e.state.players[0].life;
        play_land(&mut e, P0, index::PLAZA_OF_HARMONY);
        assert_eq!(!stack_is_empty(&e), gate_count == 2);
        if remove_gate {
            let gate = source(&e, index::AZORIUS_GUILDGATE);
            e.state
                .move_object(
                    gate,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.players[0].life,
            before
                + if gate_count == 2 && !remove_gate {
                    3
                } else {
                    0
                }
        );
    }
}

#[test]
fn batch46_plaza_reads_only_own_gates_even_when_their_costs_cannot_be_paid() {
    let mut e = prepared(
        &[],
        &[
            index::PLAZA_OF_HARMONY,
            index::AZORIUS_GUILDGATE,
            index::FOREST,
        ],
        &[index::RAKDOS_GUILDGATE],
    );
    let plaza = source(&e, index::PLAZA_OF_HARMONY);
    let gate = source(&e, index::AZORIUS_GUILDGATE);
    e.state.set_tapped(gate, true);
    pool(&mut e, &[]);
    ability(&mut e, plaza, 2);
    assert_eq!(colors(&e), vec![ManaColor::White, ManaColor::Blue]);
    color(&mut e, ManaColor::Blue);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Blue), 1);
}

#[test]
fn batch46_plaza_dynamic_amount_zero_does_not_promise_five_colors() {
    for extra_gate in [false, true] {
        let mut lands = vec![index::PLAZA_OF_HARMONY, index::BALDUR_S_GATE];
        if extra_gate {
            lands.push(index::AZORIUS_GUILDGATE);
        }
        let mut e = prepared(&[], &lands, &[]);
        pool(&mut e, &[]);
        let plaza = source(&e, index::PLAZA_OF_HARMONY);
        ability(&mut e, plaza, 2);
        if extra_gate {
            assert_eq!(colors(&e), ManaColor::ALL.to_vec());
            color(&mut e, ManaColor::Red);
        } else {
            assert!(matches!(e.pending(), Pending::Priority { .. }));
            assert_eq!(
                e.state.players[0].mana_pool.available(ManaColor::Colorless),
                1
            );
        }
    }
}

#[test]
fn batch46_plaza_resolves_cyclic_gate_dependencies_without_inventing_colors() {
    let mut e = prepared(
        &[index::FOREST],
        &[index::PLAZA_OF_HARMONY, index::EXOTIC_ORCHARD],
        &[index::EXOTIC_ORCHARD],
    );
    let plaza = source(&e, index::PLAZA_OF_HARMONY);
    let own = source(&e, index::EXOTIC_ORCHARD);
    let other = on_battlefield(&e, P1, index::EXOTIC_ORCHARD).unwrap();
    // A land-type-changing effect can make any land a Gate; retain both
    // real Orchard definitions so their dependency cycle remains genuine.
    for id in [own, other] {
        e.state
            .object_mut(id)
            .unwrap()
            .base_mut()
            .subtypes
            .insert(subtypes::land::GATE);
    }
    e.refresh_offer();
    let gate_mana = ManaSource::LandColor {
        filter: &Filter::HasSubtype(subtypes::land::GATE),
        mine: true,
        any_type: true,
    };
    assert!(crate::resolve::colors_of(e.state(), P0, gate_mana, plaza).is_empty());
    play_land(&mut e, P0, index::FOREST);
    pool(&mut e, &[]);
    ability(&mut e, plaza, 2);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 1);
}

#[test]
fn batch46_reflecting_pool_does_not_invent_a_commanders_identity() {
    let e = prepared(&[], &[index::REFLECTING_POOL, index::COMMAND_TOWER], &[]);
    assert!(
        crate::resolve::colors_of(
            e.state(),
            P0,
            ManaSource::LandColor {
                filter: &Filter::Any,
                mine: true,
                any_type: true
            },
            source(&e, index::REFLECTING_POOL)
        )
        .is_empty()
    );
}
