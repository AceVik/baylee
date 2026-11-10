//! `cards/enchantments/mv_3/knighthood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Knighthood` is an enchantment costing `{2}{W}` under `Coverage::Implemented`.
/// It prints "Creatures you control have first strike."
/// While on the battlefield, creatures controlled by its controller gain first strike,
/// while creatures controlled by the opponent do not gain the keyword.
#[test]
fn knighthood_grants_first_strike_to_controlled_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[knighthood(), llanowar_elves()])
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls Llanowar Elves");
    let their_drake =
        on_battlefield(&engine, p1, desert_drake()).expect("opponent controls Desert Drake");

    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::FIRST_STRIKE),
        "creature controlled by Knighthood's controller has first strike"
    );
    assert!(
        !keywords(&engine, their_drake).contains(KeywordSet::FIRST_STRIKE),
        "opponent's creature does not gain first strike"
    );
}
