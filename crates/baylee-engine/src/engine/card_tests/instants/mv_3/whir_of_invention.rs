//! `cards/instants/mv_3/whir_of_invention.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Whir of Invention — {X}{U}{U}{U}: "Search your library for an artifact
/// card with mana value X or less, put it onto the battlefield, then
/// shuffle." The third card in the pool to write `Filter::CmcAtMostX`, and
/// the only one of the three that is an **instant** — so that is what this
/// plays, on the opponent's turn, where a sorcery-speed reading would never
/// have offered the spell at all.
///
/// The file is `Coverage::Partial` for improvise, and the shape of that gap
/// is worth naming: improvise would let artifacts pay part of the cost, so
/// what is missing makes the spell *dearer* and never cheaper. Four Islands
/// pay {1}{U}{U}{U} here with nothing left over, which is the full printed
/// price.
#[test]
fn whir_of_invention_is_an_instant_and_finds_an_artifact_within_its_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, quiet_artifact())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[whir_of_invention()])
        .start();
    keep_mulligans(&mut engine);

    // The opponent's main phase, with priority back on p0: an instant may be
    // cast here and a sorcery may not, which is the half of this card a mode
    // flag could get wrong. The active player holds priority first (CR
    // 117.3a), so the walk is to the pass after that and not to the phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, crate::turn::Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, which is {{1}}{{U}}{{U}}{{U}} exactly"
    );
    cast_with_floating(&mut engine, p0, whir_of_invention());
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("X = 1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        !options.is_empty(),
        "Sol Ring is mana value 1 and the bound announced was 1"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "improvise is the half that is missing, and its absence can only make \
         the spell dearer: the four Islands paid the whole printed cost"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact arrived on the battlefield, on the opponent's turn"
    );
}
