use super::{CHAIN, chain_spell};
use baylee_core::ids::ObjectId;
use baylee_engine::choice::PlayerAction;

#[test]
fn a_kept_tap_names_the_spell_its_chain_cast_and_nothing_else() {
    let tap = |source: u32| PlayerAction::ActivateManaAbility {
        source: ObjectId::new(source, 0),
    };
    let cast = |card: u32| PlayerAction::CastSpell {
        card: ObjectId::new(card, 0),
    };
    let inputs = vec![
        (0, 4, tap(10)),
        (1, 5, PlayerAction::PassPriority),
        (0, 6, tap(11)),
        (0, 7, cast(29)),
        (0, 8, PlayerAction::PassPriority),
        (0, 9, tap(12)),
        (0, 10, PlayerAction::PassPriority),
        (0, 11, cast(30)),
    ];
    let id = |card: u32| {
        u32::try_from(
            serde_json::to_value(ObjectId::new(card, 0))
                .unwrap()
                .as_u64()
                .unwrap(),
        )
        .unwrap()
    };
    // From the tap at 4: the other seat's pass is not the chain's; the
    // cast at 7 is.
    assert_eq!(chain_spell(&inputs, 0, 4), id(29));
    // From the tap at 9 the seat passes before it casts.
    assert_eq!(chain_spell(&inputs, 0, 9), u32::MAX);
    // Nothing after the last input.
    assert_eq!(chain_spell(&inputs, 0, 11), u32::MAX);
    // A chain that only taps for longer than it is followed names none.
    let long: Vec<_> = (0..=CHAIN as u64)
        .map(|n| (0, n, tap(10)))
        .chain([(0, 99, cast(29))])
        .collect();
    assert_eq!(chain_spell(&long, 0, 0), u32::MAX);
}
