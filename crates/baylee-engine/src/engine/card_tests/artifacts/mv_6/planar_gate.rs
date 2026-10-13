//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn planar_gate_reductions_stack_and_apply_to_both_x_symbols_once() {
    let p0 = PlayerId::new(0);
    for (copies, mana, x) in [(1, 0, 1), (1, 2, 2), (2, 0, 2), (2, 2, 3)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &vec![index::PLANAR_GATE; copies])
            .hand(0, &[index::WALKING_BALLISTA])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, mana);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::WALKING_BALLISTA);
        assert!(matches!(e.pending(),Pending::ChooseNumber{min:0,max,..} if *max>=x));
        e.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        pass_until(&mut e, stack_is_empty);
        let ballista = on_battlefield(&e, p0, index::WALKING_BALLISTA).unwrap();
        assert_eq!(u32::from(counters_on(&e, ballista, CounterKind::P1P1)), x);
    }
}
#[test]
fn planar_gate_reduces_after_gloom_tax_without_erasing_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::PLANAR_GATE, index::GLOOM])
        .hand(0, &[index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    e.refresh_offer();
    let card = in_hand(&e, p0, index::SAVANNAH_LIONS).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.castable.contains(&card));
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::SAVANNAH_LIONS);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, index::SAVANNAH_LIONS).is_some());
}

/// "Creature spells **you** cast cost {2} less to cast." The Gate's
/// controller is part of the sentence: a Gate on the other side of the table
/// leaves the same Bears at its full {1}{G}, and the identical board with the
/// Gate on p0's own side is the control that makes the refusal mean it.
#[test]
fn planar_gate_cheapens_only_creature_spells_its_own_controller_casts() {
    let p0 = PlayerId::new(0);
    for gate_seat in [0_usize, 1] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(gate_seat, &[index::PLANAR_GATE])
            .hand(0, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        assert!(
            walk_to_own_main(&mut e, p0),
            "p0 reaches its own main phase"
        );
        // {1}{G} less {2} is {G}: one green is the whole reduced price.
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Green, 1);
        e.refresh_offer();
        let bears = in_hand(&e, p0, index::GRIZZLY_BEARS).expect("the Bears are in hand");
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority, got {:?}", e.pending())
        };
        assert_eq!(
            legal.castable.contains(&bears),
            gate_seat == 0,
            "a Gate on seat {gate_seat}: only the Gate's own controller's creature spells cost {{2}} less"
        );
        if gate_seat == 1 {
            let hash = e.state().snapshot_hash();
            assert!(
                e.apply(p0, PlayerAction::CastSpell { card: bears })
                    .is_err(),
                "the opponent's Gate does not make {{G}} enough for {{1}}{{G}}"
            );
            assert_eq!(
                e.state().snapshot_hash(),
                hash,
                "the refusal changed nothing"
            );
        } else {
            cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
            assert_eq!(
                e.state().players[0].mana_pool.total(),
                0,
                "the reduced {{G}} was the whole price"
            );
        }
    }
}

/// "**Creature** spells you cast": a Mind Stone ({2}, an artifact that is not
/// a creature) costs its full price under the Gate. One mana short of it is
/// not castable, and the printed {2} is, so the refusal is the price and not
/// a missing card.
#[test]
fn planar_gate_does_not_cheapen_a_noncreature_spell() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::PLANAR_GATE])
        .hand(0, &[index::MIND_STONE])
        .start();
    keep_mulligans(&mut e);
    assert!(
        walk_to_own_main(&mut e, p0),
        "p0 reaches its own main phase"
    );
    let stone = in_hand(&e, p0, index::MIND_STONE).expect("the Mind Stone is in hand");
    // One colorless at a time: the first leaves the Stone one short of {2},
    // the second pays it exactly.
    for (floating, castable) in [(1, false), (2, true)] {
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 1);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority, got {:?}", e.pending())
        };
        assert_eq!(
            legal.castable.contains(&stone),
            castable,
            "{floating} floating: the Gate's reduction is for creature spells, so the {{2}} artifact stays {{2}}"
        );
    }
    cast_with_floating(&mut e, p0, index::MIND_STONE);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        0,
        "the full {{2}} was spent"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, index::MIND_STONE).is_some());
}
