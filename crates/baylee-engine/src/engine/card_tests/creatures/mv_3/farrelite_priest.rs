//! `cards/creatures/mv_3/farrelite_priest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn farrelite_priest() -> CardIndex {
    card_index("3ee18e5f-3d12-489e-b337-29208caaed7a")
}

fn board() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[farrelite_priest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let priest = on_battlefield(&engine, P0, farrelite_priest()).expect("the Priest is out");
    (engine, priest)
}

fn pay_one_make_white(engine: &mut Engine<RegistryLookup>, priest: ObjectId) {
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    engine.refresh_offer();
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: priest,
                ability_index: 0,
            },
        )
        .expect("{1}: Add {W} activates");
}

/// Farrelite Priest (3ee18e5f-3d12-489e-b337-29208caaed7a) — "{1}: Add {W}.
/// If this ability has been activated four or more times this turn, sacrifice
/// this creature at the beginning of the next end step."
#[test]
fn farrelite_priest_turns_one_into_white() {
    let (mut engine, priest) = board();
    pay_one_make_white(&mut engine, priest);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "one white came out");
    assert_eq!(pool.available(ManaColor::Colorless), 0, "the {{1}} went in");
    assert_eq!(pool.total(), 1);
    assert!(
        !is_tapped(&engine, priest),
        "no {{T}} in the cost: the Priest stays untapped"
    );
}

#[test]
fn farrelite_priest_survives_three_activations() {
    let (mut engine, priest) = board();
    for _ in 0..3 {
        pay_one_make_white(&mut engine, priest);
    }
    pass_until(&mut engine, |e| e.state().turn.step == Step::End);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(priest).map(|o| o.zone),
        Some(Zone::Battlefield),
        "three activations are not four"
    );
    reach_their_main_phase(&mut engine, P1);
    assert!(on_battlefield(&engine, P0, farrelite_priest()).is_some());
}

#[test]
fn farrelite_priest_is_sacrificed_at_the_next_end_step_after_four_activations() {
    let (mut engine, priest) = board();
    for _ in 0..4 {
        pay_one_make_white(&mut engine, priest);
    }
    assert_eq!(
        engine.state().object(priest).map(|o| o.zone),
        Some(Zone::Battlefield),
        "not sacrificed at once: only at the beginning of the next end step"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, farrelite_priest()).is_some());

    pass_until(&mut engine, |e| e.state().turn.step == Step::End);
    assert!(!stack_is_empty(&engine), "the sacrifice is on the stack");
    assert_eq!(
        engine.state().object(priest).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, farrelite_priest()).is_none());
    assert!(in_graveyard(&engine, P0, farrelite_priest()).is_some());
}
