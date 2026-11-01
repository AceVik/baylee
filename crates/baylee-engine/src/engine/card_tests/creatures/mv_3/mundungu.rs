//! `cards/creatures/mv_3/mundungu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mundungu() -> CardIndex {
    card_index("3fb4161d-a5b9-411d-942c-2fdb039e7bde")
}

/// P0 has a Mundungu that is no longer summoning-sick; P1 casts a Grizzly
/// Bears off `forests` Forests (the spell costs two, so the rest float) and
/// P0 gets priority with the Bears on the stack. Returns the engine, the spell
/// and the question about to be asked: Mundungu's activation resolved up to
/// the tax prompt.
fn counter_attempt(forests: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), swamp()])
        .battlefield(1, &vec![forest(); forests])
        .hand(0, &[mundungu()])
        .hand(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mundungu());
    pass_until(&mut engine, stack_is_empty);

    // Advance through P1's turn and P0's next turn to P1's next main phase so Mundungu is no longer summoning-sick.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, GRIZZLY_BEARS);
    let spell = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .copied()
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    activate(&mut engine, P0, mundungu(), 0);
    aim_at(&mut engine, P0, spell);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    (engine, spell)
}

/// `Mundungu` ({1}{U}{B}, 1/1 Creature — Human Wizard, `Coverage::Partial`)
/// taps to counter target spell unless its controller pays `{1}`. The tax
/// belongs to the spell's controller, who has nothing left to pay it with and
/// declines: the spell is countered.
#[test]
fn mundungu_counters_target_spell_when_controller_cannot_pay_one() {
    let (mut engine, _spell) = counter_attempt(2);

    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the tax question was reached")
    };
    assert_eq!(player, P1, "the spell's controller is asked");
    assert_eq!(mana, 1);
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, GRIZZLY_BEARS).is_some());
}

/// The other side of "unless its controller pays {1}": with a mana left over
/// the controller pays it and the spell resolves.
#[test]
fn mundungu_does_not_counter_a_spell_whose_controller_pays_one() {
    let (mut engine, _spell) = counter_attempt(3);

    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P1, GRIZZLY_BEARS).is_some(),
        "the {{1}} was paid and the Bears resolved"
    );
    assert!(in_graveyard(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the floating {{G}} was the payment"
    );
}
