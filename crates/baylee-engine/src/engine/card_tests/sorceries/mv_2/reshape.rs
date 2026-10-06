//! `cards/sorceries/mv_2/reshape.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reshape — {X}{U}{U}: "Search your library for an artifact card with mana
/// value X or less, put it onto the battlefield, then shuffle."
///
/// `Filter::CmcAtMostX` is what says that bound, and a bound is only a bound
/// if some value falls outside it — so the spell is cast twice over a
/// library of Sol Rings, mana value 1. At **X = 1** the search offers them;
/// at **X = 0** it offers nothing, which is the assertion that separates
/// reading the announced number from ignoring it. A filter that always
/// matched would pass the first half and hand the player a Sol Ring for
/// {U}{U}.
///
/// The file is `Coverage::Partial` for the additional cost — "sacrifice an
/// artifact" is a `CostPart` no spell cost list can carry — and nothing
/// below pays it, so both readings of the search agree here.
#[test]
fn reshape_finds_an_artifact_within_the_x_it_announced_and_nothing_above_it() {
    let p0 = PlayerId::new(0);

    // X = 1: {1}{U}{U} is three mana, and three Islands are what is here.
    let mut engine = Duel::new(41, quiet_artifact())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[reshape()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, reshape());

    // CR 601.2b: X is announced before any cost is paid.
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("a spell with {{X}} asks for X, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster announces the value");
    assert!(min <= 1 && 1 <= max, "X = 1 is on offer: {min}..={max}");
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("the value the question enumerated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        !options.is_empty(),
        "Sol Ring is mana value 1, and 1 is `X or less` for X = 1"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{U}}{{U}} with X = 1 is three mana, which is what the Islands \
         made: an engine that never charged the X would have one floating"
    );
    let library_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let found = on_battlefield(&engine, p0, quiet_artifact()).expect("the artifact it found");
    assert!(
        !is_tapped(&engine, found),
        "\"put it onto the battlefield\" — the printing says nothing about tapped"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card left the library"
    );

    // X = 0: the same library, and nothing in it is `0 or less`.
    let mut zero = Duel::new(41, quiet_artifact())
        .battlefield(0, &[island(), island()])
        .hand(0, &[reshape()])
        .start();
    keep_mulligans(&mut zero);
    assert!(walk_to_own_main(&mut zero, p0), "p0 reaches its own main");
    tap_all_mana(&mut zero, p0);
    cast_with_floating(&mut zero, p0, reshape());
    zero.apply(p0, PlayerAction::ChooseNumber(0))
        .expect("X = 0 is a legal announcement");
    let zero_library = library_size(&zero, p0);
    pass_until(&mut zero, |e| at_rest(e, p0));
    assert_eq!(
        library_size(&zero, p0),
        zero_library,
        "Sol Ring is mana value 1, and the bound the spell announced was 0"
    );
    assert!(
        on_battlefield(&zero, p0, quiet_artifact()).is_none(),
        "so nothing arrived"
    );
}
