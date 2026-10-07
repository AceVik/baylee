//! `cards/creatures/mv_2/steadfast_guard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Steadfast Guard is {W}{W} for a 2/2 Human Rebel whose entire text is
/// vigilance: "Attacking doesn't cause this creature to tap" (CR 702.20b).
/// The test plays the printed card in a real game — cast off two Plains in
/// one turn, attacking in the next, because a creature that arrived this turn
/// is sick (CR 302.6) — and the Llanowar Elves beside it is the control: it
/// has no vigilance, so the *same* declaration has to tap it. Without that,
/// `!is_tapped` on the Guard would be satisfied just as well by an attack step
/// that never happened. The 2/2 body is read too, so the card is not just its
/// keyword.
#[test]
fn steadfast_guard_attacks_without_tapping_for_its_vigilance() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[steadfast_guard()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, steadfast_guard());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, steadfast_guard()).is_some() && stack_is_empty(e)
    });

    let guard = on_battlefield(&engine, p0, steadfast_guard()).expect("the Guard resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf was already out");
    assert_eq!(pt(&engine, guard), (2, 2), "the printed 2/2");
    assert!(
        keywords(&engine, guard).contains(KeywordSet::VIGILANCE),
        "vigilance is the card's whole keyword line"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::VIGILANCE),
        "the control creature has none of it"
    );

    // A full turn cycle: the Guard was cast on p0's own turn and is sick
    // (CR 302.6) until p0's next one, and the same untap step that stands it
    // back up is what makes the comparison below a fair one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, guard), "the untap step stood it up");
    assert!(
        !is_tapped(&engine, elves),
        "and the Elf with it — otherwise the attack below is a tapped creature \
         against a tapped creature"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&guard) && attackers.contains(&elves),
        "both stand untapped and are offered: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guard, Defender::Player(p1)), (elves, Defender::Player(p1))],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, guard),
        "vigilance: declaring it as an attacker does not tap it (CR 702.20b)"
    );
    assert!(
        is_tapped(&engine, elves),
        "the Elf without vigilance is tapped by the very same declaration"
    );
}

/// "Otherwise, put that card into your hand." A creature on top goes to the
/// hand, not the battlefield.
#[test]
fn coiling_oracle_puts_a_revealed_nonland_into_the_hand() {
    let p0 = PlayerId::new(0);
    let (engine, top) = coil(steadfast_guard());
    assert!(revealed(&engine, top));
    assert!(on_battlefield(&engine, p0, steadfast_guard()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == steadfast_guard()))),
        "the Guard is in the hand"
    );
}
