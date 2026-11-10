//! `cards/creatures/mv_4/aurochs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aurochs: "+1/+0 until end of turn **for each other attacking** Aurochs".
///
/// Three on the battlefield and two of them attacking, which is the one board
/// that separates the two words. `Filter::Another` dropped would count the
/// attacker itself, `Filter::Attacking` dropped would count the one standing
/// at home — and both mistakes make the same 4/3, so a board with two
/// Aurochs or with all three attacking cannot tell any of it apart.
///
/// It is also the counted pump on a **triggered** ability, where no `X` is
/// announced at all. That is why the card was a stub rather than wrong: the
/// transcoder refuses `Amount::X` on a `T:` line, because `x.unwrap_or(0)` is
/// a card that claims `Coverage::Implemented` and pumps by nothing.
#[test]
fn aurochs_counts_the_other_attacking_aurochs_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(315, forest())
        .battlefield(0, &[forest(), aurochs(), aurochs(), aurochs()])
        .start();
    keep_mulligans(&mut engine);

    // Seeded creatures are summoning sick on the turn the game began
    // (CR 302.6), so the swing is on this seat's second turn.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let herd = all_on_battlefield(&engine, p0, aurochs());
    assert_eq!(herd.len(), 3, "three of them, and only two will attack");
    assert_eq!(pt(&engine, herd[0]), (2, 3), "the premise: a 2/3");

    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    assert!(
        herd.iter().all(|a| attackers.contains(a)),
        "all three are offered as attackers"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (
                        herd[0],
                        baylee_core::ids::Defender::Player(PlayerId::new(1)),
                    ),
                    (
                        herd[1],
                        baylee_core::ids::Defender::Player(PlayerId::new(1)),
                    ),
                ],
            },
        )
        .expect("two of the three attack");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        (pt(&engine, herd[0]), pt(&engine, herd[1])),
        ((3, 3), (3, 3)),
        "each attacker sees exactly one *other attacking* Aurochs: +1/+0, \
         not +2/+0"
    );
    assert_eq!(
        pt(&engine, herd[2]),
        (2, 3),
        "and the one that stayed home never triggered at all"
    );
}
