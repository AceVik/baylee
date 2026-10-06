//! `cards/lands/filter/mossfire_valley.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mossfire Valley prints exactly one line — "{1}, {T}: Add {R}{G}" — and
/// that line's arithmetic is the whole card: a filter land charges mana on
/// top of its tap, and it pays out a pair rather than one colour. Both halves
/// are read off one activation, because the pool the activation sits in holds
/// nothing but the two colourless a Sol Ring made: three mana left afterwards
/// is the pair less the {1}, where a bare "{T}: Add {R}{G}" would have left
/// four, and the red and the green are one each, so neither colour was
/// miscounted or the same one twice.
#[test]
fn mossfire_valley_charges_one_mana_for_a_red_and_a_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_artifact()])
        .hand(0, &[mossfire_valley()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let valley = play_land(&mut engine, p0, mossfire_valley());
    assert!(
        !is_tapped(&engine, valley),
        "a land with no printed entry modifier arrives untapped"
    );

    // Colourless and only colourless on purpose: whichever mana the {1} is
    // paid out of, it cannot be one of the two the card is about to make.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "the Sol Ring taps for {{C}}{{C}}"
    );
    assert!(
        !is_tapped(&engine, valley),
        "and the Valley is left standing, because its price is not its own \
         tap and `tap_all_mana` pays nothing larger (#159)"
    );

    activate(&mut engine, p0, mossfire_valley(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "\"Add {{R}}{{G}}\": the red half"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the green half — one of each, and not two of either"
    );
    assert_eq!(
        pool.total(),
        3,
        "two colourless less the {{1}} plus the pair: a filter land's mana is \
         not free, so four here would mean the cost was never charged"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so it has already resolved"
    );
    assert!(
        is_tapped(&engine, valley),
        "and the tap symbol was the other half of the price"
    );
}
