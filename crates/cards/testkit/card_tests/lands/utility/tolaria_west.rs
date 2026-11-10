//! `cards/lands/utility/tolaria_west.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tolaria West` prints `This land enters tapped.`, `{{T}}: Add {{U}}.`, and `Transmute {{1}}{{U}}{{U}} ({{1}}{{U}}{{U}}, Discard this card: Search your library for a card with mana value 0, reveal it, put it into your hand, then shuffle. Transmute only as a sorcery.)`
///
/// Marked `Coverage::Implemented`, `Transmute` is an activated ability in `ActivationZone::Hand` requiring sorcery timing.
/// Floating `{{1}}{{U}}{{U}}` from three `island()` lands pays the transmute cost, discards `Tolaria West` into the graveyard, and searches the library via `ChoicePrompt::SearchLibrary` for a card with mana value 0 (an `island()`) to put into hand.
#[test]
fn tolaria_west_transmutes_from_hand_for_a_zero_mana_value_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[tolaria_west()])
        .battlefield(0, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(in_hand(&engine, p0, tolaria_west()).is_some());

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3
    );

    activate(&mut engine, p0, tolaria_west(), 1);
    assert!(in_graveyard(&engine, p0, tolaria_west()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search prompt, got {:?}", engine.pending());
    };
    assert!(
        !options.is_empty(),
        "library contains basic lands with mana value 0"
    );
    let chosen = options[0];

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(chosen).unwrap().zone, Zone::Hand);
}
