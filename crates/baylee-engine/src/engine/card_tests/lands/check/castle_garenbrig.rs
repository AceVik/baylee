//! `cards/lands/check/castle_garenbrig.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Castle Garenbrig is `Coverage::Partial`: "This land enters tapped unless
/// you control a Forest" and "{T}: Add {G}" are written, while the restricted
/// `{2}{G}{G}, {T}: Add six {G}` is not. The modifier's two branches need two
/// boards, so each is played once — with a Forest under the same seat the
/// land arrives untapped and its own tap is the only green in the pool (the
/// Forest named as the source kept back), and with the Forest on the *other*
/// side of the table it arrives tapped, which is what separates "you control
/// a Forest" from "there is a Forest somewhere on the battlefield".
#[test]
fn castle_garenbrig_enters_untapped_with_a_forest_of_its_own_and_taps_for_one_green() {
    let p0 = PlayerId::new(0);

    // A Forest of p0's own: the condition holds, so the land comes in ready.
    let mut mine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[castle_garenbrig()])
        .start();
    keep_mulligans(&mut mine);
    reach_main_phase(&mut mine, p0);
    let castle = play_land(&mut mine, p0, castle_garenbrig());
    assert!(
        !entered_tapped(&mine, castle),
        "\"enters tapped unless you control a Forest\" — the Forest is right there"
    );

    // {T}: Add {G}, with the Forest named as the one source kept back so the
    // green has nowhere else on this board to come from.
    let woods = on_battlefield(&mine, p0, forest()).expect("the Forest is out");
    assert_eq!(
        tap_mana_except(&mut mine, p0, woods),
        1,
        "the Castle is the only other source on the board"
    );
    assert_eq!(
        mine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "{{T}}: Add {{G}}"
    );
    assert!(is_tapped(&mine, castle), "paid with its own tap");
    assert!(!is_tapped(&mine, woods), "and nothing else moved");

    // A Forest across the table is not a Forest you control: the same card on
    // an otherwise identical board enters tapped.
    let mut theirs = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .battlefield(1, &[forest()])
        .hand(0, &[castle_garenbrig()])
        .start();
    keep_mulligans(&mut theirs);
    reach_main_phase(&mut theirs, p0);
    let castle = play_land(&mut theirs, p0, castle_garenbrig());
    assert!(
        entered_tapped(&theirs, castle),
        "the opponent's Forest does not satisfy \"you control a Forest\", and \
         a Plains of your own is no Forest at all"
    );
}
