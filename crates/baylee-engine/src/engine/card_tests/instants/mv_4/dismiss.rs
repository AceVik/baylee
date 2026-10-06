//! `cards/instants/mv_4/dismiss.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dismiss` is an instant costing `{2}{U}{U}` under `Coverage::Implemented`.
/// It prints "Counter target spell. Draw a card."
/// When an opponent casts a spell (such as `Llanowar Elves`), `Dismiss` can be cast
/// targeting that spell on the stack, countering it to the graveyard and drawing a card.
#[test]
fn dismiss_counters_target_spell_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[dismiss()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let initial_p1_cards = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    let elf_spell =
        on_stack(&engine, llanowar_elves()).expect("Llanowar Elves spell is on the stack");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    let dismiss_card = in_hand(&engine, p1, dismiss()).expect("Dismiss is in p1's hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: dismiss_card })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Dismiss, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf_spell),
        "target spell is offered: {options:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elf_spell],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "countered spell is placed into p0's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "countered spell never entered the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, dismiss()).is_some(),
        "Dismiss resolved and went to p1's graveyard"
    );
    assert_eq!(
        library_size(&engine, p1),
        initial_p1_cards - 1,
        "Dismiss caused p1 to draw a card"
    );
}
