//! `cards/lands/gates/heap_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Heap Gate: "{1}, {T}, Tap an untapped Gate you control: Create a Treasure
/// token." The second Gate is the tap cost, so it ends tapped.
#[test]
fn heap_gate_taps_another_gate_to_make_a_treasure() {
    let p0 = PlayerId::new(0);
    let gate = card_index("35922a30-6b84-44dd-a2f0-306554a1ae90");
    let mut engine = Duel::new(2306, forest())
        .battlefield(0, &[gate, gate, forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gates: Vec<ObjectId> = lands_of(&engine, p0)
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == gate))
        })
        .collect();
    let (a, b) = (gates[0], gates[1]);
    tap_mana_where(&mut engine, p0, |id| id != a && id != b);

    let offer = priority_offer(&engine);
    let (source, ability_index) = offer
        .abilities
        .iter()
        .copied()
        .find(|(id, i)| *id == a && *i == 2)
        .expect("the Treasure ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert!(options.contains(&b), "the other Gate is the one to tap");
        assert!(!options.contains(&a), "not the Gate that is activating");
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![b] })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, p0).len(), 1, "a Treasure token");
    assert!(is_tapped(&engine, a) && is_tapped(&engine, b));
}
