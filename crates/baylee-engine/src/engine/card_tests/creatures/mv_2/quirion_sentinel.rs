//! `cards/creatures/mv_2/quirion_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Quirion Sentinel is `{1}{G}` for a 2/1 Elf Druid and prints exactly
/// one line: "When this creature enters, add one mana of any color." The
/// card is the only place in the scenario where mana can be produced — two
/// Forests pay for the spell completely (GG against `{1}{G}`), so the pool
/// is empty when the trigger resolves, and exactly one floating mana of the
/// chosen color is a statement about the card and not about a land.
/// The trigger goes on the stack as a normal triggered ability (CR 603.3),
/// so it is only asked for after priority passes — hence `pass_until`
/// until the `ChooseColor` question, and five colors to choose from, because
/// "any color" means the five colors of the game (CR 105.4) and not
/// colorless.
#[test]
fn quiron_sentinel_adds_one_mana_of_any_color_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[quiron_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{G} from two Forests: afterwards the pool is empty, and nothing on
    // the board can make mana anymore — the Sentinel itself is not a mana
    // source.
    cast_from_hand(&mut engine, p0, quiron_sentinel());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Forests are tapped and the spell is paid"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("die Bedingung hat gerade gematcht")
    };
    assert_eq!(
        player, p0,
        "the controller of the Sentinel chooses the color"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" schließt {color:?} ein: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is not a color (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the offered colors");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "one mana of the chosen color, from the trigger and not from a land"
    );
    assert_eq!(
        pool.total(),
        1,
        "exactly one: the board has no other mana that could float next to it"
    );

    let sentinel =
        on_battlefield(&engine, p0, quiron_sentinel()).expect("der Sentinel ist angekommen");
    assert_eq!(
        pt(&engine, sentinel),
        (2, 1),
        "und was angekommen ist, ist der gedruckte 2/1-Körper"
    );
}
