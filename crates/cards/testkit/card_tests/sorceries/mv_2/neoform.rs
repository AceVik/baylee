//! `cards/sorceries/mv_2/neoform.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Neoform — {G}{U} sorcery: "As an additional cost to cast this spell,
/// sacrifice a creature. Search your library for a creature card with mana
/// value equal to 1 plus the sacrificed creature's mana value, put that card
/// onto the battlefield with an additional +1/+1 counter on it, then
/// shuffle."
///
/// The library is Llanowar Elves, mana value 1. Sacrificing Ornithopter (0)
/// finds one, which arrives with its counter; sacrificing an Elves (1) asks
/// for exactly 2 and finds none — the half that separates "equal to" from
/// "or less".
#[test]
fn neoform_finds_exactly_one_more_and_puts_a_counter_on_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                island(),
                island(),
                ornithopter(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[neoform(), neoform()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let first_elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();

    cast_from_hand(&mut engine, p0, neoform());
    sacrifice_as_cast(&mut engine, p0, thopter);
    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!("the helper returns only a card choice")
    };
    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(fielded(&engine, p0, llanowar_elves()), 2);
    assert_eq!(
        counters_on(&engine, found, CounterKind::P1P1),
        1,
        "\"with an additional +1/+1 counter on it\""
    );
    assert_eq!(counters_on(&engine, first_elves, CounterKind::P1P1), 0);

    let library_before = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, neoform());
    sacrifice_as_cast(&mut engine, p0, first_elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        fielded(&engine, p0, llanowar_elves()),
        1,
        "mana value exactly 1 + 1 finds no Elves"
    );
    assert_eq!(library_size(&engine, p0), library_before);
}
