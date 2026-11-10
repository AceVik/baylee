//! `cards/creatures/mv_6/phyrexian_gargantua.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Phyrexian Gargantua` is a `{4}{B}{B}` 4/4 Phyrexian Horror under `Coverage::Implemented`.
/// When it enters the battlefield, its triggered ability causes its controller to draw two cards and lose 2 life.
/// Casting it from hand off six Swamps resolves the creature and triggers its enters-the-battlefield effect,
/// leaving a 4/4 body on the battlefield, reducing controller life by 2, and drawing two cards from the library.
#[test]
fn phyrexian_gargantua_enters_draws_two_cards_and_loses_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[phyrexian_gargantua()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let initial_life = engine.state().players[0].life;
    let initial_library_size = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, phyrexian_gargantua());

    pass_until(&mut engine, stack_is_empty);

    let gargantua = on_battlefield(&engine, p0, phyrexian_gargantua())
        .expect("Phyrexian Gargantua resolved and entered the battlefield");
    assert_eq!(
        pt(&engine, gargantua),
        (4, 4),
        "Phyrexian Gargantua is a 4/4"
    );

    assert_eq!(
        engine.state().players[0].life,
        initial_life - 2,
        "controller loses 2 life from ETB trigger"
    );
    assert_eq!(
        library_size(&engine, p0),
        initial_library_size - 2,
        "controller draws 2 cards from library"
    );
}
